//! Contract fixtures, frozen golden JSON and the shared conformance corpus.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FixturePayload {
    action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContractFixture {
    pub(super) service: ServiceCapabilities,
    pub(super) request: RequestEnvelope<FixturePayload>,
    pub(super) success: ResponseEnvelope<FixturePayload>,
    pub(super) failure: ResponseEnvelope<FixturePayload>,
    pub(super) idempotency: IdempotencyBinding,
}

#[test]
pub(super) fn forward_rollback_request_is_strict_and_counts_cannot_overflow() {
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

pub(super) fn contract_fixture() -> ContractFixture {
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
pub(super) fn public_contract_matches_frozen_golden_json() {
    let fixture = contract_fixture();
    fixture.service.validate().unwrap();
    fixture.request.validate(true).unwrap();
    fixture.success.validate().unwrap();
    fixture.failure.validate().unwrap();
    fixture.idempotency.validate().unwrap();

    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/public-contract-v1.json")).unwrap();
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
pub(super) fn deployment_axes_and_shared_conformance_corpus_are_versioned_and_strict() {
    let corpus: rrd_contract::DeploymentConformanceCorpus = serde_json::from_str(include_str!(
        "../../../../../fixtures/rrd-deployment-conformance-v1.json"
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
pub(super) fn supported_sdks_share_one_strict_semantic_corpus() {
    let corpus: rrd_contract::SdkConformanceCorpus = serde_json::from_str(include_str!(
        "../../../../../fixtures/rrd-sdk-conformance-v1.json"
    ))
    .unwrap();
    corpus.validate().unwrap();
    assert_eq!(corpus.required_domains.len(), 13);
    assert_eq!(
        usize::from(corpus.expected.endpoint_count),
        rrd_contract::endpoint_catalogue().endpoints.len()
    );
    assert_eq!(
        rrd_contract::transaction_operation_sha256(&corpus.transaction.commit.mutations),
        corpus.transaction.commit.operation_sha256
    );

    let mut unknown = serde_json::to_value(&corpus).unwrap();
    unknown["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<rrd_contract::SdkConformanceCorpus>(unknown).is_err());
}

#[test]
pub(super) fn malformed_identifiers_and_unknown_fields_fail_during_decode() {
    assert!(serde_json::from_str::<CanonicalId>(r#""Not Canonical""#).is_err());
    assert!(CorrelationId::new("request/id").is_err());

    let mut request = serde_json::to_value(&contract_fixture().request).unwrap();
    request
        .as_object_mut()
        .unwrap()
        .insert("unknown".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<RequestEnvelope<FixturePayload>>(request).is_err());
}
