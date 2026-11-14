use super::*;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use rrd_contract::ColdHistoryCompression;
use rrd_core::{RuntimeLogicalModel, RuntimeRecord, RuntimeSchemaRegistry, RuntimeTableSchema};

fn policy() -> ColdHistoryPolicy {
    ColdHistoryPolicy {
        interval_ms: 1_000,
        max_changes: 16,
        max_input_bytes: 1024 * 1024,
        max_output_bytes: 1024 * 1024,
        max_memory_bytes: 64 * 1024 * 1024,
        max_elapsed_ms: 30_000,
        row_group_rows: 2,
        compression: ColdHistoryCompression::Zstd { level: 3 },
        dictionary: true,
        statistics: true,
    }
}

#[test]
fn policy_minimum_equals_the_encoder_reservation_for_a_full_page() {
    let policy = ColdHistoryPolicy {
        interval_ms: 1_000,
        max_changes: 512,
        max_input_bytes: 8 * 1024 * 1024,
        max_output_bytes: 8 * 1024 * 1024,
        max_memory_bytes: 4_294_967_296,
        max_elapsed_ms: 30_000,
        row_group_rows: 64,
        compression: ColdHistoryCompression::Zstd { level: 3 },
        dictionary: true,
        statistics: true,
    };
    const COLUMNS: usize = 15;
    let reservation = rrd_query::parquet_working_set_reservation(
        policy.max_input_bytes as usize,
        policy.max_changes as usize * COLUMNS,
        policy.max_output_bytes as usize,
        COLUMNS,
    )
    .unwrap();
    assert_eq!(policy.minimum_working_memory_bytes(), reservation as u64);
}

fn open(path: &Path) -> RrdEngine {
    RrdEngine::open(path, CanonicalId::new("cold").unwrap(), [7; 32]).unwrap()
}

fn seed(engine: &RrdEngine) {
    let mut registry = RuntimeSchemaRegistry::empty(1, "cold maintenance fixture");
    registry.tables.insert(
        RuntimeType::new("note").unwrap(),
        RuntimeTableSchema::schemaless(RuntimeLogicalModel::Relational),
    );
    engine
        .storage
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:cold").unwrap(),
            at: 10,
            actor: "cold-test".into(),
            expected_cursor: 0,
            mutations: vec![
                RuntimeMutation::Schema { registry },
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("note", "one").unwrap(),
                        valid_from: 10,
                        valid_to: None,
                        properties: BTreeMap::from([(
                            "body".into(),
                            RuntimeValue::String("before".into()),
                        )]),
                    },
                },
            ],
        })
        .unwrap();
}

fn state(engine: &RrdEngine) -> ColdState {
    let bytes = engine
        .storage
        .control()
        .get("server/state/cold/cold-history/head")
        .unwrap()
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn prepared_history_replays_after_restart_and_new_writes_then_detects_corruption() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("estate");
    let engine = open(&path);
    seed(&engine);
    let policy = policy();
    assert!(engine.maintain_cold_history_at(&policy, 100).unwrap());
    let prepared = state(&engine);
    let pending = prepared.pending.as_ref().unwrap();
    assert_eq!(prepared.through_cursor, 0);
    let original = engine
        .storage
        .runtime()
        .read_changes(
            &pending.stamp,
            pending.requested_after,
            pending.scan_limit as usize,
        )
        .unwrap()
        .changes;
    let directory = engine
        .storage
        .as_rrflow_kv()
        .unwrap()
        .anchored_path()
        .join("cold-history");
    directory.create_dir_all().unwrap();
    let stage = directory.join(format!(".{}.tmp", pending.artifact_sha256));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stage)
        .unwrap();
    file.write_all(b"interrupted stage").unwrap();
    file.sync_all().unwrap();
    drop(file);
    engine
        .storage
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:cold").unwrap(),
            at: 20,
            actor: "cold-test".into(),
            expected_cursor: 2,
            mutations: vec![RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: RuntimeRef::new("note", "two").unwrap(),
                    valid_from: 20,
                    valid_to: None,
                    properties: BTreeMap::from([(
                        "body".into(),
                        RuntimeValue::String("after".into()),
                    )]),
                },
            }],
        })
        .unwrap();
    drop(engine);
    let engine = open(&path);
    assert!(engine.maintain_cold_history_at(&policy, 101).unwrap());
    let published = state(&engine);
    assert_eq!(published.generation, 1);
    assert_eq!(published.through_cursor, pending.through_cursor);
    assert!(published.pending.is_none());
    assert!(!stage.try_exists().unwrap());
    let key = "server/state/cold/cold-history/generation/00000000000000000001";
    let receipt: ColdReceipt =
        serde_json::from_slice(&engine.storage.control().get(key).unwrap().unwrap()).unwrap();
    let file = directory
        .join(format!("{}.parquet", receipt.pending.artifact_sha256))
        .open_read()
        .unwrap();
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .unwrap()
        .build()
        .unwrap();
    let decoded = reader
        .flat_map(|batch| rrd_query::record_batch_to_rows(&batch.unwrap()).unwrap())
        .map(|row| {
            let RuntimeValue::String(json) = &row.values["change_json"] else {
                panic!("history value")
            };
            serde_json::from_str::<RuntimeChange>(json).unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(decoded, original);
    assert!(engine.maintain_cold_history_at(&policy, 102).unwrap());
    assert!(engine.maintain_cold_history_at(&policy, 103).unwrap());
    assert_eq!(state(&engine).through_cursor, 3);
    let last = state(&engine);
    let key = format!(
        "server/state/cold/cold-history/generation/{:020}",
        last.generation
    );
    let receipt: ColdReceipt =
        serde_json::from_slice(&engine.storage.control().get(&key).unwrap().unwrap()).unwrap();
    let file = directory.join(format!("{}.parquet", receipt.pending.artifact_sha256));
    let mut writer = OpenOptions::new().write(true).open(&file).unwrap();
    writer.write_all(b"corrupt").unwrap();
    writer.sync_all().unwrap();
    engine
        .storage
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:cold").unwrap(),
            at: 30,
            actor: "cold-test".into(),
            expected_cursor: 3,
            mutations: vec![RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: RuntimeRef::new("note", "three").unwrap(),
                    valid_from: 30,
                    valid_to: None,
                    properties: BTreeMap::new(),
                },
            }],
        })
        .unwrap();
    assert!(engine.maintain_cold_history_at(&policy, 104).is_err());
    assert_eq!(state(&engine).through_cursor, 3);
}

#[test]
fn changed_policy_replans_pending_work_without_rewinding_committed_cursor() {
    let root = tempfile::tempdir().unwrap();
    let engine = open(&root.path().join("estate"));
    seed(&engine);
    let first = policy();
    assert!(engine.maintain_cold_history_at(&first, 100).unwrap());
    let mut changed = first;
    changed.compression = ColdHistoryCompression::Zstd { level: 4 };
    assert!(engine.maintain_cold_history_at(&changed, 101).unwrap());
    assert!(state(&engine).pending.is_none());
    assert_eq!(state(&engine).through_cursor, 0);
    assert!(engine.maintain_cold_history_at(&changed, 102).unwrap());
    assert!(engine.maintain_cold_history_at(&changed, 103).unwrap());
    assert_eq!(state(&engine).through_cursor, 2);
}

#[test]
fn changed_catalogue_replans_a_stale_stamp_without_losing_progress() {
    let root = tempfile::tempdir().unwrap();
    let engine = open(&root.path().join("estate"));
    seed(&engine);
    let policy = policy();
    assert!(engine.maintain_cold_history_at(&policy, 100).unwrap());
    let old_manifest = state(&engine).pending.unwrap().stamp.manifest_id;
    let scope = ScopeId::new("instance:cold").unwrap();
    engine
        .storage
        .control()
        .commit_catalog(
            &scope,
            &ControlTransition {
                key: "server/state/cold/test-catalogue".into(),
                expected: None,
                replacement: Some(b"updated".to_vec()),
                at: 101,
                actor: "cold-test".into(),
                action: "test.catalogue_changed".into(),
                request_id: "cold-test".into(),
                operation_id: "cold-test".into(),
            },
        )
        .unwrap();
    assert!(engine.maintain_cold_history_at(&policy, 102).unwrap());
    assert!(state(&engine).pending.is_none());
    assert_eq!(state(&engine).through_cursor, 0);
    assert!(engine.maintain_cold_history_at(&policy, 103).unwrap());
    assert_ne!(
        state(&engine).pending.unwrap().stamp.manifest_id,
        old_manifest
    );
    assert!(engine.maintain_cold_history_at(&policy, 104).unwrap());
    assert_eq!(state(&engine).through_cursor, 2);
}

#[test]
fn authenticated_generation_readback_reopens_and_rejects_corruption_and_limits() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("estate");
    let mut engine = open(&path);
    seed(&engine);
    let policy = policy();
    engine.estate_configuration.storage.cold_history = Some(policy.clone());
    for _ in 0..64 {
        engine.maintain_storage().unwrap();
        if let Some(bytes) = engine
            .storage
            .control()
            .get("server/state/cold/cold-history/head")
            .unwrap()
        {
            let state: ColdState = serde_json::from_slice(&bytes).unwrap();
            if state.generation == 1 {
                break;
            }
        }
    }
    assert_eq!(state(&engine).generation, 1);
    drop(engine);
    let mut engine = open(&path);
    engine.estate_configuration.storage.cold_history = Some(policy);
    let session = engine
        .create_session(
            &rrd_contract::CreateSession {
                limits: rrd_contract::SessionLimits {
                    idle_timeout_ms: 5_000,
                    absolute_timeout_ms: 10_000,
                    max_open_transactions: 2,
                },
            },
            &CorrelationId::new("cold-read-session").unwrap(),
            1_000,
            "create",
            "create",
        )
        .unwrap();
    let mut request = ReadColdHistory {
        scope: "instance:cold".into(),
        generation: None,
        max_artifact_bytes: 1024 * 1024,
        budget: rrd_contract::QueryBudget::default(),
    };
    let read = |request: &ReadColdHistory| {
        engine.read_cold_history(
            &session.session_id,
            &session.token,
            request,
            1_100,
            "read",
            "read",
        )
    };
    let status = read(&request).unwrap();
    assert!(status.enabled);
    assert_eq!(status.current_generation, 1);
    assert_eq!(status.through_cursor, 2);
    assert!(status.generation.is_none());
    request.generation = Some(1);
    let result = read(&request).unwrap();
    assert_eq!(result.changes.len(), 2);
    let artifact = result.generation.unwrap();
    assert_eq!(artifact.through_cursor, 2);
    let mut restricted = request.clone();
    restricted.max_artifact_bytes = 1;
    assert!(read(&restricted).is_err());
    restricted = request.clone();
    restricted.scope = "instance:other".into();
    assert!(read(&restricted).is_err());
    assert!(engine
        .read_cold_history(
            &session.session_id,
            &CorrelationId::new("wrong-token").unwrap(),
            &request,
            1_100,
            "read",
            "read",
        )
        .is_err());
    let path = engine
        .storage
        .as_rrflow_kv()
        .unwrap()
        .anchored_path()
        .join("cold-history")
        .join(format!("{}.parquet", artifact.artifact_sha256));
    let mut file = OpenOptions::new().write(true).open(&path).unwrap();
    file.write_all(b"corrupt").unwrap();
    file.sync_all().unwrap();
    assert!(read(&request).is_err());
    assert_eq!(state(&engine).through_cursor, 2);
}

#[test]
fn publication_waits_for_a_foreground_transaction() {
    // A foreground commit and publication both append to the control
    // journal. Without the gate, they meet as a physical write conflict
    // that clients see as a retryable runtime conflict at an unchanged cursor.
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("estate");
    let mut engine = open(&path);
    seed(&engine);
    engine.estate_configuration.storage.cold_history = Some(policy());
    let before = engine.storage.control().sequence().unwrap();
    let foreground = engine
        .admit_transaction(&"foreground", "request-foreground", "operation-foreground")
        .unwrap();
    std::thread::scope(|scope| {
        let publication = scope.spawn(|| engine.maintain_cold_history());
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert_eq!(
            engine.storage.control().sequence().unwrap(),
            before,
            "publication committed while a foreground transaction held the gate"
        );
        drop(foreground);
        publication.join().unwrap().unwrap();
    });
    assert!(
        engine.storage.control().sequence().unwrap() > before,
        "publication proceeds once the gate is free"
    );
}

#[test]
fn maintenance_failure_is_durable_deduplicated_and_cleared_by_progress() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("estate");
    let mut engine = open(&path);
    seed(&engine);
    let mut too_small = policy();
    too_small.max_input_bytes = 1;
    engine.estate_configuration.storage.cold_history = Some(too_small.clone());
    for _ in 0..64 {
        if engine.maintain_storage().is_err() {
            break;
        }
    }
    assert_eq!(
        state(&engine).failure.unwrap().code,
        ColdHistoryFailureCode::ResourceLimit
    );
    let sequence = engine.storage.control().sequence().unwrap();
    assert!(engine.maintain_storage().is_err());
    assert_eq!(engine.storage.control().sequence().unwrap(), sequence);
    drop(engine);

    let mut engine = open(&path);
    engine.estate_configuration.storage.cold_history = Some(too_small);
    let session = engine
        .create_session(
            &rrd_contract::CreateSession {
                limits: rrd_contract::SessionLimits {
                    idle_timeout_ms: 5_000,
                    absolute_timeout_ms: 10_000,
                    max_open_transactions: 2,
                },
            },
            &CorrelationId::new("cold-failure-session").unwrap(),
            1_000,
            "create",
            "create",
        )
        .unwrap();
    let request = ReadColdHistory {
        scope: "instance:cold".into(),
        generation: None,
        max_artifact_bytes: 1024 * 1024,
        budget: rrd_contract::QueryBudget::default(),
    };
    let status = engine
        .read_cold_history(
            &session.session_id,
            &session.token,
            &request,
            1_100,
            "read",
            "read",
        )
        .unwrap();
    assert_eq!(
        status.failure.unwrap().code,
        ColdHistoryFailureCode::ResourceLimit
    );
    engine.estate_configuration.storage.cold_history = Some(policy());
    for _ in 0..64 {
        engine.maintain_storage().unwrap();
        if state(&engine).generation == 1 {
            break;
        }
    }
    assert_eq!(state(&engine).generation, 1);
    let recovered = engine
        .read_cold_history(
            &session.session_id,
            &session.token,
            &request,
            1_100,
            "read",
            "read",
        )
        .unwrap();
    assert!(recovered.failure.is_none());
}

#[test]
fn encoded_output_limit_is_reported_as_a_resource_failure() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = open(&root.path().join("estate"));
    seed(&engine);
    let mut limited = policy();
    limited.max_output_bytes = 1;
    engine.estate_configuration.storage.cold_history = Some(limited);

    let error = engine.maintain_cold_history().unwrap_err();
    assert_eq!(error.kind(), ServiceErrorKind::ResourceExhausted);
    let recorded = state(&engine);
    assert_eq!(recorded.generation, 0);
    assert!(recorded.pending.is_none());
    assert_eq!(
        recorded.failure.unwrap().code,
        ColdHistoryFailureCode::ResourceLimit
    );
}
