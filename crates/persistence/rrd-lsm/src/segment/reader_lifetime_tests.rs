use super::*;
use crate::{
    Database, DatabaseOptions, Durability, Mutation, SegmentCompressionPolicy, SegmentIoMode,
    SegmentIoPolicy, WriteBatch,
};

fn terminal_owners(outcome: ProjectedReadOutcome) {
    for physical in ["memtable", "mapped-raw", "bounded-raw", "bounded-lz4"] {
        let directory = tempfile::tempdir().unwrap();
        let mut database = Database::create_with_options(
            &directory.path().join(physical),
            DatabaseOptions {
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
            },
        )
        .unwrap();
        database
            .write_owned(
                WriteBatch::new(vec![
                    Mutation::Put {
                        key: b"a".to_vec(),
                        value: vec![b'x'; 4096],
                    },
                    Mutation::Put {
                        key: b"b".to_vec(),
                        value: vec![b'y'; 4096],
                    },
                    Mutation::Put {
                        key: b"c".to_vec(),
                        value: vec![b'z'; 4096],
                    },
                ])
                .unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
        if physical != "memtable" {
            database.flush_memtable(1).unwrap();
        }
        let mut stream = database
            .begin_projected_read(ProjectedReadRequest {
                ranges: vec![ProjectedReadRange::all()],
                projection: ProjectedReadProjection::KeyValue,
                snapshot: database.snapshot(),
                budget: ProjectedReadBudget {
                    max_batch_rows: 1,
                    max_output_rows: if outcome == ProjectedReadOutcome::Failed {
                        1
                    } else {
                        3
                    },
                    ..ProjectedReadBudget::default()
                },
            })
            .unwrap();
        let registry = Arc::clone(&stream.lease.as_ref().unwrap().registry);
        let first = stream.next_batch().unwrap().unwrap();
        let retained = first.clone();
        let mut memtables = Vec::new();
        let mut segments = Vec::new();
        let mut pages = Vec::new();
        for cursor in &stream.cursors {
            match cursor {
                RunCursor::Memtable(cursor) => memtables.push(Arc::downgrade(&cursor.table)),
                RunCursor::Segment(cursor) => {
                    segments.push(Arc::downgrade(&cursor.segment));
                    if let Some(spine) = &cursor.loaded_spine {
                        pages.extend([
                            Arc::downgrade(&spine.offsets),
                            Arc::downgrade(&spine.data),
                            Arc::downgrade(&spine.sequences),
                        ]);
                    }
                }
            }
        }
        assert_eq!(memtables.len() + segments.len(), 1);
        assert_eq!(pages.len(), if physical == "memtable" { 0 } else { 3 });
        assert!(pages.iter().all(|owner| owner.upgrade().is_some()));
        match physical {
            "mapped-raw" => assert!(stream.evidence().bytes_borrowed > 0),
            "bounded-raw" => assert!(stream.evidence().bounded_read_operations > 0),
            "bounded-lz4" => assert!(stream.evidence().bytes_decompressed > 0),
            _ => {}
        }
        drop(database);
        assert!(memtables.iter().all(|owner| owner.upgrade().is_some()));
        assert!(segments.iter().all(|owner| owner.upgrade().is_some()));
        assert_eq!(registry.lock().unwrap().active, 1);

        match outcome {
            ProjectedReadOutcome::Completed => {
                let second = stream.next_batch().unwrap().unwrap();
                assert_eq!(second.key(0), Some(b"b".as_slice()));
                assert_eq!(second.value(0), Some(vec![b'y'; 4096].as_slice()));
                let third = stream.next_batch().unwrap().unwrap();
                assert_eq!(third.key(0), Some(b"c".as_slice()));
                assert_eq!(third.value(0), Some(vec![b'z'; 4096].as_slice()));
                assert!(stream.next_batch().unwrap().is_none());
            }
            ProjectedReadOutcome::Cancelled => stream.cancel().unwrap(),
            ProjectedReadOutcome::Failed => assert!(matches!(
                stream.next_batch(),
                Err(Error::ProjectedReadLimit {
                    resource: ProjectedReadResource::OutputRows,
                    ..
                })
            )),
            _ => unreachable!(),
        }
        // The terminal stream stays live. Weak owners observe actual retention,
        // independently of the registry's claimed admission/pin release.
        assert!(
            memtables.iter().all(|owner| owner.upgrade().is_none()),
            "{physical}"
        );
        assert!(
            segments.iter().all(|owner| owner.upgrade().is_none()),
            "{physical}"
        );
        assert!(
            pages.iter().all(|owner| owner.upgrade().is_none()),
            "{physical}"
        );
        let views = registry.lock().unwrap();
        assert_eq!(views.active, 0);
        assert_eq!(views.pinned_bytes, 0);
        assert!(views.manifests.is_empty());
        drop(views);
        assert_eq!(stream.cursors.capacity(), 0);
        assert_eq!(stream.heads.capacity(), 0);
        assert_eq!(stream.heap.capacity(), 0);
        assert!(stream.pending.is_none());
        assert_eq!(stream.evidence().outcome, outcome);
        stream.cancel().unwrap();
        assert!(stream.next_batch().unwrap().is_none());
        assert_eq!(stream.evidence().outcome, outcome);
        drop(stream);
        for batch in [&first, &retained] {
            assert_eq!(batch.key(0), Some(b"a".as_slice()));
            assert_eq!(batch.value(0), Some(vec![b'x'; 4096].as_slice()));
        }
        directory.close().unwrap();
        assert_eq!(retained.value(0), Some(vec![b'x'; 4096].as_slice()));
    }
}

#[test]
fn completed_stream_releases_sources_and_preserves_copied_batches() {
    terminal_owners(ProjectedReadOutcome::Completed);
}

#[test]
fn cancelled_stream_releases_sources_and_preserves_copied_batches() {
    terminal_owners(ProjectedReadOutcome::Cancelled);
}

#[test]
fn failed_stream_releases_sources_and_preserves_copied_batches() {
    terminal_owners(ProjectedReadOutcome::Failed);
}
