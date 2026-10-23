use rrd_lsm::{
    CompactionPolicy, Database, DatabaseOptions, Durability, Mutation, ProjectedReadBudget,
    ProjectedReadProjection, ProjectedReadRange, ProjectedReadRequest, WriteBatch,
};
use std::collections::BTreeSet;

fn batch(operations: Vec<Mutation>) -> WriteBatch {
    WriteBatch::new(operations).unwrap()
}

fn put(key: &str, value: &str) -> Mutation {
    Mutation::Put {
        key: key.as_bytes().to_vec(),
        value: value.as_bytes().to_vec(),
    }
}

fn delete(key: &str) -> Mutation {
    Mutation::Delete {
        key: key.as_bytes().to_vec(),
    }
}

#[test]
fn automatic_compaction_selects_a_budgeted_subset_before_rejecting_writes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let options = DatabaseOptions {
        compaction: CompactionPolicy {
            l0_compaction_trigger: 4,
            target_segment_bytes: 512,
            ..CompactionPolicy::default()
        },
        compaction_limits: rrd_lsm::CompactionLimits {
            input_logical_bytes: 3000,
            ..rrd_lsm::CompactionLimits::default()
        },
        ..DatabaseOptions::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    let value = "x".repeat(1024);
    let mut acknowledged = Vec::new();
    for ordinal in 0..4 {
        let key = format!("key-{ordinal}");
        database
            .write(&batch(vec![put(&key, &value)]), Durability::Authoritative)
            .unwrap();
        acknowledged.push((key, database.snapshot()));
        database.flush_memtable(ordinal).unwrap();
    }
    let outcome = database
        .compact_if_needed(5)
        .unwrap()
        .expect("admitted subset");
    assert_eq!(outcome.input_segments, 2);
    assert!(!outcome.history_pruned);
    for (key, snapshot) in &acknowledged {
        assert_eq!(
            database.get(key.as_bytes(), *snapshot).unwrap(),
            Some(value.as_bytes().to_vec())
        );
    }
    database
        .write(
            &batch(vec![put("later", "accepted")]),
            Durability::Authoritative,
        )
        .unwrap();
    drop(database);
    let reopened = Database::open_with_options(&root, options).unwrap();
    for (key, snapshot) in acknowledged {
        assert_eq!(
            reopened.get(key.as_bytes(), snapshot).unwrap(),
            Some(value.as_bytes().to_vec())
        );
    }
    assert_eq!(
        reopened.get(b"later", reopened.snapshot()).unwrap(),
        Some(b"accepted".to_vec())
    );
}

#[test]
fn blocked_l0_can_clear_lower_level_overlap_without_exceeding_input_budget() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let options = DatabaseOptions {
        compaction: CompactionPolicy {
            l0_compaction_trigger: 4,
            target_segment_bytes: 256,
            ..CompactionPolicy::default()
        },
        compaction_limits: rrd_lsm::CompactionLimits {
            input_logical_bytes: 2500,
            ..rrd_lsm::CompactionLimits::default()
        },
        ..DatabaseOptions::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    let baseline = "b".repeat(1000);
    for (ordinal, key) in ["a", "z"].into_iter().enumerate() {
        database
            .write(&batch(vec![put(key, &baseline)]), Durability::Authoritative)
            .unwrap();
        database.flush_memtable(ordinal as u64).unwrap();
    }
    database.compact(&[], 2).unwrap().unwrap();
    let mut acknowledged = vec![(database.snapshot(), baseline)];
    for ordinal in 0..4 {
        let value = format!("{ordinal}").repeat(512);
        database
            .write(
                &batch(vec![put("a", &value), put("z", &value)]),
                Durability::Authoritative,
            )
            .unwrap();
        acknowledged.push((database.snapshot(), value));
        database.flush_memtable(ordinal + 3).unwrap();
    }
    let relief = database
        .compact_if_needed(10)
        .unwrap()
        .expect("lower-level relief");
    assert_eq!(relief.source_level, 1);
    assert!(!relief.history_pruned);
    let resumed = database
        .compact_if_needed(11)
        .unwrap()
        .expect("L0 resumes after relief");
    assert_eq!(resumed.source_level, 0);
    assert!(!resumed.history_pruned);
    for (snapshot, value) in &acknowledged {
        for key in [b"a", b"z"] {
            assert_eq!(
                database.get(key, *snapshot).unwrap(),
                Some(value.as_bytes().to_vec())
            );
        }
    }
    drop(database);
    let reopened = Database::open_with_options(&root, options).unwrap();
    for (snapshot, value) in acknowledged {
        for key in [b"a", b"z"] {
            assert_eq!(
                reopened.get(key, snapshot).unwrap(),
                Some(value.as_bytes().to_vec())
            );
        }
    }
}

#[test]
fn hard_compaction_input_limits_preserve_every_protected_version() {
    use rrd_lsm::{CompactionLimits, Error, MaintenanceResource};
    for resource in [
        MaintenanceResource::InputBytes,
        MaintenanceResource::InputLogicalBytes,
        MaintenanceResource::InputVersions,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("database");
        let mut limits = CompactionLimits::default();
        match resource {
            MaintenanceResource::InputBytes => limits.input_bytes = 1,
            MaintenanceResource::InputLogicalBytes => limits.input_logical_bytes = 1,
            MaintenanceResource::InputVersions => limits.input_versions = 1,
            _ => unreachable!(),
        }
        let mut database = Database::create_with_options(
            &root,
            DatabaseOptions {
                compaction_limits: limits,
                ..DatabaseOptions::default()
            },
        )
        .unwrap();
        database
            .write(&batch(vec![put("hot", "first")]), Durability::Authoritative)
            .unwrap();
        let first = database.snapshot();
        database.flush_memtable(1).unwrap();
        database
            .write(
                &batch(vec![put("hot", "second")]),
                Durability::Authoritative,
            )
            .unwrap();
        let second = database.snapshot();
        database.flush_memtable(2).unwrap();
        let before = database.manifest().digest.clone();
        assert!(
            matches!(database.compact(&[first], 3), Err(Error::MaintenanceLimit { resource: actual, .. }) if actual == resource)
        );
        assert_eq!(database.manifest().digest, before);
        assert_eq!(
            database.get(b"hot", first).unwrap(),
            Some(b"first".to_vec())
        );
        assert_eq!(
            database.get(b"hot", second).unwrap(),
            Some(b"second".to_vec())
        );
        drop(database);
        let mut reopened = Database::open(&root).unwrap();
        reopened.compact(&[first], 4).unwrap().unwrap();
        assert_eq!(
            reopened.get(b"hot", first).unwrap(),
            Some(b"first".to_vec())
        );
        assert_eq!(
            reopened.get(b"hot", second).unwrap(),
            Some(b"second".to_vec())
        );
    }
}

#[test]
fn blocked_l0_maintenance_cannot_be_bypassed_by_repeating_a_write() {
    use rrd_lsm::{CompactionLimits, Error};
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create_with_options(
        &root,
        DatabaseOptions {
            compaction: CompactionPolicy {
                l0_compaction_trigger: 2,
                ..CompactionPolicy::default()
            },
            compaction_limits: CompactionLimits {
                input_bytes: 1,
                l0_segments: 2,
                ..CompactionLimits::default()
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    for (index, value) in ["first", "second"].into_iter().enumerate() {
        database
            .write(&batch(vec![put("hot", value)]), Durability::Authoritative)
            .unwrap();
        database.flush_memtable(index as u64).unwrap();
    }
    let sequence = database.snapshot().sequence;
    for _ in 0..3 {
        assert!(matches!(
            database.write(
                &batch(vec![put("new", "must-not-appear")]),
                Durability::Authoritative
            ),
            Err(Error::MaintenanceLimit { .. })
        ));
        assert_eq!(database.snapshot().sequence, sequence);
        assert_eq!(database.get(b"new", database.snapshot()).unwrap(), None);
    }
    drop(database);
    let mut reopened = Database::open(&root).unwrap();
    reopened.compact(&[], 3).unwrap().unwrap();
    reopened
        .write(
            &batch(vec![put("new", "admitted")]),
            Durability::Authoritative,
        )
        .unwrap();
    assert_eq!(
        reopened.get(b"hot", reopened.snapshot()).unwrap(),
        Some(b"second".to_vec())
    );
    assert_eq!(
        reopened.get(b"new", reopened.snapshot()).unwrap(),
        Some(b"admitted".to_vec())
    );
}

#[test]
fn pinned_transaction_reads_survive_compaction_garbage_collect_cycles() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create_with_options(
        &root,
        DatabaseOptions {
            compaction: CompactionPolicy {
                l0_compaction_trigger: 2,
                ..CompactionPolicy::default()
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap();

    database
        .write(
            &batch(vec![put("shared", "old")]),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(10).unwrap();

    let historical = database.begin_transaction().unwrap();
    database
        .write(
            &batch(vec![put("shared", "new")]),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(20).unwrap();
    database
        .write(
            &batch(vec![put("other", "fresh")]),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(30).unwrap();

    let before_compaction = database
        .manifest()
        .segments
        .iter()
        .map(|segment| segment.id.clone())
        .collect::<BTreeSet<_>>();
    assert!(before_compaction.len() >= 2);

    let outcome = database.compact(&[], 40).unwrap().unwrap();
    assert!(outcome.input_segments >= 2);
    assert!(outcome.output_segments >= 1);

    let after_compaction = database
        .manifest()
        .segments
        .iter()
        .map(|segment| segment.id.clone())
        .collect::<BTreeSet<_>>();
    assert!(!after_compaction.is_empty());

    assert_eq!(
        historical.get(&database, b"shared").unwrap(),
        Some(b"old".to_vec())
    );
    let gc = database.garbage_collect().unwrap();
    let removed_expected = before_compaction
        .difference(&after_compaction)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|segment| segment.as_str())
        .collect::<Vec<_>>();
    assert!(
        removed_expected
            .iter()
            .all(|segment| gc.removed_segments.contains(&format!("{segment}.seg"))),
        "expected every pre-compaction segment not in current manifest to be deleted"
    );
    assert_eq!(
        historical.get(&database, b"shared").unwrap(),
        Some(b"old".to_vec())
    );
    historical.rollback().unwrap();
}

#[test]
fn pinned_projected_stream_survives_flush_compaction_and_gc_until_drop() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create_with_options(
        &root,
        DatabaseOptions {
            compaction: CompactionPolicy {
                l0_compaction_trigger: 2,
                ..CompactionPolicy::default()
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    database
        .write(
            &batch(vec![put("shared", "old"), put("removed", "present")]),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(10).unwrap();
    let pinned_manifest = database.manifest().digest.clone();
    let pinned_segments = database
        .manifest()
        .segments
        .iter()
        .map(|segment| segment.id.clone())
        .collect::<BTreeSet<_>>();
    let mut stream = database
        .begin_projected_read(ProjectedReadRequest {
            ranges: vec![ProjectedReadRange::all()],
            projection: ProjectedReadProjection::KeyValue,
            snapshot: database.snapshot(),
            budget: ProjectedReadBudget {
                max_batch_rows: 1,
                ..ProjectedReadBudget::default()
            },
        })
        .unwrap();

    database
        .write(
            &batch(vec![put("shared", "new"), delete("removed")]),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(20).unwrap();
    database
        .write(
            &batch(vec![put("other", "fresh")]),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(30).unwrap();
    database.compact(&[], 40).unwrap().unwrap();

    let while_pinned = database.garbage_collect().unwrap();
    assert!(while_pinned.retained_manifests.contains(&pinned_manifest));
    for segment in &pinned_segments {
        assert!(while_pinned.retained_segments.contains(segment));
        assert!(root
            .join("segments")
            .join(format!("{segment}.seg"))
            .exists());
    }

    let mut rows = Vec::new();
    while let Some(batch) = stream.next_batch().unwrap() {
        for index in 0..batch.len() {
            rows.push((
                batch.key(index).unwrap().to_vec(),
                batch.value(index).unwrap().to_vec(),
            ));
        }
    }
    assert_eq!(
        rows,
        vec![
            (b"removed".to_vec(), b"present".to_vec()),
            (b"shared".to_vec(), b"old".to_vec()),
        ]
    );
    drop(stream);

    let after_drop = database.garbage_collect().unwrap();
    assert!(after_drop
        .removed_manifests
        .contains(&format!("{pinned_manifest}.json")));
    assert!(pinned_segments.iter().any(|segment| {
        after_drop
            .removed_segments
            .contains(&format!("{segment}.seg"))
    }));
}

#[test]
fn compaction_retains_only_versions_visible_to_protected_snapshots() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create(&root).unwrap();

    database
        .write(
            &batch(vec![put("key", "v1"), put("removed", "present")]),
            Durability::Authoritative,
        )
        .unwrap();
    let first = database.snapshot();
    database.flush_memtable(10).unwrap();
    database
        .write(
            &batch(vec![put("key", "v2"), delete("removed")]),
            Durability::Authoritative,
        )
        .unwrap();
    let second = database.snapshot();
    database.flush_memtable(20).unwrap();
    database
        .write(&batch(vec![put("key", "v3")]), Durability::Authoritative)
        .unwrap();
    let current = database.snapshot();

    let outcome = database.compact(&[first, second], 30).unwrap().unwrap();
    assert_eq!(outcome.input_segments, 3);
    assert_eq!(outcome.output_segments, 1);
    assert_eq!(outcome.input_versions, 5);
    assert_eq!(outcome.output_versions, 5);
    assert_eq!(outcome.protected_sequences, vec![2, 4, 5]);
    assert_eq!(
        database.get(b"key", first).unwrap().as_deref(),
        Some(b"v1".as_slice())
    );
    assert_eq!(
        database.get(b"key", second).unwrap().as_deref(),
        Some(b"v2".as_slice())
    );
    assert_eq!(
        database.get(b"key", current).unwrap().as_deref(),
        Some(b"v3".as_slice())
    );
    assert_eq!(
        database.get(b"removed", first).unwrap().as_deref(),
        Some(b"present".as_slice())
    );
    assert_eq!(database.get(b"removed", second).unwrap().as_deref(), None);

    drop(database);
    let reopened = Database::open(&root).unwrap();
    assert_eq!(
        reopened.get(b"key", first).unwrap().as_deref(),
        Some(b"v1".as_slice())
    );
    assert_eq!(
        reopened.get(b"key", second).unwrap().as_deref(),
        Some(b"v2".as_slice())
    );
    assert_eq!(
        reopened.get(b"key", current).unwrap().as_deref(),
        Some(b"v3".as_slice())
    );
    assert_eq!(reopened.get(b"removed", second).unwrap().as_deref(), None);
}

#[test]
fn compaction_prunes_unprotected_history_and_obsolete_tombstones() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create(&root).unwrap();
    database
        .write(&batch(vec![put("key", "old")]), Durability::Authoritative)
        .unwrap();
    let old = database.snapshot();
    database.flush_memtable(10).unwrap();
    database
        .write(
            &batch(vec![put("key", "new"), put("gone", "value")]),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(20).unwrap();
    database
        .write(&batch(vec![delete("gone")]), Durability::Authoritative)
        .unwrap();
    let current = database.snapshot();

    let outcome = database.compact(&[], 30).unwrap().unwrap();
    assert_eq!(outcome.output_versions, 1);
    assert_eq!(database.get(b"key", old).unwrap().as_deref(), None);
    assert_eq!(
        database.get(b"key", current).unwrap().as_deref(),
        Some(b"new".as_slice())
    );
    assert_eq!(database.get(b"gone", current).unwrap().as_deref(), None);
    assert_eq!(database.manifest().segments[0].entries, 1);
}

#[test]
fn garbage_collection_preserves_current_and_checkpoint_reachability() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create(&root).unwrap();
    database
        .write(&batch(vec![put("key", "v1")]), Durability::Authoritative)
        .unwrap();
    database.flush_memtable(10).unwrap();
    let checkpoint = database.checkpoint("keep-first", 11).unwrap();
    let first_segment = database.manifest().segments[0].id.clone();

    database
        .write(&batch(vec![put("key", "v2")]), Durability::Authoritative)
        .unwrap();
    database.flush_memtable(20).unwrap();
    database.compact(&[], 30).unwrap();
    let current_segment = database.manifest().segments[0].id.clone();

    let first = database.garbage_collect().unwrap();
    assert!(first.retained_manifests.contains(&checkpoint.manifest));
    assert!(first
        .retained_manifests
        .contains(&database.manifest().digest));
    assert!(first.retained_segments.contains(&first_segment));
    assert!(first.retained_segments.contains(&current_segment));
    assert!(root
        .join("segments")
        .join(format!("{first_segment}.seg"))
        .exists());
    assert!(root
        .join("segments")
        .join(format!("{current_segment}.seg"))
        .exists());
    // Normal pre-write maintenance may already have removed the first WAL.
    // Prove physical reachability rather than which collector call did it.
    assert!(!root.join("wal").join("00000000000000000001.wal").exists());
    let physical_wals = std::fs::read_dir(root.join("wal"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(physical_wals, first.retained_wals.iter().cloned().collect());

    assert!(database.release_checkpoint("keep-first").unwrap());
    let second = database.garbage_collect().unwrap();
    assert!(second
        .removed_segments
        .contains(&format!("{first_segment}.seg")));
    assert!(second
        .removed_manifests
        .contains(&format!("{}.json", checkpoint.manifest)));
    assert!(!root
        .join("segments")
        .join(format!("{first_segment}.seg"))
        .exists());
    assert!(root
        .join("segments")
        .join(format!("{current_segment}.seg"))
        .exists());
}

#[test]
fn explicit_compaction_promotes_a_single_segment() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create(&root).unwrap();
    database
        .write(&batch(vec![put("key", "value")]), Durability::Authoritative)
        .unwrap();
    let snapshot = database.snapshot();
    database.flush_memtable(10).unwrap();

    let outcome = database.compact(&[snapshot], 20).unwrap().unwrap();

    assert_eq!(outcome.input_segments, 1);
    assert_eq!(outcome.output_segments, 1);
    assert_eq!(outcome.source_level, 0);
    assert_eq!(outcome.target_level, 1);
    assert_eq!(database.manifest().segments[0].level, 1);
    assert_eq!(
        database.get(b"key", snapshot).unwrap().as_deref(),
        Some(b"value".as_slice())
    );
}

#[test]
fn leveled_compaction_streams_into_key_partitioned_outputs() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create_with_options(
        &root,
        DatabaseOptions {
            compaction: CompactionPolicy {
                target_segment_bytes: 512,
                ..CompactionPolicy::default()
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    for phase in 0..3 {
        let operations = (0..32)
            .map(|index| {
                put(
                    &format!("key:{index:04}"),
                    &format!("phase:{phase}:{}", "x".repeat(48)),
                )
            })
            .collect();
        database
            .write(&batch(operations), Durability::Authoritative)
            .unwrap();
        database.flush_memtable(phase + 1).unwrap();
    }
    let snapshot = database.snapshot();
    let outcome = database.compact(&[], 10).unwrap().unwrap();
    assert_eq!(outcome.source_level, 0);
    assert_eq!(outcome.target_level, 1);
    assert!(outcome.output_segments > 1);
    assert!(outcome.peak_buffer_bytes <= 512);
    assert_eq!(outcome.input_versions, 96);
    assert_eq!(outcome.output_versions, 32);
    assert!(database
        .manifest()
        .segments
        .iter()
        .all(|segment| segment.level == 1));
    for pair in database.manifest().segments.windows(2) {
        assert!(pair[0].last_key < pair[1].first_key);
    }
    for index in 0..32 {
        let key = format!("key:{index:04}");
        assert_eq!(
            database.get(key.as_bytes(), snapshot).unwrap(),
            Some(format!("phase:2:{}", "x".repeat(48)).into_bytes())
        );
    }
}

#[test]
fn automatic_leveled_maintenance_preserves_untracked_snapshot_history() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create_with_options(
        &root,
        DatabaseOptions {
            maintenance: rrd_lsm::MaintenancePolicy {
                wal_payload_max_bytes: usize::MAX,
                memtable_max_versions: 1,
            },
            compaction: CompactionPolicy {
                l0_compaction_trigger: 2,
                ..CompactionPolicy::default()
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    database
        .write(&batch(vec![put("key", "v1")]), Durability::Authoritative)
        .unwrap();
    let old = database.snapshot();
    database
        .write(
            &batch(vec![put("other", "value")]),
            Durability::Authoritative,
        )
        .unwrap();
    database
        .write(&batch(vec![put("key", "v2")]), Durability::Authoritative)
        .unwrap();

    assert_eq!(database.maintenance_stats().automatic_compactions, 1);
    assert_eq!(
        database.get(b"key", old).unwrap().as_deref(),
        Some(b"v1".as_slice())
    );
    assert_eq!(
        database
            .get(b"key", database.snapshot())
            .unwrap()
            .as_deref(),
        Some(b"v2".as_slice())
    );
}
