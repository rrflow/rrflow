use rrd_lsm::{Database, DatabaseOptions, Error, ReadOnlyDatabase};
#[test]
fn zero_source_page_budget_rejects_before_any_store_creation_or_open() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("estate");
    let options = DatabaseOptions {
        source_page_bytes: 0,
        ..Default::default()
    };
    assert!(matches!(
        Database::create_with_options(&root, options),
        Err(Error::InvalidConfiguration(_))
    ));
    assert!(!root.exists());
    drop(Database::create(&root).unwrap());
    assert!(matches!(
        Database::open_with_options(&root, options),
        Err(Error::InvalidConfiguration(_))
    ));
    assert!(matches!(
        ReadOnlyDatabase::open_with_options(&root, options),
        Err(Error::InvalidConfiguration(_))
    ));
    assert_eq!(
        Database::open(&root)
            .unwrap()
            .source_page_memory()
            .stats()
            .reserved_bytes,
        0
    );
}

#[test]
fn source_page_maintenance_denial_preserves_acknowledged_data_and_reopen_identity() {
    use rrd_lsm::{
        Durability, Mutation, SegmentCompressionPolicy, SegmentIoMode, SegmentIoPolicy, WriteBatch,
    };
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("estate");
    let options = DatabaseOptions {
        source_page_bytes: 128,
        page_cache_bytes: 0,
        segment_compression: SegmentCompressionPolicy::None,
        segment_io: SegmentIoPolicy {
            mode: SegmentIoMode::Bounded,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut db = Database::create_with_options(&root, options).unwrap();
    let value = vec![42; 512];
    db.write_owned(
        WriteBatch::new(vec![Mutation::Put {
            key: b"ack".to_vec(),
            value: value.clone(),
        }])
        .unwrap(),
        Durability::Authoritative,
    )
    .unwrap();
    let stamp = db.snapshot();
    let manifest = db.manifest().clone();
    let pool = db.source_page_memory();
    assert!(matches!(
        db.flush_memtable(1),
        Err(Error::SourcePageLimit { .. })
    ));
    assert_eq!(db.manifest(), &manifest);
    assert_eq!(db.get(b"ack", stamp).unwrap(), Some(value.clone()));
    assert_eq!(db.snapshot(), stamp);
    assert_eq!(pool.stats().reserved_bytes, 0);
    assert!(pool.stats().peak_reserved_bytes <= 128);
    drop(db);
    let mut reopened = Database::open_with_options(
        &root,
        DatabaseOptions {
            source_page_bytes: 2048,
            ..options
        },
    )
    .unwrap();
    assert_eq!(reopened.snapshot(), stamp);
    reopened.flush_memtable(2).unwrap();
    assert_eq!(reopened.get(b"ack", stamp).unwrap(), Some(value));
    assert_eq!(pool.stats().reserved_bytes, 0);
    assert_eq!(reopened.source_page_memory().stats().limit_bytes, 2048);
}
