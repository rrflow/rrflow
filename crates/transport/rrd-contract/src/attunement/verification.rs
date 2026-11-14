//! Attunement verification checks and results.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttunementVerificationStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementVerificationCheck {
    pub id: CanonicalId,
    pub passed: bool,
    pub evidence_sha256: String,
}

impl AttunementVerificationCheck {
    fn validate(&self) -> Result<()> {
        validate_sha256(
            &self.evidence_sha256,
            "attunement verification evidence_sha256",
        )
    }
}

/// Evidence envelope bound to the authoritative job snapshot. A successful
/// envelope requires a succeeded job and the final `verify` checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementVerification {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub plan_sha256: String,
    pub job_revision: u64,
    pub last_checkpoint_sha256: String,
    pub runtime_manifest_sha256: String,
    pub status: AttunementVerificationStatus,
    pub checks: Vec<AttunementVerificationCheck>,
    pub verified_at_unix_ms: u64,
    pub verification_sha256: String,
}

impl AttunementVerification {
    pub fn validate_for(&self, job: &AttunementJob, plan: &AttunementPlan) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)?;
        job.validate(plan)?;
        if self.job_id != job.id
            || self.plan_sha256 != job.plan_sha256
            || self.job_revision != job.revision
            || Some(self.last_checkpoint_sha256.as_str()) != job.last_checkpoint_sha256()
        {
            return invalid("attunement verification does not bind the job checkpoint");
        }
        let Some(last_checkpoint) = job.checkpoints.last() else {
            return invalid("attunement verification requires a committed checkpoint");
        };
        validate_sha256(
            &self.runtime_manifest_sha256,
            "attunement verification runtime_manifest_sha256",
        )?;
        if self.runtime_manifest_sha256 != last_checkpoint.coordinates.runtime_manifest_sha256 {
            return invalid("attunement verification runtime manifest differs from its checkpoint");
        }
        if self.checks.is_empty() || self.checks.len() > MAX_ATTUNEMENT_VERIFICATION_CHECKS {
            return invalid("attunement verification check count is outside its bound");
        }
        let mut ids = BTreeSet::new();
        for check in &self.checks {
            check.validate()?;
            if !ids.insert(&check.id) {
                return invalid("attunement verification check ids must be unique");
            }
        }
        match self.status {
            AttunementVerificationStatus::Passed
                if job.state == AttunementJobState::Succeeded
                    && last_checkpoint.phase == AttunementPhase::Verify
                    && self.checks.iter().all(|check| check.passed) => {}
            AttunementVerificationStatus::Failed
                if job.state == AttunementJobState::Failed
                    && job.current_phase == Some(AttunementPhase::Verify)
                    && self.checks.iter().any(|check| !check.passed) => {}
            _ => return invalid("attunement verification status differs from job and checks"),
        }
        if self.verified_at_unix_ms < job.updated_at_unix_ms {
            return invalid("attunement verification predates its job snapshot");
        }
        validate_sha256(&self.verification_sha256, "attunement verification_sha256")?;
        if self.verification_sha256 != attunement_verification_sha256(self)? {
            return invalid("attunement verification digest does not match its content");
        }
        Ok(())
    }
}
