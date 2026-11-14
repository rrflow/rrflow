//! The vector runtime: artifacts, planning and search.

use super::*;

/// In-process vector search coordinator.
///
/// Canonical candidates remain the truth path. Rebuildable artifacts are
/// installed through the CAS catalog and are selected only through the typed
/// planner. Execution rechecks the exact published descriptor before touching
/// artifact bytes, preventing stale or substituted generations from serving.
#[derive(Debug, Clone, Default)]
pub struct VectorRuntime {
    canonical: Vec<VectorCandidate>,
    catalog: VectorCatalog,
    artifacts: BTreeMap<(ProjectionId, u64), Arc<VectorArtifact>>,
    cost_model: VectorCostModel,
}

impl VectorRuntime {
    pub fn new(canonical: impl IntoIterator<Item = VectorCandidate>) -> Result<Self> {
        let canonical = canonical.into_iter().collect::<Vec<_>>();
        for candidate in &canonical {
            candidate.validate()?;
        }
        Ok(Self {
            canonical,
            catalog: VectorCatalog::default(),
            artifacts: BTreeMap::new(),
            cost_model: VectorCostModel::default(),
        })
    }

    pub fn cost_model(&self) -> &VectorCostModel {
        &self.cost_model
    }

    /// Replaces the per-candidate costs the planner prices codecs with, for
    /// costs measured on this hardware.
    pub fn set_cost_model(&mut self, cost_model: VectorCostModel) -> Result<()> {
        cost_model.validate()?;
        self.cost_model = cost_model;
        Ok(())
    }

    pub fn catalog(&self) -> &VectorCatalog {
        &self.catalog
    }

    pub fn artifact(&self, id: &ProjectionId, generation: u64) -> Option<&VectorArtifact> {
        self.artifacts
            .get(&(id.clone(), generation))
            .map(Arc::as_ref)
    }

    pub fn publish(
        &mut self,
        expected_revision: u64,
        artifact: impl Into<VectorArtifact>,
    ) -> Result<u64> {
        let artifact = artifact.into();
        let descriptor = artifact.descriptor();
        let key = (descriptor.stamp().id.clone(), descriptor.stamp().generation);
        if self.artifacts.contains_key(&key) {
            return invalid("vector artifact generation is already installed");
        }
        let revision = self.catalog.publish(expected_revision, descriptor)?;
        self.artifacts.insert(key, Arc::new(artifact));
        Ok(revision)
    }

    /// Installs the one generation selected by the already-validated durable
    /// quantization lifecycle during restart.
    pub fn restore_quantization_lifecycle_active(
        &mut self,
        artifact: impl Into<VectorArtifact>,
    ) -> Result<u64> {
        let artifact = artifact.into();
        let descriptor = artifact.descriptor();
        let key = (descriptor.stamp().id.clone(), descriptor.stamp().generation);
        if self.artifacts.contains_key(&key) {
            return invalid("restored vector artifact generation is already installed");
        }
        let revision = self
            .catalog
            .restore_quantization_lifecycle_active(descriptor)?;
        self.artifacts.insert(key, Arc::new(artifact));
        Ok(revision)
    }

    /// Replays one durable generic-catalogue descriptor without materializing
    /// its immutable object bytes. Revisions remain exact and planning can run
    /// before a residency policy chooses which artifact to load.
    pub fn replay_catalog_descriptor(
        &mut self,
        expected_revision: u64,
        descriptor: VectorProjectionDescriptor,
    ) -> Result<u64> {
        self.catalog.publish(expected_revision, descriptor)
    }

    /// Restores one quantization-lifecycle-selected descriptor without loading
    /// bytes.
    pub fn restore_quantization_lifecycle_active_descriptor(
        &mut self,
        descriptor: VectorProjectionDescriptor,
    ) -> Result<u64> {
        self.catalog
            .restore_quantization_lifecycle_active(descriptor)
    }

    /// Installs bytes selected by a physical residency authority without
    /// changing catalogue identity or revision.
    pub fn install_loaded_artifact(&mut self, artifact: Arc<VectorArtifact>) -> Result<()> {
        let descriptor = artifact.descriptor();
        let key = (descriptor.stamp().id.clone(), descriptor.stamp().generation);
        let published = self
            .catalog
            .entries
            .get(&descriptor.stamp().id)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: "loaded vector artifact has no published descriptor".into(),
            })?;
        if published != &descriptor || self.artifacts.contains_key(&key) {
            return invalid("loaded vector artifact is duplicate or differs from its descriptor");
        }
        self.artifacts.insert(key, artifact);
        Ok(())
    }

    /// Removes one path from this process-local serving view without changing
    /// its durable catalogue revision. Used only when a bounded physical
    /// residency policy cannot admit that path for the current request.
    pub fn suppress_projection(&mut self, id: &ProjectionId) -> bool {
        let Some(descriptor) = self.catalog.entries.remove(id) else {
            return false;
        };
        self.artifacts
            .remove(&(id.clone(), descriptor.stamp().generation));
        true
    }

    pub fn quarantine(
        &mut self,
        expected_revision: u64,
        id: &ProjectionId,
        generation: u64,
    ) -> Result<u64> {
        self.catalog.quarantine(expected_revision, id, generation)
    }

    pub fn reclaim_retired(&mut self, protected: &BTreeSet<(ProjectionId, u64)>) -> Vec<String> {
        let reclaimed = self.catalog.reclaim_retired(protected);
        let digests = reclaimed.iter().cloned().collect::<BTreeSet<_>>();
        self.artifacts.retain(|_, artifact| {
            !digests.contains(&artifact.descriptor().stamp().artifact_digest)
        });
        reclaimed
    }

    pub fn prepare_search(
        &self,
        request: &SearchRequest,
        ef_search: usize,
    ) -> Result<PreparedVectorSearch> {
        self.prepare_search_at(request, request.read.commit_cursor, ef_search)
    }

    /// Canonical versions a generation at `generation_cursor` does not
    /// contain: those committed after it, up to the request's cursor.
    fn overlay<'a>(
        &'a self,
        generation_cursor: u64,
        required_source_cursor: u64,
        covers: impl Fn(&VectorCandidate) -> bool + 'a,
    ) -> impl Iterator<Item = &'a VectorCandidate> + 'a {
        self.canonical.iter().filter(move |candidate| {
            candidate.source_cursor > generation_cursor
                && candidate.source_cursor <= required_source_cursor
                && covers(candidate)
        })
    }

    pub fn prepare_search_at(
        &self,
        request: &SearchRequest,
        required_source_cursor: u64,
        ef_search: usize,
    ) -> Result<PreparedVectorSearch> {
        if ef_search == 0 || ef_search > 1_000_000 {
            return invalid("vector ef_search must be in 1..=1000000");
        }
        let mut paths = Vec::with_capacity(self.catalog.entries.len());
        for descriptor in self.catalog.entries.values() {
            let estimated_cost = match descriptor {
                VectorProjectionDescriptor::ExactSegment { descriptor } => {
                    descriptor.candidate_versions.max(1) as u64
                }
                VectorProjectionDescriptor::Hnsw { descriptor } => self
                    .artifacts
                    .get(&(descriptor.stamp.id.clone(), descriptor.stamp.generation))
                    .and_then(|artifact| match artifact.as_ref() {
                        VectorArtifact::Hnsw(index) => {
                            index.estimated_search_cost(request, ef_search).ok()
                        }
                        VectorArtifact::ExactSegment(_)
                        | VectorArtifact::CompactDense(_)
                        | VectorArtifact::Quantized(_)
                        | VectorArtifact::TurboQuant(_) => None,
                    })
                    .unwrap_or(descriptor.nodes.max(1) as u64),
                // A codec scans every code, then reranks exactly; the overlay is
                // added below. Exact mode never reaches an approximate path.
                VectorProjectionDescriptor::TurboQuant { descriptor } => codec_scan_cost(
                    self.cost_model.turboquant(descriptor.estimator),
                    descriptor.dimensions,
                    descriptor.candidate_versions,
                    request.mode.exact_rerank(),
                ),
                VectorProjectionDescriptor::Quantized { descriptor } => codec_scan_cost(
                    self.cost_model.quantized(descriptor.method),
                    descriptor.dimensions,
                    descriptor.candidate_versions,
                    request.mode.exact_rerank(),
                ),
            };
            let mut path = descriptor.candidate_path(estimated_cost);
            // A generation behind the request serves with an exact overlay of
            // the canonical versions committed after it.
            let overlay_candidates = match descriptor {
                VectorProjectionDescriptor::Hnsw { descriptor } => self
                    .overlay(
                        descriptor.stamp.source_cursor,
                        required_source_cursor,
                        |c| hnsw_covers_candidate(descriptor, c),
                    )
                    .count(),
                VectorProjectionDescriptor::TurboQuant { descriptor } => self
                    .overlay(
                        descriptor.stamp.source_cursor,
                        required_source_cursor,
                        |c| turboquant_covers_candidate(descriptor, c),
                    )
                    .count(),
                VectorProjectionDescriptor::ExactSegment { .. }
                | VectorProjectionDescriptor::Quantized { .. } => 0,
            };
            if overlay_candidates != 0 {
                path.overlay_source_cursor = Some(required_source_cursor);
                path.overlay_candidates =
                    u64::try_from(overlay_candidates).map_err(|_| Error::InvalidRuntime {
                        reason: "vector overlay candidate count exceeds u64".into(),
                    })?;
                path.estimated_candidates = path
                    .estimated_candidates
                    .saturating_add(path.overlay_candidates);
                path.estimated_cost = path.estimated_cost.saturating_add(path.overlay_candidates);
            }
            paths.push(path);
        }
        let plan = VectorPlanner::new(self.canonical.len() as u64).plan_at(
            request,
            required_source_cursor,
            paths,
        )?;
        let selected_stamp = if plan.selected.id.as_str() == crate::EXACT_SCAN_PROJECTION_ID {
            ProjectionStamp {
                contract_version: DATA_RUNTIME_CONTRACT_VERSION,
                id: plan.selected.id.clone(),
                generation: plan.selected.generation,
                source_cursor: plan.selected.source_cursor,
                config_digest: "00".repeat(32),
                artifact_digest: "00".repeat(32),
                state: ProjectionState::Ready,
            }
        } else {
            self.catalog
                .entries
                .get(&plan.selected.id)
                .map(|descriptor| descriptor.stamp().clone())
                .ok_or_else(|| Error::InvalidRuntime {
                    reason: "selected vector projection is absent from the catalog".into(),
                })?
        };
        let request_digest = request.digest()?;
        let plan_digest = prepared_plan_digest(
            &request_digest,
            self.catalog.revision,
            ef_search,
            &selected_stamp,
            &plan,
        )?;
        let prepared = PreparedVectorSearch {
            request_digest,
            catalog_revision: self.catalog.revision,
            ef_search,
            plan_digest,
            selected_stamp,
            plan,
        };
        prepared.validate()?;
        Ok(prepared)
    }

    pub fn execute_search(
        &self,
        request: &SearchRequest,
        prepared: &PreparedVectorSearch,
    ) -> Result<SearchExecution> {
        prepared.validate()?;
        if request.digest()? != prepared.request_digest {
            return invalid("prepared vector plan belongs to another request");
        }
        if self.catalog.revision != prepared.catalog_revision {
            return invalid("prepared vector plan uses a stale catalog revision");
        }
        let expected = self.prepare_search_at(
            request,
            prepared.plan.required_source_cursor,
            prepared.ef_search,
        )?;
        if &expected != prepared {
            return invalid("prepared vector plan no longer matches planner output");
        }
        let plan = &prepared.plan;
        let hits = match plan.selected.kind {
            AccessPathKind::ExactScan => search_exact_ref(request, &self.canonical)?,
            AccessPathKind::ExactSegment
            | AccessPathKind::Hnsw
            | AccessPathKind::ScalarQuantized
            | AccessPathKind::ProductQuantized
            | AccessPathKind::BinaryQuantized
            | AccessPathKind::TurboQuant => {
                let key = (plan.selected.id.clone(), plan.selected.generation);
                let artifact =
                    self.artifacts
                        .get(&key)
                        .ok_or_else(|| rrd_core::Error::InvalidRuntime {
                            reason: "selected vector artifact bytes are absent".into(),
                        })?;
                let published = self.catalog.entries.get(&plan.selected.id).ok_or_else(|| {
                    rrd_core::Error::InvalidRuntime {
                        reason: "selected vector projection is absent from the catalog".into(),
                    }
                })?;
                if artifact.descriptor() != *published {
                    return invalid("selected vector artifact differs from its catalog descriptor");
                }
                match (plan.selected.kind, artifact.as_ref()) {
                    (AccessPathKind::ExactSegment, VectorArtifact::ExactSegment(segment)) => {
                        let hits = segment.search_at(request, plan.required_source_cursor)?;
                        self.confirm_segment_hits(request, hits)?
                    }
                    (AccessPathKind::ExactSegment, VectorArtifact::CompactDense(segment)) => {
                        let hits = segment.search_at(
                            request,
                            crate::DenseKernel::Auto,
                            plan.required_source_cursor,
                        )?;
                        self.confirm_segment_hits(request, hits)?
                    }
                    (AccessPathKind::Hnsw, VectorArtifact::Hnsw(index)) => {
                        // The graph proposes the versions it stored and the
                        // overlay those changed since it. Each proposed
                        // identity is reranked from its canonical history, so
                        // a version the graph kept after its identity moved
                        // to another collection never answers.
                        let mut references = index
                            .search_candidates_with_kernel(
                                request,
                                prepared.ef_search,
                                HnswKernel::Auto,
                            )?
                            .into_iter()
                            .map(|candidate| candidate.vector.reference)
                            .collect::<BTreeSet<_>>();
                        references.extend(
                            self.overlay(
                                index.descriptor().stamp.source_cursor,
                                plan.required_source_cursor,
                                |candidate| hnsw_covers_candidate(index.descriptor(), candidate),
                            )
                            .map(|candidate| candidate.vector.reference.clone()),
                        );
                        search_exact_ref(
                            request,
                            self.canonical.iter().filter(|candidate| {
                                references.contains(&candidate.vector.reference)
                            }),
                        )?
                    }
                    (
                        AccessPathKind::ScalarQuantized
                        | AccessPathKind::ProductQuantized
                        | AccessPathKind::BinaryQuantized,
                        VectorArtifact::Quantized(segment),
                    ) => {
                        let approximate = segment.search_candidates_at(
                            request,
                            plan.selected.exact_rerank,
                            plan.required_source_cursor,
                            QuantizedKernel::Auto,
                        )?;
                        let references = approximate
                            .into_iter()
                            .map(|hit| hit.reference)
                            .collect::<BTreeSet<_>>();
                        search_exact_ref(
                            request,
                            self.canonical.iter().filter(|candidate| {
                                references.contains(&candidate.vector.reference)
                            }),
                        )?
                    }
                    (AccessPathKind::TurboQuant, VectorArtifact::TurboQuant(segment)) => {
                        // The generation answers at its own cursor; every
                        // identity changed since it is reranked from its full
                        // canonical history, so the exact scorer sees the
                        // version visible at the request's valid time.
                        let generation = segment.descriptor().stamp.source_cursor;
                        let mut references = segment
                            .search_candidates_at(request, plan.selected.exact_rerank, generation)?
                            .into_iter()
                            .map(|hit| hit.reference)
                            .collect::<BTreeSet<_>>();
                        references.extend(
                            self.overlay(generation, plan.required_source_cursor, |candidate| {
                                turboquant_covers_candidate(segment.descriptor(), candidate)
                            })
                            .map(|candidate| candidate.vector.reference.clone()),
                        );
                        search_exact_ref(
                            request,
                            self.canonical.iter().filter(|candidate| {
                                references.contains(&candidate.vector.reference)
                            }),
                        )?
                    }
                    _ => return invalid("selected vector access path has the wrong artifact kind"),
                }
            }
        };
        Ok(SearchExecution {
            plan: plan.clone(),
            hits,
        })
    }

    pub fn search(&self, request: &SearchRequest, ef_search: usize) -> Result<SearchExecution> {
        let prepared = self.prepare_search(request, ef_search)?;
        self.execute_search(request, &prepared)
    }

    /// An exact segment answers from the versions it stored. Its hits stand
    /// when every one is the version the canonical candidates make visible
    /// and they are as many as the request can have. Otherwise, as when an
    /// identity the segment stored has moved to another collection, the
    /// canonical exact scan answers.
    fn confirm_segment_hits(
        &self,
        request: &SearchRequest,
        hits: Vec<SearchHit>,
    ) -> Result<Vec<SearchHit>> {
        let visible = visible_versions(request, &self.canonical)?;
        if hits.len() == request.top_k.min(visible.len())
            && hits
                .iter()
                .all(|hit| visible.contains(&(&hit.reference, hit.source_cursor)))
        {
            return Ok(hits);
        }
        search_exact_ref(request, &self.canonical)
    }
}

pub(super) fn turboquant_covers_candidate(
    descriptor: &crate::TurboQuantDescriptor,
    candidate: &VectorCandidate,
) -> bool {
    candidate.scope == descriptor.scope
        && candidate.vector.field == descriptor.field
        && candidate.vector.value.dimensions() == descriptor.dimensions
        && candidate.matches_model(descriptor.embedding_model.as_ref())
        && matches!(candidate.vector.value, rrd_core::VectorValue::Dense { .. })
}

pub(super) fn hnsw_covers_candidate(
    descriptor: &crate::HnswDescriptor,
    candidate: &VectorCandidate,
) -> bool {
    candidate.scope == descriptor.scope
        && candidate.vector.field == descriptor.field
        && candidate.vector.value.dimensions() == descriptor.dimensions
        && candidate.matches_model(descriptor.embedding_model.as_ref())
        && matches!(candidate.vector.value, rrd_core::VectorValue::Dense { .. })
}

pub(super) fn prepared_plan_digest(
    request_digest: &str,
    catalog_revision: u64,
    ef_search: usize,
    selected_stamp: &ProjectionStamp,
    plan: &SearchPlan,
) -> Result<String> {
    let encoded = serde_json::to_vec(&(
        request_digest,
        catalog_revision,
        ef_search,
        selected_stamp,
        plan,
    ))
    .map_err(|error| Error::InvalidRuntime {
        reason: format!("prepared vector plan cannot be encoded: {error}"),
    })?;
    let mut bytes = b"rrflow-prepared-vector-search-v1\0".to_vec();
    bytes.extend_from_slice(&encoded);
    Ok(digest::sha256_hex(&bytes))
}
