//! Attunement jobs, their state, lease and failure.

use super::*;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AttunementJobState {
    Pending,
    Running,
    Paused,
    CancelRequested,
    Cancelled,
    Failed,
    Succeeded,
}

impl AttunementJobState {
    pub const fn permits(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Running | Self::CancelRequested)
                | (
                    Self::Running,
                    Self::Running
                        | Self::Paused
                        | Self::CancelRequested
                        | Self::Failed
                        | Self::Succeeded
                )
                | (Self::Paused, Self::Running | Self::CancelRequested)
                | (Self::Failed, Self::Running | Self::CancelRequested)
                | (Self::CancelRequested, Self::Cancelled)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementLease {
    pub id: CanonicalId,
    pub holder: CanonicalId,
    pub generation: u64,
    pub acquired_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

impl AttunementLease {
    fn validate(&self) -> Result<()> {
        if self.generation == 0
            || self.acquired_at_unix_ms == 0
            || self.expires_at_unix_ms <= self.acquired_at_unix_ms
        {
            return invalid("attunement lease generation or time interval is invalid");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementFailure {
    pub phase: AttunementPhase,
    pub code: CanonicalId,
    pub message: String,
    pub retryable: bool,
    pub evidence_sha256: String,
    pub observed_at_unix_ms: u64,
}

impl AttunementFailure {
    fn validate(&self) -> Result<()> {
        validate_bounded_text(
            &self.message,
            "attunement failure message",
            MAX_ATTUNEMENT_FAILURE_BYTES,
        )?;
        validate_sha256(&self.evidence_sha256, "attunement failure evidence_sha256")?;
        if self.observed_at_unix_ms == 0 {
            return invalid("attunement failure observation time must be greater than zero");
        }
        Ok(())
    }
}

/// The authoritative durable job snapshot projected by `RrdEngine`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementJob {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub id: CanonicalId,
    pub plan_id: CanonicalId,
    pub plan_sha256: String,
    pub revision: u64,
    pub state: AttunementJobState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<AttunementPhase>,
    #[serde(default)]
    pub checkpoints: Vec<AttunementPhaseCheckpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<AttunementLease>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<AttunementFailure>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

impl AttunementJob {
    pub fn validate(&self, plan: &AttunementPlan) -> Result<()> {
        plan.validate()?;
        validate_attunement_contract_version(self.contract_version)?;
        if self.plan_id != plan.id || self.plan_sha256 != plan.plan_sha256 {
            return invalid("attunement job does not bind its plan identity and digest");
        }
        if self.revision == 0 {
            return invalid("attunement job revision must be greater than zero");
        }
        if self.created_at_unix_ms == 0 || self.updated_at_unix_ms < self.created_at_unix_ms {
            return invalid("attunement job timestamps are invalid");
        }
        if self.checkpoints.len() > plan.phases.len() {
            return invalid("attunement job has more checkpoints than planned steps");
        }
        let mut previous = None;
        for (checkpoint, planned) in self.checkpoints.iter().zip(&plan.phases) {
            if checkpoint.phase != planned.phase || checkpoint.sequence != planned.sequence {
                return invalid("attunement job checkpoints are not a plan prefix");
            }
            checkpoint.validate_for(plan, &self.id, previous)?;
            if checkpoint.committed_at_unix_ms < self.created_at_unix_ms
                || checkpoint.committed_at_unix_ms > self.updated_at_unix_ms
            {
                return invalid("attunement checkpoint lies outside the job time interval");
            }
            previous = Some(checkpoint);
        }

        let next_phase = self.next_phase(plan);
        match self.state {
            AttunementJobState::Pending => {
                if self.revision != 1
                    || !self.checkpoints.is_empty()
                    || self.current_phase.is_some()
                    || self.lease.is_some()
                    || self.failure.is_some()
                {
                    return invalid("a pending attunement job cannot contain execution state");
                }
            }
            AttunementJobState::Running => {
                if self.current_phase != next_phase || next_phase.is_none() || self.lease.is_none()
                {
                    return invalid("a running attunement job must lease its next canonical phase");
                }
                if self.failure.is_some() {
                    return invalid("a running attunement job cannot retain a failure");
                }
            }
            AttunementJobState::Paused => {
                if self.current_phase != next_phase
                    || next_phase.is_none()
                    || self.lease.is_some()
                    || self.failure.is_some()
                {
                    return invalid(
                        "a paused attunement job must identify its unleased next phase",
                    );
                }
            }
            AttunementJobState::CancelRequested => {
                if self.current_phase != next_phase
                    || next_phase.is_none()
                    || self.failure.is_some()
                {
                    return invalid("a cancellation request must identify an incomplete phase");
                }
            }
            AttunementJobState::Cancelled => {
                if self.current_phase.is_some() || self.lease.is_some() || self.failure.is_some() {
                    return invalid("a cancelled attunement job must be terminal and unleased");
                }
            }
            AttunementJobState::Failed => {
                let Some(failure) = &self.failure else {
                    return invalid("a failed attunement job must carry failure evidence");
                };
                failure.validate()?;
                if self.current_phase != next_phase
                    || next_phase != Some(failure.phase)
                    || self.lease.is_some()
                    || failure.observed_at_unix_ms > self.updated_at_unix_ms
                {
                    return invalid("attunement failure is not bound to the next phase");
                }
            }
            AttunementJobState::Succeeded => {
                if self.checkpoints.len() != plan.phases.len()
                    || self.current_phase.is_some()
                    || self.lease.is_some()
                    || self.failure.is_some()
                {
                    return invalid("a succeeded attunement job must checkpoint every phase");
                }
            }
        }
        if let Some(lease) = &self.lease {
            lease.validate()?;
            if lease.acquired_at_unix_ms < self.created_at_unix_ms {
                return invalid("attunement lease predates its job");
            }
        }
        if self
            .failure
            .as_ref()
            .is_some_and(|failure| failure.observed_at_unix_ms < self.created_at_unix_ms)
        {
            return invalid("attunement failure predates its job");
        }
        Ok(())
    }

    pub fn validate_transition(&self, next: &Self, plan: &AttunementPlan) -> Result<()> {
        self.validate(plan)?;
        next.validate(plan)?;
        if self.id != next.id
            || self.plan_id != next.plan_id
            || self.plan_sha256 != next.plan_sha256
            || self.created_at_unix_ms != next.created_at_unix_ms
        {
            return invalid("attunement transition changed immutable job identity");
        }
        if next.revision
            != self
                .revision
                .checked_add(1)
                .ok_or_else(|| crate::ContractError("attunement job revision overflowed".into()))?
            || next.updated_at_unix_ms <= self.updated_at_unix_ms
        {
            return invalid("attunement transition must advance revision and update time once");
        }
        if !self.state.permits(next.state) {
            return invalid("attunement job state transition is not permitted");
        }
        if next.checkpoints.len() < self.checkpoints.len()
            || next.checkpoints.len() > self.checkpoints.len() + 1
            || !next.checkpoints.starts_with(&self.checkpoints)
        {
            return invalid("attunement transition removed, changed, or skipped checkpoints");
        }
        let appended_checkpoint = next.checkpoints.len() == self.checkpoints.len() + 1;
        if appended_checkpoint
            && (self.state != AttunementJobState::Running
                || !matches!(
                    next.state,
                    AttunementJobState::Running | AttunementJobState::Succeeded
                ))
        {
            return invalid("only a running worker may append one committed checkpoint");
        }
        if next.state == AttunementJobState::Succeeded && !appended_checkpoint {
            return invalid("attunement success must append the final selected checkpoint");
        }
        if self.state == AttunementJobState::Running
            && next.state == AttunementJobState::Running
            && !appended_checkpoint
        {
            let lease_advanced =
                self.lease
                    .as_ref()
                    .zip(next.lease.as_ref())
                    .is_some_and(|(before, after)| {
                        before.id == after.id
                            && before.holder == after.holder
                            && after.generation > before.generation
                            && after.expires_at_unix_ms > before.expires_at_unix_ms
                    });
            if !lease_advanced {
                return invalid(
                    "a running self-transition must checkpoint work or renew its lease",
                );
            }
        }
        if self.state == AttunementJobState::Failed
            && next.state == AttunementJobState::Running
            && !self
                .failure
                .as_ref()
                .is_some_and(|failure| failure.retryable)
        {
            return invalid("a non-retryable attunement failure cannot resume");
        }
        Ok(())
    }

    pub fn next_phase(&self, plan: &AttunementPlan) -> Option<AttunementPhase> {
        if self.plan_id != plan.id
            || self.plan_sha256 != plan.plan_sha256
            || plan.validate().is_err()
        {
            return None;
        }
        plan.phases
            .get(self.checkpoints.len())
            .map(|step| step.phase)
    }

    pub(super) fn last_checkpoint_sha256(&self) -> Option<&str> {
        self.checkpoints
            .last()
            .map(|checkpoint| checkpoint.checkpoint_sha256.as_str())
    }
}
