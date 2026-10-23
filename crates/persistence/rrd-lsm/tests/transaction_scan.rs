use rrd_lsm::{
    Database, DatabaseOptions, Durability, Error, ProjectedReadResource, SegmentRowGroupBudget,
};

fn key(index: usize) -> Vec<u8> {
    format!("row:{index:04}").into_bytes()
}
fn value(index: usize) -> Vec<u8> {
    format!("original-{index:04}").into_bytes()
}

#[test]
fn limited_scan_preserves_old_snapshot_and_overlay_without_reading_distant_pages() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("scan");
    let mut database = Database::create_with_options(
        &root,
        DatabaseOptions {
            page_cache_bytes: 0,
            segment_row_group_budget: SegmentRowGroupBudget {
                max_rows: 1,
                target_bytes: 128,
            },
            ..Default::default()
        },
    )
    .unwrap();
    let mut seed = database.begin_transaction().unwrap();
    for index in 0..1024 {
        seed.put(key(index), value(index)).unwrap();
    }
    database
        .commit_transaction(seed, Durability::Authoritative)
        .unwrap();
    database.flush_memtable(1).unwrap();
    let mut historical = database.begin_transaction().unwrap();
    let mut update = database.begin_transaction().unwrap();
    update.put(key(0), b"newer".to_vec()).unwrap();
    update.delete(key(1)).unwrap();
    database
        .commit_transaction(update, Durability::Authoritative)
        .unwrap();
    database.flush_memtable(2).unwrap();
    database.compact(&[], 3).unwrap();
    historical.delete(key(0)).unwrap();
    historical.put(key(1), b"overlay".to_vec()).unwrap();
    historical
        .put(b"row:0000:extra".to_vec(), b"inserted".to_vec())
        .unwrap();
    let before = database.page_cache_stats();
    let rows = historical.scan(&database, b"row:", b"row;", 3).unwrap();
    assert_eq!(
        rows,
        vec![
            (b"row:0000:extra".to_vec(), b"inserted".to_vec()),
            (key(1), b"overlay".to_vec()),
            (key(2), value(2)),
        ]
    );
    let after = database.page_cache_stats();
    assert!(
        after.loads - before.loads < 128,
        "limited scan visited distant row groups"
    );
    assert_eq!(database.read_view_ownership().unwrap(), (0, 0));
    assert_eq!(database.projected_output_memory().stats().reserved_bytes, 0);
    assert!(matches!(
        database.commit_transaction(historical, Durability::Authoritative),
        Err(Error::TransactionConflict { .. })
    ));
    drop(database);
    let reopened = Database::open(&root).unwrap();
    let transaction = reopened.begin_transaction().unwrap();
    assert_eq!(
        transaction.scan(&reopened, b"row:", b"row;", 2).unwrap(),
        vec![(key(0), b"newer".to_vec()), (key(2), value(2))]
    );
}

#[test]
fn every_limit_matches_independent_ordered_state_across_pending_shadows() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = Database::create(&directory.path().join("overlay")).unwrap();
    let mut seed = database.begin_transaction().unwrap();
    let mut expected = std::collections::BTreeMap::new();
    for index in (0..64).step_by(2) {
        seed.put(key(index), value(index)).unwrap();
        expected.insert(key(index), value(index));
    }
    database
        .commit_transaction(seed, Durability::Authoritative)
        .unwrap();
    database.flush_memtable(1).unwrap();
    let mut transaction = database.begin_transaction().unwrap();
    for index in 0..64 {
        if index % 3 == 0 {
            transaction.delete(key(index)).unwrap();
            expected.remove(&key(index));
        } else {
            let changed = format!("pending-{index}").into_bytes();
            transaction.put(key(index), changed.clone()).unwrap();
            expected.insert(key(index), changed);
        }
    }
    for limit in [1, 2, 3, 8, 17, 64, usize::MAX] {
        let expected = expected
            .iter()
            .take(limit)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            transaction
                .scan(&database, b"row:", b"row;", limit)
                .unwrap(),
            expected
        );
        assert_eq!(database.read_view_ownership().unwrap(), (0, 0));
        assert_eq!(database.projected_output_memory().stats().reserved_bytes, 0);
    }
}

#[test]
fn oversized_pending_output_is_denied_before_copy_and_releases_source_owners() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::create_with_options(
        &directory.path().join("output"),
        DatabaseOptions {
            projected_output_bytes: 128,
            ..Default::default()
        },
    )
    .unwrap();
    let mut transaction = database.begin_transaction().unwrap();
    transaction.put(b"a".to_vec(), vec![7; 256]).unwrap();
    let owners = database.transaction_ownership().unwrap();
    assert!(matches!(
        transaction.scan(&database, b"a", b"z", 1),
        Err(Error::ProjectedReadLimit {
            resource: ProjectedReadResource::OutputBufferBytes,
            ..
        })
    ));
    assert_eq!(database.read_view_ownership().unwrap(), (0, 0));
    assert_eq!(database.projected_output_memory().stats().reserved_bytes, 0);
    assert_eq!(database.transaction_ownership().unwrap(), owners);
    assert_eq!(
        transaction.get(&database, b"a").unwrap(),
        Some(vec![7; 256])
    );
    transaction.put(b"a".to_vec(), b"small".to_vec()).unwrap();
    assert_eq!(
        transaction.scan(&database, b"a", b"z", 1).unwrap(),
        vec![(b"a".to_vec(), b"small".to_vec())]
    );
}
