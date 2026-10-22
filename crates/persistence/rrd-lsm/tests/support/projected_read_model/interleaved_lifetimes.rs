use super::*;
use rrd_lsm::{ProjectedReadBatch, ProjectedReadStream};
use std::collections::BTreeSet;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::time::Duration;

type Rows = Vec<(Vec<u8>, Vec<u8>)>;
const WAIT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug)]
enum Terminal {
    Complete,
    Cancel,
    Drop,
}

enum Command {
    Next,
    End,
    Release,
}

enum Event {
    Ready(Box<(ProjectedReadBatch, ProjectedReadEvidence)>),
    Progress,
    Terminal,
}

fn append(batch: &ProjectedReadBatch, seen: &mut Rows, expected: &Rows) {
    for row in 0..batch.len() {
        seen.push((
            batch.key(row).unwrap().to_vec(),
            batch.value(row).unwrap().to_vec(),
        ));
    }
    assert!(
        seen.len() <= expected.len(),
        "reader emitted additional rows"
    );
    assert_eq!(
        seen.as_slice(),
        &expected[..seen.len()],
        "captured snapshot prefix changed"
    );
}

fn reader(
    stream: ProjectedReadStream,
    expected: Rows,
    terminal: Terminal,
    commands: Receiver<Command>,
    events: SyncSender<Event>,
) {
    let mut stream = Some(stream);
    let mut seen = Vec::new();
    let first = stream.as_mut().unwrap().next_batch().unwrap().unwrap();
    append(&first, &mut seen, &expected);
    events
        .send(Event::Ready(Box::new((
            first,
            stream.as_ref().unwrap().evidence().clone(),
        ))))
        .unwrap();
    loop {
        match commands.recv_timeout(WAIT).expect("bounded reader command") {
            Command::Next => {
                let batch = stream.as_mut().unwrap().next_batch().unwrap().unwrap();
                append(&batch, &mut seen, &expected);
                events.send(Event::Progress).unwrap();
            }
            Command::End => {
                match terminal {
                    Terminal::Complete => {
                        let stream = stream.as_mut().unwrap();
                        while let Some(batch) = stream.next_batch().unwrap() {
                            append(&batch, &mut seen, &expected);
                        }
                        assert_eq!(seen, expected);
                        assert_eq!(stream.evidence().outcome, ProjectedReadOutcome::Completed);
                    }
                    Terminal::Cancel => {
                        let stream = stream.as_mut().unwrap();
                        assert!(
                            seen.len() < expected.len(),
                            "cancel must interrupt unread data"
                        );
                        stream.cancel().unwrap();
                        assert!(stream.next_batch().unwrap().is_none());
                        assert_eq!(stream.evidence().outcome, ProjectedReadOutcome::Cancelled);
                    }
                    Terminal::Drop => {
                        assert!(
                            seen.len() < expected.len(),
                            "drop must interrupt unread data"
                        );
                        drop(stream.take());
                    }
                }
                events.send(Event::Terminal).unwrap();
                // Keep completed/cancelled stream objects alive while the writer
                // proves source reclamation and admission reuse.
                assert!(matches!(
                    commands.recv_timeout(WAIT).unwrap(),
                    Command::Release
                ));
                return;
            }
            Command::Release => panic!("release before terminal transition"),
        }
    }
}

fn request(database: &Database, max_active_views: usize) -> ProjectedReadRequest {
    ProjectedReadRequest {
        ranges: vec![ProjectedReadRange::all()],
        projection: ProjectedReadProjection::KeyValue,
        snapshot: database.snapshot(),
        budget: ProjectedReadBudget {
            max_active_views,
            max_batch_rows: 4,
            ..ProjectedReadBudget::default()
        },
    }
}

fn commit(database: &mut Database, model: &mut Model, mutations: Vec<Mutation>) {
    let first = model.current_sequence + 1;
    let last = first + mutations.len() as u64 - 1;
    let receipt = database
        .write_owned(
            WriteBatch::new(mutations.clone()).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    assert_eq!(
        (receipt.first_sequence, receipt.last_sequence),
        (first, last)
    );
    model.apply(&mutations, first);
}

fn mutations(epoch: u8) -> Vec<Mutation> {
    FAMILY_PREFIXES
        .iter()
        .enumerate()
        .flat_map(|(family, _)| {
            (0..4).map(move |identifier| {
                let key = key(family, identifier);
                if epoch > 0 && (family + identifier + usize::from(epoch)).is_multiple_of(5) {
                    Mutation::Delete { key }
                } else {
                    Mutation::Put {
                        key,
                        value: vec![epoch.wrapping_mul(13).wrapping_add(family as u8); 1024],
                    }
                }
            })
        })
        .collect()
}

fn segment_ids(database: &Database) -> BTreeSet<String> {
    database
        .manifest()
        .segments
        .iter()
        .map(|segment| segment.id.clone())
        .collect()
}

fn assert_retained(database: &Database, root: &Path, manifest: &str, segments: &BTreeSet<String>) {
    let gc = database.garbage_collect().unwrap();
    assert!(gc.retained_manifests.iter().any(|value| value == manifest));
    assert!(root
        .join("manifests")
        .join(format!("{manifest}.json"))
        .exists());
    for segment in segments {
        assert!(gc.retained_segments.contains(segment));
        assert!(root
            .join("segments")
            .join(format!("{segment}.seg"))
            .exists());
    }
}

fn assert_reclaimed(database: &Database, root: &Path, manifest: &str, segments: &BTreeSet<String>) {
    assert_ne!(
        database.manifest().digest,
        manifest,
        "fixture must retire the source manifest"
    );
    assert!(
        segment_ids(database).is_disjoint(segments),
        "fixture must retire every source segment"
    );
    let gc = database.garbage_collect().unwrap();
    assert!(gc.removed_manifests.contains(&format!("{manifest}.json")));
    assert!(!root
        .join("manifests")
        .join(format!("{manifest}.json"))
        .exists());
    for segment in segments {
        assert!(gc.removed_segments.contains(&format!("{segment}.seg")));
        assert!(!root
            .join("segments")
            .join(format!("{segment}.seg"))
            .exists());
    }
}

pub fn run_interleaved_lifetime_matrix(physical: &str) {
    for terminal in [Terminal::Complete, Terminal::Cancel, Terminal::Drop] {
        run_history(physical, terminal);
    }
}

fn run_history(physical: &str, terminal: Terminal) {
    println!("interleaved_lifetime begin physical={physical} terminal={terminal:?}");
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("history");
    let options = DatabaseOptions {
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
        compaction: CompactionPolicy {
            target_segment_bytes: 1024 * 1024,
            ..CompactionPolicy::default()
        },
        ..DatabaseOptions::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    let mut model = Model::default();
    commit(&mut database, &mut model, mutations(0));
    database.flush_memtable(1).unwrap().unwrap();
    // The first reader captures both an immutable source and a mutable head.
    commit(
        &mut database,
        &mut model,
        vec![Mutation::Put {
            key: key(0, 0),
            value: vec![b'm'; 1024],
        }],
    );
    let old_sequence = model.current_sequence;
    let old_rows = model.scan(&[], None, old_sequence);
    let old_manifest = database.manifest().digest.clone();
    let old_segments = segment_ids(&database);
    assert!(!old_segments.is_empty());
    database.checkpoint("held-generation", 2).unwrap();
    let pinned_transaction = database.begin_transaction().unwrap();
    let old_stream = database
        .begin_projected_read(request(&database, 2))
        .unwrap();
    let (commands, receive_commands) = sync_channel(1);
    let (send_events, events) = sync_channel(1);
    let retained = std::thread::scope(|scope| {
        let expected = old_rows.clone();
        let worker = scope.spawn(move || {
            reader(
                old_stream,
                expected,
                terminal,
                receive_commands,
                send_events,
            )
        });
        let (first, evidence) = match events.recv_timeout(WAIT).unwrap() {
            Event::Ready(ready) => *ready,
            _ => panic!("first event must bind the source path"),
        };
        assert_eq!(evidence.snapshot_sequence, old_sequence);
        assert!(
            evidence.selected_runs >= 2,
            "must include immutable and mutable generations"
        );
        match physical {
            "mapped-raw" => {
                assert!(evidence.bytes_borrowed > 0 && evidence.mmap_read_operations > 0)
            }
            "bounded-raw" => {
                assert!(evidence.bounded_read_operations > 0 && evidence.bytes_decompressed == 0)
            }
            "bounded-lz4" => {
                assert!(evidence.bounded_read_operations > 0 && evidence.bytes_decompressed > 0)
            }
            _ => panic!("unknown physical path"),
        }
        let retained = first.clone();
        let key_buffer = first.key_data_buffer().clone();
        let value_buffer = first.value_data_buffer().unwrap().clone();
        drop(first);

        commit(&mut database, &mut model, mutations(1));
        database.flush_memtable(3).unwrap().unwrap();
        database.compact(&[], 4).unwrap().unwrap();
        let newer_rows = model.scan(&[], None, model.current_sequence);
        assert_ne!(newer_rows, old_rows);
        let newer_manifest = database.manifest().digest.clone();
        let newer_segments = segment_ids(&database);
        assert!(newer_segments.is_disjoint(&old_segments));
        let mut newer = database
            .begin_projected_read(request(&database, 2))
            .unwrap();
        let mut newer_seen = Vec::new();
        let newer_first = newer.next_batch().unwrap().unwrap();
        append(&newer_first, &mut newer_seen, &newer_rows);
        assert!(matches!(
            database.begin_projected_read(request(&database, 2)),
            Err(Error::ProjectedReadLimit {
                resource: ProjectedReadResource::ActiveViews,
                ..
            })
        ));

        for epoch in 2..=4 {
            // Both threads may read/write during this phase; exactly one bounded
            // command is in flight, and acknowledgement closes the phase.
            commands.send(Command::Next).unwrap();
            commit(&mut database, &mut model, mutations(epoch));
            database
                .flush_memtable(u64::from(epoch) * 10)
                .unwrap()
                .unwrap();
            database
                .compact(&[], u64::from(epoch) * 10 + 1)
                .unwrap()
                .unwrap();
            if epoch == 2 {
                assert!(database.release_checkpoint("held-generation").unwrap());
                assert!(database.checkpoints().unwrap().is_empty());
            }
            assert_retained(&database, &root, &old_manifest, &old_segments);
            assert_retained(&database, &root, &newer_manifest, &newer_segments);
            assert!(matches!(
                events.recv_timeout(WAIT).unwrap(),
                Event::Progress
            ));
            let batch = newer.next_batch().unwrap().unwrap();
            append(&batch, &mut newer_seen, &newer_rows);
            for (key, expected) in &old_rows {
                assert_eq!(
                    pinned_transaction.get(&database, key).unwrap().as_ref(),
                    Some(expected)
                );
            }
            assert_eq!(
                database.scan(&[], None, database.snapshot()).unwrap(),
                model.scan(&[], None, model.current_sequence)
            );
        }
        commands.send(Command::End).unwrap();
        assert!(matches!(
            events.recv_timeout(WAIT).unwrap(),
            Event::Terminal
        ));
        assert_reclaimed(&database, &root, &old_manifest, &old_segments);
        // Terminal stream objects and copied Arrow outputs are still alive.
        for _ in 0..8 {
            let mut admitted = database
                .begin_projected_read(request(&database, 2))
                .unwrap();
            admitted.cancel().unwrap();
        }
        while let Some(batch) = newer.next_batch().unwrap() {
            append(&batch, &mut newer_seen, &newer_rows);
        }
        assert_eq!(newer_seen, newer_rows);
        assert_reclaimed(&database, &root, &newer_manifest, &newer_segments);
        drop(
            database
                .begin_projected_read(request(&database, 1))
                .unwrap(),
        );
        commands.send(Command::Release).unwrap();
        worker
            .join()
            .expect("owned reader must finish without panic");
        (retained, key_buffer, value_buffer, newer_first, newer_rows)
    });
    pinned_transaction.rollback().unwrap();
    database.compact(&[], 100).unwrap().unwrap();
    database.garbage_collect().unwrap();
    let expected_current = model.scan(&[], None, model.current_sequence);
    drop(database);
    let reopened = Database::open_with_options(&root, options).unwrap();
    assert_eq!(reopened.snapshot().sequence, model.current_sequence);
    assert_eq!(
        reopened.scan(&[], None, reopened.snapshot()).unwrap(),
        expected_current
    );
    drop(reopened);
    directory
        .close()
        .expect("all native source files must be removable");
    let (batch, keys, values, newer_batch, newer_rows) = retained;
    append(&batch, &mut Vec::new(), &old_rows);
    append(&newer_batch, &mut Vec::new(), &newer_rows);
    let expected_keys = old_rows[..batch.len()]
        .iter()
        .flat_map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let expected_values = old_rows[..batch.len()]
        .iter()
        .flat_map(|(_, value)| value.clone())
        .collect::<Vec<_>>();
    drop(batch);
    assert_eq!(keys.as_slice(), expected_keys);
    assert_eq!(values.as_slice(), expected_values);
    println!("interleaved_lifetime physical={physical} terminal={terminal:?} writes=6 reader_snapshots=2 checkpoint_releases=1 reopen=1 cleanup=complete");
}
