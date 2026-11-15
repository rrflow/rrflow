//! Engram desired and observed state, authority, leases, backups and recovery.

use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadEngram {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramDesiredPhase {
    Running,
    Stopped,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramObservedPhase {
    Unknown,
    Provisioning,
    Starting,
    Running,
    Stopping,
    Stopped,
    Deleting,
    Absent,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramActivityClass {
    Unknown,
    Active,
    Idle,
    Stale,
    Neglected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramOperationKind {
    Provision,
    Start,
    Stop,
    Restart,
    Upgrade,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramOperationState {
    Pending,
    Leased,
    Prepared,
    Applied,
    Succeeded,
    Failed,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramReceiptBoundary {
    Prepared,
    Applied,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramActivityPolicySnapshot {
    pub idle_after_ms: u64,
    pub stale_after_ms: u64,
    pub neglected_after_ms: u64,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EngramAuthorityResourceKind {
    Organisation,
    Account,
    Entitlement,
    Project,
    Environment,
    Instance,
    Node,
    Shard,
    Job,
    Assignment,
    Health,
    SecretReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramAuthorityStatus {
    Pending,
    Ready,
    Degraded,
    Failed,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramAuthorityReceiptBoundary {
    DesiredAccepted,
    Assigned,
    Applied,
    Observed,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramAuthorityDesiredSnapshot {
    pub generation: u64,
    pub spec_sha256: String,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramAuthorityObservedSnapshot {
    pub generation: u64,
    pub status: EngramAuthorityStatus,
    pub observed_at_unix_ms: u64,
    pub evidence_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramAuthorityResourceSnapshot {
    pub id: CanonicalId,
    pub kind: EngramAuthorityResourceKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_ids: Vec<CanonicalId>,
    pub name: String,
    pub spec_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired: Option<EngramAuthorityDesiredSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed: Option<EngramAuthorityObservedSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_reference_ids: Vec<CanonicalId>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramAuthorityReceiptSnapshot {
    pub id: CanonicalId,
    pub resource_id: CanonicalId,
    pub operation_id: CanonicalId,
    pub lease_epoch: u64,
    pub boundary: EngramAuthorityReceiptBoundary,
    pub at_unix_ms: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramAuthoritySnapshot {
    pub catalogue_sha256: String,
    pub resources: Vec<EngramAuthorityResourceSnapshot>,
    pub receipts: Vec<EngramAuthorityReceiptSnapshot>,
    pub idempotency_binding_count: u32,
}

impl EngramAuthoritySnapshot {
    pub fn empty() -> Self {
        Self {
            catalogue_sha256: sha256_bytes(
                br#"{"format":1,"resources":{},"receipts":{},"idempotency":{}}"#,
            ),
            resources: Vec::new(),
            receipts: Vec::new(),
            idempotency_binding_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramDesiredSnapshot {
    pub generation: u64,
    pub phase: EngramDesiredPhase,
    pub deployment_ref: CanonicalId,
    pub version: String,
    pub configuration_sha256: String,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramObservedSnapshot {
    pub generation: u64,
    pub phase: EngramObservedPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    pub observed_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramActivitySnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_meaningful_runtime_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at_unix_ms: Option<u64>,
    pub class: EngramActivityClass,
    pub evaluated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramInstanceSnapshot {
    pub id: CanonicalId,
    pub desired: EngramDesiredSnapshot,
    pub observed: EngramObservedSnapshot,
    pub activity: EngramActivitySnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramLeaseSnapshot {
    pub owner: CanonicalId,
    pub epoch: u64,
    pub acquired_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramReceiptSnapshot {
    pub boundary: EngramReceiptBoundary,
    pub lease_epoch: u64,
    pub at_unix_ms: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramOperationSnapshot {
    pub id: CanonicalId,
    pub instance_id: CanonicalId,
    pub kind: EngramOperationKind,
    pub desired_generation: u64,
    pub request_sha256: String,
    pub state: EngramOperationState,
    pub attempts: u32,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<EngramLeaseSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<EngramReceiptSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramSnapshot {
    pub format_version: u16,
    pub id: CanonicalId,
    pub revision: u64,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub activity_policy: EngramActivityPolicySnapshot,
    pub authority: EngramAuthoritySnapshot,
    pub instances: Vec<EngramInstanceSnapshot>,
    pub operations: Vec<EngramOperationSnapshot>,
    pub idempotency_binding_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramMutationResult {
    pub engram: EngramSnapshot,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramBackupJobState {
    Pending,
    Leased,
    Prepared,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramBackupReceiptBoundary {
    Prepared,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramBackupReceiptSnapshot {
    pub boundary: EngramBackupReceiptBoundary,
    pub lease_epoch: u64,
    pub at_unix_ms: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramBackupRecoveryPolicySnapshot {
    pub revision: u64,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    pub minimum_recovery_points: u16,
    pub retention_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramBackupJobSnapshot {
    pub id: CanonicalId,
    pub instance_id: CanonicalId,
    pub source_generation: u64,
    pub label: String,
    pub request_sha256: String,
    pub state: EngramBackupJobState,
    pub attempts: u32,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub recovery_policy: EngramBackupRecoveryPolicySnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<EngramLeaseSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<EngramBackupReceiptSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramBackupJobsSnapshot {
    pub engram_id: CanonicalId,
    pub engram_revision: u64,
    pub jobs: Vec<EngramBackupJobSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramBackupMutationResult {
    pub engram: EngramSnapshot,
    pub job: EngramBackupJobSnapshot,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRecoveryPolicySnapshot {
    pub instance_id: CanonicalId,
    pub revision: u64,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    pub minimum_recovery_points: u16,
    pub retention_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRecoveryPointSnapshot {
    pub backup_sha256: String,
    pub instance_id: CanonicalId,
    pub backup_job_id: CanonicalId,
    pub source_generation: u64,
    pub archive_sha256: String,
    pub catalogue_sha256: String,
    pub source_cut_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub policy_revision: u64,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pruned_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngramRetentionPinKindSnapshot {
    Policy,
    ExplicitHold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRetentionPinSnapshot {
    pub id: CanonicalId,
    pub backup_sha256: String,
    pub kind: EngramRetentionPinKindSnapshot,
    pub created_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRestoreEvidenceSnapshot {
    pub restore_id: CanonicalId,
    pub instance_id: CanonicalId,
    pub backup_sha256: String,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub duration_ms: u64,
    pub recovery_point_age_ms: u64,
    pub restored_claim_sequence: u64,
    pub restored_runtime_cursor: u64,
    pub closure_sha256: String,
    pub policy_revision: u64,
    pub rpo_within_objective: bool,
    pub rto_within_objective: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRecoveryPruneIntentSnapshot {
    pub id: CanonicalId,
    pub instance_id: CanonicalId,
    pub based_on_engram_revision: u64,
    pub evaluated_at_unix_ms: u64,
    pub expected_catalogue_sha256: String,
    pub retained_backup_ids: Vec<String>,
    pub prune_candidate_backup_ids: Vec<String>,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRecoverySnapshot {
    pub engram_id: CanonicalId,
    pub engram_revision: u64,
    pub policies: Vec<EngramRecoveryPolicySnapshot>,
    pub recovery_points: Vec<EngramRecoveryPointSnapshot>,
    pub retention_pins: Vec<EngramRetentionPinSnapshot>,
    pub restore_evidence: Vec<EngramRestoreEvidenceSnapshot>,
    pub prune_intents: Vec<EngramRecoveryPruneIntentSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRetentionDecisionSnapshot {
    pub engram_revision: u64,
    pub instance_id: CanonicalId,
    pub evaluated_at_unix_ms: u64,
    pub retained_backup_ids: Vec<String>,
    pub prune_candidate_backup_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRecoveryMutationResult {
    pub recovery: EngramRecoverySnapshot,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRecoveryPruneResult {
    pub recovery: EngramRecoverySnapshot,
    pub decision: EngramRetentionDecisionSnapshot,
    pub catalogue_revision: u64,
    pub catalogue_sha256: String,
    pub pruned_backup_ids: Vec<String>,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramRecoveryRestoreResult {
    pub recovery: EngramRecoverySnapshot,
    pub restore_id: CanonicalId,
    pub backup_sha256: String,
    pub inventory: LogicalArchiveSnapshot,
    pub reopened: bool,
    pub idempotent_replay: bool,
}
