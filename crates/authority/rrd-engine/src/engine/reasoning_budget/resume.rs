use super::*;
use rrd_contract::{ResumeTaskBudget, TaskBudgetView};

impl RrdEngine {
    /// Resume only known-completed stopped work under its original allowance.
    /// This commits no attempt, provider dispatch, lease clearance or output.
    pub fn resume_task_budget(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ResumeTaskBudget,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TaskBudgetView> {
        self.resume_task_budget_with_clock(
            session,
            token,
            request,
            request_id,
            operation_id,
            &HostSystemClock,
        )
    }

    pub(in crate::engine) fn resume_task_budget_with_clock(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ResumeTaskBudget,
        request_id: &str,
        operation_id: &str,
        clock: &dyn ClockSource,
    ) -> Result<TaskBudgetView> {
        request
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let at = observe_required(clock, "reasoning-resume")?.observed_at_unix_ms;
        // Capture authority before authorization, then compare these exact
        // bytes in the budget transaction. Revocation cannot race the commit.
        let (security_key, security_bytes) =
            rrd_security::SecurityRepository::new(&self.storage, self.instance.clone())
                .authority_snapshot()?
                .ok_or(ServiceError::PermissionDenied)?;
        let (session_bytes, _) = self.authorize(
            session,
            token,
            SecurityAction::ReasoningWrite,
            at,
            request_id,
            operation_id,
        )?;
        let read = self
            .storage
            .runtime()
            .read_stamp(&self.query_scope(&request.scope)?)?;
        self.open_task_at(&read, &request.scope, &request.task_id)?;
        let (expected, state) = self.load_task_budget(&request.scope, &request.task_id)?;
        let mut budget = state.ok_or(ServiceError::TaskNotFound)?;
        if operation_digest(&budget)? != request.expected_budget_sha256 {
            return Err(ServiceError::IdempotencyConflict);
        }
        if budget.active_attempt.is_some() || budget.execution_may_continue {
            return Err(ServiceError::ReasoningBudgetStopped(
                ReasoningBudgetStop::UnresolvedAttempt,
            ));
        }
        match budget.stopped {
            Some(
                ReasoningBudgetStop::Cancelled
                | ReasoningBudgetStop::AttemptFailed
                | ReasoningBudgetStop::RepeatedWork,
            ) => {}
            Some(reason) => return Err(ServiceError::ReasoningBudgetStopped(reason)),
            None => return Err(ServiceError::Contract("task budget is not stopped".into())),
        }
        let denied = if at < budget.last_observed_at_unix_ms {
            Some(ReasoningBudgetStop::ClockRollback)
        } else if budget.configuration_sha256 != self.engram_configuration.configuration_sha256 {
            Some(ReasoningBudgetStop::ConfigurationChanged)
        } else if at >= budget.deadline_unix_ms
            || budget.charged_elapsed_ms >= budget.limits.max_run_elapsed_ms
        {
            Some(ReasoningBudgetStop::Deadline)
        } else if budget.reserved_steps >= budget.limits.max_steps {
            Some(ReasoningBudgetStop::StepLimit)
        } else {
            None
        };
        if let Some(reason) = denied {
            return Err(ServiceError::ReasoningBudgetStopped(reason));
        }
        budget.stopped = None;
        // A prior Stop receipt cannot authorize stopping a newly resumed budget.
        // Its event/control audit and all attempts remain retained unchanged.
        budget.stop_expected_sha256 = None;
        budget.last_observed_at_unix_ms = at;
        let admission = TaskCompletionAdmission {
            guards: vec![
                AttemptAuthorityGuard {
                    key: security_key,
                    expected: security_bytes,
                },
                AttemptAuthorityGuard {
                    key: super::super::session::session_key(&self.instance, session),
                    expected: session_bytes,
                },
            ],
            ..Default::default()
        };
        self.commit_budget_controlled(
            &read,
            expected,
            &budget,
            None,
            &admission,
            &EngineControlContext {
                now: at,
                session_id: session,
                action: "reasoning-resume",
                request_id,
                operation_id,
            },
        )?;
        public::view(Some(budget))
    }
}
