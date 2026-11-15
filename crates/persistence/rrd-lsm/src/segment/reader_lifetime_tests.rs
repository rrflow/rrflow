use super::*;
use crate::{
    Database, DatabaseOptions, Durability, Mutation, SegmentCompressionPolicy, SegmentIoMode,
    SegmentIoPolicy, WriteBatch,
};

#[test]
fn read_view_shares_manifest_generation_and_survives_publication() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = Database::create(&directory.path().join("engram")).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![Mutation::Put {
                key: b"a".to_vec(),
                value: b"old".to_vec(),
            }])
            .unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(1).unwrap();
    assert!(!database.manifest().segments.is_empty());
    let descriptor_allocation = database.manifest().segments.as_ptr();
    let generation = database.manifest().digest.clone();
    let mut stream = database
        .begin_projected_read(ProjectedReadRequest {
            ranges: vec![ProjectedReadRange::all()],
            projection: ProjectedReadProjection::KeyValue,
            snapshot: database.snapshot(),
            budget: ProjectedReadBudget::default(),
        })
        .unwrap();
    let registry = Arc::clone(&stream.lease.as_ref().unwrap().registry);
    let generation_owner =
        Arc::downgrade(&registry.lock().unwrap().manifests[&generation].manifest);
    assert_eq!(
        registry.lock().unwrap().manifests[&generation]
            .manifest
            .segments
            .as_ptr(),
        descriptor_allocation,
        "opening a bounded view must not copy unrelated manifest descriptors"
    );
    database
        .write_owned(
            WriteBatch::new(vec![Mutation::Put {
                key: b"a".to_vec(),
                value: b"new".to_vec(),
            }])
            .unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(2).unwrap();
    assert_ne!(database.manifest().digest, generation);
    assert_eq!(
        registry.lock().unwrap().manifests[&generation]
            .manifest
            .segments
            .as_ptr(),
        descriptor_allocation
    );
    drop(database);
    let batch = stream.next_batch().unwrap().unwrap();
    assert_eq!(batch.key(0), Some(b"a".as_slice()));
    assert_eq!(batch.value(0), Some(b"old".as_slice()));
    assert!(stream.next_batch().unwrap().is_none());
    let owners = registry.lock().unwrap();
    assert_eq!(owners.active, 0);
    assert_eq!(owners.pinned_bytes, 0);
    assert!(owners.manifests.is_empty());
    assert!(owners.memtables.is_empty());
    assert!(generation_owner.upgrade().is_none());
}

#[test]
fn shared_manifest_admission_retains_content_checks_and_releases_denials() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::create(&directory.path().join("engram")).unwrap();
    let manifest = Arc::new(database.manifest().clone());
    let registry = Arc::new(Mutex::new(ActiveReadViews::default()));
    let budget = ProjectedReadBudget::default();
    let first =
        ActiveReadViews::acquire(&registry, Arc::clone(&manifest), None, 0, &[], budget).unwrap();
    // A separately allocated but equal generation remains valid.
    let second = ActiveReadViews::acquire(
        &registry,
        Arc::new(manifest.as_ref().clone()),
        None,
        0,
        &[],
        budget,
    )
    .unwrap();
    assert_eq!(ActiveReadViews::ownership(&registry).unwrap(), (2, 0));
    drop(second);
    let mut inconsistent = manifest.as_ref().clone();
    inconsistent.created_at += 1; // Same claimed digest, different content.
    assert!(matches!(
        ActiveReadViews::acquire(&registry, Arc::new(inconsistent), None, 0, &[], budget),
        Err(Error::PoisonedReadViewRegistry)
    ));
    let owners_before = Arc::strong_count(&manifest);
    assert!(matches!(
        ActiveReadViews::acquire(
            &registry,
            Arc::clone(&manifest),
            None,
            0,
            &[],
            ProjectedReadBudget {
                max_active_views: 1,
                ..budget
            }
        ),
        Err(Error::ProjectedReadLimit {
            resource: ProjectedReadResource::ActiveViews,
            ..
        })
    ));
    assert_eq!(Arc::strong_count(&manifest), owners_before);
    assert_eq!(ActiveReadViews::ownership(&registry).unwrap(), (1, 0));
    drop(first);
    assert_eq!(ActiveReadViews::ownership(&registry).unwrap(), (0, 0));
    assert_eq!(Arc::strong_count(&manifest), 1);
}

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
        assert!(views.memtables.is_empty());
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

#[test]
fn immutable_segment_identity_allows_level_changes_but_rejects_changed_content() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = Database::create(&directory.path().join("identity")).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![Mutation::Put {
                key: b"a".to_vec(),
                value: b"value".to_vec(),
            }])
            .unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(1).unwrap();
    let descriptor = &database.manifest().segments[0];
    let mut relocated = descriptor.clone();
    relocated.level = descriptor.level.saturating_add(1);
    assert!(same_segment_file(descriptor, &relocated));
    relocated.bytes += 1;
    assert!(!same_segment_file(descriptor, &relocated));
}
