//! Engine vector search operations.

use super::*;

impl RrdEngine {
    pub fn search_vectors(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &SearchVectors,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<VectorSearchResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::VectorSearch,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        self.search_vectors_at(request, read)
    }

    pub(in crate::engine) fn search_vectors_at(
        &self,
        request: &SearchVectors,
        read: ReadStamp,
    ) -> Result<VectorSearchResult> {
        let scope = self.query_scope(&request.scope)?;
        if read.scope != scope {
            return Err(ServiceError::WrongScope);
        }
        // Capture the shared data/catalogue stamp before loading collection
        // metadata. The subsequent runtime read validates the same stamp and
        // rejects a concurrent catalogue transition instead of combining old
        // collection metadata with a new read manifest.
        let (field, metric, embedding_model, collection_address, memory_tier) =
            if let (Some(collection_id), Some(vector_name)) =
                (&request.collection_id, &request.vector_name)
            {
                let catalogue =
                    rrd_vector::VectorCollectionRepository::new(&self.storage, scope.clone())
                        .load()
                        .map_err(vector_collection_error)?;
                let collection = catalogue
                    .collections
                    .get(&ProjectionId::new(collection_id.as_str()).map_err(core_vector)?)
                    .ok_or_else(|| {
                        ServiceError::Vector(format!("unknown vector collection {collection_id}"))
                    })?;
                let vector = collection
                    .definition
                    .vectors
                    .get(&ProjectionId::new(vector_name.as_str()).map_err(core_vector)?)
                    .ok_or_else(|| {
                        ServiceError::Vector(format!(
                            "unknown named vector {vector_name} in collection {collection_id}"
                        ))
                    })?;
                validate_collection_query(vector, &request.query)?;
                (
                    vector.field.clone(),
                    vector.metric,
                    vector.embedding_model.clone(),
                    Some((collection_id.clone(), vector_name.clone())),
                    vector.memory_tier,
                )
            } else {
                let field = request
                    .field
                    .as_ref()
                    .ok_or_else(|| ServiceError::Vector("vector field is absent".into()))?;
                let metric = request
                    .metric
                    .ok_or_else(|| ServiceError::Vector("vector metric is absent".into()))?;
                (
                    field.as_str().to_owned(),
                    internal_vector_metric(metric),
                    None,
                    None,
                    rrd_vector::VectorMemoryTier::Cached,
                )
            };
        let (mode, ef_search) = internal_search_mode(request.mode)?;
        let search = rrd_vector::SearchRequest {
            scope: scope.clone(),
            read: read.clone(),
            valid_at: request.valid_at,
            field: field.clone(),
            query: internal_vector_query(&request.query),
            metric,
            embedding_model,
            top_k: usize::try_from(request.top_k)
                .map_err(|_| ServiceError::Vector("vector top_k exceeds usize".into()))?,
            mode,
            filter: request
                .filter
                .as_ref()
                .map(internal_vector_filter)
                .transpose()?,
        };
        // A request that requires an approximate path on a named dense vector
        // reads the stamped catalogue first, so an active generation can serve
        // it without reading every vector version, and a request nothing can
        // serve is refused before reading any.
        let catalogue = match (&collection_address, &search.mode, &search.query) {
            (
                Some(_),
                rrd_vector::SearchMode::RequireApproximate { .. },
                rrd_vector::VectorQuery::Dense { .. },
            ) => Some(
                crate::runtime::vector_catalog::read_vector_catalogue_versions(
                    &self.storage,
                    &read,
                    &[],
                    rrd_store::RuntimeReadBudget::new(request.max_storage_keys)?,
                )
                .map_err(crate::engine::error::storage_read_error)?,
            ),
            _ => None,
        };
        if let (Some(catalogue), Some((collection_id, vector_name))) =
            (&catalogue, &collection_address)
        {
            if let Some(result) = self.search_active_generation(
                request,
                &read,
                &search,
                (collection_id, vector_name),
                memory_tier,
                ef_search,
                catalogue,
            )? {
                return Ok(result);
            }
        }
        let direct = match catalogue {
            None => read_vector_search_versions(self, &read, request.max_storage_keys)?,
            // The catalogue is already read: read the vector versions under
            // what remains of the one budget, in the combined read's order.
            Some(catalogue) => {
                let vectors = self
                    .storage
                    .runtime()
                    .read_versioned(
                        &read,
                        &[rrd_store::RuntimeVersionedSource::Vectors { kind: None }],
                        remaining_budget(
                            request.max_storage_keys,
                            catalogue.evidence.keys_examined,
                        )?,
                    )
                    .map_err(crate::engine::error::storage_read_error)?;
                let read_evidence = crate::runtime::merge_read_evidence(
                    request.max_storage_keys,
                    [
                        crate::runtime::public_read_evidence(catalogue.evidence)?,
                        crate::runtime::public_read_evidence(vectors.evidence)?,
                    ],
                )?;
                let mut changes = vectors.changes;
                changes.extend(catalogue.changes);
                DirectVectorRead {
                    selected_versions: u64::try_from(changes.len()).map_err(|_| {
                        ServiceError::Vector("selected vector versions exceed u64".into())
                    })?,
                    changes,
                    read_evidence,
                }
            }
        };
        let candidates = rrd_vector::candidates_from_changes(&direct.changes, &scope);
        let candidates = match &collection_address {
            Some((collection, vector)) => {
                collection_versions(&search_visibility(&search), candidates, collection, vector)?
            }
            None => candidates,
        };
        let canonical_candidates = u64::try_from(candidates.len())
            .map_err(|_| ServiceError::Vector("vector candidate count exceeds u64".into()))?;
        let source_cursor = candidates
            .iter()
            .filter(|candidate| candidate.vector.field == field)
            .map(|candidate| candidate.source_cursor)
            .max()
            .unwrap_or(0);
        let crate::VectorRuntimeManifest {
            mut runtime,
            bindings,
        } = crate::runtime::vector_catalog::vector_runtime_metadata_from_changes(
            &direct.changes,
            &scope,
            candidates,
        )
        .map_err(|error| ServiceError::Vector(error.to_string()))?;
        let active = bindings
            .values()
            .map(crate::VectorArtifactBinding::key)
            .collect::<std::collections::BTreeSet<_>>();
        let lifecycle =
            crate::runtime::vector_catalog::quantization_artifact_catalogue_from_changes(
                &direct.changes,
                &scope,
            )
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        let mut first_pressure = None;
        let mut cold_source_hits = 0u64;
        {
            let mut residency = self
                .vector_residency
                .lock()
                .map_err(|_| ServiceError::Storage("vector residency lock is poisoned".into()))?;
            residency.reconcile(&active);
            for binding in bindings
                .values()
                .filter(|binding| vector_artifact_relevant(&binding.descriptor, &search))
            {
                if let Some((collection, vector)) = &collection_address {
                    let source = rrd_core::VectorCollectionAddress {
                        collection_id: collection.as_str().into(),
                        vector_name: vector.as_str().into(),
                    };
                    if binding.descriptor.candidate_path(1).kind.is_approximate()
                        && binding.source.as_ref() != Some(&source)
                    {
                        // A legacy unbound generation, or one belonging to a
                        // different source, is not an authorized named path.
                        runtime.suppress_projection(&binding.descriptor.stamp().id);
                        continue;
                    }
                }
                match residency.acquire(memory_tier, binding, &self.objects) {
                    Ok(acquisition) => {
                        if collection_address.as_ref().is_some_and(|(id, name)| {
                            !generation_serves(
                                &lifecycle,
                                binding,
                                &acquisition.artifact,
                                (id, name),
                            )
                        }) {
                            runtime.suppress_projection(&binding.descriptor.stamp().id);
                            continue;
                        }
                        if acquisition.source.is_cold_source() {
                            self.record_rrflow_cld_vector_trigger(
                                acquisition.source,
                                request.valid_at,
                            )?;
                            cold_source_hits = cold_source_hits.saturating_add(1);
                        }
                        runtime
                            .install_loaded_artifact(acquisition.artifact)
                            .map_err(core_vector)?;
                    }
                    Err(error) if error.is_pressure() => {
                        first_pressure.get_or_insert_with(|| error.to_string());
                        runtime.suppress_projection(&binding.descriptor.stamp().id);
                    }
                    Err(error) => return Err(ServiceError::Vector(error.to_string())),
                }
            }
        }
        let prepared = match runtime.prepare_search_at(&search, source_cursor, ef_search) {
            Ok(prepared) => prepared,
            Err(_error)
                if matches!(
                    search.mode,
                    rrd_vector::SearchMode::RequireApproximate { .. }
                ) && first_pressure.is_some() =>
            {
                return Err(ServiceError::VectorPressure(
                    first_pressure.expect("pressure was checked as present"),
                ));
            }
            Err(error) => return Err(core_vector(error)),
        };
        let execution = runtime
            .execute_search(&search, &prepared)
            .map_err(core_vector)?;
        let loaded_artifact_bytes = bindings
            .get(&(
                execution.plan.selected.id.clone(),
                execution.plan.selected.generation,
            ))
            .map_or(0, |binding| binding.object.length);
        search_result(
            request,
            read,
            Searched {
                selected_versions: direct.selected_versions,
                read_evidence: direct.read_evidence,
                canonical_candidates,
                loaded_artifact_bytes,
                cold_source_hits,
            },
            &prepared,
            execution,
        )
    }

    /// Serves a request that requires an approximate path on a named dense
    /// vector from that vector's active generation and the stamped source
    /// delta since it, when that generation is the request's only eligible
    /// approximate path. It reads only the identities the delta names and the
    /// generation's own candidates, and reranks every one exactly through the
    /// same planner and execution as every search. A request no active
    /// generation can serve is refused here, having read no vector. `None`
    /// leaves the request to the general path.
    #[allow(clippy::too_many_arguments)]
    fn search_active_generation(
        &self,
        request: &SearchVectors,
        read: &ReadStamp,
        search: &rrd_vector::SearchRequest,
        (collection_id, vector_name): (&CanonicalId, &CanonicalId),
        memory_tier: rrd_vector::VectorMemoryTier,
        ef_search: usize,
        catalogue: &rrd_store::RuntimeVersionedRead,
    ) -> Result<Option<VectorSearchResult>> {
        let rrd_vector::SearchMode::RequireApproximate { exact_rerank } = search.mode else {
            return Ok(None);
        };
        let scope = &read.scope;
        let address = (collection_id, vector_name);
        let manifest = crate::runtime::vector_catalog::vector_runtime_metadata_from_changes(
            &catalogue.changes,
            scope,
            Vec::new(),
        )
        .map_err(|error| ServiceError::Vector(error.to_string()))?;
        let refuse = |runtime: &rrd_vector::VectorRuntime| {
            // The planner's own refusal, from the same descriptors. A path it would still accept
            // is left to the general path, so this can only ever refuse what that would.
            runtime
                .prepare_search_at(search, 0, ef_search)
                .map(|_| None)
                .map_err(core_vector)
        };
        let mut eligible = manifest
            .bindings
            .values()
            .filter(|binding| vector_artifact_relevant(&binding.descriptor, search));
        let binding = match (eligible.next(), eligible.next()) {
            (None, _) => return refuse(&manifest.runtime),
            (Some(binding), None) => binding,
            (Some(_), Some(_)) => return Ok(None),
        };
        if !matches!(
            binding.kind,
            rrd_vector::VectorArtifactKind::TurboQuant | rrd_vector::VectorArtifactKind::Hnsw
        ) {
            return Ok(None);
        }
        let acquisition = {
            let mut residency = self
                .vector_residency
                .lock()
                .map_err(|_| ServiceError::Storage("vector residency lock is poisoned".into()))?;
            residency.reconcile(
                &manifest
                    .bindings
                    .values()
                    .map(crate::VectorArtifactBinding::key)
                    .collect(),
            );
            match residency.acquire(memory_tier, binding, &self.objects) {
                Ok(acquisition) => acquisition,
                Err(error) if error.is_pressure() => {
                    return Err(ServiceError::VectorPressure(error.to_string()));
                }
                Err(error) => return Err(ServiceError::Vector(error.to_string())),
            }
        };
        if acquisition.source.is_cold_source() {
            self.record_rrflow_cld_vector_trigger(acquisition.source, request.valid_at)?;
        }
        let artifact = acquisition.artifact;
        let lifecycle =
            crate::runtime::vector_catalog::quantization_artifact_catalogue_from_changes(
                &catalogue.changes,
                scope,
            )
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        if !generation_serves(&lifecycle, binding, &artifact, address) {
            let mut runtime = manifest.runtime;
            runtime.suppress_projection(&binding.descriptor.stamp().id);
            return refuse(&runtime);
        }
        let generation = binding.descriptor.stamp().source_cursor;

        // Every identity whose version in this source changed after the
        // generation, with its complete authenticated history.
        let budget = request.max_storage_keys;
        let mut examined = catalogue.evidence.keys_examined;
        let source = rrd_store::VectorSourceAddress {
            collection: rrd_core::VectorCollectionAddress {
                collection_id: collection_id.as_str().into(),
                vector_name: vector_name.as_str().into(),
            },
            field: search.field.clone(),
        };
        let deltas = self
            .storage
            .runtime()
            .read_vector_deltas(
                read,
                &source,
                generation,
                remaining_budget(budget, examined)?,
            )
            .map_err(crate::engine::error::storage_read_error)?;
        examined = examined.saturating_add(deltas.evidence.keys_examined);
        let changed = deltas
            .deltas
            .iter()
            .map(|delta| delta.reference.clone())
            .collect::<std::collections::BTreeSet<_>>();
        // The generation's own candidates at its cursor, with the planner's
        // rerank width; those unchanged since it are read by identity.
        let proposed = match artifact.as_ref() {
            rrd_vector::VectorArtifact::TurboQuant(segment) => segment
                .search_candidates_at(search, exact_rerank, generation)
                .map_err(core_vector)?
                .into_iter()
                .map(|hit| hit.reference)
                .collect::<Vec<_>>(),
            rrd_vector::VectorArtifact::Hnsw(index) => index
                .search_candidates(search, ef_search)
                .map_err(core_vector)?
                .into_iter()
                .map(|candidate| candidate.vector.reference)
                .collect(),
            _ => {
                return Err(ServiceError::Vector(
                    "the active generation's binding loaded another artifact".into(),
                ))
            }
        };
        let unchanged = proposed
            .into_iter()
            .filter(|reference| !changed.contains(reference))
            .collect::<std::collections::BTreeSet<_>>();
        let rerank = if unchanged.is_empty() {
            None
        } else {
            Some(
                self.storage
                    .runtime()
                    .read_versioned(
                        read,
                        &unchanged
                            .into_iter()
                            .map(|reference| rrd_store::RuntimeVersionedSource::Identity {
                                model: rrd_core::RuntimeLogicalModel::Vector,
                                reference,
                            })
                            .collect::<Vec<_>>(),
                        remaining_budget(budget, examined)?,
                    )
                    .map_err(crate::engine::error::storage_read_error)?,
            )
        };
        // Both reads hold each identity's complete history, so a moved
        // identity resolves to the collection it moved to.
        let canonical =
            collection_versions(
                &search_visibility(search),
                rrd_vector::candidates_from_changes(&deltas.changes, scope)
                    .into_iter()
                    .chain(rerank.iter().flat_map(|rerank| {
                        rrd_vector::candidates_from_changes(&rerank.changes, scope)
                    }))
                    .collect(),
                collection_id,
                vector_name,
            )?;
        let canonical_candidates = u64::try_from(canonical.len())
            .map_err(|_| ServiceError::Vector("vector candidate count exceeds u64".into()))?;
        let source_cursor = canonical
            .iter()
            .filter(|candidate| candidate.vector.field == search.field)
            .map(|candidate| candidate.source_cursor)
            .max()
            .unwrap_or(0);

        let mut runtime = match binding.kind {
            rrd_vector::VectorArtifactKind::TurboQuant => {
                let mut runtime = rrd_vector::VectorRuntime::new(canonical).map_err(core_vector)?;
                runtime
                    .restore_quantization_lifecycle_active_descriptor(binding.descriptor.clone())
                    .map_err(core_vector)?;
                runtime
            }
            // The same catalogue the general path replays, so a plan names the same revision.
            _ => {
                crate::runtime::vector_catalog::vector_runtime_metadata_from_changes(
                    &catalogue.changes,
                    scope,
                    canonical,
                )
                .map_err(|error| ServiceError::Vector(error.to_string()))?
                .runtime
            }
        };
        runtime
            .install_loaded_artifact(artifact.clone())
            .map_err(core_vector)?;
        let prepared = runtime
            .prepare_search_at(search, source_cursor, ef_search)
            .map_err(core_vector)?;
        if prepared.plan().selected.id != binding.descriptor.stamp().id
            || prepared.plan().selected.exact_rerank != exact_rerank
        {
            return Err(ServiceError::Vector(
                "the generation's plan differs from the candidates it was given".into(),
            ));
        }
        let execution = runtime
            .execute_search(search, &prepared)
            .map_err(core_vector)?;
        let mut selected_versions = catalogue.changes.len() + deltas.changes.len();
        let mut evidence = vec![
            crate::runtime::public_read_evidence(catalogue.evidence.clone())?,
            crate::runtime::public_read_evidence(deltas.evidence)?,
        ];
        if let Some(rerank) = rerank {
            selected_versions += rerank.changes.len();
            evidence.push(crate::runtime::public_read_evidence(rerank.evidence)?);
        }
        search_result(
            request,
            read.clone(),
            Searched {
                selected_versions: u64::try_from(selected_versions).map_err(|_| {
                    ServiceError::Vector("selected vector versions exceed u64".into())
                })?,
                read_evidence: crate::runtime::merge_read_evidence(budget, evidence)?,
                canonical_candidates,
                loaded_artifact_bytes: binding.object.length,
                cold_source_hits: if acquisition.source.is_cold_source() {
                    1
                } else {
                    0
                },
            },
            &prepared,
            execution,
        )
        .map(Some)
    }
}
