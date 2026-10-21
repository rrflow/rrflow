use rrd_lsm::{Error, Manifest, ManifestStore, SegmentDescriptor, MANIFEST_FORMAT_VERSION};

#[test]
#[cfg(any(unix, windows))]
fn retained_root_rejects_replaced_manifest_directory() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("estate");
    let outside = fixture.path().join("outside");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside must remain unchanged").unwrap();
    let store = ManifestStore::open(&root).unwrap();
    let directory = root.join("manifests");
    std::fs::rename(&directory, root.join("original-manifests")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &directory).unwrap();
    #[cfg(windows)]
    {
        let output = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "New-Item -ItemType Junction -Path $env:RRFLOW_TEST_LINK -Target $env:RRFLOW_TEST_TARGET -ErrorAction Stop | Out-Null",
            ])
            .env("RRFLOW_TEST_LINK", &directory)
            .env("RRFLOW_TEST_TARGET", &outside)
            .output()
            .unwrap();
        assert!(output.status.success(), "junction fixture: {:?}", output);
    }
    let manifest = Manifest::new(1, None, 100, 0, 1, Vec::new()).unwrap();
    let result = store.publish(&manifest, None);
    assert_eq!(
        std::fs::read_dir(&outside).unwrap().count(),
        1,
        "publication escaped into the outside directory: {result:?}"
    );
    assert_eq!(
        std::fs::read(outside.join("sentinel")).unwrap(),
        b"outside must remain unchanged"
    );
    assert!(
        result.is_err(),
        "a substituted manifest directory must be denied"
    );
}

fn segment(id: &str, first: &[u8], last: &[u8], minimum: u64, maximum: u64) -> SegmentDescriptor {
    SegmentDescriptor {
        id: id.into(),
        level: 0,
        format_version: rrd_lsm::SEGMENT_FORMAT_VERSION,
        schema_digest: rrd_lsm::SEGMENT_SCHEMA_DIGEST.into(),
        key_codec_digest: rrd_lsm::SEGMENT_KEY_CODEC_DIGEST.into(),
        page_format_digest: rrd_lsm::SEGMENT_PAGE_FORMAT_DIGEST.into(),
        first_key: first.into(),
        last_key: last.into(),
        minimum_sequence: minimum,
        maximum_sequence: maximum,
        entries: 2,
        bytes: 128,
        checksum: "22".repeat(32),
    }
}

fn leveled_segment(
    id: &str,
    level: u8,
    first: &[u8],
    last: &[u8],
    minimum: u64,
    maximum: u64,
) -> SegmentDescriptor {
    SegmentDescriptor {
        level,
        ..segment(id, first, last, minimum, maximum)
    }
}

#[test]
fn manifest_identity_is_stable_and_segment_order_is_canonical() {
    let left = Manifest::new(
        1,
        None,
        100,
        4,
        5,
        vec![
            segment(&"bb".repeat(32), b"m", b"z", 3, 4),
            segment(&"aa".repeat(32), b"a", b"l", 1, 2),
        ],
    )
    .unwrap();
    let right = Manifest::new(
        1,
        None,
        100,
        4,
        5,
        vec![
            segment(&"aa".repeat(32), b"a", b"l", 1, 2),
            segment(&"bb".repeat(32), b"m", b"z", 3, 4),
        ],
    )
    .unwrap();
    assert_eq!(left, right);
    left.validate().unwrap();
    assert!(
        left.segments
            .iter()
            .all(|segment| segment.format_version == 6),
        "the current manifest must reference only direct v6 segments"
    );

    let actual = format!("{}\n", serde_json::to_string_pretty(&left).unwrap());
    assert_eq!(left.format_version, MANIFEST_FORMAT_VERSION);
    assert_eq!(left.application_format, None);
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/manifest-v3.json");
    if std::env::var_os("RRFLOW_UPDATE_GOLDENS").is_some() {
        std::fs::create_dir_all(format!("{}/fixtures", env!("CARGO_MANIFEST_DIR"))).unwrap();
        std::fs::write(fixture, &actual).unwrap();
    }
    assert_eq!(actual, std::fs::read_to_string(fixture).unwrap());
}

#[test]
fn pre_1_0_manifest_fixture_is_rejected() {
    let fixture = include_bytes!("../fixtures/manifest-v1.json");
    let manifest: Manifest = serde_json::from_slice(fixture).unwrap();
    assert!(matches!(
        manifest.validate(),
        Err(Error::UnsupportedVersion {
            object: "manifest",
            version: 1
        })
    ));
}

#[test]
fn application_format_is_authenticated() {
    let mut manifest = Manifest::new_with_application_format(
        1,
        None,
        100,
        0,
        1,
        Vec::new(),
        0x5252_4453_4b30_3032,
    )
    .unwrap();
    assert_eq!(manifest.format_version, MANIFEST_FORMAT_VERSION);
    assert_eq!(manifest.application_format, Some(0x5252_4453_4b30_3032));
    manifest.validate().unwrap();

    manifest.application_format = Some(7);
    assert!(matches!(
        manifest.validate(),
        Err(Error::InvalidManifest(reason)) if reason.contains("digest")
    ));
    assert!(Manifest::new_with_application_format(1, None, 100, 0, 1, Vec::new(), 0).is_err());
}

#[test]
fn tampering_and_invalid_reachability_fail_closed() {
    let mut manifest = Manifest::new(
        2,
        Some("11".repeat(32)),
        100,
        4,
        3,
        vec![segment(&"aa".repeat(32), b"a", b"z", 1, 4)],
    )
    .unwrap();
    manifest.durable_sequence = 3;
    assert!(matches!(
        manifest.validate(),
        Err(Error::InvalidManifest(_))
    ));

    assert!(matches!(
        Manifest::new(
            1,
            None,
            100,
            2,
            3,
            vec![
                segment(&"aa".repeat(32), b"a", b"b", 1, 1),
                segment(&"aa".repeat(32), b"c", b"d", 2, 2),
            ],
        ),
        Err(Error::InvalidManifest(_))
    ));
}

#[test]
fn levels_above_l0_reject_overlapping_ranges() {
    let overlap = Manifest::new(
        1,
        None,
        100,
        4,
        5,
        vec![
            leveled_segment(&"aa".repeat(32), 1, b"a", b"m", 1, 2),
            leveled_segment(&"bb".repeat(32), 1, b"m", b"z", 3, 4),
        ],
    );
    assert!(matches!(overlap, Err(Error::InvalidManifest(_))));

    Manifest::new(
        1,
        None,
        100,
        4,
        5,
        vec![
            leveled_segment(&"aa".repeat(32), 1, b"a", b"l", 1, 2),
            leveled_segment(&"bb".repeat(32), 1, b"m", b"z", 3, 4),
        ],
    )
    .unwrap();
}

#[test]
fn current_publication_is_ordered_content_addressed_and_compare_and_swap() {
    let directory = tempfile::tempdir().unwrap();
    let store = ManifestStore::open(directory.path()).unwrap();
    assert!(store.current().unwrap().is_none());
    let first = Manifest::new(1, None, 100, 0, 1, Vec::new()).unwrap();
    let pointer = store.publish(&first, None).unwrap();
    assert_eq!(pointer.manifest, first.digest);
    assert_eq!(store.current().unwrap().unwrap().1, first);

    let second = Manifest::new(2, Some(first.digest.clone()), 101, 0, 1, Vec::new()).unwrap();
    assert!(matches!(
        store.publish(&second, None),
        Err(Error::ManifestConflict { .. })
    ));
    store.publish(&second, Some(&first.digest)).unwrap();
    assert_eq!(store.current().unwrap().unwrap().1, second);
    assert_eq!(store.load(&first.digest).unwrap(), first);
    drop(store);

    let reopened = ManifestStore::open(directory.path()).unwrap();
    assert_eq!(reopened.current().unwrap().unwrap().1, second);
}

#[test]
fn a_tampered_current_pointer_fails_closed() {
    let directory = tempfile::tempdir().unwrap();
    let store = ManifestStore::open(directory.path()).unwrap();
    let manifest = Manifest::new(1, None, 100, 0, 1, Vec::new()).unwrap();
    store.publish(&manifest, None).unwrap();
    drop(store);
    let path = directory.path().join("CURRENT");
    let mut bytes = std::fs::read(&path).unwrap();
    let index = bytes.iter().position(|byte| *byte == b'1').unwrap();
    bytes[index] = b'2';
    std::fs::write(path, bytes).unwrap();
    let reopened = ManifestStore::open(directory.path()).unwrap();
    assert!(reopened.current().is_err());
}

#[test]
fn named_checkpoints_pin_historical_manifests_until_explicit_release() {
    let directory = tempfile::tempdir().unwrap();
    let store = ManifestStore::open(directory.path()).unwrap();
    let first = Manifest::new(1, None, 100, 0, 1, Vec::new()).unwrap();
    store.publish(&first, None).unwrap();
    let checkpoint = store
        .checkpoint("before-migration", &first.digest, 101)
        .unwrap();
    assert_eq!(checkpoint.manifest, first.digest);
    assert_eq!(
        store
            .checkpoint("before-migration", &first.digest, 101)
            .unwrap(),
        checkpoint,
        "identical checkpoint creation is idempotent"
    );
    let second = Manifest::new(2, Some(first.digest.clone()), 102, 0, 1, Vec::new()).unwrap();
    store.publish(&second, Some(&first.digest)).unwrap();
    assert_eq!(store.checkpoints().unwrap(), vec![checkpoint]);
    assert_eq!(store.load(&first.digest).unwrap(), first);
    assert!(matches!(
        store.checkpoint("before-migration", &second.digest, 103),
        Err(Error::InvalidManifest(_))
    ));
    assert!(store.release_checkpoint("before-migration").unwrap());
    assert!(!store.release_checkpoint("before-migration").unwrap());
    assert!(store.checkpoints().unwrap().is_empty());
    assert!(store.checkpoint("../escape", &second.digest, 103).is_err());
}
