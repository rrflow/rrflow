//! Finishing task attempts and settling their reservations.

use super::*;

impl RrdEngine {
    pub(super) fn finish_task_attempt(
        &self,
        reservation: Reservation,
        result: Result<GenerateEmbeddingsResult>,
        clock: &dyn ClockSource,
        context: &EngineControlContext<'_>,
    ) -> Result<GenerateEmbeddingsResult> {
        match self.finish_admitted_task_attempt(
            reservation,
            result.map(|output| TaskAttemptOutput::Embedding(Box::new(output))),
            |_| Ok(Vec::new()),
            clock,
            context,
        )? {
            TaskAttemptOutput::Embedding(output) => Ok(*output),
            TaskAttemptOutput::EndpointProposal(_)
            | TaskAttemptOutput::Worker(_)
            | TaskAttemptOutput::Assessment(_) => {
                Err(ServiceError::Storage("unexpected task output kind".into()))
            }
        }
    }

    pub(in super::super) fn finish_admitted_task_attempt(
        &self,
        reservation: Reservation,
        result: Result<TaskAttemptOutput>,
        mut admit_completion: impl FnMut(&ReadStamp) -> Result<Vec<AttemptAuthorityGuard>>,
        clock: &dyn ClockSource,
        context: &EngineControlContext<'_>,
    ) -> Result<TaskAttemptOutput> {
        self.finish_controlled_task_attempt(
            reservation,
            result,
            |read| match admit_completion(read) {
                Ok(guards) => Ok(TaskCompletionAdmission {
                    guards,
                    ..Default::default()
                }),
                Err(ServiceError::TaskAttemptInvalidated(reason)) => Ok(TaskCompletionAdmission {
                    invalidation: Some(reason),
                    ..Default::default()
                }),
                Err(error) => Err(error),
            },
            clock,
            context,
        )
    }

    pub(in super::super) fn finish_controlled_task_attempt(
        &self,
        reservation: Reservation,
        mut result: Result<TaskAttemptOutput>,
        mut admit_completion: impl FnMut(&ReadStamp) -> Result<TaskCompletionAdmission>,
        clock: &dyn ClockSource,
        context: &EngineControlContext<'_>,
    ) -> Result<TaskAttemptOutput> {
        let Reservation {
            budget: reserved,
            mut attempt,
            control,
            started,
            ..
        } = reservation;
        let expected_attempt = Some(serde_json::to_vec(&attempt).map_err(contract_json)?);
        if let Ok(output) = &result {
            if let Err(error) = output.validate_for(&reserved, &attempt) {
                result = Err(error);
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
            let scope = self.query_scope(&budget.scope)?;
            let read = self.storage.runtime().read_stamp(&scope)?;
            // Cleanup also runs for known failures. If admission cannot be read,
            // leave the reservation unresolved rather than releasing capacity.
            let admission = admit_completion(&read)?;
            if let Some(source) = admission.worker_source.clone() {
                if let Ok(output) = &mut result {
                    match output {
                        TaskAttemptOutput::Worker(output) => output.source = Some(source),
                        _ => {
                            result = Err(ServiceError::Storage(
                                "worker source admission returned for a non-worker attempt".into(),
                            ));
                        }
                    }
                }
            }
            if let Some(source) = admission.assessment_source.clone() {
                if let Ok(output) = &mut result {
                    match output {
                        TaskAttemptOutput::Assessment(output) => output.source = source,
                        _ => {
                            result = Err(ServiceError::Storage(
                                "assessment source admission returned for a non-assessment attempt"
                                    .into(),
                            ));
                        }
                    }
                }
            }
            if let Ok(output) = &result {
                if let Err(error) = output.validate_for(&reserved, &attempt) {
                    result = Err(error);
                }
            }
            if result.is_ok() {
                if let Some(reason) = admission.invalidation {
                    result = Err(ServiceError::TaskAttemptInvalidated(reason));
                }
            }
            // Reobserve after admission and on every conflict retry. A slow
            // freshness check cannot extend the original deadline/effort.
            let at = observe_required(clock, "reasoning-complete")?.observed_at_unix_ms;
            let elapsed = u64::try_from(started.elapsed().as_millis())
                .unwrap_or(u64::MAX)
                .saturating_add(1);
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
                } else if result.is_err()
                    && !matches!(result, Err(ServiceError::TaskAttemptInvalidated(_)))
                {
                    Some(ReasoningBudgetStop::AttemptFailed)
                } else {
                    None
                }
            });
            if let Some(reason) = stop {
                budget.stopped = Some(reason);
                if result.as_ref().err().is_none_or(ServiceError::retryable)
                    || matches!(result, Err(ServiceError::TaskAttemptInvalidated(_)))
                {
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
                })
                | Err(ServiceError::EndpointCallFailed {
                    execution_may_continue,
                    ..
                }) => *execution_may_continue,
                Err(ServiceError::Storage(_)) => true,
                _ => false,
            };
            attempt.invalidation = match &result {
                Err(ServiceError::TaskAttemptInvalidated(reason)) => Some(*reason),
                _ => None,
            };
            attempt.outcome = if result.is_ok() {
                AttemptOutcome::Accepted
            } else if attempt.invalidation.is_some() {
                AttemptOutcome::Invalidated
            } else {
                AttemptOutcome::Stopped
            };
            attempt.outcome_sha256 = result
                .as_ref()
                .ok()
                .map(|output| output.bytes().map(|bytes| digest::sha256_hex(&bytes)))
                .transpose()?;
            attempt.result = match &result {
                Ok(TaskAttemptOutput::Embedding(output)) => Some((**output).clone()),
                _ => None,
            };
            attempt.endpoint_proposal = match &result {
                Ok(TaskAttemptOutput::EndpointProposal(output)) => Some((**output).clone()),
                _ => None,
            };
            attempt.worker_output = match &result {
                Ok(TaskAttemptOutput::Worker(output)) => Some((**output).clone()),
                _ => None,
            };
            attempt.assessment_output = match &result {
                Ok(TaskAttemptOutput::Assessment(output)) => Some((**output).clone()),
                _ => None,
            };
            match self.commit_budget_controlled(
                &read,
                expected_budget,
                &budget,
                Some((expected_attempt.clone(), &attempt)),
                &admission,
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
                .map_err(super::super::inference::embedding_dispatch_error)?;
        }
        result
    }
}
