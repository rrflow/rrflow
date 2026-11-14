//! Durable task effort at the existing control/runtime transaction boundary.
//! An acknowledged reservation is never permission to repeat an uncertain call.

mod finish;
mod inspection;
mod reserve;

use super::clock::{observe_required, ClockSource, HostSystemClock};

use super::*;

use rrd_inference::{EmbeddingCancellation, EmbeddingExecutionControl};

mod persistence;

mod public;

mod repetition;

mod resume;

pub use rrd_contract::{ReasoningBudgetStop, TaskReasoningBudget};

/// Attempt identity is stable across a lost response or reconnect. A new
/// identity spends another step; it cannot change this task's original budget.
pub struct TaskEmbeddingAttempt<'a> {
    pub task_id: &'a CanonicalId,
    pub attempt_id: &'a CorrelationId,
    pub request: &'a GenerateEmbeddings,
    pub cancellation: &'a EmbeddingCancellation,
}

/// Engine-internal effect identity for the one task budget. Callers must
/// authorize and derive this identity before reserving; it is not a public
/// capability or permission to dispatch an effect.
pub(super) struct TaskAttemptAdmission<'a> {
    /// Captured before effect-specific identity preparation, never reset by it.
    pub started: Instant,
    pub scope: &'a str,
    pub task_id: &'a CanonicalId,
    pub attempt_id: &'a CorrelationId,
    pub request_sha256: &'a str,
    pub cancellation: &'a EmbeddingCancellation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttemptRecord {
    format_version: u16,
    attempt_id: CorrelationId,
    request_sha256: String,
    ordinal: u64,
    read_manifest_sha256: String,
    read_cursor: u64,
    outcome: AttemptOutcome,
    outcome_sha256: Option<String>,
    result: Option<GenerateEmbeddingsResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    endpoint_proposal: Option<rrd_contract::EndpointSelectionProposal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    worker_output: Option<rrd_contract::TaskWorkerOutput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lfg: Option<rrd_contract::LfgAdvisory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    assessment_output: Option<rrd_contract::TaskAssessmentOutput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    invalidation: Option<rrd_contract::TaskAttemptInvalidation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AttemptOutcome {
    Reserved,
    Accepted,
    Stopped,
    Invalidated,
}

/// Internal typed completion; public callers cannot inject a retained output.
#[derive(Debug, Clone)]
pub(super) enum TaskAttemptOutput {
    Embedding(Box<GenerateEmbeddingsResult>),
    EndpointProposal(Box<rrd_contract::EndpointSelectionProposal>),
    Worker(Box<rrd_contract::TaskWorkerOutput>),
    Assessment(Box<rrd_contract::TaskAssessmentOutput>),
}

impl From<rrd_contract::EndpointSelectionProposal> for TaskAttemptOutput {
    fn from(output: rrd_contract::EndpointSelectionProposal) -> Self {
        Self::EndpointProposal(Box::new(output))
    }
}

impl TaskAttemptOutput {
    fn bytes(&self) -> Result<Vec<u8>> {
        match self {
            Self::Embedding(output) => serde_json::to_vec(output),
            Self::EndpointProposal(output) => serde_json::to_vec(output),
            Self::Worker(output) => serde_json::to_vec(output),
            Self::Assessment(output) => serde_json::to_vec(output),
        }
        .map_err(contract_json)
    }

    fn validate_for(&self, budget: &TaskReasoningBudget, attempt: &AttemptRecord) -> Result<()> {
        match self {
            Self::Worker(output) => {
                output
                    .validate()
                    .map_err(|e| ServiceError::Contract(e.to_string()))?;
                if output.scope != budget.scope
                    || output.task_id != budget.task_id
                    || output.attempt_id != attempt.attempt_id
                    || attempt.lfg.is_some()
                {
                    return Err(ServiceError::Contract(
                        "task worker output identity mismatch".into(),
                    ));
                }
            }
            Self::Assessment(output) => {
                output
                    .validate()
                    .map_err(|e| ServiceError::Contract(e.to_string()))?;
                if output.scope != budget.scope
                    || output.task_id != budget.task_id
                    || output.attempt_id != attempt.attempt_id
                {
                    return Err(ServiceError::Contract(
                        "task assessment output identity mismatch".into(),
                    ));
                }
            }
            Self::Embedding(output) => {
                output
                    .validate()
                    .map_err(|e| ServiceError::Contract(e.to_string()))?;
                if output.scope != budget.scope
                    || self.bytes()?.len() > rrd_contract::MAX_TASK_EMBEDDING_OUTPUT_BYTES
                {
                    return Err(ServiceError::ConfigurationLimit(
                        "task embedding output scope or bound mismatch".into(),
                    ));
                }
            }
            Self::EndpointProposal(output) => {
                output
                    .validate()
                    .map_err(|e| ServiceError::Contract(e.to_string()))?;
                if output.scope != budget.scope
                    || output.task_id != budget.task_id
                    || output.attempt_id != attempt.attempt_id
                    || output.lfg != attempt.lfg
                {
                    return Err(ServiceError::Contract(
                        "task proposal output identity mismatch".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Engine-owned control bytes that must still match at the completion commit.
/// These are compare-only semantic guards: their replacement is the same bytes.
/// They never accept client-supplied keys or modify another authority's value.
#[derive(Clone)]
pub(super) struct AttemptAuthorityGuard {
    pub key: String,
    pub expected: Vec<u8>,
}

/// Effect-specific admission state committed with the existing attempt ledger.
/// Only engine composition supplies these keys; callers cannot inject writes.
pub(super) struct AttemptControlWrite {
    pub key: String,
    pub expected: Option<Vec<u8>>,
    pub replacement: Vec<u8>,
}

#[derive(Default)]
pub(super) struct TaskCompletionAdmission {
    pub guards: Vec<AttemptAuthorityGuard>,
    pub writes: Vec<AttemptControlWrite>,
    pub invalidation: Option<rrd_contract::TaskAttemptInvalidation>,
    pub worker_source: Option<rrd_contract::TaskWorkerSourceProvenance>,
    pub assessment_source: Option<rrd_contract::TaskWorkerSourceProvenance>,
}

pub(super) struct Reservation {
    budget: TaskReasoningBudget,
    attempt: AttemptRecord,
    read: ReadStamp,
    control: EmbeddingExecutionControl,
    started: Instant,
}

impl Reservation {
    pub(super) fn execution_control(&self) -> &EmbeddingExecutionControl {
        &self.control
    }
}

fn validate_attempt_digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ServiceError::Contract(
            "task attempt identity must be a lowercase SHA-256 digest".into(),
        ));
    }
    Ok(())
}

pub(super) struct ActiveTaskCancellation<'a> {
    pub engine: &'a RrdEngine,
    pub key: String,
    pub attempt_id: CorrelationId,
}

impl Drop for ActiveTaskCancellation<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.engine.active_task_cancellations.lock() {
            // Completion may have admitted the next attempt before this guard
            // runs. An old guard must never unregister the next attempt's Stop.
            if active
                .get(&self.key)
                .is_some_and(|(id, _)| id == &self.attempt_id)
            {
                active.remove(&self.key);
            }
        }
    }
}

#[cfg(test)]
mod legacy_attempt_record_tests {
    use super::{AttemptOutcome, AttemptRecord};
    use rrd_contract::CorrelationId;

    #[test]
    fn prior_v1_attempt_record_without_assessment_output_loads_and_keeps_its_wire_shape() {
        let legacy = serde_json::json!({
            "format_version": 1,
            "attempt_id": "legacy-attempt-v1",
            "request_sha256": "a".repeat(64),
            "ordinal": 3,
            "read_manifest_sha256": "b".repeat(64),
            "read_cursor": 17,
            "outcome": "stopped",
            "outcome_sha256": null,
            "result": null
        });
        let record: AttemptRecord = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(
            record.attempt_id,
            CorrelationId::new("legacy-attempt-v1").unwrap()
        );
        assert_eq!(record.outcome, AttemptOutcome::Stopped);
        assert!(record.assessment_output.is_none());
        let roundtrip = serde_json::to_value(record).unwrap();
        assert_eq!(roundtrip, legacy);
        assert!(!roundtrip
            .as_object()
            .unwrap()
            .contains_key("assessment_output"));
    }
}
