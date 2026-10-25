use rrd_contract::{
    transaction_operation_sha256, ActivateVectorQuantizationArtifact, BeginTransaction,
    BuildVectorQuantizationArtifact, CanonicalId, CapabilityDescriptor, CapabilityStatus,
    CloseSession, CommitReceipt, CommitTransaction, CorrelationId, CreateInstanceBackup,
    DataCatalogueIdentity, DataLogicalModel, DataRecordSchema, DataReference, DataSchemaMode,
    DataSchemaRegistry, DataSnapshot, DataTableSchema, DataTarget, DeleteVectorCollection,
    DeleteVectorPayloadIndex, DeploymentForm, DeploymentProfile, EmbedAndSearchVectors,
    EmbeddingInput, EmbeddingNetworkPolicy, EndpointPresentation, EnsureQueryIndex,
    EnsureVectorCollection, EnsureVectorIndex, EnsureVectorPayloadIndex, ErrorBody, ErrorCode,
    EstateActivityPolicySnapshot, EstateAuthoritySnapshot, EstateBackupJobSnapshot,
    EstateBackupJobState, EstateBackupJobsSnapshot, EstateBackupRecoveryPolicySnapshot,
    EstateConfiguration, EstateMutationResult, EstateSnapshot, ExecuteQuery,
    ExecuteQueryTransaction, ExecuteRetrievalQuery, FollowChangefeed, ForwardRollbackCounts,
    ForwardRollbackRequest, GenerateEmbeddings, HybridFusion, IdempotencyBinding,
    InstalledEstateIdentity, ListQueryIndexes, ListVectorCollections, ListVectorPayloadIndexes,
    ListVectorQuantizationArtifacts, Liveness, NamedVectorDefinition, PollLiveQuery,
    PreviewTransaction, ProductCapability, ProductCapabilityCatalogue, ProductSurface, QueryBudget,
    QueryExecutionAnalysisSnapshot, QueryExecutionSnapshot, QueryIndexKind, QueryPlanCandidate,
    QueryPlanSnapshot, QueryResult, QueryRowSnapshot, QueryValue, ReadAccessPath, ReadAudit,
    ReadChangefeed, ReadDataSnapshot, ReadEstate, ReadEvidence, ReadPathEvidence,
    ReadStampValidationEvidence, ReadStampValidationMethod, Readiness, RenewSession,
    RequestContext, RequestEnvelope, ResourceId, ResourceKind, ResourcePath, ResponseEnvelope,
    ResponseOutcome, RestoreInstanceBackup, RetireVectorQuantizationArtifact, RetrievalFusion,
    RetrievalPrefetch, RetrievalQuery, RetrievalResultShape, SearchHybrid, SearchVectors,
    ServiceCapabilities, SessionEndState, SessionLease, SessionLimits, SessionTermination,
    StorageProfileKind, SurfaceBinding, SurfaceDisposition, TransactionMutation,
    TransactionPreview, TransactionState, VectorIndexBuildPolicy, VectorIndexConfiguration,
    VectorMemoryTier, VectorPayloadCondition, VectorPayloadFilter, VectorPayloadIndexKind,
    VectorPayloadOperator, VectorProductCompression, VectorQuantizationMethod, VectorSearchMetric,
    VectorSearchMode, VectorSearchQuery, VectorValueKind, PROTOCOL, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixturePayload {
    action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractFixture {
    service: ServiceCapabilities,
    request: RequestEnvelope<FixturePayload>,
    success: ResponseEnvelope<FixturePayload>,
    failure: ResponseEnvelope<FixturePayload>,
    idempotency: IdempotencyBinding,
}

#[test]
fn forward_rollback_request_is_strict_and_counts_cannot_overflow() {
    let request: ForwardRollbackRequest = serde_json::from_value(serde_json::json!({
        "target_valid_at": 100,
        "target_known_at_cursor": 7,
        "effective_at": 200,
        "reason": "restore the accepted state"
    }))
    .unwrap();
    request.validate().unwrap();

    let mut unknown = serde_json::to_value(&request).unwrap();
    unknown["rewrite_history"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ForwardRollbackRequest>(unknown).is_err());

    let counts = ForwardRollbackCounts {
        restored_records: u64::MAX,
        retired_records: 1,
        restored_relations: 0,
        retired_relations: 0,
    };
    assert_eq!(counts.checked_mutation_count(), None);
}

fn contract_fixture() -> ContractFixture {
    let request_id = CorrelationId::new("req-01j5y0h4k7").unwrap();
    let operation_id = CorrelationId::new("op-01j5y0h4k8").unwrap();
    let idempotency_key = CorrelationId::new("idem-01j5y0h4k9").unwrap();
    ContractFixture {
        service: ServiceCapabilities {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            implementation: CanonicalId::new("rrflow").unwrap(),
            implementation_version: "1.0.0".into(),
            deployment: DeploymentProfile {
                contract_version: 1,
                deployment_form: DeploymentForm::Embedded,
                storage_profile: StorageProfileKind::RrflowKv,
                endpoint_presentation: EndpointPresentation::InProcess,
            },
            installed_estate: Some(InstalledEstateIdentity {
                project_id: CanonicalId::new("alpha-project").unwrap(),
                estate_id: CanonicalId::new("alpha-estate").unwrap(),
                instance_id: CanonicalId::new("project-alpha").unwrap(),
            }),
            configuration: EstateConfiguration::default(),
            instance: ResourceId::new(ResourceKind::Instance, "project-alpha").unwrap(),
            capabilities: vec![
                CapabilityDescriptor {
                    name: CanonicalId::new("logical-archive").unwrap(),
                    contract_version: 1,
                    status: CapabilityStatus::Unavailable,
                    limits: BTreeMap::new(),
                    limitation: Some("F1 implementation gate is not complete".into()),
                },
                CapabilityDescriptor {
                    name: CanonicalId::new("native-persistence").unwrap(),
                    contract_version: 1,
                    status: CapabilityStatus::Experimental,
                    limits: BTreeMap::from([(
                        CanonicalId::new("max-snapshot-bytes").unwrap(),
                        1_073_741_824,
                    )]),
                    limitation: Some(
                        "local alpha; independent-host qualification remains open".into(),
                    ),
                },
            ],
            product_capabilities: ProductCapabilityCatalogue {
                contract_version: PROTOCOL_VERSION,
                capabilities: vec![ProductCapability {
                    id: "backup-create".into(),
                    label: "Backup Create".into(),
                    category: "backup".into(),
                    summary: "Create one authenticated logical backup.".into(),
                    bindings: vec![
                        SurfaceBinding {
                            surface: ProductSurface::Engine,
                            disposition: SurfaceDisposition::Available,
                            entrypoints: vec![
                                "rrd-engine:RrdOperation::CreateInstanceBackup".into()
                            ],
                            reason: None,
                        },
                        SurfaceBinding {
                            surface: ProductSurface::Rrflowql,
                            disposition: SurfaceDisposition::Unavailable,
                            entrypoints: Vec::new(),
                            reason: Some(
                                "No rrflowQL statement maps this fixture operation.".into(),
                            ),
                        },
                        SurfaceBinding {
                            surface: ProductSurface::Graphql,
                            disposition: SurfaceDisposition::Unavailable,
                            entrypoints: Vec::new(),
                            reason: Some("No GraphQL field maps this fixture operation.".into()),
                        },
                        SurfaceBinding {
                            surface: ProductSurface::RrdHttp,
                            disposition: SurfaceDisposition::Available,
                            entrypoints: vec!["POST /v1/backups/create".into()],
                            reason: None,
                        },
                        SurfaceBinding {
                            surface: ProductSurface::WebSocket,
                            disposition: SurfaceDisposition::Unavailable,
                            entrypoints: Vec::new(),
                            reason: Some(
                                "No WebSocket operation maps this fixture operation.".into(),
                            ),
                        },
                        SurfaceBinding {
                            surface: ProductSurface::Grpc,
                            disposition: SurfaceDisposition::Unavailable,
                            entrypoints: Vec::new(),
                            reason: Some("No gRPC method maps this fixture operation.".into()),
                        },
                        SurfaceBinding {
                            surface: ProductSurface::Mcp,
                            disposition: SurfaceDisposition::Unavailable,
                            entrypoints: Vec::new(),
                            reason: Some("No MCP runtime tool is present in this fixture.".into()),
                        },
                        SurfaceBinding {
                            surface: ProductSurface::Cli,
                            disposition: SurfaceDisposition::Unavailable,
                            entrypoints: Vec::new(),
                            reason: Some("No CLI command is present in this fixture.".into()),
                        },
                        SurfaceBinding {
                            surface: ProductSurface::Sdk,
                            disposition: SurfaceDisposition::Available,
                            entrypoints: vec!["openapi:operation#backup-create".into()],
                            reason: None,
                        },
                        SurfaceBinding {
                            surface: ProductSurface::Connectome,
                            disposition: SurfaceDisposition::Planned,
                            entrypoints: Vec::new(),
                            reason: Some("Connectome exposure is planned in this fixture.".into()),
                        },
                    ],
                }],
            },
        },
        request: RequestEnvelope {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            context: RequestContext {
                request_id: request_id.clone(),
                operation_id: operation_id.clone(),
                idempotency_key: Some(idempotency_key.clone()),
                deadline_unix_ms: Some(1_800_000_000_000),
            },
            resource: ResourcePath {
                segments: vec![
                    ResourceId::new(ResourceKind::Organization, "local").unwrap(),
                    ResourceId::new(ResourceKind::Estate, "developer").unwrap(),
                    ResourceId::new(ResourceKind::Project, "alpha").unwrap(),
                    ResourceId::new(ResourceKind::Instance, "project-alpha").unwrap(),
                ],
            },
            payload: FixturePayload {
                action: "backup.create".into(),
            },
        },
        success: ResponseEnvelope {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.clone(),
            operation_id: operation_id.clone(),
            outcome: ResponseOutcome::Ok {
                payload: FixturePayload {
                    action: "backup.created".into(),
                },
            },
        },
        failure: ResponseEnvelope {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            request_id,
            operation_id,
            outcome: ResponseOutcome::Error {
                error: ErrorBody {
                    code: ErrorCode::FailedPrecondition,
                    message: "source read stamp is no longer current".into(),
                    retryable: true,
                    details: BTreeMap::from([(
                        CanonicalId::new("observed-cursor").unwrap(),
                        "42".into(),
                    )]),
                },
            },
        },
        idempotency: IdempotencyBinding {
            key: idempotency_key,
            operation_sha256: "3c94150b4ea4f9dcb27d3b602e9f190debe656533c047b99e11367bc6a28017f"
                .into(),
        },
    }
}

#[test]
fn public_contract_matches_frozen_golden_json() {
    let fixture = contract_fixture();
    fixture.service.validate().unwrap();
    fixture.request.validate(true).unwrap();
    fixture.success.validate().unwrap();
    fixture.failure.validate().unwrap();
    fixture.idempotency.validate().unwrap();

    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/public-contract-v1.json")).unwrap();
    let actual = serde_json::to_value(&fixture).unwrap();
    assert!(actual["service"].get("deployment_mode").is_none());
    assert_eq!(
        actual,
        expected,
        "{}",
        serde_json::to_string_pretty(&actual).unwrap()
    );

    let reopened: ContractFixture = serde_json::from_value(expected).unwrap();
    assert_eq!(reopened, fixture);
}

#[test]
fn deployment_axes_and_shared_conformance_corpus_are_versioned_and_strict() {
    let corpus: rrd_contract::DeploymentConformanceCorpus = serde_json::from_str(include_str!(
        "../../../../fixtures/rrd-deployment-conformance-v1.json"
    ))
    .unwrap();
    corpus.validate().unwrap();
    assert_eq!(corpus.expected_ids[0].as_str(), "alpha");
    assert_eq!(
        serde_json::to_value([
            DeploymentForm::Embedded,
            DeploymentForm::SingleNodeServer,
            DeploymentForm::ClusteredServer,
        ])
        .unwrap(),
        serde_json::json!(["embedded", "single_node_server", "clustered_server"])
    );
    assert_eq!(
        serde_json::to_value([StorageProfileKind::RrflowMx, StorageProfileKind::RrflowKv]).unwrap(),
        serde_json::json!(["rrflow_mx", "rrflow_kv"])
    );
    assert_eq!(
        serde_json::to_value([
            EndpointPresentation::InProcess,
            EndpointPresentation::LoopbackHttpWebsocket,
            EndpointPresentation::NetworkHttpWebsocket,
        ])
        .unwrap(),
        serde_json::json!([
            "in_process",
            "loopback_http_websocket",
            "network_http_websocket"
        ])
    );

    let mut unknown = serde_json::to_value(&corpus).unwrap();
    unknown["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<rrd_contract::DeploymentConformanceCorpus>(unknown).is_err());
}

#[test]
fn supported_sdks_share_one_strict_semantic_corpus() {
    let corpus: rrd_contract::SdkConformanceCorpus = serde_json::from_str(include_str!(
        "../../../../fixtures/rrd-sdk-conformance-v1.json"
    ))
    .unwrap();
    corpus.validate().unwrap();
    assert_eq!(corpus.required_domains.len(), 13);
    assert_eq!(corpus.expected.endpoint_count, 55);
    assert_eq!(
        rrd_contract::transaction_operation_sha256(&corpus.transaction.commit.mutations),
        corpus.transaction.commit.operation_sha256
    );

    let mut unknown = serde_json::to_value(&corpus).unwrap();
    unknown["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<rrd_contract::SdkConformanceCorpus>(unknown).is_err());
}

#[test]
fn malformed_identifiers_and_unknown_fields_fail_during_decode() {
    assert!(serde_json::from_str::<CanonicalId>(r#""Not Canonical""#).is_err());
    assert!(CorrelationId::new("request/id").is_err());

    let mut request = serde_json::to_value(&contract_fixture().request).unwrap();
    request
        .as_object_mut()
        .unwrap()
        .insert("unknown".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<RequestEnvelope<FixturePayload>>(request).is_err());
}

#[test]
fn estate_read_contract_has_frozen_field_names_and_strict_empty_payload() {
    let snapshot = EstateSnapshot {
        format_version: 1,
        id: CanonicalId::new("estate-a").unwrap(),
        revision: 7,
        created_at_unix_ms: 10,
        updated_at_unix_ms: 20,
        activity_policy: EstateActivityPolicySnapshot {
            idle_after_ms: 300_000,
            stale_after_ms: 1_800_000,
            neglected_after_ms: 604_800_000,
        },
        authority: EstateAuthoritySnapshot::empty(),
        instances: Vec::new(),
        operations: Vec::new(),
        idempotency_binding_count: 0,
    };
    let expected = serde_json::json!({
        "format_version": 1,
        "id": "estate-a",
        "revision": 7,
        "created_at_unix_ms": 10,
        "updated_at_unix_ms": 20,
        "activity_policy": {
            "idle_after_ms": 300000,
            "stale_after_ms": 1800000,
            "neglected_after_ms": 604800000
        },
        "authority": serde_json::to_value(EstateAuthoritySnapshot::empty()).unwrap(),
        "instances": [],
        "operations": [],
        "idempotency_binding_count": 0
    });
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<EstateSnapshot>(expected).unwrap(),
        snapshot
    );
    let mutation = EstateMutationResult {
        estate: snapshot,
        idempotent_replay: true,
    };
    let mutation_json = serde_json::to_value(&mutation).unwrap();
    assert_eq!(mutation_json["estate"]["id"], "estate-a");
    assert_eq!(mutation_json["idempotent_replay"], true);
    let mut unknown = mutation_json;
    unknown
        .as_object_mut()
        .unwrap()
        .insert("unknown".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<EstateMutationResult>(unknown).is_err());
    assert_eq!(
        serde_json::from_str::<ReadEstate>("{}").unwrap(),
        ReadEstate {}
    );
    assert!(serde_json::from_str::<ReadEstate>(r#"{"unknown":true}"#).is_err());
}

#[test]
fn estate_backup_jobs_are_a_separate_strict_public_resource() {
    let snapshot = EstateBackupJobsSnapshot {
        estate_id: CanonicalId::new("estate-a").unwrap(),
        estate_revision: 8,
        jobs: vec![EstateBackupJobSnapshot {
            id: CanonicalId::new("backup-daily").unwrap(),
            instance_id: CanonicalId::new("instance-a").unwrap(),
            source_generation: 1,
            label: "daily.0001".into(),
            request_sha256: "a".repeat(64),
            state: EstateBackupJobState::Pending,
            attempts: 0,
            created_at_unix_ms: 80,
            updated_at_unix_ms: 80,
            recovery_policy: EstateBackupRecoveryPolicySnapshot {
                revision: 1,
                max_rpo_ms: 86_400_000,
                max_rto_ms: 3_600_000,
                minimum_recovery_points: 1,
                retention_ms: 604_800_000,
            },
            lease: None,
            receipts: Vec::new(),
            backup_id: None,
            archive_sha256: None,
            catalogue_sha256: None,
            error: None,
        }],
    };
    let expected = serde_json::json!({
        "estate_id": "estate-a",
        "estate_revision": 8,
        "jobs": [{
            "id": "backup-daily",
            "instance_id": "instance-a",
            "source_generation": 1,
            "label": "daily.0001",
            "request_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "state": "pending",
            "attempts": 0,
            "created_at_unix_ms": 80,
            "updated_at_unix_ms": 80,
            "recovery_policy": {
                "revision": 1,
                "max_rpo_ms": 86400000,
                "max_rto_ms": 3600000,
                "minimum_recovery_points": 1,
                "retention_ms": 604800000
            }
        }]
    });
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<EstateBackupJobsSnapshot>(expected.clone()).unwrap(),
        snapshot
    );
    let mut missing_policy = expected.clone();
    missing_policy["jobs"][0]
        .as_object_mut()
        .unwrap()
        .remove("recovery_policy");
    assert!(serde_json::from_value::<EstateBackupJobsSnapshot>(missing_policy).is_err());
    let mut unknown = expected;
    unknown["jobs"][0]["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<EstateBackupJobsSnapshot>(unknown).is_err());
}

#[test]
fn estate_recovery_posture_is_strict_bounded_by_identity_and_path_free() {
    let recovery = serde_json::json!({
        "estate_id": "estate-a",
        "estate_revision": 12,
        "policies": [{
            "instance_id": "instance-a",
            "revision": 1,
            "max_rpo_ms": 86400000,
            "max_rto_ms": 3600000,
            "minimum_recovery_points": 2,
            "retention_ms": 604800000,
            "updated_at_unix_ms": 80
        }],
        "recovery_points": [],
        "retention_pins": [],
        "restore_evidence": [],
        "prune_intents": []
    });
    let decoded: rrd_contract::EstateRecoverySnapshot =
        serde_json::from_value(recovery.clone()).unwrap();
    assert_eq!(decoded.policies[0].revision, 1);
    assert!(!serde_json::to_string(&decoded).unwrap().contains("path"));
    let mut unknown = recovery;
    unknown["policies"][0]["target_path"] = serde_json::json!("/tmp/escape");
    assert!(serde_json::from_value::<rrd_contract::EstateRecoverySnapshot>(unknown).is_err());
}

#[test]
fn query_contract_is_transport_neutral_bounded_and_strict() {
    let request = ExecuteQuery {
        scope: "instance:project-alpha".into(),
        query: "FROM record:document AT VALID 42 KNOWN HEAD PROJECT title".into(),
        parameters: BTreeMap::from([("title".into(), QueryValue::String("alpha".into()))]),
        budget: QueryBudget::default(),
    };
    request.validate().unwrap();
    let encoded = serde_json::to_value(&request).unwrap();
    assert_eq!(encoded["scope"], "instance:project-alpha");
    let mut unknown = encoded.clone();
    unknown["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ExecuteQuery>(unknown).is_err());

    let result = QueryResult {
        canonical_query: request.query.clone(),
        scope: request.scope.clone(),
        read_manifest_sha256: "a".repeat(64),
        known_at_cursor: 7,
        schema_revision: 1,
        plan: QueryPlanSnapshot {
            plan_sha256: "b".repeat(64),
            security_policy_revision: 4,
            authorization_sha256: "c".repeat(64),
            exact: true,
            deterministic_order: "identity_ascending".into(),
            authorization_boundary: "scope:instance:project-alpha".into(),
            candidates: vec![QueryPlanCandidate {
                name: "versioned_source_read".into(),
                selected: true,
                exact: true,
                reason: "direct semantic-version fixture".into(),
            }],
        },
        execution: QueryExecutionSnapshot {
            selected_versions: 7,
            read_evidence: read_evidence(),
            returned_rows: 1,
            output_bytes: 32,
            truncated: false,
            analysis: Some(QueryExecutionAnalysisSnapshot {
                engine: "datafusion-55".into(),
                provider_scans: 1,
                input_rows: 2,
                input_batches: 1,
                input_memory_bytes: 512,
                output_batches: 1,
                projection_pushdown: "exact".into(),
                filter_pushdown: "unsupported_exact_post_scan".into(),
                limit_pushdown: "retained_above_scan".into(),
                physical_operators: 5,
                peak_memory_bytes: 1_024,
                spill_count: 0,
                spilled_bytes: 0,
                spilled_rows: 0,
                elapsed_micros: 50,
            }),
        },
        rows: vec![QueryRowSnapshot {
            identity: "document:alpha".into(),
            values: BTreeMap::from([("title".into(), QueryValue::String("Alpha".into()))]),
        }],
    };
    assert_eq!(
        serde_json::from_value::<QueryResult>(serde_json::to_value(&result).unwrap()).unwrap(),
        result
    );

    let mut invalid_budget = request_budget_fixture();
    invalid_budget.max_rows = 0;
    assert!(invalid_budget.validate().is_err());
    let minimal_budget: QueryBudget = serde_json::from_value(serde_json::json!({
        "max_storage_keys": 100,
        "max_rows": 10,
        "max_output_bytes": 4096,
        "max_batch_rows": 10
    }))
    .unwrap();
    assert_eq!(minimal_budget.max_memory_bytes, 64 * 1024 * 1024);
    assert_eq!(minimal_budget.max_spill_bytes, 256 * 1024 * 1024);
    assert_eq!(minimal_budget.max_elapsed_ms, 30_000);
    minimal_budget.validate().unwrap();
    let mut invalid_memory = minimal_budget;
    invalid_memory.max_memory_bytes = rrd_contract::MAX_QUERY_MEMORY_BYTES + 1;
    assert!(invalid_memory.validate().is_err());
    let mut invalid_parameter = request.clone();
    invalid_parameter
        .parameters
        .insert("nested".into(), QueryValue::List(Vec::new()));
    assert!(invalid_parameter.validate().is_err());
}

#[test]
fn query_transaction_contract_reuses_typed_mutations_and_is_bounded() {
    let request = ExecuteQueryTransaction {
        scope: "instance:project-alpha".into(),
        program: "BEGIN; MUTATE $document; COMMIT;".into(),
        mutation_bindings: BTreeMap::from([(
            "document".into(),
            TransactionMutation::PutRecord {
                reference: DataReference {
                    kind: CanonicalId::new("document").unwrap(),
                    id: CanonicalId::new("alpha").unwrap(),
                },
                valid_from: 100,
                valid_to: None,
                properties: BTreeMap::from([("title".into(), QueryValue::String("Alpha".into()))]),
            },
        )]),
        timeout_ms: 1_000,
    };
    request.validate().unwrap();
    let mut unknown = serde_json::to_value(&request).unwrap();
    unknown["autocommit"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ExecuteQueryTransaction>(unknown).is_err());

    let mut invalid = request.clone();
    invalid.timeout_ms = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = request;
    invalid.mutation_bindings.clear();
    assert!(invalid.validate().is_err());
}

fn request_budget_fixture() -> QueryBudget {
    QueryBudget::default()
}

fn read_evidence() -> ReadEvidence {
    ReadEvidence {
        contract_version: 1,
        key_budget: 64,
        point_reads: 1,
        range_scans: 1,
        keys_examined: 2,
        values_decoded: 1,
        decoded_bytes: 64,
        stamp_validation: ReadStampValidationEvidence {
            method: ReadStampValidationMethod::AuthenticatedCurrentHead,
            change_reads: 0,
            proof_nodes: 0,
        },
        paths: vec![
            ReadPathEvidence {
                path: ReadAccessPath::ReadStamp,
                point_reads: 1,
                range_scans: 0,
                keys_examined: 1,
                values_decoded: 0,
                decoded_bytes: 0,
            },
            ReadPathEvidence {
                path: ReadAccessPath::RecordVersions,
                point_reads: 0,
                range_scans: 1,
                keys_examined: 1,
                values_decoded: 1,
                decoded_bytes: 64,
            },
        ],
    }
}

#[test]
fn live_query_contract_is_resumable_bounded_and_strict() {
    let request = PollLiveQuery {
        scope: "instance:project-alpha".into(),
        query: "FROM record:document AT VALID 42 KNOWN HEAD PROJECT title".into(),
        parameters: BTreeMap::new(),
        after_cursor: 41,
        budget: QueryBudget::default(),
        max_delta_rows: 256,
        wait_timeout_ms: 1_000,
    };
    request.validate().unwrap();
    let mut encoded = serde_json::to_value(&request).unwrap();
    encoded["stream"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PollLiveQuery>(encoded).is_err());
    let mut invalid_wait = request.clone();
    invalid_wait.wait_timeout_ms = 5_001;
    assert!(invalid_wait.validate().is_err());
    let mut invalid = request;
    invalid.max_delta_rows = 0;
    assert!(invalid.validate().is_err());
}

#[test]
fn query_index_administration_contract_is_bounded_and_strict() {
    EnsureQueryIndex {
        scope: "instance:project-alpha".into(),
        index_id: CanonicalId::new("documents-by-status").unwrap(),
        definition_query: "FROM record:document AT VALID 42 KNOWN HEAD PROJECT status, title"
            .into(),
        unique: false,
        kind: QueryIndexKind::Scalar,
        full_text: None,
        budget: QueryBudget::default(),
    }
    .validate()
    .unwrap();
    assert!(
        serde_json::from_value::<EnsureVectorIndex>(serde_json::json!({
            "scope": "instance:project-alpha",
            "collection_id": "documents",
            "vector_name": "title",
            "configuration": {
                "kind": "turbo_quant",
                "bits": "bits2",
                "seed": 11,
                "filter_properties": ["tenant"]
            },
            "build_policy": {"kind": "cpu"},
            "max_storage_keys": 10000
        }))
        .is_err()
    );
    ListQueryIndexes {
        scope: "instance:project-alpha".into(),
    }
    .validate()
    .unwrap();
    assert!(ListQueryIndexes {
        scope: String::new()
    }
    .validate()
    .is_err());
}

#[test]
fn vector_quantization_lifecycle_contract_is_bounded_and_strict() {
    let build = BuildVectorQuantizationArtifact {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("title").unwrap(),
        method: VectorQuantizationMethod::Product {
            compression: VectorProductCompression::X64,
        },
        filter_properties: vec![CanonicalId::new("tenant").unwrap()],
        max_storage_keys: 10_000,
    };
    build.validate().unwrap();
    assert_eq!(build.method.maximum_compression_ratio(), 64);

    let mut duplicate_filter = build.clone();
    duplicate_filter
        .filter_properties
        .push(CanonicalId::new("tenant").unwrap());
    assert!(duplicate_filter.validate().is_err());
    let mut unbounded = build.clone();
    unbounded.max_storage_keys = 0;
    assert!(unbounded.validate().is_err());
    let mut unknown = serde_json::to_value(&build).unwrap();
    unknown["always_ram"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BuildVectorQuantizationArtifact>(unknown).is_err());

    ListVectorQuantizationArtifacts {
        scope: build.scope.clone(),
        collection_id: Some(build.collection_id.clone()),
        vector_name: None,
        max_artifacts: 100,
    }
    .validate()
    .unwrap();
    assert!(ListVectorQuantizationArtifacts {
        scope: build.scope.clone(),
        collection_id: None,
        vector_name: None,
        max_artifacts: 0,
    }
    .validate()
    .is_err());

    let artifact_id = CanonicalId::new("quant-product-documents-title").unwrap();
    ActivateVectorQuantizationArtifact {
        scope: build.scope.clone(),
        artifact_id: artifact_id.clone(),
        generation: 1,
    }
    .validate()
    .unwrap();
    assert!(RetireVectorQuantizationArtifact {
        scope: build.scope,
        artifact_id,
        generation: 0,
    }
    .validate()
    .is_err());
}

#[test]
fn vector_collection_contract_is_named_bounded_and_search_addressable() {
    let vector = NamedVectorDefinition {
        name: CanonicalId::new("title").unwrap(),
        field: CanonicalId::new("title_embedding").unwrap(),
        kind: VectorValueKind::Dense,
        dimensions: 2,
        metric: VectorSearchMetric::Cosine,
        embedding_model: None,
        memory_tier: VectorMemoryTier::Cached,
    };
    EnsureVectorCollection {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vectors: vec![vector.clone()],
    }
    .validate()
    .unwrap();
    ListVectorCollections {
        scope: "instance:project-alpha".into(),
    }
    .validate()
    .unwrap();
    EnsureVectorPayloadIndex {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        field: CanonicalId::new("tenant").unwrap(),
        kind: VectorPayloadIndexKind::Keyword,
    }
    .validate()
    .unwrap();
    ListVectorPayloadIndexes {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
    }
    .validate()
    .unwrap();
    DeleteVectorPayloadIndex {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        field: CanonicalId::new("tenant").unwrap(),
    }
    .validate()
    .unwrap();
    DeleteVectorCollection {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        valid_at: 42,
        max_storage_keys: 10_000,
    }
    .validate()
    .unwrap();
    assert!(DeleteVectorCollection {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        valid_at: 0,
        max_storage_keys: 10_000,
    }
    .validate()
    .is_err());
    EnsureVectorIndex {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("title").unwrap(),
        configuration: VectorIndexConfiguration::Hnsw {
            m: 16,
            ef_construction: 100,
            max_level: 8,
            seed: 7,
            filter_properties: vec![CanonicalId::new("tenant").unwrap()],
        },
        build_policy: VectorIndexBuildPolicy::Cpu,
        max_storage_keys: 10_000,
    }
    .validate()
    .unwrap();
    SearchVectors {
        scope: "instance:project-alpha".into(),
        valid_at: 42,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: Some(vector.name),
        field: None,
        query: VectorSearchQuery::Dense {
            values: vec![0.6, 0.8],
        },
        filter: Some(VectorPayloadFilter::Condition {
            condition: VectorPayloadCondition {
                property: CanonicalId::new("tenant").unwrap(),
                operator: VectorPayloadOperator::Equals {
                    value: QueryValue::String("alpha".into()),
                },
            },
        }),
        metric: None,
        top_k: 10,
        mode: VectorSearchMode::Exact,
        max_storage_keys: 100,
    }
    .validate()
    .unwrap();
    let approximate = SearchVectors {
        scope: "instance:project-alpha".into(),
        valid_at: 42,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: Some(CanonicalId::new("title").unwrap()),
        field: None,
        query: VectorSearchQuery::Dense {
            values: vec![0.6, 0.8],
        },
        filter: None,
        metric: None,
        top_k: 10,
        mode: VectorSearchMode::RequireApproximate {
            exact_rerank: 20,
            ef_search: 100,
        },
        max_storage_keys: 100,
    };
    approximate.validate().unwrap();
    SearchHybrid {
        scope: "instance:project-alpha".into(),
        valid_at: 42,
        document_kind: CanonicalId::new("document").unwrap(),
        text_field: CanonicalId::new("title").unwrap(),
        text_query: "reason ready flow".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("title").unwrap(),
        vector_query: VectorSearchQuery::Dense {
            values: vec![0.6, 0.8],
        },
        vector_filter: None,
        vector_mode: VectorSearchMode::RequireApproximate {
            exact_rerank: 20,
            ef_search: 100,
        },
        fusion: HybridFusion::ReciprocalRank {
            rank_constant: 60,
            text_weight_millionths: 1_000_000,
            vector_weight_millionths: 1_000_000,
        },
        top_k: 10,
        candidate_k: 20,
        max_storage_keys: 100,
    }
    .validate()
    .unwrap();
    let mut invalid_approximate = approximate;
    invalid_approximate.mode = VectorSearchMode::AllowApproximate {
        exact_rerank: 9,
        ef_search: 100,
    };
    assert!(invalid_approximate.validate().is_err());
    assert!(SearchVectors {
        scope: "instance:project-alpha".into(),
        valid_at: 42,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: None,
        field: None,
        query: VectorSearchQuery::Dense {
            values: vec![0.6, 0.8],
        },
        filter: None,
        metric: None,
        top_k: 10,
        mode: VectorSearchMode::Exact,
        max_storage_keys: 100,
    }
    .validate()
    .is_err());
    let point: TransactionMutation = serde_json::from_value(serde_json::json!({
        "mutation": "put_vector",
        "reference": {"kind": "embedding", "id": "alpha-title"},
        "subject": {"kind": "document", "id": "alpha"},
        "collection_id": "documents",
        "vector_name": "title",
        "field": "title_embedding",
        "valid_from": 42,
        "value": {"kind": "dense", "values": [0.6, 0.8]},
        "properties": {"tenant": {"type": "string", "value": "alpha"}}
    }))
    .unwrap();
    point.validate().unwrap();
    let mut incomplete = serde_json::to_value(point).unwrap();
    incomplete.as_object_mut().unwrap().remove("vector_name");
    assert!(serde_json::from_value::<TransactionMutation>(incomplete).is_err());
}

#[test]
fn changefeed_request_is_cursor_addressed_bounded_and_strict() {
    let request = ReadChangefeed {
        scope: "instance:project-alpha".into(),
        after_cursor: 41,
        limit: 256,
    };
    request.validate().unwrap();
    let mut encoded = serde_json::to_value(&request).unwrap();
    encoded["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ReadChangefeed>(encoded).is_err());

    let mut invalid = request.clone();
    invalid.limit = 0;
    assert!(invalid.validate().is_err());
    invalid.limit = rrd_contract::MAX_CHANGEFEED_PAGE + 1;
    assert!(invalid.validate().is_err());

    let mut follow = FollowChangefeed {
        read: request,
        wait_timeout_ms: 1_000,
    };
    follow.validate().unwrap();
    follow.wait_timeout_ms = rrd_contract::MAX_CHANGEFEED_WAIT_MS + 1;
    assert!(follow.validate().is_err());
}

#[test]
fn managed_backup_contract_accepts_no_filesystem_paths() {
    CreateInstanceBackup {
        label: "before-upgrade".into(),
        created_at_unix_ms: 500,
    }
    .validate()
    .unwrap();

    RestoreInstanceBackup {
        backup_sha256: "a".repeat(64),
        restore_id: CanonicalId::new("restore-a").unwrap(),
        restored_at_unix_ms: 600,
    }
    .validate()
    .unwrap();

    let with_path = serde_json::json!({
        "label": "before-upgrade",
        "created_at_unix_ms": 500,
        "target": "/tmp/escape"
    });
    assert!(serde_json::from_value::<CreateInstanceBackup>(with_path).is_err());

    let invalid = RestoreInstanceBackup {
        backup_sha256: "not-a-digest".into(),
        restore_id: CanonicalId::new("restore-a").unwrap(),
        restored_at_unix_ms: 600,
    };
    assert!(invalid.validate().is_err());
}

#[test]
fn audit_read_contract_is_bounded_and_strict() {
    ReadAudit {
        after_sequence: 41,
        limit: 128,
    }
    .validate()
    .unwrap();
    assert!(ReadAudit {
        after_sequence: 0,
        limit: 0,
    }
    .validate()
    .is_err());
    assert!(serde_json::from_value::<ReadAudit>(serde_json::json!({
        "after_sequence": 0,
        "limit": 1,
        "include_bodies": true
    }))
    .is_err());
    rrd_contract::ExportAudit {
        after_sequence: 41,
        limit: 128,
    }
    .validate()
    .unwrap();
    assert!(
        serde_json::from_value::<rrd_contract::ExportAudit>(serde_json::json!({
            "after_sequence": 0,
            "limit": 1,
            "target_path": "/tmp/audit"
        }))
        .is_err()
    );
}

#[test]
fn audit_export_rejects_content_record_and_chain_substitution() {
    let mut record = rrd_contract::AuditRecordSnapshot {
        sequence: 7,
        audit_id: CanonicalId::new("audit-contract-record").unwrap(),
        at_unix_ms: 100,
        principal_id: Some(CanonicalId::new("contract-auditor").unwrap()),
        action: rrd_contract::SecurityAction::AuditRead,
        resource: ResourcePath {
            segments: vec![ResourceId::new(ResourceKind::Instance, "contract-instance").unwrap()],
        },
        request_id: "request-audit-contract".into(),
        operation_id: "operation-audit-contract".into(),
        phase: rrd_contract::AuditPhase::Completed,
        decision: rrd_contract::AuditDecision::Allowed,
        status_code: 200,
        request_sha256: "a".repeat(64),
        response_sha256: "b".repeat(64),
        previous_audit_sha256: Some("c".repeat(64)),
        audit_sha256: String::new(),
    };
    record.audit_sha256 = sha256_for_test(
        &serde_json::to_vec(&(
            &record.audit_id,
            record.at_unix_ms,
            &record.principal_id,
            record.action,
            &record.resource,
            &record.request_id,
            &record.operation_id,
            record.phase,
            record.decision,
            record.status_code,
            &record.request_sha256,
            &record.response_sha256,
            &record.previous_audit_sha256,
        ))
        .unwrap(),
    );
    let json_lines = format!("{}\n", serde_json::to_string(&record).unwrap());
    let export = rrd_contract::AuditExport {
        requested_after_sequence: 0,
        through_sequence: 8,
        chain_anchor_sha256: record.previous_audit_sha256.clone(),
        chain_head_sha256: Some(record.audit_sha256.clone()),
        record_count: 1,
        media_type: "application/x-ndjson; profile=rrd-audit-v1".into(),
        content_sha256: sha256_for_test(json_lines.as_bytes()),
        json_lines,
    };
    export.validate().unwrap();

    let mut content_tampered = export.clone();
    content_tampered.json_lines.push(' ');
    assert!(content_tampered.validate().is_err());

    let mut record_tampered = record;
    record_tampered.response_sha256 = "d".repeat(64);
    let substituted = format!("{}\n", serde_json::to_string(&record_tampered).unwrap());
    let mut sealed_substitution = export;
    sealed_substitution.content_sha256 = sha256_for_test(substituted.as_bytes());
    sealed_substitution.json_lines = substituted;
    assert!(sealed_substitution.validate().is_err());
}

#[test]
fn diagnostic_read_contract_is_bounded_strict_and_separately_authorized() {
    rrd_contract::ReadDiagnosticSnapshot {
        scope: "instance:alpha".into(),
        graph_valid_at_unix_ms: 1_000,
        graph_known_at_cursor: Some(30),
        graph_compare_cursor: 10,
        runtime_max_scanned_changes: 1_024,
        changes_after_cursor: 10,
        change_limit: 128,
        audit_after_sequence: 20,
        audit_limit: 128,
    }
    .validate()
    .unwrap();
    assert!(rrd_contract::ReadDiagnosticSnapshot {
        scope: "instance:alpha".into(),
        graph_valid_at_unix_ms: 1_000,
        graph_known_at_cursor: Some(30),
        graph_compare_cursor: 10,
        runtime_max_scanned_changes: 1_024,
        changes_after_cursor: 0,
        change_limit: 0,
        audit_after_sequence: 0,
        audit_limit: 1,
    }
    .validate()
    .is_err());
    assert!(
        serde_json::from_value::<rrd_contract::ReadDiagnosticSnapshot>(serde_json::json!({
            "scope": "instance:alpha",
            "graph_valid_at_unix_ms": 1000,
            "graph_known_at_cursor": 30,
            "graph_compare_cursor": 10,
            "runtime_max_scanned_changes": 1024,
            "changes_after_cursor": 0,
            "change_limit": 1,
            "audit_after_sequence": 0,
            "audit_limit": 1,
            "physical_store": "/tmp/escape"
        }))
        .is_err()
    );

    let descriptor = rrd_contract::endpoint_catalogue()
        .endpoints
        .into_iter()
        .find(|endpoint| endpoint.operation.as_str() == "diagnostics-read")
        .unwrap();
    assert_eq!(descriptor.path, "/v1/diagnostics/read");
    assert_eq!(
        descriptor.action.fixed_action(),
        Some(rrd_contract::SecurityAction::DiagnosticsRead)
    );
    assert!(!descriptor.mutation);
}

fn rrflow_kv_storage_counters() -> rrd_contract::DiagnosticRrflowKvSnapshot {
    rrd_contract::DiagnosticRrflowKvSnapshot {
        physical_sequence: 12,
        manifest_generation: 2,
        durable_sequence: 10,
        memtable: rrd_contract::DiagnosticMemtableSnapshot {
            versions: 4,
            max_versions: 64,
            approximate_bytes: 512,
            keys: 4,
            key_payload_bytes: 96,
            value_payload_bytes: 256,
            tombstones: 0,
            spilled_chains: 0,
            spilled_version_capacity: 0,
            version_record_bytes: 128,
            owned_bytes_lower_bound: 480,
        },
        write_ahead_log: rrd_contract::DiagnosticWriteAheadLogSnapshot {
            payload_bytes: 2_048,
            payload_max_bytes: 1 << 20,
        },
        maintenance: rrd_contract::DiagnosticMaintenanceSnapshot {
            automatic_flushes: 1,
            failed_flushes: 0,
            write_stalls: 0,
            oversized_batches: 0,
            automatic_compactions: 0,
            failed_compactions: 0,
            compaction_input_bytes: 0,
            compaction_output_bytes: 0,
            peak_compaction_buffer_bytes: 0,
        },
        segment_set: rrd_contract::DiagnosticSegmentSetSnapshot {
            segments: 1,
            segment_bytes: 4_096,
            l0_segments: 1,
            l0_compaction_trigger: 4,
            compaction_debt_segments: 0,
            compaction_target_segment_bytes: 1 << 26,
        },
        page_cache: rrd_contract::DiagnosticPageCacheSnapshot {
            capacity_bytes: 1 << 24,
            resident_bytes: 4_096,
            entries: 1,
            hits: 9,
            misses: 1,
            evictions: 0,
            page_loads: 1,
            bytes_read: 4_096,
            bytes_decoded: 4_096,
            bytes_borrowed: 0,
            bytes_allocated: 4_096,
            bytes_copied: 0,
            bytes_decompressed: 0,
            filter_checks: 3,
            filter_negatives: 2,
        },
        segment_io: rrd_contract::DiagnosticSegmentIoSnapshot {
            max_open_files: 128,
            open_files: 0,
            peak_open_files: 0,
            handle_evictions: 0,
            handle_reopens: 0,
            handle_denials: 0,

            requested_mode: rrd_contract::DiagnosticSegmentIoMode::Auto,
            mmap_segments: 0,
            io_uring_segments: 0,
            bounded_segments: 1,
            fallbacks: 0,
            last_fallback_reason: None,
            read_operations: 1,
            mmap_reads: 0,
            io_uring_reads: 0,
            bounded_reads: 1,
            bytes_read: 4_096,
            max_request_bytes: 1 << 20,
            peak_request_bytes: 4_096,
        },
    }
}

#[test]
fn diagnostic_storage_observation_matches_its_profile_and_evidence_level() {
    use rrd_contract::{DiagnosticStorageEvidenceLevel as Level, StorageProfileKind as Profile};
    let storage = |profile, evidence_level, rrflow_kv| rrd_contract::DiagnosticStorageSnapshot {
        admission: None,
        observed_at_unix_ms: 1_000,
        profile,
        evidence_level,
        rrflow_kv,
    };

    storage(
        Profile::RrflowKv,
        Level::RrflowKvCounters,
        Some(rrflow_kv_storage_counters()),
    )
    .validate()
    .unwrap();
    storage(Profile::RrflowMx, Level::LogicalOnly, None)
        .validate()
        .unwrap();
    assert!(storage(Profile::RrflowKv, Level::RrflowKvCounters, None)
        .validate()
        .is_err());
    assert!(storage(
        Profile::RrflowMx,
        Level::RrflowKvCounters,
        Some(rrflow_kv_storage_counters())
    )
    .validate()
    .is_err());
    assert!(storage(
        Profile::RrflowKv,
        Level::LogicalOnly,
        Some(rrflow_kv_storage_counters())
    )
    .validate()
    .is_err());
    let mut empty_reason = rrflow_kv_storage_counters();
    empty_reason.segment_io.last_fallback_reason = Some(String::new());
    assert!(storage(
        Profile::RrflowKv,
        Level::RrflowKvCounters,
        Some(empty_reason)
    )
    .validate()
    .is_err());
    let mut observed_at_zero = storage(Profile::RrflowMx, Level::LogicalOnly, None);
    observed_at_zero.observed_at_unix_ms = 0;
    assert!(observed_at_zero.validate().is_err());

    let encoded = serde_json::to_value(storage(
        Profile::RrflowKv,
        Level::RrflowKvCounters,
        Some(rrflow_kv_storage_counters()),
    ))
    .unwrap();
    assert_eq!(encoded["profile"], "rrflow_kv");
    assert_eq!(encoded["evidence_level"], "rrflow_kv_counters");
    assert_eq!(encoded["rrflow_kv"]["segment_io"]["requested_mode"], "auto");
    assert!(encoded["rrflow_kv"]["segment_io"]
        .get("last_fallback_reason")
        .is_none());
    let mut unknown = encoded;
    unknown["rrflow_kv"]["page_cache"]["physical_path"] = "/tmp/escape".into();
    assert!(serde_json::from_value::<rrd_contract::DiagnosticStorageSnapshot>(unknown).is_err());
    assert_eq!(
        serde_json::to_value(rrd_contract::DiagnosticAuthority::Observation).unwrap(),
        "observation"
    );
}

#[test]
fn endpoint_catalogue_is_complete_sorted_and_transport_neutral() {
    let catalogue = rrd_contract::endpoint_catalogue();
    catalogue.validate().unwrap();
    assert_eq!(catalogue.endpoints.len(), 55);
    assert_eq!(catalogue.endpoints[0].operation.as_str(), "audit-export");
    let create = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "session-create")
        .unwrap();
    assert_eq!(create.path, "/v1/sessions");
    assert!(create.mutation);
    assert_eq!(
        create.authentication,
        rrd_contract::EndpointAuthentication::ApiKey
    );
    assert!(catalogue.endpoints.iter().all(|endpoint| {
        !endpoint.request_type.contains("::") && !endpoint.response_type.contains("::")
    }));
    let context = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "context-assemble")
        .unwrap();
    assert_eq!(
        context.action.fixed_action(),
        Some(rrd_contract::SecurityAction::MemoryContextRead)
    );
    assert_eq!(context.path, "/v1/context/assemble");
    for (operation, path, action, mutation, request, response) in [
        (
            "knowledge-tree-list",
            "/v1/knowledge/trees/list",
            rrd_contract::SecurityAction::KnowledgeTreeRead,
            false,
            "ListKnowledgeTrees",
            "KnowledgeTreeCatalogue",
        ),
        (
            "knowledge-tree-replace",
            "/v1/knowledge/trees/replace",
            rrd_contract::SecurityAction::KnowledgeTreeWrite,
            true,
            "ReplaceKnowledgeTrees",
            "ReplaceKnowledgeTreesResult",
        ),
    ] {
        let descriptor = catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == operation)
            .unwrap();
        assert_eq!(descriptor.path, path);
        assert_eq!(descriptor.method, rrd_contract::HttpMethod::Post);
        assert_eq!(
            descriptor.authentication,
            rrd_contract::EndpointAuthentication::SessionBearer
        );
        assert_eq!(descriptor.action.fixed_action(), Some(action));
        assert_eq!(descriptor.mutation, mutation);
        assert_eq!(descriptor.request_type, request);
        assert_eq!(descriptor.response_type, response);
    }
    let encoded = serde_json::to_value(&catalogue).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::EndpointCatalogue>(encoded).unwrap(),
        catalogue
    );
}

#[test]
fn openapi_is_derived_from_every_catalogue_operation_and_wire_type() {
    let catalogue = rrd_contract::endpoint_catalogue();
    let document = rrd_contract::openapi_document().unwrap();
    assert_eq!(document["openapi"], "3.1.0");
    assert_eq!(document["x-rrd-protocol"], PROTOCOL);
    assert_eq!(document["x-rrd-protocol-version"], PROTOCOL_VERSION);
    assert_eq!(document["x-rrd-endpoint-count"], catalogue.endpoints.len());
    assert_eq!(
        document["x-rrd-websocket-endpoint-count"],
        catalogue.websocket_endpoints.len()
    );
    assert_eq!(
        document["x-rrd-websocket-endpoints"],
        serde_json::to_value(&catalogue.websocket_endpoints).unwrap()
    );
    assert!(document["components"]["schemas"]["WebSocketFrame"].is_object());
    for endpoint in catalogue.endpoints {
        let method = match endpoint.method {
            rrd_contract::HttpMethod::Get => "get",
            rrd_contract::HttpMethod::Post => "post",
            rrd_contract::HttpMethod::Delete => "delete",
        };
        let operation = &document["paths"][&endpoint.path][method];
        assert_eq!(operation["operationId"], endpoint.operation.as_str());
        assert!(operation["responses"]["200"]["content"]["application/json"]["schema"].is_object());
        if endpoint.method != rrd_contract::HttpMethod::Get {
            assert!(operation["requestBody"]["content"]["application/json"]["schema"].is_object());
        }
    }
    let encoded = serde_json::to_string(&document).unwrap();
    assert!(!encoded.contains("rrd_store"));
    assert!(!encoded.contains("rrd_server"));
    let mut pretty = serde_json::to_vec_pretty(&document).unwrap();
    pretty.push(b'\n');
    assert_eq!(
        sha256_for_test(&pretty),
        rrd_contract::OPENAPI_DOCUMENT_SHA256,
        "OpenAPI drift requires an intentional protocol/schema review"
    );
}

#[test]
fn native_schema_discovery_is_bounded_strict_and_administratively_authorized() {
    let request = rrd_contract::ReadDataSchema {
        scope: "instance:local".into(),
        max_storage_keys: 4096,
    };
    request.validate().unwrap();
    for max_storage_keys in [0, u32::MAX] {
        assert!(rrd_contract::ReadDataSchema {
            max_storage_keys,
            ..request.clone()
        }
        .validate()
        .is_err());
    }
    assert!(
        serde_json::from_value::<rrd_contract::ReadDataSchema>(serde_json::json!({
            "scope": "instance:local", "max_storage_keys": 4096, "cloud_account": "implicit"
        }))
        .is_err()
    );
    let catalogue = rrd_contract::endpoint_catalogue();
    let descriptor = catalogue
        .endpoints
        .iter()
        .find(|entry| entry.operation.as_str() == "data-schema-read")
        .unwrap();
    assert_eq!(
        descriptor.action.fixed_action(),
        Some(rrd_contract::SecurityAction::DiagnosticsRead)
    );
    assert!(!descriptor.mutation);
}

#[test]
fn durable_subscription_contract_bounds_retention_backpressure_and_stream_shape() {
    let mut request = rrd_contract::OpenSubscription {
        subscription_id: CorrelationId::new("subscription-contract").unwrap(),
        stream: rrd_contract::SubscriptionStream::Changefeed {
            scope: "instance:test-instance".into(),
        },
        after_cursor: 7,
        batch_size: 16,
        max_in_flight: 4,
        retention_cursor_window: 1_024,
        lease_ms: 10_000,
        heartbeat_interval_ms: 500,
    };
    request.validate().unwrap();
    let encoded = serde_json::to_value(&request).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::OpenSubscription>(encoded).unwrap(),
        request
    );
    request.max_in_flight = 0;
    assert!(request.validate().is_err());
    request.max_in_flight = 1;
    request.retention_cursor_window = 15;
    assert!(request.validate().is_err());

    let live = rrd_contract::OpenSubscription {
        subscription_id: CorrelationId::new("subscription-live-contract").unwrap(),
        stream: rrd_contract::SubscriptionStream::LiveQuery {
            scope: "instance:test-instance".into(),
            query: "SELECT * FROM document AT VALID 1000 ORDER BY @id ASC".into(),
            parameters: std::collections::BTreeMap::new(),
            budget: rrd_contract::QueryBudget::default(),
            max_delta_rows: 128,
        },
        after_cursor: 0,
        batch_size: 16,
        max_in_flight: 1,
        retention_cursor_window: 1_024,
        lease_ms: 10_000,
        heartbeat_interval_ms: 500,
    };
    live.validate().unwrap();

    let acknowledgement = rrd_contract::SubscriptionAcknowledgement {
        subscription_id: CorrelationId::new("subscription-contract").unwrap(),
        connection_generation: 3,
        delivery_sequence: 9,
        through_cursor: 42,
    };
    acknowledgement.validate().unwrap();
    let catalogue = rrd_contract::endpoint_catalogue();
    assert_eq!(catalogue.websocket_endpoints.len(), 1);
    assert_eq!(catalogue.websocket_endpoints[0].path, "/v1/ws");
    assert_eq!(
        catalogue.websocket_endpoints[0].operation.as_str(),
        "websocket-connect"
    );
    assert_eq!(
        catalogue.websocket_endpoints[0].frame_type,
        "WebSocketFrame"
    );
    assert_eq!(
        catalogue.websocket_endpoints[0].connect_action,
        rrd_contract::SecurityAction::WebSocketConnect
    );
    assert_eq!(
        catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == "subscription-open")
            .unwrap()
            .action
            .fixed_action(),
        Some(rrd_contract::SecurityAction::SubscriptionOpen)
    );
    assert_eq!(
        catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == "subscription-close")
            .unwrap()
            .action
            .fixed_action(),
        Some(rrd_contract::SecurityAction::SubscriptionClose)
    );
}

fn sha256_for_test(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(&mut output, "{byte:02x}").unwrap();
            output
        })
}

#[test]
fn mutation_idempotency_protocol_and_capability_order_fail_closed() {
    let fixture = contract_fixture();
    let mut request = fixture.request;
    request.context.idempotency_key = None;
    assert!(request.validate(true).is_err());
    assert!(request.validate(false).is_ok());

    request.protocol_version += 1;
    assert!(request.validate(false).is_err());

    let mut service = fixture.service;
    service.capabilities.reverse();
    assert!(service.validate().is_err());
    service
        .capabilities
        .sort_by(|left, right| left.name.cmp(&right.name));
    service.capabilities.push(service.capabilities[0].clone());
    assert!(service.validate().is_err());
}

#[test]
fn resource_paths_are_explicit_bounded_and_non_repeating() {
    let empty = ResourcePath { segments: vec![] };
    assert!(empty.validate().is_err());

    let repeated = ResourcePath {
        segments: vec![
            ResourceId::new(ResourceKind::Project, "alpha").unwrap(),
            ResourceId::new(ResourceKind::Project, "beta").unwrap(),
        ],
    };
    assert!(repeated.validate().is_err());
}

#[test]
fn session_and_claim_transaction_contracts_are_bounded() {
    let limits = SessionLimits {
        idle_timeout_ms: 30_000,
        absolute_timeout_ms: 300_000,
        max_open_transactions: 4,
    };
    limits.validate().unwrap();
    SessionLease {
        session_id: CorrelationId::new("session-1").unwrap(),
        token: CorrelationId::new("token-1").unwrap(),
        issued_at_unix_ms: 100,
        idle_expires_at_unix_ms: 30_100,
        absolute_expires_at_unix_ms: 300_100,
        limits: limits.clone(),
    }
    .validate()
    .unwrap();
    BeginTransaction {
        scope: CanonicalId::new("instance-a").unwrap(),
        timeout_ms: 10_000,
    }
    .validate()
    .unwrap();
    let commit = CommitTransaction {
        operation_sha256: "3c94150b4ea4f9dcb27d3b602e9f190debe656533c047b99e11367bc6a28017f".into(),
        mutations: vec![TransactionMutation::AssertClaim {
            subject: CanonicalId::new("task-1").unwrap(),
            predicate: CanonicalId::new("status").unwrap(),
            object: "verified".into(),
            valid_from: 100,
            tx_time: 101,
            producer: CanonicalId::new("agent-test").unwrap(),
            confidence: Some(0.9),
        }],
    };
    commit.validate().unwrap();
    CommitReceipt {
        transaction_id: CorrelationId::new("transaction-1").unwrap(),
        operation_sha256: "3c94150b4ea4f9dcb27d3b602e9f190debe656533c047b99e11367bc6a28017f".into(),
        first_claim_sequence: 7,
        last_claim_sequence: 7,
        mutation_count: 1,
        runtime_commit_sha256: None,
        first_runtime_cursor: None,
        last_runtime_cursor: None,
        claim_mutation_count: None,
        idempotent_replay: false,
    }
    .validate()
    .unwrap();
    assert_eq!(TransactionState::Open, TransactionState::Open);

    let mut invalid = limits;
    invalid.idle_timeout_ms = invalid.absolute_timeout_ms + 1;
    assert!(invalid.validate().is_err());
    let mut invalid_commit = commit;
    let TransactionMutation::AssertClaim { confidence, .. } = &mut invalid_commit.mutations[0]
    else {
        unreachable!()
    };
    *confidence = Some(f32::NAN);
    assert!(invalid_commit.validate().is_err());
}

#[test]
fn vector_mutation_requires_collection_and_named_vector_identity() {
    let collectionless = serde_json::json!({
        "mutation": "put_vector",
        "reference": {"kind": "embedding", "id": "document-a-body"},
        "subject": {"kind": "document", "id": "document-a"},
        "field": "body-embedding",
        "valid_from": 1,
        "value": {"kind": "dense", "values": [1.0, 0.0]},
        "properties": {}
    });
    assert!(serde_json::from_value::<TransactionMutation>(collectionless).is_err());
}

#[test]
fn lifecycle_health_and_preview_payloads_are_bounded_and_strict() {
    let mutation = TransactionMutation::AssertClaim {
        subject: CanonicalId::new("task-1").unwrap(),
        predicate: CanonicalId::new("status").unwrap(),
        object: "verified".into(),
        valid_from: 100,
        tx_time: 101,
        producer: CanonicalId::new("agent-test").unwrap(),
        confidence: None,
    };
    let preview = PreviewTransaction {
        mutations: vec![mutation.clone()],
        valid_at: Some(100),
        max_storage_keys: 100,
    };
    preview.validate().unwrap();
    TransactionPreview {
        transaction_id: CorrelationId::new("transaction-1").unwrap(),
        read_cursor: 7,
        operation_sha256: "3c94150b4ea4f9dcb27d3b602e9f190debe656533c047b99e11367bc6a28017f".into(),
        mutations: preview.mutations,
        prospective: DataSnapshot {
            scope: "instance:test".into(),
            valid_at: 100,
            known_at_cursor: 8,
            schema_revision: 1,
            read_manifest_sha256: "0".repeat(64),
            selected_versions: 0,
            read_evidence: read_evidence(),
            entries: Vec::new(),
        },
        idempotent_replay: false,
    }
    .validate()
    .unwrap();
    SessionTermination {
        session_id: CorrelationId::new("session-1").unwrap(),
        state: SessionEndState::Closed,
        ended_at_unix_ms: 1,
        affected_open_transactions: 2,
        idempotent_replay: false,
    }
    .validate()
    .unwrap();
    Liveness {
        observed_at_unix_ms: 1,
    }
    .validate()
    .unwrap();
    Readiness {
        observed_at_unix_ms: 1,
        claim_sequence: 2,
        runtime_cursor: 3,
        backend: CanonicalId::new("rrd-lsm").unwrap(),
    }
    .validate()
    .unwrap();

    assert!(serde_json::from_str::<RenewSession>(r#"{"unknown":true}"#).is_err());
    assert!(serde_json::from_str::<CloseSession>(r#"{"unknown":true}"#).is_err());
    assert!(
        serde_json::from_str::<PreviewTransaction>(r#"{"mutations":[],"unknown":true}"#).is_err()
    );
}

#[test]
fn transaction_digest_has_a_cross_language_golden_vector() {
    let first: Vec<TransactionMutation> = serde_json::from_str(
        r#"[{"mutation":"assert_claim","subject":"task-1","predicate":"status","object":"verified","valid_from":100,"tx_time":101,"producer":"agent-test"}]"#,
    )
    .unwrap();
    let reordered: Vec<TransactionMutation> = serde_json::from_str(
        r#"[{"producer":"agent-test","tx_time":101,"valid_from":100,"object":"verified","predicate":"status","subject":"task-1","mutation":"assert_claim"}]"#,
    )
    .unwrap();
    let expected = "d85547ca333304b54db4bc0249b6e11f83cf975fdc7cfebcf551ec6564bb71f8";
    assert_eq!(transaction_operation_sha256(&first), expected);
    assert_eq!(transaction_operation_sha256(&reordered), expected);
}

#[test]
fn unified_catalogue_is_strict_round_trippable_and_rejects_mode_confusion() {
    let document = CanonicalId::new("document").unwrap();
    let mut registry = DataSchemaRegistry {
        revision: 1,
        migration: "freeze public catalogue".into(),
        catalogue: DataCatalogueIdentity {
            namespace: CanonicalId::new("project").unwrap(),
            database: CanonicalId::new("runtime").unwrap(),
        },
        tables: BTreeMap::from([
            (
                document.clone(),
                DataTableSchema {
                    model: DataLogicalModel::Document,
                    mode: DataSchemaMode::Strict,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            ),
            (
                CanonicalId::new("kv").unwrap(),
                DataTableSchema {
                    model: DataLogicalModel::KeyValue,
                    mode: DataSchemaMode::Schemaless,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            ),
        ]),
        records: BTreeMap::from([(document.clone(), DataRecordSchema::default())]),
        relations: BTreeMap::new(),
        events: BTreeMap::new(),
    };
    let mutation = TransactionMutation::PutSchema {
        registry: registry.clone(),
    };
    mutation.validate().unwrap();
    let encoded = serde_json::to_value(&mutation).unwrap();
    let decoded: TransactionMutation = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, mutation);

    registry.tables.get_mut(&document).unwrap().mode = DataSchemaMode::Schemaless;
    assert!(TransactionMutation::PutSchema { registry }
        .validate()
        .is_err());
}

#[test]
fn public_schema_requires_one_explicit_nonempty_table_map() {
    let document = CanonicalId::new("document").unwrap();
    let registry = DataSchemaRegistry {
        revision: 1,
        migration: "reject inferred public table models".into(),
        catalogue: DataCatalogueIdentity::default(),
        tables: BTreeMap::new(),
        records: BTreeMap::from([(document, DataRecordSchema::default())]),
        relations: BTreeMap::new(),
        events: BTreeMap::new(),
    };
    assert!(TransactionMutation::PutSchema {
        registry: registry.clone()
    }
    .validate()
    .is_err());

    let mut encoded = serde_json::to_value(TransactionMutation::PutSchema { registry }).unwrap();
    encoded["registry"]
        .as_object_mut()
        .unwrap()
        .remove("tables");
    assert!(serde_json::from_value::<TransactionMutation>(encoded).is_err());
}

#[test]
fn data_retirement_targets_are_model_typed_and_event_cursor_addressed() {
    let document = DataTarget::Reference {
        reference: DataReference {
            kind: CanonicalId::new("document").unwrap(),
            id: CanonicalId::new("alpha").unwrap(),
        },
    };
    TransactionMutation::RetireData {
        model: DataLogicalModel::Document,
        target: document.clone(),
        effective_at: 10,
    }
    .validate()
    .unwrap();
    let event = DataTarget::Event {
        kind: CanonicalId::new("observed").unwrap(),
        cursor: 7,
    };
    let retirement = TransactionMutation::RetireData {
        model: DataLogicalModel::Event,
        target: event,
        effective_at: 10,
    };
    retirement.validate().unwrap();
    assert_eq!(
        serde_json::from_value::<TransactionMutation>(serde_json::to_value(&retirement).unwrap())
            .unwrap(),
        retirement
    );
    assert!(TransactionMutation::RetireData {
        model: DataLogicalModel::Event,
        target: document,
        effective_at: 10,
    }
    .validate()
    .is_err());
    assert!(TransactionMutation::RetireData {
        model: DataLogicalModel::Vector,
        target: DataTarget::Event {
            kind: CanonicalId::new("observed").unwrap(),
            cursor: 7,
        },
        effective_at: 10,
    }
    .validate()
    .is_err());
    assert!(ReadDataSnapshot {
        valid_at: 10,
        max_storage_keys: 1,
    }
    .validate()
    .is_ok());
    assert!(ReadDataSnapshot {
        valid_at: 0,
        max_storage_keys: 0,
    }
    .validate()
    .is_err());
}

#[test]
fn recursive_retrieval_contract_is_strict_bounded_and_multimodal() {
    let nearest = |using: &str, query: VectorSearchQuery| RetrievalQuery::Nearest {
        using: CanonicalId::new(using).unwrap(),
        query,
        filter: None,
        mode: VectorSearchMode::Exact,
    };
    let request = ExecuteRetrievalQuery {
        scope: "instance:alpha".into(),
        valid_at: 10,
        collection_id: CanonicalId::new("documents").unwrap(),
        query: RetrievalQuery::Fusion {
            prefetch: vec![
                RetrievalPrefetch {
                    query: Box::new(nearest(
                        "dense",
                        VectorSearchQuery::Dense {
                            values: vec![1.0, 0.0],
                        },
                    )),
                    limit: 4,
                },
                RetrievalPrefetch {
                    query: Box::new(nearest(
                        "sparse",
                        VectorSearchQuery::Sparse {
                            dimensions: 8,
                            indices: vec![1, 6],
                            values: vec![0.5, 0.75],
                        },
                    )),
                    limit: 4,
                },
            ],
            fusion: RetrievalFusion::ReciprocalRank {
                rank_constant: 60,
                weights_millionths: vec![1_000_000, 750_000],
            },
        },
        result_shape: RetrievalResultShape::Groups {
            property: CanonicalId::new("category").unwrap(),
            max_groups: 2,
            hits_per_group: 2,
        },
        limit: 2,
        candidate_limit: 4,
        max_storage_keys: 10_000,
    };
    request.validate().unwrap();
    let encoded = serde_json::to_value(&request).unwrap();
    assert_eq!(
        serde_json::from_value::<ExecuteRetrievalQuery>(encoded.clone()).unwrap(),
        request
    );
    let mut unknown = encoded;
    unknown
        .as_object_mut()
        .unwrap()
        .insert("middleware".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<ExecuteRetrievalQuery>(unknown).is_err());

    let mut wrong_weights = request.clone();
    let RetrievalQuery::Fusion { fusion, .. } = &mut wrong_weights.query else {
        unreachable!()
    };
    let RetrievalFusion::ReciprocalRank {
        weights_millionths, ..
    } = fusion;
    weights_millionths.pop();
    assert!(wrong_weights.validate().is_err());

    let mut oversized_prefetch = request;
    let RetrievalQuery::Fusion { prefetch, .. } = &mut oversized_prefetch.query else {
        unreachable!()
    };
    prefetch[0].limit = 5;
    assert!(oversized_prefetch.validate().is_err());
}

#[test]
fn public_embedding_contract_is_bounded_strict_and_collection_addressed() {
    let input = EmbeddingInput {
        id: CanonicalId::new("query-text").unwrap(),
        media_type: "text/plain".into(),
        bytes: b"source-grounded inference".to_vec(),
    };
    let batch = GenerateEmbeddings {
        scope: "instance:alpha".into(),
        backend_id: "rrflow:feature-hash:cpu:v1".into(),
        network_policy: EmbeddingNetworkPolicy::Deny,
        inputs: vec![input.clone()],
    };
    batch.validate().unwrap();
    let encoded = serde_json::to_value(&batch).unwrap();
    assert_eq!(
        serde_json::from_value::<GenerateEmbeddings>(encoded.clone()).unwrap(),
        batch
    );
    let mut unknown = encoded;
    unknown["provider_api_key"] = serde_json::json!("must-not-cross-contract");
    assert!(serde_json::from_value::<GenerateEmbeddings>(unknown).is_err());

    let request = EmbedAndSearchVectors {
        scope: "instance:alpha".into(),
        backend_id: batch.backend_id,
        network_policy: EmbeddingNetworkPolicy::Deny,
        input,
        valid_at: 10,
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        filter: None,
        top_k: 4,
        mode: VectorSearchMode::Exact,
        max_storage_keys: 10_000,
    };
    request.validate().unwrap();
    let mut invalid = request;
    invalid.input.bytes.clear();
    assert!(invalid.validate().is_err());
}

fn flight_context_request() -> rrd_contract::PromptFlightContextRequest {
    rrd_contract::PromptFlightContextRequest {
        valid_at: 1_000,
        seeds: Vec::new(),
        max_graph_depth: 2,
        max_items: 16,
        max_output_bytes: 65_536,
        max_storage_keys: 4_096,
    }
}

fn flight_read_stamp(runtime_cursor: u64) -> rrd_contract::ContextReadStamp {
    rrd_contract::ContextReadStamp {
        runtime_manifest_sha256: "a".repeat(64),
        runtime_cursor,
        schema_revision: Some(3),
        catalogue_revision: 1,
    }
}

fn flight_event(
    ordinal: u64,
    elapsed_ms: u64,
    stage: rrd_contract::PromptFlightStage,
) -> rrd_contract::PromptFlightEvent {
    rrd_contract::PromptFlightEvent {
        ordinal,
        at_unix_ms: 2_000 + elapsed_ms,
        elapsed_ms,
        stage,
        kind: CanonicalId::new("prompt_received").unwrap(),
        label: "Prompt entered the runtime boundary".into(),
        detail: String::new(),
        data: BTreeMap::from([("bytes".into(), rrd_contract::QueryValue::Unsigned(16))]),
    }
}

fn flight_record() -> rrd_contract::PromptFlightRecord {
    let prompt = "  inspect runtime ";
    rrd_contract::PromptFlightRecord {
        format_version: rrd_contract::PROMPT_FLIGHT_FORMAT_VERSION,
        flight_id: CanonicalId::new(format!("flight-{}", "b".repeat(64))).unwrap(),
        scope: "instance:alpha".into(),
        cohort_sha256: rrd_contract::prompt_cohort_sha256(prompt),
        prompt: prompt.into(),
        provider: rrd_contract::PromptFlightProvider::Observe,
        context_mode: rrd_contract::PromptFlightContextMode::Fresh,
        context_request: flight_context_request(),
        acceptance_marker: Some("all checks passed".into()),
        task_id: None,
        status: rrd_contract::PromptFlightStatus::Prepared,
        created_at_unix_ms: 2_000,
        idempotency_key: CorrelationId::new("idem-flight-1").unwrap(),
        operation_sha256: "c".repeat(64),
        read: flight_read_stamp(40),
        context: None,
        metrics: rrd_contract::PromptFlightMetrics {
            context_items: 0,
            context_output_bytes: 0,
            elapsed_ms: 7,
        },
        flight_cursor: 41,
        known_at_cursor: 41,
        use_count: 0,
        event_count: 3,
    }
}

fn flight_use(rank: u64, id: &str) -> rrd_contract::PromptFlightUse {
    rrd_contract::PromptFlightUse {
        reference: rrd_contract::DataReference {
            kind: CanonicalId::new("note").unwrap(),
            id: CanonicalId::new(id).unwrap(),
        },
        rank,
        score: 0.03,
        // Deliberately unsorted (text < graph); callers sort it.
        evidence: vec![
            rrd_contract::ContextEvidenceKind::Graph,
            rrd_contract::ContextEvidenceKind::Text,
        ],
    }
}

#[test]
fn prompt_flight_contract_is_bounded_strict_and_separately_authorized() {
    let launch = rrd_contract::LaunchPromptFlight {
        scope: "instance:alpha".into(),
        prompt: "inspect runtime".into(),
        provider: rrd_contract::PromptFlightProvider::Observe,
        context_mode: rrd_contract::PromptFlightContextMode::Pruned,
        context: flight_context_request(),
        acceptance_marker: None,
        task_id: None,
    };
    launch.validate().unwrap();
    assert_eq!(
        launch.cohort_sha256(),
        rrd_contract::prompt_cohort_sha256("  inspect runtime\n")
    );
    let mut empty_prompt = launch.clone();
    empty_prompt.prompt = "   ".into();
    assert!(empty_prompt.validate().is_err());
    let mut oversized_prompt = launch.clone();
    oversized_prompt.prompt = "x".repeat(rrd_contract::MAX_PROMPT_FLIGHT_PROMPT_BYTES + 1);
    assert!(oversized_prompt.validate().is_err());
    let mut bad_budget = launch.clone();
    bad_budget.context.max_items = rrd_contract::MAX_CONTEXT_ITEMS + 1;
    assert!(bad_budget.validate().is_err());
    let mut empty_marker = launch.clone();
    empty_marker.acceptance_marker = Some(" ".into());
    assert!(empty_marker.validate().is_err());
    assert!(serde_json::from_value::<rrd_contract::LaunchPromptFlight>(serde_json::json!({
        "scope": "instance:alpha",
        "prompt": "inspect runtime",
        "provider": "codex",
        "context_mode": "pruned",
        "context": {"valid_at": 1000, "max_graph_depth": 2, "max_items": 16, "max_output_bytes": 65536, "max_storage_keys": 4096}
    }))
    .is_err());
    assert!(serde_json::from_value::<rrd_contract::LaunchPromptFlight>(serde_json::json!({
        "scope": "instance:alpha",
        "prompt": "inspect runtime",
        "provider": "observe",
        "context_mode": "pruned",
        "context": {"valid_at": 1000, "max_graph_depth": 2, "max_items": 16, "max_output_bytes": 65536, "max_storage_keys": 4096},
        "reasoning_profile": "ultra"
    }))
    .is_err());
    assert_eq!(
        serde_json::to_value(rrd_contract::PromptFlightStage::Outcome).unwrap(),
        "outcome"
    );

    let record = flight_record();
    record.validate().unwrap();
    let mut wrong_cohort = record.clone();
    wrong_cohort.cohort_sha256 = "d".repeat(64);
    assert!(wrong_cohort.validate().is_err());
    let mut fresh_with_context = record.clone();
    fresh_with_context.metrics.context_items = 1;
    assert!(fresh_with_context.validate().is_err());
    let mut committed_before_read = record.clone();
    committed_before_read.flight_cursor = 40;
    committed_before_read.known_at_cursor = 40;
    assert!(committed_before_read.validate().is_err());
    let mut pruned_without_summary = record.clone();
    pruned_without_summary.context_mode = rrd_contract::PromptFlightContextMode::Pruned;
    assert!(pruned_without_summary.validate().is_err());
    pruned_without_summary.context = Some(rrd_contract::PromptFlightContextSummary {
        request_sha256: "e".repeat(64),
        plan_sha256: "e".repeat(64),
        packet_sha256: "e".repeat(64),
        items: 2,
        output_bytes: 300,
        truncated: false,
        selected_versions: 9,
    });
    assert!(pruned_without_summary.validate().is_err());
    pruned_without_summary.metrics.context_items = 2;
    pruned_without_summary.metrics.context_output_bytes = 300;
    pruned_without_summary.validate().unwrap();

    let mut fresh_with_uses = record.clone();
    fresh_with_uses.use_count = 1;
    assert!(fresh_with_uses.validate().is_err());

    let flight = rrd_contract::PromptFlight {
        record: record.clone(),
        uses: Vec::new(),
        events: vec![
            flight_event(0, 0, rrd_contract::PromptFlightStage::Prompt),
            flight_event(1, 3, rrd_contract::PromptFlightStage::Context),
            flight_event(2, 7, rrd_contract::PromptFlightStage::Outcome),
        ],
    };
    flight.validate().unwrap();
    let mut gapped = flight.clone();
    gapped.events[1].ordinal = 5;
    assert!(gapped.validate().is_err());
    let mut backwards = flight.clone();
    backwards.events[2].elapsed_ms = 1;
    assert!(backwards.validate().is_err());
    let mut too_long = flight.clone();
    too_long.record.metrics.elapsed_ms = 6;
    assert!(too_long.validate().is_err());
    let mut miscounted = flight.clone();
    miscounted.events.pop();
    assert!(miscounted.validate().is_err());
    let encoded = serde_json::to_value(&flight).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::PromptFlight>(encoded).unwrap(),
        flight
    );

    // A pruned flight's edges: rank order, one per record, sorted evidence.
    let mut graph = flight.clone();
    graph.record.context_mode = rrd_contract::PromptFlightContextMode::Pruned;
    graph.record.context = Some(rrd_contract::PromptFlightContextSummary {
        request_sha256: "e".repeat(64),
        plan_sha256: "e".repeat(64),
        packet_sha256: "e".repeat(64),
        items: 3,
        output_bytes: 300,
        truncated: false,
        selected_versions: 9,
    });
    graph.record.metrics.context_items = 3;
    graph.record.metrics.context_output_bytes = 300;
    graph.record.use_count = 2;
    let mut sorted_evidence = flight_use(1, "alpha");
    sorted_evidence.evidence.sort();
    let mut second = flight_use(3, "beta");
    second.evidence.sort();
    graph.uses = vec![sorted_evidence.clone(), second.clone()];
    graph.validate().unwrap();
    let mut too_many = graph.clone();
    too_many.record.use_count = 4;
    assert!(too_many.record.validate().is_err(), "more edges than items");
    let mut miscounted_uses = graph.clone();
    miscounted_uses.uses.pop();
    assert!(miscounted_uses.validate().is_err());
    let mut unordered_uses = graph.clone();
    unordered_uses.uses.swap(0, 1);
    assert!(unordered_uses.validate().is_err());
    let mut duplicate_record = graph.clone();
    duplicate_record.uses[1].reference = sorted_evidence.reference.clone();
    assert!(duplicate_record.validate().is_err());
    let mut unsorted_evidence = graph.clone();
    unsorted_evidence.uses[0] = flight_use(1, "alpha");
    assert!(
        unsorted_evidence.validate().is_err(),
        "evidence must be sorted"
    );
    let mut zero_score = graph.clone();
    zero_score.uses[0].score = 0.0;
    assert!(zero_score.validate().is_err());
    let encoded = serde_json::to_value(&graph).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::PromptFlight>(encoded).unwrap(),
        graph
    );

    rrd_contract::ListPromptFlights {
        scope: "instance:alpha".into(),
        after_cursor: 0,
        limit: rrd_contract::MAX_PROMPT_FLIGHT_PAGE,
        max_storage_keys: 1,
    }
    .validate()
    .unwrap();
    assert!(rrd_contract::ListPromptFlights {
        scope: "instance:alpha".into(),
        after_cursor: 0,
        limit: 0,
        max_storage_keys: 1,
    }
    .validate()
    .is_err());
    assert!(rrd_contract::ReadPromptFlight {
        scope: "instance:alpha".into(),
        flight_id: record.flight_id.clone(),
        max_storage_keys: rrd_contract::MAX_PROMPT_FLIGHT_STORAGE_KEYS + 1,
    }
    .validate()
    .is_err());
    let mut later = record.clone();
    later.flight_id = CanonicalId::new("flight-later").unwrap();
    later.flight_cursor = 45;
    later.known_at_cursor = 45;
    let page = rrd_contract::PromptFlightPage {
        scope: "instance:alpha".into(),
        read: flight_read_stamp(50),
        requested_after_cursor: 40,
        flights: vec![record.clone(), later.clone()],
        truncated: false,
        selected_versions: 2,
        read_evidence: read_evidence(),
    };
    page.validate().unwrap();
    let mut unordered = page.clone();
    unordered.flights.swap(0, 1);
    assert!(unordered.validate().is_err());
    let mut beyond_stamp = page.clone();
    beyond_stamp.read = flight_read_stamp(44);
    assert!(beyond_stamp.validate().is_err());

    let catalogue = rrd_contract::endpoint_catalogue();
    let launch_descriptor = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "flight-launch")
        .unwrap();
    assert_eq!(launch_descriptor.path, "/v1/flights");
    assert!(launch_descriptor.mutation);
    assert_eq!(
        launch_descriptor.action.fixed_action(),
        Some(rrd_contract::SecurityAction::FlightLaunch)
    );
    for (operation, path) in [
        ("flight-list", "/v1/flights/list"),
        ("flight-read", "/v1/flights/read"),
    ] {
        let descriptor = catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == operation)
            .unwrap();
        assert_eq!(descriptor.path, path);
        assert!(!descriptor.mutation);
        assert_eq!(
            descriptor.action.fixed_action(),
            Some(rrd_contract::SecurityAction::FlightRead)
        );
    }
    assert_eq!(rrd_contract::SecurityAction::ALL.len(), 53);
}

fn task_record() -> rrd_contract::TaskRecord {
    rrd_contract::TaskRecord {
        format_version: rrd_contract::TASK_FORMAT_VERSION,
        task_id: CanonicalId::new(format!("task-{}", "d".repeat(64))).unwrap(),
        scope: "instance:alpha".into(),
        title: "Session recovery".into(),
        goal: "Recover a durable session.".into(),
        status: rrd_contract::TaskStatus::Open,
        outcome: None,
        opened_at_unix_ms: 2_000,
        closed_at_unix_ms: None,
        idempotency_key: CorrelationId::new("idem-task-1").unwrap(),
        operation_sha256: "e".repeat(64),
        task_cursor: 41,
        known_at_cursor: 41,
    }
}

#[test]
fn task_contract_is_bounded_strict_and_separately_authorized() {
    let open = rrd_contract::OpenTask {
        scope: "instance:alpha".into(),
        title: "Session recovery".into(),
        goal: "Recover a durable session.".into(),
    };
    open.validate().unwrap();
    for title in [
        "   ".to_owned(),
        "two\nlines".to_owned(),
        "x".repeat(rrd_contract::MAX_TASK_TITLE_BYTES + 1),
    ] {
        let mut invalid = open.clone();
        invalid.title = title;
        assert!(invalid.validate().is_err());
    }
    let mut no_goal = open.clone();
    no_goal.goal = " ".into();
    assert!(no_goal.validate().is_err());
    let mut long_goal = open.clone();
    long_goal.goal = "g".repeat(rrd_contract::MAX_TASK_GOAL_BYTES + 1);
    assert!(long_goal.validate().is_err());
    assert!(
        serde_json::from_value::<rrd_contract::OpenTask>(serde_json::json!({
            "scope": "instance:alpha", "title": "t", "goal": "g", "owner": "someone"
        }))
        .is_err()
    );

    let close = rrd_contract::CloseTask {
        scope: "instance:alpha".into(),
        task_id: task_record().task_id,
        status: rrd_contract::TaskStatus::Completed,
        outcome: Some("Recovered.".into()),
    };
    close.validate().unwrap();
    let mut reopen = close.clone();
    reopen.status = rrd_contract::TaskStatus::Open;
    assert!(reopen.validate().is_err());
    let mut blank_outcome = close.clone();
    blank_outcome.outcome = Some("  ".into());
    assert!(blank_outcome.validate().is_err());
    assert_eq!(
        serde_json::to_value(rrd_contract::TaskStatus::Abandoned).unwrap(),
        serde_json::json!("abandoned")
    );

    let open_task = task_record();
    open_task.validate().unwrap();
    let mut open_with_outcome = open_task.clone();
    open_with_outcome.outcome = Some("early".into());
    assert!(open_with_outcome.validate().is_err());
    let mut open_twice_known = open_task.clone();
    open_twice_known.known_at_cursor = 42;
    assert!(open_twice_known.validate().is_err());
    let mut closed = open_task.clone();
    closed.status = rrd_contract::TaskStatus::Completed;
    closed.outcome = Some("Recovered.".into());
    closed.closed_at_unix_ms = Some(2_500);
    closed.known_at_cursor = 47;
    closed.validate().unwrap();
    let mut closed_without_time = closed.clone();
    closed_without_time.closed_at_unix_ms = None;
    assert!(closed_without_time.validate().is_err());
    let mut closed_before_open = closed.clone();
    closed_before_open.closed_at_unix_ms = Some(1_999);
    assert!(closed_before_open.validate().is_err());
    let mut closed_in_one_version = closed.clone();
    closed_in_one_version.known_at_cursor = closed.task_cursor;
    assert!(closed_in_one_version.validate().is_err());
    let encoded = serde_json::to_value(&closed).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::TaskRecord>(encoded).unwrap(),
        closed
    );

    let page = rrd_contract::TaskPage {
        scope: "instance:alpha".into(),
        read: flight_read_stamp(50),
        requested_after_cursor: 0,
        tasks: vec![open_task.clone(), {
            let mut later = open_task.clone();
            later.task_cursor = 45;
            later.known_at_cursor = 45;
            later
        }],
        truncated: false,
        selected_versions: 2,
        read_evidence: read_evidence(),
    };
    page.validate().unwrap();
    let mut unordered = page.clone();
    unordered.tasks.swap(0, 1);
    assert!(unordered.validate().is_err());
    let mut mixed = page.clone();
    mixed.tasks[1].scope = "instance:beta".into();
    assert!(mixed.validate().is_err());
    let read = rrd_contract::TaskReadResult {
        task: closed.clone(),
        read: flight_read_stamp(46),
        selected_versions: 2,
        read_evidence: read_evidence(),
    };
    assert!(read.validate().is_err(), "read beyond its stamp");

    // A flight names the task it joins; without one the field is omitted.
    let mut joined = rrd_contract::LaunchPromptFlight {
        scope: "instance:alpha".into(),
        prompt: "inspect runtime".into(),
        provider: rrd_contract::PromptFlightProvider::Observe,
        context_mode: rrd_contract::PromptFlightContextMode::Fresh,
        context: flight_context_request(),
        acceptance_marker: None,
        task_id: None,
    };
    assert!(serde_json::to_value(&joined)
        .unwrap()
        .get("task_id")
        .is_none());
    joined.task_id = Some(open_task.task_id.clone());
    let encoded = serde_json::to_value(&joined).unwrap();
    assert_eq!(
        encoded["task_id"],
        serde_json::json!(open_task.task_id.as_str())
    );
    assert_eq!(
        serde_json::from_value::<rrd_contract::LaunchPromptFlight>(encoded).unwrap(),
        joined
    );

    let catalogue = rrd_contract::endpoint_catalogue();
    for (operation, path, mutation, action) in [
        (
            "task-open",
            "/v1/tasks",
            true,
            rrd_contract::SecurityAction::TaskWrite,
        ),
        (
            "task-close",
            "/v1/tasks/close",
            true,
            rrd_contract::SecurityAction::TaskWrite,
        ),
        (
            "task-list",
            "/v1/tasks/list",
            false,
            rrd_contract::SecurityAction::TaskRead,
        ),
        (
            "task-read",
            "/v1/tasks/read",
            false,
            rrd_contract::SecurityAction::TaskRead,
        ),
    ] {
        let descriptor = catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == operation)
            .unwrap();
        assert_eq!(descriptor.path, path);
        assert_eq!(descriptor.mutation, mutation);
        assert_eq!(descriptor.action.fixed_action(), Some(action));
    }
}

#[test]
fn storage_observation_has_bounded_scope_and_a_read_only_authenticated_route() {
    use rrd_contract::ReadStorageDiagnostics;
    for scope in [
        String::new(),
        "x".repeat(rrd_contract::MAX_ID_BYTES + 1),
        "instance:a\0".into(),
    ] {
        assert!(ReadStorageDiagnostics { scope }.validate().is_err());
    }
    assert!(
        serde_json::from_value::<ReadStorageDiagnostics>(serde_json::json!({
            "scope":"instance:alpha", "physical_store":"/outside"
        }))
        .is_err()
    );
    let endpoint = rrd_contract::endpoint_catalogue()
        .endpoints
        .into_iter()
        .find(|e| e.operation.as_str() == "diagnostics-storage-read")
        .unwrap();
    assert_eq!(endpoint.path, "/v1/diagnostics/storage/read");
    assert_eq!(
        endpoint.authentication,
        rrd_contract::EndpointAuthentication::SessionBearer
    );
    assert_eq!(
        endpoint.action.fixed_action(),
        Some(rrd_contract::SecurityAction::DiagnosticsRead)
    );
    assert!(!endpoint.mutation);
}

#[test]
fn source_page_capacity_is_explicit_finite_and_part_of_sealed_configuration() {
    let mut limits = rrd_contract::StorageLimits::default();
    assert_eq!(limits.source_page_bytes, 256 * 1024 * 1024);
    limits.source_page_bytes = 0;
    assert!(limits.validate().is_err());
    limits.source_page_bytes = (1_u64 << 40) + 1;
    assert!(limits.validate().is_err());
    let mut wire = serde_json::to_value(rrd_contract::StorageLimits::default()).unwrap();
    wire.as_object_mut().unwrap().remove("source_page_bytes");
    assert!(serde_json::from_value::<rrd_contract::StorageLimits>(wire).is_err());
}

#[test]
fn storage_cursor_rejects_unknown_actions_fields_and_unbounded_budgets() {
    use rrd_contract::{CorrelationId, InspectStorageCursor, StorageCursorAction};
    for action in [
        serde_json::json!({"kind":"observe","extra":true}),
        serde_json::json!({"kind":"release","extra":true}),
        serde_json::json!({"kind":"unknown"}),
    ] {
        assert!(serde_json::from_value::<StorageCursorAction>(action).is_err());
    }
    for (rows, bytes, lifetime) in [
        (0, 4096, 1000),
        (4097, 4096, 1000),
        (1, 0, 1000),
        (1, 4194305, 1000),
        (1, 4096, 0),
        (1, 4096, 60001),
    ] {
        assert!(InspectStorageCursor {
            scope: "instance:test".into(),
            cursor_id: CorrelationId::new("reader").unwrap(),
            action: StorageCursorAction::Open {
                batch_rows: rows,
                batch_bytes: bytes,
                lifetime_ms: lifetime
            }
        }
        .validate()
        .is_err());
    }
}
