use rrd_contract::{
    transaction_operation_sha256, AuditDecision, AuditPhase, BeginTransaction, CanonicalId,
    CloseSession, CommitTransaction, CorrelationId, CreateSession, DataCatalogueIdentity,
    DataLogicalModel, DataProperties, DataPropertySchema, DataRecordSchema, DataReference,
    DataSchemaMode, DataSchemaRegistry, DataTableSchema, DataValueType, DataVectorValue,
    DiagnosticAuthority, DiagnosticStorageEvidenceLevel, EnsureQueryIndex, EnsureVectorCollection,
    EstateDesiredPhase, InstallationManagedPathKind, InstallationTargetKind, NamedVectorDefinition,
    QueryBudget, QueryIndexKind, QueryValue, ReadAudit, ReadChangefeed, ReadDiagnosticSnapshot,
    RequestContext, ResourceId, ResourceKind, ResourcePath, SecurityAction, SessionLimits,
    StorageProfileKind, TransactionMutation, VectorMemoryTier, VectorSearchMetric, VectorValueKind,
};
use rrd_core::{digest, Claim, Predicate, Producer, Subject};
use rrd_engine::{
    EstateAdminAction, EstateAdminResult, Invocation, InvocationCredential, RrdEngine,
    RrdOperation, ServiceError,
};
use rrd_estate::{
    DesiredPhase, DesiredTarget, EstateRepository, LeaseRequest, LocalEstatePermission,
    LocalOperatorPolicy, MutationContext, ObservationRequest, ObservedPhase, ReceiptBoundary,
    ReceiptRequest, ScheduleBackup, SetDesired, SetRecoveryPolicy, LOCAL_OPERATOR_POLICY_FORMAT,
};
use rrd_store::StorageEngine;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

fn canonical(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn correlation(value: &str) -> CorrelationId {
    CorrelationId::new(value).unwrap()
}

fn context(request: &str, operation: &str, idempotency: &str) -> RequestContext {
    RequestContext {
        request_id: correlation(request),
        operation_id: correlation(operation),
        idempotency_key: Some(correlation(idempotency)),
        deadline_unix_ms: Some(u64::MAX),
    }
}

fn resource(instance: &CanonicalId) -> ResourcePath {
    ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance.as_str()).unwrap()],
    }
}

fn reference(kind: &str, id: &str) -> DataReference {
    DataReference {
        kind: canonical(kind),
        id: canonical(id),
    }
}

fn diagnostic_request(scope: &str) -> ReadDiagnosticSnapshot {
    ReadDiagnosticSnapshot {
        scope: scope.into(),
        graph_valid_at_unix_ms: 1_500,
        graph_known_at_cursor: None,
        graph_compare_cursor: 0,
        runtime_max_scanned_changes: 1_024,
        changes_after_cursor: 0,
        change_limit: 64,
        audit_after_sequence: 0,
        audit_limit: 64,
    }
}

#[test]
fn one_authority_coordinates_security_data_catalogues_audit_and_reopen() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    fs::create_dir(&project).unwrap();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable,
    )
    .unwrap();
    let installation = RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable,
    )
    .unwrap();
    assert!(!installation.idempotent_replay);
    let at = |offset: u64| installation.applied_at_unix_ms + offset;
    let database = project.join(
        &preview
            .installation
            .managed_path(InstallationManagedPathKind::StorageRoot)
            .relative_path,
    );
    let token_path = project.join(
        &preview
            .installation
            .managed_path(InstallationManagedPathKind::TokenKey)
            .relative_path,
    );
    let instance = preview.installation.target.instance_id.clone();
    let scope = format!("instance:{instance}");

    let api_key = RrdEngine::read_installed_api_key(&project).unwrap();
    let principal_id = api_key.principal_id.clone();
    let authority_secret = fs::read(&token_path).unwrap();

    let operator_key = temporary.path().join("operator.key");
    let operator_policy = temporary.path().join("operator.json");
    let key = [13_u8; 32];
    fs::write(&operator_key, key).unwrap();
    private(&operator_key);
    let policy = LocalOperatorPolicy {
        format: LOCAL_OPERATOR_POLICY_FORMAT,
        operator_id: canonical("operator-one"),
        key_sha256: Sha256::digest(key)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        not_before_unix_ms: 1,
        expires_at_unix_ms: 10_000,
        estates: BTreeMap::from([(
            "estate-a".into(),
            BTreeSet::from([LocalEstatePermission::Create]),
        )]),
    };
    fs::write(&operator_policy, serde_json::to_vec(&policy).unwrap()).unwrap();
    private(&operator_policy);
    let create_estate = || {
        RrdEngine::administer_estate_store(
            &database,
            instance.clone(),
            &operator_policy,
            &operator_key,
            canonical("estate-a"),
            200,
            "create-estate-request".into(),
            canonical("create-estate-operation"),
            EstateAdminAction::Create,
        )
        .unwrap()
    };
    assert!(matches!(
        create_estate(),
        EstateAdminResult::Mutation(result) if !result.idempotent_replay
    ));
    assert!(matches!(
        create_estate(),
        EstateAdminResult::Mutation(result) if result.idempotent_replay
    ));

    let engine = RrdEngine::open_installed(&project, &executable).unwrap();
    assert_eq!(engine.instance_id(), &instance);
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            api_key.credential().as_bytes(),
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 60_000,
                    max_open_transactions: 2,
                },
            },
            &correlation("authority-session"),
            at(1_000),
            "request-session",
            "operation-session",
        )
        .unwrap();

    let vectors = engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: scope.clone(),
                collection_id: canonical("documents"),
                vectors: vec![NamedVectorDefinition {
                    name: canonical("body"),
                    field: canonical("body-embedding"),
                    kind: VectorValueKind::Dense,
                    dimensions: 2,
                    metric: VectorSearchMetric::Cosine,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &context(
                "request-vector-catalogue",
                "operation-vector-catalogue",
                "vector-catalogue-key",
            ),
            at(1_100),
        )
        .unwrap();
    assert_eq!(vectors.catalogue_revision, 1);

    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: canonical("data"),
                timeout_ms: 10_000,
            },
            &context(
                "request-transaction-begin",
                "operation-transaction-begin",
                "transaction-begin-key",
            ),
            at(1_200),
        )
        .unwrap();
    let document = reference("document", "document-a");
    let mutations = vec![
        TransactionMutation::PutSchema {
            registry: DataSchemaRegistry {
                revision: 2,
                migration: "install authority fixture schema".into(),
                catalogue: DataCatalogueIdentity::default(),
                tables: BTreeMap::from([
                    (
                        canonical("document"),
                        DataTableSchema {
                            model: DataLogicalModel::Relational,
                            mode: DataSchemaMode::Strict,
                            properties: BTreeMap::new(),
                            allow_additional_properties: false,
                        },
                    ),
                    (
                        canonical("embedding"),
                        DataTableSchema {
                            model: DataLogicalModel::Vector,
                            mode: DataSchemaMode::Schemaless,
                            properties: BTreeMap::new(),
                            allow_additional_properties: false,
                        },
                    ),
                ]),
                records: BTreeMap::from([(
                    canonical("document"),
                    DataRecordSchema {
                        properties: BTreeMap::from([(
                            "title".into(),
                            DataPropertySchema {
                                value_type: DataValueType::String,
                                required: true,
                            },
                        )]),
                        allow_additional_properties: false,
                        unique_properties: BTreeSet::new(),
                    },
                )]),
                relations: BTreeMap::new(),
                events: BTreeMap::new(),
            },
        },
        TransactionMutation::PutRecord {
            reference: document.clone(),
            valid_from: 1_250,
            valid_to: None,
            properties: DataProperties::from([(
                "title".into(),
                QueryValue::String("one authority".into()),
            )]),
        },
        TransactionMutation::PutVector {
            reference: reference("embedding", "document-a-body"),
            subject: document,
            collection_id: canonical("documents"),
            vector_name: canonical("body"),
            field: canonical("body-embedding"),
            valid_from: 1_250,
            valid_to: None,
            value: DataVectorValue::Dense {
                values: vec![1.0, 0.0],
            },
            provenance: None,
            properties: DataProperties::new(),
        },
        TransactionMutation::AssertClaim {
            subject: canonical("document-a"),
            predicate: canonical("authority"),
            object: "rrd-engine".into(),
            valid_from: 1_250,
            tx_time: 1_250,
            producer: canonical("authority-test"),
            confidence: Some(1.0),
        },
    ];
    let commit_request = CommitTransaction {
        operation_sha256: transaction_operation_sha256(&mutations),
        mutations,
    };
    let commit = engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &correlation("transaction-commit-key"),
            &commit_request,
            at(1_300),
            "request-transaction-commit",
            "operation-transaction-commit",
        )
        .unwrap();
    assert_eq!(commit.mutation_count, 4);
    assert_eq!(commit.claim_mutation_count, Some(1));

    let runtime_before_denial = engine.readiness(at(1_450)).unwrap().runtime_cursor;
    let invalid_session_token = correlation("invalid-session-token");
    let denied = engine.begin_invocation(
        Invocation {
            context: context(
                "request-denied-backup",
                "operation-denied-backup",
                "denied-backup-key",
            ),
            resource: resource(&instance),
            observed_at_unix_ms: at(1_450),
            attempt: 1,
            request_sha256: digest::sha256_hex(b"denied-backup"),
        },
        RrdOperation::BackupCreate,
        InvocationCredential::Session {
            session_id: &lease.session_id,
            token: &invalid_session_token,
        },
    );
    assert!(
        matches!(denied, Err(ServiceError::Unauthenticated)),
        "invalid session credential should be denied, got {denied:?}"
    );
    assert_eq!(
        engine.readiness(at(1_451)).unwrap().runtime_cursor,
        runtime_before_denial,
        "authentication denial must not mutate the runtime log"
    );

    let coordinated_vectors = engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: scope.clone(),
                collection_id: canonical("notes"),
                vectors: vec![NamedVectorDefinition {
                    name: canonical("title"),
                    field: canonical("title-embedding"),
                    kind: VectorValueKind::Dense,
                    dimensions: 2,
                    metric: VectorSearchMetric::Cosine,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &context(
                "request-coordinated-vector-catalogue",
                "operation-coordinated-vector-catalogue",
                "coordinated-vector-catalogue-key",
            ),
            at(1_480),
        )
        .unwrap();
    let vector_read = engine
        .read_diagnostic_snapshot(
            &lease.session_id,
            &lease.token,
            &diagnostic_request(&scope),
            at(1_490),
            "request-vector-read-stamp",
            "operation-vector-read-stamp",
        )
        .unwrap();
    let query_index = engine
        .ensure_query_index(
            &lease.session_id,
            &lease.token,
            &correlation("query-index-key"),
            &EnsureQueryIndex {
                scope: scope.clone(),
                index_id: canonical("document-title"),
                definition_query: "FROM record:document AT VALID 1250 KNOWN HEAD PROJECT title"
                    .into(),
                unique: false,
                kind: QueryIndexKind::Scalar,
                full_text: None,
                budget: QueryBudget::default(),
            },
            at(1_500),
            "request-query-index",
            "operation-query-index",
        )
        .unwrap();
    assert_eq!(
        query_index.catalogue_revision,
        coordinated_vectors.catalogue_revision + 1,
        "query and vector definitions must advance one shared catalogue revision"
    );

    let changes = engine
        .read_changefeed(
            &lease.session_id,
            &lease.token,
            &ReadChangefeed {
                scope: scope.clone(),
                after_cursor: 0,
                limit: 64,
            },
            at(1_600),
            "request-changefeed",
            "operation-changefeed",
        )
        .unwrap();
    assert_eq!(changes.through_cursor, changes.head_cursor);
    assert_eq!(changes.head_cursor, runtime_before_denial);

    let diagnostic = engine
        .read_diagnostic_snapshot(
            &lease.session_id,
            &lease.token,
            &diagnostic_request(&scope),
            at(1_700),
            "request-diagnostic",
            "operation-diagnostic",
        )
        .unwrap();
    assert_eq!(diagnostic.read.runtime_cursor, changes.head_cursor);
    assert!(
        diagnostic.read.catalogue_revision > vector_read.read.catalogue_revision,
        "the query build transitions must advance the same read-stamp catalogue coordinate observed after the vector transition"
    );
    assert_eq!(diagnostic.readiness.runtime_cursor, changes.head_cursor);
    assert_eq!(diagnostic.changes.head_cursor, changes.head_cursor);
    assert_eq!(diagnostic.vector_collections.revision, 2);
    assert_eq!(
        diagnostic.query_indexes.revision,
        query_index.catalogue_revision
    );
    assert_eq!(diagnostic.storage.profile, StorageProfileKind::RrflowKv);
    assert_eq!(
        diagnostic.storage.evidence_level,
        DiagnosticStorageEvidenceLevel::RrflowKvCounters
    );
    assert_eq!(
        diagnostic.storage.observed_at_unix_ms,
        diagnostic.observed_at_unix_ms
    );
    let rrflow_kv = diagnostic
        .storage
        .rrflow_kv
        .as_ref()
        .expect("rrflowKV publishes its physical counters");
    assert!(
        rrflow_kv.physical_sequence > 0,
        "committed transactions advance the physical sequence"
    );
    assert!(rrflow_kv.memtable.max_versions > 0);
    assert!(rrflow_kv.write_ahead_log.payload_max_bytes > 0);
    assert!(rrflow_kv.page_cache.capacity_bytes > 0);
    let storage_section = diagnostic
        .sections
        .iter()
        .find(|section| section.id.as_str() == "storage")
        .expect("the storage observation is disclosed as a section");
    assert_eq!(storage_section.authority, DiagnosticAuthority::Observation);
    assert_eq!(storage_section.row_count, 1);

    let audit = engine
        .read_audit(
            &lease.session_id,
            &lease.token,
            &ReadAudit {
                after_sequence: 0,
                limit: 64,
            },
            at(1_800),
            "request-audit",
            "operation-audit",
        )
        .unwrap();
    audit.validate().unwrap();
    let denied_backup = audit
        .records
        .iter()
        .find(|record| record.action == SecurityAction::BackupCreate)
        .unwrap();
    assert_eq!(denied_backup.phase, AuditPhase::Completed);
    assert_eq!(denied_backup.decision, AuditDecision::Denied);
    let estate_audit = audit
        .records
        .iter()
        .filter(|record| record.action == SecurityAction::EstateAdmin)
        .collect::<Vec<_>>();
    assert!(estate_audit
        .iter()
        .any(|record| record.phase == AuditPhase::Authorized));
    assert!(estate_audit.iter().any(|record| {
        record.phase == AuditPhase::Completed && record.decision == AuditDecision::Allowed
    }));
    assert_eq!(
        diagnostic.audit.records,
        audit.records[..diagnostic.audit.records.len()],
        "diagnostic and protected audit reads observe one shared ordered audit prefix"
    );

    let closed = engine
        .close_session(
            &lease.session_id,
            &lease.token,
            &CloseSession {},
            &correlation("session-close-key"),
            at(1_900),
            "request-session-close",
            "operation-session-close",
        )
        .unwrap();
    assert!(!closed.idempotent_replay);
    drop(engine);

    let reopened = RrdEngine::open_installed(&project, &executable).unwrap();
    assert_eq!(reopened.instance_id(), &instance);
    let readiness = reopened.readiness(at(2_000)).unwrap();
    assert_eq!(readiness.runtime_cursor, diagnostic.read.runtime_cursor);
    assert_eq!(readiness.claim_sequence, diagnostic.read.claim_sequence);
    let close_replay = reopened
        .close_session(
            &lease.session_id,
            &lease.token,
            &CloseSession {},
            &correlation("session-close-key"),
            at(2_000),
            "request-session-close-replay",
            "operation-session-close-replay",
        )
        .unwrap();
    assert!(close_replay.idempotent_replay);
    assert_eq!(close_replay.ended_at_unix_ms, closed.ended_at_unix_ms);
    assert_eq!(fs::read(&token_path).unwrap(), authority_secret);
    drop(reopened);
    let replay = RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.plan_sha256, installation.plan_sha256);
}

#[cfg(unix)]
fn private(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[cfg(not(unix))]
fn private(_path: &Path) {}

#[test]
fn public_admin_phase_type_does_not_reintroduce_a_physical_estate_dependency() {
    let action = EstateAdminAction::SetDesired {
        instance: canonical("project-a"),
        idempotency_key: "desired-a".into(),
        phase: EstateDesiredPhase::Stopped,
        deployment: canonical("rrd-server"),
        version: "1.0.0".into(),
        configuration_sha256: "a".repeat(64),
    };
    assert!(matches!(action, EstateAdminAction::SetDesired { .. }));
}

#[test]
fn estate_recovery_prune_and_restore_are_one_fenced_engine_workflow() {
    let temporary = tempfile::tempdir().unwrap();
    let database = temporary.path().join("estate-authority");
    let state_root = temporary.path().join("state");
    for path in [
        state_root.clone(),
        state_root.join("instances"),
        state_root.join("processes"),
    ] {
        fs::create_dir(&path).unwrap();
    }
    let state_root = fs::canonicalize(state_root).unwrap();
    // The managed instance is a canonically installed estate: the estate names
    // it `instance-a` (its project directory under `instances/`), while the
    // installed engine carries its own derived instance identity, which names
    // the backup catalogue and restore staging roots inside `.rrflow/rrd`.
    let project = state_root.join("instances/instance-a");
    fs::create_dir(&project).unwrap();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable,
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable,
    )
    .unwrap();
    let installed_instance = preview.installation.target.instance_id.clone();
    let catalogue_root = project
        .join(".rrflow/rrd/backups")
        .join(installed_instance.as_str());
    let restores_root = project
        .join(".rrflow/rrd/restores")
        .join(installed_instance.as_str());
    let storage_root = project.join(
        &preview
            .installation
            .managed_path(InstallationManagedPathKind::StorageRoot)
            .relative_path,
    );

    let key_path = temporary.path().join("operator.key");
    let policy_path = temporary.path().join("operator.json");
    let key = [23_u8; 32];
    fs::write(&key_path, key).unwrap();
    private(&key_path);
    let policy = LocalOperatorPolicy {
        format: LOCAL_OPERATOR_POLICY_FORMAT,
        operator_id: canonical("operator-one"),
        key_sha256: Sha256::digest(key)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        not_before_unix_ms: 1,
        expires_at_unix_ms: 10_000,
        estates: BTreeMap::from([(
            "estate-a".into(),
            BTreeSet::from([
                LocalEstatePermission::PruneRecovery,
                LocalEstatePermission::RestoreRecovery,
            ]),
        )]),
    };
    fs::write(&policy_path, serde_json::to_vec(&policy).unwrap()).unwrap();
    private(&policy_path);

    let authority = rrd_store::RrflowKvStore::open(&database).unwrap();
    let repository = EstateRepository::new(&authority, canonical("estate-a"));
    let estate_context = |at, request: &str, operation: &str| MutationContext {
        at,
        actor: "operator-one".into(),
        request_id: request.into(),
        operation_id: canonical(operation),
    };
    repository
        .create(&estate_context(10, "create-estate", "create-estate"))
        .unwrap();
    repository
        .set_desired(&SetDesired {
            context: estate_context(20, "stop-instance", "stop-instance"),
            instance_id: canonical("instance-a"),
            idempotency_key: "stop-instance".into(),
            target: DesiredTarget {
                phase: DesiredPhase::Stopped,
                deployment_ref: canonical("rrd-server"),
                version: "1.0.0".into(),
                configuration_sha256: "a".repeat(64),
            },
        })
        .unwrap();
    repository
        .acquire_lease(&LeaseRequest {
            context: estate_context(30, "lease-stop", "stop-instance"),
            worker: canonical("estate-worker"),
            lease_ms: 1_000,
        })
        .unwrap();
    for (at, request, boundary, evidence) in [
        (40, "prepare-stop", ReceiptBoundary::Prepared, "b"),
        (50, "apply-stop", ReceiptBoundary::Applied, "c"),
    ] {
        repository
            .record_receipt(&ReceiptRequest {
                context: estate_context(at, request, "stop-instance"),
                worker: canonical("estate-worker"),
                lease_epoch: 1,
                boundary,
                evidence_sha256: evidence.repeat(64),
                error: None,
            })
            .unwrap();
    }
    repository
        .record_observation(&ObservationRequest {
            context: estate_context(60, "observe-stop", "stop-instance"),
            worker: canonical("estate-worker"),
            lease_epoch: 1,
            phase: ObservedPhase::Stopped,
            version: None,
            process_id: None,
            evidence_sha256: "d".repeat(64),
            error: None,
        })
        .unwrap();
    repository
        .record_receipt(&ReceiptRequest {
            context: estate_context(70, "complete-stop", "stop-instance"),
            worker: canonical("estate-worker"),
            lease_epoch: 1,
            boundary: ReceiptBoundary::Completed,
            evidence_sha256: "e".repeat(64),
            error: None,
        })
        .unwrap();
    repository
        .set_recovery_policy(&SetRecoveryPolicy {
            context: estate_context(75, "set-policy", "set-policy"),
            instance_id: canonical("instance-a"),
            idempotency_key: "set-policy".into(),
            max_rpo_ms: 1_000,
            max_rto_ms: 2_000,
            minimum_recovery_points: 1,
            retention_ms: 3_000,
        })
        .unwrap();
    drop(authority);

    for (job, scheduled_at) in [("backup-one", 100), ("backup-two", 200)] {
        let authority = rrd_store::RrflowKvStore::open(&database).unwrap();
        EstateRepository::new(&authority, canonical("estate-a"))
            .schedule_backup(&ScheduleBackup {
                context: estate_context(scheduled_at, job, job),
                instance_id: canonical("instance-a"),
                idempotency_key: job.into(),
                label: job.into(),
            })
            .unwrap();
        drop(authority);
        for at in [scheduled_at + 10, scheduled_at + 20, scheduled_at + 30] {
            RrdEngine::reconcile_estate_backup_store(
                &database,
                canonical("estate-control-instance"),
                &state_root,
                &executable,
                canonical("estate-a"),
                canonical("backup-worker"),
                1_000,
                at,
                None,
            )
            .unwrap();
        }
    }
    let before = rrd_store::verify_backup_catalogue(&catalogue_root).unwrap();
    assert_eq!(before.backups.len(), 2);
    assert!(
        !state_root.join("backups").exists(),
        "the estate controller writes only the instance's own catalogue"
    );
    let first_id = before.backups[0].backup_id.clone();
    let second_id = before.backups[1].backup_id.clone();

    let pruned = RrdEngine::prune_estate_backups_store(
        &database,
        canonical("estate-control-instance"),
        &state_root,
        &executable,
        &policy_path,
        &key_path,
        canonical("estate-a"),
        canonical("instance-a"),
        4_000,
        4_000,
        "prune-request".into(),
        canonical("prune-operation"),
        "prune-daily".into(),
    )
    .unwrap();
    assert!(!pruned.idempotent_replay);
    assert_eq!(pruned.pruned_backup_ids, vec![first_id.clone()]);
    assert_eq!(
        pruned
            .recovery
            .recovery_points
            .iter()
            .find(|point| point.backup_sha256 == first_id)
            .unwrap()
            .pruned_at_unix_ms,
        Some(4_000)
    );
    assert!(pruned.recovery.prune_intents.is_empty());
    let physical = rrd_store::verify_backup_catalogue(&catalogue_root).unwrap();
    assert_eq!(physical.backups.len(), 1);
    assert_eq!(physical.backups[0].backup_id, second_id);
    let prune_replay = RrdEngine::prune_estate_backups_store(
        &database,
        canonical("estate-control-instance"),
        &state_root,
        &executable,
        &policy_path,
        &key_path,
        canonical("estate-a"),
        canonical("instance-a"),
        4_000,
        4_000,
        "prune-request".into(),
        canonical("prune-operation"),
        "prune-daily".into(),
    )
    .unwrap();
    assert!(prune_replay.idempotent_replay);

    // A different distribution is refused before any estate, catalogue, or
    // filesystem state is touched.
    let foreign_executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let mismatch = RrdEngine::prune_estate_backups_store(
        &database,
        canonical("estate-control-instance"),
        &state_root,
        &foreign_executable,
        &policy_path,
        &key_path,
        canonical("estate-a"),
        canonical("instance-a"),
        4_050,
        4_050,
        "prune-foreign-request".into(),
        canonical("prune-foreign-operation"),
        "prune-foreign".into(),
    )
    .unwrap_err();
    assert!(matches!(mismatch, ServiceError::ProjectBindingMismatch));

    let restored = RrdEngine::restore_estate_backup_store(
        &database,
        canonical("estate-control-instance"),
        &state_root,
        &executable,
        &policy_path,
        &key_path,
        canonical("estate-a"),
        canonical("instance-a"),
        second_id.clone(),
        canonical("restore-one"),
        4_100,
        4_200,
        "restore-request".into(),
        canonical("restore-operation"),
        "restore-daily".into(),
    )
    .unwrap();
    assert!(!restored.idempotent_replay);
    assert!(restored.reopened);
    let evidence = restored
        .recovery
        .restore_evidence
        .iter()
        .find(|evidence| evidence.restore_id.as_str() == "restore-one")
        .unwrap();
    assert!(!evidence.rpo_within_objective);
    assert!(evidence.rto_within_objective);
    assert!(restores_root.join("restore-one/CURRENT").is_file());
    assert!(storage_root.join("CURRENT").is_file());
    assert!(
        !state_root.join("restores").exists(),
        "the estate controller stages restores only inside the instance's container"
    );

    let pruned_restore = RrdEngine::restore_estate_backup_store(
        &database,
        canonical("estate-control-instance"),
        &state_root,
        &executable,
        &policy_path,
        &key_path,
        canonical("estate-a"),
        canonical("instance-a"),
        first_id,
        canonical("restore-pruned"),
        4_205,
        4_210,
        "restore-pruned-request".into(),
        canonical("restore-pruned-operation"),
        "restore-pruned".into(),
    )
    .unwrap_err();
    assert!(pruned_restore.to_string().contains("unavailable"));

    let restore_replay = RrdEngine::restore_estate_backup_store(
        &database,
        canonical("estate-control-instance"),
        &state_root,
        &executable,
        &policy_path,
        &key_path,
        canonical("estate-a"),
        canonical("instance-a"),
        second_id.clone(),
        canonical("restore-one"),
        4_100,
        4_200,
        "restore-request".into(),
        canonical("restore-operation"),
        "restore-daily".into(),
    )
    .unwrap();
    assert!(restore_replay.idempotent_replay);

    let gap_target = restores_root.join("restore-gap");
    rrd_store::restore_catalogued_backup(&catalogue_root, &second_id, &gap_target, 4_250).unwrap();
    let recovered_gap = RrdEngine::restore_estate_backup_store(
        &database,
        canonical("estate-control-instance"),
        &state_root,
        &executable,
        &policy_path,
        &key_path,
        canonical("estate-a"),
        canonical("instance-a"),
        second_id.clone(),
        canonical("restore-gap"),
        4_250,
        4_300,
        "restore-gap-request".into(),
        canonical("restore-gap-operation"),
        "restore-gap".into(),
    )
    .unwrap();
    assert!(recovered_gap.idempotent_replay);

    // An existing target that is not a restored store is refused and left
    // empty rather than created as a new database.
    let empty_target = restores_root.join("restore-empty");
    std::fs::create_dir_all(&empty_target).unwrap();
    let empty_error = RrdEngine::restore_estate_backup_store(
        &database,
        canonical("estate-control-instance"),
        &state_root,
        &executable,
        &policy_path,
        &key_path,
        canonical("estate-a"),
        canonical("instance-a"),
        second_id.clone(),
        canonical("restore-empty"),
        4_320,
        4_330,
        "restore-empty-request".into(),
        canonical("restore-empty-operation"),
        "restore-empty".into(),
    )
    .unwrap_err();
    assert!(
        !empty_error.to_string().contains("watermarks differ"),
        "an empty directory is not a store with the wrong watermarks: {empty_error}"
    );
    assert!(
        std::fs::read_dir(&empty_target).unwrap().next().is_none(),
        "a refused restore creates nothing inside the target"
    );

    let divergent_target = restores_root.join("restore-divergent");
    let divergent = rrd_store::RrflowKvStore::open(&divergent_target).unwrap();
    divergent
        .claims()
        .append_batch(&[Claim::new(
            Subject::new("restore:divergent").unwrap(),
            Predicate::new("status").unwrap(),
            "different",
            4_350,
            4_350,
            Producer {
                actor: "test:recovery".into(),
                on_behalf_of: None,
                session: None,
            },
        )])
        .unwrap();
    drop(divergent);
    let divergent_error = RrdEngine::restore_estate_backup_store(
        &database,
        canonical("estate-control-instance"),
        &state_root,
        &executable,
        &policy_path,
        &key_path,
        canonical("estate-a"),
        canonical("instance-a"),
        second_id,
        canonical("restore-divergent"),
        4_350,
        4_400,
        "restore-divergent-request".into(),
        canonical("restore-divergent-operation"),
        "restore-divergent".into(),
    )
    .unwrap_err();
    assert!(divergent_error.to_string().contains("watermarks differ"));
}
