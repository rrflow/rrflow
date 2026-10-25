use rrd_lsm::{Database, DatabaseOptions, Durability, Mutation, SegmentIoMode, WriteBatch};

#[test]
fn evicted_sources_remain_readable_through_compaction_gc_and_database_drop() {
    use rrd_lsm::{
        ProjectedReadBudget, ProjectedReadProjection, ProjectedReadRange, ProjectedReadRequest,
    };
    let directory = tempfile::tempdir().unwrap();
    let mut options = DatabaseOptions::default();
    options.segment_io.mode = SegmentIoMode::Bounded;
    options.segment_io.max_open_files = 1;
    options.page_cache_bytes = 0;
    let mut database =
        Database::create_with_options(&directory.path().join("pinned"), options).unwrap();
    for index in 0..12_u8 {
        database
            .write(
                &WriteBatch::new(vec![Mutation::Put {
                    key: vec![index],
                    value: vec![index; 4096],
                }])
                .unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(u64::from(index) + 1).unwrap();
    }
    let mut stream = database
        .begin_projected_read(ProjectedReadRequest {
            ranges: vec![ProjectedReadRange::all()],
            projection: ProjectedReadProjection::KeyValue,
            snapshot: database.snapshot(),
            budget: ProjectedReadBudget {
                max_batch_rows: 1,
                ..Default::default()
            },
        })
        .unwrap();
    database.compact(&[], 100).unwrap();
    database.garbage_collect().unwrap();
    assert_eq!(database.segment_io_stats().peak_open_files, 1);
    drop(database);
    for index in 0..12_u8 {
        let batch = stream.next_batch().unwrap().unwrap();
        assert_eq!(batch.key(0), Some([index].as_slice()));
        assert_eq!(batch.value(0), Some(vec![index; 4096].as_slice()));
    }
    assert!(stream.next_batch().unwrap().is_none());
}

/// Native regression: old PageSource ownership exhausts this child at <384
/// segments. Keep RLIMIT changes isolated from the test runner and other tests.
#[test]
fn segment_estate_exceeds_native_descriptor_ceiling_and_reopens() {
    #[cfg(unix)]
    if std::env::var_os("RRFLOW_SEGMENT_DESCRIPTOR_CHILD").is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "segment_estate_exceeds_native_descriptor_ceiling_and_reopens",
                "--nocapture",
            ])
            .env("RRFLOW_SEGMENT_DESCRIPTOR_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "child failed: {}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    #[cfg(unix)]
    {
        let mut limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        assert_eq!(
            unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
            0
        );
        limit.rlim_cur = limit.rlim_cur.min(256);
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) }, 0);
    }
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("estate");
    let mut options = DatabaseOptions::default();
    options.segment_io.mode = SegmentIoMode::Bounded;
    options.segment_io.allow_fallback = false;
    options.page_cache_bytes = 0;
    options.compaction.l0_compaction_trigger = 1024;
    options.compaction_limits.l0_segments = 1024;
    let mut database = Database::create_with_options(&root, options).unwrap();
    for index in 0..384_u64 {
        database
            .write(
                &WriteBatch::new(vec![Mutation::Put {
                    key: index.to_be_bytes().to_vec(),
                    value: index.to_le_bytes().to_vec(),
                }])
                .unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        database.flush_memtable(index + 1).unwrap();
    }
    for index in 0..384_u64 {
        assert_eq!(
            database
                .get(&index.to_be_bytes(), database.snapshot())
                .unwrap(),
            Some(index.to_le_bytes().to_vec())
        );
    }
    assert!(database.segment_io_stats().handle_evictions > 0);
    assert!(database.segment_io_stats().peak_open_files <= options.segment_io.max_open_files);
    drop(database);
    let reopened = Database::open_with_options(&root, options).unwrap();
    for index in (0..384_u64).rev() {
        assert_eq!(
            reopened
                .get(&index.to_be_bytes(), reopened.snapshot())
                .unwrap(),
            Some(index.to_le_bytes().to_vec())
        );
    }
    assert!(reopened.segment_io_stats().handle_reopens > 0);
    assert!(reopened.segment_io_stats().peak_open_files <= options.segment_io.max_open_files);
}
