use crate::{
    invalid, validate_sha256, validate_vector_scope, CanonicalId, CorrelationId,
    GenerateEmbeddings, GenerateEmbeddingsResult, ReasoningLimits, Result,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningBudgetStop {
    StepLimit,
    Deadline,
    Cancelled,
    ClockRollback,
    ConfigurationChanged,
    UnresolvedAttempt,
    AttemptFailed,
}

/// Public observation of engine-owned control state. Task lifecycle remains
/// owned by the task record; clients cannot write this budget directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskReasoningBudget {
    pub format_version: u16,
    pub instance_id: CanonicalId,
    pub scope: String,
    pub task_id: CanonicalId,
    pub configuration_sha256: String,
    pub limits: ReasoningLimits,
    pub started_at_unix_ms: u64,
    pub deadline_unix_ms: u64,
    pub last_observed_at_unix_ms: u64,
    pub charged_elapsed_ms: u64,
    pub reserved_steps: u64,
    pub active_attempt: Option<CorrelationId>,
    /// Conservative persisted observation; only explicit future reconciliation
    /// may clear uncertainty about non-cooperative external work.
    pub execution_may_continue: bool,
    pub stopped: Option<ReasoningBudgetStop>,
    pub stop_expected_sha256: Option<String>,
}

pub const MAX_TASK_EMBEDDING_OUTPUT_BYTES: usize = 512 * 1024;
pub const MAX_TASK_EMBEDDING_VALUES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigureTaskBudget {
    pub scope: String,
    pub task_id: CanonicalId,
    pub limits: ReasoningLimits,
}
impl ConfigureTaskBudget {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        self.limits.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadTaskBudget {
    pub scope: String,
    pub task_id: CanonicalId,
}
impl ReadTaskBudget {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StopTaskBudget {
    pub scope: String,
    pub task_id: CanonicalId,
    pub expected_budget_sha256: String,
}
impl StopTaskBudget {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        validate_sha256(&self.expected_budget_sha256, "expected budget digest")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskBudgetView {
    pub budget: Option<TaskReasoningBudget>,
    pub budget_sha256: Option<String>,
}
impl TaskBudgetView {
    pub fn validate(&self) -> Result<()> {
        match (&self.budget, &self.budget_sha256) {
            (None, None) => Ok(()),
            (Some(budget), Some(digest)) => {
                budget.limits.validate()?;
                validate_vector_scope(&budget.scope)?;
                validate_sha256(digest, "budget digest")?;
                validate_sha256(&budget.configuration_sha256, "configuration digest")?;
                if let Some(expected) = &budget.stop_expected_sha256 {
                    validate_sha256(expected, "Stop expected digest")?;
                    if budget.stopped != Some(ReasoningBudgetStop::Cancelled) {
                        return invalid("a Stop receipt requires cancelled state");
                    }
                }
                if budget.format_version != 1
                    || budget.started_at_unix_ms == 0
                    || budget
                        .started_at_unix_ms
                        .checked_add(budget.limits.max_run_elapsed_ms)
                        != Some(budget.deadline_unix_ms)
                    || budget.reserved_steps > budget.limits.max_steps
                    || budget.last_observed_at_unix_ms < budget.started_at_unix_ms
                    || (budget.active_attempt.is_some() && budget.reserved_steps == 0)
                {
                    return invalid("task budget bounds are inconsistent");
                }
                let actual = crate::sha256_bytes(
                    &serde_json::to_vec(budget).map_err(|e| crate::ContractError(e.to_string()))?,
                );
                if actual != *digest {
                    return invalid("task budget digest mismatch");
                }
                Ok(())
            }
            _ => invalid("budget and digest must be present together"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecuteTaskEmbedding {
    pub task_id: CanonicalId,
    pub attempt_id: CorrelationId,
    pub request: GenerateEmbeddings,
}
impl ExecuteTaskEmbedding {
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        if self.request.inputs.len() > 16
            || self
                .request
                .inputs
                .iter()
                .map(|i| i.bytes.len())
                .sum::<usize>()
                > 65_536
        {
            return invalid("public task embedding input exceeds 16 inputs or 64 KiB");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadTaskAttempt {
    pub scope: String,
    pub task_id: CanonicalId,
    pub attempt_id: CorrelationId,
}
impl ReadTaskAttempt {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskAttemptStatus {
    Reserved,
    Accepted,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskAttemptSnapshot {
    pub attempt_id: CorrelationId,
    pub ordinal: u64,
    pub status: TaskAttemptStatus,
    pub outcome_sha256: Option<String>,
    pub result: Option<GenerateEmbeddingsResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskAttemptView {
    pub attempt: Option<TaskAttemptSnapshot>,
}

impl TaskAttemptView {
    pub fn validate(&self) -> Result<()> {
        if let Some(attempt) = &self.attempt {
            if attempt.ordinal == 0 {
                return invalid("attempt ordinal must be nonzero");
            }
            if let Some(digest) = &attempt.outcome_sha256 {
                validate_sha256(digest, "outcome digest")?;
            }
            if (attempt.status == TaskAttemptStatus::Accepted) != attempt.outcome_sha256.is_some() {
                return invalid("only accepted attempts have an outcome digest");
            }
            if let Some(result) = &attempt.result {
                if attempt.status != TaskAttemptStatus::Accepted {
                    return invalid("only an accepted attempt can retain output");
                }
                result.validate()?;
                let bytes =
                    serde_json::to_vec(result).map_err(|e| crate::ContractError(e.to_string()))?;
                if bytes.len() > MAX_TASK_EMBEDDING_OUTPUT_BYTES
                    || attempt.outcome_sha256.as_deref()
                        != Some(crate::sha256_bytes(&bytes).as_str())
                {
                    return invalid("retained output size or digest mismatch");
                }
            }
        }
        Ok(())
    }
}

/// Explicit, process-local activation of the bundled deterministic reference
/// adapter. This is not model training, an LLM, or an external model fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActivateBuiltinEmbedding {
    pub scope: String,
    pub dimensions: u32,
    pub seed: u32,
}
impl ActivateBuiltinEmbedding {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if !(8..=1024).contains(&self.dimensions) {
            return invalid("builtin dimensions must be in 8..=1024");
        }
        Ok(())
    }
}
