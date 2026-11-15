use super::*;
use rrd_core::{
    digest, Claim, DataTransaction, ObjectReceipt, Predicate, Producer, ProjectionId, RuntimeEvent,
    RuntimeEventSchema, RuntimeLogicalModel, RuntimeMutation, RuntimeProperties,
    RuntimePropertySchema, RuntimeRecord, RuntimeRecordSchema, RuntimeRef, RuntimeRelation,
    RuntimeRelationSchema, RuntimeSchemaRegistry, RuntimeTableSchema, RuntimeType, RuntimeValue,
    RuntimeValueType, RuntimeVector, Subject, VectorCollectionAddress, VectorValue,
};
use rrd_lsm::{FailureMode, WriteBatch, WriteBoundary};
use std::collections::BTreeMap;

#[test]
fn physical_diagnostic_trace_attributes_real_mutex_contention() {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc, Mutex, Weak};
    use std::time::Duration;

    #[derive(Clone)]
    struct Capture {
        bytes: Arc<Mutex<Vec<u8>>>,
        store: Weak<RrflowKvStore>,
        lock_available: Arc<AtomicBool>,
    }
    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let store = self.store.upgrade().unwrap();
            self.lock_available
                .store(store.database.try_write().is_ok(), Ordering::SeqCst);
            self.bytes.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(RrflowKvStore::open(&directory.path().join("diagnostic")).unwrap());
    let mut tx = store.begin_transaction().unwrap();
    for index in 0..256u64 {
        tx.put(format!("diagnostic-{index}").into_bytes(), vec![42; 32])
            .unwrap();
    }
    tx.commit(Durability::Authoritative).unwrap();
    let expected = store.physical_store_evidence().unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let lock_available = Arc::new(AtomicBool::new(false));
    let held = store.lock().unwrap();
    let (started_tx, started_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::scope(|scope| {
        let output = captured.clone();
        let store = &store;
        let weak_store = Arc::downgrade(store);
        let writer_lock_available = lock_available.clone();
        scope.spawn(move || {
            let writer = move || Capture {
                bytes: output.clone(),
                store: weak_store.clone(),
                lock_available: writer_lock_available.clone(),
            };
            let subscriber = tracing_subscriber::fmt()
                .json()
                .with_max_level(tracing::Level::DEBUG)
                .with_writer(writer)
                .finish();
            tracing::subscriber::with_default(subscriber, || {
                started_tx.send(()).unwrap();
                let start = Instant::now();
                let result = store.physical_store_evidence().unwrap();
                done_tx.send((result, start.elapsed())).unwrap();
            });
        });
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(done_rx.recv_timeout(Duration::from_millis(100)).is_err());
        drop(held);
        let (actual, elapsed) = done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(actual, expected);
        assert!(
            lock_available.load(Ordering::SeqCst),
            "trace writer must run after releasing the database mutex"
        );
        let bytes = captured.lock().unwrap().clone();
        let event: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let fields = &event["fields"];
        let wait = fields["mutex_wait_ns"].as_u64().unwrap();
        let hold = fields["mutex_held_assembly_ns"].as_u64().unwrap();
        let profile = fields["memtable_profile_ns"].as_u64().unwrap();
        assert!(
            wait >= 1_000_000,
            "the deliberately blocked call must record at least one millisecond of mutex waiting"
        );
        assert!(profile <= hold);
        assert!(u128::from(wait) + u128::from(hold) <= elapsed.as_nanos());
        assert_eq!(fields["memtable_keys"].as_u64(), expected.memtable_keys);
        assert_eq!(
            fields["memtable_versions"].as_u64(),
            expected.memtable_versions
        );
        println!(
            "controlled diagnostic wait_ns={wait} hold_ns={hold} profile_ns={profile} outer_ns={}",
            elapsed.as_nanos()
        );
    });
}

#[test]
#[ignore = "bounded native diagnostic characterization; not capacity evidence"]
fn characterize_diagnostic_profile_at_observed_memtable_size() {
    let directory = tempfile::tempdir().unwrap();
    let store = RrflowKvStore::open(&directory.path().join("profile")).unwrap();
    // Match the approximate key/version count of the retained failed
    // observation, not its segment count, workload, or resource limits.
    for round in 0..2 {
        let mut tx = store.begin_transaction().unwrap();
        for index in 0..19_440u64 {
            tx.put(
                format!("diagnostic-{index:08}").into_bytes(),
                vec![round; 32],
            )
            .unwrap();
        }
        tx.commit(Durability::Authoritative).unwrap();
    }
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        for _ in 0..3 {
            let evidence = store.physical_store_evidence().unwrap();
            assert_eq!(evidence.memtable_keys, Some(19_440));
            assert_eq!(evidence.memtable_versions, Some(38_880));
            assert_eq!(evidence.segment_count, Some(0));
        }
    });
}

/// Runs `read` while another reader is inside the database, and says whether it finished.
fn finishes_beside_another_reader(store: &RrflowKvStore, read: impl FnOnce() + Send) -> bool {
    let held = store.read_lock().unwrap();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        scope.spawn(move || {
            read();
            let _ = done_tx.send(());
        });
        let finished = done_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .is_ok();
        drop(held);
        finished
    })
}

fn store_with_a_key(directory: &tempfile::TempDir) -> RrflowKvStore {
    let store = RrflowKvStore::open(&directory.path().join("reads")).unwrap();
    let mut tx = store.begin_transaction().unwrap();
    tx.put(b"present".to_vec(), b"value".to_vec()).unwrap();
    tx.commit(Durability::Authoritative).unwrap();
    store
}

/// A transaction on its own thread that reads when told, and says when it has.
fn reader_on_its_own_thread<'scope>(
    scope: &'scope std::thread::Scope<'scope, '_>,
    store: &'scope RrflowKvStore,
) -> (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>) {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    scope.spawn(move || {
        let tx = store.begin_transaction().unwrap();
        ready_tx.send(()).unwrap();
        go_rx.recv().unwrap();
        assert_eq!(tx.get(b"present").unwrap(), Some(b"value".to_vec()));
        assert_eq!(tx.scan(b"p", b"q", 8).unwrap().len(), 1);
        done_tx.send(()).unwrap();
    });
    ready_rx.recv().unwrap();
    (go_tx, done_rx)
}

#[test]
fn a_read_does_not_wait_for_another_read() {
    let directory = tempfile::tempdir().unwrap();
    let store = store_with_a_key(&directory);
    std::thread::scope(|scope| {
        let (go, done) = reader_on_its_own_thread(scope, &store);
        let held = store.read_lock().unwrap();
        go.send(()).unwrap();
        let finished = done
            .recv_timeout(std::time::Duration::from_secs(10))
            .is_ok();
        drop(held);
        assert!(finished, "a read queued behind another read");
    });
}

#[test]
fn a_new_transaction_does_not_wait_for_a_read() {
    let directory = tempfile::tempdir().unwrap();
    let store = store_with_a_key(&directory);
    let finished = finishes_beside_another_reader(&store, || {
        drop(store.begin_transaction().unwrap());
    });
    assert!(finished, "beginning a transaction queued behind a read");
}

#[test]
fn a_commit_still_excludes_reads() {
    let directory = tempfile::tempdir().unwrap();
    let store = store_with_a_key(&directory);
    std::thread::scope(|scope| {
        let (go, done) = reader_on_its_own_thread(scope, &store);
        let owner = store.lock().unwrap();
        go.send(()).unwrap();
        assert!(
            done.recv_timeout(std::time::Duration::from_millis(300))
                .is_err(),
            "a read ran while the database's owner held it"
        );
        drop(owner);
        done.recv_timeout(std::time::Duration::from_secs(10))
            .expect("the read ran once the owner let go");
    });
}

#[test]
fn overlapping_reads_never_see_half_of_a_commit() {
    let directory = tempfile::tempdir().unwrap();
    let store = RrflowKvStore::open(&directory.path().join("overlap")).unwrap();
    let write_pair = |value: u64| {
        let mut tx = store.begin_transaction().unwrap();
        tx.put(b"left".to_vec(), value.to_le_bytes().to_vec())
            .unwrap();
        tx.put(b"right".to_vec(), value.to_le_bytes().to_vec())
            .unwrap();
        tx.commit(Durability::Authoritative).unwrap();
    };
    write_pair(0);
    let finished = AtomicBool::new(false);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                let mut reads = 0_u64;
                while !finished.load(Ordering::Acquire) || reads == 0 {
                    let tx = store.begin_transaction().unwrap();
                    let left = tx.get(b"left").unwrap();
                    let right = tx.get(b"right").unwrap();
                    assert_eq!(left, right, "a snapshot saw half of a commit");
                    assert!(left.is_some());
                    reads += 1;
                }
            });
        }
        for value in 1..=200 {
            write_pair(value);
        }
        finished.store(true, Ordering::Release);
    });
    let tx = store.begin_transaction().unwrap();
    assert_eq!(
        tx.get(b"left").unwrap(),
        Some(200_u64.to_le_bytes().to_vec())
    );
}

#[test]
fn idle_maintenance_does_not_queue_behind_the_database_owner() {
    let directory = tempfile::tempdir().unwrap();
    let store = RrflowKvStore::open(&directory.path().join("idle")).unwrap();
    let owner = store.lock().unwrap();
    assert!(!store.try_maintenance_step().unwrap());
    drop(owner);
    // Opening a store can leave one physical reclamation quantum.
    store.try_maintenance_step().unwrap();
    assert!(!store.try_maintenance_step().unwrap());
}

#[test]
fn reconciliation_evidence_fails_on_counter_regression() {
    let before_cache = PageCacheStats {
        hits: 1,
        ..PageCacheStats::default()
    };
    let after_cache = PageCacheStats::default();
    let error = reconciliation_evidence(
        &before_cache,
        &after_cache,
        &SegmentIoStats::default(),
        &SegmentIoStats::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("counter regressed: cache hits"));
}

#[test]
fn reconciliation_evidence_fails_on_derived_counter_overflow() {
    let after_cache = PageCacheStats {
        hits: u64::MAX,
        misses: 1,
        ..PageCacheStats::default()
    };
    let error = reconciliation_evidence(
        &PageCacheStats::default(),
        &after_cache,
        &SegmentIoStats::default(),
        &SegmentIoStats::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("page requests overflowed"));
}

#[test]
fn reconciliation_evidence_attributes_actual_io_backend() {
    let after_cache = PageCacheStats {
        misses: 1,
        loads: 1,
        bytes_read: 64,
        ..PageCacheStats::default()
    };
    let after_io = SegmentIoStats {
        fallback_count: 1,
        read_operations: 1,
        bounded_read_operations: 1,
        bytes_read: 64,
        ..SegmentIoStats::default()
    };
    let evidence = reconciliation_evidence(
        &PageCacheStats::default(),
        &after_cache,
        &SegmentIoStats::default(),
        &after_io,
    )
    .unwrap();
    assert_eq!(evidence.io_fallbacks, 1);
    assert_eq!(evidence.read_operations, 1);
    assert_eq!(evidence.io_uring_read_operations, 0);
    assert_eq!(evidence.bounded_read_operations, 1);
    assert_eq!(evidence.bytes_read, 64);
}

fn claim() -> Claim {
    Claim::new(
        rrd_core::Subject::new("test-subject").unwrap(),
        rrd_core::Predicate::new("test-predicate").unwrap(),
        "test-value",
        10,
        11,
        Producer {
            actor: "rrflow-kv-test".into(),
            on_behalf_of: None,
            session: None,
        },
    )
}

fn failure_scope() -> ScopeId {
    ScopeId::new("instance:rrflow-kv-failure").unwrap()
}

fn failure_schema() -> RuntimeSchemaRegistry {
    let item_kind = RuntimeType::new("item").unwrap();
    let mut registry = RuntimeSchemaRegistry::empty(1, "rrflowKV semantic failure matrix");
    registry
        .define_record_table(
            item_kind.clone(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema {
                properties: BTreeMap::from([
                    (
                        "status".into(),
                        RuntimePropertySchema::required(RuntimeValueType::String),
                    ),
                    (
                        "title".into(),
                        RuntimePropertySchema::required(RuntimeValueType::String),
                    ),
                    (
                        "code".into(),
                        RuntimePropertySchema::required(RuntimeValueType::String),
                    ),
                ]),
                ..RuntimeRecordSchema::default()
            },
        )
        .unwrap();
    registry.tables.insert(
        RuntimeType::new("embedding").unwrap(),
        RuntimeTableSchema::schemaless(RuntimeLogicalModel::Vector),
    );
    registry
        .define_event_table(
            RuntimeType::new("pulse").unwrap(),
            RuntimeLogicalModel::Event,
            RuntimeEventSchema {
                subject_required: true,
                subject_types: BTreeSet::from([item_kind.clone()]),
                properties: BTreeMap::new(),
                allow_additional_properties: false,
            },
        )
        .unwrap();
    registry
        .define_relation_table(
            RuntimeType::new("links").unwrap(),
            RuntimeRelationSchema {
                from: BTreeSet::from([item_kind.clone()]),
                to: BTreeSet::from([item_kind]),
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    registry
}

fn failure_record(id: &str, status: &str, title: &str, valid_from: u64) -> RuntimeRecord {
    RuntimeRecord {
        reference: RuntimeRef::new("item", id).unwrap(),
        valid_from,
        valid_to: None,
        properties: BTreeMap::from([
            (
                "code".into(),
                RuntimeValue::String(format!("{id}-{valid_from}")),
            ),
            ("status".into(), RuntimeValue::String(status.into())),
            ("title".into(), RuntimeValue::String(title.into())),
        ]),
    }
}

fn bootstrap_failure_database(root: &Path) -> Database {
    let store = RrflowKvStore::open(root).unwrap();
    store
        .runtime()
        .commit(&RuntimeCommit {
            scope: failure_scope(),
            at: 10,
            actor: "agent:rrflow-kv-failure".into(),
            expected_cursor: 0,
            mutations: vec![
                RuntimeMutation::Schema {
                    registry: failure_schema(),
                },
                RuntimeMutation::Record {
                    record: failure_record("one", "open", "First", 10),
                },
                RuntimeMutation::Record {
                    record: failure_record("two", "closed", "Second", 10),
                },
            ],
        })
        .unwrap();
    let bindings = [
        crate::IndexCommitBindingDefinition {
            id: ProjectionId::new("item-status").unwrap(),
            record_kind: RuntimeType::new("item").unwrap(),
            fields: vec!["status".into()],
            kind: crate::IndexCommitBindingKind::Scalar { unique: false },
            configuration_sha256: digest::sha256_hex(b"item-status:scalar"),
            lexical: None,
        },
        crate::IndexCommitBindingDefinition {
            id: ProjectionId::new("item-code").unwrap(),
            record_kind: RuntimeType::new("item").unwrap(),
            fields: vec!["code".into()],
            kind: crate::IndexCommitBindingKind::Scalar { unique: true },
            configuration_sha256: digest::sha256_hex(b"item-code:unique"),
            lexical: None,
        },
        crate::IndexCommitBindingDefinition {
            id: ProjectionId::new("item-title-bm25").unwrap(),
            record_kind: RuntimeType::new("item").unwrap(),
            fields: vec!["title".into()],
            kind: crate::IndexCommitBindingKind::Bm25,
            configuration_sha256: digest::sha256_hex(b"item-title:bm25"),
            lexical: Some(crate::lexical::Bm25Config::default()),
        },
    ];
    let scope = failure_scope();
    store
        .control()
        .commit_catalog_with_index_bindings(
            &scope,
            1,
            &crate::ControlTransition {
                key: format!("server/state/index-catalogue/{scope}"),
                expected: None,
                replacement: Some(br#"{"contract":"semantic-failure-matrix-v1"}"#.to_vec()),
                at: 20,
                actor: "agent:rrflow-kv-failure".into(),
                action: "index_catalogue.installed".into(),
                request_id: "request-semantic-failure-index".into(),
                operation_id: "operation-semantic-failure-index".into(),
            },
            &bindings,
        )
        .unwrap();
    drop(store);
    Database::open(root).unwrap()
}

fn failure_transaction(database: &Database) -> DataTransaction {
    let scope = failure_scope();
    let read = rrflow_kv_read_stamp(database, database.snapshot(), &scope).unwrap();
    let item = RuntimeRef::new("item", "one").unwrap();
    DataTransaction::new(
        read,
        RuntimeCommit {
            scope,
            at: 100,
            actor: "agent:rrflow-kv-failure".into(),
            expected_cursor: 3,
            mutations: vec![
                RuntimeMutation::Record {
                    record: failure_record("one", "pending", "First revised", 100),
                },
                RuntimeMutation::Relation {
                    relation: RuntimeRelation {
                        reference: RuntimeRef::new("links", "one-two").unwrap(),
                        from: RuntimeRef::new("item", "one").unwrap(),
                        to: RuntimeRef::new("item", "two").unwrap(),
                        valid_from: 100,
                        valid_to: None,
                        properties: RuntimeProperties::new(),
                    },
                },
                RuntimeMutation::Vector {
                    vector: RuntimeVector {
                        reference: RuntimeRef::new("embedding", "one-title").unwrap(),
                        subject: item.clone(),
                        collection: VectorCollectionAddress {
                            collection_id: "items".into(),
                            vector_name: "semantic".into(),
                        },
                        field: "title".into(),
                        valid_from: 100,
                        valid_to: None,
                        value: VectorValue::Dense {
                            values: vec![1.0, 0.0],
                        },
                        provenance: None,
                        properties: BTreeMap::new(),
                    },
                },
                RuntimeMutation::Event {
                    event: RuntimeEvent {
                        kind: RuntimeType::new("pulse").unwrap(),
                        subject: Some(item),
                        properties: RuntimeProperties::new(),
                    },
                },
                RuntimeMutation::Claim {
                    claim: Claim::new(
                        Subject::new("item-one").unwrap(),
                        Predicate::new("status").unwrap(),
                        "ready",
                        100,
                        100,
                        Producer {
                            actor: "agent:rrflow-kv-failure".into(),
                            on_behalf_of: None,
                            session: Some("rrflow-kv-failure".into()),
                        },
                    ),
                },
            ],
        },
    )
    .unwrap()
}

fn failure_function_receipt(commit_id: &str) -> crate::FunctionInvocationReceiptRecord {
    let canonical_receipt_json = format!(
        "{{\"invocation_id\":\"failure-function\",\"runtime_commit_sha256\":\"{commit_id}\"}}"
    );
    crate::FunctionInvocationReceiptRecord {
        format_version: crate::FUNCTION_INVOCATION_RECEIPT_FORMAT_VERSION,
        invocation_id: "failure-function".into(),
        runtime_commit_sha256: Some(commit_id.into()),
        receipt_sha256: digest::sha256_hex(format!("receipt:{commit_id}").as_bytes()),
        canonical_receipt_sha256: digest::sha256_hex(canonical_receipt_json.as_bytes()),
        canonical_receipt_json,
    }
}

#[test]
fn rrflow_kv_multi_family_transaction_recovers_all_or_none_at_every_wal_boundary() {
    for mode in [FailureMode::Crash, FailureMode::StorageFull] {
        for boundary in [
            WriteBoundary::Prepared,
            WriteBoundary::WalAppended,
            WriteBoundary::WalSynced,
            WriteBoundary::Visible,
        ] {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path().join("rrflow-kv-failure");
            let mut database = bootstrap_failure_database(&root);
            let transaction = failure_transaction(&database);
            let function_receipt = failure_function_receipt(&transaction.commit.digest());
            let plan = prepare_rrflow_kv_commit_at_read(
                &database,
                &transaction.commit,
                Some(&transaction.read),
                None,
                Some(("rrflow-kv-failure", std::slice::from_ref(&function_receipt))),
            )
            .unwrap();
            let expected = plan.outcome().clone();
            let (_, operations) = plan.into_parts_for(&database).unwrap();
            for (space, name, minimum) in [
                (keyspaces::RUNTIME_RECORDS, "current record", 1),
                (keyspaces::RUNTIME_RECORD_VERSIONS, "record version", 1),
                (keyspaces::RUNTIME_RELATIONS, "current relation", 1),
                (keyspaces::RUNTIME_RELATION_VERSIONS, "relation version", 1),
                (keyspaces::RUNTIME_OUTGOING_EDGES, "outgoing edge", 1),
                (
                    keyspaces::RUNTIME_OUTGOING_EDGE_VERSIONS,
                    "outgoing edge version",
                    1,
                ),
                (keyspaces::RUNTIME_INCOMING_EDGES, "incoming edge", 1),
                (
                    keyspaces::RUNTIME_INCOMING_EDGE_VERSIONS,
                    "incoming edge version",
                    1,
                ),
                (
                    keyspaces::RUNTIME_INDEX_SOURCE_DELTAS,
                    "scalar, unique, and BM25 source deltas",
                    3,
                ),
                (keyspaces::RUNTIME_SCALAR_ENTRIES, "scalar index entry", 1),
                (keyspaces::RUNTIME_UNIQUE_ENTRIES, "unique index entry", 1),
                (
                    keyspaces::RUNTIME_TERM_DICTIONARY,
                    "BM25 term dictionary entry",
                    1,
                ),
                (
                    keyspaces::RUNTIME_TERM_STATISTICS,
                    "BM25 term statistics",
                    1,
                ),
                (keyspaces::RUNTIME_TERM_POSTINGS, "BM25 term posting", 1),
                (keyspaces::RUNTIME_VECTORS, "current vector", 1),
                (keyspaces::RUNTIME_VECTOR_VERSIONS, "vector version", 1),
                (
                    keyspaces::RUNTIME_VECTOR_SOURCE_DELTAS,
                    "vector source delta",
                    1,
                ),
                (keyspaces::RUNTIME_CHANGES, "runtime changes", 5),
                (keyspaces::RUNTIME_PROJECTION_DELTAS, "projection deltas", 1),
                (keyspaces::RUNTIME_OUTBOX, "outbox", 1),
                (keyspaces::RUNTIME_AUDIT, "audit", 1),
                (keyspaces::RUNTIME_COMMITS, "commit outcome", 1),
                (keyspaces::INVOCATIONS, "function receipt", 1),
            ] {
                let prefix = keyspaces::space_prefix(space);
                assert!(
                    operations
                        .iter()
                        .filter(|operation| operation.key().starts_with(&prefix))
                        .count()
                        >= minimum,
                    "semantic plan lacks {name}"
                );
            }
            let baseline_snapshot = database.snapshot();
            let baseline = operations
                .iter()
                .map(|operation| {
                    (
                        operation.key().to_vec(),
                        database.get(operation.key(), baseline_snapshot).unwrap(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let mut physical = database.begin_transaction().unwrap();
            for operation in &operations {
                match operation {
                    Mutation::Put { key, value } => {
                        physical.put(key.clone(), value.clone()).unwrap()
                    }
                    Mutation::Delete { key } => physical.delete(key.clone()).unwrap(),
                }
            }
            let error = database
                .commit_transaction_with_failure(
                    physical,
                    rrd_lsm::Durability::Authoritative,
                    boundary,
                    mode,
                )
                .unwrap_err();
            assert!(matches!(error, rrd_lsm::Error::InjectedFailure { .. }));

            let visible_before_reopen = boundary == WriteBoundary::Visible;
            let in_process_snapshot = database.snapshot();
            for operation in &operations {
                let expected_value = if visible_before_reopen {
                    match operation {
                        Mutation::Put { value, .. } => Some(value.clone()),
                        Mutation::Delete { .. } => None,
                    }
                } else {
                    baseline[operation.key()].clone()
                };
                assert_eq!(
                    database.get(operation.key(), in_process_snapshot).unwrap(),
                    expected_value,
                    "in-process family split for mode={mode:?} boundary={boundary:?}"
                );
            }
            drop(database);

            let mut recovered = Database::open(&root).unwrap();
            let snapshot = recovered.snapshot();
            let published = boundary != WriteBoundary::Prepared;
            for operation in &operations {
                let expected_value = if published {
                    match operation {
                        Mutation::Put { value, .. } => Some(value.clone()),
                        Mutation::Delete { .. } => None,
                    }
                } else {
                    baseline[operation.key()].clone()
                };
                assert_eq!(
                    recovered.get(operation.key(), snapshot).unwrap(),
                    expected_value,
                    "reopened family split for mode={mode:?} boundary={boundary:?}"
                );
            }
            let expected_runtime_cursor = if published { 8 } else { 3 };
            let expected_claim_sequence = if published { 1 } else { 0 };
            assert_eq!(
                read_sequence(&recovered, snapshot, &keyspaces::runtime_cursor_key(),).unwrap(),
                expected_runtime_cursor,
                "mode={mode:?} boundary={boundary:?}"
            );
            assert_eq!(
                read_sequence(&recovered, snapshot, &keyspaces::sequence_watermark_key(),).unwrap(),
                expected_claim_sequence,
                "mode={mode:?} boundary={boundary:?}"
            );
            let outcome: Option<RuntimeCommitOutcome> = get_json(
                &recovered,
                snapshot,
                keyspaces::RUNTIME_COMMITS,
                &keyspaces::runtime_commit_key(&expected.commit_id),
            )
            .unwrap();
            assert_eq!(outcome.as_ref(), published.then_some(&expected));
            let audit: Option<AuditEnvelope> = get_json(
                &recovered,
                snapshot,
                keyspaces::RUNTIME_AUDIT,
                &keyspaces::runtime_audit_key(&expected.commit_id),
            )
            .unwrap();
            assert_eq!(
                audit.as_ref().and_then(|value| value.read.as_ref()),
                published.then_some(&transaction.read)
            );
            assert_eq!(
                audit.as_ref().and_then(|value| value.outcome_cursor),
                published.then_some(8)
            );
            let stored_function_receipt: Option<crate::FunctionInvocationReceiptRecord> = get_json(
                &recovered,
                snapshot,
                keyspaces::INVOCATIONS,
                &keyspaces::function_invocation_receipt_key(
                    "rrflow-kv-failure",
                    &function_receipt.invocation_id,
                ),
            )
            .unwrap();
            assert_eq!(
                stored_function_receipt.as_ref(),
                published.then_some(&function_receipt)
            );
            let record: Option<RuntimeRecord> = get_json(
                &recovered,
                snapshot,
                keyspaces::RUNTIME_RECORDS,
                &keyspaces::runtime_identity_key(
                    keyspaces::RUNTIME_RECORDS,
                    &transaction.commit.scope,
                    &RuntimeRef::new("item", "one").unwrap(),
                ),
            )
            .unwrap();
            let expected_record = if published {
                failure_record("one", "pending", "First revised", 100)
            } else {
                failure_record("one", "open", "First", 10)
            };
            assert_eq!(record.unwrap().properties, expected_record.properties);
            let relation: Option<RuntimeRelation> = get_json(
                &recovered,
                snapshot,
                keyspaces::RUNTIME_RELATIONS,
                &keyspaces::runtime_identity_key(
                    keyspaces::RUNTIME_RELATIONS,
                    &transaction.commit.scope,
                    &RuntimeRef::new("links", "one-two").unwrap(),
                ),
            )
            .unwrap();
            assert_eq!(relation.is_some(), published);

            recovered
                .write_owned(
                    WriteBatch::new(vec![Mutation::Put {
                        key: keyspaces::control_record_key("post-reopen"),
                        value: b"accepted".to_vec(),
                    }])
                    .unwrap(),
                    rrd_lsm::Durability::Authoritative,
                )
                .unwrap();
        }
    }
}

#[test]
fn new_rrflow_kv_database_authenticates_and_reopens_typed_key_format() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("compact-rrflow-kv");
    let expected = claim();
    let engine = RrflowKvStore::open(&root).unwrap();
    assert_eq!(
        engine.manifest().unwrap().application_format,
        Some(keyspaces::RRFLOW_KV_FORMAT)
    );
    engine
        .claims()
        .append_batch(std::slice::from_ref(&expected))
        .unwrap();
    {
        let database = engine.lock().unwrap();
        let start = keyspaces::space_prefix(keyspaces::CLAIMS);
        let end = prefix_end(&start).unwrap();
        let rows = database
            .scan(&start, Some(&end), database.snapshot())
            .unwrap();
        assert_eq!(rows.len(), 1);
        keyspaces::parse_claim_key(KeyCodec, &rows[0].0).unwrap();
    }
    engine.flush(12).unwrap();
    drop(engine);

    let reopened = RrflowKvStore::open(&root).unwrap();
    assert_eq!(
        reopened.manifest().unwrap().application_format,
        Some(keyspaces::RRFLOW_KV_FORMAT)
    );
    assert_eq!(
        reopened.claims().claims_in_range(0, 1).unwrap(),
        vec![expected]
    );
}

#[test]
fn rrflow_kv_open_denies_unknown_authenticated_application_format() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("unknown-rrflow-kv-format");
    drop(Database::create_with_application_format(&root, DatabaseOptions::default(), 7).unwrap());
    assert!(matches!(
        RrflowKvStore::open(&root),
        Err(Error::Substrate(reason)) if reason.contains("unsupported rrflowKV application format")
    ));
}

#[test]
fn snapshot_object_closure_denies_foreign_project_references() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("multi-project-rrflow-kv");
    let mut database = Database::create_with_application_format(
        &root,
        DatabaseOptions::default(),
        keyspaces::RRFLOW_KV_FORMAT,
    )
    .unwrap();
    let first_scope = ScopeId::new("project:first").unwrap();
    let second_scope = ScopeId::new("project:second").unwrap();
    let object = |id: &str, bytes: &[u8]| {
        let sha256 = rrd_core::digest::sha256_hex(bytes);
        ObjectReference::for_bytes(
            id,
            None,
            "application/octet-stream",
            bytes,
            ObjectReceipt {
                backend: "fixture".into(),
                key: ObjectReference::canonical_key(&sha256).unwrap(),
                version: None,
                etag: None,
            },
        )
        .unwrap()
    };
    let first = object("first-bytes", b"first");
    let second = object("second-bytes", b"second");
    database
        .write_owned(
            WriteBatch::new(vec![
                Mutation::Put {
                    key: storage_key(
                        keyspaces::RUNTIME_OBJECTS,
                        &keyspaces::runtime_identity_key(
                            keyspaces::RUNTIME_OBJECTS,
                            &first_scope,
                            &first.reference,
                        ),
                    ),
                    value: serde_json::to_vec(&first).unwrap(),
                },
                Mutation::Put {
                    key: storage_key(
                        keyspaces::RUNTIME_OBJECTS,
                        &keyspaces::runtime_identity_key(
                            keyspaces::RUNTIME_OBJECTS,
                            &second_scope,
                            &second.reference,
                        ),
                    ),
                    value: serde_json::to_vec(&second).unwrap(),
                },
            ])
            .unwrap(),
            rrd_lsm::Durability::Authoritative,
        )
        .unwrap();
    let spool = directory.path().join("multi-project.snapshot");
    let bundle = database.export_snapshot_file(1, &spool).unwrap();

    let error = rrflow_kv_snapshot_object_references(&bundle, &first_scope).unwrap_err();
    assert!(error.to_string().contains("project scope"));
}

// Carried from the archived worktree f06-main-integration-20261107.
#[test]
fn immutable_file_admission_skips_held_database_locks() {
    let directory = tempfile::tempdir().unwrap();
    let store = RrflowKvStore::open(&directory.path().join("cold-admission")).unwrap();
    let reader = store.database.read().unwrap();
    assert!(!store.try_admit_immutable_file(1).unwrap());
    drop(reader);
    let writer = store.lock().unwrap();
    assert!(!store.try_admit_immutable_file(1).unwrap());
    drop(writer);
    assert!(store.try_admit_immutable_file(1).unwrap());
}
