//! Building, loading and searching a TurboQuant segment.

use super::*;

impl TurboQuantSegment {
    pub fn build(
        config: TurboQuantSegmentConfig,
        generation: u64,
        source_cursor: u64,
        candidates: impl IntoIterator<Item = VectorCandidate>,
    ) -> Result<Self> {
        config.validate()?;
        if generation == 0 {
            return invalid("TurboQuant segment generation must be greater than zero");
        }
        let mut candidates = candidates.into_iter().collect::<Vec<_>>();
        if candidates.is_empty() || candidates.len() > MAX_TURBOQUANT_CANDIDATES {
            return invalid("TurboQuant segment candidate count is outside bounds");
        }
        validate_candidate_versions(&candidates)?;
        sort_candidates(&mut candidates);
        let projection = config.projection()?;
        let mut versions = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if candidate.source_cursor > source_cursor {
                return invalid("TurboQuant candidate violates configuration or coverage");
            }
            versions.push(encode_candidate(&config, projection.as_ref(), candidate)?);
        }
        Self::assemble(config, generation, source_cursor, versions, projection)
    }

    /// The next generation of `previous`, from the identities that changed
    /// since it.
    ///
    /// `replaced` names every identity whose history in this source changed
    /// after `previous`. `candidates` holds what the source holds of each such
    /// identity's complete history at the new read. Every other identity
    /// keeps its stored versions and their codes. The codec encodes each
    /// vector on its own, with the configuration's seed and fixed codebook, so
    /// re-encoding a kept version would give the same bytes. `source_cursor`
    /// is the cursor the new generation covers, as for a full build: never
    /// less than `previous` covers or than any version it holds. The result is
    /// byte for byte the segment `build` makes from the same versions and
    /// cursor.
    pub fn extend(
        previous: &Self,
        config: TurboQuantSegmentConfig,
        generation: u64,
        replaced: &BTreeSet<RuntimeRef>,
        candidates: impl IntoIterator<Item = VectorCandidate>,
        source_cursor: u64,
    ) -> Result<Self> {
        config.validate()?;
        if config.digest()? != previous.descriptor.stamp.config_digest {
            return invalid("a TurboQuant generation extends only one with its configuration");
        }
        if generation <= previous.descriptor.stamp.generation {
            return invalid("a TurboQuant generation extends only an earlier generation");
        }
        if source_cursor < previous.descriptor.stamp.source_cursor {
            return invalid("a TurboQuant generation never covers less than its base");
        }
        let mut candidates = candidates.into_iter().collect::<Vec<_>>();
        validate_candidate_versions(&candidates)?;
        if candidates
            .iter()
            .any(|candidate| !replaced.contains(&candidate.vector.reference))
        {
            return invalid("an unchanged identity keeps its stored TurboQuant versions");
        }
        sort_candidates(&mut candidates);
        let payload = previous.payload();
        let width = previous.packed_bytes_per_vector;
        let mut versions = previous
            .candidates
            .iter()
            .zip(payload.chunks_exact(width))
            .filter(|(stored, _)| !replaced.contains(&stored.reference))
            .map(|(stored, packed)| (stored.clone(), packed.to_vec()))
            .collect::<Vec<_>>();
        let projection = config.projection()?;
        for candidate in candidates {
            versions.push(encode_candidate(&config, projection.as_ref(), candidate)?);
        }
        versions.sort_by(|(left, _), (right, _)| {
            left.reference
                .cmp(&right.reference)
                .then_with(|| left.source_cursor.cmp(&right.source_cursor))
        });
        if versions.is_empty() || versions.len() > MAX_TURBOQUANT_CANDIDATES {
            return invalid("TurboQuant segment candidate count is outside bounds");
        }
        if versions
            .iter()
            .any(|(stored, _)| stored.source_cursor > source_cursor)
        {
            return invalid("TurboQuant candidate violates configuration or coverage");
        }
        Self::assemble(config, generation, source_cursor, versions, projection)
    }

    /// The artifact for `versions`, already in reference and cursor order,
    /// each with its packed codes.
    fn assemble(
        config: TurboQuantSegmentConfig,
        generation: u64,
        source_cursor: u64,
        versions: Vec<(StoredCandidate, Vec<u8>)>,
        projection: Option<QjlProjection>,
    ) -> Result<Self> {
        let mut stored = Vec::with_capacity(versions.len());
        let mut payload = Vec::new();
        let mut packed_bytes_per_vector = None;
        for (candidate, packed) in versions {
            match packed_bytes_per_vector {
                Some(expected) if expected != packed.len() => {
                    return invalid("TurboQuant vectors produced inconsistent packed lengths");
                }
                None => packed_bytes_per_vector = Some(packed.len()),
                _ => {}
            }
            payload.extend_from_slice(&packed);
            stored.push(candidate);
        }
        let header = SegmentHeader {
            config,
            generation,
            source_cursor,
            minimum_cursor: 0,
            packed_bytes_per_vector: packed_bytes_per_vector.unwrap_or_default(),
            candidates: stored,
        };
        let header_bytes = encode_json(&header)?;
        let artifact_digest = artifact_digest(&header_bytes, &payload);
        let bytes = encode_artifact(&header_bytes, &payload, &artifact_digest)?;
        let segment = Self::decode(TurboQuantStorage::Owned(Arc::from(bytes)))?;
        // The build already holds S; the segment keeps it for its searches.
        if let Some(projection) = projection {
            let _ = segment.projection.set(projection);
        }
        Ok(segment)
    }

    pub(super) fn payload(&self) -> &[u8] {
        &self.as_bytes()[self.payload_offset..self.payload_offset + self.payload_len]
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Self::decode(TurboQuantStorage::Owned(Arc::from(bytes)))
    }

    pub fn open_mmap(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|error| {
            runtime_error(format!(
                "cannot open TurboQuant artifact {}: {error}",
                path.display()
            ))
        })?;
        Self::open_mmap_file(file)
    }

    /// Maps the already admitted immutable file without reopening a pathname.
    pub fn open_mmap_file(file: File) -> Result<Self> {
        let length = file
            .metadata()
            .map_err(|error| runtime_error(format!("cannot stat TurboQuant artifact: {error}")))?
            .len();
        if length == 0 || length > MAX_TURBOQUANT_SEGMENT_BYTES as u64 {
            return invalid("TurboQuant artifact file length is outside safety bounds");
        }
        // SAFETY: the file is opened read-only and its bounded non-zero length
        // was checked. Published artifact files are immutable.
        let mmap = unsafe { MmapOptions::new().map(&file) }
            .map_err(|error| runtime_error(format!("cannot map TurboQuant artifact: {error}")))?;
        Self::decode(TurboQuantStorage::Mapped(Arc::new(mmap)))
    }

    fn decode(storage: TurboQuantStorage) -> Result<Self> {
        let bytes = storage.as_bytes();
        if bytes.len() < HEADER_PREFIX_BYTES || bytes.len() > MAX_TURBOQUANT_SEGMENT_BYTES {
            return invalid("TurboQuant artifact byte length is outside bounds");
        }
        if &bytes[..8] != TURBOQUANT_SEGMENT_MAGIC {
            return invalid("TurboQuant artifact magic is invalid");
        }
        let version = u16::from_le_bytes(bytes[8..10].try_into().expect("fixed version slice"));
        if version != TURBOQUANT_SEGMENT_FORMAT_VERSION {
            return invalid("TurboQuant artifact format version is unsupported");
        }
        let header_len = u32::from_le_bytes(bytes[10..14].try_into().expect("fixed header slice"));
        let payload_len =
            u64::from_le_bytes(bytes[14..22].try_into().expect("fixed payload slice"));
        let header_len =
            usize::try_from(header_len).map_err(|_| rrd_core::Error::InvalidRuntime {
                reason: "TurboQuant header length overflows this host".into(),
            })?;
        let payload_len =
            usize::try_from(payload_len).map_err(|_| rrd_core::Error::InvalidRuntime {
                reason: "TurboQuant payload length overflows this host".into(),
            })?;
        let expected_len = HEADER_PREFIX_BYTES
            .checked_add(header_len)
            .and_then(|value| value.checked_add(payload_len))
            .ok_or_else(|| rrd_core::Error::InvalidRuntime {
                reason: "TurboQuant artifact length overflows".into(),
            })?;
        if bytes.len() != expected_len {
            return invalid("TurboQuant artifact framing length is inconsistent");
        }
        let advertised =
            std::str::from_utf8(&bytes[22..86]).map_err(|_| rrd_core::Error::InvalidRuntime {
                reason: "TurboQuant digest is not ASCII".into(),
            })?;
        let header_bytes = &bytes[HEADER_PREFIX_BYTES..HEADER_PREFIX_BYTES + header_len];
        let payload = &bytes[HEADER_PREFIX_BYTES + header_len..];
        let actual = artifact_digest(header_bytes, payload);
        if advertised != actual {
            return invalid("TurboQuant artifact digest does not match its bytes");
        }
        let header: SegmentHeader = serde_json::from_slice(header_bytes).map_err(|error| {
            rrd_core::Error::InvalidRuntime {
                reason: format!("TurboQuant header cannot be decoded: {error}"),
            }
        })?;
        if encode_json(&header)? != header_bytes {
            return invalid("TurboQuant header is not in canonical encoding");
        }
        header.config.validate()?;
        if header.generation == 0
            || header.minimum_cursor != 0
            || header.candidates.is_empty()
            || header.candidates.len() > MAX_TURBOQUANT_CANDIDATES
            || header.packed_bytes_per_vector == 0
            || payload_len
                != header
                    .candidates
                    .len()
                    .saturating_mul(header.packed_bytes_per_vector)
        {
            return invalid("TurboQuant header coverage or payload length is invalid");
        }
        let inner_product = header.config.estimator == TurboQuantEstimator::InnerProduct;
        if inner_product
            && Some(header.packed_bytes_per_vector)
                != inner_product_packed_len(
                    header.config.bits,
                    inner_product_padded_dimensions(header.config.dimensions),
                )
        {
            return invalid("TurboQuant inner-product code width differs from its configuration");
        }
        // Versions strictly ascend by reference, then source cursor. This
        // makes each version unique, and lets a search resolve every
        // identity's latest version in one linear pass.
        if header.candidates.windows(2).any(|pair| {
            (&pair[0].reference, pair[0].source_cursor)
                >= (&pair[1].reference, pair[1].source_cursor)
        }) {
            return invalid("TurboQuant candidate versions are not in reference and cursor order");
        }
        for candidate in &header.candidates {
            let estimator_norms = if inner_product {
                candidate.centroid_norm >= 0.0
                    && candidate
                        .residual_norm
                        .is_some_and(|residual| residual.is_finite() && residual >= 0.0)
            } else {
                candidate.centroid_norm > 0.0 && candidate.residual_norm.is_none()
            };
            if candidate.source_cursor == 0
                || candidate.source_cursor > header.source_cursor
                || !candidate.original_norm.is_finite()
                || candidate.original_norm < 0.0
                || !candidate.centroid_norm.is_finite()
                || !estimator_norms
            {
                return invalid("TurboQuant candidate metadata is invalid");
            }
        }
        let descriptor = TurboQuantDescriptor {
            stamp: ProjectionStamp {
                contract_version: DATA_RUNTIME_CONTRACT_VERSION,
                id: header.config.id.clone(),
                generation: header.generation,
                source_cursor: header.source_cursor,
                config_digest: header.config.digest()?,
                artifact_digest: actual,
                state: ProjectionState::Ready,
            },
            scope: header.config.scope,
            field: header.config.field,
            dimensions: header.config.dimensions,
            metric: header.config.metric,
            bits: header.config.bits,
            seed: header.config.seed,
            embedding_model: header.config.embedding_model,
            filter_properties: header.config.filter_properties,
            estimator: header.config.estimator,
            minimum_cursor: header.minimum_cursor,
            candidate_versions: header.candidates.len(),
            packed_vector_bytes: payload_len,
            full_precision_vector_bytes: header
                .candidates
                .len()
                .saturating_mul(header.config.dimensions)
                .saturating_mul(std::mem::size_of::<f32>()),
        };
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            candidates: header.candidates,
            packed_bytes_per_vector: header.packed_bytes_per_vector,
            payload_offset: HEADER_PREFIX_BYTES + header_len,
            payload_len,
            storage,
            projection: Arc::new(OnceLock::new()),
        })
    }

    pub fn descriptor(&self) -> &TurboQuantDescriptor {
        &self.descriptor
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.storage.as_bytes()
    }

    pub fn memory_placement(&self) -> QuantizedMemoryPlacement {
        self.storage.placement()
    }

    pub fn search_candidates_at(
        &self,
        request: &SearchRequest,
        candidate_limit: usize,
        required_source_cursor: u64,
    ) -> Result<Vec<SearchHit>> {
        self.search_candidates_at_with_kernel(
            request,
            candidate_limit,
            required_source_cursor,
            QuantizedKernel::Auto,
        )
    }

    pub fn search_candidates_at_with_kernel(
        &self,
        request: &SearchRequest,
        candidate_limit: usize,
        required_source_cursor: u64,
        kernel: QuantizedKernel,
    ) -> Result<Vec<SearchHit>> {
        request.validate()?;
        self.descriptor.validate()?;
        let VectorQuery::Dense { values } = &request.query else {
            return invalid("TurboQuant artifact requires a dense query");
        };
        let required_properties = request
            .filter
            .as_ref()
            .map(|filter| {
                filter
                    .referenced_properties()
                    .into_iter()
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        if candidate_limit < request.top_k
            || required_source_cursor > request.read.commit_cursor
            || self.descriptor.stamp.state != ProjectionState::Ready
            || self.descriptor.scope != request.scope
            || self.descriptor.field != request.field
            || self.descriptor.metric != request.metric
            || self.descriptor.embedding_model != request.embedding_model
            || self.descriptor.dimensions != values.len()
            || self.descriptor.stamp.source_cursor < required_source_cursor
            || !required_properties.is_subset(&self.descriptor.filter_properties)
        {
            return invalid("TurboQuant artifact does not satisfy the search contract");
        }
        // `decode` guarantees candidates strictly ascend by reference, then
        // source cursor. Each identity's versions are therefore one run, and
        // the last visible version in a run is that identity's latest.
        let visible = |candidate: &StoredCandidate| {
            candidate.source_cursor <= request.read.commit_cursor
                && candidate.valid_from <= request.valid_at
        };
        let mut latest = Vec::new();
        let mut start = 0;
        while start < self.candidates.len() {
            let reference = &self.candidates[start].reference;
            let end = self.candidates[start..]
                .iter()
                .position(|candidate| &candidate.reference != reference)
                .map_or(self.candidates.len(), |offset| start + offset);
            if let Some(index) = (start..end).rfind(|&index| visible(&self.candidates[index])) {
                latest.push(index);
            }
            start = end;
        }
        let payload = &self.as_bytes()[self.payload_offset..self.payload_offset + self.payload_len];
        let query = match self.descriptor.estimator {
            TurboQuantEstimator::Mse => TurboQuantQuery::mse(
                values,
                request.metric,
                self.descriptor.bits,
                self.descriptor.seed,
            )?,
            TurboQuantEstimator::InnerProduct => TurboQuantQuery::inner_product(
                values,
                request.metric,
                self.descriptor.bits,
                self.descriptor.seed,
                self.projection()?,
            )?,
        };
        let mut hits = CandidateHits::new(candidate_limit);
        for index in latest {
            let candidate = &self.candidates[index];
            if candidate
                .valid_to
                .is_some_and(|valid_to| request.valid_at >= valid_to)
                || request
                    .filter
                    .as_ref()
                    .is_some_and(|filter| !filter.matches(&candidate.properties))
            {
                continue;
            }
            let start = index * self.packed_bytes_per_vector;
            let packed = &payload[start..start + self.packed_bytes_per_vector];
            let score = match candidate.residual_norm {
                None => query.score_mse(
                    candidate.original_norm,
                    candidate.centroid_norm,
                    packed,
                    kernel,
                )?,
                Some(residual_norm) => query.score_inner_product(
                    candidate.original_norm,
                    residual_norm,
                    packed,
                    kernel,
                )?,
            };
            if hits.admits(score, &candidate.reference, candidate.source_cursor) {
                hits.consider(SearchHit {
                    reference: candidate.reference.clone(),
                    subject: candidate.subject.clone(),
                    source_cursor: candidate.source_cursor,
                    score,
                });
            }
        }
        Ok(hits.finish())
    }

    /// S for this inner-product generation, materialized once and kept.
    fn projection(&self) -> Result<&QjlProjection> {
        if let Some(projection) = self.projection.get() {
            return Ok(projection);
        }
        let projection = QjlProjection::new(
            inner_product_padded_dimensions(self.descriptor.dimensions),
            self.descriptor.seed,
        )?;
        // A concurrent search may have set it first; both drew the same S.
        let _ = self.projection.set(projection);
        self.projection
            .get()
            .map_or_else(|| invalid("the QJL projection could not be kept"), Ok)
    }
}
