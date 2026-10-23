use rcgen::{
    BasicConstraints, Certificate, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer,
    KeyPair, KeyUsagePurpose,
};
use rrd_client::{
    is_unauthenticated, ClientConfig, Error, RequestOptions, RrdClient, RrdWebSocket, Session,
};
use rrd_contract::{
    transaction_operation_sha256, AbortTransaction, AssembleContext, BeginTransaction, CanonicalId,
    CloseSubscription, CloseTask, CommitTransaction, ContextEvidenceKind, CreateSession,
    DeploymentConformanceCorpus, DeploymentForm, EndpointPresentation, ErrorCode, ExecuteQuery,
    ExportAudit, InstallationManagedPathKind, InstallationTargetKind, LaunchPromptFlight,
    ListPromptFlights, ListTasks, OpenSubscription, OpenTask, PreviewTransaction,
    PromptFlightContextMode, PromptFlightContextRequest, PromptFlightProvider, PromptFlightStage,
    QueryBudget, ReadAccessPath, ReadAudit, ReadChangefeed, ReadDiagnosticSnapshot,
    ReadPromptFlight, ReadTask, RequestContext, RequestEnvelope, ResourceId, ResourceKind,
    ResourcePath, SessionLimits, StorageProfileKind, SubscriptionAcknowledgement,
    SubscriptionDelivery, SubscriptionResume, SubscriptionStream, TaskStatus, TransactionMutation,
    WebSocketCancel, WebSocketCancellationDisposition, WebSocketErrorTarget, WebSocketFrame,
    WebSocketPayload, WebSocketRequest, PROTOCOL, PROTOCOL_VERSION,
};
use rrd_core::{
    digest, RuntimeCommit, RuntimeLogicalModel, RuntimeProperties, RuntimePropertySchema,
    RuntimeRecord, RuntimeRecordSchema, RuntimeRef, RuntimeType, RuntimeValue, RuntimeValueType,
    ScopeId,
};
use rrd_engine::RrdEngine;
use rrd_security::{
    Action, Principal, PrincipalKind, ResourceGrant, SecurityRepository, SecurityState,
    SECURITY_FORMAT,
};
use rrd_server::RrdHttpServer;
use rrd_server::RrdMutualTlsServerConfig;
use rrd_store::{RrflowKvStore, StorageEngine};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig as RustlsClientConfig, RootCertStore};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const INTEGRATION_IO_TIMEOUT: Duration = Duration::from_secs(10);
const COMPONENT_TOKEN_KEY: [u8; 32] = [0x6b; 32];

async fn receive_application_frame(socket: &mut RrdWebSocket) -> WebSocketFrame {
    loop {
        let frame = socket
            .receive_until(tokio::time::Instant::now() + INTEGRATION_IO_TIMEOUT)
            .await
            .unwrap();
        if !matches!(frame.payload, WebSocketPayload::Heartbeat(_)) {
            return frame;
        }
    }
}

fn test_ca() -> (Certificate, Issuer<'static, KeyPair>) {
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
    ];
    let key = KeyPair::generate().unwrap();
    let certificate = params.self_signed(&key).unwrap();
    (certificate, Issuer::new(params, key))
}

fn test_identity(
    issuer: &Issuer<'static, KeyPair>,
    dns_names: Vec<String>,
    usage: ExtendedKeyUsagePurpose,
) -> (
    Vec<rustls::pki_types::CertificateDer<'static>>,
    PrivateKeyDer<'static>,
) {
    let mut params = CertificateParams::new(dns_names).unwrap();
    params.extended_key_usages = vec![usage];
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    let key = KeyPair::generate().unwrap();
    let certificate = params.signed_by(&key, issuer).unwrap();
    (
        vec![certificate.der().clone()],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
    )
}

fn roots(ca: &Certificate) -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots.add(ca.der().clone()).unwrap();
    roots
}

fn instance_resource(instance: &CanonicalId) -> ResourcePath {
    ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance.as_str()).unwrap()],
    }
}

fn seed(engine: &RrflowKvStore, scope: &str) {
    let scope = ScopeId::new(scope).unwrap();
    let read = engine.runtime().read_stamp(&scope).unwrap();
    let mut registry = engine.runtime().schema(&scope).unwrap().unwrap();
    registry.revision += 1;
    registry.migration = "extend installed schema for Rust SDK fixture".into();
    registry
        .define_record_table(
            RuntimeType::new("document").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema {
                properties: BTreeMap::from([(
                    "title".into(),
                    RuntimePropertySchema::required(RuntimeValueType::String),
                )]),
                ..RuntimeRecordSchema::default()
            },
        )
        .unwrap();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope,
            at: 100,
            actor: "rrd-client-fixture".into(),
            expected_cursor: read.commit_cursor,
            mutations: vec![
                rrd_core::RuntimeMutation::Schema { registry },
                rrd_core::RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", "alpha").unwrap(),
                        valid_from: 100,
                        valid_to: None,
                        properties: RuntimeProperties::from([(
                            "title".into(),
                            RuntimeValue::String("Alpha".into()),
                        )]),
                    },
                },
            ],
        })
        .unwrap();
}

fn deployment_corpus() -> DeploymentConformanceCorpus {
    let corpus = serde_json::from_str(include_str!(
        "../../../../fixtures/rrd-deployment-conformance-v1.json"
    ))
    .unwrap();
    DeploymentConformanceCorpus::validate(&corpus).unwrap();
    corpus
}

fn deployment_mutations() -> Vec<TransactionMutation> {
    let corpus = deployment_corpus();
    let mut mutations = vec![json!({
        "mutation": "put_schema",
        "registry": {
            "revision": 1,
            "migration": "install deployment conformance corpus",
            "tables": {
                "document": {"model": "relational", "mode": "strict"}
            },
            "records": {
                "document": {
                    "properties": {
                        "body": {"value_type": "string", "required": true}
                    },
                    "allow_additional_properties": false,
                    "unique_properties": []
                }
            },
            "relations": {},
            "events": {}
        }
    })];
    mutations.extend(corpus.documents.iter().map(|document| {
        json!({
            "mutation": "put_record",
            "reference": {"kind": "document", "id": document.id},
            "valid_from": corpus.query.valid_at,
            "properties": {
                "body": {"type": "string", "value": document.text}
            }
        })
    }));
    serde_json::from_value(Value::Array(mutations)).unwrap()
}

async fn commit_client_deployment_corpus(client: &RrdClient, session: &Session, prefix: &str) {
    let transaction = client
        .begin_transaction(
            session,
            BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 10_000,
            },
            RequestOptions::mutation(
                &format!("request-{prefix}-begin"),
                &format!("operation-{prefix}-begin"),
                &format!("{prefix}-begin-key"),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let mutations = deployment_mutations();
    let receipt = client
        .commit_transaction(
            session,
            &transaction.transaction_id,
            CommitTransaction {
                operation_sha256: transaction_operation_sha256(&mutations),
                mutations,
            },
            RequestOptions::new(
                &format!("request-{prefix}-commit"),
                &format!("operation-{prefix}-commit"),
                Some(&format!("{prefix}-commit-key")),
                Some(u64::MAX),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(receipt.first_runtime_cursor, Some(1));
    assert_eq!(receipt.last_runtime_cursor, Some(3));
    assert_eq!(receipt.mutation_count, 3);
}

async fn assert_client_deployment_corpus(
    client: &RrdClient,
    session: &Session,
    endpoint_presentation: EndpointPresentation,
) {
    let corpus = deployment_corpus();
    let capabilities = client.capabilities().await.unwrap();
    assert_eq!(
        capabilities.deployment.deployment_form,
        DeploymentForm::SingleNodeServer
    );
    assert_eq!(
        capabilities.deployment.storage_profile,
        StorageProfileKind::RrflowKv
    );
    assert_eq!(
        capabilities.deployment.endpoint_presentation,
        endpoint_presentation
    );
    capabilities.configuration.validate().unwrap();
    let result = client
        .execute_query(
            session,
            ExecuteQuery {
                scope: format!("instance:{}", client.instance_id()),
                query: corpus.query.rrflowql,
                parameters: BTreeMap::new(),
                budget: QueryBudget::default(),
            },
            RequestOptions::read("request-mode-corpus", "operation-mode-corpus").unwrap(),
        )
        .await
        .unwrap();
    let actual = result
        .rows
        .iter()
        .map(|row| {
            CanonicalId::new(
                row.identity
                    .strip_prefix("record:document:")
                    .expect("corpus row has document identity"),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, corpus.expected_ids);
    assert!(result.execution.selected_versions > 0);
    result.execution.read_evidence.validate().unwrap();
    assert!(result
        .execution
        .read_evidence
        .paths
        .iter()
        .any(|path| path.path == ReadAccessPath::RecordVersions));
    assert_eq!(
        result.rows[0].values["body"],
        rrd_contract::QueryValue::String(corpus.documents[0].text.clone())
    );
}

#[tokio::test]
async fn rust_client_negotiates_authenticates_queries_and_reads_audit() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let installation = RrdEngine::plan_installation(
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
        &installation,
        &installation.installation.plan_sha256,
        &executable,
    )
    .unwrap();
    let instance = installation.installation.target.instance_id.clone();
    let scope = format!("instance:{instance}");
    let root = project.join(
        &installation
            .installation
            .managed_path(InstallationManagedPathKind::StorageRoot)
            .relative_path,
    );
    let storage = RrflowKvStore::open(&root).unwrap();
    seed(&storage, &scope);
    let runtime_head = storage.runtime().cursor().unwrap();
    storage
        .runtime()
        .open_snapshot(
            &ScopeId::new(scope.clone()).unwrap(),
            "rust-sdk-fixture",
            u64::try_from(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
            )
            .unwrap(),
            3_600_000,
        )
        .unwrap();
    let resource = instance_resource(&instance);
    let principal = Principal {
        id: CanonicalId::new("rust-sdk").unwrap(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"sdk-api-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: [
            Action::SessionCreate,
            Action::QueryExecute,
            Action::AuditRead,
            Action::AuditExport,
            Action::TransactionBegin,
            Action::TransactionPreview,
            Action::TransactionAbort,
            Action::ChangefeedRead,
            Action::ChangefeedFollow,
            Action::WebSocketConnect,
            Action::SubscriptionOpen,
            Action::SubscriptionConnect,
            Action::SubscriptionAck,
            Action::SubscriptionClose,
            Action::MemoryContextRead,
            Action::ServiceInspect,
            Action::DiagnosticsRead,
            Action::FlightLaunch,
            Action::FlightRead,
            Action::TaskWrite,
            Action::TaskRead,
        ]
        .into_iter()
        .map(|action| ResourceGrant {
            action,
            resource_prefix: resource.clone(),
            data_policy: None,
        })
        .collect(),
    };
    let websocket_limited_principal = Principal {
        id: CanonicalId::new("websocket-limited").unwrap(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"websocket-limited-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: [
            Action::SessionCreate,
            Action::WebSocketConnect,
            Action::SubscriptionOpen,
            Action::SubscriptionClose,
            Action::ChangefeedFollow,
        ]
        .into_iter()
        .map(|action| ResourceGrant {
            action,
            resource_prefix: resource.clone(),
            data_policy: None,
        })
        .collect(),
    };
    let security = SecurityRepository::new(&storage, instance.clone());
    let mut state = security.load().unwrap().unwrap();
    assert_eq!(state.format_version, SECURITY_FORMAT);
    assert_eq!(state.revision, 1);
    state.revision = 2;
    state.principals.insert(principal.id.clone(), principal);
    state.principals.insert(
        websocket_limited_principal.id.clone(),
        websocket_limited_principal,
    );
    security
        .replace(
            1,
            state,
            2,
            "rrd-client-fixture",
            "request-fixture-security",
            "operation-fixture-security",
        )
        .unwrap();
    drop(storage);
    let engine = RrdEngine::open_installed(&project, &executable).unwrap();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let address = server.local_addr();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));

    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_address = proxy_listener.local_addr().unwrap();
    let (proxy_shutdown, mut proxy_shutdown_receiver) = tokio::sync::watch::channel(false);
    let proxy = tokio::spawn(async move {
        let (first, _) = proxy_listener.accept().await.unwrap();
        drop(first);
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                changed = proxy_shutdown_receiver.changed() => {
                    if changed.is_err() || *proxy_shutdown_receiver.borrow() {
                        break;
                    }
                }
                accepted = proxy_listener.accept() => {
                    let (mut downstream, _) = accepted.unwrap();
                    connections.spawn(async move {
                        let mut upstream = tokio::net::TcpStream::connect(address).await.unwrap();
                        tokio::io::copy_bidirectional(&mut downstream, &mut upstream)
                            .await
                            .unwrap();
                    });
                }
            }
        }
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    });

    let client = RrdClient::connect_local(
        proxy_address,
        instance.clone(),
        ClientConfig {
            request_timeout: INTEGRATION_IO_TIMEOUT,
            max_attempts: 2,
            websocket_limits: Default::default(),
        },
    )
    .unwrap();
    let capabilities = client.capabilities().await.unwrap();
    assert_eq!(capabilities.protocol_version, 1);
    assert_eq!(
        capabilities.product_capabilities,
        rrd_engine::product_capability_catalogue()
    );
    let catalogue = client.endpoint_catalogue().await.unwrap();
    assert!(catalogue
        .endpoints
        .iter()
        .any(|endpoint| endpoint.path == "/v1/context/assemble"));
    let openapi = client.openapi_document().await.unwrap();
    assert_eq!(openapi["x-rrd-endpoint-count"], catalogue.endpoints.len());

    let session_request = CreateSession {
        limits: SessionLimits {
            idle_timeout_ms: 60_000,
            absolute_timeout_ms: 300_000,
            max_open_transactions: 2,
        },
    };
    let wrong = client
        .create_session(
            CanonicalId::new("rust-sdk").unwrap(),
            "wrong",
            session_request.clone(),
            RequestOptions::mutation("request-wrong", "operation-wrong", "session-wrong").unwrap(),
        )
        .await
        .unwrap_err();
    assert!(is_unauthenticated(&wrong));

    let session = client
        .create_session(
            CanonicalId::new("rust-sdk").unwrap(),
            "sdk-api-key",
            session_request,
            RequestOptions::mutation("request-session", "operation-session", "session-key")
                .unwrap(),
        )
        .await
        .unwrap();
    let websocket_limited_session = client
        .create_session(
            CanonicalId::new("websocket-limited").unwrap(),
            "websocket-limited-key",
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 300_000,
                    max_open_transactions: 1,
                },
            },
            RequestOptions::mutation(
                "request-websocket-limited-session",
                "operation-websocket-limited-session",
                "websocket-limited-session-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let limited_subscription_id =
        rrd_contract::CorrelationId::new("websocket-limited-subscription").unwrap();
    let limited_opened = client
        .open_subscription(
            &websocket_limited_session,
            OpenSubscription {
                subscription_id: limited_subscription_id.clone(),
                stream: SubscriptionStream::Changefeed {
                    scope: scope.clone(),
                },
                after_cursor: 0,
                batch_size: 1,
                max_in_flight: 1,
                retention_cursor_window: 128,
                lease_ms: 60_000,
                heartbeat_interval_ms: 100,
            },
            RequestOptions::mutation(
                "request-websocket-limited-open",
                "operation-websocket-limited-open",
                "websocket-limited-open-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let mut limited_socket = client
        .connect_websocket(&websocket_limited_session)
        .await
        .unwrap();
    limited_socket
        .subscribe(SubscriptionResume::from_snapshot(
            &limited_opened.subscription,
        ))
        .await
        .unwrap();
    match limited_socket.receive().await.unwrap_err() {
        Error::WebSocket(error) => {
            assert_eq!(error.error.code, ErrorCode::PermissionDenied);
            assert!(matches!(
                error.target,
                WebSocketErrorTarget::Subscription {
                    ref subscription_id,
                    connection_generation: 1,
                } if subscription_id == &limited_subscription_id
            ));
        }
        error => panic!("expected subscription authorization denial, got {error:?}"),
    }
    limited_socket.close().await.unwrap();
    client
        .close_subscription(
            &websocket_limited_session,
            CloseSubscription {
                subscription_id: limited_subscription_id,
            },
            RequestOptions::mutation(
                "request-websocket-limited-close",
                "operation-websocket-limited-close",
                "websocket-limited-close-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let subscription_id = rrd_contract::CorrelationId::new("rust-sdk-subscription").unwrap();
    let second_subscription_id =
        rrd_contract::CorrelationId::new("rust-sdk-subscription-two").unwrap();
    let opened = client
        .open_subscription(
            &session,
            OpenSubscription {
                subscription_id: subscription_id.clone(),
                stream: SubscriptionStream::Changefeed {
                    scope: scope.clone(),
                },
                after_cursor: 0,
                batch_size: 1,
                max_in_flight: 1,
                retention_cursor_window: 128,
                lease_ms: 60_000,
                heartbeat_interval_ms: 100,
            },
            RequestOptions::mutation(
                "request-subscription-open",
                "operation-subscription-open",
                "subscription-open-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let second_opened = client
        .open_subscription(
            &session,
            OpenSubscription {
                subscription_id: second_subscription_id.clone(),
                stream: SubscriptionStream::Changefeed {
                    scope: scope.clone(),
                },
                after_cursor: 0,
                batch_size: 1,
                max_in_flight: 1,
                retention_cursor_window: 128,
                lease_ms: 60_000,
                heartbeat_interval_ms: 100,
            },
            RequestOptions::mutation(
                "request-subscription-open-two",
                "operation-subscription-open-two",
                "subscription-open-key-two",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(opened.subscription.acknowledged_cursor, 0);
    assert_eq!(second_opened.subscription.acknowledged_cursor, 0);

    let mut socket = client.connect_websocket(&session).await.unwrap();
    let websocket_request = WebSocketRequest {
        operation: CanonicalId::new("query-execute").unwrap(),
        request: RequestEnvelope {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            context: RequestContext {
                request_id: rrd_contract::CorrelationId::new("websocket-query-request").unwrap(),
                operation_id: rrd_contract::CorrelationId::new("websocket-query-operation")
                    .unwrap(),
                idempotency_key: None,
                deadline_unix_ms: None,
            },
            resource: instance_resource(&instance),
            payload: serde_json::to_value(ExecuteQuery {
                scope: scope.clone(),
                query: "FROM record:document AT VALID 100 KNOWN HEAD PROJECT id, title".into(),
                parameters: BTreeMap::new(),
                budget: QueryBudget::default(),
            })
            .unwrap(),
        },
    };
    let request_target = socket.send_request(websocket_request).await.unwrap();
    let cancellation_id = rrd_contract::CorrelationId::new("websocket-query-cancellation").unwrap();
    socket
        .cancel(WebSocketCancel {
            cancellation_id: cancellation_id.clone(),
            target: request_target.clone(),
            reason: "exercise terminal cancellation correlation".into(),
        })
        .await
        .unwrap();
    loop {
        match socket.receive().await {
            Err(Error::WebSocket(error)) => {
                assert_eq!(error.error.code, ErrorCode::FailedPrecondition);
                assert!(matches!(
                    error.target,
                    WebSocketErrorTarget::Request { ref target }
                        if target == &request_target
                ));
                break;
            }
            Ok(frame) if matches!(frame.payload, WebSocketPayload::Heartbeat(_)) => {}
            result => panic!("unexpected WebSocket request result: {result:?}"),
        }
    }
    match receive_application_frame(&mut socket).await.payload {
        WebSocketPayload::Cancellation(result) => {
            assert_eq!(result.cancellation_id, cancellation_id);
            assert_eq!(result.target, request_target);
            assert_eq!(
                result.disposition,
                WebSocketCancellationDisposition::NotFound
            );
        }
        payload => panic!("unexpected cancellation result: {payload:?}"),
    }
    socket
        .subscribe(SubscriptionResume::from_snapshot(&opened.subscription))
        .await
        .unwrap();
    socket
        .subscribe(SubscriptionResume::from_snapshot(
            &second_opened.subscription,
        ))
        .await
        .unwrap();

    let mut subscribed = BTreeMap::new();
    let mut first_deliveries = BTreeMap::new();
    while subscribed.len() < 2 || first_deliveries.len() < 2 {
        match receive_application_frame(&mut socket).await.payload {
            WebSocketPayload::Subscribed(result) => {
                assert_eq!(result.subscription.connection_generation, 1);
                subscribed.insert(
                    result.subscription.subscription_id.clone(),
                    result.subscription,
                );
            }
            WebSocketPayload::Delivery(delivery) => {
                let SubscriptionDelivery::Changefeed { page } = delivery.delivery else {
                    panic!("expected a changefeed delivery");
                };
                assert_eq!(page.requested_after_cursor, 0);
                assert_eq!(page.through_cursor, 1);
                first_deliveries.insert(
                    delivery.subscription_id,
                    (delivery.delivery_sequence, page.through_cursor),
                );
            }
            WebSocketPayload::Backpressure(_) => {}
            payload => panic!("unexpected multiplex setup payload: {payload:?}"),
        }
    }
    assert!(subscribed.contains_key(&subscription_id));
    assert!(subscribed.contains_key(&second_subscription_id));

    for (id, (delivery_sequence, through_cursor)) in &first_deliveries {
        let generation = socket
            .subscription(id)
            .expect("multiplexed subscription is active")
            .connection_generation;
        socket
            .acknowledge(SubscriptionAcknowledgement {
                subscription_id: id.clone(),
                connection_generation: generation,
                delivery_sequence: *delivery_sequence,
                through_cursor: *through_cursor,
            })
            .await
            .unwrap();
    }
    let mut acknowledged = BTreeMap::new();
    let mut second_deliveries = BTreeMap::new();
    while acknowledged.len() < 2 {
        match receive_application_frame(&mut socket).await.payload {
            WebSocketPayload::Acknowledged(result) => {
                assert_eq!(result.subscription.acknowledged_cursor, 1);
                acknowledged.insert(
                    result.subscription.subscription_id.clone(),
                    result.subscription,
                );
            }
            WebSocketPayload::Delivery(delivery) => {
                second_deliveries.insert(delivery.subscription_id.clone(), delivery);
            }
            WebSocketPayload::Backpressure(_) => {}
            payload => panic!("unexpected multiplex ACK payload: {payload:?}"),
        }
    }
    let resume = SubscriptionResume::from_snapshot(
        socket
            .subscription(&subscription_id)
            .expect("primary subscription remains active"),
    );
    while !second_deliveries.contains_key(&subscription_id) {
        match receive_application_frame(&mut socket).await.payload {
            WebSocketPayload::Delivery(delivery) => {
                second_deliveries.insert(delivery.subscription_id.clone(), delivery);
            }
            WebSocketPayload::Backpressure(_) => {}
            payload => panic!("unexpected unacknowledged payload: {payload:?}"),
        }
    }
    let unacknowledged = &second_deliveries[&subscription_id];
    assert_eq!(unacknowledged.from_cursor, 1);
    assert_eq!(unacknowledged.through_cursor, 2);
    drop(socket);

    let mut socket = client.connect_websocket(&session).await.unwrap();
    socket.subscribe(resume).await.unwrap();
    match receive_application_frame(&mut socket).await.payload {
        WebSocketPayload::Subscribed(result) => {
            assert_eq!(result.subscription.subscription_id, subscription_id);
            assert_eq!(result.subscription.connection_generation, 2);
            assert_eq!(result.subscription.acknowledged_cursor, 1);
        }
        payload => panic!("unexpected reconnect payload: {payload:?}"),
    }
    let replay = loop {
        match receive_application_frame(&mut socket).await.payload {
            WebSocketPayload::Delivery(delivery) => break delivery,
            WebSocketPayload::Backpressure(_) => {}
            payload => panic!("unexpected replay payload: {payload:?}"),
        }
    };
    let SubscriptionDelivery::Changefeed { ref page } = replay.delivery else {
        panic!("expected replayed changefeed delivery");
    };
    assert_eq!(page.requested_after_cursor, 1);
    assert_eq!(page.through_cursor, 2);
    socket
        .acknowledge(SubscriptionAcknowledgement {
            subscription_id: subscription_id.clone(),
            connection_generation: replay.connection_generation,
            delivery_sequence: replay.delivery_sequence,
            through_cursor: replay.through_cursor,
        })
        .await
        .unwrap();
    loop {
        match receive_application_frame(&mut socket).await.payload {
            WebSocketPayload::Acknowledged(result)
                if result.subscription.subscription_id == subscription_id =>
            {
                assert_eq!(result.subscription.acknowledged_cursor, 2);
                break;
            }
            WebSocketPayload::Backpressure(_) => {}
            payload => panic!("unexpected replay ACK payload: {payload:?}"),
        }
    }
    socket.unsubscribe(&subscription_id).await.unwrap();
    // Delivery runs on its own polling cadence, so the next page after the
    // acknowledged cursor may already be on the wire when the server handles
    // the unsubscribe. Such an in-flight page continues the acknowledged
    // stream and is bounded by the subscription's max_in_flight (1 here); the
    // server removes the subscription before it answers, so the `unsubscribed`
    // frame ends the stream.
    let mut in_flight = 0;
    loop {
        match receive_application_frame(&mut socket).await.payload {
            WebSocketPayload::Unsubscribed(result) => {
                assert_eq!(result.subscription.subscription_id, subscription_id);
                assert_eq!(result.subscription.acknowledged_cursor, 2);
                break;
            }
            WebSocketPayload::Delivery(delivery) if delivery.subscription_id == subscription_id => {
                in_flight += 1;
                assert_eq!(
                    in_flight, 1,
                    "max_in_flight 1 admits one unacknowledged delivery"
                );
                assert_eq!(delivery.connection_generation, replay.connection_generation);
                assert!(delivery.delivery_sequence > replay.delivery_sequence);
                assert_eq!(delivery.from_cursor, 2);
            }
            WebSocketPayload::Backpressure(_) => {}
            payload => panic!("unexpected unsubscribe payload: {payload:?}"),
        }
    }
    socket.close().await.unwrap();

    let closed = client
        .close_subscription(
            &session,
            CloseSubscription {
                subscription_id: subscription_id.clone(),
            },
            RequestOptions::mutation(
                "request-subscription-close",
                "operation-subscription-close",
                "subscription-close-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        closed.subscription.status,
        rrd_contract::SubscriptionStatus::Closed
    );
    let second_closed = client
        .close_subscription(
            &session,
            CloseSubscription {
                subscription_id: second_subscription_id,
            },
            RequestOptions::mutation(
                "request-subscription-close-two",
                "operation-subscription-close-two",
                "subscription-close-key-two",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        second_closed.subscription.status,
        rrd_contract::SubscriptionStatus::Closed
    );
    let context = client
        .assemble_context(
            &session,
            AssembleContext {
                scope: scope.clone(),
                query: "Alpha".into(),
                valid_at: 100,
                seeds: Vec::new(),
                max_graph_depth: 2,
                max_items: 16,
                max_output_bytes: 16_384,
                max_storage_keys: 1_024,
            },
            RequestOptions::read("request-context", "operation-context").unwrap(),
        )
        .await
        .unwrap();
    assert!(context
        .items
        .iter()
        .any(|item| item.identity == "record:document:alpha"
            && item
                .evidence
                .iter()
                .any(|evidence| evidence.kind == ContextEvidenceKind::Text)));
    assert!(context.selected_versions > 0);
    context.read_evidence.validate().unwrap();
    assert!(context
        .read_evidence
        .paths
        .iter()
        .any(|path| path.path == ReadAccessPath::RecordVersions));
    let query_request = ExecuteQuery {
        scope: scope.clone(),
        query: "FROM record:document AT VALID 100 KNOWN HEAD PROJECT id, title EXPLAIN CONTRACT"
            .into(),
        parameters: BTreeMap::new(),
        budget: QueryBudget {
            max_storage_keys: 100,
            max_rows: 10,
            max_output_bytes: 4_096,
            max_batch_rows: 10,
            ..QueryBudget::default()
        },
    };
    let query = client
        .execute_query(
            &session,
            query_request.clone(),
            RequestOptions::read("request-query", "operation-query").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(query.rows.len(), 1);
    assert_eq!(query.rows[0].identity, "record:document:alpha");
    assert!(query.execution.selected_versions > 0);
    query.execution.read_evidence.validate().unwrap();
    assert!(query
        .execution
        .read_evidence
        .paths
        .iter()
        .any(|path| path.path == ReadAccessPath::RecordVersions));
    let expired = client
        .execute_query(
            &session,
            query_request,
            RequestOptions::new("request-expired", "operation-expired", None, Some(1)).unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(expired, Error::Timeout));

    let transaction = client
        .begin_transaction(
            &session,
            BeginTransaction {
                scope: CanonicalId::new("claims").unwrap(),
                timeout_ms: 60_000,
            },
            RequestOptions::mutation("request-begin", "operation-begin", "transaction-begin")
                .unwrap(),
        )
        .await
        .unwrap();
    let preview = client
        .preview_transaction(
            &session,
            &transaction.transaction_id,
            PreviewTransaction {
                mutations: vec![TransactionMutation::AssertClaim {
                    subject: CanonicalId::new("document-alpha").unwrap(),
                    predicate: CanonicalId::new("contains").unwrap(),
                    object: "preview-only".into(),
                    valid_from: 100,
                    tx_time: 100,
                    producer: CanonicalId::new("rust-sdk").unwrap(),
                    confidence: Some(0.9),
                }],
                valid_at: Some(100),
                max_storage_keys: 100,
            },
            RequestOptions::mutation("request-preview", "operation-preview", "prepare-preview")
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(preview.transaction_id, transaction.transaction_id);
    let aborted = client
        .abort_transaction(
            &session,
            &transaction.transaction_id,
            AbortTransaction {},
            RequestOptions::mutation("request-abort", "operation-abort", "transaction-abort")
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(aborted.state, rrd_contract::TransactionState::Aborted);

    let changes = client
        .read_changefeed(
            &session,
            ReadChangefeed {
                scope: scope.clone(),
                after_cursor: 0,
                limit: 64,
            },
            RequestOptions::read("request-changes", "operation-changes").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(changes.through_cursor, runtime_head);
    assert_eq!(
        changes.changes.len(),
        usize::try_from(runtime_head).unwrap()
    );

    let diagnostics = client
        .read_diagnostic_snapshot(
            &session,
            ReadDiagnosticSnapshot {
                scope: scope.clone(),
                graph_valid_at_unix_ms: 1_100,
                graph_known_at_cursor: Some(runtime_head),
                graph_compare_cursor: 0,
                runtime_max_scanned_changes: 1_024,
                changes_after_cursor: 0,
                change_limit: 64,
                audit_after_sequence: 0,
                audit_limit: 128,
            },
            RequestOptions::read("request-diagnostics", "operation-diagnostics").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(diagnostics.instance.id, instance);
    assert_eq!(diagnostics.read.runtime_cursor, runtime_head);
    assert!(diagnostics.read.control_sequence > 0);
    assert_eq!(diagnostics.read.runtime_manifest_sha256.len(), 64);
    assert_eq!(diagnostics.changes.head_cursor, runtime_head);
    assert_eq!(diagnostics.schema.as_ref().unwrap().revision, 2);
    let physical = client
        .read_storage_diagnostics(
            &session,
            rrd_contract::ReadStorageDiagnostics {
                scope: format!("instance:{}", client.instance_id()),
            },
            RequestOptions::read("storage-only", "storage-only").unwrap(),
        )
        .await
        .unwrap();
    physical.validate().unwrap();
    assert_eq!(physical.profile, StorageProfileKind::RrflowKv);
    assert!(physical.rrflow_kv.is_some());
    assert!(physical.admission.is_some());
    let cursor_request = |action| rrd_contract::InspectStorageCursor {
        scope: format!("instance:{}", client.instance_id()),
        cursor_id: rrd_contract::CorrelationId::new("native-inspection").unwrap(),
        action,
    };
    assert!(client
        .inspect_storage_cursor(
            &websocket_limited_session,
            cursor_request(rrd_contract::StorageCursorAction::Open {
                batch_rows: 1,
                batch_bytes: 4096,
                lifetime_ms: 30_000
            }),
            RequestOptions::read("cursor-denied", "cursor-denied").unwrap()
        )
        .await
        .is_err());
    let held = client
        .inspect_storage_cursor(
            &session,
            cursor_request(rrd_contract::StorageCursorAction::Open {
                batch_rows: 1,
                batch_bytes: 4096,
                lifetime_ms: 30_000,
            }),
            RequestOptions::read("cursor-open", "cursor-open").unwrap(),
        )
        .await
        .unwrap();
    assert!(held.rows > 0 && held.allocated_bytes > 0);
    let cancelled = client
        .inspect_storage_cursor(
            &session,
            cursor_request(rrd_contract::StorageCursorAction::CancelSource {}),
            RequestOptions::read("cursor-cancel", "cursor-cancel").unwrap(),
        )
        .await
        .unwrap();
    assert!(!cancelled.source_open);
    assert_eq!(cancelled.page_sha256, held.page_sha256);
    assert!(
        client
            .inspect_storage_cursor(
                &session,
                cursor_request(rrd_contract::StorageCursorAction::Release {}),
                RequestOptions::read("cursor-release", "cursor-release").unwrap()
            )
            .await
            .unwrap()
            .released
    );
    assert!(client
        .read_storage_diagnostics(
            &websocket_limited_session,
            rrd_contract::ReadStorageDiagnostics {
                scope: format!("instance:{}", client.instance_id())
            },
            RequestOptions::read("storage-forbidden", "storage-forbidden").unwrap()
        )
        .await
        .is_err());
    assert!(diagnostics.models.models.iter().any(|model| {
        model.id.as_str() == "document"
            && model.kind == rrd_contract::DiagnosticModelKind::Record
            && model.property_count == 1
            && model.required_property_count == 1
    }));
    assert_eq!(diagnostics.graph.valid_at_unix_ms, 1_100);
    assert_eq!(diagnostics.graph.known_at_cursor, runtime_head);
    assert!(diagnostics
        .graph
        .records
        .iter()
        .any(|record| record.reference.kind.as_str() == "document"));
    assert_eq!(diagnostics.graph_difference.from_cursor, 0);
    assert_eq!(diagnostics.graph_difference.to_cursor, runtime_head);
    assert_eq!(
        diagnostics.graph_difference.added_records.len(),
        diagnostics.graph.records.len()
    );
    assert_eq!(diagnostics.retention.leases.len(), 1);
    assert_eq!(diagnostics.retention.pins.len(), 1);
    assert_eq!(
        diagnostics.retention.oldest_retained_cursor,
        Some(diagnostics.read.runtime_cursor)
    );
    assert_eq!(diagnostics.retention.leases[0].owner, "rust-sdk-fixture");
    assert_eq!(diagnostics.vector_artifacts.revision, 0);
    assert!(diagnostics.vector_artifacts.artifacts.is_empty());
    let impossible_graph_cursor = client
        .read_diagnostic_snapshot(
            &session,
            ReadDiagnosticSnapshot {
                scope: scope.clone(),
                graph_valid_at_unix_ms: 1_100,
                graph_known_at_cursor: Some(runtime_head.saturating_add(1)),
                graph_compare_cursor: 0,
                runtime_max_scanned_changes: 1_024,
                changes_after_cursor: 0,
                change_limit: 64,
                audit_after_sequence: 0,
                audit_limit: 32,
            },
            RequestOptions::read("request-diagnostics-future", "operation-diagnostics-future")
                .unwrap(),
        )
        .await;
    assert!(impossible_graph_cursor.is_err());
    assert!(diagnostics
        .sections
        .iter()
        .any(|section| section.id.as_str() == "vector-collections"));

    let audit = client
        .read_audit(
            &session,
            ReadAudit {
                after_sequence: 0,
                limit: 128,
            },
            RequestOptions::read("request-audit", "operation-audit").unwrap(),
        )
        .await
        .unwrap();
    assert!(audit.records.iter().any(|record| {
        record.action == rrd_contract::SecurityAction::QueryExecute
            && record.phase == rrd_contract::AuditPhase::Completed
    }));
    let exported = client
        .export_audit(
            &session,
            ExportAudit {
                after_sequence: 0,
                limit: 128,
            },
            RequestOptions::read("request-audit-export", "operation-audit-export").unwrap(),
        )
        .await
        .unwrap();
    exported.validate().unwrap();
    assert_eq!(
        exported.record_count as usize,
        exported.json_lines.lines().count()
    );
    assert_eq!(
        exported.content_sha256,
        digest::sha256_hex(exported.json_lines.as_bytes())
    );
    assert!(audit.records.iter().any(|record| {
        record.action == rrd_contract::SecurityAction::MemoryContextRead
            && record.principal_id.as_ref().map(CanonicalId::as_str) == Some("rust-sdk")
            && record.phase == rrd_contract::AuditPhase::Completed
            && record.decision == rrd_contract::AuditDecision::Allowed
    }));
    assert!(audit.records.iter().any(|record| {
        record.action == rrd_contract::SecurityAction::SubscriptionConnect
            && record.phase == rrd_contract::AuditPhase::Completed
            && record.decision == rrd_contract::AuditDecision::Allowed
    }));
    assert!(audit.records.iter().any(|record| {
        record.action == rrd_contract::SecurityAction::SubscriptionAck
            && record.phase == rrd_contract::AuditPhase::Completed
            && record.decision == rrd_contract::AuditDecision::Allowed
    }));

    let launch = LaunchPromptFlight {
        scope: scope.clone(),
        prompt: "Alpha".into(),
        provider: PromptFlightProvider::Observe,
        context_mode: PromptFlightContextMode::Pruned,
        context: PromptFlightContextRequest {
            valid_at: 100,
            seeds: Vec::new(),
            max_graph_depth: 2,
            max_items: 16,
            max_output_bytes: 16_384,
            max_storage_keys: 1_024,
        },
        acceptance_marker: None,
        task_id: None,
    };
    let flight = client
        .launch_prompt_flight(
            &session,
            launch.clone(),
            RequestOptions::mutation("request-flight", "operation-flight", "flight-key").unwrap(),
        )
        .await
        .unwrap();
    assert!(!flight.idempotent_replay);
    let record = &flight.flight.record;
    assert_eq!(record.metrics.context_items, context.items.len() as u64);
    assert_eq!(record.metrics.context_output_bytes, context.output_bytes);
    // The flight is the first runtime write of this session: one schema
    // mutation installing both flight tables precedes the row.
    assert_eq!(record.read.runtime_cursor, runtime_head);
    assert_eq!(record.flight_cursor, runtime_head + 2);
    assert_eq!(
        flight
            .flight
            .events
            .iter()
            .map(|event| event.stage)
            .collect::<Vec<_>>(),
        vec![
            PromptFlightStage::Prompt,
            PromptFlightStage::Context,
            PromptFlightStage::Outcome
        ]
    );
    let replay = client
        .launch_prompt_flight(
            &session,
            launch.clone(),
            RequestOptions::mutation(
                "request-flight-again",
                "operation-flight-again",
                "flight-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.flight, flight.flight);
    let flights = client
        .list_prompt_flights(
            &session,
            ListPromptFlights {
                scope: scope.clone(),
                after_cursor: 0,
                limit: 8,
                max_storage_keys: 1_024,
            },
            RequestOptions::read("request-flight-list", "operation-flight-list").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(flights.flights, vec![record.clone()]);
    assert!(!flights.truncated);
    let read = client
        .read_prompt_flight(
            &session,
            ReadPromptFlight {
                scope: scope.clone(),
                flight_id: record.flight_id.clone(),
                max_storage_keys: 1_024,
            },
            RequestOptions::read("request-flight-read", "operation-flight-read").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(read.flight, flight.flight);
    assert!(read
        .read_evidence
        .paths
        .iter()
        .any(|path| path.path == ReadAccessPath::EventVersions));
    let after_flight = client
        .read_changefeed(
            &session,
            ReadChangefeed {
                scope: scope.clone(),
                after_cursor: runtime_head,
                limit: 64,
            },
            RequestOptions::read("request-changes-flight", "operation-changes-flight").unwrap(),
        )
        .await
        .unwrap();
    // Schema, row, one edge per record the context used, and three events.
    assert_eq!(record.use_count, 1, "the flight used document:alpha");
    assert_eq!(flight.flight.uses[0].reference.id.as_str(), "alpha");
    let flight_changes = 5 + record.use_count;
    assert_eq!(after_flight.through_cursor, runtime_head + flight_changes);
    assert_eq!(after_flight.changes.len() as u64, flight_changes);
    let flight_audit = client
        .read_audit(
            &session,
            ReadAudit {
                after_sequence: 0,
                limit: 256,
            },
            RequestOptions::read("request-audit-flight", "operation-audit-flight").unwrap(),
        )
        .await
        .unwrap();
    assert!(flight_audit.records.iter().any(|record| {
        record.action == rrd_contract::SecurityAction::FlightLaunch
            && record.phase == rrd_contract::AuditPhase::Completed
            && record.decision == rrd_contract::AuditDecision::Allowed
    }));
    assert!(flight_audit.records.iter().any(|record| {
        record.action == rrd_contract::SecurityAction::FlightRead
            && record.phase == rrd_contract::AuditPhase::Completed
            && record.decision == rrd_contract::AuditDecision::Allowed
    }));

    // A named task over the real socket: open, join one flight, read, list,
    // close, and refuse a flight once closed.
    let opened = client
        .open_task(
            &session,
            OpenTask {
                scope: scope.clone(),
                title: "Alpha recall".into(),
                goal: "Answer Alpha from the estate.".into(),
            },
            RequestOptions::mutation("request-task-open", "operation-task-open", "task-key")
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(!opened.idempotent_replay);
    assert_eq!(opened.task.status, TaskStatus::Open);
    let joined = client
        .launch_prompt_flight(
            &session,
            LaunchPromptFlight {
                task_id: Some(opened.task.task_id.clone()),
                ..launch.clone()
            },
            RequestOptions::mutation(
                "request-task-flight",
                "operation-task-flight",
                "task-flight-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        joined.flight.record.task_id.as_ref(),
        Some(&opened.task.task_id)
    );
    let read_task = client
        .read_task(
            &session,
            ReadTask {
                scope: scope.clone(),
                task_id: opened.task.task_id.clone(),
                max_storage_keys: 1_024,
            },
            RequestOptions::read("request-task-read", "operation-task-read").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(read_task.task, opened.task);
    let closed = client
        .close_task(
            &session,
            CloseTask {
                scope: scope.clone(),
                task_id: opened.task.task_id.clone(),
                status: TaskStatus::Completed,
                outcome: Some("Alpha answered.".into()),
            },
            RequestOptions::mutation("request-task-close", "operation-task-close", "task-close")
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(closed.task.status, TaskStatus::Completed);
    let tasks = client
        .list_tasks(
            &session,
            ListTasks {
                scope: scope.clone(),
                after_cursor: 0,
                limit: 8,
                max_storage_keys: 1_024,
            },
            RequestOptions::read("request-task-list", "operation-task-list").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tasks.tasks, vec![closed.task.clone()]);
    match client
        .launch_prompt_flight(
            &session,
            LaunchPromptFlight {
                task_id: Some(opened.task.task_id.clone()),
                ..launch.clone()
            },
            RequestOptions::mutation(
                "request-task-flight-late",
                "operation-task-flight-late",
                "task-flight-late-key",
            )
            .unwrap(),
        )
        .await
    {
        Err(Error::Api { error, .. }) => {
            assert_eq!(error.code, ErrorCode::FailedPrecondition)
        }
        other => panic!("a closed task admits no flight: {other:?}"),
    }

    assert!(matches!(
        RrdClient::connect_local(
            "192.0.2.1:9477".parse().unwrap(),
            instance.clone(),
            ClientConfig::default(),
        ),
        Err(Error::Contract(_))
    ));
    drop(client);
    for _ in 0..10 {
        let reconnected = RrdClient::connect_local(
            proxy_address,
            instance.clone(),
            ClientConfig {
                request_timeout: INTEGRATION_IO_TIMEOUT,
                max_attempts: 2,
                websocket_limits: Default::default(),
            },
        )
        .unwrap();
        assert_eq!(
            reconnected.capabilities().await.unwrap().protocol_version,
            1
        );
        drop(reconnected);
    }
    proxy_shutdown.send(true).unwrap();
    proxy.await.unwrap();
    shutdown.send(()).unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn loopback_rust_client_passes_the_shared_deployment_corpus() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("local-conformance");
    let storage = RrflowKvStore::open(&root).unwrap();
    let instance = CanonicalId::new("local-conformance").unwrap();
    let principal = Principal {
        id: CanonicalId::new("local-client").unwrap(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"local-conformance-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: [
            Action::SessionCreate,
            Action::TransactionBegin,
            Action::TransactionCommit,
            Action::QueryExecute,
        ]
        .into_iter()
        .map(|action| ResourceGrant {
            action,
            resource_prefix: ResourcePath {
                segments: vec![ResourceId::new(ResourceKind::Instance, instance.as_str()).unwrap()],
            },
            data_policy: None,
        })
        .collect(),
    };
    SecurityRepository::new(&storage, instance.clone())
        .initialize(
            SecurityState {
                format_version: SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([(principal.id.clone(), principal)]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "bootstrap",
            "request-local-conformance-bootstrap",
            "operation-local-conformance-bootstrap",
        )
        .unwrap();
    drop(storage);

    let engine = RrdEngine::open(&root, instance.clone(), COMPONENT_TOKEN_KEY).unwrap();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let address = server.local_addr();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));

    let client = RrdClient::connect_local(address, instance, ClientConfig::default()).unwrap();
    let session = client
        .create_session(
            CanonicalId::new("local-client").unwrap(),
            "local-conformance-key",
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 300_000,
                    max_open_transactions: 1,
                },
            },
            RequestOptions::mutation(
                "request-local-conformance-session",
                "operation-local-conformance-session",
                "local-conformance-session-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    commit_client_deployment_corpus(&client, &session, "local-conformance").await;
    assert_client_deployment_corpus(
        &client,
        &session,
        EndpointPresentation::LoopbackHttpWebsocket,
    )
    .await;

    shutdown.send(()).unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn remote_transport_requires_mutual_tls_and_exact_server_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("mtls-instance");
    let storage = RrflowKvStore::open(&root).unwrap();
    let instance = CanonicalId::new("mtls-sdk-test").unwrap();
    let principal = Principal {
        id: CanonicalId::new("mtls-client").unwrap(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"mtls-api-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: [
            Action::SessionCreate,
            Action::TransactionBegin,
            Action::TransactionCommit,
            Action::QueryExecute,
            Action::ChangefeedFollow,
            Action::WebSocketConnect,
            Action::SubscriptionOpen,
            Action::SubscriptionConnect,
            Action::SubscriptionAck,
            Action::SubscriptionClose,
        ]
        .into_iter()
        .map(|action| ResourceGrant {
            action,
            resource_prefix: ResourcePath {
                segments: vec![ResourceId::new(ResourceKind::Instance, instance.as_str()).unwrap()],
            },
            data_policy: None,
        })
        .collect(),
    };
    SecurityRepository::new(&storage, instance.clone())
        .initialize(
            SecurityState {
                format_version: SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([(principal.id.clone(), principal)]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "bootstrap",
            "request-bootstrap-mtls",
            "operation-bootstrap-mtls",
        )
        .unwrap();
    drop(storage);

    let (ca, issuer) = test_ca();
    let (server_chain, server_key) = test_identity(
        &issuer,
        vec!["localhost".into()],
        ExtendedKeyUsagePurpose::ServerAuth,
    );
    let (client_chain, client_key) =
        test_identity(&issuer, Vec::new(), ExtendedKeyUsagePurpose::ClientAuth);
    let server_tls = RrdMutualTlsServerConfig::new(server_chain, server_key, roots(&ca)).unwrap();
    let engine = RrdEngine::open(&root, instance.clone(), COMPONENT_TOKEN_KEY).unwrap();
    let server =
        RrdHttpServer::bind_mtls(engine, "127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
    let endpoint = format!("https://localhost:{}", server.local_addr().port());
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));

    let anonymous_tls = RustlsClientConfig::builder()
        .with_root_certificates(roots(&ca))
        .with_no_client_auth();
    let anonymous = RrdClient::connect_mtls(
        endpoint.clone(),
        instance.clone(),
        anonymous_tls,
        ClientConfig::default(),
    )
    .unwrap();
    assert!(matches!(
        anonymous.capabilities().await,
        Err(Error::Transport(_))
    ));

    let (wrong_name_chain, wrong_name_key) =
        test_identity(&issuer, Vec::new(), ExtendedKeyUsagePurpose::ClientAuth);
    let wrong_name_tls = RustlsClientConfig::builder()
        .with_root_certificates(roots(&ca))
        .with_client_auth_cert(wrong_name_chain, wrong_name_key)
        .unwrap();
    let wrong_identity = RrdClient::connect_mtls(
        format!("https://127.0.0.1:{}", server_address_port(&endpoint)),
        instance.clone(),
        wrong_name_tls,
        ClientConfig::default(),
    )
    .unwrap();
    assert!(matches!(
        wrong_identity.capabilities().await,
        Err(Error::Transport(_))
    ));

    let client_tls = RustlsClientConfig::builder()
        .with_root_certificates(roots(&ca))
        .with_client_auth_cert(client_chain, client_key)
        .unwrap();
    let client =
        RrdClient::connect_mtls(endpoint, instance, client_tls, ClientConfig::default()).unwrap();
    let capabilities = client.capabilities().await.unwrap();
    assert_eq!(capabilities.protocol_version, 1);
    assert_eq!(
        capabilities.deployment.deployment_form,
        DeploymentForm::SingleNodeServer
    );
    assert_eq!(
        capabilities.deployment.storage_profile,
        StorageProfileKind::RrflowKv
    );
    assert_eq!(
        capabilities.deployment.endpoint_presentation,
        EndpointPresentation::LoopbackHttpWebsocket
    );
    assert_eq!(
        capabilities
            .capabilities
            .iter()
            .find(|capability| capability.name.as_str() == "remote-listen")
            .unwrap()
            .status,
        rrd_contract::CapabilityStatus::Experimental
    );
    let session = client
        .create_session(
            CanonicalId::new("mtls-client").unwrap(),
            "mtls-api-key",
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 300_000,
                    max_open_transactions: 1,
                },
            },
            RequestOptions::mutation(
                "request-mtls-session",
                "operation-mtls-session",
                "mtls-session-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    commit_client_deployment_corpus(&client, &session, "remote-conformance").await;
    assert_client_deployment_corpus(
        &client,
        &session,
        EndpointPresentation::LoopbackHttpWebsocket,
    )
    .await;
    let subscription_id = rrd_contract::CorrelationId::new("mtls-subscription").unwrap();
    let opened = client
        .open_subscription(
            &session,
            OpenSubscription {
                subscription_id: subscription_id.clone(),
                stream: SubscriptionStream::Changefeed {
                    scope: "instance:mtls-sdk-test".into(),
                },
                after_cursor: 0,
                batch_size: 3,
                max_in_flight: 1,
                retention_cursor_window: 128,
                lease_ms: 60_000,
                heartbeat_interval_ms: 100,
            },
            RequestOptions::mutation(
                "request-mtls-subscription",
                "operation-mtls-subscription",
                "mtls-subscription-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let mut socket = client.connect_websocket(&session).await.unwrap();
    socket
        .subscribe(SubscriptionResume::from_snapshot(&opened.subscription))
        .await
        .unwrap();
    match receive_application_frame(&mut socket).await.payload {
        WebSocketPayload::Subscribed(result) => {
            assert_eq!(result.subscription.connection_generation, 1);
        }
        payload => panic!("unexpected mTLS connect payload: {payload:?}"),
    }
    let pushed = receive_application_frame(&mut socket).await;
    assert!(matches!(
        pushed.payload,
        WebSocketPayload::Delivery(ref delivery)
            if matches!(
                &delivery.delivery,
                SubscriptionDelivery::Changefeed { page }
                    if page.requested_after_cursor == 0 && page.through_cursor == 3
            )
    ));
    socket.close().await.unwrap();

    drop(client);
    drop(anonymous);
    shutdown.send(()).unwrap();
    task.await.unwrap().unwrap();
}

fn server_address_port(endpoint: &str) -> u16 {
    endpoint
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse().ok())
        .expect("test TLS endpoint carries a port")
}
