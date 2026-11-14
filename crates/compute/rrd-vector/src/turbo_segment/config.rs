//! TurboQuant segment format, configuration, descriptor, candidates and storage.

use super::*;

pub const TURBOQUANT_SEGMENT_FORMAT_VERSION: u16 = 1;
pub(super) const TURBOQUANT_SEGMENT_MAGIC: &[u8; 8] = b"RRDTQ001";
pub(super) const HEADER_PREFIX_BYTES: usize = 8 + 2 + 4 + 8 + 64;
pub(super) const MAX_TURBOQUANT_SEGMENT_BYTES: usize = 1 << 30;
pub(super) const MAX_TURBOQUANT_CANDIDATES: usize = 10_000_000;

// The existing best-first comparison puts the worst retained hit at the top
// of the max-heap. Only the requested candidate width is retained here; the
// temporal visibility map and the scan still scale with the artifact size.
pub(super) struct RankedHit(SearchHit);

impl PartialEq for RankedHit {
    fn eq(&self, other: &Self) -> bool {
        SearchHit::compare_best_first(&self.0, &other.0) == Ordering::Equal
    }
}
impl Eq for RankedHit {}
impl PartialOrd for RankedHit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for RankedHit {
    fn cmp(&self, other: &Self) -> Ordering {
        SearchHit::compare_best_first(&self.0, &other.0)
    }
}

pub(super) struct CandidateHits {
    pub(super) limit: usize,
    pub(super) hits: BinaryHeap<RankedHit>,
}

impl CandidateHits {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            limit,
            hits: BinaryHeap::new(),
        }
    }

    /// Whether a hit with these keys would enter the heap, decided before its
    /// references are cloned. It orders exactly as `compare_best_first`.
    pub(super) fn admits(&self, score: f64, reference: &RuntimeRef, source_cursor: u64) -> bool {
        self.hits.len() < self.limit
            || self.hits.peek().is_some_and(|worst| {
                worst
                    .0
                    .score
                    .total_cmp(&score)
                    .then_with(|| reference.cmp(&worst.0.reference))
                    .then_with(|| worst.0.source_cursor.cmp(&source_cursor))
                    == Ordering::Less
            })
    }

    pub(super) fn consider(&mut self, hit: SearchHit) {
        if self.hits.len() < self.limit {
            self.hits.push(RankedHit(hit));
        } else if self
            .hits
            .peek()
            .is_some_and(|worst| SearchHit::compare_best_first(&hit, &worst.0) == Ordering::Less)
        {
            *self
                .hits
                .peek_mut()
                .expect("a full candidate heap is nonempty") = RankedHit(hit);
        }
    }

    pub(super) fn finish(self) -> Vec<SearchHit> {
        let mut hits = self.hits.into_iter().map(|hit| hit.0).collect::<Vec<_>>();
        hits.sort_by(SearchHit::compare_best_first);
        hits
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurboQuantSegmentConfig {
    pub id: ProjectionId,
    pub scope: ScopeId,
    pub field: String,
    pub dimensions: usize,
    pub metric: ScoreMetric,
    pub bits: TurboQuantBits,
    pub seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<EmbeddingModelBinding>,
    #[serde(default)]
    pub filter_properties: BTreeSet<String>,
    /// Absent for MSE, so an MSE configuration, and its digest, read as they
    /// did before the inner-product estimator existed.
    #[serde(default, skip_serializing_if = "TurboQuantEstimator::is_mse")]
    pub estimator: TurboQuantEstimator,
}

impl TurboQuantSegmentConfig {
    pub fn validate(&self) -> Result<()> {
        if self.field.trim().is_empty() || self.field.as_bytes().contains(&0) {
            return invalid("TurboQuant segment field must be non-empty and contain no NUL bytes");
        }
        if self.dimensions == 0 || self.dimensions > 1_048_576 {
            return invalid("TurboQuant segment dimensions must be in 1..=1048576");
        }
        if self
            .filter_properties
            .iter()
            .any(|property| property.trim().is_empty() || property.as_bytes().contains(&0))
        {
            return invalid("TurboQuant filter properties must be valid names");
        }
        if let Some(model) = &self.embedding_model {
            model.validate()?;
        }
        if self.estimator == TurboQuantEstimator::InnerProduct {
            if self.bits == TurboQuantBits::Bits1_5 {
                return invalid("1.5-bit TurboQuant has no inner-product estimator");
            }
            if self.metric == ScoreMetric::Manhattan {
                return invalid(
                    "the TurboQuant inner-product estimator scores dot, cosine and euclidean metrics",
                );
            }
            if inner_product_padded_dimensions(self.dimensions) > MAX_QJL_DIMENSIONS {
                return invalid(format!(
                    "the TurboQuant inner-product estimator accepts at most {MAX_QJL_DIMENSIONS} padded dimensions"
                ));
            }
        }
        Ok(())
    }

    /// S for this configuration, which an inner-product build and search use.
    pub(super) fn projection(&self) -> Result<Option<QjlProjection>> {
        match self.estimator {
            TurboQuantEstimator::Mse => Ok(None),
            TurboQuantEstimator::InnerProduct => {
                QjlProjection::new(inner_product_padded_dimensions(self.dimensions), self.seed)
                    .map(Some)
            }
        }
    }

    /// The digest a generation's stamp carries as its configuration digest.
    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_vec(self)
            .map(|bytes| digest::sha256_hex(&bytes))
            .map_err(|error| rrd_core::Error::InvalidRuntime {
                reason: format!("TurboQuant configuration cannot be encoded: {error}"),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurboQuantDescriptor {
    pub stamp: ProjectionStamp,
    pub scope: ScopeId,
    pub field: String,
    pub dimensions: usize,
    pub metric: ScoreMetric,
    pub bits: TurboQuantBits,
    pub seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<EmbeddingModelBinding>,
    #[serde(default)]
    pub filter_properties: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "TurboQuantEstimator::is_mse")]
    pub estimator: TurboQuantEstimator,
    pub minimum_cursor: u64,
    pub candidate_versions: usize,
    pub packed_vector_bytes: usize,
    pub full_precision_vector_bytes: usize,
}

impl TurboQuantDescriptor {
    pub fn validate(&self) -> Result<()> {
        self.stamp.validate()?;
        if self.minimum_cursor != 0
            || self.minimum_cursor > self.stamp.source_cursor
            || self.dimensions == 0
            || self.candidate_versions > MAX_TURBOQUANT_CANDIDATES
            || self.packed_vector_bytes == 0
            || self.full_precision_vector_bytes
                != self
                    .candidate_versions
                    .saturating_mul(self.dimensions)
                    .saturating_mul(std::mem::size_of::<f32>())
        {
            return invalid("TurboQuant descriptor coverage or byte accounting is invalid");
        }
        if let Some(model) = &self.embedding_model {
            model.validate()?;
        }
        Ok(())
    }

    pub fn candidate_path(&self, estimated_cost: u64) -> CandidatePath {
        CandidatePath {
            stamp: self.stamp.clone(),
            kind: AccessPathKind::TurboQuant,
            field: self.field.clone(),
            dimensions: self.dimensions,
            metric: self.metric,
            embedding_model: self.embedding_model.clone(),
            filter_properties: self.filter_properties.clone(),
            estimated_candidates: self.candidate_versions as u64,
            estimated_cost,
            overlay_source_cursor: None,
            overlay_candidates: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredCandidate {
    pub(super) reference: RuntimeRef,
    pub(super) subject: RuntimeRef,
    pub(super) collection: VectorCollectionAddress,
    pub(super) source_cursor: u64,
    pub(super) valid_from: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) valid_to: Option<u64>,
    #[serde(default)]
    pub(super) properties: RuntimeProperties,
    pub(super) original_norm: f32,
    pub(super) centroid_norm: f32,
    /// The unit residual's length, under the inner-product estimator only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) residual_norm: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SegmentHeader {
    pub(super) config: TurboQuantSegmentConfig,
    pub(super) generation: u64,
    pub(super) source_cursor: u64,
    pub(super) minimum_cursor: u64,
    pub(super) packed_bytes_per_vector: usize,
    pub(super) candidates: Vec<StoredCandidate>,
}

#[derive(Debug, Clone)]
pub(super) enum TurboQuantStorage {
    Owned(Arc<[u8]>),
    Mapped(Arc<Mmap>),
}

impl TurboQuantStorage {
    pub(super) fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes,
            Self::Mapped(bytes) => bytes,
        }
    }

    pub(super) fn placement(&self) -> QuantizedMemoryPlacement {
        match self {
            Self::Owned(_) => QuantizedMemoryPlacement::Owned,
            Self::Mapped(_) => QuantizedMemoryPlacement::Mapped,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurboQuantSegment {
    pub(super) descriptor: TurboQuantDescriptor,
    pub(super) candidates: Vec<StoredCandidate>,
    pub(super) packed_bytes_per_vector: usize,
    pub(super) payload_offset: usize,
    pub(super) payload_len: usize,
    pub(super) storage: TurboQuantStorage,
    /// S for an inner-product generation, materialized on its first search.
    pub(super) projection: Arc<OnceLock<QjlProjection>>,
}

impl PartialEq for TurboQuantSegment {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor == other.descriptor && self.as_bytes() == other.as_bytes()
    }
}
