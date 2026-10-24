use rrd_lsm::{
    CompactionLimits, CompactionPolicy, Database, DatabaseOptions, Durability, Error, FailureMode,
    FlushBoundary, MaintenanceResource, Mutation, Snapshot, WriteBatch,
};

fn options() -> DatabaseOptions {
    DatabaseOptions {
        compaction: CompactionPolicy {
            target_segment_bytes: 512,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn populate(database: &mut Database) -> Vec<Snapshot> {
    (0..3)
        .map(|phase| {
            let operations = (0..12)
                .map(|key| Mutation::Put {
                    key: format!("key-{key:02}").into_bytes(),
                    value: vec![phase; 64],
                })
                .collect();
            database
                .write_owned(
                    WriteBatch::new(operations).unwrap(),
                    Durability::Authoritative,
                )
                .unwrap();
            database.snapshot()
        })
        .collect()
}

fn verify(database: &Database, snapshots: &[Snapshot]) {
    for (phase, snapshot) in snapshots.iter().enumerate() {
        for key in 0..12 {
            assert_eq!(
                database
                    .get(format!("key-{key:02}").as_bytes(), *snapshot)
                    .unwrap(),
                Some(vec![phase as u8; 64])
            );
        }
    }
}

#[test]
fn flush_partitions_without_splitting_version_chains_and_reopens() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let mut database = Database::create_with_options(&root, options()).unwrap();
    let snapshots = populate(&mut database);
    let before = database.manifest().clone();
    let manifest = database.flush_memtable(1).unwrap().unwrap();
    assert_eq!(manifest.generation, before.generation + 1);
    assert_eq!(manifest.segments.len(), 12);
    assert!(manifest.segments.iter().all(|segment| segment.entries == 3));
    for pair in manifest.segments.windows(2) {
        assert!(pair[0].last_key < pair[1].first_key);
    }
    assert_eq!(database.memtable().version_count(), 0);
    verify(&database, &snapshots);
    drop(database);
    let reopened = Database::open_with_options(&root, options()).unwrap();
    assert_eq!(reopened.manifest(), &manifest);
    verify(&reopened, &snapshots);
}

#[test]
fn partition_admission_has_no_file_or_publication_effect() {
    for resource in [
        MaintenanceResource::OutputSegments,
        MaintenanceResource::L0Segments,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("database");
        let mut config = options();
        config.compaction_limits = match resource {
            MaintenanceResource::OutputSegments => CompactionLimits {
                output_segments: 4,
                ..Default::default()
            },
            MaintenanceResource::L0Segments => CompactionLimits {
                l0_segments: 8,
                ..Default::default()
            },
            _ => unreachable!(),
        };
        let mut database = Database::create_with_options(&root, config).unwrap();
        let snapshots = populate(&mut database);
        let manifest = database.manifest().clone();
        let wal_bytes = database.wal_payload_bytes();
        let files =
            || ["segments", "wal"].map(|path| std::fs::read_dir(root.join(path)).unwrap().count());
        let before = files();
        assert!(
            matches!(database.flush_memtable(1), Err(Error::MaintenanceLimit { resource: actual, .. }) if actual == resource)
        );
        assert_eq!(database.manifest(), &manifest);
        assert_eq!(database.wal_payload_bytes(), wal_bytes);
        assert_eq!(database.memtable().version_count(), 36);
        assert_eq!(files(), before);
        verify(&database, &snapshots);
        drop(database);
        let mut reopened = Database::open_with_options(&root, options()).unwrap();
        verify(&reopened, &snapshots);
        assert_eq!(
            reopened.flush_memtable(2).unwrap().unwrap().segments.len(),
            12
        );
    }
}

#[test]
fn partitioned_flush_recovers_every_acknowledgement_at_each_boundary() {
    for boundary in [
        FlushBoundary::WalSynced,
        FlushBoundary::SegmentSynced,
        FlushBoundary::SuccessorWalSynced,
        FlushBoundary::ManifestPublished,
    ] {
        for mode in [FailureMode::Crash, FailureMode::StorageFull] {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path().join("database");
            let mut database = Database::create_with_options(&root, options()).unwrap();
            let snapshots = populate(&mut database);
            let before = database.manifest().clone();
            assert!(matches!(
                database.flush_memtable_with_failure(1, boundary, mode),
                Err(Error::InjectedFailure { .. })
            ));
            let published = boundary == FlushBoundary::ManifestPublished;
            assert_eq!(
                database.manifest().generation,
                before.generation + u64::from(published)
            );
            assert_eq!(
                database.manifest().segments.len(),
                if published { 12 } else { 0 }
            );
            verify(&database, &snapshots);
            drop(database);
            let mut reopened = Database::open_with_options(&root, options()).unwrap();
            verify(&reopened, &snapshots);
            reopened.flush_memtable(2).unwrap();
            assert_eq!(reopened.manifest().segments.len(), 12);
            reopened.garbage_collect().unwrap();
            assert_eq!(
                std::fs::read_dir(root.join("segments")).unwrap().count(),
                12
            );
            verify(&reopened, &snapshots);
        }
    }
}

#[test]
fn repeated_partitioned_flushes_make_progress_with_bounded_compaction_inputs() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let config = DatabaseOptions {
        compaction: CompactionPolicy {
            l0_compaction_trigger: 4,
            max_input_segments: 4,
            target_segment_bytes: 512,
            ..Default::default()
        },
        compaction_limits: CompactionLimits {
            l0_segments: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut database = Database::create_with_options(&root, config).unwrap();
    let snapshots = populate(&mut database);
    let original = snapshots[0];
    database.flush_memtable(1).unwrap();
    let mut maintenance = 0;
    for phase in 3..35u8 {
        let mutations = (0..12)
            .map(|key| Mutation::Put {
                key: format!("key-{key:02}").into_bytes(),
                value: vec![phase; 64],
            })
            .collect();
        database
            .write_owned(
                WriteBatch::new(mutations).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(u64::from(phase)).unwrap();
        // A finite offered maintenance allowance: no retry-until-green loop.
        for _ in 0..16 {
            if let Some(outcome) = database.compact(&[original], u64::from(phase)).unwrap() {
                assert!(outcome.input_segments <= 4);
                maintenance += 1;
            } else {
                break;
            }
        }
        assert!(database.l0_segment_count() < 32);
        for key in 0..12 {
            let key = format!("key-{key:02}");
            assert_eq!(
                database.get(key.as_bytes(), original).unwrap(),
                Some(vec![0; 64])
            );
            assert_eq!(
                database.get(key.as_bytes(), database.snapshot()).unwrap(),
                Some(vec![phase; 64])
            );
        }
        database.garbage_collect().unwrap();
    }
    assert!(maintenance > 32);
    drop(database);
    let reopened = Database::open_with_options(&root, config).unwrap();
    for key in 0..12 {
        let key = format!("key-{key:02}");
        assert_eq!(
            reopened.get(key.as_bytes(), original).unwrap(),
            Some(vec![0; 64])
        );
        assert_eq!(
            reopened.get(key.as_bytes(), reopened.snapshot()).unwrap(),
            Some(vec![34; 64])
        );
    }
}
