//! Compact dense segment format, metadata, storage and the segment.

use super::*;

pub const COMPACT_DENSE_FORMAT_VERSION: u16 = 1;
pub(super) const MAGIC: &[u8; 8] = b"RRDDMAP1";
pub(super) const HEADER_BYTES: usize = 128;
pub(super) const DIGEST_OFFSET: usize = 72;
pub(super) const DIGEST_BYTES: usize = 32;
pub(super) const ALIGNMENT: usize = 64;
pub(super) const MAX_ARTIFACT_BYTES: usize = 1 << 30;
pub(super) const MAX_CANDIDATES: usize = 10_000_000;
pub(super) static TEMPORARY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenseMemoryPlacement {
    Owned,
    Mapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenseKernel {
    /// Byte-decoding scalar reference, independent of host alignment.
    Scalar,
    /// Runtime-dispatched AVX2 where available, otherwise the scalar oracle.
    Auto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DenseMetadata {
    pub(super) config: VectorSegmentConfig,
    pub(super) generation: u64,
    pub(super) source_cursor: u64,
    pub(super) minimum_cursor: u64,
    pub(super) candidates: Vec<DenseCandidateMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DenseCandidateMetadata {
    pub(super) scope: rrd_core::ScopeId,
    pub(super) source_cursor: u64,
    pub(super) reference: RuntimeRef,
    pub(super) subject: RuntimeRef,
    pub(super) collection: VectorCollectionAddress,
    pub(super) field: String,
    pub(super) valid_from: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) valid_to: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) provenance: Option<EmbeddingProvenance>,
    #[serde(default)]
    pub(super) properties: RuntimeProperties,
}

#[derive(Debug, Clone)]
pub(super) enum DenseStorage {
    Owned(Arc<[u8]>),
    Mapped(Arc<Mmap>),
}

impl DenseStorage {
    fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes,
            Self::Mapped(bytes) => bytes,
        }
    }

    fn placement(&self) -> DenseMemoryPlacement {
        match self {
            Self::Owned(_) => DenseMemoryPlacement::Owned,
            Self::Mapped(_) => DenseMemoryPlacement::Mapped,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompactDenseSegment {
    descriptor: crate::SegmentDescriptor,
    metadata: DenseMetadata,
    vector_offset: usize,
    row_stride: usize,
    storage: DenseStorage,
}

impl PartialEq for CompactDenseSegment {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor == other.descriptor && self.as_bytes() == other.as_bytes()
    }
}

impl CompactDenseSegment {
    pub fn build(
        config: VectorSegmentConfig,
        generation: u64,
        source_cursor: u64,
        candidates: impl IntoIterator<Item = VectorCandidate>,
    ) -> Result<Self> {
        config.validate()?;
        if generation == 0 {
            return invalid("compact dense generation must be greater than zero");
        }
        let mut candidates = candidates.into_iter().collect::<Vec<_>>();
        if candidates.len() > MAX_CANDIDATES {
            return invalid("compact dense candidate limit exceeded");
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
                return invalid("compact dense candidate violates configuration or coverage");
            }
        }
        candidates.sort_by(|left, right| {
            left.vector
                .reference
                .cmp(&right.vector.reference)
                .then_with(|| left.source_cursor.cmp(&right.source_cursor))
        });
        let metadata = DenseMetadata {
            config,
            generation,
            source_cursor,
            minimum_cursor: 0,
            candidates: candidates
                .iter()
                .map(DenseCandidateMetadata::from)
                .collect(),
        };
        let metadata_bytes = encode_json(&metadata)?;
        let vector_offset = align_up(
            HEADER_BYTES
                .checked_add(metadata_bytes.len())
                .ok_or_else(|| runtime_error("compact dense metadata length overflow"))?,
            ALIGNMENT,
        )?;
        let row_bytes = metadata
            .config
            .dimensions
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or_else(|| runtime_error("compact dense row length overflow"))?;
        let row_stride = align_up(row_bytes, ALIGNMENT)?;
        let vector_bytes = row_stride
            .checked_mul(candidates.len())
            .ok_or_else(|| runtime_error("compact dense payload length overflow"))?;
        let total = vector_offset
            .checked_add(vector_bytes)
            .ok_or_else(|| runtime_error("compact dense artifact length overflow"))?;
        if total > MAX_ARTIFACT_BYTES {
            return invalid("compact dense artifact exceeds the 1 GiB safety limit");
        }

        let mut bytes = vec![0; total];
        write_header(
            &mut bytes,
            metadata_bytes.len(),
            vector_offset,
            vector_bytes,
            row_stride,
            candidates.len(),
            metadata.config.dimensions,
        )?;
        bytes[HEADER_BYTES..HEADER_BYTES + metadata_bytes.len()].copy_from_slice(&metadata_bytes);
        for (row, candidate) in candidates.iter().enumerate() {
            let VectorValue::Dense { values } = &candidate.vector.value else {
                unreachable!("dense shape was checked above")
            };
            let start = vector_offset + row * row_stride;
            for (chunk, value) in bytes[start..start + row_bytes]
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(values)
            {
                chunk.copy_from_slice(&value.to_le_bytes());
            }
        }
        let artifact_digest = artifact_digest(&bytes)?;
        bytes[DIGEST_OFFSET..DIGEST_OFFSET + DIGEST_BYTES].copy_from_slice(&artifact_digest);
        Self::decode(DenseStorage::Owned(Arc::from(bytes)))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return invalid("compact dense artifact exceeds the 1 GiB safety limit");
        }
        Self::decode(DenseStorage::Owned(Arc::from(bytes)))
    }

    /// Opens a verified read-only memory map. The artifact must remain
    /// immutable for the lifetime of this value; publication through
    /// [`Self::write_atomic`] establishes that contract for local files.
    pub fn open_mmap(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|error| {
            runtime_error(format!(
                "cannot open compact dense artifact {}: {error}",
                path.display()
            ))
        })?;
        Self::open_mmap_file(file)
    }

    /// Maps the already admitted immutable file without reopening a pathname.
    pub fn open_mmap_file(file: File) -> Result<Self> {
        let length = file
            .metadata()
            .map_err(|error| runtime_error(format!("cannot stat compact dense artifact: {error}")))?
            .len();
        if length == 0 || length > MAX_ARTIFACT_BYTES as u64 {
            return invalid("compact dense artifact file length is outside safety bounds");
        }
        // SAFETY: the file is opened read-only, its non-zero bounded length was
        // checked, and the returned Mmap owns the mapping independently of File.
        // Callers must uphold the documented immutable-file contract.
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|error| {
            runtime_error(format!("cannot map compact dense artifact: {error}"))
        })?;
        Self::decode(DenseStorage::Mapped(Arc::new(mmap)))
    }

    /// Durably stages and atomically publishes this immutable artifact. An
    /// existing path is accepted only when it contains the same verified bytes.
    pub fn write_atomic(&self, path: impl AsRef<Path>) -> Result<()> {
        use rrd_store::filesystem::{AnchoredPath, OpenOptions};
        let path = AnchoredPath::from_path(path.as_ref())
            .map_err(|error| runtime_error(error.to_string()))?;
        let parent = path
            .parent()
            .ok_or_else(|| runtime_error("artifact has no parent"))?;
        parent
            .create_dir_all()
            .map_err(|error| runtime_error(error.to_string()))?;
        if path
            .try_exists()
            .map_err(|error| runtime_error(error.to_string()))?
        {
            let existing = Self::open_mmap_file(
                path.open_read()
                    .map_err(|error| runtime_error(error.to_string()))?,
            )?;
            if existing.as_bytes() == self.as_bytes() {
                return Ok(());
            }
            return invalid("compact dense publication path contains different bytes");
        }
        let name = path
            .relative_path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| runtime_error("compact dense publication path has no UTF-8 filename"))?;
        let temporary = parent.join(format!(
            ".{name}.{}.{}.tmp",
            std::process::id(),
            TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| {
                runtime_error(format!("cannot stage compact dense artifact: {error}"))
            })?;
        let publish = (|| -> std::io::Result<()> {
            file.write_all(self.as_bytes())?;
            file.sync_all()?;
            temporary.rename_new_to(&path)
        })();
        drop(file);
        if let Err(error) = publish {
            let _ = temporary.remove_file();
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                let existing = Self::open_mmap_file(
                    path.open_read()
                        .map_err(|error| runtime_error(error.to_string()))?,
                )?;
                if existing.as_bytes() == self.as_bytes() {
                    return Ok(());
                }
                return invalid("compact dense publication race installed different bytes");
            }
            return Err(runtime_error(format!(
                "cannot publish compact dense artifact: {error}"
            )));
        }
        let published = Self::open_mmap_file(
            path.open_read()
                .map_err(|error| runtime_error(error.to_string()))?,
        )?;
        if published.descriptor != self.descriptor {
            return invalid("published compact dense descriptor failed verification");
        }
        Ok(())
    }

    pub fn descriptor(&self) -> &crate::SegmentDescriptor {
        &self.descriptor
    }

    pub fn memory_placement(&self) -> DenseMemoryPlacement {
        self.storage.placement()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.storage.as_bytes()
    }

    pub fn vector_payload_bytes(&self) -> usize {
        self.row_stride * self.metadata.candidates.len()
    }

    pub fn search(&self, request: &SearchRequest, kernel: DenseKernel) -> Result<Vec<SearchHit>> {
        self.search_at(request, kernel, request.read.commit_cursor)
    }

    pub fn search_at(
        &self,
        request: &SearchRequest,
        kernel: DenseKernel,
        required_source_cursor: u64,
    ) -> Result<Vec<SearchHit>> {
        request.validate()?;
        self.descriptor.validate()?;
        if required_source_cursor > request.read.commit_cursor {
            return invalid("compact dense source cursor exceeds the request read stamp");
        }
        if self.descriptor.stamp.state != ProjectionState::Ready
            || self.descriptor.scope != request.scope
            || self.descriptor.field != request.field
            || self.descriptor.metric != request.metric
            || self.descriptor.embedding_model != request.embedding_model
            || self.descriptor.dimensions != request.query.dimensions()
            || self.descriptor.stamp.source_cursor < required_source_cursor
        {
            return invalid("compact dense segment does not satisfy request identity or freshness");
        }
        let crate::VectorQuery::Dense { values: query } = &request.query else {
            return invalid("compact dense segment requires a dense query");
        };
        let mut latest = BTreeMap::<&RuntimeRef, usize>::new();
        for (row, candidate) in self.metadata.candidates.iter().enumerate() {
            if candidate.scope != request.scope
                || candidate.source_cursor > request.read.commit_cursor
            {
                continue;
            }
            if candidate.valid_from > request.valid_at {
                continue;
            }
            latest
                .entry(&candidate.reference)
                .and_modify(|current| {
                    if self.metadata.candidates[*current].source_cursor < candidate.source_cursor {
                        *current = row;
                    }
                })
                .or_insert(row);
        }
        let mut hits = Vec::new();
        for row in latest.into_values() {
            let candidate = &self.metadata.candidates[row];
            if candidate.field != request.field
                || candidate
                    .valid_to
                    .is_some_and(|valid_to| request.valid_at >= valid_to)
                || request
                    .filter
                    .as_ref()
                    .is_some_and(|filter| !filter.matches(&candidate.properties))
            {
                continue;
            }
            hits.push(SearchHit {
                reference: candidate.reference.clone(),
                subject: candidate.subject.clone(),
                source_cursor: candidate.source_cursor,
                score: score_row(query, self.row(row), request.metric, kernel)?,
            });
        }
        hits.sort_by(SearchHit::compare_best_first);
        hits.truncate(request.top_k);
        Ok(hits)
    }

    fn decode(storage: DenseStorage) -> Result<Self> {
        let bytes = storage.as_bytes();
        let header = parse_header(bytes)?;
        let expected_digest = artifact_digest(bytes)?;
        if bytes[DIGEST_OFFSET..DIGEST_OFFSET + DIGEST_BYTES] != expected_digest {
            return invalid("compact dense artifact digest mismatch");
        }
        let metadata_bytes = &bytes[HEADER_BYTES..HEADER_BYTES + header.metadata_len];
        let metadata: DenseMetadata = serde_json::from_slice(metadata_bytes).map_err(|error| {
            runtime_error(format!("compact dense metadata cannot be decoded: {error}"))
        })?;
        if encode_json(&metadata)? != metadata_bytes {
            return invalid("compact dense metadata is not canonical JSON");
        }
        validate_decoded(&metadata, &header, bytes)?;
        let config_digest = metadata.config.digest()?;
        let descriptor = crate::SegmentDescriptor {
            stamp: ProjectionStamp {
                contract_version: DATA_RUNTIME_CONTRACT_VERSION,
                id: metadata.config.id.clone(),
                generation: metadata.generation,
                source_cursor: metadata.source_cursor,
                config_digest,
                artifact_digest: hex_digest(&expected_digest),
                state: ProjectionState::Ready,
            },
            scope: metadata.config.scope.clone(),
            field: metadata.config.field.clone(),
            dimensions: metadata.config.dimensions,
            metric: metadata.config.metric,
            embedding_model: metadata.config.embedding_model.clone(),
            filter_properties: metadata.config.filter_properties.clone(),
            minimum_cursor: metadata.minimum_cursor,
            candidate_versions: metadata.candidates.len(),
        };
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            metadata,
            vector_offset: header.vector_offset,
            row_stride: header.row_stride,
            storage,
        })
    }

    fn row(&self, row: usize) -> &[u8] {
        let row_bytes = self.metadata.config.dimensions * 4;
        let start = self.vector_offset + row * self.row_stride;
        &self.as_bytes()[start..start + row_bytes]
    }
}
