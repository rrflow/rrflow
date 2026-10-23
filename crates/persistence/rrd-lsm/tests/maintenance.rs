use rrd_lsm::{
    Database, DatabaseOptions, Durability, Error, MaintenancePolicy, Mutation, WriteBatch,
};
use tempfile::tempdir;

fn put(key: &str, value: &str) -> Mutation {
    Mutation::Put {
        key: key.as_bytes().to_vec(),
        value: value.as_bytes().to_vec(),
    }
}

fn options(max_versions: usize) -> DatabaseOptions {
    DatabaseOptions {
        maintenance: MaintenancePolicy {
            wal_payload_max_bytes: usize::MAX,
            memtable_max_versions: max_versions,
        },
        ..DatabaseOptions::default()
    }
}

#[test]
fn soft_compaction_denial_preserves_write_room_until_the_hard_l0_cap() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("soft-versus-hard");
    let options = DatabaseOptions {
        maintenance: MaintenancePolicy {
            wal_payload_max_bytes: usize::MAX,
            memtable_max_versions: 1,
        },
        compaction: rrd_lsm::CompactionPolicy {
            l0_compaction_trigger: 2,
            ..Default::default()
        },
        compaction_limits: rrd_lsm::CompactionLimits {
            input_logical_bytes: 1,
            l0_segments: 4,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    let mut acknowledged = Vec::new();
    for ordinal in 0..4 {
        let key = format!("key-{ordinal}");
        database
            .write_owned(
                WriteBatch::new(vec![put(&key, "ack")]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap_or_else(|error| panic!("write {ordinal} fits hard capacity: {error}"));
        acknowledged.push((key, database.snapshot()));
        assert!(database.l0_segment_count() < options.compaction_limits.l0_segments as usize);
    }
    assert!(database.maintenance_stats().failed_compactions > 0);
    let head = database.snapshot();
    let denied = database.write_owned(
        WriteBatch::new(vec![put("denied", "no-effect")]).unwrap(),
        Durability::Authoritative,
    );
    assert!(
        matches!(denied, Err(Error::MaintenanceLimit { .. })),
        "{denied:?}"
    );
    assert_eq!(database.snapshot().sequence, head.sequence);
    assert_eq!(database.l0_segment_count(), 4);
    drop(database);
    let database = Database::open_with_options(&root, options).unwrap();
    assert_eq!(database.snapshot().sequence, head.sequence);
    assert_eq!(database.get(b"denied", head).unwrap(), None);
    for (key, snapshot) in &acknowledged {
        assert_eq!(
            database.get(key.as_bytes(), *snapshot).unwrap(),
            Some(b"ack".to_vec())
        );
    }
    drop(database);
    // Explicitly sufficient maintenance admission permits recovery. The hard
    // L0 cap and historical read identities remain exactly the same.
    let mut sufficient = options;
    sufficient.compaction_limits.input_logical_bytes = 4096;
    let mut database = Database::open_with_options(&root, sufficient).unwrap();
    for _ in 0..16 {
        if !database.maintenance_step().unwrap() {
            break;
        }
    }
    assert_eq!(database.compaction_debt_segments(), 0);
    database
        .write_owned(
            WriteBatch::new(vec![put("denied", "now-accepted")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    assert_eq!(database.snapshot().sequence, head.sequence + 1);
    for (key, snapshot) in acknowledged {
        assert_eq!(
            database.get(key.as_bytes(), snapshot).unwrap(),
            Some(b"ack".to_vec())
        );
    }
    drop(database);
    let database = Database::open_with_options(&root, sufficient).unwrap();
    assert_eq!(
        database.get(b"denied", database.snapshot()).unwrap(),
        Some(b"now-accepted".to_vec())
    );
}

#[test]
fn idle_maintenance_drains_without_flushing_the_acknowledged_mutable_tail() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("idle");
    let options = DatabaseOptions {
        compaction: rrd_lsm::CompactionPolicy {
            l0_compaction_trigger: 4,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("shared", "first")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(1).unwrap();
    let historical = database.begin_transaction().unwrap();
    let old = database.snapshot();
    for ordinal in 0..9 {
        database
            .write_owned(
                WriteBatch::new(vec![put("shared", &ordinal.to_string())]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(ordinal + 2).unwrap();
    }
    database
        .write_owned(
            WriteBatch::new(vec![put("shared", "tail"), put("only-tail", "ack")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    let acknowledged = database.snapshot();
    let wal_start = database.manifest().wal_start_sequence;
    let durable = database.manifest().durable_sequence;
    let versions = database.memtable().version_count();
    assert!(database.compaction_debt_segments() > 0);
    for _ in 0..30 {
        if !database.maintenance_step().unwrap() {
            break;
        }
        assert_eq!(database.snapshot().sequence, acknowledged.sequence);
        assert_eq!(database.manifest().wal_start_sequence, wal_start);
        assert_eq!(database.manifest().durable_sequence, durable);
        assert_eq!(database.memtable().version_count(), versions);
        assert_eq!(
            historical.get(&database, b"shared").unwrap(),
            Some(b"first".to_vec())
        );
    }
    assert_eq!(database.compaction_debt_segments(), 0);
    assert!(!database.maintenance_step().unwrap());
    assert!(database.maintenance_stats().automatic_compactions > 0);
    assert_eq!(database.maintenance_stats().automatic_flushes, 0);
    drop(historical);
    drop(database);
    let reopened = Database::open_with_options(&root, options).unwrap();
    assert_eq!(reopened.snapshot().sequence, acknowledged.sequence);
    assert_eq!(
        reopened.get(b"shared", old).unwrap(),
        Some(b"first".to_vec())
    );
    assert_eq!(
        reopened.get(b"shared", acknowledged).unwrap(),
        Some(b"tail".to_vec())
    );
    assert_eq!(
        reopened.get(b"only-tail", acknowledged).unwrap(),
        Some(b"ack".to_vec())
    );
}

#[test]
fn idle_maintenance_rejects_an_unadmitted_merge_without_publishing_or_flushing() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("denied-idle");
    let options = DatabaseOptions {
        compaction: rrd_lsm::CompactionPolicy {
            l0_compaction_trigger: 4,
            ..Default::default()
        },
        compaction_limits: rrd_lsm::CompactionLimits {
            input_logical_bytes: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    for ordinal in 0..4 {
        database
            .write_owned(
                WriteBatch::new(vec![put(&format!("k{ordinal}"), "ack")]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(ordinal + 1).unwrap();
    }
    database
        .write_owned(
            WriteBatch::new(vec![put("tail", "ack")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    let before = database.manifest().clone();
    let head = database.snapshot();
    // A pending reclamation quantum, if any, is separate from compaction.
    let mut denied = false;
    for _ in 0..3 {
        match database.maintenance_step() {
            Ok(_) => {}
            Err(Error::MaintenanceLimit { .. }) => {
                denied = true;
                break;
            }
            Err(error) => panic!("unexpected denial: {error}"),
        }
    }
    assert!(denied);
    assert_eq!(database.manifest().digest, before.digest);
    assert_eq!(database.snapshot().sequence, head.sequence);
    assert_eq!(database.memtable().version_count(), 1);
    assert_eq!(database.maintenance_stats().failed_compactions, 1);
    drop(database);
    let reopened = Database::open(&root).unwrap();
    for key in ["k0", "k1", "k2", "k3", "tail"] {
        assert_eq!(
            reopened.get(key.as_bytes(), head).unwrap(),
            Some(b"ack".to_vec())
        );
    }
}

#[test]
fn failed_idle_partition_is_reclaimed_before_another_quantum() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("idle-partition");
    let options = DatabaseOptions {
        compaction: rrd_lsm::CompactionPolicy {
            l0_compaction_trigger: 2,
            target_segment_bytes: 64,
            ..Default::default()
        },
        compaction_limits: rrd_lsm::CompactionLimits {
            output_segments: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    // Seed broad inputs under the normal output target. The intentionally tiny
    // target/output cap below is the compaction failure being characterized.
    let mut database = Database::create(&root).unwrap();
    for ordinal in 0..2 {
        let value = ordinal.to_string().repeat(128);
        database
            .write_owned(
                WriteBatch::new(vec![put("a", &value), put("z", &value)]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(ordinal + 1).unwrap();
    }
    drop(database);
    let mut database = Database::open_with_options(&root, options).unwrap();
    assert!(database.maintenance_step().unwrap()); // pending flush cleanup
    let count_segments = || std::fs::read_dir(root.join("segments")).unwrap().count();
    let baseline = count_segments();
    let manifest = database.manifest().digest.clone();
    for _ in 0..2 {
        assert!(matches!(
            database.maintenance_step(),
            Err(Error::MaintenanceLimit {
                resource: rrd_lsm::MaintenanceResource::OutputSegments,
                ..
            })
        ));
        assert_eq!(database.manifest().digest, manifest);
        assert_eq!(
            count_segments(),
            baseline + 1,
            "one unpublished partition retained"
        );
        assert!(
            database.maintenance_step().unwrap(),
            "next quantum must own cleanup"
        );
        assert_eq!(count_segments(), baseline);
        for key in [b"a", b"z"] {
            assert_eq!(
                database.get(key, database.snapshot()).unwrap(),
                Some("1".repeat(128).into_bytes())
            );
        }
    }
    database
        .write_owned(
            WriteBatch::new(vec![put("tail", "ack")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    drop(database);
    let database = Database::open_with_options(&root, options).unwrap();
    assert_eq!(
        database.get(b"tail", database.snapshot()).unwrap(),
        Some(b"ack".to_vec())
    );
}

#[test]
fn idle_pruning_is_explicit_and_preserves_snapshots_transactions_and_checkpoints() {
    for prune in [false, true] {
        let directory = tempdir().unwrap();
        let root = directory.path().join("protected-idle");
        let mut database = Database::create(&root).unwrap();
        database
            .write_owned(
                WriteBatch::new(vec![put("hot", "0")]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(1).unwrap();
        database.checkpoint("original", 1).unwrap();
        let original = root
            .join("segments")
            .join(format!("{}.seg", database.manifest().segments[0].id));
        database
            .write_owned(
                WriteBatch::new(vec![put("hot", "1")]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(2).unwrap();
        let protected = database.snapshot();
        database
            .write_owned(
                WriteBatch::new(vec![put("hot", "2")]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(3).unwrap();
        let registered = database.begin_transaction().unwrap();
        for ordinal in 3..12 {
            database
                .write_owned(
                    WriteBatch::new(vec![put("hot", &ordinal.to_string())]).unwrap(),
                    Durability::Authoritative,
                )
                .unwrap();
            database.flush_memtable(ordinal + 1).unwrap();
        }
        let durable = database.manifest().durable_sequence;
        database
            .write_owned(
                WriteBatch::new(vec![put("hot", "tail")]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        let head = database.snapshot();
        for _ in 0..64 {
            let worked = if prune {
                database
                    .maintenance_step_with_protected_snapshots(&[protected])
                    .unwrap()
            } else {
                database.maintenance_step().unwrap()
            };
            assert_eq!(
                database.get(b"hot", protected).unwrap(),
                Some(b"1".to_vec())
            );
            assert_eq!(
                registered.get(&database, b"hot").unwrap(),
                Some(b"2".to_vec())
            );
            if !worked {
                break;
            }
        }
        assert_eq!(database.compaction_debt_segments(), 0);
        assert_eq!(
            database
                .manifest()
                .segments
                .iter()
                .map(|segment| segment.entries)
                .sum::<u64>(),
            if prune { 3 } else { 12 }
        );
        assert_eq!(database.manifest().durable_sequence, durable);
        assert_eq!(database.snapshot(), head);
        assert_eq!(
            rrd_lsm::Segment::open(&original)
                .unwrap()
                .get(b"hot", 1)
                .unwrap(),
            Some(b"0".to_vec())
        );
        drop(registered);
        drop(database);
        let database = Database::open(&root).unwrap();
        assert_eq!(
            database.get(b"hot", protected).unwrap(),
            Some(b"1".to_vec())
        );
        assert_eq!(database.get(b"hot", head).unwrap(), Some(b"tail".to_vec()));
        assert!(original.exists());
        database.release_checkpoint("original").unwrap();
        database.garbage_collect().unwrap();
        assert!(!original.exists());
    }
}

#[test]
fn foreground_commit_retains_explicit_untracked_snapshots() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("foreground-protected");
    let options = DatabaseOptions {
        maintenance: MaintenancePolicy {
            wal_payload_max_bytes: usize::MAX,
            memtable_max_versions: 1,
        },
        compaction: rrd_lsm::CompactionPolicy {
            l0_compaction_trigger: 2,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("hot", "initial")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    let protected = database.snapshot();
    for ordinal in 0..16 {
        let mut transaction = database.begin_transaction().unwrap();
        transaction
            .put(b"hot".to_vec(), ordinal.to_string().into_bytes())
            .unwrap();
        database
            .commit_transaction_with_protected_snapshots(
                transaction,
                Durability::Authoritative,
                &[protected],
                Default::default(),
            )
            .unwrap();
        assert_eq!(
            database.get(b"hot", protected).unwrap(),
            Some(b"initial".to_vec())
        );
    }
    assert!(database.maintenance_stats().automatic_compactions > 0);
    let head = database.snapshot();
    drop(database);
    let database = Database::open_with_options(&root, options).unwrap();
    assert_eq!(
        database.get(b"hot", protected).unwrap(),
        Some(b"initial".to_vec())
    );
    assert_eq!(database.get(b"hot", head).unwrap(), Some(b"15".to_vec()));
}

#[test]
fn reserve_denies_wal_flush_and_compaction_without_losing_prior_acknowledgements() {
    // Policy denial against observed native free space, not actual ENOSPC proof.
    for phase in ["wal", "flush", "compaction"] {
        let temporary = tempdir().unwrap();
        let root = temporary.path().join("native");
        let mut database = Database::create(&root).unwrap();
        for ordinal in 0..3 {
            database
                .write_owned(
                    WriteBatch::new(vec![put(&format!("key-{ordinal}"), "ack")]).unwrap(),
                    Durability::Authoritative,
                )
                .unwrap();
            if phase == "compaction" {
                database.flush_memtable(ordinal + 1).unwrap();
            }
        }
        let before = database.snapshot();
        let manifest = database.manifest().digest.clone();
        drop(database);
        let policy = DatabaseOptions {
            recovery_reserve_bytes: 1 << 60,
            ..DatabaseOptions::default()
        };
        let mut database = Database::open_with_options(&root, policy).unwrap();
        let denied = match phase {
            "wal" => database
                .write_owned(
                    WriteBatch::new(vec![put("denied", "none")]).unwrap(),
                    Durability::Authoritative,
                )
                .map(|_| ()),
            "flush" => database.flush_memtable(10).map(|_| ()),
            _ => database.compact(&[], 10).map(|_| ()),
        };
        assert!(
            matches!(denied, Err(Error::DiskAdmission { .. })),
            "{phase}: {denied:?}"
        );
        assert_eq!(database.snapshot().sequence, before.sequence);
        assert_eq!(database.manifest().digest, manifest);
        assert!(database
            .get(b"denied", database.snapshot())
            .unwrap()
            .is_none());
        drop(database);
        let mut database = Database::open(&root).unwrap();
        for ordinal in 0..3 {
            assert_eq!(
                database
                    .get(format!("key-{ordinal}").as_bytes(), database.snapshot())
                    .unwrap(),
                Some(b"ack".to_vec())
            );
        }
        database
            .write_owned(
                WriteBatch::new(vec![put("later", "accepted")]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(20).unwrap();
    }
}

#[test]
fn the_next_writer_synchronously_flushes_before_crossing_the_mutable_bound() {
    let root = tempdir().unwrap().keep().join("native");
    let mut database = Database::create_with_options(&root, options(2)).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("a", "one"), put("b", "two")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    assert_eq!(database.manifest().durable_sequence, 0);
    assert_eq!(database.memtable().version_count(), 2);

    database
        .write_owned(
            WriteBatch::new(vec![put("c", "three")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();

    assert_eq!(database.manifest().durable_sequence, 2);
    assert_eq!(database.memtable().version_count(), 1);
    assert_eq!(database.maintenance_stats().automatic_flushes, 1);
    assert_eq!(database.maintenance_stats().write_stalls, 1);
    assert_eq!(database.maintenance_stats().failed_flushes, 0);
    let snapshot = database.snapshot();
    assert_eq!(
        database.get(b"a", snapshot).unwrap().as_deref(),
        Some(b"one".as_slice())
    );
    assert_eq!(
        database.get(b"c", snapshot).unwrap().as_deref(),
        Some(b"three".as_slice())
    );

    drop(database);
    let reopened = Database::open_with_options(&root, options(2)).unwrap();
    let snapshot = reopened.snapshot();
    assert_eq!(snapshot.sequence, 3);
    assert_eq!(
        reopened.get(b"b", snapshot).unwrap().as_deref(),
        Some(b"two".as_slice())
    );
    assert_eq!(
        reopened.get(b"c", snapshot).unwrap().as_deref(),
        Some(b"three".as_slice())
    );
}

#[test]
fn one_oversized_atomic_batch_is_retained_and_reported_without_splitting() {
    let root = tempdir().unwrap().keep().join("native");
    let mut database = Database::create_with_options(&root, options(1)).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("a", "one"), put("b", "two")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();

    assert_eq!(database.snapshot().sequence, 2);
    assert_eq!(database.memtable().version_count(), 2);
    assert_eq!(database.maintenance_stats().oversized_batches, 1);
    assert_eq!(database.maintenance_stats().automatic_flushes, 0);
}

#[test]
fn zero_maintenance_limits_fail_before_creating_storage() {
    let root = tempdir().unwrap().keep().join("native");
    let error = Database::create_with_options(
        &root,
        DatabaseOptions {
            maintenance: MaintenancePolicy {
                wal_payload_max_bytes: 0,
                memtable_max_versions: 1,
            },
            ..DatabaseOptions::default()
        },
    )
    .err()
    .expect("invalid policy must fail");
    assert!(matches!(error, Error::InvalidConfiguration(_)));
    assert!(!root.exists());
}

#[test]
fn retained_generations_reject_before_wal_and_release_on_actual_owner_drop() {
    use rrd_lsm::{
        MutableResource, ProjectedReadBudget, ProjectedReadProjection, ProjectedReadRange,
        ProjectedReadRequest,
    };
    let directory = tempdir().unwrap();
    let root = directory.path().join("native");
    let mut policy = options(100);
    policy.mutable_retention.generations = 2;
    let mut database = Database::create_with_options(&root, policy).unwrap();
    let memory = database.mutable_memory();
    let write = |value: &str| WriteBatch::new(vec![put("key", value)]).unwrap();
    let reader = |database: &Database| {
        database
            .begin_projected_read(ProjectedReadRequest {
                ranges: vec![ProjectedReadRange::all()],
                projection: ProjectedReadProjection::KeyValue,
                snapshot: database.snapshot(),
                budget: ProjectedReadBudget::default(),
            })
            .unwrap()
    };
    database
        .write_owned(write("one"), Durability::Authoritative)
        .unwrap();
    let mut first = reader(&database);
    database
        .write_owned(write("two"), Durability::Authoritative)
        .unwrap();
    let mut second = reader(&database);
    assert_eq!(memory.stats().generations, 2);
    assert_eq!(memory.stats().versions, 3);
    let wal = root.join("wal").join("00000000000000000001.wal");
    let before = std::fs::read(&wal).unwrap();
    assert!(matches!(
        database.write_owned(write("three"), Durability::Authoritative),
        Err(Error::MutableLimit {
            resource: MutableResource::Generations,
            ..
        })
    ));
    assert_eq!(std::fs::read(&wal).unwrap(), before);
    assert_eq!(database.snapshot().sequence, 2);
    assert_eq!(memory.stats().denied_writes, 1);
    second.cancel().unwrap();
    database
        .write_owned(write("three"), Durability::Authoritative)
        .unwrap();
    assert_eq!(memory.stats().versions, 4);
    drop(database);
    assert_eq!(memory.stats().generations, 1);
    let batch = first.next_batch().unwrap().unwrap();
    assert_eq!(batch.value(0), Some(b"one".as_slice()));
    first.cancel().unwrap();
    assert_eq!(memory.stats().accounted_bytes, 0);
    assert_eq!(batch.value(0), Some(b"one".as_slice()));
    let reopened = Database::open(&root).unwrap();
    assert_eq!(
        reopened.get(b"key", reopened.snapshot()).unwrap(),
        Some(b"three".to_vec())
    );
}

#[test]
fn installed_write_maintenance_reclaims_only_an_admitted_prefix() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("native");
    let mut policy = options(1);
    policy.reclamation.delete_files = 1;
    let mut database = Database::create_with_options(&root, policy).unwrap();
    let write = |database: &mut Database, value: &str| {
        for _ in 0..100 {
            let before = database.snapshot().sequence;
            match database.write_owned(
                WriteBatch::new(vec![put("key", value)]).unwrap(),
                Durability::Authoritative,
            ) {
                Ok(_) => return,
                Err(Error::ReclamationPending) => assert_eq!(database.snapshot().sequence, before),
                Err(error) => panic!("unexpected maintenance error: {error}"),
            }
        }
        panic!("bounded cleanup failed to drain");
    };
    write(&mut database, "one");
    database.flush_memtable(1).unwrap();
    let pinned = database.checkpoint("retain", 2).unwrap();
    write(&mut database, "two");
    database.flush_memtable(3).unwrap();
    database.compact(&[], 4).unwrap();
    let report = database.garbage_collect().unwrap();
    assert!(
        report.removed_manifests.len() + report.removed_segments.len() + report.removed_wals.len()
            <= 1
    );
    assert!(report.retained_manifests.contains(&pinned.manifest));
    assert!(database.release_checkpoint("retain").unwrap());
    for index in 0..12 {
        write(&mut database, &index.to_string());
    }
    assert!(!root
        .join("manifests")
        .join(format!("{}.json", pinned.manifest))
        .exists());
    assert_eq!(
        database.get(b"key", database.snapshot()).unwrap(),
        Some(b"11".to_vec())
    );
}

#[test]
fn reclamation_inventory_denial_preserves_incoming_wal_and_unreachable_files() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("native");
    let database = Database::create(&root).unwrap();
    drop(database);
    let orphan = root.join("manifests").join("orphan.json");
    std::fs::write(&orphan, b"evidence").unwrap();
    let mut policy = options(100);
    policy.reclamation.inventory_entries = 1;
    let mut database = Database::open_with_options(&root, policy).unwrap();
    let wal = root.join("wal").join("00000000000000000001.wal");
    let before = std::fs::read(&wal).unwrap();
    assert!(matches!(
        database.write_owned(
            WriteBatch::new(vec![put("key", "denied")]).unwrap(),
            Durability::Authoritative
        ),
        Err(Error::ReclamationLimit { .. })
    ));
    assert_eq!(std::fs::read(&wal).unwrap(), before);
    assert_eq!(std::fs::read(&orphan).unwrap(), b"evidence");
    assert_eq!(database.snapshot().sequence, 0);
}
