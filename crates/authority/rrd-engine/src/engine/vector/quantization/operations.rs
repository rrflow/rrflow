//! Engine vector quantization operations and admission.

use super::*;

/// How one lifecycle mutation proceeds once its key and digest are known.
pub(super) enum QuantizationAdmission {
    /// The same request was already applied under this key.
    Replay { scope: ScopeId },
    /// A new operation, authorized with an active session.
    Apply {
        scope: ScopeId,
        receipt: rrd_vector::QuantizationOperationReceipt,
        lifecycle: rrd_vector::QuantizationArtifactCatalogue,
    },
}

impl RrdEngine {
    /// Admits one build, activate or retire. Like collection operations, a
    /// recorded operation replays after its session's lease has expired;
    /// a new one needs an active session and is authorized and audited.
    #[allow(clippy::too_many_arguments)]
    fn admit_quantization_operation(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &impl serde::Serialize,
        scope: &str,
        context: &RequestContext,
        action: rrd_vector::QuantizationLifecycleAction,
        now: u64,
    ) -> Result<QuantizationAdmission> {
        context
            .validate(true)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let idempotency_key = context.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::Contract("a quantization lifecycle operation requires idempotency".into())
        })?;
        // The session and its policy are checked before the receipt, and a
        // refusal is audited as `authorize` audits it.
        let mut principal_id = None;
        let admitted = (|| {
            let (_, session) = self.load_authenticated(session_id, token)?;
            principal_id.clone_from(&session.principal_id);
            self.authorize_session_policy(&session, SecurityAction::VectorCollectionEnsure, now)
        })();
        if let Err(error) = admitted {
            self.record_direct_authorization(
                principal_id,
                SecurityAction::VectorCollectionEnsure,
                &self.instance_resource(),
                now,
                context.request_id.as_str(),
                context.operation_id.as_str(),
                Some(&error),
            )?;
            return Err(error);
        }
        let scope = self.query_scope(scope)?;
        let operation_sha256 = operation_digest(request)?;
        let lifecycle = crate::quantization_artifact_catalogue(&self.storage, &scope)
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        if let Some(outcome) = lifecycle.operation(idempotency_key.as_str()) {
            if outcome.operation_sha256 != operation_sha256 || outcome.action != action {
                return Err(ServiceError::IdempotencyConflict);
            }
            return Ok(QuantizationAdmission::Replay { scope });
        }
        self.authorize(
            session_id,
            token,
            SecurityAction::VectorCollectionEnsure,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        Ok(QuantizationAdmission::Apply {
            scope,
            receipt: rrd_vector::QuantizationOperationReceipt {
                idempotency_key: idempotency_key.as_str().into(),
                operation_sha256,
            },
            lifecycle,
        })
    }

    pub fn build_vector_quantization_artifact(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &BuildVectorQuantizationArtifact,
        context: &RequestContext,
        now: u64,
    ) -> Result<BuildVectorQuantizationArtifactResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        match self.admit_quantization_operation(
            session_id,
            token,
            request,
            &request.scope,
            context,
            rrd_vector::QuantizationLifecycleAction::Build,
            now,
        )? {
            QuantizationAdmission::Replay { scope } => {
                self.replay_quantization_build(request, &scope, context)
            }
            QuantizationAdmission::Apply {
                scope,
                receipt,
                lifecycle,
            } => self.build_vector_quantization_artifact_authorized(
                session_id, request, &scope, lifecycle, receipt, context, now,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_vector_quantization_artifact_authorized(
        &self,
        session_id: &CorrelationId,
        request: &BuildVectorQuantizationArtifact,
        scope: &ScopeId,
        lifecycle: rrd_vector::QuantizationArtifactCatalogue,
        receipt: rrd_vector::QuantizationOperationReceipt,
        context: &RequestContext,
        now: u64,
    ) -> Result<BuildVectorQuantizationArtifactResult> {
        let scope = scope.clone();
        let catalogue = rrd_vector::VectorCollectionRepository::new(&self.storage, scope.clone())
            .load()
            .map_err(vector_collection_error)?;
        let collection = catalogue
            .collections
            .get(&ProjectionId::new(request.collection_id.as_str()).map_err(core_vector)?)
            .ok_or_else(|| {
                ServiceError::Vector(format!(
                    "unknown vector collection {}",
                    request.collection_id
                ))
            })?;
        let vector = collection
            .definition
            .vectors
            .get(&ProjectionId::new(request.vector_name.as_str()).map_err(core_vector)?)
            .ok_or_else(|| {
                ServiceError::Vector(format!(
                    "unknown named vector {} in collection {}",
                    request.vector_name, request.collection_id
                ))
            })?;
        if vector.kind != rrd_vector::VectorValueKind::Dense {
            return Err(ServiceError::Vector(
                "quantization artifacts require a dense named vector".into(),
            ));
        }
        for property in &request.filter_properties {
            let field = ProjectionId::new(property.as_str()).map_err(core_vector)?;
            if !collection.payload_indexes.contains_key(&field) {
                return Err(ServiceError::Vector(format!(
                    "quantization filter property {property} has no active payload index in collection {}",
                    request.collection_id
                )));
            }
        }

        let read = self.storage.runtime().read_stamp(&scope)?;
        let in_source = |candidate: &rrd_vector::VectorCandidate| {
            vector_matches_collection(
                &candidate.vector,
                &request.collection_id,
                &request.vector_name,
            )
        };
        // What this source holds of every vector version in the scope, for a
        // full build.
        let read_everything = || -> Result<(rrd_vector::SourceHistory, VersionsRead)> {
            let direct = read_vector_versions(self, &read, request.max_storage_keys)?;
            let history = rrd_vector::source_history(
                rrd_vector::candidates_from_changes(&direct.changes, &scope),
                in_source,
            );
            Ok((
                history,
                VersionsRead {
                    selected_versions: direct.selected_versions,
                    read_evidence: direct.read_evidence,
                },
            ))
        };
        let method_name = match request.method {
            VectorQuantizationMethod::Scalar => "scalar",
            VectorQuantizationMethod::Product { .. } => "product",
            VectorQuantizationMethod::Binary => "binary",
            VectorQuantizationMethod::TurboQuant { .. } => "turboquant",
        };
        let artifact_id =
            quantization_artifact_id(method_name, &request.collection_id, &request.vector_name)?;
        let generation = lifecycle
            .artifacts
            .keys()
            .filter(|(id, _)| id == &artifact_id)
            .map(|(_, generation)| *generation)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| ServiceError::Vector("quantization generation overflowed".into()))?;
        let dimensions = usize::try_from(vector.dimensions)
            .map_err(|_| ServiceError::Vector("vector dimensions exceed usize".into()))?;
        let filter_properties = request
            .filter_properties
            .iter()
            .map(|property| property.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let (artifact, versions, base_generation, encoded_versions): (
            rrd_vector::VectorArtifact,
            VersionsRead,
            Option<u64>,
            usize,
        ) = match request.method {
            VectorQuantizationMethod::Scalar
            | VectorQuantizationMethod::Product { .. }
            | VectorQuantizationMethod::Binary => {
                let (history, versions) = read_everything()?;
                let encoded = history.versions.len();
                let artifact = rrd_vector::QuantizedSegment::build(
                    rrd_vector::QuantizedSegmentConfig {
                        id: artifact_id.clone(),
                        scope: scope.clone(),
                        field: vector.field.clone(),
                        dimensions,
                        metric: vector.metric,
                        method: internal_quantization_method(&request.method)?,
                        embedding_model: vector.embedding_model.clone(),
                        filter_properties,
                    },
                    generation,
                    history.source_cursor,
                    history.versions,
                )
                .map(Into::into)
                .map_err(core_vector)?;
                (artifact, versions, None, encoded)
            }
            VectorQuantizationMethod::TurboQuant {
                bits,
                seed,
                estimator,
            } => {
                let config = rrd_vector::TurboQuantSegmentConfig {
                    id: artifact_id.clone(),
                    scope: scope.clone(),
                    field: vector.field.clone(),
                    dimensions,
                    metric: vector.metric,
                    bits: internal_turboquant_bits(bits),
                    seed,
                    embedding_model: vector.embedding_model.clone(),
                    filter_properties,
                    estimator: match estimator {
                        rrd_contract::VectorQuantizationEstimator::Mse => {
                            rrd_vector::TurboQuantEstimator::Mse
                        }
                        rrd_contract::VectorQuantizationEstimator::InnerProduct => {
                            rrd_vector::TurboQuantEstimator::InnerProduct
                        }
                    },
                };
                match self.turboquant_base(&lifecycle, &artifact_id, &config)? {
                    // The base's versions and codes stand; only the identities
                    // changed in this source since it are read and encoded.
                    Some(base) => {
                        let source = rrd_store::VectorSourceAddress {
                            collection: rrd_core::VectorCollectionAddress {
                                collection_id: request.collection_id.as_str().into(),
                                vector_name: request.vector_name.as_str().into(),
                            },
                            field: vector.field.clone(),
                        };
                        let deltas = self
                            .storage
                            .runtime()
                            .read_vector_deltas(
                                &read,
                                &source,
                                base.descriptor().stamp.source_cursor,
                                rrd_store::RuntimeReadBudget::new(request.max_storage_keys)
                                    .map_err(|error| ServiceError::Vector(error.to_string()))?,
                            )
                            .map_err(crate::engine::error::storage_read_error)?;
                        let replaced = deltas
                            .deltas
                            .iter()
                            .map(|delta| delta.reference.clone())
                            .collect::<BTreeSet<_>>();
                        // The deltas hold each changed identity's complete
                        // history, so a move out resolves as a full build
                        // resolves it.
                        let history = rrd_vector::source_history(
                            rrd_vector::candidates_from_changes(&deltas.changes, &scope),
                            in_source,
                        );
                        let encoded = history.versions.len();
                        let versions = VersionsRead {
                            selected_versions: u64::try_from(deltas.changes.len()).map_err(
                                |_| {
                                    ServiceError::Vector(
                                        "selected vector versions exceed u64".into(),
                                    )
                                },
                            )?,
                            read_evidence: crate::runtime::public_read_evidence(deltas.evidence)?,
                        };
                        // The base covers every unchanged identity's
                        // versions and exits, so with the changed histories'
                        // cursor this is the cursor a full build covers.
                        let source_cursor = base
                            .descriptor()
                            .stamp
                            .source_cursor
                            .max(history.source_cursor);
                        let artifact = rrd_vector::TurboQuantSegment::extend(
                            &base,
                            config,
                            generation,
                            &replaced,
                            history.versions,
                            source_cursor,
                        )
                        .map(Into::into)
                        .map_err(core_vector)?;
                        (
                            artifact,
                            versions,
                            Some(base.descriptor().stamp.generation),
                            encoded,
                        )
                    }
                    None => {
                        let (history, versions) = read_everything()?;
                        let encoded = history.versions.len();
                        let artifact = rrd_vector::TurboQuantSegment::build(
                            config,
                            generation,
                            history.source_cursor,
                            history.versions,
                        )
                        .map(Into::into)
                        .map_err(core_vector)?;
                        (artifact, versions, None, encoded)
                    }
                }
            }
        };
        let data = rrd_store::DataRuntimeRef::new(&self.storage, &self.objects);
        let publication = match crate::build_traced_quantization_artifact(
            &data,
            ProjectionId::new(request.collection_id.as_str()).map_err(core_vector)?,
            ProjectionId::new(request.vector_name.as_str()).map_err(core_vector)?,
            artifact,
            &format!("session:{}", session_id.as_str()),
            now,
            Some(receipt.clone()),
        ) {
            Ok(publication) => publication,
            Err(error) => {
                return match self.settle_failed_publication(&scope, &receipt, error)? {
                    crate::QuantizationOperationRecorded::Replay => {
                        self.replay_quantization_build(request, &scope, context)
                    }
                    crate::QuantizationOperationRecorded::Conflict => {
                        Err(ServiceError::IdempotencyConflict)
                    }
                };
            }
        };
        Ok(BuildVectorQuantizationArtifactResult {
            artifact: public_quantization_artifact(
                &publication.entry,
                publication.state,
                publication.lifecycle_revision,
            )?,
            selected_versions: versions.selected_versions,
            read_evidence: versions.read_evidence,
            base_generation,
            encoded_versions: u64::try_from(encoded_versions)
                .map_err(|_| ServiceError::Vector("encoded vector versions exceed u64".into()))?,
            idempotent_replay: false,
        })
    }

    /// The base a TurboQuant build extends: the newest published generation
    /// of `artifact_id` whose configuration is `config`, decoded from its
    /// object through the binding serving uses, which verifies the bytes
    /// against the published descriptor. Without one, or when its object
    /// does not verify, the build encodes every version instead.
    fn turboquant_base(
        &self,
        lifecycle: &rrd_vector::QuantizationArtifactCatalogue,
        artifact_id: &ProjectionId,
        config: &rrd_vector::TurboQuantSegmentConfig,
    ) -> Result<Option<rrd_vector::TurboQuantSegment>> {
        let digest = config.digest().map_err(core_vector)?;
        let Some(entry) = lifecycle
            .artifacts
            .iter()
            .rev()
            .filter(|((id, _), _)| id == artifact_id)
            .map(|(_, artifact)| &artifact.entry)
            .find(|entry| {
                entry.kind == rrd_vector::VectorArtifactKind::TurboQuant
                    && entry.descriptor.stamp().config_digest == digest
            })
        else {
            return Ok(None);
        };
        let binding = crate::VectorArtifactBinding {
            kind: entry.kind,
            descriptor: entry.descriptor.clone(),
            object: entry.object.clone(),
            source: Some(rrd_core::VectorCollectionAddress {
                collection_id: entry.collection_id.to_string(),
                vector_name: entry.vector_name.to_string(),
            }),
        };
        let decoded = match binding.decode_mapped(&self.objects) {
            Ok(Some(artifact)) => Ok(artifact),
            Ok(None) => binding.decode_owned(&self.objects),
            Err(error) => Err(error),
        };
        Ok(match decoded.as_deref() {
            Ok(rrd_vector::VectorArtifact::TurboQuant(segment)) => Some(segment.clone()),
            _ => None,
        })
    }

    pub fn list_vector_quantization_artifacts(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ListVectorQuantizationArtifacts,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<ListVectorQuantizationArtifactsResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::VectorCollectionList,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let catalogue = crate::quantization_artifact_catalogue(&self.storage, &scope)
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        let limit = usize::try_from(request.max_artifacts)
            .map_err(|_| ServiceError::Vector("quantization list bound exceeds usize".into()))?;
        let artifacts = catalogue
            .artifacts
            .values()
            .filter(|artifact| {
                request.collection_id.as_ref().is_none_or(|collection| {
                    artifact.entry.collection_id.as_str() == collection.as_str()
                }) && request
                    .vector_name
                    .as_ref()
                    .is_none_or(|vector| artifact.entry.vector_name.as_str() == vector.as_str())
            })
            .take(limit)
            .map(|artifact| {
                public_quantization_artifact(&artifact.entry, artifact.state, catalogue.revision)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(ListVectorQuantizationArtifactsResult {
            scope: request.scope.clone(),
            lifecycle_revision: catalogue.revision,
            artifacts,
        })
    }

    pub fn activate_vector_quantization_artifact(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ActivateVectorQuantizationArtifact,
        context: &RequestContext,
        now: u64,
    ) -> Result<ActivateVectorQuantizationArtifactResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        self.transition_quantization_artifact(
            session_id,
            token,
            request,
            (&request.scope, &request.artifact_id, request.generation),
            context,
            rrd_vector::QuantizationLifecycleAction::Activate,
            now,
        )
        .map(
            |(artifact, idempotent_replay)| ActivateVectorQuantizationArtifactResult {
                artifact,
                idempotent_replay,
            },
        )
    }

    pub fn retire_vector_quantization_artifact(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &RetireVectorQuantizationArtifact,
        context: &RequestContext,
        now: u64,
    ) -> Result<RetireVectorQuantizationArtifactResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        self.transition_quantization_artifact(
            session_id,
            token,
            request,
            (&request.scope, &request.artifact_id, request.generation),
            context,
            rrd_vector::QuantizationLifecycleAction::Retire,
            now,
        )
        .map(
            |(artifact, idempotent_replay)| RetireVectorQuantizationArtifactResult {
                artifact,
                idempotent_replay,
            },
        )
    }

    /// Activates or retires one generation, or replays the recorded
    /// operation, and returns the artifact with whether it was a replay.
    #[allow(clippy::too_many_arguments)]
    fn transition_quantization_artifact(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &impl serde::Serialize,
        (scope, artifact_id, generation): (&str, &CanonicalId, u64),
        context: &RequestContext,
        action: rrd_vector::QuantizationLifecycleAction,
        now: u64,
    ) -> Result<(VectorQuantizationArtifactSnapshot, bool)> {
        let (scope, receipt) = match self
            .admit_quantization_operation(session_id, token, request, scope, context, action, now)?
        {
            QuantizationAdmission::Replay { scope } => {
                return self
                    .replay_quantization_transition(&scope, context)
                    .map(|artifact| (artifact, true));
            }
            QuantizationAdmission::Apply { scope, receipt, .. } => (scope, receipt),
        };
        let data = rrd_store::DataRuntimeRef::new(&self.storage, &self.objects);
        match crate::transition_traced_quantization_artifact(
            &data,
            &scope,
            &ProjectionId::new(artifact_id.as_str()).map_err(core_vector)?,
            generation,
            action,
            &format!("session:{}", session_id.as_str()),
            now,
            Some(receipt.clone()),
        ) {
            Ok(transition) => Ok((
                public_quantization_artifact(
                    &transition.entry,
                    transition.state,
                    transition.lifecycle_revision,
                )?,
                false,
            )),
            Err(error) => match self.settle_failed_publication(&scope, &receipt, error)? {
                crate::QuantizationOperationRecorded::Replay => self
                    .replay_quantization_transition(&scope, context)
                    .map(|artifact| (artifact, true)),
                crate::QuantizationOperationRecorded::Conflict => {
                    Err(ServiceError::IdempotencyConflict)
                }
            },
        }
    }

    /// A publication that failed because its key was already recorded at
    /// its commit's read, or because its conditional commit lost to a
    /// concurrent request with the same key, replays or conflicts. Every
    /// other failure is returned as it is.
    fn settle_failed_publication(
        &self,
        scope: &ScopeId,
        receipt: &rrd_vector::QuantizationOperationReceipt,
        error: Box<dyn std::error::Error>,
    ) -> Result<crate::QuantizationOperationRecorded> {
        if let Some(recorded) = error
            .downcast_ref::<crate::QuantizationOperationRecorded>()
            .copied()
        {
            return Ok(recorded);
        }
        let lifecycle = crate::quantization_artifact_catalogue(&self.storage, scope)
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        match lifecycle.operation(&receipt.idempotency_key) {
            Some(outcome) if outcome.operation_sha256 == receipt.operation_sha256 => {
                Ok(crate::QuantizationOperationRecorded::Replay)
            }
            Some(_) => Ok(crate::QuantizationOperationRecorded::Conflict),
            None => Err(ServiceError::Vector(error.to_string())),
        }
    }

    /// The artifact a recorded build published, as it is now, with the
    /// evidence of the catalogue read the replay made under the request's
    /// key budget; no vector is read and nothing is encoded.
    fn replay_quantization_build(
        &self,
        request: &BuildVectorQuantizationArtifact,
        scope: &ScopeId,
        context: &RequestContext,
    ) -> Result<BuildVectorQuantizationArtifactResult> {
        let read = self.storage.runtime().read_stamp(scope)?;
        let direct = crate::runtime::vector_catalog::read_vector_catalogue_versions(
            &self.storage,
            &read,
            &[],
            rrd_store::RuntimeReadBudget::new(request.max_storage_keys)?,
        )
        .map_err(crate::engine::error::storage_read_error)?;
        let catalogue =
            crate::runtime::vector_catalog::quantization_artifact_catalogue_from_changes(
                &direct.changes,
                scope,
            )
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        Ok(BuildVectorQuantizationArtifactResult {
            artifact: recorded_artifact(&catalogue, context)?,
            selected_versions: u64::try_from(direct.changes.len())
                .map_err(|_| ServiceError::Vector("selected versions exceed u64".into()))?,
            read_evidence: crate::runtime::public_read_evidence(direct.evidence)?,
            base_generation: None,
            encoded_versions: 0,
            idempotent_replay: true,
        })
    }

    /// The artifact a recorded activation or retirement transitioned, as it
    /// is now.
    fn replay_quantization_transition(
        &self,
        scope: &ScopeId,
        context: &RequestContext,
    ) -> Result<VectorQuantizationArtifactSnapshot> {
        let catalogue = crate::quantization_artifact_catalogue(&self.storage, scope)
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        recorded_artifact(&catalogue, context)
    }
}
