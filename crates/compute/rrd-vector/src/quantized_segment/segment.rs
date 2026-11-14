//! Building, loading and searching a quantized segment.

use super::*;

impl QuantizedSegment {
    pub fn build(
        config: QuantizedSegmentConfig,
        generation: u64,
        source_cursor: u64,
        candidates: impl IntoIterator<Item = VectorCandidate>,
    ) -> Result<Self> {
        config.validate()?;
        if generation == 0 {
            return invalid("quantized segment generation must be greater than zero");
        }
        let mut candidates = candidates.into_iter().collect::<Vec<_>>();
        if candidates.is_empty() || candidates.len() > MAX_CANDIDATES {
            return invalid("quantized segment candidate count is outside bounds");
        }
        validate_candidate_versions(&candidates)?;
        for candidate in &candidates {
            if candidate.scope != config.scope
                || candidate.source_cursor > source_cursor
                || candidate.vector.field != config.field
                || candidate.vector.value.dimensions() != config.dimensions
                || !candidate.matches_model(config.embedding_model.as_ref())
                || !matches!(candidate.vector.value, VectorValue::Dense { .. })
            {
                return invalid("quantized candidate violates configuration or coverage");
            }
        }
        candidates.sort_by(|left, right| {
            left.vector
                .reference
                .cmp(&right.vector.reference)
                .then_with(|| left.source_cursor.cmp(&right.source_cursor))
        });
        let values = candidates
            .iter()
            .map(|candidate| match &candidate.vector.value {
                VectorValue::Dense { values } => values.as_slice(),
                _ => unreachable!("dense shape was checked above"),
            })
            .collect::<Vec<_>>();
        let (layout, row_bytes, payload) = encode_payload(config.method, &values)?;
        let metadata = SegmentMetadata {
            config,
            generation,
            source_cursor,
            minimum_cursor: 0,
            row_bytes,
            layout,
            candidates: candidates
                .into_iter()
                .map(|candidate| StoredCandidate {
                    reference: candidate.vector.reference,
                    subject: candidate.vector.subject,
                    collection: candidate.vector.collection,
                    source_cursor: candidate.source_cursor,
                    valid_from: candidate.vector.valid_from,
                    valid_to: candidate.vector.valid_to,
                    properties: candidate.vector.properties,
                })
                .collect(),
        };
        let metadata_bytes = encode_json(&metadata)?;
        let payload_offset = align_up(
            HEADER_BYTES
                .checked_add(metadata_bytes.len())
                .ok_or_else(|| runtime_error("quantized metadata length overflow"))?,
            ALIGNMENT,
        )?;
        let total = payload_offset
            .checked_add(payload.len())
            .ok_or_else(|| runtime_error("quantized artifact length overflow"))?;
        if total > MAX_ARTIFACT_BYTES {
            return invalid("quantized artifact exceeds the 1 GiB safety limit");
        }
        let mut bytes = vec![0_u8; total];
        write_header(
            &mut bytes,
            metadata_bytes.len(),
            payload_offset,
            payload.len(),
            row_bytes,
            metadata.candidates.len(),
            metadata.config.dimensions,
        )?;
        bytes[HEADER_BYTES..HEADER_BYTES + metadata_bytes.len()].copy_from_slice(&metadata_bytes);
        bytes[payload_offset..].copy_from_slice(&payload);
        let artifact_digest = artifact_digest(&bytes)?;
        bytes[DIGEST_OFFSET..DIGEST_OFFSET + DIGEST_BYTES].copy_from_slice(&artifact_digest);
        Self::decode(QuantizedStorage::Owned(Arc::from(bytes)))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return invalid("quantized artifact exceeds the 1 GiB safety limit");
        }
        Self::decode(QuantizedStorage::Owned(Arc::from(bytes)))
    }

    pub fn open_mmap(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|error| {
            runtime_error(format!(
                "cannot open quantized artifact {}: {error}",
                path.display()
            ))
        })?;
        Self::open_mmap_file(file)
    }

    /// Maps the already admitted immutable file without reopening a pathname.
    pub fn open_mmap_file(file: File) -> Result<Self> {
        let length = file
            .metadata()
            .map_err(|error| runtime_error(format!("cannot stat quantized artifact: {error}")))?
            .len();
        if length == 0 || length > MAX_ARTIFACT_BYTES as u64 {
            return invalid("quantized artifact file length is outside safety bounds");
        }
        // SAFETY: the file is opened read-only and its bounded non-zero length
        // was checked. Published artifact files are immutable.
        let mmap = unsafe { MmapOptions::new().map(&file) }
            .map_err(|error| runtime_error(format!("cannot map quantized artifact: {error}")))?;
        Self::decode(QuantizedStorage::Mapped(Arc::new(mmap)))
    }

    pub fn descriptor(&self) -> &QuantizedDescriptor {
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
        kernel: QuantizedKernel,
    ) -> Result<Vec<SearchHit>> {
        request.validate()?;
        self.descriptor.validate()?;
        let VectorQuery::Dense { values: query } = &request.query else {
            return invalid("quantized artifact requires a dense query");
        };
        let required_properties: BTreeSet<String> = request
            .filter
            .as_ref()
            .map(|filter| filter.referenced_properties().into_iter().collect())
            .unwrap_or_default();
        if candidate_limit < request.top_k
            || required_source_cursor > request.read.commit_cursor
            || self.descriptor.stamp.state != ProjectionState::Ready
            || self.descriptor.scope != request.scope
            || self.descriptor.field != request.field
            || self.descriptor.metric != request.metric
            || self.descriptor.embedding_model != request.embedding_model
            || self.descriptor.dimensions != query.len()
            || self.descriptor.stamp.source_cursor < required_source_cursor
            || !required_properties.is_subset(&self.descriptor.filter_properties)
        {
            return invalid("quantized artifact does not satisfy the search contract");
        }
        let mut latest = BTreeMap::<&RuntimeRef, usize>::new();
        for (index, candidate) in self.metadata.candidates.iter().enumerate() {
            if candidate.source_cursor > request.read.commit_cursor
                || candidate.valid_from > request.valid_at
            {
                continue;
            }
            if latest.get(&candidate.reference).is_none_or(|current| {
                self.metadata.candidates[*current].source_cursor < candidate.source_cursor
            }) {
                latest.insert(&candidate.reference, index);
            }
        }
        let payload = &self.as_bytes()[self.payload_offset..];
        let mut hits = Vec::new();
        for index in latest.into_values() {
            let candidate = &self.metadata.candidates[index];
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
            let start = index * self.metadata.row_bytes;
            let row = &payload[start..start + self.metadata.row_bytes];
            let score = score_quantized(query, row, &self.metadata.layout, request.metric, kernel)?;
            hits.push(SearchHit {
                reference: candidate.reference.clone(),
                subject: candidate.subject.clone(),
                source_cursor: candidate.source_cursor,
                score,
            });
        }
        hits.sort_by(SearchHit::compare_best_first);
        hits.truncate(candidate_limit);
        Ok(hits)
    }

    fn decode(storage: QuantizedStorage) -> Result<Self> {
        let bytes = storage.as_bytes();
        let header = read_header(bytes)?;
        let metadata_end = HEADER_BYTES
            .checked_add(header.metadata_len)
            .ok_or_else(|| runtime_error("quantized metadata offset overflow"))?;
        let metadata: SegmentMetadata = serde_json::from_slice(&bytes[HEADER_BYTES..metadata_end])
            .map_err(|error| {
                runtime_error(format!("quantized metadata cannot be decoded: {error}"))
            })?;
        if encode_json(&metadata)? != bytes[HEADER_BYTES..metadata_end] {
            return invalid("quantized metadata is not in canonical encoding");
        }
        validate_decoded(&metadata, &header, bytes)?;
        let expected_digest = artifact_digest(bytes)?;
        if bytes[DIGEST_OFFSET..DIGEST_OFFSET + DIGEST_BYTES] != expected_digest {
            return invalid("quantized artifact digest does not match its bytes");
        }
        let full_precision_vector_bytes = metadata
            .candidates
            .len()
            .checked_mul(metadata.config.dimensions)
            .and_then(|value| value.checked_mul(std::mem::size_of::<f32>()))
            .ok_or_else(|| runtime_error("quantized full-precision bytes overflow"))?;
        let auxiliary_bytes = match &metadata.layout {
            QuantizedLayout::Product { codebooks, .. } => codebooks
                .iter()
                .map(|codebook| codebook.len() * std::mem::size_of::<f32>())
                .sum(),
            QuantizedLayout::Scalar { .. } | QuantizedLayout::Binary => 0,
        };
        let descriptor = QuantizedDescriptor {
            stamp: ProjectionStamp {
                contract_version: DATA_RUNTIME_CONTRACT_VERSION,
                id: metadata.config.id.clone(),
                generation: metadata.generation,
                source_cursor: metadata.source_cursor,
                config_digest: metadata.config.digest()?,
                artifact_digest: hex_digest(&expected_digest),
                state: ProjectionState::Ready,
            },
            scope: metadata.config.scope.clone(),
            field: metadata.config.field.clone(),
            dimensions: metadata.config.dimensions,
            metric: metadata.config.metric,
            method: metadata.config.method,
            embedding_model: metadata.config.embedding_model.clone(),
            filter_properties: metadata.config.filter_properties.clone(),
            minimum_cursor: metadata.minimum_cursor,
            candidate_versions: metadata.candidates.len(),
            packed_vector_bytes: header.payload_len,
            full_precision_vector_bytes,
            auxiliary_bytes,
        };
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            metadata,
            payload_offset: header.payload_offset,
            storage,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Header {
    pub(super) metadata_len: usize,
    pub(super) payload_offset: usize,
    pub(super) payload_len: usize,
    pub(super) row_bytes: usize,
    pub(super) rows: usize,
    pub(super) dimensions: usize,
}
