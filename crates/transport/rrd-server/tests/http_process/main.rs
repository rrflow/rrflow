#[path = "../support/server_fixture.rs"]
mod server_fixture;

use server_fixture::*;

use rrd_contract::{
    transaction_operation_sha256, CanonicalId, CorrelationId, DeploymentConformanceCorpus,
    InstallationManagedPathKind, InstallationTargetKind, ReadAccessPath, ReadEvidence,
    TransactionMutation,
};

use rrd_core::{
    RuntimeCommit, RuntimeEventSchema, RuntimeLogicalModel, RuntimeMutation, RuntimeProperties,
    RuntimePropertySchema, RuntimeRecord, RuntimeRecordSchema, RuntimeRef, RuntimeRelationSchema,
    RuntimeSchemaRegistry, RuntimeType, RuntimeValue, RuntimeValueType, ScopeId,
};

use rrd_engine::RrdEngine;

use rrd_security::{
    Action as SecurityAction, AuditDecision, AuditPhase, DataPolicy, JwtIssueRequest, JwtIssuer,
    Principal, PrincipalKind, ResourceGrant, SecurityRepository, SecurityState, SECURITY_FORMAT,
};

use rrd_server::{HttpError, RrdHttpServer, RrdJwtVerificationKey, RRD_MAX_BODY_BYTES};

use rrd_store::RrflowKvStore;

use rrd_store::StorageEngine;

use serde_json::{json, Value};

use std::collections::{BTreeMap, BTreeSet};

use std::io::{Read, Write};

use std::net::{Shutdown, SocketAddr, TcpStream};

use std::path::{Path, PathBuf};

use std::process::{Child, Command};

use std::sync::{Condvar, Mutex};

use std::thread::JoinHandle;

use std::time::{Duration, Instant};

#[test]
fn authenticated_storage_diagnostic_trace_correlates_store_without_secrets() {
    use std::sync::Arc;

    const CHILD: &str = "RRFLOW_TEST_DIAGNOSTIC_TRACE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        for mode in ["enabled", "disabled"] {
            let result = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "authenticated_storage_diagnostic_trace_correlates_store_without_secrets",
                    "--nocapture",
                ])
                .env(CHILD, mode)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "isolated diagnostic trace test ({mode}) failed: stdout={} stderr={}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        return;
    }
    let timing_enabled = std::env::var(CHILD).unwrap() == "enabled";

    #[derive(Clone)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer_output = output.clone();
    let filter = if timing_enabled {
        "rrd_store::diagnostic=debug,rrd_server::diagnostic=debug,rrd_engine::transaction_diagnostic=debug"
    } else {
        "rrd_store::diagnostic=debug,rrd_server::diagnostic=debug"
    };
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .with_writer(move || Capture(writer_output.clone()))
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("instance");
    let engine = RrflowKvStore::open(&root).unwrap();
    let instance = CanonicalId::new("socket-test").unwrap();
    let resource = rrd_contract::ResourcePath {
        segments: vec![rrd_contract::ResourceId::new(
            rrd_contract::ResourceKind::Instance,
            "socket-test",
        )
        .unwrap()],
    };
    let principal = Principal {
        id: CanonicalId::new("trace-client").unwrap(),
        kind: PrincipalKind::User,
        credential_sha256: rrd_core::digest::sha256_hex(b"trace-test-api-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: [
            SecurityAction::SessionCreate,
            SecurityAction::DiagnosticsRead,
            SecurityAction::TransactionBegin,
        ]
        .into_iter()
        .map(|action| ResourceGrant {
            action,
            resource_prefix: resource.clone(),
            data_policy: None,
        })
        .collect(),
    };
    SecurityRepository::new(&engine, instance)
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
            "request-bootstrap",
            "operation-bootstrap",
        )
        .unwrap();
    drop(engine);
    let server = start(&root);
    let create = envelope(
        json!({"limits": {"idle_timeout_ms": 60000, "absolute_timeout_ms": 300000, "max_open_transactions": 2}}),
        Some("trace-session"),
        None,
    );
    let (status, created) = post_with_api_key(
        &server,
        "/v1/sessions",
        &create,
        "trace-client",
        "trace-test-api-key",
    );
    assert_eq!(status, 200, "{created}");
    let session = payload(&created)["session_id"].as_str().unwrap();
    let token = payload(&created)["token"].as_str().unwrap();
    let mut request = envelope(json!({"scope": "instance:socket-test"}), None, None);
    request["context"]["request_id"] = json!("private-request-sentinel");
    let (status, response) = post(
        &server,
        "/v1/diagnostics/storage/read",
        &request,
        Some((session, token)),
    );
    assert_eq!(status, 200, "{response}");
    let mut begin = envelope(
        json!({"scope": "claims", "timeout_ms": 5000}),
        Some("private-begin-key"),
        None,
    );
    begin["context"]["request_id"] = json!("private-begin-sentinel");
    let (status, created_tx) = post(&server, "/v1/transactions", &begin, Some((session, token)));
    assert_eq!(status, 200, "{created_tx}");
    let (status, replayed_tx) = post(&server, "/v1/transactions", &begin, Some((session, token)));
    assert_eq!(status, 200, "{replayed_tx}");
    assert_eq!(payload(&created_tx), payload(&replayed_tx));
    let mut second_begin = begin.clone();
    second_begin["context"]["idempotency_key"] = json!("second-begin");
    let (status, second) = post(
        &server,
        "/v1/transactions",
        &second_begin,
        Some((session, token)),
    );
    assert_eq!(status, 200, "{second}");
    let mut over_quota = begin.clone();
    over_quota["context"]["idempotency_key"] = json!("third-begin");
    over_quota["context"]["request_id"] = json!("private-error-sentinel");
    let (status, rejection) = post(
        &server,
        "/v1/transactions",
        &over_quota,
        Some((session, token)),
    );
    assert_eq!(status, 429, "{rejection}");
    server.stop();

    let bytes = output.lock().unwrap().clone();
    let raw = std::str::from_utf8(&bytes).unwrap();
    assert!(!raw.contains("private-request-sentinel"));
    assert!(!raw.contains("trace-test-api-key"));
    assert!(!raw.contains("private-begin-sentinel"));
    assert!(!raw.contains("private-begin-key"));
    assert!(!raw.contains("private-error-sentinel"));
    assert!(!raw.contains(token));
    let events: Vec<Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let begin_events: Vec<_> = events
        .iter()
        .filter(|event| event["target"] == "rrd_engine::transaction_diagnostic")
        .collect();
    if !timing_enabled {
        assert!(begin_events.is_empty());
    } else {
        for outcome in ["created", "replayed"] {
            let event = begin_events
                .iter()
                .find(|event| event["fields"]["outcome"] == outcome)
                .expect("HTTP transaction begin must emit its actual outcome");
            assert_eq!(
                event["fields"]["request_sha256"],
                rrd_core::digest::sha256_hex(b"private-begin-sentinel")
            );
            for field in [
                "validation_ns",
                "admission_ns",
                "authentication_ns",
                "replay_ns",
                "snapshot_ns",
                "publication_ns",
                "total_ns",
            ] {
                assert!(event["fields"][field].is_u64(), "missing {field}: {event}");
            }
        }
        let rejection = begin_events
            .iter()
            .find(|event| event["fields"]["outcome"] == "error")
            .expect("quota rejection must emit an error timing");
        assert_eq!(
            rejection["fields"]["request_sha256"],
            rrd_core::digest::sha256_hex(b"private-error-sentinel")
        );
    }
    let event = events
        .iter()
        .find(|event| event["target"] == "rrd_store::diagnostic")
        .expect("actual HTTP diagnostic must emit a store timing event");
    assert_eq!(
        event["span"]["request_sha256"],
        rrd_core::digest::sha256_hex(b"private-request-sentinel")
    );
    assert!(event["fields"]["mutex_wait_ns"].is_u64());
    assert!(event["fields"]["mutex_held_assembly_ns"].is_u64());
    assert!(event["fields"]["memtable_profile_ns"].is_u64());
    let service = events
        .iter()
        .find(|event| event["target"] == "rrd_server::diagnostic")
        .expect("actual HTTP diagnostic must emit a service timing event");
    assert_eq!(
        service["span"]["request_sha256"],
        rrd_core::digest::sha256_hex(b"private-request-sentinel")
    );
    assert_eq!(service["fields"]["success"], true);
    let elapsed = service["fields"]["service_elapsed_ns"].as_u64().unwrap();
    let wait = event["fields"]["mutex_wait_ns"].as_u64().unwrap();
    let hold = event["fields"]["mutex_held_assembly_ns"].as_u64().unwrap();
    assert!(u128::from(wait) + u128::from(hold) <= u128::from(elapsed));
}

mod daemon;
mod data_transaction;
mod fixtures;
mod installed_flows;
mod query_and_changefeed;
mod security_sessions;
mod socket_faults;

use fixtures::*;
