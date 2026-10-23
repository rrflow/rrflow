use super::*;
use std::collections::BTreeSet;

const EVENT_KIND: &str = "rrflow-reasoning-budget-event";

impl RrdEngine {
    pub(super) fn budget_key(&self, scope: &str, task: &CanonicalId) -> String {
        // Typed identities are encoded before hashing; no delimiter ambiguity
        // or caller-controlled control-key suffix is accepted.
        let identity = serde_json::to_vec(&(self.instance.as_str(), scope, task.as_str()))
            .expect("strings serialize");
        format!(
            "server/state/reasoning-budget/{}",
            digest::sha256_hex(&identity)
        )
    }

    pub(super) fn attempt_key(
        &self,
        scope: &str,
        task: &CanonicalId,
        attempt: &CorrelationId,
    ) -> String {
        let identity = serde_json::to_vec(&(
            self.instance.as_str(),
            scope,
            task.as_str(),
            attempt.as_str(),
        ))
        .expect("strings serialize");
        format!(
            "server/state/reasoning-attempt/{}",
            digest::sha256_hex(&identity)
        )
    }

    pub(super) fn load_task_budget(
        &self,
        scope: &str,
        task: &CanonicalId,
    ) -> Result<(Option<Vec<u8>>, Option<TaskReasoningBudget>)> {
        let bytes = self.storage.control().get(&self.budget_key(scope, task))?;
        let state: Option<TaskReasoningBudget> = bytes
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()
            .map_err(contract_json)?;
        if let Some(state) = &state {
            state
                .limits
                .validate()
                .map_err(|e| ServiceError::Storage(e.to_string()))?;
            if state.format_version != 1
                || state.instance_id != self.instance
                || state.scope != scope
                || state.task_id != *task
                || state.started_at_unix_ms == 0
                || state
                    .started_at_unix_ms
                    .checked_add(state.limits.max_run_elapsed_ms)
                    != Some(state.deadline_unix_ms)
                || state.last_observed_at_unix_ms < state.started_at_unix_ms
                || state.reserved_steps > state.limits.max_steps
                || (state.active_attempt.is_some() && state.reserved_steps == 0)
            {
                return Err(ServiceError::Storage(
                    "reasoning budget identity or bounds are corrupt".into(),
                ));
            }
        }
        Ok((bytes, state))
    }

    pub(super) fn commit_budget(
        &self,
        read: &ReadStamp,
        expected: Option<Vec<u8>>,
        state: &TaskReasoningBudget,
        attempt: Option<(Option<Vec<u8>>, &AttemptRecord)>,
        context: &EngineControlContext<'_>,
    ) -> Result<()> {
        let current = self.storage.runtime().schema(&read.scope)?;
        if current.as_ref().map(|s| s.revision) != read.schema_revision {
            return Err(ServiceError::StorageConflict(
                "reasoning schema changed".into(),
            ));
        }
        let mut registry = current
            .clone()
            .ok_or_else(|| ServiceError::Storage("task schema is absent".into()))?;
        let mut mutations = Vec::with_capacity(2);
        let required = RuntimePropertySchema::required;
        if registry
            .define_event_table(
                RuntimeType::new(EVENT_KIND).map_err(core_contract)?,
                RuntimeLogicalModel::LifecycleEvent,
                RuntimeEventSchema {
                    subject_required: true,
                    subject_types: BTreeSet::from([
                        RuntimeType::new(rrd_contract::TASK_KIND).map_err(core_contract)?
                    ]),
                    properties: BTreeMap::from([
                        ("budget_sha256".into(), required(RuntimeValueType::Digest)),
                        (
                            "reserved_steps".into(),
                            required(RuntimeValueType::Unsigned),
                        ),
                        (
                            "deadline_unix_ms".into(),
                            required(RuntimeValueType::Unsigned),
                        ),
                        (
                            "attempt_sha256".into(),
                            RuntimePropertySchema::optional(RuntimeValueType::Digest),
                        ),
                    ]),
                    allow_additional_properties: false,
                },
            )
            .map_err(core_contract)?
        {
            registry.revision = registry
                .revision
                .checked_add(1)
                .ok_or_else(|| ServiceError::Contract("schema revision overflow".into()))?;
            registry.migration = "install reasoning budget evidence".into();
            mutations.push(RuntimeMutation::Schema { registry });
        }
        let replacement = serde_json::to_vec(state).map_err(contract_json)?;
        let mut properties = RuntimeProperties::from([
            (
                "budget_sha256".into(),
                RuntimeValue::Digest(digest::sha256_hex(&replacement)),
            ),
            (
                "reserved_steps".into(),
                RuntimeValue::Unsigned(state.reserved_steps),
            ),
            (
                "deadline_unix_ms".into(),
                RuntimeValue::Unsigned(state.deadline_unix_ms),
            ),
        ]);
        let transition = |key, expected, replacement| ControlTransition {
            key,
            expected,
            replacement: Some(replacement),
            at: state.last_observed_at_unix_ms,
            actor: format!("session:{}", context.session_id.as_str()),
            action: context.action.into(),
            request_id: context.request_id.into(),
            operation_id: context.operation_id.into(),
        };
        let mut transitions = vec![transition(
            self.budget_key(&state.scope, &state.task_id),
            expected,
            replacement,
        )];
        if let Some((expected, attempt)) = attempt {
            let replacement = serde_json::to_vec(attempt).map_err(contract_json)?;
            properties.insert(
                "attempt_sha256".into(),
                RuntimeValue::Digest(digest::sha256_hex(&replacement)),
            );
            transitions.push(transition(
                self.attempt_key(&state.scope, &state.task_id, &attempt.attempt_id),
                expected,
                replacement,
            ));
        }
        mutations.push(RuntimeMutation::Event {
            event: RuntimeEvent {
                kind: RuntimeType::new(EVENT_KIND).map_err(core_contract)?,
                subject: Some(
                    RuntimeRef::new(rrd_contract::TASK_KIND, state.task_id.as_str())
                        .map_err(core_contract)?,
                ),
                properties,
            },
        });
        let commit = RuntimeCommit {
            scope: read.scope.clone(),
            at: state.last_observed_at_unix_ms,
            actor: format!("session:{}", context.session_id.as_str()),
            expected_cursor: read.commit_cursor,
            mutations,
        };
        // Both serialized control payloads count toward engine write admission.
        let _permit = self.admit_transaction(
            &(&commit, &transitions),
            context.request_id,
            context.operation_id,
        )?;
        self.storage
            .runtime()
            .commit_with_control_transitions(&commit, &transitions)?;
        Ok(())
    }
}
