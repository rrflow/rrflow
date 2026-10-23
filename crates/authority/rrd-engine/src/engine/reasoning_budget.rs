//! Durable task effort at the existing control/runtime transaction boundary.
//! An acknowledged reservation is never permission to repeat an uncertain call.

use super::clock::{observe_required, ClockSource, HostSystemClock};
use super::*;
use rrd_inference::{EmbeddingCancellation, EmbeddingExecutionControl};

mod persistence;
mod public;

pub use rrd_contract::{ReasoningBudgetStop, TaskReasoningBudget};

/// Attempt identity is stable across a lost response or reconnect. A new
/// identity spends another step; it cannot change this task's original budget.
pub struct TaskEmbeddingAttempt<'a> {
    pub task_id: &'a CanonicalId,
    pub attempt_id: &'a CorrelationId,
    pub request: &'a GenerateEmbeddings,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AttemptOutcome {
    Reserved,
    Accepted,
    Stopped,
}

pub(super) struct Reservation {
    budget: TaskReasoningBudget,
    attempt: AttemptRecord,
    read: ReadStamp,
    control: EmbeddingExecutionControl,
    started: Instant,
}

impl RrdEngine {
    /// Execute one embedding attempt under the task's original durable effort
    /// allowance. This entry point requires ReasoningWrite AND EmbeddingGenerate.
    /// No automatic retries; a recorded attempt returns a receipt digest or
    /// unresolved status rather than re-executing or fabricating lost output.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_task_embeddings(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        attempt: &TaskEmbeddingAttempt<'_>,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<GenerateEmbeddingsResult> {
        self.generate_task_embeddings_with_clock(
            session_id,
            token,
            attempt,
            now,
            request_id,
            operation_id,
            &HostSystemClock,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_task_embeddings_with_clock(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        attempt: &TaskEmbeddingAttempt<'_>,
        now: u64,
        request_id: &str,
        operation_id: &str,
        clock: &dyn ClockSource,
    ) -> Result<GenerateEmbeddingsResult> {
        attempt
            .request
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        for action in [
            SecurityAction::ReasoningWrite,
            SecurityAction::EmbeddingGenerate,
        ] {
            self.authorize(session_id, token, action, now, request_id, operation_id)?;
        }
        let context = EngineControlContext {
            now,
            session_id,
            action: "reasoning-attempt",
            request_id,
            operation_id,
        };
        // Registration and durable Stop share this short lock. No provider
        // computation holds it; the durable budget remains the only authority.
        let key = self.budget_key(&attempt.request.scope, attempt.task_id);
        let reservation = {
            let mut active = self
                .active_task_cancellations
                .lock()
                .map_err(|_| ServiceError::Storage("task cancellation lock poisoned".into()))?;
            let reservation = self.reserve_task_attempt(attempt, clock, &context)?;
            active.insert(
                key.clone(),
                (attempt.attempt_id.clone(), attempt.cancellation.clone()),
            );
            reservation
        };
        let _active = ActiveTaskCancellation {
            engine: self,
            key,
            attempt_id: attempt.attempt_id.clone(),
        };
        let result = self.generate_embeddings_at_with_control(
            attempt.request,
            reservation.read.clone(),
            &reservation.control,
        );
        self.finish_task_attempt(reservation, result, clock, &context)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn inspect_task_reasoning_budget(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        scope: &str,
        task_id: &CanonicalId,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<Option<TaskReasoningBudget>> {
        self.authorize(
            session_id,
            token,
            SecurityAction::ReasoningRead,
            now,
            request_id,
            operation_id,
        )?;
        self.query_scope(scope)?;
        self.load_task_budget(scope, task_id)
            .map(|(_, budget)| budget)
    }

    pub(super) fn reserve_task_attempt(
        &self,
        input: &TaskEmbeddingAttempt<'_>,
        clock: &dyn ClockSource,
        context: &EngineControlContext<'_>,
    ) -> Result<Reservation> {
        let started = Instant::now();
        let at = observe_required(clock, "reasoning-reserve")?.observed_at_unix_ms;
        let scope = self.query_scope(&input.request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        // The following semantic/control commit also checks this runtime cursor:
        // concurrent task closure cannot publish a new reservation.
        self.open_task_at(&read, &input.request.scope, input.task_id)?;
        let (expected, existing) = self.load_task_budget(&input.request.scope, input.task_id)?;
        let request_sha256 = operation_digest(input.request)?;
        let attempt_key = self.attempt_key(&input.request.scope, input.task_id, input.attempt_id);
        if let Some(bytes) = self.storage.control().get(&attempt_key)? {
            let previous: AttemptRecord = serde_json::from_slice(&bytes).map_err(contract_json)?;
            if previous.format_version != 1 || previous.attempt_id != *input.attempt_id {
                return Err(ServiceError::Storage(
                    "reasoning attempt identity is corrupt".into(),
                ));
            }
            if previous.request_sha256 != request_sha256 {
                return Err(ServiceError::IdempotencyConflict);
            }
            return Err(ServiceError::ReasoningAttemptRecorded {
                outcome_sha256: previous.outcome_sha256,
            });
        }
        let limits = self.estate_configuration.reasoning.clone();
        let mut budget = existing.unwrap_or(TaskReasoningBudget {
            format_version: 1,
            instance_id: self.instance.clone(),
            scope: input.request.scope.clone(),
            task_id: input.task_id.clone(),
            configuration_sha256: self.estate_configuration.configuration_sha256.clone(),
            deadline_unix_ms: at
                .checked_add(limits.max_run_elapsed_ms)
                .ok_or(ServiceError::DeadlineExceeded)?,
            limits,
            started_at_unix_ms: at,
            last_observed_at_unix_ms: at,
            charged_elapsed_ms: 0,
            reserved_steps: 0,
            active_attempt: None,
            execution_may_continue: false,
            stopped: None,
            stop_expected_sha256: None,
        });
        if let Some(reason) = budget.stopped {
            return Err(ServiceError::ReasoningBudgetStopped(reason));
        }
        if budget.active_attempt.is_some() {
            return Err(ServiceError::ReasoningBudgetStopped(
                ReasoningBudgetStop::UnresolvedAttempt,
            ));
        }
        let stop = if at < budget.last_observed_at_unix_ms {
            Some(ReasoningBudgetStop::ClockRollback)
        } else if budget.configuration_sha256 != self.estate_configuration.configuration_sha256 {
            Some(ReasoningBudgetStop::ConfigurationChanged)
        } else if input.cancellation.is_cancelled() {
            Some(ReasoningBudgetStop::Cancelled)
        } else if at >= budget.deadline_unix_ms
            || budget.charged_elapsed_ms >= budget.limits.max_run_elapsed_ms
        {
            Some(ReasoningBudgetStop::Deadline)
        } else if budget.reserved_steps >= budget.limits.max_steps {
            Some(ReasoningBudgetStop::StepLimit)
        } else {
            None
        };
        budget.last_observed_at_unix_ms = budget.last_observed_at_unix_ms.max(at);
        if let Some(reason) = stop {
            budget.stopped = Some(reason);
            self.commit_budget(&read, expected, &budget, None, context)?;
            return Err(ServiceError::ReasoningBudgetStopped(reason));
        }
        let remaining = (budget.deadline_unix_ms - at)
            .min(budget.limits.max_run_elapsed_ms - budget.charged_elapsed_ms)
            .min(budget.limits.max_step_elapsed_ms);
        let deadline = started
            .checked_add(Duration::from_millis(remaining))
            .ok_or(ServiceError::DeadlineExceeded)?;
        budget.reserved_steps += 1;
        budget.active_attempt = Some(input.attempt_id.clone());
        budget.execution_may_continue = true;
        let attempt = AttemptRecord {
            format_version: 1,
            attempt_id: input.attempt_id.clone(),
            request_sha256,
            ordinal: budget.reserved_steps,
            read_manifest_sha256: read.manifest_id.clone(),
            read_cursor: read.commit_cursor,
            outcome: AttemptOutcome::Reserved,
            outcome_sha256: None,
            result: None,
        };
        self.commit_budget(&read, expected, &budget, Some((None, &attempt)), context)?;
        Ok(Reservation {
            budget,
            attempt,
            read,
            started,
            control: EmbeddingExecutionControl::new(deadline, input.cancellation.clone()),
        })
    }

    fn finish_task_attempt(
        &self,
        reservation: Reservation,
        mut result: Result<GenerateEmbeddingsResult>,
        clock: &dyn ClockSource,
        context: &EngineControlContext<'_>,
    ) -> Result<GenerateEmbeddingsResult> {
        let Reservation {
            budget: reserved,
            mut attempt,
            control,
            started,
            ..
        } = reservation;
        let expected_attempt = Some(serde_json::to_vec(&attempt).map_err(contract_json)?);
        let at = observe_required(clock, "reasoning-complete")?.observed_at_unix_ms;
        let elapsed = u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        if let Ok(output) = &result {
            if serde_json::to_vec(output).map_err(contract_json)?.len()
                > rrd_contract::MAX_TASK_EMBEDDING_OUTPUT_BYTES
            {
                result = Err(ServiceError::ConfigurationLimit(
                    "task output exceeds the retained output bound".into(),
                ));
            }
        }
        for index in 0..3 {
            let (expected_budget, current) =
                self.load_task_budget(&reserved.scope, &reserved.task_id)?;
            let mut budget = current
                .ok_or_else(|| ServiceError::Storage("reserved budget disappeared".into()))?;
            if budget.active_attempt.as_ref() != Some(&attempt.attempt_id)
                || budget.reserved_steps != reserved.reserved_steps
                || budget.limits != reserved.limits
                || budget.configuration_sha256 != reserved.configuration_sha256
            {
                return Err(ServiceError::StorageConflict(
                    "attempt no longer owns completion".into(),
                ));
            }
            budget.charged_elapsed_ms = reserved.charged_elapsed_ms.saturating_add(elapsed);
            let stop = budget.stopped.or_else(|| {
                if at < budget.last_observed_at_unix_ms {
                    Some(ReasoningBudgetStop::ClockRollback)
                } else if at >= budget.deadline_unix_ms
                    || budget.charged_elapsed_ms >= budget.limits.max_run_elapsed_ms
                    || control.check().is_err()
                {
                    Some(match control.check() {
                        Err(rrd_inference::EmbeddingDispatchError::Cancelled { .. }) => {
                            ReasoningBudgetStop::Cancelled
                        }
                        _ => ReasoningBudgetStop::Deadline,
                    })
                } else if result.is_err() {
                    Some(ReasoningBudgetStop::AttemptFailed)
                } else {
                    None
                }
            });
            if let Some(reason) = stop {
                budget.stopped = Some(reason);
                if result.as_ref().err().is_none_or(ServiceError::retryable) {
                    result = Err(ServiceError::ReasoningBudgetStopped(reason));
                }
            }
            budget.last_observed_at_unix_ms = budget.last_observed_at_unix_ms.max(at);
            budget.active_attempt = None;
            budget.execution_may_continue = match &result {
                Err(ServiceError::InferenceDeadlineExceeded {
                    execution_may_continue,
                })
                | Err(ServiceError::InferenceCancelled {
                    execution_may_continue,
                }) => *execution_may_continue,
                Err(ServiceError::Storage(_)) => true,
                _ => false,
            };
            attempt.outcome = if result.is_ok() {
                AttemptOutcome::Accepted
            } else {
                AttemptOutcome::Stopped
            };
            attempt.outcome_sha256 = result.as_ref().ok().map(operation_digest).transpose()?;
            attempt.result = result.as_ref().ok().cloned();
            let scope = self.query_scope(&budget.scope)?;
            let read = self.storage.runtime().read_stamp(&scope)?;
            match self.commit_budget(
                &read,
                expected_budget,
                &budget,
                Some((expected_attempt.clone(), &attempt)),
                context,
            ) {
                Err(ServiceError::StorageConflict(_)) if index < 2 => continue,
                outcome => {
                    outcome?;
                    break;
                }
            }
        }
        if result.is_ok() {
            control
                .check()
                .map_err(super::inference::embedding_dispatch_error)?;
        }
        result
    }
}

struct ActiveTaskCancellation<'a> {
    engine: &'a RrdEngine,
    key: String,
    attempt_id: CorrelationId,
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
