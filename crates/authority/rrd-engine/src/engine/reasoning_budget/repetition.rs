use super::*;

/// The whole task history is bounded; never evict a fingerprint and silently
/// permit a cycle to start again. Existing entries may spend their remainder.
pub(super) fn check(budget: &TaskReasoningBudget, identity: &str) -> Option<ReasoningBudgetStop> {
    let policy = budget.repetition_policy.as_ref()?;
    match budget.equivalent_attempts.get(identity) {
        Some(count) if *count >= policy.max_equivalent_attempts => {
            Some(ReasoningBudgetStop::RepeatedWork)
        }
        None if budget.equivalent_attempts.len()
            >= rrd_contract::MAX_TASK_REPETITION_IDENTITIES =>
        {
            Some(ReasoningBudgetStop::RepetitionTrackingLimit)
        }
        _ => None,
    }
}

impl RrdEngine {
    pub(super) fn embedding_repetition_identity(
        &self,
        request: &GenerateEmbeddings,
    ) -> Result<String> {
        let model = self
            .embedding_backends
            .lock()
            .map_err(|_| ServiceError::Storage("embedding registry lock poisoned".into()))?
            .descriptor(&request.backend_id)
            .ok_or_else(|| {
                ServiceError::Inference("repetition policy requires an activated backend".into())
            })?
            .model
            .clone();
        // Names, correlation IDs, input IDs, batch order and the changing
        // budget-event cursor are not new embedding evidence. Preserve batch
        // multiplicity, exact bytes/media and model processing identity.
        let mut inputs = request
            .inputs
            .iter()
            .map(|input| (input.media_type.as_str(), digest::sha256_hex(&input.bytes)))
            .collect::<Vec<_>>();
        inputs.sort_unstable();
        operation_digest(&(
            "rrflow-task-embedding-equivalence-v1",
            &model.model_digest,
            model.modality,
            model.dimensions,
            model.normalization,
            inputs,
        ))
    }
}
