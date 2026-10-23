use rrd_lsm::{
    Database, DatabaseOptions, Durability, Error, Mutation, ReadOnlyDatabase, RecoveryResource,
    WriteBatch,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

fn fixture(root: &Path) {
    let mut database = Database::create(root).unwrap();
    for value in 0u8..3 {
        database
            .write_owned(
                WriteBatch::new(vec![Mutation::Put {
                    key: vec![value],
                    value: vec![value + 10],
                }])
                .unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
    }
}

#[test]
fn recovery_and_inspection_caps_preserve_bytes_and_adequate_reopen_reads_every_ack() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("native");
    fixture(&root);
    let before = bytes(&root);
    let wal_len = std::fs::metadata(root.join("wal/00000000000000000001.wal"))
        .unwrap()
        .len();
    for resource in [
        RecoveryResource::MetadataBytes,
        RecoveryResource::Files,
        RecoveryResource::ReadBytes,
        RecoveryResource::WalBytes,
        RecoveryResource::WalFrames,
    ] {
        let mut options = DatabaseOptions::default();
        match resource {
            RecoveryResource::MetadataBytes => options.recovery.metadata_bytes = 1,
            RecoveryResource::Files => options.recovery.files = 1,
            RecoveryResource::ReadBytes => options.recovery.read_bytes = 1,
            RecoveryResource::WalBytes => options.recovery.wal_bytes = wal_len - 1,
            RecoveryResource::WalFrames => options.recovery.wal_frames = 2,
            RecoveryResource::ElapsedMs => unreachable!(),
        }
        assert!(matches!(Database::open_with_options(&root, options),
            Err(Error::RecoveryLimit { resource: actual, .. }) if actual == resource));
        assert_eq!(bytes(&root), before);
        assert!(
            matches!(ReadOnlyDatabase::open_with_options(&root, options),
            Err(Error::RecoveryLimit { resource: actual, .. }) if actual == resource)
        );
        assert_eq!(bytes(&root), before);
    }
    let mut options = DatabaseOptions::default();
    options.recovery.wal_bytes = wal_len;
    options.recovery.wal_frames = 3;
    let database = Database::open_with_options(&root, options).unwrap();
    assert_eq!(database.snapshot().sequence, 3);
    for key in 0u8..3 {
        assert_eq!(
            database.get(&[key], database.snapshot()).unwrap(),
            Some(vec![key + 10])
        );
    }
    drop(database);
    assert_eq!(bytes(&root), before);
    let inspection = ReadOnlyDatabase::open_with_options(&root, options).unwrap();
    assert_eq!(inspection.manifest().durable_sequence, 0);
    drop(inspection);
    assert_eq!(bytes(&root), before);
}

#[test]
fn increasing_recovery_budget_does_not_turn_corruption_into_repair() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("native");
    fixture(&root);
    let wal = root.join("wal/00000000000000000001.wal");
    let mut damaged = std::fs::read(&wal).unwrap();
    *damaged.last_mut().unwrap() ^= 0x80;
    std::fs::write(&wal, damaged).unwrap();
    let before = bytes(&root);
    let mut options = DatabaseOptions::default();
    options.recovery.wal_frames = 2;
    assert!(matches!(
        Database::open_with_options(&root, options),
        Err(Error::RecoveryLimit { .. })
    ));
    assert_eq!(bytes(&root), before);
    assert!(matches!(
        Database::open(&root),
        Err(Error::Corruption { .. })
    ));
    assert!(matches!(
        ReadOnlyDatabase::open(&root),
        Err(Error::Corruption { .. })
    ));
    assert_eq!(bytes(&root), before);
}
