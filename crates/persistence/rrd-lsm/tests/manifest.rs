use rrd_lsm::{Error, Manifest, ManifestStore, SegmentDescriptor, MANIFEST_FORMAT_VERSION};

#[test]
fn restarted_writer_preserves_interrupted_temporary_files_and_can_publish() {
    struct OwnedChild(std::process::Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            if !matches!(self.0.try_wait(), Ok(Some(_))) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }
    for family in ["manifest", "current", "checkpoint"] {
        let root = tempfile::tempdir().unwrap();
        let first = Manifest::new(1, None, 100, 0, 1, vec![]).unwrap();
        let next = Manifest::new(2, Some(first.digest.clone()), 101, 0, 1, vec![]).unwrap();
        let store = ManifestStore::open(root.path()).unwrap();
        store.publish(&first, None).unwrap();
        drop(store);
        let orphan = match family {
            "manifest" => root
                .path()
                .join("manifests")
                .join(format!(".{}.1.tmp", next.digest)),
            "current" => {
                // The immutable manifest already reached disk before CURRENT.
                std::fs::write(
                    root.path()
                        .join("manifests")
                        .join(format!("{}.json", next.digest)),
                    serde_json::to_vec(&next).unwrap(),
                )
                .unwrap();
                root.path().join(".CURRENT.1.tmp")
            }
            _ => root.path().join("checkpoints/.held.1.tmp"),
        };
        std::fs::write(&orphan, b"interrupted publication must remain unchanged").unwrap();
        let mut child = OwnedChild(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "restarted_manifest_writer_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("RRFLOW_MANIFEST_RESTART_ROOT", root.path())
                .env("RRFLOW_MANIFEST_RESTART_FAMILY", family)
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "bounded {family} restart helper"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert!(status.success(), "restarted {family} writer failed");
        assert_eq!(
            std::fs::read(&orphan).unwrap(),
            b"interrupted publication must remain unchanged"
        );
        let store = ManifestStore::open_existing(root.path()).unwrap();
        if family == "checkpoint" {
            assert_eq!(store.checkpoints().unwrap().len(), 1);
        } else {
            assert_eq!(store.current().unwrap().unwrap().1, next);
        }
    }
}

#[test]
#[ignore = "owned subprocess helper; parent supplies its disposable manifest root"]
fn restarted_manifest_writer_child() {
    let root = std::env::var_os("RRFLOW_MANIFEST_RESTART_ROOT").unwrap();
    let family = std::env::var("RRFLOW_MANIFEST_RESTART_FAMILY").unwrap();
    assert!(["manifest", "current", "checkpoint"].contains(&family.as_str()));
    let store = ManifestStore::open_existing(std::path::Path::new(&root)).unwrap();
    let first = store.current().unwrap().unwrap().1;
    if family == "checkpoint" {
        store.checkpoint("held", &first.digest, 101).unwrap();
    } else {
        let next = Manifest::new(2, Some(first.digest.clone()), 101, 0, 1, vec![]).unwrap();
        store.publish(&next, Some(&first.digest)).unwrap();
    }
}

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

#[test]
fn checkpoint_admission_is_atomic_idempotent_and_reusable_after_release() {
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let store = ManifestStore::open(root.path())
        .unwrap()
        .with_checkpoint_limits(2, rrd_lsm::ReclamationLimits::default())
        .unwrap();
    let manifest = Manifest::new(1, None, 100, 0, 1, vec![]).unwrap();
    store.publish(&manifest, None).unwrap();
    let first = store.checkpoint("first", &manifest.digest, 100).unwrap();
    let store = Arc::new(store);
    let outcomes = std::thread::scope(|threads| {
        let handles = (0..8)
            .map(|i| {
                let store = store.clone();
                let digest = &manifest.digest;
                threads.spawn(move || store.checkpoint(&format!("contender-{i}"), digest, 100))
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| matches!(
                r,
                Err(Error::CheckpointLimit {
                    limit: 2,
                    observed: 3
                })
            ))
            .count(),
        7
    );
    assert_eq!(store.checkpoint_ownership().unwrap(), (2, 2));
    assert_eq!(
        std::fs::read_dir(root.path().join("checkpoints"))
            .unwrap()
            .count(),
        2
    );
    assert_eq!(
        store.checkpoint("first", &manifest.digest, 100).unwrap(),
        first
    );
    assert!(store.release_checkpoint("first").unwrap());
    store
        .checkpoint("replacement", &manifest.digest, 101)
        .unwrap();
    let before = store.checkpoints().unwrap();
    drop(store);
    let reopened = ManifestStore::open_existing(root.path())
        .unwrap()
        .with_checkpoint_limits(2, rrd_lsm::ReclamationLimits::default())
        .unwrap();
    assert_eq!(reopened.checkpoints().unwrap(), before);
    assert!(matches!(
        reopened.checkpoint("over", &manifest.digest, 102),
        Err(Error::CheckpointLimit { .. })
    ));
}

#[test]
fn checkpoint_inventory_and_metadata_are_bounded_without_deleting_pins() {
    let root = tempfile::tempdir().unwrap();
    let store = ManifestStore::open(root.path())
        .unwrap()
        .with_checkpoint_limits(
            1,
            rrd_lsm::ReclamationLimits {
                metadata_bytes: 1024,
                ..Default::default()
            },
        )
        .unwrap();
    let manifest = Manifest::new(1, None, 100, 0, 1, vec![]).unwrap();
    store.publish(&manifest, None).unwrap();
    store.checkpoint("kept", &manifest.digest, 100).unwrap();
    let checkpoint = root.path().join("checkpoints/kept.json");
    let original = std::fs::read(&checkpoint).unwrap();
    let excess = root.path().join("checkpoints/.interrupted.tmp");
    std::fs::write(&excess, b"retained interrupted publication").unwrap();
    assert!(matches!(
        store.checkpoints(),
        Err(Error::CheckpointLimit { .. })
    ));
    assert_eq!(std::fs::read(&checkpoint).unwrap(), original);
    std::fs::remove_file(&excess).unwrap();
    std::fs::write(&checkpoint, vec![b' '; 1025]).unwrap();
    assert!(matches!(
        store.checkpoints(),
        Err(Error::ReclamationLimit {
            resource: rrd_lsm::ReclamationResource::MetadataBytes,
            ..
        })
    ));
    assert_eq!(std::fs::metadata(&checkpoint).unwrap().len(), 1025);
}
