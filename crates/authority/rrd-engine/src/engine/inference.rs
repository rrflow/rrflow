use super::*;

impl RrdEngine {
    /// Installs one executable adapter into this process authority. Exact
    /// descriptors are public, while credentials and runtime handles remain
    /// process-local and never enter durable RRD state.
    pub fn install_embedding_backend<B>(&self, backend: B) -> Result<u64>
    where
        B: rrd_inference::EmbeddingBackend + Send + 'static,
    {
        self.embedding_backends
            .lock()
            .map_err(|_| ServiceError::Storage("embedding registry lock is poisoned".into()))?
            .install(Box::new(backend))
            .map_err(inference_error)
    }

    pub fn install_feature_hash_embedding(&self, dimensions: u32, seed: u64) -> Result<u64> {
        self.install_embedding_backend(
            rrd_inference::FeatureHashBackend::new(dimensions, seed).map_err(inference_error)?,
        )
    }

    pub fn list_embedding_models(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ListEmbeddingModels,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<EmbeddingModelCatalogue> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::EmbeddingModelList,
            now,
            request_id,
            operation_id,
        )?;
        self.query_scope(&request.scope)?;
        let registry = self
            .embedding_backends
            .lock()
            .map_err(|_| ServiceError::Storage("embedding registry lock is poisoned".into()))?;
        Ok(EmbeddingModelCatalogue {
            scope: request.scope.clone(),
            registry_revision: registry.revision(),
            backends: registry
                .descriptors()
                .iter()
                .map(public_embedding_backend)
                .collect(),
        })
    }

    pub fn generate_embeddings(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &GenerateEmbeddings,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<GenerateEmbeddingsResult> {
        self.generate_embeddings_with_cancellation(
            session_id,
            token,
            request,
            now,
            request_id,
            operation_id,
            &rrd_inference::EmbeddingCancellation::default(),
        )
    }

    /// Embedded caller-owned cancellation. Authorization and the installed
    /// ceiling remain engine-owned; cancellation never extends that ceiling.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_embeddings_with_cancellation(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &GenerateEmbeddings,
        now: u64,
        request_id: &str,
        operation_id: &str,
        cancellation: &rrd_inference::EmbeddingCancellation,
    ) -> Result<GenerateEmbeddingsResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::EmbeddingGenerate,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        self.generate_embeddings_at_with_cancellation(request, read, cancellation)
    }

    pub fn embed_and_search_vectors(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &EmbedAndSearchVectors,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<EmbedAndSearchVectorsResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::EmbeddingSearch,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let descriptor = self.embedding_descriptor(&request.backend_id)?;
        validate_embedding_collection(self, request, &descriptor, scope)?;
        let generated = self.generate_embeddings_at(
            &GenerateEmbeddings {
                scope: request.scope.clone(),
                backend_id: request.backend_id.clone(),
                network_policy: request.network_policy,
                inputs: vec![request.input.clone()],
            },
            read.clone(),
        )?;
        let mut embeddings = generated.embeddings;
        let embedding = embeddings
            .pop()
            .expect("one validated input returns one generated embedding");
        let DataVectorValue::Dense { values } = &embedding.value else {
            return Err(ServiceError::Inference(
                "embed-and-search requires a dense embedding backend".into(),
            ));
        };
        let search = self.search_vectors_at(
            &SearchVectors {
                scope: request.scope.clone(),
                valid_at: request.valid_at,
                collection_id: Some(request.collection_id.clone()),
                vector_name: Some(request.vector_name.clone()),
                field: None,
                query: VectorSearchQuery::Dense {
                    values: values.clone(),
                },
                filter: request.filter.clone(),
                metric: None,
                top_k: request.top_k,
                mode: request.mode,
                max_storage_keys: request.max_storage_keys,
            },
            read,
        )?;
        let result = EmbedAndSearchVectorsResult {
            backend: generated.backend,
            embedding,
            search,
        };
        result
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(result)
    }

    fn embedding_descriptor(
        &self,
        backend_id: &str,
    ) -> Result<rrd_inference::EmbeddingBackendDescriptor> {
        self.embedding_backends
            .lock()
            .map_err(|_| ServiceError::Storage("embedding registry lock is poisoned".into()))?
            .descriptor(backend_id)
            .cloned()
            .ok_or_else(|| {
                ServiceError::Inference(format!("embedding backend {backend_id} is not installed"))
            })
    }

    pub(in crate::engine) fn generate_embeddings_at(
        &self,
        request: &GenerateEmbeddings,
        read: ReadStamp,
    ) -> Result<GenerateEmbeddingsResult> {
        self.generate_embeddings_at_with_cancellation(
            request,
            read,
            &rrd_inference::EmbeddingCancellation::default(),
        )
    }

    fn generate_embeddings_at_with_cancellation(
        &self,
        request: &GenerateEmbeddings,
        read: ReadStamp,
        cancellation: &rrd_inference::EmbeddingCancellation,
    ) -> Result<GenerateEmbeddingsResult> {
        // Bind the sealed estate ceiling once, before input preparation. The
        // adapter cannot reset it by taking longer to validate or execute.
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_millis(
                self.estate_configuration.reasoning.max_step_elapsed_ms,
            ))
            .ok_or(ServiceError::DeadlineExceeded)?;
        let control = rrd_inference::EmbeddingExecutionControl::new(deadline, cancellation.clone());
        self.generate_embeddings_at_with_control(request, read, &control)
    }

    pub(in crate::engine) fn generate_embeddings_at_with_control(
        &self,
        request: &GenerateEmbeddings,
        read: ReadStamp,
        inherited: &rrd_inference::EmbeddingExecutionControl,
    ) -> Result<GenerateEmbeddingsResult> {
        let scope = self.query_scope(&request.scope)?;
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(
                self.estate_configuration.reasoning.max_step_elapsed_ms,
            ))
            .ok_or(ServiceError::DeadlineExceeded)?
            .min(inherited.deadline());
        // The caller owns the cancellation signal and a possibly stricter
        // deadline. Reuse it at every checkpoint; never extend either bound.
        let control = inherited.with_deadline(deadline);
        control.check().map_err(embedding_dispatch_error)?;
        if read.scope != scope {
            return Err(ServiceError::WrongScope);
        }
        let execution = self
            .embedding_backends
            .lock()
            .map_err(|_| ServiceError::Storage("embedding registry lock is poisoned".into()))?
            .try_acquire(&request.backend_id)
            .map_err(embedding_dispatch_error)?;
        let descriptor = execution.descriptor();
        let requests = request
            .inputs
            .iter()
            .map(|input| {
                control.check().map_err(embedding_dispatch_error)?;
                embedding_request(&read, descriptor, input)
            })
            .collect::<Result<Vec<_>>>()?;
        let model = descriptor.model.clone();
        let batch = execution
            .embed_batch_with_control(
                model,
                internal_network_policy(request.network_policy),
                requests,
                control.with_deadline(deadline),
            )
            .map_err(embedding_dispatch_error)?;
        let backend = public_embedding_backend(&batch.backend);
        let embeddings = request
            .inputs
            .iter()
            .zip(batch.vectors.iter())
            .map(|(input, value)| {
                control.check().map_err(embedding_dispatch_error)?;
                let generated = GeneratedEmbedding {
                    input_id: input.id.clone(),
                    value: public_vector_value(value),
                    provenance: DataEmbeddingProvenance {
                        source_sha256: digest::sha256_hex(&input.bytes),
                        model: batch.backend.model.canonical_name(),
                        model_sha256: batch.backend.model.model_digest.clone(),
                        dimensions: batch.backend.model.dimensions,
                        normalization: public_normalization(batch.backend.model.normalization),
                        generation_parameters: DataProperties::from([
                            (
                                "backend".into(),
                                QueryValue::String(batch.backend.id.clone()),
                            ),
                            (
                                "registry_revision".into(),
                                QueryValue::Unsigned(batch.registry_revision),
                            ),
                            (
                                "execution".into(),
                                QueryValue::String(execution_name(&batch.backend.execution)),
                            ),
                            (
                                "trust_boundary".into(),
                                QueryValue::String(trust_name(&batch.backend.trust)),
                            ),
                        ]),
                    },
                };
                generated
                    .validate()
                    .map_err(|error| ServiceError::Contract(error.to_string()))?;
                Ok(generated)
            })
            .collect::<Result<Vec<_>>>()?;
        control.check().map_err(embedding_dispatch_error)?;
        Ok(GenerateEmbeddingsResult {
            scope: request.scope.clone(),
            read_manifest_sha256: read.manifest_id,
            known_at_cursor: read.commit_cursor,
            registry_revision: batch.registry_revision,
            backend,
            embeddings,
        })
    }
}

fn embedding_request(
    read: &ReadStamp,
    descriptor: &rrd_inference::EmbeddingBackendDescriptor,
    input: &rrd_contract::EmbeddingInput,
) -> Result<rrd_inference::EmbeddingRequest> {
    let source_digest = digest::sha256_hex(&input.bytes);
    let job_identity = serde_json::to_vec(&(
        "rrflow-public-embedding-v1",
        &read.scope,
        read.commit_cursor,
        &read.manifest_id,
        descriptor.id.as_str(),
        descriptor.model.model_digest.as_str(),
        input.id.as_str(),
        source_digest.as_str(),
    ))
    .map_err(|error| ServiceError::Contract(error.to_string()))?;
    Ok(rrd_inference::EmbeddingRequest {
        job_id: rrd_core::RuntimeId::new(input.id.as_str()).map_err(super::vector::core_vector)?,
        job_digest: digest::sha256_hex(&job_identity),
        source_digest,
        media_type: input.media_type.clone(),
        bytes: input.bytes.clone(),
    })
}

fn validate_embedding_collection(
    engine: &RrdEngine,
    request: &EmbedAndSearchVectors,
    descriptor: &rrd_inference::EmbeddingBackendDescriptor,
    scope: ScopeId,
) -> Result<()> {
    let catalogue = rrd_vector::VectorCollectionRepository::new(&engine.storage, scope)
        .load()
        .map_err(super::vector::vector_collection_error)?;
    let collection = catalogue
        .collections
        .get(
            &ProjectionId::new(request.collection_id.as_str())
                .map_err(super::vector::core_vector)?,
        )
        .ok_or_else(|| {
            ServiceError::Vector(format!(
                "unknown vector collection {}",
                request.collection_id
            ))
        })?;
    let vector = collection
        .definition
        .vectors
        .get(&ProjectionId::new(request.vector_name.as_str()).map_err(super::vector::core_vector)?)
        .ok_or_else(|| {
            ServiceError::Vector(format!(
                "unknown named vector {} in collection {}",
                request.vector_name, request.collection_id
            ))
        })?;
    let binding = vector.embedding_model.as_ref().ok_or_else(|| {
        ServiceError::Inference(
            "embed-and-search requires a named vector with an exact embedding model binding".into(),
        )
    })?;
    if vector.kind != rrd_vector::VectorValueKind::Dense
        || vector.dimensions != descriptor.model.dimensions
        || binding.name != descriptor.model.canonical_name()
        || binding.digest != descriptor.model.model_digest
    {
        return Err(ServiceError::Inference(
            "embedding backend differs from the named-vector model contract".into(),
        ));
    }
    Ok(())
}

fn public_embedding_backend(
    descriptor: &rrd_inference::EmbeddingBackendDescriptor,
) -> EmbeddingBackendSnapshot {
    EmbeddingBackendSnapshot {
        id: descriptor.id.clone(),
        provider: descriptor.model.provider.clone(),
        model: descriptor.model.model.clone(),
        revision: descriptor.model.revision.clone(),
        model_sha256: descriptor.model.model_digest.clone(),
        modality: match descriptor.model.modality {
            rrd_inference::EmbeddingModality::Text => rrd_contract::EmbeddingModality::Text,
            rrd_inference::EmbeddingModality::Image => rrd_contract::EmbeddingModality::Image,
        },
        dimensions: descriptor.model.dimensions,
        normalization: public_normalization(descriptor.model.normalization),
        execution: match &descriptor.execution {
            rrd_inference::ExecutionTarget::Cpu => EmbeddingExecutionTarget::Cpu,
            rrd_inference::ExecutionTarget::Gpu { platform, device } => {
                EmbeddingExecutionTarget::Gpu {
                    platform: platform.clone(),
                    device: device.clone(),
                }
            }
            rrd_inference::ExecutionTarget::Remote { provider } => {
                EmbeddingExecutionTarget::Remote {
                    provider: provider.clone(),
                }
            }
        },
        network_required: descriptor.network == rrd_inference::NetworkRequirement::Required,
        trust: match &descriptor.trust {
            rrd_inference::InferenceTrustBoundary::LocalOffline => {
                EmbeddingTrustBoundary::LocalOffline
            }
            rrd_inference::InferenceTrustBoundary::RemoteProvider { provider } => {
                EmbeddingTrustBoundary::RemoteProvider {
                    provider: provider.clone(),
                }
            }
        },
        resources: EmbeddingResourceLimits {
            maximum_batch_inputs: descriptor.resources.maximum_batch_inputs,
            maximum_input_bytes: descriptor.resources.maximum_input_bytes,
            maximum_batch_bytes: descriptor.resources.maximum_batch_bytes,
            maximum_output_values: descriptor.resources.maximum_output_values,
        },
        deterministic: descriptor.deterministic,
    }
}

fn public_normalization(value: VectorNormalization) -> DataVectorNormalization {
    match value {
        VectorNormalization::None => DataVectorNormalization::None,
        VectorNormalization::UnitL2 => DataVectorNormalization::UnitL2,
    }
}

fn internal_network_policy(value: EmbeddingNetworkPolicy) -> rrd_inference::NetworkPolicy {
    match value {
        EmbeddingNetworkPolicy::Deny => rrd_inference::NetworkPolicy::Deny,
        EmbeddingNetworkPolicy::Allow => rrd_inference::NetworkPolicy::Allow,
    }
}

fn execution_name(value: &rrd_inference::ExecutionTarget) -> String {
    match value {
        rrd_inference::ExecutionTarget::Cpu => "cpu".into(),
        rrd_inference::ExecutionTarget::Gpu { platform, device } => {
            format!("gpu:{platform}:{device}")
        }
        rrd_inference::ExecutionTarget::Remote { provider } => format!("remote:{provider}"),
    }
}

fn trust_name(value: &rrd_inference::InferenceTrustBoundary) -> String {
    match value {
        rrd_inference::InferenceTrustBoundary::LocalOffline => "local_offline".into(),
        rrd_inference::InferenceTrustBoundary::RemoteProvider { provider } => {
            format!("remote_provider:{provider}")
        }
    }
}

fn inference_error(error: rrd_core::Error) -> ServiceError {
    ServiceError::Inference(error.to_string())
}

pub(super) fn embedding_dispatch_error(
    error: rrd_inference::EmbeddingDispatchError,
) -> ServiceError {
    match error {
        rrd_inference::EmbeddingDispatchError::Busy => ServiceError::InferenceBusy,
        rrd_inference::EmbeddingDispatchError::UnhealthyBackend => {
            ServiceError::Storage("embedding backend lock is poisoned".into())
        }
        rrd_inference::EmbeddingDispatchError::DeadlineExceeded {
            execution_may_continue,
        } => ServiceError::InferenceDeadlineExceeded {
            execution_may_continue,
        },
        rrd_inference::EmbeddingDispatchError::WorkerUnavailable => {
            ServiceError::Storage("embedding worker unavailable".into())
        }
        rrd_inference::EmbeddingDispatchError::Cancelled {
            execution_may_continue,
        } => ServiceError::InferenceCancelled {
            execution_may_continue,
        },
        rrd_inference::EmbeddingDispatchError::Rejected(error) => inference_error(error),
    }
}
