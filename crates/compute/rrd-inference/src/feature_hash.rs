//! The deterministic feature-hash embedding backend.

use super::*;

/// Deterministic, dependency-free local baseline for offline operation.
///
/// It is a feature-hashing model, not a semantic-model quality claim. Its role
/// is to keep the full source/provenance/commit pipeline executable when no
/// ONNX or accelerator adapter is installed.
pub struct FeatureHashBackend {
    descriptor: EmbeddingBackendDescriptor,
    seed: u64,
}

impl FeatureHashBackend {
    pub fn new(dimensions: u32, seed: u64) -> Result<Self> {
        if !(8..=65_536).contains(&dimensions) {
            return invalid("feature-hash dimensions must be in 8..=65536");
        }
        let model_identity = serde_json::to_vec(&("rrflow-feature-hash-v1", dimensions, seed))
            .map_err(|error| Error::InvalidRuntime {
                reason: format!("feature-hash model identity cannot be encoded: {error}"),
            })?;
        Ok(Self {
            descriptor: {
                let model = EmbeddingModelSpec {
                    provider: "rrflow".into(),
                    model: "feature-hash".into(),
                    revision: "v1".into(),
                    model_digest: digest::sha256_hex(&model_identity),
                    modality: EmbeddingModality::Text,
                    dimensions,
                    normalization: VectorNormalization::UnitL2,
                    maximum_input_bytes: 4 * 1024 * 1024,
                };
                EmbeddingBackendDescriptor {
                    id: "rrflow:feature-hash:cpu:v1".into(),
                    resources: EmbeddingResourceLimits::for_model(&model, 256)?,
                    model,
                    execution: ExecutionTarget::Cpu,
                    network: NetworkRequirement::None,
                    trust: InferenceTrustBoundary::LocalOffline,
                    deterministic: true,
                }
            },
            seed,
        })
    }
}

impl EmbeddingBackend for FeatureHashBackend {
    fn descriptor(&self) -> &EmbeddingBackendDescriptor {
        &self.descriptor
    }

    fn embed(&mut self, request: &EmbeddingRequest) -> Result<VectorValue> {
        self.embed_controlled(request, None)
    }

    fn embed_batch_controlled(
        &mut self,
        requests: &[EmbeddingRequest],
        control: &EmbeddingExecutionControl,
    ) -> std::result::Result<Vec<VectorValue>, EmbeddingDispatchError> {
        control.check()?;
        let result: Result<Vec<_>> = requests
            .iter()
            .map(|request| self.embed_controlled(request, Some(control)))
            .collect();
        control.check()?;
        result.map_err(Into::into)
    }
}

impl FeatureHashBackend {
    fn embed_controlled(
        &mut self,
        request: &EmbeddingRequest,
        control: Option<&EmbeddingExecutionControl>,
    ) -> Result<VectorValue> {
        self.embed_with_checkpoint(request, || -> Result<()> {
            if let Some(control) = control {
                control.check().map_err(|error| Error::InvalidRuntime {
                    reason: error.to_string(),
                })?;
            }
            Ok(())
        })
    }

    pub(super) fn embed_with_checkpoint(
        &mut self,
        request: &EmbeddingRequest,
        mut checkpoint: impl FnMut() -> Result<()>,
    ) -> Result<VectorValue> {
        checkpoint()?;
        if request.bytes.len() > self.descriptor.model.maximum_input_bytes as usize {
            return invalid("embedding input exceeds model byte limit");
        }
        if !request.media_type.starts_with("text/") && request.media_type != "application/json" {
            return invalid("feature-hash backend accepts text or JSON only");
        }
        let text = std::str::from_utf8(&request.bytes)
            .map_err(|_| Error::InvalidRuntime {
                reason: "feature-hash input must be UTF-8".into(),
            })?
            .to_lowercase();
        let mut values = vec![0.0_f32; self.descriptor.model.dimensions as usize];
        let mut tokens = BTreeSet::new();
        checkpoint()?;
        for token in text.split(|character: char| !character.is_alphanumeric()) {
            checkpoint()?;
            if !token.is_empty() {
                tokens.insert(token);
            }
        }
        if tokens.is_empty() {
            return invalid("feature-hash input contains no tokens");
        }
        for token in tokens {
            checkpoint()?;
            let mut identity = b"rrflow-feature-hash-token-v1\0".to_vec();
            identity.extend_from_slice(&self.seed.to_be_bytes());
            identity.extend_from_slice(token.as_bytes());
            let hash = digest::sha256(&identity);
            let index = u64::from_be_bytes(hash[..8].try_into().expect("eight-byte hash prefix"))
                as usize
                % values.len();
            let sign = if hash[8] & 1 == 0 { 1.0 } else { -1.0 };
            values[index] += sign;
        }
        let norm = values
            .iter()
            .map(|value| f64::from(*value).powi(2))
            .sum::<f64>()
            .sqrt() as f32;
        checkpoint()?;
        if !norm.is_finite() || norm <= 0.0 {
            return invalid("feature-hash token contributions have no finite nonzero norm");
        }
        for value in &mut values {
            *value /= norm;
        }
        checkpoint()?;
        Ok(VectorValue::Dense { values })
    }
}
