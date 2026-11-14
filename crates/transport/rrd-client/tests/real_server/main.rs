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
        "../../../../../fixtures/rrd-deployment-conformance-v1.json"
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

mod negotiation;
mod transport;
