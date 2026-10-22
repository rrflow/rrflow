use super::*;
use crate::{Database, Durability, Error, Mutation, WalBatch, WalWriter, WriteBatch};

fn pair(id: u8) -> WriteBatch {
    WriteBatch::new(vec![
        Mutation::Put {
            key: vec![id, 0],
            value: vec![id, 10],
        },
        Mutation::Put {
            key: vec![id, 1],
            value: vec![id, 11],
        },
    ])
    .unwrap()
}

fn assert_pair(database: &Database, id: u8) {
    for column in 0..2 {
        assert_eq!(
            database.get(&[id, column], database.snapshot()).unwrap(),
            Some(vec![id, 10 + column])
        );
    }
}

#[test]
fn standalone_sync_failure_fences_writer() {
    for mode in [Fault::Before, Fault::After] {
        let temp = tempfile::tempdir().unwrap();
        let mut writer = WalWriter::create(&temp.path().join("active.wal")).unwrap();
        writer
            .append(
                &WalBatch {
                    first_sequence: 1,
                    last_sequence: 1,
                    payload: b"buffered",
                },
                Durability::Buffered,
            )
            .unwrap();
        let session = Session::start(temp.path(), Some((0, mode)));
        assert!(writer.sync().is_err());
        let state = session.finish();
        assert!(state.fired);
        assert!(matches!(
            writer.append(
                &WalBatch {
                    first_sequence: 2,
                    last_sequence: 2,
                    payload: b"forbidden"
                },
                Durability::Authoritative
            ),
            Err(Error::PoisonedWriter)
        ));
    }
}

#[test]
fn zero_vectored_write_is_an_error_and_fences_writer() {
    let temp = tempfile::tempdir().unwrap();
    let mut writer = WalWriter::create(&temp.path().join("active.wal")).unwrap();
    let session = Session::start(temp.path(), Some((0, Fault::Zero)));
    assert!(writer
        .append(
            &WalBatch {
                first_sequence: 1,
                last_sequence: 1,
                payload: b"zero"
            },
            Durability::Authoritative
        )
        .is_err());
    let state = session.finish();
    assert!(state.fired);
    assert!(matches!(writer.sync(), Err(Error::PoisonedWriter)));
}

#[test]
fn new_wal_name_and_acknowledged_bytes_survive_model_loss() {
    let temp = tempfile::tempdir().unwrap();
    let session = Session::start(temp.path(), None);
    let mut writer = WalWriter::create(&temp.path().join("active.wal")).unwrap();
    writer
        .append(
            &WalBatch {
                first_sequence: 1,
                last_sequence: 1,
                payload: b"durable",
            },
            Durability::Authoritative,
        )
        .unwrap();
    drop(writer);
    let state = session.finish();
    state.device.lose_volatile();
    let recovery = crate::recover(&temp.path().join("active.wal")).unwrap();
    assert_eq!(recovery.batches.len(), 1);
    assert_eq!(recovery.batches[0].payload, b"durable");
}

#[test]
fn uncertain_current_publication_fences_writes_and_reclamation() {
    // First discover the actual CURRENT rename ordinal rather than hardcoding
    // a call count. Both fixture inputs are identical; temporary names may vary.
    fn fixture() -> (tempfile::TempDir, Database) {
        let temp = tempfile::tempdir().unwrap();
        let mut database = Database::create(&temp.path().join("db")).unwrap();
        database.write(&pair(1), Durability::Authoritative).unwrap();
        (temp, database)
    }
    let (_temp, mut database) = fixture();
    let session = Session::start(database.root(), None);
    database.flush_memtable(10).unwrap();
    let state = session.finish();
    let ordinal = state
        .events
        .iter()
        .position(|event| {
            event.kind == Kind::Rename
                && event
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".CURRENT.")
        })
        .unwrap();
    drop(database);

    let (temp, mut database) = fixture();
    let session = Session::start(database.root(), Some((ordinal, Fault::After)));
    assert!(database.flush_memtable(10).is_err());
    assert!(session.finish().fired);
    assert!(
        database.write(&pair(2), Durability::Authoritative).is_err(),
        "stale WAL must not acknowledge another write"
    );
    assert!(
        database.garbage_collect().is_err(),
        "stale root must not reclaim the published generation"
    );
    let mut source = Database::create(&temp.path().join("source")).unwrap();
    for id in 1..=3 {
        source.write(&pair(id), Durability::Authoritative).unwrap();
    }
    let bundle = source.export_snapshot_bundle(20).unwrap();
    let bundle_file = source
        .export_snapshot_file(20, temp.path().join("snapshot"))
        .unwrap();
    let before = model::inventory(database.root());
    assert!(matches!(
        database.install_snapshot_bundle(&bundle, 30),
        Err(Error::RecoveryRequired { .. })
    ));
    assert!(matches!(
        database.install_snapshot_file(&bundle_file, 30),
        Err(Error::RecoveryRequired { .. })
    ));
    assert_eq!(
        model::inventory(database.root()),
        before,
        "fenced snapshot installation must have no file effects"
    );
    drop(database);
    let database = Database::open(&temp.path().join("db")).unwrap();
    assert_pair(&database, 1);
}

fn fixture() -> (tempfile::TempDir, Database) {
    let temp = tempfile::tempdir().unwrap();
    let mut database = Database::create(&temp.path().join("db")).unwrap();
    for id in 1..=2 {
        database
            .write(&pair(id), Durability::Authoritative)
            .unwrap();
        database.flush_memtable(u64::from(id)).unwrap();
    }
    database.checkpoint("retained", 2).unwrap();
    (temp, database)
}

fn history(
    database: &mut Database,
    acknowledged: &mut Vec<u8>,
    attempted: &mut Vec<u8>,
) -> crate::Result<()> {
    for id in 3..=4 {
        attempted.push(id);
        database.write(&pair(id), Durability::Authoritative)?;
        acknowledged.push(id);
        database.flush_memtable(u64::from(id))?;
    }
    database.compact(&[], 5)?;
    database.garbage_collect()?; // Must retain the named older root.
    database.release_checkpoint("retained")?;
    database.garbage_collect()?;
    Ok(())
}

fn check_history(database: &Database, acknowledged: &[u8], attempted: &[u8]) {
    for id in 1..=4 {
        let values = (0..2)
            .map(|column| database.get(&[id, column], database.snapshot()).unwrap())
            .collect::<Vec<_>>();
        let expected = vec![Some(vec![id, 10]), Some(vec![id, 11])];
        if id <= 2 || acknowledged.contains(&id) {
            assert_eq!(values, expected, "lost acknowledged atomic pair {id}");
        } else if attempted.contains(&id) {
            assert!(
                values == expected || values == vec![None, None],
                "uncertain pair {id} is partial or wrong: {values:?}"
            );
        } else {
            assert_eq!(values, vec![None, None], "unattempted pair appeared");
        }
    }
}

#[test]
fn sweep_io_failures_and_compound_recovery_preserves_acknowledged_history() {
    let (_temp, mut database) = fixture();
    let session = Session::start(database.root(), None);
    history(&mut database, &mut Vec::new(), &mut Vec::new()).unwrap();
    let baseline = session.finish();
    drop(database);
    for required in [
        Kind::Open,
        Kind::Read,
        Kind::Write,
        Kind::SyncData,
        Kind::SyncAll,
        Kind::DirectorySync,
        Kind::Rename,
        Kind::Remove,
    ] {
        assert!(
            baseline.events.iter().any(|event| event.kind == required),
            "missing {required:?}"
        );
    }
    let mut cases = 0;
    let mut recovery_cases = 0;
    for (ordinal, event) in baseline.events.iter().enumerate() {
        let mut modes = vec![Fault::Before, Fault::After];
        if event.kind == Kind::Write {
            modes.extend([
                Fault::Zero,
                Fault::Short(1),
                Fault::Short(33),
                Fault::Interrupted,
            ]);
        }
        for mode in modes {
            let (temp, mut database) = fixture();
            let session = Session::start(database.root(), Some((ordinal, mode)));
            let mut acknowledged = Vec::new();
            let mut attempted = Vec::new();
            let result = history(&mut database, &mut acknowledged, &mut attempted);
            let state = session.finish();
            assert!(state.fired, "unreached {ordinal} {event:?} {mode:?}");
            if matches!(mode, Fault::Before | Fault::After | Fault::Zero) {
                assert!(result.is_err(), "unreported {ordinal} {event:?} {mode:?}");
            } else {
                result.unwrap();
            }
            drop(database);
            state.device.lose_volatile();
            let root = temp.path().join("db");
            let before = model::inventory(&root);
            let session = Session::start(&root, None);
            let recovered = Database::open(&root)
                .unwrap_or_else(|error| panic!("reopen {ordinal} {event:?} {mode:?}: {error}"));
            let recovery = session.finish();
            check_history(&recovered, &acknowledged, &attempted);
            drop(recovered);
            assert_eq!(
                model::inventory(&root),
                before,
                "ordinary recovery changed bytes"
            );

            // Every call in recovery is faulted too, while preserving the
            // first failure's exact crash image. No expected values come from
            // engine readback, manifests or the device model.
            for second in 0..recovery.events.len() {
                for recovery_mode in [Fault::Before, Fault::After] {
                    let session = Session::start(&root, Some((second, recovery_mode)));
                    assert!(
                        Database::open(&root).is_err(),
                        "recovery fault did not fail: {ordinal}/{second}"
                    );
                    assert!(session.finish().fired);
                    assert_eq!(
                        model::inventory(&root),
                        before,
                        "failed recovery changed evidence"
                    );
                    let reopened = Database::open(&root).unwrap();
                    check_history(&reopened, &acknowledged, &attempted);
                    drop(reopened);
                    recovery_cases += 1;
                }
            }
            let mut reopened = Database::open(&root).unwrap();
            reopened.write(&pair(5), Durability::Authoritative).unwrap();
            assert_pair(&reopened, 5);
            reopened.garbage_collect().unwrap();
            drop(reopened);
            let final_open = Database::open(&root).unwrap();
            check_history(&final_open, &acknowledged, &attempted);
            assert_pair(&final_open, 5);
            cases += 1;
        }
    }
    eprintln!("C-07b: {} baseline calls, {cases} fault/loss cases, {recovery_cases} compound recovery cases", baseline.events.len());
}

#[test]
fn model_separates_buffered_bytes_file_barriers_and_namespace_barriers() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("active.wal");
    let mut writer = WalWriter::create(&path).unwrap();
    let session = Session::start(temp.path(), None);
    writer
        .append(
            &WalBatch {
                first_sequence: 1,
                last_sequence: 1,
                payload: b"ack",
            },
            Durability::Authoritative,
        )
        .unwrap();
    writer
        .append(
            &WalBatch {
                first_sequence: 2,
                last_sequence: 2,
                payload: b"buffered",
            },
            Durability::Buffered,
        )
        .unwrap();
    drop(writer);
    let state = session.finish();
    assert!(
        model::inventory(temp.path())[Path::new("active.wal")].len()
            > state.device.crash_image()[Path::new("active.wal")].len()
    );
    state.device.lose_volatile();
    let recovered = crate::recover(&path).unwrap();
    assert_eq!(recovered.recovered_through, 1);
    assert_eq!(recovered.batches[0].payload, b"ack");

    // Independent model characterization: bytes may be synced while their
    // renamed directory entry still rolls back to its previous identity.
    let root = temp.path().canonicalize().unwrap();
    let mut device = Device::seed(&root);
    let source = root.join("pending");
    let target = root.join("ready");
    device.opened(&source);
    device.write(&source, 0, b"new");
    device.sync_file(&source);
    device.rename(&source, &target);
    assert!(!device.crash_image().contains_key(Path::new("ready")));
    device.sync_directory(&root);
    assert_eq!(device.crash_image()[Path::new("ready")], b"new");
    device.remove(&target);
    assert_eq!(device.crash_image()[Path::new("ready")], b"new");
    device.sync_directory(&root);
    assert!(!device.crash_image().contains_key(Path::new("ready")));
}

#[test]
fn publication_retries_and_reopen_stabilize_selected_names_before_gc() {
    for namespace in ["segments", "manifests", "CURRENT"] {
        let (_temp, mut database) = fixture();
        database.write(&pair(3), Durability::Authoritative).unwrap();
        let session = Session::start(database.root(), None);
        database.flush_memtable(3).unwrap();
        let baseline = session.finish();
        let ordinal = baseline
            .events
            .iter()
            .position(|event| {
                event.kind == Kind::Rename
                    && if namespace == "CURRENT" {
                        event
                            .path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with(".CURRENT.")
                    } else {
                        event.path.parent().unwrap().ends_with(namespace)
                    }
            })
            .unwrap();
        drop(database);

        let (temp, mut database) = fixture();
        database.write(&pair(3), Durability::Authoritative).unwrap();
        let session = Session::start(database.root(), Some((ordinal, Fault::After)));
        assert!(database.flush_memtable(3).is_err());
        if namespace == "segments" {
            // No authority changed yet. Reusing content-addressed output must
            // establish its directory durability before publishing a root.
            database.flush_memtable(3).unwrap();
        } else {
            drop(database);
            database = Database::open(&temp.path().join("db")).unwrap();
            database.flush_memtable(3).unwrap();
        }
        database.garbage_collect().unwrap();
        drop(database);
        let state = session.finish();
        assert!(state.fired);
        state.device.lose_volatile();
        let database = Database::open(&temp.path().join("db"))
            .unwrap_or_else(|error| panic!("{namespace}: {error}"));
        for id in 1..=3 {
            assert_pair(&database, id);
        }
        database.checkpoints().unwrap();
    }
}

#[test]
fn partial_header_and_payload_errors_preserve_ack_and_require_explicit_tail_repair() {
    for prefix in [1, 15, 31, 32, 33, 40] {
        for mode in [Fault::Before, Fault::After, Fault::Zero] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("active.wal");
            let mut writer = WalWriter::create(&path).unwrap();
            writer
                .append(
                    &WalBatch {
                        first_sequence: 1,
                        last_sequence: 1,
                        payload: b"acknowledged",
                    },
                    Durability::Authoritative,
                )
                .unwrap();
            let session = Session::schedule(temp.path(), &[(0, Fault::Short(prefix)), (1, mode)]);
            assert!(writer
                .append(
                    &WalBatch {
                        first_sequence: 2,
                        last_sequence: 3,
                        payload: b"uncertain-pair-payload"
                    },
                    Durability::Authoritative
                )
                .is_err());
            assert!(matches!(writer.sync(), Err(Error::PoisonedWriter)));
            let state = session.finish();
            assert_eq!(state.fired_count, 2);
            drop(writer);
            let before = model::inventory(temp.path());
            let recovered = crate::recover(&path).unwrap();
            assert_eq!(recovered.batches[0].payload, b"acknowledged");
            assert_eq!(model::inventory(temp.path()), before);
            if recovered.torn_tail.is_some() {
                assert!(matches!(
                    WalWriter::open(&path),
                    Err(Error::TornTail { .. })
                ));
                assert_eq!(
                    model::inventory(temp.path()),
                    before,
                    "ordinary reopen must not repair"
                );
                let repaired = crate::repair_torn_tail(&path).unwrap();
                assert_eq!(repaired.batches.len(), 1);
                assert_eq!(repaired.batches[0].payload, b"acknowledged");
            } else {
                assert_eq!(recovered.batches.len(), 2);
                assert_eq!(recovered.batches[1].payload, b"uncertain-pair-payload");
            }
            // Loss of all unsynchronized writes must retain the prior ACK,
            // independently of the explicit repair path above.
            state.device.lose_volatile();
            let recovered = crate::recover(&path).unwrap();
            assert_eq!(recovered.batches.len(), 1);
            assert_eq!(recovered.batches[0].payload, b"acknowledged");
        }
    }
}

#[test]
fn checkpoint_unlink_uncertainty_is_reconciled_before_reclamation() {
    for mode in [Fault::Before, Fault::After] {
        let (temp, mut database) = fixture();
        database.write(&pair(3), Durability::Authoritative).unwrap();
        database.flush_memtable(3).unwrap();
        let session = Session::start(database.root(), Some((0, mode)));
        assert!(database.release_checkpoint("retained").is_err());
        assert!(database.garbage_collect().is_err());
        drop(database);
        let database = Database::open(&temp.path().join("db")).unwrap();
        database.garbage_collect().unwrap();
        drop(database);
        let state = session.finish();
        assert!(state.fired);
        state.device.lose_volatile();
        let database = Database::open(&temp.path().join("db")).unwrap();
        for id in 1..=3 {
            assert_pair(&database, id);
        }
        assert_eq!(
            database.checkpoints().unwrap().len(),
            usize::from(mode == Fault::Before)
        );
    }
}

#[test]
fn reused_successor_wal_is_synced_before_becoming_reachable() {
    let (_temp, mut database) = fixture();
    database.write(&pair(3), Durability::Authoritative).unwrap();
    let session = Session::start(database.root(), None);
    database.flush_memtable(3).unwrap();
    let baseline = session.finish();
    let ordinal = baseline
        .events
        .iter()
        .position(|event| {
            event.kind == Kind::SyncAll
                && event
                    .path
                    .extension()
                    .is_some_and(|extension| extension == "wal")
        })
        .unwrap();
    drop(database);

    let (temp, mut database) = fixture();
    database.write(&pair(3), Durability::Authoritative).unwrap();
    let session = Session::start(database.root(), Some((ordinal, Fault::Before)));
    assert!(database.flush_memtable(3).is_err());
    database.flush_memtable(3).unwrap();
    database.garbage_collect().unwrap();
    drop(database);
    let state = session.finish();
    assert!(state.fired);
    state.device.lose_volatile();
    let mut database = Database::open(&temp.path().join("db")).unwrap();
    for id in 1..=3 {
        assert_pair(&database, id);
    }
    database.write(&pair(4), Durability::Authoritative).unwrap();
    assert_pair(&database, 4);
}
