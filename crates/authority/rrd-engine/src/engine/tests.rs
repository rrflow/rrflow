use super::{
    public_runtime_commit, Invocation, InvocationCompletion, InvocationCredential, RrdEngine,
    RrdOperation, ServiceError,
};
use rrd_contract::{
    AbortTransaction, AuditDecision, AuditPhase, BeginTransaction, CanonicalId, CloseSession,
    CommitTransaction, CorrelationId, CreateSession, DataCatalogueIdentity, EnsureVectorCollection,
    NamedVectorDefinition, PreviewTransaction, RenewSession, RequestContext, ResourceId,
    ResourceKind, ResourcePath, SecurityAction, SessionLimits, TransactionMutation,
    TransactionState, VectorMemoryTier, VectorSearchMetric, VectorValueKind,
};
use rrd_core::digest;
use rrd_security::{Principal, PrincipalKind, ResourceGrant, SecurityRepository, SecurityState};
use rrd_store::{ControlTransition, StorageEngine};
use serde_json::Value;

const TOKEN_KEY: [u8; 32] = [7; 32];

fn id(value: &str) -> CorrelationId {
    CorrelationId::new(value).unwrap()
}

fn mutation_context(
    idempotency_key: &CorrelationId,
    request_id: &str,
    operation_id: &str,
) -> RequestContext {
    RequestContext {
        request_id: id(request_id),
        operation_id: id(operation_id),
        idempotency_key: Some(idempotency_key.clone()),
        deadline_unix_ms: None,
    }
}

fn instance() -> CanonicalId {
    CanonicalId::new("test-instance").unwrap()
}

fn isolated_engine() -> (tempfile::TempDir, RrdEngine) {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    (root, engine)
}

fn session_request(idle_timeout_ms: u64, max_open_transactions: u16) -> CreateSession {
    CreateSession {
        limits: SessionLimits {
            idle_timeout_ms,
            absolute_timeout_ms: 10_000,
            max_open_transactions,
        },
    }
}

fn begin_request() -> BeginTransaction {
    BeginTransaction {
        scope: CanonicalId::new("claims").unwrap(),
        timeout_ms: 1_000,
    }
}

fn mutation(object: &str) -> TransactionMutation {
    TransactionMutation::AssertClaim {
        subject: CanonicalId::new("document-1").unwrap(),
        predicate: CanonicalId::new("contains").unwrap(),
        object: object.into(),
        valid_from: 1_000,
        tx_time: 1_000,
        producer: CanonicalId::new("test-runner").unwrap(),
        confidence: Some(0.9),
    }
}

fn commit_request(object: &str) -> CommitTransaction {
    let mutations = vec![mutation(object)];
    CommitTransaction {
        operation_sha256: digest::sha256_hex(&serde_json::to_vec(&mutations).unwrap()),
        mutations,
    }
}

mod catalogue;
mod context;
mod data_crud;
mod deployment_conformance;
mod endpoint_configuration;
mod estate_control;
mod flight;
mod function;
mod index_foundation;
mod installation;
mod installation_bootstrap;
mod installation_concurrency;
mod installation_recovery;
mod lifecycle;
mod memory_estate;
mod native_inference;
mod query_admission;
mod query_transaction;
mod recovery;
mod security;
mod session_transactions;
mod storage_cursor;
mod subscription;
mod task;
mod transaction_stamp;
mod vector_index;
mod write_admission;
