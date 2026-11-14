//! Reserving task attempts within the reasoning budget.

use super::*;

impl RrdEngine {
    pub(in super::super) fn reserve_task_attempt(
        &self,
        input: &TaskEmbeddingAttempt<'_>,
        clock: &dyn ClockSource,
        context: &EngineControlContext<'_>,
    ) -> Result<Reservation> {
        let started = Instant::now();
        let request_sha256 = operation_digest(input.request)?;
        self.reserve_admitted_task_attempt(
            &TaskAttemptAdmission {
                started,
                scope: &input.request.scope,
                task_id: input.task_id,
                attempt_id: input.attempt_id,
                request_sha256: &request_sha256,
                cancellation: input.cancellation,
            },
            || self.embedding_repetition_identity(input.request),
            clock,
            context,
        )
    }

    /// Shared reservation, with deferred effect-specific equivalence. Replay,
    /// prior stops and exhausted budgets must not resolve an adapter again.
    pub(in super::super) fn reserve_admitted_task_attempt(
        &self,
        input: &TaskAttemptAdmission<'_>,
        repetition_identity: impl FnOnce() -> Result<String>,
        clock: &dyn ClockSource,
        context: &EngineControlContext<'_>,
    ) -> Result<Reservation> {
        self.reserve_controlled_task_attempt(
            input,
            repetition_identity,
            &TaskCompletionAdmission::default(),
            clock,
            context,
        )
    }

    pub(in super::super) fn reserve_controlled_task_attempt(
        &self,
        input: &TaskAttemptAdmission<'_>,
        repetition_identity: impl FnOnce() -> Result<String>,
        admission: &TaskCompletionAdmission,
        clock: &dyn ClockSource,
        context: &EngineControlContext<'_>,
    ) -> Result<Reservation> {
        if let Some(reason) = admission.invalidation {
            return Err(ServiceError::TaskAttemptInvalidated(reason));
        }
        let started = input.started;
        validate_attempt_digest(input.request_sha256)?;
        let at = observe_required(clock, "reasoning-reserve")?.observed_at_unix_ms;
        let scope = self.query_scope(input.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        // The following semantic/control commit also checks this runtime cursor:
        // concurrent task closure cannot publish a new reservation.
        self.open_task_at(&read, input.scope, input.task_id)?;
        let (expected, existing) = self.load_task_budget(input.scope, input.task_id)?;
        let attempt_key = self.attempt_key(input.scope, input.task_id, input.attempt_id);
        if let Some(bytes) = self.storage.control().get(&attempt_key)? {
            let previous: AttemptRecord = serde_json::from_slice(&bytes).map_err(contract_json)?;
            if previous.format_version != 1 || previous.attempt_id != *input.attempt_id {
                return Err(ServiceError::Storage(
                    "reasoning attempt identity is corrupt".into(),
                ));
            }
            if previous.request_sha256 != input.request_sha256 {
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
            scope: input.scope.into(),
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
            repetition_policy: None,
            equivalent_attempts: BTreeMap::new(),
        });
        if let Some(reason) = budget.stopped {
            return Err(ServiceError::ReasoningBudgetStopped(reason));
        }
        if budget.active_attempt.is_some() {
            return Err(ServiceError::ReasoningBudgetStopped(
                ReasoningBudgetStop::UnresolvedAttempt,
            ));
        }
        let mut stop = if at < budget.last_observed_at_unix_ms {
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
        let identity = if stop.is_none() && budget.repetition_policy.is_some() {
            let identity = repetition_identity()?;
            validate_attempt_digest(&identity)?;
            Some(identity)
        } else {
            None
        };
        if let Some(identity) = &identity {
            stop = repetition::check(&budget, identity);
        }
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
        if let Some(identity) = identity {
            *budget.equivalent_attempts.entry(identity).or_default() += 1;
        }
        budget.active_attempt = Some(input.attempt_id.clone());
        budget.execution_may_continue = true;
        let attempt = AttemptRecord {
            format_version: 1,
            attempt_id: input.attempt_id.clone(),
            request_sha256: input.request_sha256.into(),
            ordinal: budget.reserved_steps,
            read_manifest_sha256: read.manifest_id.clone(),
            read_cursor: read.commit_cursor,
            outcome: AttemptOutcome::Reserved,
            outcome_sha256: None,
            result: None,
            endpoint_proposal: None,
            worker_output: None,
            lfg: None,
            assessment_output: None,
            invalidation: None,
        };
        self.commit_budget_controlled(
            &read,
            expected,
            &budget,
            Some((None, &attempt)),
            admission,
            context,
        )?;
        Ok(Reservation {
            budget,
            attempt,
            read,
            started,
            control: EmbeddingExecutionControl::new(deadline, input.cancellation.clone()),
        })
    }
}
