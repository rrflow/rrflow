//! The HNSW index: build, advance, load, search and cost estimation.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct HnswIndex {
    pub(super) descriptor: HnswDescriptor,
    pub(super) config: HnswConfig,
    pub(super) entrypoint: Option<usize>,
    pub(super) nodes: Vec<HnswNode>,
    pub(super) versions: BTreeMap<rrd_core::RuntimeRef, Vec<usize>>,
    /// The collections its nodes belong to, derived when it is read and never stored.
    pub(super) collections: Vec<rrd_core::VectorCollectionAddress>,
    /// The version lists of evenly spaced identities, derived when read and never stored.
    pub(super) cost_sample: Vec<Vec<usize>>,
    pub(super) bytes: Vec<u8>,
}

impl HnswIndex {
    pub fn build(
        config: HnswConfig,
        generation: u64,
        source_cursor: u64,
        candidates: impl IntoIterator<Item = VectorCandidate>,
    ) -> Result<Self> {
        config.validate()?;
        if generation == 0 {
            return invalid("HNSW generation must be greater than zero");
        }
        let mut candidates = candidates.into_iter().collect::<Vec<_>>();
        if candidates.len() > MAX_HNSW_NODES {
            return invalid("HNSW node limit exceeded");
        }
        validate_candidate_versions(&candidates)?;
        candidates.sort_by(|left, right| {
            left.vector
                .reference
                .cmp(&right.vector.reference)
                .then_with(|| left.source_cursor.cmp(&right.source_cursor))
        });
        let mut nodes = Vec::with_capacity(candidates.len());
        let mut entrypoint = None;
        for candidate in candidates {
            if candidate.scope != config.scope
                || candidate.source_cursor > source_cursor
                || candidate.vector.field != config.field
                || candidate.vector.value.dimensions() != config.dimensions
                || !candidate.matches_model(config.embedding_model.as_ref())
                || !matches!(candidate.vector.value, VectorValue::Dense { .. })
            {
                return invalid("HNSW candidate violates configuration or coverage");
            }
            insert_node(&config, &mut nodes, &mut entrypoint, candidate)?;
        }
        repair_reachability(&config, &mut nodes, entrypoint)?;
        let body = HnswBody {
            config: config.clone(),
            generation,
            source_cursor,
            entrypoint,
            indexed_delta_vectors: nodes.len(),
            nodes,
            maintenance: HnswMaintenanceKind::FullBuild,
            previous_generation: None,
        };
        Self::from_body(body)
    }

    /// Appends only vector versions newer than this immutable generation.
    ///
    /// The returned generation is a new content-addressed artifact, so readers
    /// continue serving the prior graph while insertion work runs. Publication
    /// remains one catalogue CAS; no authoritative commit waits for graph work.
    pub fn advance(
        &self,
        generation: u64,
        source_cursor: u64,
        candidates: impl IntoIterator<Item = VectorCandidate>,
    ) -> Result<Self> {
        let expected_generation =
            self.descriptor
                .stamp
                .generation
                .checked_add(1)
                .ok_or_else(|| rrd_core::Error::InvalidRuntime {
                    reason: "HNSW generation overflowed".into(),
                })?;
        if generation != expected_generation {
            return invalid("incremental HNSW generation must immediately follow the active one");
        }
        if source_cursor <= self.descriptor.stamp.source_cursor {
            return invalid("incremental HNSW source cursor must advance");
        }
        let mut candidates = candidates.into_iter().collect::<Vec<_>>();
        if candidates.is_empty() {
            return invalid("incremental HNSW update must contain at least one vector version");
        }
        if self
            .nodes
            .len()
            .checked_add(candidates.len())
            .is_none_or(|total| total > MAX_HNSW_NODES)
        {
            return invalid("HNSW node limit exceeded");
        }
        validate_candidate_versions(&candidates)?;
        let existing = self.stored_versions();
        candidates.sort_by(|left, right| {
            left.source_cursor
                .cmp(&right.source_cursor)
                .then_with(|| left.vector.reference.cmp(&right.vector.reference))
        });
        for candidate in &candidates {
            if candidate.source_cursor <= self.descriptor.stamp.source_cursor
                || candidate.source_cursor > source_cursor
                || existing.contains(&(candidate.vector.reference.clone(), candidate.source_cursor))
                || candidate.scope != self.config.scope
                || candidate.vector.field != self.config.field
                || candidate.vector.value.dimensions() != self.config.dimensions
                || !candidate.matches_model(self.config.embedding_model.as_ref())
                || !matches!(candidate.vector.value, VectorValue::Dense { .. })
            {
                return invalid("incremental HNSW candidate violates configuration or coverage");
            }
        }
        let indexed_delta_vectors = candidates.len();
        let mut nodes = self.nodes.clone();
        let mut entrypoint = self.entrypoint;
        for candidate in candidates {
            insert_node(&self.config, &mut nodes, &mut entrypoint, candidate)?;
        }
        repair_reachability(&self.config, &mut nodes, entrypoint)?;
        Self::from_body(HnswBody {
            config: self.config.clone(),
            generation,
            source_cursor,
            entrypoint,
            nodes,
            maintenance: HnswMaintenanceKind::Incremental,
            previous_generation: Some(self.descriptor.stamp.generation),
            indexed_delta_vectors,
        })
    }

    pub(super) fn from_body(body: HnswBody) -> Result<Self> {
        let artifact_digest = digest::sha256_hex(&encode_json(&body)?);
        let envelope = HnswEnvelope {
            magic: HNSW_MAGIC.into(),
            format_version: HNSW_FORMAT_VERSION,
            artifact_digest: artifact_digest.clone(),
            body,
        };
        let bytes = encode_json(&envelope)?;
        if bytes.len() > MAX_HNSW_BYTES {
            return invalid("encoded HNSW artifact exceeds the 1 GiB safety limit");
        }
        Self::from_parts(envelope, artifact_digest, bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_HNSW_BYTES {
            return invalid("encoded HNSW artifact exceeds the 1 GiB safety limit");
        }
        let envelope: HnswEnvelope =
            serde_json::from_slice(bytes).map_err(|error| rrd_core::Error::InvalidRuntime {
                reason: format!("HNSW artifact cannot be decoded: {error}"),
            })?;
        if envelope.magic != HNSW_MAGIC || envelope.format_version != HNSW_FORMAT_VERSION {
            return invalid("HNSW magic or format version is unsupported");
        }
        if encode_json(&envelope)? != bytes {
            return invalid("HNSW bytes are not in canonical encoding");
        }
        let actual_digest = digest::sha256_hex(&encode_json(&envelope.body)?);
        if actual_digest != envelope.artifact_digest {
            return invalid("HNSW artifact digest does not match its body");
        }
        Self::from_parts(envelope, actual_digest, bytes.to_vec())
    }

    fn from_parts(envelope: HnswEnvelope, artifact_digest: String, bytes: Vec<u8>) -> Result<Self> {
        let config_digest = envelope.body.config.digest()?;
        let descriptor = HnswDescriptor {
            stamp: ProjectionStamp {
                contract_version: DATA_RUNTIME_CONTRACT_VERSION,
                id: envelope.body.config.id.clone(),
                generation: envelope.body.generation,
                source_cursor: envelope.body.source_cursor,
                config_digest,
                artifact_digest,
                state: ProjectionState::Ready,
            },
            scope: envelope.body.config.scope.clone(),
            field: envelope.body.config.field.clone(),
            dimensions: envelope.body.config.dimensions,
            metric: envelope.body.config.metric,
            embedding_model: envelope.body.config.embedding_model.clone(),
            m: envelope.body.config.m,
            ef_construction: envelope.body.config.ef_construction,
            max_level: envelope.body.config.max_level,
            nodes: envelope.body.nodes.len(),
            filter_properties: envelope.body.config.filter_properties.clone(),
            maintenance: envelope.body.maintenance,
            previous_generation: envelope.body.previous_generation,
            indexed_delta_vectors: envelope.body.indexed_delta_vectors,
        };
        descriptor.validate()?;
        validate_graph(
            &envelope.body.config,
            envelope.body.source_cursor,
            envelope.body.entrypoint,
            &envelope.body.nodes,
        )?;
        let mut versions = BTreeMap::<rrd_core::RuntimeRef, Vec<usize>>::new();
        for (id, node) in envelope.body.nodes.iter().enumerate() {
            versions
                .entry(node.candidate.vector.reference.clone())
                .or_default()
                .push(id);
        }
        for ids in versions.values_mut() {
            ids.sort_by_key(|id| envelope.body.nodes[*id].candidate.source_cursor);
        }
        let mut collections = Vec::new();
        for node in &envelope.body.nodes {
            if !collections.contains(&node.candidate.vector.collection) {
                collections.push(node.candidate.vector.collection.clone());
            }
        }
        let cost_sample = versions
            .values()
            .step_by(versions.len().div_ceil(COST_SAMPLE_IDENTITIES).max(1))
            .cloned()
            .collect();
        Ok(Self {
            descriptor,
            config: envelope.body.config,
            entrypoint: envelope.body.entrypoint,
            nodes: envelope.body.nodes,
            versions,
            collections,
            cost_sample,
            bytes,
        })
    }

    /// The candidates this generation proposes for an approximate request, for a caller that
    /// reads their canonical versions before reranking them exactly.
    pub fn search_candidates(
        &self,
        request: &SearchRequest,
        ef_search: usize,
    ) -> Result<Vec<VectorCandidate>> {
        self.search_candidates_with_kernel(request, ef_search, HnswKernel::Auto)
    }

    /// Whether this generation stores versions of `collection`. A descriptor names a field and
    /// not a collection, and two collections may share a field, so only the graph can say. An
    /// empty graph holds nothing to get wrong, and an overlay serves all of its source.
    pub fn serves_collection(&self, collection: &rrd_core::VectorCollectionAddress) -> bool {
        self.nodes.is_empty() || self.collections.contains(collection)
    }

    /// The identity and source cursor of every version this generation
    /// stores. A generation never removes one.
    pub fn stored_versions(&self) -> BTreeSet<(rrd_core::RuntimeRef, u64)> {
        self.nodes
            .iter()
            .map(|node| {
                (
                    node.candidate.vector.reference.clone(),
                    node.candidate.source_cursor,
                )
            })
            .collect()
    }

    pub fn descriptor(&self) -> &HnswDescriptor {
        &self.descriptor
    }

    /// A bound graph may contain only versions from its declared source.
    pub fn contains_only_source(&self, source: &rrd_core::VectorCollectionAddress) -> bool {
        self.nodes
            .iter()
            .all(|node| &node.candidate.vector.collection == source)
    }

    pub fn config(&self) -> &HnswConfig {
        &self.config
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Estimates graph work from the current filter/visibility cardinality.
    /// This scans compact metadata, not vector dimensions, so the planner can
    /// choose the exact path when a highly selective filter would force HNSW
    /// to traverse most of the graph.
    pub fn estimated_search_cost(&self, request: &SearchRequest, ef_search: usize) -> Result<u64> {
        request.validate()?;
        if ef_search == 0 || ef_search > 1_000_000 {
            return invalid("HNSW ef_search must be in 1..=1000000");
        }
        if request.filter.is_none()
            && request.read.commit_cursor == self.descriptor.stamp.source_cursor
        {
            return Ok(ef_search
                .min(self.nodes.len())
                .max(1)
                .saturating_mul(HNSW_NAVIGATION_COST_MULTIPLIER) as u64);
        }
        let mut examined = self.cost_sample.len();
        let mut eligible = self.eligible_identities(request, self.cost_sample.iter());
        if eligible == 0 && examined < self.versions.len() {
            // A filter too rare for the sample to see still has to reach the planner.
            examined = self.versions.len();
            eligible = self.eligible_identities(request, self.versions.values());
        }
        if eligible == 0 {
            return Ok(self
                .nodes
                .len()
                .max(1)
                .saturating_mul(HNSW_NAVIGATION_COST_MULTIPLIER) as u64);
        }
        let estimated = ef_search
            .saturating_mul(examined)
            .div_ceil(eligible)
            .min(self.nodes.len())
            .max(1);
        Ok(estimated.saturating_mul(HNSW_NAVIGATION_COST_MULTIPLIER) as u64)
    }

    /// Generates HNSW candidates, then delegates final scoring, filtering, and
    /// deterministic ordering to the exact oracle.
    pub fn search(&self, request: &SearchRequest, ef_search: usize) -> Result<Vec<SearchHit>> {
        self.search_at_with_kernel(
            request,
            ef_search,
            request.read.commit_cursor,
            HnswKernel::Auto,
        )
    }

    pub fn search_with_kernel(
        &self,
        request: &SearchRequest,
        ef_search: usize,
        kernel: HnswKernel,
    ) -> Result<Vec<SearchHit>> {
        self.search_at_with_kernel(request, ef_search, request.read.commit_cursor, kernel)
    }

    pub fn search_at(
        &self,
        request: &SearchRequest,
        ef_search: usize,
        required_source_cursor: u64,
    ) -> Result<Vec<SearchHit>> {
        self.search_at_with_kernel(request, ef_search, required_source_cursor, HnswKernel::Auto)
    }

    fn search_at_with_kernel(
        &self,
        request: &SearchRequest,
        ef_search: usize,
        required_source_cursor: u64,
        kernel: HnswKernel,
    ) -> Result<Vec<SearchHit>> {
        request.validate()?;
        if required_source_cursor > request.read.commit_cursor {
            return invalid("HNSW source cursor exceeds the request read stamp");
        }
        if self.descriptor.stamp.source_cursor < required_source_cursor {
            return invalid("HNSW artifact does not satisfy request identity or freshness");
        }
        let candidates = self.search_candidates_with_kernel(request, ef_search, kernel)?;
        search_exact(request, candidates)
    }

    pub(crate) fn search_candidates_with_kernel(
        &self,
        request: &SearchRequest,
        ef_search: usize,
        kernel: HnswKernel,
    ) -> Result<Vec<VectorCandidate>> {
        request.validate()?;
        let exact_rerank = match request.mode {
            SearchMode::Exact => return invalid("HNSW cannot serve an exact-only request"),
            SearchMode::AllowApproximate { exact_rerank }
            | SearchMode::RequireApproximate { exact_rerank } => exact_rerank,
        };
        if ef_search < exact_rerank || ef_search > 1_000_000 {
            return invalid("HNSW ef_search must be in exact_rerank..=1000000");
        }
        if self.descriptor.stamp.state != ProjectionState::Ready
            || self.descriptor.scope != request.scope
            || self.descriptor.field != request.field
            || self.descriptor.metric != request.metric
            || self.descriptor.embedding_model != request.embedding_model
            || self.descriptor.dimensions != request.query.dimensions()
        {
            return invalid("HNSW artifact does not satisfy request identity");
        }
        let required = request
            .filter
            .as_ref()
            .map(|filter| {
                filter
                    .referenced_properties()
                    .into_iter()
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        if !required.is_subset(&self.descriptor.filter_properties) {
            return invalid("HNSW artifact does not cover every filter property");
        }
        let Some(entrypoint) = self.entrypoint else {
            return Ok(Vec::new());
        };
        let mut current = entrypoint;
        let query = dense_query(&request.query)?;
        for layer in (1..=self.nodes[entrypoint].level).rev() {
            current = greedy_layer(
                &self.nodes,
                query,
                current,
                layer as usize,
                self.config.metric,
                kernel,
            )?;
        }
        let mut candidates = search_layer(
            &self.nodes,
            query,
            &[current],
            SearchLayerOptions {
                ef: ef_search,
                layer: 0,
                metric: self.config.metric,
                kernel,
                eligible: Some(&|id| self.is_visible(request, id)),
            },
        )?;
        candidates.truncate(exact_rerank);
        Ok(candidates
            .into_iter()
            .map(|scored| self.nodes[scored.id].candidate.clone())
            .collect())
    }

    /// How many of these identities a request could be answered from.
    fn eligible_identities<'a>(
        &self,
        request: &SearchRequest,
        identities: impl Iterator<Item = &'a Vec<usize>>,
    ) -> usize {
        identities
            .filter(|versions| {
                #[cfg(test)]
                IDENTITIES_VISITED.with(|visited| visited.set(visited.get() + 1));
                self.latest_version(request, versions)
                    .is_some_and(|id| self.admits(request, id))
            })
            .count()
    }

    fn latest_version(&self, request: &SearchRequest, versions: &[usize]) -> Option<usize> {
        versions.iter().rev().copied().find(|version| {
            let version = &self.nodes[*version].candidate;
            version.source_cursor <= request.read.commit_cursor
                && version.vector.valid_from <= request.valid_at
        })
    }

    fn is_visible(&self, request: &SearchRequest, id: usize) -> bool {
        self.versions
            .get(&self.nodes[id].candidate.vector.reference)
            .and_then(|versions| self.latest_version(request, versions))
            == Some(id)
            && self.admits(request, id)
    }

    /// Whether this version is still valid at the request's time and passes its filter.
    fn admits(&self, request: &SearchRequest, id: usize) -> bool {
        let candidate = &self.nodes[id].candidate;
        if candidate
            .vector
            .valid_to
            .is_some_and(|valid_to| request.valid_at >= valid_to)
        {
            return false;
        }
        request
            .filter
            .as_ref()
            .is_none_or(|filter| filter.matches(candidate.filter_properties()))
    }
}

#[cfg(test)]
thread_local! {
    /// How many identities were checked for visibility, for a test of how much one estimate reads.
    pub(super) static IDENTITIES_VISITED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ScoredNode {
    pub(super) id: usize,
    pub(super) score: f64,
}

impl PartialEq for ScoredNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.score.total_cmp(&other.score) == Ordering::Equal
    }
}

impl Eq for ScoredNode {}

impl PartialOrd for ScoredNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .total_cmp(&other.score)
            .then_with(|| other.id.cmp(&self.id))
    }
}
