//! Recovery limits, policies, points, retention pins, restore evidence, prune intents and requests.

use super::*;

pub const MAX_RECOVERY_POLICIES: usize = 1_024;
pub const MAX_RECOVERY_POINTS: usize = 4_096;
pub const MAX_RECOVERY_PINS: usize = 8_192;
pub const MAX_RESTORE_EVIDENCE: usize = 4_096;
pub const MAX_RECOVERY_IDEMPOTENCY_BINDINGS: usize = 8_192;
pub const MAX_RECOVERY_PRUNE_INTENTS: usize = 128;

pub(super) const MIN_OBJECTIVE_MS: u64 = 1_000;
pub(super) const MAX_OBJECTIVE_MS: u64 = 365 * 24 * 60 * 60 * 1_000;
pub(super) const MAX_RETENTION_MS: u64 = 10 * 365 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoveryPolicy {
    pub instance_id: CanonicalId,
    pub revision: u64,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    pub minimum_recovery_points: u16,
    pub retention_ms: u64,
    pub updated_at: u64,
}

impl EstateRecoveryPolicy {
    pub(crate) fn default_for(instance_id: CanonicalId, at: u64) -> Self {
        Self {
            instance_id,
            revision: 1,
            max_rpo_ms: 24 * 60 * 60 * 1_000,
            max_rto_ms: 60 * 60 * 1_000,
            minimum_recovery_points: 1,
            retention_ms: 7 * 24 * 60 * 60 * 1_000,
            updated_at: at,
        }
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.revision == 0
            || !(MIN_OBJECTIVE_MS..=MAX_OBJECTIVE_MS).contains(&self.max_rpo_ms)
            || !(MIN_OBJECTIVE_MS..=MAX_OBJECTIVE_MS).contains(&self.max_rto_ms)
            || !(1..=1_024).contains(&self.minimum_recovery_points)
            || self.retention_ms < self.max_rpo_ms
            || self.retention_ms > MAX_RETENTION_MS
            || self.updated_at == 0
        {
            return Err(Error::Invalid(format!(
                "recovery policy for {} is invalid",
                self.instance_id
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoveryPoint {
    pub backup_id: String,
    pub instance_id: CanonicalId,
    pub backup_job_id: CanonicalId,
    pub source_generation: u64,
    pub archive_sha256: String,
    pub catalogue_sha256: String,
    pub source_cut_at: u64,
    pub completed_at: u64,
    pub policy_revision: u64,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pruned_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstateRetentionPinKind {
    Policy,
    ExplicitHold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateRetentionPin {
    pub id: CanonicalId,
    pub backup_id: String,
    pub kind: EstateRetentionPinKind,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at: Option<u64>,
}

impl EstateRetentionPin {
    pub fn is_live(&self, at: u64) -> bool {
        self.released_at.is_none() && self.expires_at.is_none_or(|expires| expires > at)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateRestoreEvidence {
    pub restore_id: CanonicalId,
    pub instance_id: CanonicalId,
    pub backup_id: String,
    pub started_at: u64,
    pub completed_at: u64,
    pub duration_ms: u64,
    pub recovery_point_age_ms: u64,
    pub restored_claim_sequence: u64,
    pub restored_runtime_cursor: u64,
    pub closure_sha256: String,
    pub policy_revision: u64,
    pub rpo_within_objective: bool,
    pub rto_within_objective: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryIdempotencyBinding {
    pub operation_id: CanonicalId,
    pub request_sha256: String,
    pub bound_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoveryPruneIntent {
    pub id: CanonicalId,
    pub instance_id: CanonicalId,
    pub based_on_estate_revision: u64,
    pub evaluated_at: u64,
    pub expected_catalogue_sha256: String,
    pub retained_backup_ids: Vec<String>,
    pub prune_candidate_backup_ids: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct SetRecoveryPolicy {
    pub context: MutationContext,
    pub instance_id: CanonicalId,
    pub idempotency_key: String,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    pub minimum_recovery_points: u16,
    pub retention_ms: u64,
}

#[derive(Debug, Clone)]
pub struct PinRecoveryPoint {
    pub context: MutationContext,
    pub idempotency_key: String,
    pub backup_id: String,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ReleaseRecoveryPin {
    pub context: MutationContext,
    pub idempotency_key: String,
    pub pin_id: CanonicalId,
}

#[derive(Debug, Clone)]
pub struct RecordRestoreEvidence {
    pub context: MutationContext,
    pub idempotency_key: String,
    pub restore_id: CanonicalId,
    pub instance_id: CanonicalId,
    pub backup_id: String,
    pub started_at: u64,
    pub completed_at: u64,
    pub restored_claim_sequence: u64,
    pub restored_runtime_cursor: u64,
    pub closure_sha256: String,
}

#[derive(Debug, Clone)]
pub struct PrepareRecoveryPrune {
    pub context: MutationContext,
    pub idempotency_key: String,
    pub instance_id: CanonicalId,
    pub expected_estate_revision: u64,
    pub evaluated_at: u64,
    pub expected_catalogue_sha256: String,
    pub retained_backup_ids: Vec<String>,
    pub prune_candidate_backup_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompleteRecoveryPrune {
    pub context: MutationContext,
    pub idempotency_key: String,
    pub intent_id: CanonicalId,
    pub resulting_catalogue_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryMutationOutcome {
    pub document: EstateDocument,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstateRetentionDecision {
    pub estate_revision: u64,
    pub instance_id: CanonicalId,
    pub evaluated_at: u64,
    pub retained_backup_ids: Vec<String>,
    pub prune_candidate_backup_ids: Vec<String>,
}
