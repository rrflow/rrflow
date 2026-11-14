//! Resuming and cancelling attunement.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResumeAttunement {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub plan_sha256: String,
    pub expected_job_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checkpoint_sha256: Option<String>,
    pub resume_from_phase: AttunementPhase,
    pub requested_at_unix_ms: u64,
}

impl ResumeAttunement {
    /// Validate fields that do not depend on the current durable job state.
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)?;
        validate_sha256(&self.plan_sha256, "attunement command plan_sha256")
    }

    pub fn validate_for(&self, job: &AttunementJob, plan: &AttunementPlan) -> Result<()> {
        self.validate()?;
        job.validate(plan)?;
        if !matches!(
            job.state,
            AttunementJobState::Paused | AttunementJobState::Failed
        ) || (job.state == AttunementJobState::Failed
            && !job
                .failure
                .as_ref()
                .is_some_and(|failure| failure.retryable))
        {
            return invalid("only a paused or retryable failed attunement job may resume");
        }
        validate_job_command_binding(
            &self.job_id,
            &self.plan_sha256,
            self.expected_job_revision,
            self.last_checkpoint_sha256.as_deref(),
            self.requested_at_unix_ms,
            job,
        )?;
        if job.next_phase(plan) != Some(self.resume_from_phase) {
            return invalid("attunement resume phase does not match the next canonical phase");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelAttunement {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub plan_sha256: String,
    pub expected_job_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checkpoint_sha256: Option<String>,
    pub reason: String,
    pub requested_at_unix_ms: u64,
}

/// The durable cancellation request, not an assertion that a worker has stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementCancelResult {
    pub status: AttunementStatus,
    pub idempotent_replay: bool,
}

impl AttunementCancelResult {
    pub fn validate_for(&self, request: &CancelAttunement) -> Result<()> {
        validate_attunement_contract_version(self.status.contract_version)?;
        validate_attunement_contract_version(self.status.job.contract_version)?;
        let job = &self.status.job;
        if job.id != request.job_id
            || job.plan_sha256 != request.plan_sha256
            || Some(job.revision) != request.expected_job_revision.checked_add(1)
            || job.state != AttunementJobState::CancelRequested
            || self.status.observed_at_unix_ms < job.updated_at_unix_ms
        {
            return invalid("attunement cancellation result does not match the request");
        }
        Ok(())
    }
}

impl CancelAttunement {
    pub fn validate_for(&self, job: &AttunementJob, plan: &AttunementPlan) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)?;
        job.validate(plan)?;
        if !matches!(
            job.state,
            AttunementJobState::Pending
                | AttunementJobState::Running
                | AttunementJobState::Paused
                | AttunementJobState::Failed
        ) {
            return invalid("attunement job is not in a cancellable state");
        }
        validate_bounded_text(
            &self.reason,
            "attunement cancellation reason",
            MAX_ATTUNEMENT_CANCEL_REASON_BYTES,
        )?;
        validate_job_command_binding(
            &self.job_id,
            &self.plan_sha256,
            self.expected_job_revision,
            self.last_checkpoint_sha256.as_deref(),
            self.requested_at_unix_ms,
            job,
        )
    }
}
