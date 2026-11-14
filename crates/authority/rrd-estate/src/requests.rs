//! Mutation, desired-state, lease, receipt, observation and activity requests, and the repository.

use super::*;

#[derive(Debug, Clone)]
pub struct MutationContext {
    pub at: u64,
    pub actor: String,
    pub request_id: String,
    pub operation_id: CanonicalId,
}

#[derive(Debug, Clone)]
pub struct DesiredTarget {
    pub phase: DesiredPhase,
    pub deployment_ref: CanonicalId,
    pub version: String,
    pub configuration_sha256: String,
}

#[derive(Debug, Clone)]
pub struct SetDesired {
    pub context: MutationContext,
    pub instance_id: CanonicalId,
    pub idempotency_key: String,
    pub target: DesiredTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredOutcome {
    pub document: EstateDocument,
    pub operation: EstateOperation,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateOutcome {
    pub document: EstateDocument,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone)]
pub struct LeaseRequest {
    pub context: MutationContext,
    pub worker: CanonicalId,
    pub lease_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ReceiptRequest {
    pub context: MutationContext,
    pub worker: CanonicalId,
    pub lease_epoch: u64,
    pub boundary: ReceiptBoundary,
    pub evidence_sha256: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ObservationRequest {
    pub context: MutationContext,
    pub worker: CanonicalId,
    pub lease_epoch: u64,
    pub phase: ObservedPhase,
    pub version: Option<String>,
    pub process_id: Option<u32>,
    pub evidence_sha256: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActivityEvidence {
    pub context: MutationContext,
    pub instance_id: CanonicalId,
    pub meaningful_runtime_at: Option<u64>,
    pub heartbeat_at: Option<u64>,
}

pub struct EstateRepository<'a, E: StorageEngine + ?Sized> {
    pub(super) engine: &'a E,
    pub(super) estate_id: CanonicalId,
    pub(super) key: String,
}
