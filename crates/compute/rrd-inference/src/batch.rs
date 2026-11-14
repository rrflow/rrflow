//! The embedding backend trait, batch dispatch and batch validation.

use super::*;

pub trait EmbeddingBackend {
    fn descriptor(&self) -> &EmbeddingBackendDescriptor;
    fn embed(&mut self, request: &EmbeddingRequest) -> Result<VectorValue>;

    /// Backends with a native batching primitive should override this method.
    /// The coordinator applies the same batch limits before either path runs.
    fn embed_batch(&mut self, requests: &[EmbeddingRequest]) -> Result<Vec<VectorValue>> {
        requests.iter().map(|request| self.embed(request)).collect()
    }

    /// Override to observe stop requests during computation. The default
    /// preserves native batching but cannot interrupt its synchronous call.
    fn embed_batch_controlled(
        &mut self,
        requests: &[EmbeddingRequest],
        control: &EmbeddingExecutionControl,
    ) -> std::result::Result<Vec<VectorValue>, EmbeddingDispatchError> {
        control.check()?;
        let result = self.embed_batch(requests);
        control.check()?;
        result.map_err(Into::into)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingBatchResult {
    pub registry_revision: u64,
    pub backend: EmbeddingBackendDescriptor,
    pub vectors: Vec<VectorValue>,
}

pub(super) fn dispatch_embedding_batch(
    backend: &mut dyn EmbeddingBackend,
    network_policy: NetworkPolicy,
    requests: &[EmbeddingRequest],
) -> Result<Vec<VectorValue>> {
    let descriptor = backend.descriptor().clone();
    validate_embedding_batch(&descriptor, network_policy, requests)?;
    let vectors = backend.embed_batch(requests)?;
    validate_embedding_batch_output(&descriptor, requests.len(), &vectors)?;
    Ok(vectors)
}

pub(super) fn validate_embedding_batch(
    descriptor: &EmbeddingBackendDescriptor,
    network_policy: NetworkPolicy,
    requests: &[EmbeddingRequest],
) -> Result<()> {
    descriptor.validate()?;
    if network_policy == NetworkPolicy::Deny && descriptor.network == NetworkRequirement::Required {
        return invalid("embedding request denies the network required by its backend");
    }
    if requests.is_empty()
        || requests.len()
            > usize::try_from(descriptor.resources.maximum_batch_inputs).map_err(|_| {
                Error::InvalidRuntime {
                    reason: "embedding batch input limit exceeds usize".into(),
                }
            })?
    {
        return invalid("embedding batch input count exceeds the backend limit");
    }
    let mut total_bytes = 0_u64;
    let mut job_ids = BTreeSet::new();
    for request in requests {
        request.validate()?;
        if !job_ids.insert(request.job_id.clone()) {
            return invalid("embedding batch job ids must be unique");
        }
        let bytes = u64::try_from(request.bytes.len()).map_err(|_| Error::InvalidRuntime {
            reason: "embedding input length exceeds u64".into(),
        })?;
        if bytes > descriptor.resources.maximum_input_bytes {
            return invalid("embedding input exceeds the backend byte limit");
        }
        total_bytes = total_bytes
            .checked_add(bytes)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: "embedding batch byte count overflowed u64".into(),
            })?;
    }
    if total_bytes > descriptor.resources.maximum_batch_bytes {
        return invalid("embedding batch exceeds the backend byte limit");
    }
    let output_values = u64::from(descriptor.model.dimensions)
        .checked_mul(
            u64::try_from(requests.len()).map_err(|_| Error::InvalidRuntime {
                reason: "embedding batch size exceeds u64".into(),
            })?,
        )
        .ok_or_else(|| Error::InvalidRuntime {
            reason: "embedding batch output shape overflowed u64".into(),
        })?;
    if output_values > descriptor.resources.maximum_output_values {
        return invalid("embedding batch exceeds the backend output value limit");
    }

    Ok(())
}

pub(super) fn validate_embedding_batch_output(
    descriptor: &EmbeddingBackendDescriptor,
    expected: usize,
    vectors: &[VectorValue],
) -> Result<()> {
    if vectors.len() != expected {
        return invalid("embedding backend returned an unexpected batch shape");
    }
    for vector in vectors {
        validate_embedding_value(vector, &descriptor.model)?;
    }
    Ok(())
}

pub(super) fn validate_embedding_value(
    value: &VectorValue,
    model: &EmbeddingModelSpec,
) -> Result<()> {
    let VectorValue::Dense { values } = value else {
        return invalid("embedding backend must return one dense vector per input");
    };
    if values.len() != model.dimensions as usize || values.iter().any(|value| !value.is_finite()) {
        return invalid("embedding backend output differs from the declared dimensions");
    }
    if model.normalization == VectorNormalization::UnitL2 {
        let norm = values
            .iter()
            .map(|value| f64::from(*value).powi(2))
            .sum::<f64>()
            .sqrt();
        if !norm.is_finite() || (norm - 1.0).abs() > 1e-4 {
            return invalid("embedding backend output differs from the declared normalization");
        }
    }
    Ok(())
}
