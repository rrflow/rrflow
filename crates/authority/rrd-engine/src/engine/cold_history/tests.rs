use super::*;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use rrd_contract::ColdHistoryCompression;
use rrd_core::{RuntimeLogicalModel, RuntimeRecord, RuntimeSchemaRegistry, RuntimeTableSchema};

/// A test child parks here when its environment names this checkpoint, so its
/// parent can kill it at exactly this point of a publication.
pub(super) fn publication_checkpoint(name: &str) {
    if std::env::var("RRFLOW_COLD_CRASH_CHECKPOINT").as_deref() != Ok(name) {
        return;
    }
    let marker = std::env::var_os("RRFLOW_COLD_CRASH_MARKER").expect("checkpoint marker");
    std::fs::write(marker, name).expect("write checkpoint marker");
    let deadline = Instant::now() + Duration::from_secs(90);
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        std::thread::park_timeout(remaining);
    }
    panic!("publication checkpoint was not terminated by its parent");
}

/// Kills and reaps its child when dropped, so a failing parent leaves none behind.
struct OwnedTestChild(std::process::Child);

impl Drop for OwnedTestChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

/// The source changes a pending publication must contain: the oracle.
fn expected_changes(
    engine: &RrdEngine,
    stamp: &ReadStamp,
    after: u64,
    limit: usize,
) -> Vec<RuntimeChange> {
    engine
        .storage
        .runtime()
        .read_changes(stamp, after, limit)
        .unwrap()
        .changes
}

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
    let original = expected_changes(
        &engine,
        &pending.stamp,
        pending.requested_after,
        pending.scan_limit as usize,
    );
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

// Carried from the archived F-06 worktree (2026-11-07): a failure keeps the
// identity its cause had, and an artifact is read in bounded requests.

#[test]
fn encoder_errors_keep_their_cause_after_deadline() {
    let deadline = Instant::now();
    let budget = rrd_query::Error::Budget("admitted bytes exceeded".into());
    assert!(Instant::now() >= deadline);
    assert!(matches!(
        ServiceError::from(budget),
        ServiceError::ConfigurationLimit(_)
    ));
    assert!(matches!(
        ServiceError::from(rrd_query::Error::DeadlineExceeded("expired".into())),
        ServiceError::DeadlineExceeded
    ));
}

#[test]
fn decoder_deadline_and_resource_failures_are_distinct() {
    let stamp = ReadStamp::new(ScopeId::new("instance:cold").unwrap(), None, 0, 0, None).unwrap();
    let expected = ColdHistoryReadExpectation {
        stamp,
        requested_after: 0,
        through_cursor: 0,
        rows: 0,
        artifact_sha256: "0".repeat(64),
    };
    let deadline = Instant::now();
    let limits = ColdHistoryReadLimits {
        max_rows: 1,
        max_encoded_bytes: 1,
        max_decoded_bytes: 1,
        max_memory_bytes: 64,
        max_batch_rows: 1,
        deadline,
    };
    let timeout = decode_cold_history_page(vec![0], &expected, limits).unwrap_err();
    assert!(matches!(
        cold_read_error(timeout),
        ServiceError::DeadlineExceeded
    ));
    let future = Instant::now() + Duration::from_millis(500);
    let resource = decode_cold_history_page(
        vec![0, 0],
        &expected,
        ColdHistoryReadLimits {
            deadline: future,
            ..limits
        },
    )
    .unwrap_err();
    while Instant::now() < future {
        std::thread::sleep(future.saturating_duration_since(Instant::now()));
    }
    assert!(matches!(
        cold_read_error(resource),
        ServiceError::ConfigurationLimit(_)
    ));
}

#[test]
fn artifact_read_checks_deadline_and_exact_length() {
    use std::io::{self, Cursor, Read};

    let deadline = Instant::now() + Duration::from_secs(60);
    assert_eq!(
        read_cold_artifact(&mut Cursor::new(b"cold"), 4, deadline).unwrap(),
        b"cold"
    );
    assert!(read_cold_artifact(&mut Cursor::new(b"cold"), 3, deadline).is_err());
    assert!(read_cold_artifact(&mut Cursor::new(b"cold"), 5, deadline).is_err());

    struct TrackingReader<'a> {
        source: Cursor<&'a [u8]>,
        largest_request: usize,
        calls: usize,
    }
    impl Read for TrackingReader<'_> {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.largest_request = self.largest_request.max(bytes.len());
            self.calls += 1;
            self.source.read(bytes)
        }
    }
    let data: Vec<u8> = (0..(2 * 64 * 1024 + 17))
        .map(|index| (index % 251) as u8)
        .collect();
    let mut tracked = TrackingReader {
        source: Cursor::new(&data),
        largest_request: 0,
        calls: 0,
    };
    assert_eq!(
        read_cold_artifact(&mut tracked, data.len(), deadline).unwrap(),
        data
    );
    assert!(tracked.calls >= 3);
    assert!(tracked.largest_request <= 64 * 1024);

    struct InterruptedOnce {
        state: u8,
    }
    impl Read for InterruptedOnce {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.state += 1;
            match self.state {
                1 => Err(io::ErrorKind::Interrupted.into()),
                2 => {
                    bytes[0] = b'x';
                    Ok(1)
                }
                _ => Ok(0),
            }
        }
    }
    assert_eq!(
        read_cold_artifact(&mut InterruptedOnce { state: 0 }, 1, deadline).unwrap(),
        b"x"
    );

    struct SlowReader {
        calls: usize,
    }
    impl Read for SlowReader {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.calls += 1;
            std::thread::sleep(Duration::from_millis(750));
            bytes[0] = b'x';
            Ok(1)
        }
    }
    let mut slow = SlowReader { calls: 0 };
    let error =
        read_cold_artifact(&mut slow, 4, Instant::now() + Duration::from_millis(500)).unwrap_err();
    assert!(matches!(error, ServiceError::DeadlineExceeded));
    assert_eq!(slow.calls, 1);
}

// Carried from the archived F-06 worktree: native upkeep that always has
// work must not keep the cold copy from advancing.
#[test]
fn embedded_maintenance_publishes_cold_history_while_native_debt_remains() {
    use rrd_store::Durability;

    let root = tempfile::tempdir().unwrap();
    let mut engine = open(&root.path().join("estate"));
    seed(&engine);
    engine.estate_configuration.storage.cold_history = Some(policy());
    let store = engine.storage.as_rrflow_kv().unwrap();
    let mut ordinal = 0_u64;
    let debt = || {
        store
            .physical_store_evidence()
            .unwrap()
            .compaction_debt_segments
            .unwrap()
    };
    let mut replenish_debt = || {
        for _ in 0..32 {
            if debt() > 0 {
                return;
            }
            ordinal += 1;
            let mut tx = store.begin_transaction().unwrap();
            tx.put(
                format!("cold-fairness-{ordinal}").into_bytes(),
                b"ack".to_vec(),
            )
            .unwrap();
            tx.commit(Durability::Authoritative).unwrap();
            store.flush(ordinal).unwrap();
        }
        panic!("could not create native compaction debt");
    };
    replenish_debt();
    assert!(engine.maintain_storage().unwrap());
    replenish_debt();
    assert!(engine.maintain_storage().unwrap());
    let prepared = engine
        .storage
        .control()
        .get("server/state/cold/cold-history/head")
        .unwrap();
    assert!(
        prepared.is_some(),
        "cold work must be offered while native debt remains"
    );
    assert!(state(&engine).pending.is_some());
    assert!(debt() > 0);
    assert!(engine.maintain_storage().unwrap());
    assert_eq!(state(&engine).generation, 0);
    assert!(state(&engine).pending.is_some());
    replenish_debt();
    assert!(engine.maintain_storage().unwrap());
    assert_eq!(state(&engine).generation, 1);
    assert!(debt() > 0);
}

// Carried from the archived worktree f06-main-integration-20261107.
#[test]
fn cold_history_denies_row_or_field_restricted_changefeed_grants() {
    use rrd_security::{
        DataPolicy, PolicyPredicate, Principal, PrincipalKind, ResourceGrant, SecurityRepository,
        SecurityState,
    };
    use std::collections::BTreeSet;

    let root = tempfile::tempdir().unwrap();
    let mut engine = open(&root.path().join("estate"));
    seed(&engine);
    let policy = policy();
    engine.estate_configuration.storage.cold_history = Some(policy.clone());
    assert!(engine.maintain_cold_history_at(&policy, 100).unwrap());
    assert!(engine.maintain_cold_history_at(&policy, 101).unwrap());
    assert_eq!(state(&engine).generation, 1);

    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, "cold").unwrap()],
    };
    let principal = |name: &str, credential: &[u8], data_policy: Option<DataPolicy>| {
        let id = CanonicalId::new(name).unwrap();
        Principal {
            id,
            kind: PrincipalKind::User,
            credential_sha256: digest::sha256_hex(credential),
            credential_revision: 1,
            not_before_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            disabled: false,
            role_ids: BTreeSet::new(),
            grants: vec![
                ResourceGrant {
                    action: SecurityAction::SessionCreate,
                    resource_prefix: resource.clone(),
                    data_policy: None,
                },
                ResourceGrant {
                    action: SecurityAction::ChangefeedRead,
                    resource_prefix: resource.clone(),
                    data_policy,
                },
            ],
        }
    };
    let restricted_fields = principal(
        "restricted-fields",
        b"restricted-fields-key",
        Some(DataPolicy {
            tenant: None,
            rows: Vec::new(),
            allowed_fields: Some(BTreeSet::from(["body".into()])),
        }),
    );
    let restricted_rows = principal(
        "restricted-rows",
        b"restricted-rows-key",
        Some(DataPolicy {
            tenant: None,
            rows: vec![PolicyPredicate {
                field: "body".into(),
                value: RuntimeValue::String("before".into()),
            }],
            allowed_fields: None,
        }),
    );
    let unrestricted = principal("unrestricted", b"unrestricted-key", None);
    SecurityRepository::new(&engine.storage, CanonicalId::new("cold").unwrap())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([
                    (restricted_fields.id.clone(), restricted_fields),
                    (restricted_rows.id.clone(), restricted_rows),
                    (unrestricted.id.clone(), unrestricted),
                ]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "cold-security-test",
            "cold-security-init",
            "cold-security-init",
        )
        .unwrap();
    let request = CreateSession {
        limits: SessionLimits {
            idle_timeout_ms: 5_000,
            absolute_timeout_ms: 10_000,
            max_open_transactions: 2,
        },
    };
    for (name, key, denied) in [
        (
            "restricted-fields",
            b"restricted-fields-key".as_slice(),
            true,
        ),
        ("restricted-rows", b"restricted-rows-key".as_slice(), true),
        ("unrestricted", b"unrestricted-key".as_slice(), false),
    ] {
        let session_request_id = format!("cold-{name}-session-request");
        let lease = engine
            .create_authenticated_session(
                &CanonicalId::new(name).unwrap(),
                key,
                &request,
                &CorrelationId::new(format!("cold-{name}-session")).unwrap(),
                1_000,
                &session_request_id,
                &session_request_id,
            )
            .unwrap();
        for generation in [None, Some(1)] {
            let read_request_id = format!("cold-{name}-read-{}", generation.unwrap_or(0));
            let result = engine.read_cold_history(
                &lease.session_id,
                &lease.token,
                &ReadColdHistory {
                    scope: "instance:cold".into(),
                    generation,
                    max_artifact_bytes: 1024 * 1024,
                    budget: rrd_contract::QueryBudget::default(),
                },
                1_100,
                &read_request_id,
                &read_request_id,
            );
            if denied {
                assert!(
                    matches!(result, Err(ServiceError::PermissionDenied)),
                    "{name} generation {generation:?}: {result:?}"
                );
            } else {
                let result = result.unwrap();
                assert_eq!(result.generation.is_some(), generation.is_some());
                if generation.is_some() {
                    assert_eq!(result.changes.len(), 2);
                }
            }
        }
    }
}

// Carried from the archived F-06 worktree: a publication killed at any of its
// four durable boundaries reopens to exactly one generation whose rows equal
// the source changes.
#[test]
fn publication_survives_process_termination_at_each_boundary() {
    use std::process::{Command, Stdio};

    const TEST_NAME: &str =
        "engine::cold_history::tests::publication_survives_process_termination_at_each_boundary";
    if let Some(path) = std::env::var_os("RRFLOW_COLD_CRASH_ESTATE") {
        let engine = open(Path::new(&path));
        engine.maintain_cold_history_at(&policy(), 101).unwrap();
        panic!("publication completed without reaching the requested checkpoint");
    }

    for checkpoint in [
        "stage_written",
        "stage_synced",
        "artifact_renamed",
        "receipt_committed",
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("estate");
        let marker = root.path().join("checkpoint");
        let engine = open(&path);
        seed(&engine);
        assert!(engine.maintain_cold_history_at(&policy(), 100).unwrap());
        let prepared = state(&engine);
        let pending = prepared.pending.as_ref().unwrap();
        let expected = expected_changes(
            &engine,
            &pending.stamp,
            pending.requested_after,
            pending.scan_limit as usize,
        );
        let expected_cursor = pending.through_cursor;
        drop(engine);

        let mut child = OwnedTestChild(
            Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg(TEST_NAME)
                .arg("--nocapture")
                .env("RRFLOW_COLD_CRASH_ESTATE", &path)
                .env("RRFLOW_COLD_CRASH_CHECKPOINT", checkpoint)
                .env("RRFLOW_COLD_CRASH_MARKER", &marker)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(60);
        while std::fs::read(&marker).ok().as_deref() != Some(checkpoint.as_bytes()) {
            if let Some(status) = child.0.try_wait().unwrap() {
                panic!("child exited before {checkpoint}: {status}");
            }
            if Instant::now() >= deadline {
                panic!("child did not reach {checkpoint}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(child);

        let engine = open(&path);
        let reopened = state(&engine);
        if checkpoint == "receipt_committed" {
            assert_eq!(reopened.generation, 1);
            assert!(reopened.pending.is_none());
            assert_eq!(reopened.through_cursor, expected_cursor);
        } else {
            assert_eq!(reopened.generation, 0, "{checkpoint}");
            assert_eq!(reopened.through_cursor, 0, "{checkpoint}");
            assert!(reopened.pending.is_some(), "{checkpoint}");
            assert!(engine.maintain_cold_history_at(&policy(), 102).unwrap());
        }
        let published = state(&engine);
        assert_eq!(published.generation, 1, "{checkpoint}");
        assert_eq!(published.through_cursor, expected_cursor, "{checkpoint}");
        assert!(published.pending.is_none(), "{checkpoint}");
        let key = "server/state/cold/cold-history/generation/00000000000000000001";
        let receipt: ColdReceipt =
            serde_json::from_slice(&engine.storage.control().get(key).unwrap().unwrap()).unwrap();
        let artifact = engine
            .storage
            .as_rrflow_kv()
            .unwrap()
            .anchored_path()
            .join("cold-history")
            .join(format!("{}.parquet", receipt.pending.artifact_sha256));
        let reader = ParquetRecordBatchReaderBuilder::try_new(artifact.open_read().unwrap())
            .unwrap()
            .build()
            .unwrap();
        let actual = reader
            .flat_map(|batch| rrd_query::record_batch_to_rows(&batch.unwrap()).unwrap())
            .map(|row| {
                let RuntimeValue::String(json) = &row.values["change_json"] else {
                    panic!("history value")
                };
                serde_json::from_str::<RuntimeChange>(json).unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{checkpoint}");
        assert!(!engine.maintain_cold_history_at(&policy(), 103).unwrap());
        assert_eq!(state(&engine).generation, 1, "{checkpoint}");
    }
}
