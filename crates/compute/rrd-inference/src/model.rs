//! Embedding modality, network policy, trust boundaries, limits, specs, descriptors, jobs, sources and requests.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingModality {
    Text,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    Deny,
    Allow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkRequirement {
    None,
    Required,
}

/// Declares where unredacted inference input is allowed to cross.
///
/// A local/offline backend cannot require the network or declare a remote
/// execution target. A provider backend must name the same provider at both
/// the execution and trust boundaries and must require an allow-network job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InferenceTrustBoundary {
    LocalOffline,
    RemoteProvider { provider: String },
}

/// Hard dispatch limits enforced by the coordinator before backend code runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingResourceLimits {
    pub maximum_batch_inputs: u32,
    pub maximum_input_bytes: u64,
    pub maximum_batch_bytes: u64,
    pub maximum_output_values: u64,
}

impl EmbeddingResourceLimits {
    pub fn for_model(model: &EmbeddingModelSpec, maximum_batch_inputs: u32) -> Result<Self> {
        model.validate()?;
        let maximum_batch_bytes = model
            .maximum_input_bytes
            .checked_mul(u64::from(maximum_batch_inputs))
            .ok_or_else(|| Error::InvalidRuntime {
                reason: "embedding batch byte limit overflowed u64".into(),
            })?
            .min(MAX_EMBEDDING_BATCH_BYTES);
        let maximum_output_values = u64::from(model.dimensions)
            .checked_mul(u64::from(maximum_batch_inputs))
            .ok_or_else(|| Error::InvalidRuntime {
                reason: "embedding output value limit overflowed u64".into(),
            })?
            .min(MAX_EMBEDDING_OUTPUT_VALUES);
        let limits = Self {
            maximum_batch_inputs,
            maximum_input_bytes: model.maximum_input_bytes,
            maximum_batch_bytes,
            maximum_output_values,
        };
        limits.validate(model)?;
        Ok(limits)
    }

    pub fn validate(&self, model: &EmbeddingModelSpec) -> Result<()> {
        if self.maximum_batch_inputs == 0 || self.maximum_batch_inputs > MAX_EMBEDDING_BATCH_INPUTS
        {
            return invalid("embedding maximum batch inputs must be in 1..=1024");
        }
        if self.maximum_input_bytes == 0 || self.maximum_input_bytes > model.maximum_input_bytes {
            return invalid("embedding per-input byte limit exceeds the model contract");
        }
        if self.maximum_batch_bytes < self.maximum_input_bytes
            || self.maximum_batch_bytes > MAX_EMBEDDING_BATCH_BYTES
        {
            return invalid("embedding batch byte limit is invalid");
        }
        if self.maximum_output_values < u64::from(model.dimensions)
            || self.maximum_output_values > MAX_EMBEDDING_OUTPUT_VALUES
        {
            return invalid("embedding output value limit is invalid");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionTarget {
    Cpu,
    Gpu { platform: String, device: String },
    Remote { provider: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingModelSpec {
    pub provider: String,
    pub model: String,
    pub revision: String,
    pub model_digest: String,
    pub modality: EmbeddingModality,
    pub dimensions: u32,
    pub normalization: VectorNormalization,
    pub maximum_input_bytes: u64,
}

impl EmbeddingModelSpec {
    pub fn validate(&self) -> Result<()> {
        validate_text("embedding provider", &self.provider)?;
        validate_text("embedding model", &self.model)?;
        validate_text("embedding revision", &self.revision)?;
        validate_digest("embedding model", &self.model_digest)?;
        if self.dimensions == 0 || self.dimensions > 1_048_576 {
            return invalid("embedding model dimensions must be in 1..=1048576");
        }
        if self.maximum_input_bytes == 0
            || self.maximum_input_bytes > MAX_EMBEDDING_INPUT_BYTES as u64
        {
            return invalid("embedding maximum input bytes must be in 1..=67108864");
        }
        Ok(())
    }

    pub fn canonical_name(&self) -> String {
        format!("{}/{}@{}", self.provider, self.model, self.revision)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingBackendDescriptor {
    pub id: String,
    pub model: EmbeddingModelSpec,
    pub execution: ExecutionTarget,
    pub network: NetworkRequirement,
    pub trust: InferenceTrustBoundary,
    pub resources: EmbeddingResourceLimits,
    pub deterministic: bool,
}

impl EmbeddingBackendDescriptor {
    pub fn validate(&self) -> Result<()> {
        validate_text("embedding backend id", &self.id)?;
        self.model.validate()?;
        self.resources.validate(&self.model)?;
        match &self.execution {
            ExecutionTarget::Cpu => {}
            ExecutionTarget::Gpu { platform, device } => {
                validate_text("embedding GPU platform", platform)?;
                validate_text("embedding GPU device", device)?;
            }
            ExecutionTarget::Remote { provider } => {
                validate_text("embedding remote provider", provider)?;
                if self.network != NetworkRequirement::Required {
                    return invalid("remote embedding execution must require network access");
                }
            }
        }
        match (&self.trust, &self.execution, self.network) {
            (
                InferenceTrustBoundary::LocalOffline,
                ExecutionTarget::Cpu | ExecutionTarget::Gpu { .. },
                NetworkRequirement::None,
            ) => {}
            (
                InferenceTrustBoundary::RemoteProvider { provider: trusted },
                ExecutionTarget::Remote { provider: target },
                NetworkRequirement::Required,
            ) if trusted == target => validate_text("embedding trusted provider", trusted)?,
            (InferenceTrustBoundary::LocalOffline, _, _) => {
                return invalid("local/offline inference cannot require a remote trust boundary");
            }
            (InferenceTrustBoundary::RemoteProvider { .. }, _, _) => {
                return invalid(
                    "provider inference must match its remote execution and network boundary",
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingJob {
    pub contract_version: u16,
    pub id: RuntimeId,
    pub scope: ScopeId,
    pub read: ReadStamp,
    pub source: RuntimeRef,
    pub expected_source_digest: String,
    pub target: RuntimeRef,
    pub subject: RuntimeRef,
    pub collection: VectorCollectionAddress,
    pub field: String,
    pub valid_from: Millis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<Millis>,
    pub model: EmbeddingModelSpec,
    pub network_policy: NetworkPolicy,
    pub requested_at: Millis,
    #[serde(default)]
    pub properties: RuntimeProperties,
}

impl EmbeddingJob {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != EMBEDDING_CONTRACT_VERSION {
            return invalid("unsupported embedding job contract version");
        }
        self.read.validate()?;
        if self.read.scope != self.scope {
            return invalid("embedding job scope differs from its read stamp");
        }
        validate_digest("embedding source", &self.expected_source_digest)?;
        self.collection.validate()?;
        validate_text("embedding field", &self.field)?;
        if self
            .valid_to
            .is_some_and(|valid_to| valid_to <= self.valid_from)
        {
            return invalid("embedding valid-time window must be half-open and non-empty");
        }
        self.model.validate()
    }

    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        let mut bytes = b"rrflow-embedding-job-v1\0".to_vec();
        bytes.extend_from_slice(&serde_json::to_vec(self).map_err(|error| {
            Error::InvalidRuntime {
                reason: format!("embedding job cannot be encoded: {error}"),
            }
        })?);
        Ok(digest::sha256_hex(&bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingSourceSnapshot {
    pub source: RuntimeRef,
    pub media_type: String,
    pub bytes: Vec<u8>,
    pub digest: String,
}

impl EmbeddingSourceSnapshot {
    pub fn for_bytes(
        source: RuntimeRef,
        media_type: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<Self> {
        let bytes = bytes.into();
        let snapshot = Self {
            source,
            media_type: media_type.into(),
            digest: digest::sha256_hex(&bytes),
            bytes,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<()> {
        validate_text("embedding media type", &self.media_type)?;
        if self.bytes.is_empty() || self.bytes.len() > MAX_EMBEDDING_INPUT_BYTES {
            return invalid("embedding source bytes must be in 1..=67108864");
        }
        validate_digest("embedding source", &self.digest)?;
        if digest::sha256_hex(&self.bytes) != self.digest {
            return invalid("embedding source digest does not match its bytes");
        }
        Ok(())
    }
}

pub trait EmbeddingSourceReader {
    fn read(&mut self, source: &RuntimeRef) -> Result<EmbeddingSourceSnapshot>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingRequest {
    pub job_id: RuntimeId,
    pub job_digest: String,
    pub source_digest: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}

impl EmbeddingRequest {
    pub fn validate(&self) -> Result<()> {
        validate_digest("embedding job", &self.job_digest)?;
        validate_digest("embedding source", &self.source_digest)?;
        validate_text("embedding media type", &self.media_type)?;
        if self.bytes.is_empty() || self.bytes.len() > MAX_EMBEDDING_INPUT_BYTES {
            return invalid("embedding request bytes must be in 1..=67108864");
        }
        if digest::sha256_hex(&self.bytes) != self.source_digest {
            return invalid("embedding request source digest does not match its bytes");
        }
        Ok(())
    }
}
