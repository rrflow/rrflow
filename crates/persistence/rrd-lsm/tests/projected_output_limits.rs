use rrd_lsm::{
    Database, DatabaseOptions, Durability, Error, Mutation, ProjectedReadBudget,
    ProjectedReadOutcome, ProjectedReadProjection, ProjectedReadRange, ProjectedReadRequest,
    ProjectedReadResource, ProjectedReadStream, SegmentCompressionPolicy, SegmentIoMode,
    SegmentIoPolicy, WriteBatch,
};

fn read(database: &Database, projection: ProjectedReadProjection) -> ProjectedReadStream {
    database
        .begin_projected_read(ProjectedReadRequest {
            ranges: vec![ProjectedReadRange::all()],
            projection,
            snapshot: database.snapshot(),
            budget: ProjectedReadBudget {
                max_batch_rows: 1,
                ..ProjectedReadBudget::default()
            },
        })
        .unwrap()
}

fn put(database: &mut Database, key: &[u8], value: &[u8]) {
    database
        .write_owned(
            WriteBatch::new(vec![Mutation::Put {
                key: key.to_vec(),
                value: value.to_vec(),
            }])
            .unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
}

fn denied(stream: &mut ProjectedReadStream, limit: usize, observed: usize) {
    assert!(matches!(
        stream.next_batch(),
        Err(Error::ProjectedReadLimit {
            resource: ProjectedReadResource::RetainedOutputBytes,
            limit: actual_limit,
            observed: actual_observed,
        }) if actual_limit == limit as u64 && actual_observed == observed as u64
    ));
    assert_eq!(stream.evidence().outcome, ProjectedReadOutcome::Failed);
    assert!(stream.next_batch().unwrap().is_none());
}

#[test]
fn retained_batches_across_completed_streams_share_the_cap() {
    let directory = tempfile::tempdir().unwrap();
    // One row: two 16-byte offset buffers, one key byte, 64 value bytes.
    let mut database = Database::create_with_options(
        directory.path(),
        DatabaseOptions {
            projected_output_bytes: 97,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    put(&mut database, b"a", &[7; 64]);
    let mut first = read(&database, ProjectedReadProjection::KeyValue);
    let batch = first.next_batch().unwrap().unwrap();
    assert_eq!(batch.allocated_bytes(), 97);
    assert!(first.next_batch().unwrap().is_none());
    let mut second = read(&database, ProjectedReadProjection::KeyValue);
    denied(&mut second, 97, 194);
    assert_eq!(batch.value(0), Some([7; 64].as_slice()));
    drop(batch);
    let mut next = read(&database, ProjectedReadProjection::KeyValue);
    assert_eq!(
        next.next_batch().unwrap().unwrap().value(0),
        Some([7; 64].as_slice())
    );
    next.cancel().unwrap();
    drop(database);
    directory.close().unwrap();
}

#[test]
fn zero_output_capacity_rejects_before_creating_database() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("not-created");
    assert!(matches!(
        Database::create_with_options(
            &path,
            DatabaseOptions {
                projected_output_bytes: 0,
                ..DatabaseOptions::default()
            },
        ),
        Err(Error::InvalidConfiguration(_))
    ));
    assert!(!path.exists());
    let mut database = Database::create(&path).unwrap();
    put(&mut database, b"prior", b"acknowledged");
    drop(database);
    let policy = DatabaseOptions {
        projected_output_bytes: 0,
        ..DatabaseOptions::default()
    };
    assert!(matches!(
        Database::open_with_options(&path, policy),
        Err(Error::InvalidConfiguration(_))
    ));
    assert!(matches!(
        rrd_lsm::ReadOnlyDatabase::open_with_options(&path, policy),
        Err(Error::InvalidConfiguration(_))
    ));
    let reopened = Database::open(&path).unwrap();
    assert_eq!(
        reopened.get(b"prior", reopened.snapshot()).unwrap(),
        Some(b"acknowledged".to_vec())
    );
}

#[test]
fn exact_cap_boundaries_cover_offsets_empty_values_and_key_projection() {
    for projection in [
        ProjectedReadProjection::Key,
        ProjectedReadProjection::KeyValue,
    ] {
        for value_bytes in [0, 65] {
            let value = vec![3; value_bytes];
            let allocation = if projection == ProjectedReadProjection::Key {
                17
            } else {
                33 + value_bytes
            };
            for limit in [allocation - 1, allocation, allocation + 1] {
                let directory = tempfile::tempdir().unwrap();
                let mut database = Database::create_with_options(
                    directory.path(),
                    DatabaseOptions {
                        projected_output_bytes: limit,
                        ..DatabaseOptions::default()
                    },
                )
                .unwrap();
                put(&mut database, b"a", &value);
                let memory = database.projected_output_memory();
                let mut stream = read(&database, projection);
                if limit < allocation {
                    denied(&mut stream, limit, allocation);
                    assert_eq!(memory.stats().peak_reserved_bytes, 0);
                    assert_eq!(stream.evidence().peak_batch_allocated_bytes, 0);
                } else {
                    let batch = stream.next_batch().unwrap().unwrap();
                    assert_eq!(batch.allocated_bytes(), allocation);
                    assert_eq!(batch.key(0), Some(b"a".as_slice()));
                    assert_eq!(
                        batch.value(0),
                        (projection == ProjectedReadProjection::KeyValue)
                            .then_some(value.as_slice())
                    );
                    assert_eq!(memory.stats().reserved_bytes, allocation);
                    assert!(stream.next_batch().unwrap().is_none());
                    drop(batch);
                }
                assert_eq!(memory.stats().reserved_bytes, 0);
                drop(database);
                directory.close().unwrap();
            }
        }
    }
}

fn physical_options(physical: &str, limit: usize) -> DatabaseOptions {
    DatabaseOptions {
        projected_output_bytes: limit,
        page_cache_bytes: 0,
        segment_io: SegmentIoPolicy {
            mode: if physical == "mapped-raw" {
                SegmentIoMode::Mmap
            } else {
                SegmentIoMode::Bounded
            },
            allow_fallback: false,
            ..SegmentIoPolicy::default()
        },
        segment_compression: if physical == "bounded-lz4" {
            SegmentCompressionPolicy::default()
        } else {
            SegmentCompressionPolicy::None
        },
        ..DatabaseOptions::default()
    }
}

#[test]
fn allocation_charge_follows_individual_clones_and_slices_after_every_terminal_path() {
    for physical in ["memtable", "mapped-raw", "bounded-raw", "bounded-lz4"] {
        for terminal in ["complete", "cancel", "fail", "drop"] {
            let directory = tempfile::tempdir().unwrap();
            let mut database =
                Database::create_with_options(directory.path(), physical_options(physical, 4129))
                    .unwrap();
            put(&mut database, b"a", &[5; 4096]);
            if terminal == "fail" {
                put(&mut database, b"b", &[6; 4096]);
            }
            if physical != "memtable" {
                database.flush_memtable(1).unwrap().unwrap();
            }
            let memory = database.projected_output_memory();
            let mut stream = read(&database, ProjectedReadProjection::KeyValue);
            let batch = stream.next_batch().unwrap().unwrap();
            assert_eq!(batch.allocated_bytes(), 4129);
            match physical {
                "mapped-raw" => assert!(stream.evidence().bytes_borrowed > 0),
                "bounded-raw" => assert!(stream.evidence().bounded_read_operations > 0),
                "bounded-lz4" => assert!(stream.evidence().bytes_decompressed > 0),
                _ => {}
            }
            let cloned_batch = batch.clone();
            let key = batch.key_data_buffer().clone();
            let key_offsets = batch.key_offsets_buffer().clone();
            let value_offsets = batch.value_offsets_buffer().unwrap().clone();
            let value = batch
                .value_data_buffer()
                .unwrap()
                .slice_with_length(100, 10);
            let empty_slice = value.slice_with_length(0, 0);
            assert_eq!(memory.stats().reserved_bytes, 4129);
            match terminal {
                "complete" => assert!(stream.next_batch().unwrap().is_none()),
                "cancel" => stream.cancel().unwrap(),
                "fail" => denied(&mut stream, 4129, 8258),
                "drop" => {}
                _ => unreachable!(),
            }
            drop(stream);
            denied(
                &mut read(&database, ProjectedReadProjection::KeyValue),
                4129,
                8258,
            );
            drop(batch);
            drop(cloned_batch);
            assert_eq!(memory.stats().reserved_bytes, 4129);
            assert_eq!(key.as_slice(), b"a");
            assert_eq!(
                key_offsets.as_slice(),
                [0i64.to_le_bytes(), 1i64.to_le_bytes()].concat()
            );
            assert_eq!(
                value_offsets.as_slice(),
                [0i64.to_le_bytes(), 4096i64.to_le_bytes()].concat()
            );
            drop(key);
            assert_eq!(memory.stats().reserved_bytes, 4128);
            drop(key_offsets);
            drop(value_offsets);
            assert_eq!(memory.stats().reserved_bytes, 4096);
            // Retaining even a zero-length slice retains its whole allocation.
            assert_eq!(value.as_slice(), &[5; 10]);
            drop(value);
            assert_eq!(memory.stats().reserved_bytes, 4096);
            denied(
                &mut read(&database, ProjectedReadProjection::KeyValue),
                4129,
                8225,
            );
            drop(empty_slice);
            assert_eq!(memory.stats().reserved_bytes, 0);
            let mut reusable = read(&database, ProjectedReadProjection::KeyValue);
            let last = reusable.next_batch().unwrap().unwrap();
            reusable.cancel().unwrap();
            drop(reusable);
            drop(database);
            directory.close().unwrap();
            assert_eq!(last.value(0), Some([5; 4096].as_slice()));
            assert_eq!(memory.stats().reserved_bytes, 4129);
            drop(last);
            assert_eq!(memory.stats().reserved_bytes, 0);
            assert_eq!(memory.stats().peak_reserved_bytes, 4129);
        }
    }
}

#[test]
fn failed_value_page_load_releases_preallocated_output_and_allows_new_reads() {
    let directory = tempfile::tempdir().unwrap();
    let mut database =
        Database::create_with_options(directory.path(), physical_options("bounded-raw", 97))
            .unwrap();
    put(&mut database, b"a", &[7; 64]);
    database.flush_memtable(1).unwrap().unwrap();
    let memory = database.projected_output_memory();
    let mut stream = database
        .begin_projected_read(ProjectedReadRequest {
            ranges: vec![ProjectedReadRange::all()],
            projection: ProjectedReadProjection::KeyValue,
            snapshot: database.snapshot(),
            // Spine (3), validity and offsets succeed; the value page fails after
            // output reservation/allocation, before any batch can be emitted.
            budget: ProjectedReadBudget {
                max_page_requests: 5,
                ..ProjectedReadBudget::default()
            },
        })
        .unwrap();
    assert!(matches!(
        stream.next_batch(),
        Err(Error::ProjectedReadLimit {
            resource: ProjectedReadResource::PageRequests,
            ..
        })
    ));
    assert_eq!(stream.evidence().outcome, ProjectedReadOutcome::Failed);
    assert_eq!(stream.evidence().emitted_batches, 0);
    assert_eq!(memory.stats().peak_reserved_bytes, 97);
    assert_eq!(memory.stats().reserved_bytes, 0);
    let mut next = read(&database, ProjectedReadProjection::KeyValue);
    let batch = next.next_batch().unwrap().unwrap();
    assert_eq!(batch.value(0), Some([7; 64].as_slice()));
    next.cancel().unwrap();
    drop(batch);
    assert_eq!(memory.stats().reserved_bytes, 0);
    drop(database);
    directory.close().unwrap();
}

#[test]
fn competing_streams_cannot_overbook_and_reuse_capacity_after_last_drop() {
    use std::sync::mpsc::sync_channel;
    use std::time::Duration;
    let directory = tempfile::tempdir().unwrap();
    let mut database =
        Database::create_with_options(directory.path(), physical_options("memtable", 97)).unwrap();
    put(&mut database, b"a", &[8; 64]);
    let memory = database.projected_output_memory();
    for _ in 0..32 {
        let (results_tx, results_rx) = sync_channel(2);
        let mut workers = Vec::new();
        let mut starts = Vec::new();
        for _ in 0..2 {
            let mut stream = read(&database, ProjectedReadProjection::KeyValue);
            let (start_tx, start_rx) = sync_channel(1);
            starts.push(start_tx);
            let results_tx = results_tx.clone();
            workers.push(std::thread::spawn(move || {
                start_rx.recv_timeout(Duration::from_secs(30)).unwrap();
                let batch = stream.next_batch();
                stream.cancel().unwrap();
                results_tx.send(batch).unwrap();
            }));
        }
        drop(results_tx);
        for start in starts {
            start.send(()).unwrap();
        }
        let results = [
            results_rx.recv_timeout(Duration::from_secs(30)).unwrap(),
            results_rx.recv_timeout(Duration::from_secs(30)).unwrap(),
        ];
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Ok(Some(_))))
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(
                    result,
                    Err(Error::ProjectedReadLimit {
                        resource: ProjectedReadResource::RetainedOutputBytes,
                        ..
                    })
                ))
                .count(),
            1
        );
        for batch in results
            .iter()
            .filter_map(|result| result.as_ref().ok().and_then(Option::as_ref))
        {
            assert_eq!(batch.value(0), Some([8; 64].as_slice()));
        }
        assert_eq!(memory.stats().reserved_bytes, 97);
        drop(results);
        assert_eq!(memory.stats().reserved_bytes, 0);
    }
    assert_eq!(memory.stats().peak_reserved_bytes, 97);
    assert_eq!(memory.stats().denied_batches, 32);
    drop(database);
    directory.close().unwrap();
}

#[test]
fn retained_output_survives_reclamation_and_reopen_with_explicit_pool_scope() {
    let directory = tempfile::tempdir().unwrap();
    let options = physical_options("bounded-lz4", 4129);
    let mut database = Database::create_with_options(directory.path(), options).unwrap();
    put(&mut database, b"a", &[1; 4096]);
    database.flush_memtable(1).unwrap().unwrap();
    let old_segments = database
        .manifest()
        .segments
        .iter()
        .map(|segment| segment.id.clone())
        .collect::<Vec<_>>();
    let memory = database.projected_output_memory();
    let mut stream = read(&database, ProjectedReadProjection::KeyValue);
    let old = stream.next_batch().unwrap().unwrap();
    assert!(stream.next_batch().unwrap().is_none());
    for (sequence, value) in [(2, 2), (3, 3)] {
        put(&mut database, b"a", &[value; 4096]);
        database.flush_memtable(sequence).unwrap().unwrap();
    }
    database.compact(&[], 4).unwrap().unwrap();
    let gc = database.garbage_collect().unwrap();
    for id in old_segments {
        let name = format!("{id}.seg");
        assert!(gc.removed_segments.contains(&name));
        assert!(!directory.path().join("segments").join(name).exists());
    }
    assert_eq!(old.value(0), Some([1; 4096].as_slice()));
    denied(
        &mut read(&database, ProjectedReadProjection::KeyValue),
        4129,
        8258,
    );
    drop(database);
    let reopened = Database::open_with_options(directory.path(), options).unwrap();
    let new_memory = reopened.projected_output_memory();
    assert_eq!(new_memory.stats().reserved_bytes, 0);
    assert_eq!(memory.stats().reserved_bytes, 4129);
    let mut next = read(&reopened, ProjectedReadProjection::KeyValue);
    let current = next.next_batch().unwrap().unwrap();
    assert_eq!(current.value(0), Some([3; 4096].as_slice()));
    next.cancel().unwrap();
    drop(reopened);
    directory.close().unwrap();
    assert_eq!(old.value(0), Some([1; 4096].as_slice()));
    assert_eq!(current.value(0), Some([3; 4096].as_slice()));
    drop(old);
    drop(current);
    assert_eq!(memory.stats().reserved_bytes, 0);
    assert_eq!(new_memory.stats().reserved_bytes, 0);
}
