//! Reading and controlling attunement: status, receipts, start, step, pause and recovery.

use super::*;

/// Read response for an engine-owned job. Events may point at this snapshot;
/// they never replace it as lifecycle authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementStatus {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub observed_at_unix_ms: u64,
    pub job: AttunementJob,
}

impl AttunementStatus {
    pub fn validate(&self, plan: &AttunementPlan) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)?;
        self.job.validate(plan)?;
        if self.observed_at_unix_ms < self.job.updated_at_unix_ms {
            return invalid("attunement status observation predates the job snapshot");
        }
        Ok(())
    }
}

/// Read one engine-owned installed job by its durable identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadAttunement {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
}

impl ReadAttunement {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)
    }
}

/// This operation does not inspect the project's current source tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttunementSourceVerification {
    NotChecked,
}

/// The exact stored plan and job at one authorized engine read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementReadResult {
    pub plan: AttunementPlan,
    pub status: AttunementStatus,
    pub source_verification: AttunementSourceVerification,
}

impl AttunementReadResult {
    pub fn validate(&self) -> Result<()> {
        self.plan.validate()?;
        self.status.validate(&self.plan)
    }
}

/// Read the authenticated principal's receipt without replaying its command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadAttunementReceipt {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub idempotency_key: CorrelationId,
}

impl ReadAttunementReceipt {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)
    }
}

/// Original committed outcome and the immutable plan it references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementReceipt {
    pub principal_id: CanonicalId,
    pub operation_sha256: String,
    pub plan: AttunementPlan,
    pub status: AttunementStatus,
    pub source_verification: AttunementSourceVerification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementReceiptReadResult {
    pub receipt: Option<AttunementReceipt>,
}

/// Start the engine-owned job against its exact immutable plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StartAttunement {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub plan_sha256: String,
    pub expected_revision: u64,
    pub lease_ms: u64,
}

impl StartAttunement {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)?;
        validate_sha256(&self.plan_sha256, "attunement plan digest")
    }
}

/// Advance the job whose plan is already bound to this revision and lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StepAttunement {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub expected_revision: u64,
    pub lease_generation: u64,
}

impl StepAttunement {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PauseAttunement {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub expected_revision: u64,
    pub lease_generation: u64,
}

impl PauseAttunement {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecoverExpiredAttunement {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub expected_revision: u64,
}

impl RecoverExpiredAttunement {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResumeAttunementWithLease {
    pub request: ResumeAttunement,
    pub lease_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementControlResponse {
    pub status: AttunementStatus,
    pub idempotent_replay: bool,
}

impl AttunementControlResponse {
    pub fn validate_for(
        &self,
        job_id: &CanonicalId,
        previous_revision: u64,
        plan: &AttunementPlan,
    ) -> Result<()> {
        self.status.validate(plan)?;
        if self.status.job.id != *job_id
            || previous_revision.checked_add(1) != Some(self.status.job.revision)
        {
            return invalid(
                "attunement control response does not bind the requested job and revision",
            );
        }
        Ok(())
    }
}
