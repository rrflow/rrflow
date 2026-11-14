//! Prepared embeddings and the embedding coordinator.

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedEmbedding {
    pub contract_version: u16,
    pub job_id: RuntimeId,
    pub job_digest: String,
    pub scope: ScopeId,
    pub backend: EmbeddingBackendDescriptor,
    pub vector: RuntimeVector,
}

impl PreparedEmbedding {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != EMBEDDING_CONTRACT_VERSION {
            return invalid("unsupported prepared embedding contract version");
        }
        validate_digest("embedding job", &self.job_digest)?;
        self.backend.validate()?;
        self.vector.validate()?;
        let provenance = self
            .vector
            .provenance
            .as_ref()
            .ok_or_else(|| Error::InvalidRuntime {
                reason: "prepared embedding has no provenance".into(),
            })?;
        if provenance.model != self.backend.model.canonical_name()
            || provenance.model_digest != self.backend.model.model_digest
            || provenance.dimensions != self.backend.model.dimensions
            || provenance.normalization != self.backend.model.normalization
        {
            return invalid("prepared vector provenance differs from its backend model");
        }
        Ok(())
    }

    pub fn into_mutation(self) -> RuntimeMutation {
        RuntimeMutation::Vector {
            vector: self.vector,
        }
    }

    pub fn transaction(
        &self,
        job: &EmbeddingJob,
        actor: impl Into<String>,
        at: Millis,
    ) -> Result<DataTransaction> {
        self.validate()?;
        job.validate()?;
        if job.digest()? != self.job_digest
            || self.job_id != job.id
            || self.scope != job.scope
            || self.vector.reference != job.target
            || self.vector.subject != job.subject
            || self.vector.collection != job.collection
            || self.vector.field != job.field
        {
            return invalid("prepared embedding differs from its source job");
        }
        if at < job.requested_at {
            return invalid("embedding commit time precedes its request time");
        }
        DataTransaction::new(
            job.read.clone(),
            RuntimeCommit {
                scope: job.scope.clone(),
                at,
                actor: actor.into(),
                expected_cursor: job.read.commit_cursor,
                mutations: vec![RuntimeMutation::Vector {
                    vector: self.vector.clone(),
                }],
            },
        )
    }
}

pub struct EmbeddingCoordinator;

impl EmbeddingCoordinator {
    pub fn prepare<S: EmbeddingSourceReader, B: EmbeddingBackend>(
        job: &EmbeddingJob,
        source_reader: &mut S,
        backend: &mut B,
    ) -> Result<PreparedEmbedding> {
        job.validate()?;
        let job_digest = job.digest()?;
        let descriptor = backend.descriptor().clone();
        descriptor.validate()?;
        if descriptor.model != job.model {
            return invalid("embedding backend model differs from the requested model");
        }
        if job.network_policy == NetworkPolicy::Deny
            && descriptor.network == NetworkRequirement::Required
        {
            return invalid("embedding job denies the network required by its backend");
        }

        let before = source_reader.read(&job.source)?;
        validate_source(job, &before)?;
        let request = EmbeddingRequest {
            job_id: job.id.clone(),
            job_digest: job_digest.clone(),
            source_digest: before.digest.clone(),
            media_type: before.media_type.clone(),
            bytes: before.bytes,
        };
        request.validate()?;
        let mut values =
            dispatch_embedding_batch(backend, job.network_policy, std::slice::from_ref(&request))?;
        let value = values
            .pop()
            .expect("one validated embedding request returns one vector");

        // The source is sampled again after inference. A transaction-level CAS
        // remains the final commit authority, but this closes the expensive
        // inference race before a vector can even enter a commit.
        let after = source_reader.read(&job.source)?;
        after.validate()?;
        if after.source != job.source {
            return invalid("embedding source reader returned the wrong identity");
        }
        if after.digest != request.source_digest {
            return invalid("embedding source changed during inference");
        }
        validate_source(job, &after)?;

        let mut generation_parameters = RuntimeProperties::new();
        generation_parameters.insert(
            "backend".into(),
            RuntimeValue::String(descriptor.id.clone()),
        );
        generation_parameters.insert(
            "execution".into(),
            RuntimeValue::String(execution_name(&descriptor.execution)),
        );
        generation_parameters.insert(
            "job_digest".into(),
            RuntimeValue::Digest(job_digest.clone()),
        );
        generation_parameters.insert(
            "deterministic".into(),
            RuntimeValue::Bool(descriptor.deterministic),
        );
        let vector = RuntimeVector {
            reference: job.target.clone(),
            subject: job.subject.clone(),
            collection: job.collection.clone(),
            field: job.field.clone(),
            valid_from: job.valid_from,
            valid_to: job.valid_to,
            value,
            provenance: Some(EmbeddingProvenance {
                source_digest: request.source_digest,
                model: descriptor.model.canonical_name(),
                model_digest: descriptor.model.model_digest.clone(),
                dimensions: descriptor.model.dimensions,
                normalization: descriptor.model.normalization,
                generation_parameters,
            }),
            properties: job.properties.clone(),
        };
        let prepared = PreparedEmbedding {
            contract_version: EMBEDDING_CONTRACT_VERSION,
            job_id: job.id.clone(),
            job_digest,
            scope: job.scope.clone(),
            backend: descriptor,
            vector,
        };
        prepared.validate()?;
        Ok(prepared)
    }
}
