//! Task assessment records, task embeddings, budget inspection and LFG advisory checkpoints.

use super::*;

impl RrdEngine {
    pub(in super::super) fn read_task_assessment_record(
        &self,
        request: &rrd_contract::ReadTaskAssessment,
    ) -> Result<rrd_contract::TaskAssessmentView> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.query_scope(&request.scope)?;
        let key = self.attempt_key(&request.scope, &request.task_id, &request.attempt_id);
        let bytes = self.storage.control().get(&key)?;
        let Some(bytes) = bytes else {
            return Ok(rrd_contract::TaskAssessmentView {
                contract_version: rrd_contract::TASK_ASSESSMENT_VERSION,
                assessment: None,
                outcome_sha256: None,
            });
        };
        let record: AttemptRecord = serde_json::from_slice(&bytes).map_err(contract_json)?;
        if record.format_version != 1 || record.attempt_id != request.attempt_id {
            return Err(ServiceError::Storage(
                "task assessment attempt identity is corrupt".into(),
            ));
        }
        let Some(output) = record.assessment_output else {
            if record.outcome == AttemptOutcome::Accepted
                && record
                    .outcome_sha256
                    .as_ref()
                    .is_some_and(|digest| digest.is_empty())
            {
                return Err(ServiceError::Storage(
                    "task assessment outcome digest is corrupt".into(),
                ));
            }
            return Ok(rrd_contract::TaskAssessmentView {
                contract_version: rrd_contract::TASK_ASSESSMENT_VERSION,
                assessment: None,
                outcome_sha256: None,
            });
        };
        output
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let output_bytes = serde_json::to_vec(&output).map_err(contract_json)?;
        let expected = digest::sha256_hex(&output_bytes);
        if record.outcome != AttemptOutcome::Accepted
            || output.scope != request.scope
            || output.task_id != request.task_id
            || output.attempt_id != request.attempt_id
            || record.result.is_some()
            || record.endpoint_proposal.is_some()
            || record.worker_output.is_some()
            || record.outcome_sha256.as_deref() != Some(expected.as_str())
        {
            return Err(ServiceError::Storage(
                "task assessment retained output is inconsistent".into(),
            ));
        }
        Ok(rrd_contract::TaskAssessmentView {
            contract_version: rrd_contract::TASK_ASSESSMENT_VERSION,
            assessment: Some(output),
            outcome_sha256: Some(expected),
        })
    }

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
    pub(in super::super) fn generate_task_embeddings_with_clock(
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

    /// Checkpoint an observed analyzer effect before any subsequent dispatch.
    /// The attempt remains reserved; a crash cannot make it replayable.
    pub(in super::super) fn checkpoint_lfg_advisory(
        &self,
        reservation: &mut Reservation,
        advisory: rrd_contract::LfgAdvisory,
        mut admit: impl FnMut(&ReadStamp) -> Result<TaskCompletionAdmission>,
        clock: &dyn ClockSource,
        context: &EngineControlContext<'_>,
    ) -> Result<Option<rrd_contract::TaskAttemptInvalidation>> {
        advisory
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        if advisory.request_id != reservation.attempt.attempt_id
            || reservation.attempt.lfg.is_some()
        {
            return Err(ServiceError::Contract(
                "invalid LFG checkpoint identity".into(),
            ));
        }
        let expected_attempt = serde_json::to_vec(&reservation.attempt).map_err(contract_json)?;
        let mut next = reservation.attempt.clone();
        next.lfg = Some(advisory);
        for index in 0..3 {
            let (expected_budget, current) =
                self.load_task_budget(&reservation.budget.scope, &reservation.budget.task_id)?;
            let budget = current
                .ok_or_else(|| ServiceError::Storage("reserved budget disappeared".into()))?;
            if budget.active_attempt.as_ref() != Some(&next.attempt_id)
                || budget.reserved_steps != reservation.budget.reserved_steps
                || budget.configuration_sha256 != reservation.budget.configuration_sha256
                || budget.limits != reservation.budget.limits
                || budget.stopped.is_some()
            {
                return Err(ServiceError::StorageConflict(
                    "attempt no longer owns checkpoint".into(),
                ));
            }
            let read = self
                .storage
                .runtime()
                .read_stamp(&self.query_scope(&budget.scope)?)?;
            let admission = admit(&read)?;
            let invalidation = admission.invalidation;
            // Recording an already completed effect is cleanup, not authority to
            // dispatch. Preserve it even when inputs changed or time expired.
            observe_required(clock, "lfg-checkpoint")?;
            match self.commit_budget_controlled(
                &read,
                expected_budget,
                &budget,
                Some((Some(expected_attempt.clone()), &next)),
                &admission,
                context,
            ) {
                Err(ServiceError::StorageConflict(_)) if index < 2 => continue,
                outcome => {
                    outcome?;
                    reservation.attempt = next;
                    return Ok(invalidation);
                }
            }
        }
        Err(ServiceError::StorageConflict(
            "LFG checkpoint retries exhausted".into(),
        ))
    }
}
