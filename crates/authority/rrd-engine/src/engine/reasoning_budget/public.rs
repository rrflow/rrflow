use super::*;
use rrd_contract::{
    ActivateBuiltinEmbedding, ConfigureTaskBudget, ExecuteTaskEmbedding, ReadTaskAttempt,
    ReadTaskBudget, StopTaskBudget, TaskAttemptSnapshot, TaskAttemptStatus, TaskAttemptView,
    TaskBudgetView,
};
use rrd_inference::EmbeddingBackend;

fn contract(error: rrd_contract::ContractError) -> ServiceError {
    ServiceError::Contract(error.to_string())
}

fn view(budget: Option<TaskReasoningBudget>) -> Result<TaskBudgetView> {
    let budget_sha256 = budget.as_ref().map(operation_digest).transpose()?;
    let result = TaskBudgetView {
        budget,
        budget_sha256,
    };
    result.validate().map_err(contract)?;
    Ok(result)
}

impl RrdEngine {
    pub fn configure_task_budget(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ConfigureTaskBudget,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TaskBudgetView> {
        request.validate().map_err(contract)?;
        self.authorize(
            session,
            token,
            SecurityAction::ReasoningWrite,
            now,
            request_id,
            operation_id,
        )?;
        let caps = &self.estate_configuration.reasoning;
        if request.limits.max_run_elapsed_ms > caps.max_run_elapsed_ms
            || request.limits.max_steps > caps.max_steps
            || request.limits.max_step_elapsed_ms > caps.max_step_elapsed_ms
        {
            return Err(ServiceError::ConfigurationLimit(
                "task limits cannot exceed estate caps".into(),
            ));
        }
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        self.open_task_at(&read, &request.scope, &request.task_id)?;
        let (expected, existing) = self.load_task_budget(&request.scope, &request.task_id)?;
        if let Some(existing) = existing {
            if existing.limits != request.limits
                || existing.repetition_policy != request.repetition_policy
                || existing.configuration_sha256 != self.estate_configuration.configuration_sha256
            {
                return Err(ServiceError::IdempotencyConflict);
            }
            return view(Some(existing));
        }
        let at = observe_required(&HostSystemClock, "reasoning-configure")?.observed_at_unix_ms;
        let budget = TaskReasoningBudget {
            format_version: 1,
            instance_id: self.instance.clone(),
            scope: request.scope.clone(),
            task_id: request.task_id.clone(),
            configuration_sha256: self.estate_configuration.configuration_sha256.clone(),
            limits: request.limits.clone(),
            started_at_unix_ms: at,
            deadline_unix_ms: at
                .checked_add(request.limits.max_run_elapsed_ms)
                .ok_or(ServiceError::DeadlineExceeded)?,
            last_observed_at_unix_ms: at,
            charged_elapsed_ms: 0,
            reserved_steps: 0,
            active_attempt: None,
            execution_may_continue: false,
            stopped: None,
            stop_expected_sha256: None,
            repetition_policy: request.repetition_policy.clone(),
            equivalent_attempts: BTreeMap::new(),
        };
        self.commit_budget(
            &read,
            expected,
            &budget,
            None,
            &EngineControlContext {
                now,
                session_id: session,
                action: "reasoning-configure",
                request_id,
                operation_id,
            },
        )?;
        view(Some(budget))
    }

    pub fn read_task_budget(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ReadTaskBudget,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TaskBudgetView> {
        request.validate().map_err(contract)?;
        view(self.inspect_task_reasoning_budget(
            session,
            token,
            &request.scope,
            &request.task_id,
            now,
            request_id,
            operation_id,
        )?)
    }

    pub fn stop_task_budget(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &StopTaskBudget,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TaskBudgetView> {
        request.validate().map_err(contract)?;
        self.authorize(
            session,
            token,
            SecurityAction::ReasoningWrite,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let active = self
            .active_task_cancellations
            .lock()
            .map_err(|_| ServiceError::Storage("task cancellation lock poisoned".into()))?;
        let (expected, existing) = self.load_task_budget(&request.scope, &request.task_id)?;
        let mut budget = existing.ok_or(ServiceError::TaskNotFound)?;
        // A lost Stop acknowledgement is replayable using its original digest.
        if budget.stop_expected_sha256.as_ref() == Some(&request.expected_budget_sha256) {
            return view(Some(budget));
        }
        if operation_digest(&budget)? != request.expected_budget_sha256 {
            return Err(ServiceError::IdempotencyConflict);
        }
        let at = observe_required(&HostSystemClock, "reasoning-stop")?.observed_at_unix_ms;
        budget.last_observed_at_unix_ms = budget.last_observed_at_unix_ms.max(at);
        budget.stopped = Some(ReasoningBudgetStop::Cancelled);
        budget.stop_expected_sha256 = Some(request.expected_budget_sha256.clone());
        let read = self.storage.runtime().read_stamp(&scope)?;
        self.commit_budget(
            &read,
            expected,
            &budget,
            None,
            &EngineControlContext {
                now,
                session_id: session,
                action: "reasoning-stop",
                request_id,
                operation_id,
            },
        )?;
        if let Some((_, cancellation)) =
            active.get(&self.budget_key(&request.scope, &request.task_id))
        {
            cancellation.cancel();
        }
        view(Some(budget))
    }

    pub fn execute_task_embedding(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ExecuteTaskEmbedding,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<GenerateEmbeddingsResult> {
        request.validate().map_err(contract)?;
        self.authorize(
            session,
            token,
            SecurityAction::ReasoningWrite,
            now,
            request_id,
            operation_id,
        )?;
        self.query_scope(&request.request.scope)?;
        if self
            .load_task_budget(&request.request.scope, &request.task_id)?
            .1
            .is_none()
        {
            return Err(ServiceError::ConfigurationLimit(
                "configure the task budget before execution".into(),
            ));
        }
        let descriptor = self
            .embedding_backends
            .lock()
            .map_err(|_| ServiceError::Storage("embedding registry lock poisoned".into()))?
            .descriptor(&request.request.backend_id)
            .cloned()
            .ok_or_else(|| {
                ServiceError::Inference("activate the requested backend before execution".into())
            })?;
        if descriptor.model.dimensions as usize * request.request.inputs.len()
            > rrd_contract::MAX_TASK_EMBEDDING_VALUES
        {
            return Err(ServiceError::ConfigurationLimit(
                "task embedding output exceeds 4096 values".into(),
            ));
        }
        self.generate_task_embeddings(
            session,
            token,
            &TaskEmbeddingAttempt {
                task_id: &request.task_id,
                attempt_id: &request.attempt_id,
                request: &request.request,
                cancellation: &EmbeddingCancellation::default(),
            },
            now,
            request_id,
            operation_id,
        )
    }

    pub fn read_task_attempt(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ReadTaskAttempt,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TaskAttemptView> {
        request.validate().map_err(contract)?;
        self.authorize(
            session,
            token,
            SecurityAction::ReasoningRead,
            now,
            request_id,
            operation_id,
        )?;
        self.query_scope(&request.scope)?;
        let bytes = self.storage.control().get(&self.attempt_key(
            &request.scope,
            &request.task_id,
            &request.attempt_id,
        ))?;
        let record: Option<AttemptRecord> = bytes
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()
            .map_err(contract_json)?;
        if record.as_ref().is_some_and(|record| {
            record.format_version != 1
                || record
                    .result
                    .as_ref()
                    .is_some_and(|result| result.scope != request.scope)
        }) {
            return Err(ServiceError::Storage(
                "attempt format or scope mismatch".into(),
            ));
        }
        let result = TaskAttemptView {
            attempt: record.map(|record| TaskAttemptSnapshot {
                attempt_id: record.attempt_id,
                ordinal: record.ordinal,
                status: match record.outcome {
                    AttemptOutcome::Reserved => TaskAttemptStatus::Reserved,
                    AttemptOutcome::Accepted => TaskAttemptStatus::Accepted,
                    AttemptOutcome::Stopped => TaskAttemptStatus::Stopped,
                },
                outcome_sha256: record.outcome_sha256,
                result: record.result,
            }),
        };
        result.validate().map_err(contract)?;
        if result
            .attempt
            .as_ref()
            .is_some_and(|a| a.attempt_id != request.attempt_id)
        {
            return Err(ServiceError::Storage("attempt identity mismatch".into()));
        }
        Ok(result)
    }

    pub fn activate_builtin_embedding(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ActivateBuiltinEmbedding,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<EmbeddingModelCatalogue> {
        request.validate().map_err(contract)?;
        self.authorize(
            session,
            token,
            SecurityAction::ReasoningWrite,
            now,
            request_id,
            operation_id,
        )?;
        self.query_scope(&request.scope)?;
        let backend =
            rrd_inference::FeatureHashBackend::new(request.dimensions, u64::from(request.seed))
                .map_err(|e| ServiceError::Inference(e.to_string()))?;
        let mut registry = self
            .embedding_backends
            .lock()
            .map_err(|_| ServiceError::Storage("embedding registry lock poisoned".into()))?;
        if let Some(existing) = registry.descriptor(&backend.descriptor().id) {
            if existing != backend.descriptor() {
                return Err(ServiceError::IdempotencyConflict);
            }
        } else {
            registry
                .install(Box::new(backend))
                .map_err(|e| ServiceError::Inference(e.to_string()))?;
        }
        Ok(EmbeddingModelCatalogue {
            scope: request.scope.clone(),
            registry_revision: registry.revision(),
            backends: registry
                .descriptors()
                .iter()
                .map(super::super::inference::public_embedding_backend)
                .collect(),
        })
    }
}
