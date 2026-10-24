use rrd_lsm::{
    filesystem::AnchoredPath, Database, DatabaseOptions, Durability, Error, Mutation,
    ReadOnlyDatabase, TailRepair, WriteBatch,
};
use std::io::Write;

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Vec<u8>) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("native");
    let mut db = Database::create(&root).unwrap();
    db.write_owned(
        WriteBatch::new(vec![Mutation::Put {
            key: b"ack".to_vec(),
            value: b"independent expected value".to_vec(),
        }])
        .unwrap(),
        Durability::Authoritative,
    )
    .unwrap();
    drop(db);
    let wal = root.join("wal/00000000000000000001.wal");
    let prefix = std::fs::read(&wal).unwrap();
    (directory, root, prefix)
}

fn open(root: &std::path::Path) -> TailRepair {
    TailRepair::open_anchored(
        &AnchoredPath::open_root(root).unwrap(),
        DatabaseOptions::default(),
    )
    .unwrap()
}

#[test]
fn approved_repair_preserves_ack_prefix_is_read_only_until_apply_and_replays_exactly() {
    let (_directory, root, prefix) = fixture();
    let wal = root.join("wal/00000000000000000001.wal");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&wal)
        .unwrap()
        .write_all(b"partial")
        .unwrap();
    let before = std::fs::read(&wal).unwrap();
    assert!(matches!(
        ReadOnlyDatabase::open(&root),
        Err(Error::TornTail { .. })
    ));
    let repair = open(&root);
    let plan = repair.plan().unwrap();
    assert_eq!(plan.discarded_bytes, 7);
    assert_eq!(plan.valid_bytes, prefix.len() as u64);
    assert_eq!(std::fs::read(&wal).unwrap(), before);
    assert_eq!(
        repair.reader().get(b"ack").unwrap().unwrap(),
        b"independent expected value"
    );
    assert!(
        Database::open(&root).is_err(),
        "repair must retain the writer lease"
    );
    assert!(repair.apply(&plan, &"0".repeat(64)).is_err());
    assert_eq!(std::fs::read(&wal).unwrap(), before);
    let receipt = open(&root).apply(&plan, &plan.plan_sha256).unwrap();
    assert!(!receipt.idempotent_replay);
    assert_eq!(std::fs::read(&wal).unwrap(), prefix);
    let replay = open(&root).apply(&plan, &plan.plan_sha256).unwrap();
    assert!(replay.idempotent_replay);
    let reader = ReadOnlyDatabase::open(&root).unwrap();
    assert_eq!(
        reader.get(b"ack").unwrap().unwrap(),
        b"independent expected value"
    );
    drop(reader);
    let mut db = Database::open(&root).unwrap();
    db.write_owned(
        WriteBatch::new(vec![Mutation::Put {
            key: b"after".to_vec(),
            value: b"usable".to_vec(),
        }])
        .unwrap(),
        Durability::Authoritative,
    )
    .unwrap();
    drop(db);
    assert!(
        open(&root).apply(&plan, &plan.plan_sha256).is_err(),
        "later writes invalidate repair replay"
    );
}

#[test]
fn stale_tail_and_complete_corruption_never_become_approved_data_loss() {
    let (_directory, root, prefix) = fixture();
    let wal = root.join("wal/00000000000000000001.wal");
    let mut torn = prefix.clone();
    torn.extend_from_slice(b"tail");
    std::fs::write(&wal, &torn).unwrap();
    let plan = open(&root).plan().unwrap();
    torn.push(1);
    std::fs::write(&wal, &torn).unwrap();
    assert!(open(&root).apply(&plan, &plan.plan_sha256).is_err());
    assert_eq!(std::fs::read(&wal).unwrap(), torn);
    let mut corrupt = prefix;
    *corrupt.last_mut().unwrap() ^= 1;
    std::fs::write(&wal, &corrupt).unwrap();
    assert!(matches!(
        TailRepair::open_anchored(
            &AnchoredPath::open_root(&root).unwrap(),
            DatabaseOptions::default()
        ),
        Err(Error::Corruption { .. })
    ));
    assert_eq!(std::fs::read(&wal).unwrap(), corrupt);
}

#[test]
fn repair_obeys_recovery_admission_before_mutation() {
    let (_directory, root, mut bytes) = fixture();
    bytes.extend_from_slice(b"tail");
    let wal = root.join("wal/00000000000000000001.wal");
    std::fs::write(&wal, &bytes).unwrap();
    let mut options = DatabaseOptions::default();
    options.recovery.wal_bytes = bytes.len() as u64 - 1;
    assert!(matches!(
        TailRepair::open_anchored(&AnchoredPath::open_root(&root).unwrap(), options),
        Err(Error::RecoveryLimit { .. })
    ));
    assert_eq!(std::fs::read(&wal).unwrap(), bytes);
}
