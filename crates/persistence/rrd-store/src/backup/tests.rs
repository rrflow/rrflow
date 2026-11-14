use super::*;
use std::fs;

#[test]
fn oversized_manifest_is_refused_without_reading_its_payload() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("oversized-manifest.json");
    let file = fs::File::create(&path).unwrap();
    file.set_len(MAX_ARCHIVE_MANIFEST_BYTES + 1).unwrap();
    drop(file);
    let path = AnchoredPath::from_path(&path).unwrap();
    for error in [
        load_object_manifest(&path).unwrap_err(),
        load_catalogue_manifest(&path).unwrap_err(),
    ] {
        assert!(error.to_string().contains("exceeds the v1 byte bound"));
    }
    assert_eq!(
        path.metadata().unwrap().len(),
        MAX_ARCHIVE_MANIFEST_BYTES + 1
    );
}

pub(super) fn transition(
    key: &str,
    expected: Option<&[u8]>,
    replacement: Option<&[u8]>,
    at: u64,
) -> ControlTransition {
    ControlTransition {
        key: key.into(),
        expected: expected.map(<[u8]>::to_vec),
        replacement: replacement.map(<[u8]>::to_vec),
        at,
        actor: "replay-test".into(),
        action: "test.transition".into(),
        request_id: format!("request-{at}"),
        operation_id: format!("operation-{at}"),
    }
}

pub(super) fn families() -> Vec<String> {
    vec![
        "server/state/install".into(),
        "server/state/one/audit".into(),
    ]
}

/// A source with a key that moves three times and another that is created
/// and deleted, archived under the declared families.
fn archived_source(root: &Path) -> (AnchoredPath, ControlArchiveInventory) {
    let source = RrflowKvStore::create_new(&root.join("source")).unwrap();
    let control = source.control();
    control
        .commit(&transition(
            "server/state/install/record",
            None,
            Some(b"v1"),
            1,
        ))
        .unwrap();
    control
        .commit(&transition(
            "server/state/one/audit/a",
            None,
            Some(b"a1"),
            2,
        ))
        .unwrap();
    control
        .commit(&transition(
            "server/state/install/record",
            Some(b"v1"),
            Some(b"v2"),
            3,
        ))
        .unwrap();
    control
        .commit(&transition(
            "server/state/one/session/s",
            None,
            Some(b"lease"),
            4,
        ))
        .unwrap();
    control
        .commit(&transition(
            "server/state/one/audit/a",
            Some(b"a1"),
            None,
            5,
        ))
        .unwrap();
    control
        .commit(&transition(
            "server/state/install/record",
            Some(b"v2"),
            Some(b"v3"),
            6,
        ))
        .unwrap();
    let catalogue_root = root.join("catalogue");
    fs::create_dir_all(&catalogue_root).unwrap();
    let (file, inventory, _) = retain_control_journal(
        &source,
        &AnchoredPath::from_path(&catalogue_root).unwrap(),
        6,
        &families(),
    )
    .unwrap();
    assert_eq!(inventory.entry_count, 5);
    (
        AnchoredPath::from_path(&catalogue_root.join(file)).unwrap(),
        inventory,
    )
}

#[test]
fn an_interrupted_replay_resumes_after_its_last_committed_entry() {
    let root = tempfile::tempdir().unwrap();
    let (archive, inventory) = archived_source(root.path());
    let target_root = root.path().join("target");
    let target = RrflowKvStore::create_new(&target_root).unwrap();

    replay_control_entries(&archive, &inventory, &target, Some(3)).unwrap();
    let partial = control_restore_progress(&target).unwrap().unwrap();
    assert_eq!(partial.applied, 3);
    assert!(!partial.complete);
    assert_eq!(partial.first_restored_sequence, Some(1));
    assert_eq!(partial.last_restored_sequence, Some(3));
    assert_eq!(
        target.control().get("server/state/install/record").unwrap(),
        Some(b"v2".to_vec())
    );
    drop(target);
    assert!(
        verify_restored_control_journal(
            &archive,
            &inventory,
            &AnchoredPath::from_path(&target_root).unwrap(),
            Default::default(),
        )
        .unwrap_err()
        .to_string()
        .contains("incomplete"),
        "an interrupted replay does not verify"
    );

    restore_control_journal(
        &archive,
        &inventory,
        &AnchoredPath::from_path(&target_root).unwrap(),
        Default::default(),
    )
    .unwrap();
    let target = RrflowKvStore::open_existing(&target_root).unwrap();
    let progress = control_restore_progress(&target).unwrap().unwrap();
    assert!(progress.complete);
    assert_eq!(progress.applied, 5);
    assert_eq!(progress.last_restored_sequence, Some(5));
    assert_eq!(
        target.control().sequence().unwrap(),
        5,
        "no entry is replayed twice"
    );
    assert_eq!(
        target.control().get("server/state/install/record").unwrap(),
        Some(b"v3".to_vec())
    );
    assert_eq!(
        target.control().get("server/state/one/audit/a").unwrap(),
        None
    );
    assert_eq!(
        target.control().get("server/state/one/session/s").unwrap(),
        None
    );
    drop(target);

    // A completed replay is idempotent.
    restore_control_journal(
        &archive,
        &inventory,
        &AnchoredPath::from_path(&target_root).unwrap(),
        Default::default(),
    )
    .unwrap();
    let target = RrflowKvStore::open_existing(&target_root).unwrap();
    assert_eq!(target.control().sequence().unwrap(), 5);
}

#[test]
fn a_target_whose_control_state_diverges_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let (archive, inventory) = archived_source(root.path());
    let target_root = root.path().join("target");
    let target = RrflowKvStore::create_new(&target_root).unwrap();
    target
        .control()
        .commit(&transition(
            "server/state/install/record",
            None,
            Some(b"foreign"),
            9,
        ))
        .unwrap();
    drop(target);
    let error = restore_control_journal(
        &archive,
        &inventory,
        &AnchoredPath::from_path(&target_root).unwrap(),
        Default::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("diverged at source sequence 1"));
    let target = RrflowKvStore::open_existing(&target_root).unwrap();
    assert_eq!(
        target.control().sequence().unwrap(),
        1,
        "nothing was applied"
    );
    assert!(control_restore_progress(&target).unwrap().is_none());
}

#[test]
fn a_marker_from_another_archive_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let (archive, inventory) = archived_source(root.path());
    let target_root = root.path().join("target");
    let target = RrflowKvStore::create_new(&target_root).unwrap();
    replay_control_entries(&archive, &inventory, &target, Some(2)).unwrap();
    drop(target);
    let mut other = inventory.clone();
    other.archive_sha256 = "0".repeat(64);
    let target = RrflowKvStore::open_existing(&target_root).unwrap();
    assert!(replay_control_entries(&archive, &other, &target, None)
        .unwrap_err()
        .to_string()
        .contains("another control-journal replay"));
}
