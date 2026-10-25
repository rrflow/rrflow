use rrd_lsm::{
    Database, DatabaseOptions, Durability, Mutation, SegmentCompressionPolicy, SegmentIoMode,
    SegmentIoPolicy, Snapshot, WriteBatch,
};

fn write(database: &mut Database, value: Option<Vec<u8>>) -> Snapshot {
    let key = b"shared".to_vec();
    let operation = match value {
        Some(value) => Mutation::Put { key, value },
        None => Mutation::Delete { key },
    };
    database
        .write_owned(
            WriteBatch::new(vec![operation]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.snapshot()
}

fn check(database: &Database, snapshot: Snapshot, value: &Option<Vec<u8>>) {
    assert_eq!(&database.get(b"shared", snapshot).unwrap(), value);
    assert_eq!(
        database
            .get_many(
                &[
                    b"z-missing".to_vec(),
                    b"shared".to_vec(),
                    b"a-missing".to_vec(),
                    b"shared".to_vec()
                ],
                snapshot,
            )
            .unwrap(),
        vec![None, value.clone(), None, value.clone()]
    );
    assert!(database.get_many(&[], snapshot).unwrap().is_empty());
    // Selection must not retain loaded source pages after the call.
    assert_eq!(database.source_page_memory().stats().reserved_bytes, 0);
}

#[test]
fn immutable_point_reads_materialize_only_the_winning_version() {
    for mode in [SegmentIoMode::Bounded, SegmentIoMode::Mmap] {
        for compression in [
            SegmentCompressionPolicy::None,
            SegmentCompressionPolicy::default(),
        ] {
            for tombstone in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let root = temp.path().join("estate");
                let options = DatabaseOptions {
                    page_cache_bytes: 0,
                    segment_compression: compression,
                    segment_io: SegmentIoPolicy {
                        mode,
                        allow_fallback: false,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let mut database = Database::create_with_options(&root, options).unwrap();
                let empty = database.snapshot();
                let mut history = vec![(empty, None)];
                for value in [
                    Some(vec![7; 64 * 1024]),
                    Some(vec![9; 64 * 1024]),
                    if tombstone {
                        None
                    } else {
                        Some(b"winner".to_vec())
                    },
                ] {
                    let snapshot = write(&mut database, value.clone());
                    database.flush_memtable(snapshot.sequence).unwrap();
                    history.push((snapshot, value));
                }
                assert_eq!(database.manifest().segments.len(), 3);
                drop(database);
                // Full reopen authentication is outside the point-read measurement.
                let mut database = Database::open_with_options(&root, options).unwrap();
                let (current, expected) = history.last().unwrap();
                for batched in [false, true] {
                    let before = database.page_cache_stats();
                    if batched {
                        assert_eq!(
                            database.get_many(&[b"shared".to_vec()], *current).unwrap(),
                            vec![expected.clone()]
                        );
                    } else {
                        assert_eq!(&database.get(b"shared", *current).unwrap(), expected);
                    }
                    let after = database.page_cache_stats();
                    let loads = after.loads - before.loads;
                    let bytes = after.bytes_read - before.bytes_read;
                    println!("point_selection mode={mode:?} compression={compression:?} tombstone={tombstone} batched={batched} loads={loads} bytes={bytes}");
                    // Three key spines (three pages each), plus only the winner's
                    // validity and, for a value, its offsets/data. Superseded
                    // payloads must not be loaded even when compression is cheap.
                    assert_eq!(loads, if tombstone { 10 } else { 12 });
                    assert!(bytes < 1024, "read discarded payloads: {bytes} bytes");
                    assert_eq!(database.page_cache_stats().resident_bytes, 0);
                    assert_eq!(database.source_page_memory().stats().reserved_bytes, 0);
                }
                for (snapshot, value) in &history {
                    check(&database, *snapshot, value);
                }

                // The active memtable must dominate immutable history, including
                // deletion and resurrection, without changing older snapshots.
                for value in [None, Some(b"resurrected".to_vec())] {
                    let snapshot = write(&mut database, value.clone());
                    history.push((snapshot, value));
                    for (snapshot, value) in &history {
                        check(&database, *snapshot, value);
                    }
                }
                let current = database.snapshot();
                database.flush_memtable(current.sequence).unwrap();
                let protected = history
                    .iter()
                    .map(|(snapshot, _)| *snapshot)
                    .collect::<Vec<_>>();
                assert!(database
                    .compact(&protected, current.sequence)
                    .unwrap()
                    .is_some());
                for (snapshot, value) in &history {
                    check(&database, *snapshot, value);
                }
                drop(database);
                let database = Database::open_with_options(&root, options).unwrap();
                for (snapshot, value) in &history {
                    check(&database, *snapshot, value);
                }
            }
        }
    }
}

#[test]
fn batched_selection_preserves_segment_locality_with_a_small_cache() {
    check_batch_locality(false);
    check_batch_locality(true);
}

fn check_batch_locality(disjoint: bool) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("estate");
    let options = DatabaseOptions {
        page_cache_bytes: if disjoint { 256 } else { 1024 },
        page_cache_policy: rrd_lsm::PageCachePolicy::ExactLru,
        segment_compression: SegmentCompressionPolicy::None,
        ..Default::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    let keys = (0..16)
        .map(|i| format!("key-{i:03}").into_bytes())
        .collect::<Vec<_>>();
    for generation in 0..4u8 {
        database
            .write_owned(
                WriteBatch::new(
                    keys.iter()
                        .enumerate()
                        .filter(|(index, _)| !disjoint || index % 4 == usize::from(generation))
                        .map(|(_, key)| Mutation::Put {
                            key: key.clone(),
                            value: vec![generation; 8],
                        })
                        .collect(),
                )
                .unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database
            .flush_memtable(database.snapshot().sequence)
            .unwrap();
    }
    drop(database);
    let database = Database::open_with_options(&root, options).unwrap();
    let before = database.page_cache_stats();
    assert_eq!(
        database.get_many(&keys, database.snapshot()).unwrap(),
        (0..16)
            .map(|index| Some(vec![if disjoint { index % 4 } else { 3 }; 8]))
            .collect::<Vec<_>>()
    );
    let after = database.page_cache_stats();
    println!(
        "batch_locality disjoint={disjoint} loads={} bytes={}",
        after.loads - before.loads,
        after.bytes_read - before.bytes_read
    );
    // The cache fits one segment's row group, not all four. Selection must walk
    // keys within a segment before moving on; then read the winner's values.
    assert!(after.loads - before.loads <= if disjoint { 24 } else { 15 });
    assert!(after.resident_bytes <= options.page_cache_bytes);
}
