use rrd_lsm::{
    Database, DatabaseOptions, Durability, Error, MaintenancePolicy, MutableLimits,
    MutableResource, Mutation, ReadOnlyDatabase, VersionedValue, WriteBatch,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn options(bytes: usize, versions: usize) -> DatabaseOptions {
    DatabaseOptions {
        mutable_limits: MutableLimits {
            accounted_bytes: bytes,
            versions,
        },
        // Deliberately above the hard cap: hard admission must work independently.
        maintenance: MaintenancePolicy {
            wal_payload_max_bytes: usize::MAX,
            memtable_max_versions: usize::MAX,
        },
        ..DatabaseOptions::default()
    }
}

fn put(value_bytes: usize) -> Mutation {
    Mutation::Put {
        key: b"k".to_vec(),
        value: vec![b'x'; value_bytes],
    }
}

fn write(
    db: &mut Database,
    mutations: Vec<Mutation>,
    owned: bool,
) -> rrd_lsm::Result<rrd_lsm::AppendReceipt> {
    let batch = WriteBatch::new(mutations).unwrap();
    if owned {
        db.write_owned(batch, Durability::Authoritative)
    } else {
        db.write(&batch, Durability::Authoritative)
    }
}

fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &path, output);
            } else {
                // A live Windows manifest lock cannot be read, even at EOF.
                let bytes = if entry.metadata().unwrap().len() == 0 {
                    Vec::new()
                } else {
                    std::fs::read(&path).unwrap()
                };
                output.insert(path.strip_prefix(root).unwrap().to_owned(), bytes);
            }
        }
    }
    let mut output = BTreeMap::new();
    visit(root, root, &mut output);
    output
}

fn limit(error: Error, resource: MutableResource, cap: usize, observed: usize) {
    assert!(
        matches!(error, Error::MutableLimit { resource: r, limit: l, observed: o }
        if r == resource && l == cap && o == observed),
        "{error:?}"
    );
}

#[test]
fn byte_boundaries_reject_before_effects_and_reuse_the_writer() {
    let record = std::mem::size_of::<VersionedValue>();
    for owned in [false, true] {
        for value_bytes in [9, 7, 8] {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path().join("native");
            let cap = record + 1 + 8;
            let policy = options(cap, 16);
            let mut db = Database::create_with_options(&root, policy).unwrap();
            write(&mut db, vec![put(1)], owned).unwrap();
            let before = inventory(&root);
            let result = write(&mut db, vec![put(value_bytes)], owned);
            if value_bytes > 8 {
                limit(
                    result.unwrap_err(),
                    MutableResource::AccountedBytes,
                    cap,
                    cap + 1,
                );
                assert_eq!(
                    inventory(&root),
                    before,
                    "rejection must precede even a pressure flush"
                );
                assert_eq!(db.snapshot().sequence, 1);
                assert_eq!(db.get(b"k", db.snapshot()).unwrap(), Some(vec![b'x']));
                write(&mut db, vec![put(8)], owned).unwrap();
            } else {
                assert!(result.unwrap().durable);
            }
            assert_eq!(db.snapshot().sequence, 2);
            assert_eq!(db.manifest().durable_sequence, 1);
            assert!(db.memtable().approximate_bytes() <= cap);
            drop(db);
            let reopened = Database::open_with_options(&root, policy).unwrap();
            assert_eq!(reopened.snapshot().sequence, 2);
            assert_eq!(
                reopened.get(b"k", reopened.snapshot()).unwrap(),
                Some(vec![b'x'; value_bytes.min(8)])
            );
        }
    }
}

#[test]
fn version_boundaries_charge_duplicate_keys_and_tombstones_atomically() {
    for owned in [false, true] {
        for count in [2, 3, 4] {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path().join("native");
            let policy = options(4096, 3);
            let mut db = Database::create_with_options(&root, policy).unwrap();
            let before = inventory(&root);
            let mut mutations = vec![put(2); count - 1];
            mutations.push(Mutation::Delete { key: b"k".to_vec() });
            let result = write(&mut db, mutations, owned);
            if count == 4 {
                limit(result.unwrap_err(), MutableResource::Versions, 3, 4);
                assert_eq!(inventory(&root), before);
                assert_eq!(db.snapshot().sequence, 0);
                write(&mut db, vec![put(2)], owned).unwrap();
            } else {
                assert_eq!(result.unwrap().last_sequence, count as u64);
                assert_eq!(db.memtable().version_count(), count);
                assert_eq!(db.get(b"k", db.snapshot()).unwrap(), None);
            }
        }
    }
}

#[test]
fn sustained_hot_key_pressure_preserves_held_history_and_current_deletes() {
    for owned in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("native");
        let policy = options(4096, 2);
        let mut db = Database::create_with_options(&root, policy).unwrap();
        write(&mut db, vec![put(1)], owned).unwrap();
        let held = db.begin_transaction().unwrap();
        for step in 2..=40 {
            let deleted = step % 3 == 0;
            let mutation = if deleted {
                Mutation::Delete { key: b"k".to_vec() }
            } else {
                put(step)
            };
            write(&mut db, vec![mutation], owned).unwrap();
            assert_eq!(db.snapshot().sequence, step as u64);
            assert_eq!(held.get(&db, b"k").unwrap(), Some(vec![b'x']));
            assert_eq!(
                db.get(b"k", db.snapshot()).unwrap(),
                if deleted {
                    None
                } else {
                    Some(vec![b'x'; step])
                }
            );
            assert!(db.memtable().version_count() <= 2);
            assert!(db.memtable().approximate_bytes() <= 4096);
        }
        assert_eq!(db.maintenance_stats().automatic_flushes, 19);
        assert_eq!(db.maintenance_stats().peak_memtable_versions, 2);
        assert!(db.maintenance_stats().peak_accounted_memtable_bytes <= 4096);
        assert!(db.maintenance_stats().automatic_compactions > 0);
        held.rollback().unwrap();
        drop(db);
        let reopened = Database::open_with_options(&root, policy).unwrap();
        assert_eq!(reopened.snapshot().sequence, 40);
        assert_eq!(
            reopened.get(b"k", reopened.snapshot()).unwrap(),
            Some(vec![b'x'; 40])
        );
    }
}

#[test]
fn replay_denial_preserves_bytes_and_larger_budget_recovers_exact_history() {
    for (byte_limited, one_frame) in [(false, false), (false, true), (true, false), (true, true)] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("native");
        let charge = std::mem::size_of::<VersionedValue>() + 1 + 8;
        let ample = options(charge * 4, 4);
        let mut db = Database::create_with_options(&root, ample).unwrap();
        if one_frame {
            write(&mut db, vec![put(8); 3], true).unwrap();
        } else {
            for _ in 0..3 {
                write(&mut db, vec![put(8)], true).unwrap();
            }
        }
        assert_eq!(
            db.manifest().durable_sequence,
            0,
            "fixture must exercise WAL replay"
        );
        drop(db);
        let before = inventory(&root);
        let tight = if byte_limited {
            options(charge * 3 - 1, 4)
        } else {
            options(charge * 4, 2)
        };
        let (resource, cap, observed) = if byte_limited {
            (MutableResource::AccountedBytes, charge * 3 - 1, charge * 3)
        } else {
            (MutableResource::Versions, 2, 3)
        };
        limit(
            Database::open_with_options(&root, tight)
                .err()
                .expect("writer must deny over-budget replay"),
            resource,
            cap,
            observed,
        );
        assert_eq!(inventory(&root), before);
        limit(
            ReadOnlyDatabase::open_with_options(&root, tight)
                .err()
                .expect("inspection must deny over-budget replay"),
            resource,
            cap,
            observed,
        );
        assert_eq!(inventory(&root), before);
        let inspected = ReadOnlyDatabase::open_with_options(&root, ample).unwrap();
        assert_eq!(inspected.inspection().visible_sequence, 3);
        assert_eq!(inspected.get(b"k").unwrap(), Some(vec![b'x'; 8]));
        drop(inspected);
        assert_eq!(inventory(&root), before);
        let exact = options(charge * 3, 3);
        let mut reopened = Database::open_with_options(&root, exact).unwrap();
        assert_eq!(reopened.snapshot().sequence, 3);
        assert_eq!(reopened.mutable_limits(), exact.mutable_limits);
        assert_eq!(
            reopened.maintenance_stats().peak_accounted_memtable_bytes,
            charge * 3
        );
        write(
            &mut reopened,
            vec![Mutation::Delete { key: b"k".to_vec() }],
            true,
        )
        .unwrap();
        drop(reopened);
        let final_read = ReadOnlyDatabase::open_with_options(&root, exact).unwrap();
        assert_eq!(final_read.inspection().visible_sequence, 4);
        assert_eq!(final_read.get(b"k").unwrap(), None);
    }
}

#[test]
fn zero_hard_limits_fail_before_creating_storage() {
    for policy in [options(0, 2), options(64, 0)] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("absent");
        assert!(matches!(
            Database::create_with_options(&root, policy),
            Err(Error::InvalidConfiguration(_))
        ));
        assert!(!root.exists());
    }
}

#[test]
fn pressure_flush_preserves_rejection_and_uncertain_wal_outcomes() {
    use rrd_lsm::{FailureMode, WriteBoundary};
    for owned in [false, true] {
        for boundary in [WriteBoundary::Prepared, WriteBoundary::WalSynced] {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path().join("native");
            let policy = options(4096, 1);
            let mut db = Database::create_with_options(&root, policy).unwrap();
            write(&mut db, vec![put(1)], owned).unwrap();
            let next = WriteBatch::new(vec![put(2)]).unwrap();
            let failure = if owned {
                db.write_owned_with_failure(
                    next,
                    Durability::Authoritative,
                    boundary,
                    FailureMode::Crash,
                )
            } else {
                db.write_with_failure(
                    &next,
                    Durability::Authoritative,
                    boundary,
                    FailureMode::Crash,
                )
            };
            assert!(matches!(failure, Err(Error::InjectedFailure { .. })));
            assert_eq!(db.manifest().durable_sequence, 1);
            assert_eq!(db.snapshot().sequence, 1);
            assert_eq!(db.get(b"k", db.snapshot()).unwrap(), Some(vec![b'x']));
            if boundary == WriteBoundary::WalSynced {
                assert!(matches!(
                    write(&mut db, vec![put(3)], owned),
                    Err(Error::RecoveryRequired { .. })
                ));
            }
            drop(db);
            let mut reopened = Database::open_with_options(&root, policy).unwrap();
            let recovered = if boundary == WriteBoundary::Prepared {
                1
            } else {
                2
            };
            assert_eq!(reopened.snapshot().sequence, recovered);
            assert_eq!(
                reopened.get(b"k", reopened.snapshot()).unwrap(),
                Some(vec![b'x'; recovered as usize])
            );
            write(&mut reopened, vec![put(3)], owned).unwrap();
            assert_eq!(reopened.snapshot().sequence, recovered + 1);
            assert_eq!(reopened.memtable().version_count(), 1);
            drop(reopened);
            let inspected = ReadOnlyDatabase::open_with_options(&root, policy).unwrap();
            assert_eq!(inspected.inspection().visible_sequence, recovered + 1);
            assert_eq!(inspected.get(b"k").unwrap(), Some(vec![b'x'; 3]));
        }
    }
}
