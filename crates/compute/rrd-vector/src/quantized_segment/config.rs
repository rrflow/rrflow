//! Quantized segment format, compression, methods, kernels, configuration, descriptor and storage.

use super::*;

pub const QUANTIZED_SEGMENT_FORMAT_VERSION: u16 = 1;
pub(super) const MAGIC: &[u8; 8] = b"RRDQNT01";
pub(super) const HEADER_BYTES: usize = 128;
pub(super) const DIGEST_OFFSET: usize = 72;
pub(super) const DIGEST_BYTES: usize = 32;
pub(super) const ALIGNMENT: usize = 64;
pub(super) const MAX_ARTIFACT_BYTES: usize = 1 << 30;
pub(super) const MAX_CANDIDATES: usize = 10_000_000;
pub(super) const MAX_DIMENSIONS: usize = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductCompression {
    X4,
    X8,
    X16,
    X32,
    X64,
}

impl ProductCompression {
    pub const fn ratio(self) -> usize {
        match self {
            Self::X4 => 4,
            Self::X8 => 8,
            Self::X16 => 16,
            Self::X32 => 32,
            Self::X64 => 64,
        }
    }

    pub(super) const fn block_dimensions(self) -> usize {
        self.ratio() / std::mem::size_of::<f32>()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum QuantizationMethod {
    Scalar,
    Product { compression: ProductCompression },
    Binary,
}

impl QuantizationMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Product { .. } => "product",
            Self::Binary => "binary",
        }
    }

    pub const fn maximum_compression_ratio(self) -> usize {
        match self {
            Self::Scalar => 4,
            Self::Product { compression } => compression.ratio(),
            Self::Binary => 32,
        }
    }

    pub const fn access_path_kind(self) -> AccessPathKind {
        match self {
            Self::Scalar => AccessPathKind::ScalarQuantized,
            Self::Product { .. } => AccessPathKind::ProductQuantized,
            Self::Binary => AccessPathKind::BinaryQuantized,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizedKernel {
    Scalar,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizedMemoryPlacement {
    Owned,
    Mapped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantizedSegmentConfig {
    pub id: ProjectionId,
    pub scope: ScopeId,
    pub field: String,
    pub dimensions: usize,
    pub metric: ScoreMetric,
    pub method: QuantizationMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<EmbeddingModelBinding>,
    #[serde(default)]
    pub filter_properties: BTreeSet<String>,
}

impl QuantizedSegmentConfig {
    pub fn validate(&self) -> Result<()> {
        if self.field.trim().is_empty() || self.field.as_bytes().contains(&0) {
            return invalid("quantized segment field must be non-empty and contain no NUL bytes");
        }
        if self.dimensions == 0 || self.dimensions > MAX_DIMENSIONS {
            return invalid("quantized segment dimensions must be in 1..=1048576");
        }
        if self
            .filter_properties
            .iter()
            .any(|property| property.trim().is_empty() || property.as_bytes().contains(&0))
        {
            return invalid("quantized segment filter properties must be valid names");
        }
        if let Some(model) = &self.embedding_model {
            model.validate()?;
        }
        Ok(())
    }

    pub(super) fn digest(&self) -> Result<String> {
        self.validate()?;
        encode_json(self).map(|bytes| digest::sha256_hex(&bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantizedDescriptor {
    pub stamp: ProjectionStamp,
    pub scope: ScopeId,
    pub field: String,
    pub dimensions: usize,
    pub metric: ScoreMetric,
    pub method: QuantizationMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<EmbeddingModelBinding>,
    #[serde(default)]
    pub filter_properties: BTreeSet<String>,
    pub minimum_cursor: u64,
    pub candidate_versions: usize,
    pub packed_vector_bytes: usize,
    pub full_precision_vector_bytes: usize,
    pub auxiliary_bytes: usize,
}

impl QuantizedDescriptor {
    pub fn validate(&self) -> Result<()> {
        self.stamp.validate()?;
        let expected_full = self
            .candidate_versions
            .checked_mul(self.dimensions)
            .and_then(|value| value.checked_mul(std::mem::size_of::<f32>()))
            .ok_or_else(|| runtime_error("quantized full-precision byte count overflow"))?;
        if self.minimum_cursor != 0
            || self.minimum_cursor > self.stamp.source_cursor
            || self.dimensions == 0
            || self.dimensions > MAX_DIMENSIONS
            || self.candidate_versions == 0
            || self.candidate_versions > MAX_CANDIDATES
            || self.packed_vector_bytes == 0
            || self.full_precision_vector_bytes != expected_full
            || self.full_precision_vector_bytes / self.packed_vector_bytes
                > self.method.maximum_compression_ratio()
        {
            return invalid("quantized descriptor coverage or byte accounting is invalid");
        }
        if let Some(model) = &self.embedding_model {
            model.validate()?;
        }
        Ok(())
    }

    pub fn candidate_path(&self, estimated_cost: u64) -> CandidatePath {
        CandidatePath {
            stamp: self.stamp.clone(),
            kind: self.method.access_path_kind(),
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "layout", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum QuantizedLayout {
    Scalar {
        scale: f32,
    },
    Product {
        block_dimensions: usize,
        subspaces: usize,
        codebook_size: usize,
        /// One flat `(codebook_size * block_dimensions)` table per subspace.
        codebooks: Vec<Vec<f32>>,
    },
    Binary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SegmentMetadata {
    pub(super) config: QuantizedSegmentConfig,
    pub(super) generation: u64,
    pub(super) source_cursor: u64,
    pub(super) minimum_cursor: u64,
    pub(super) row_bytes: usize,
    pub(super) layout: QuantizedLayout,
    pub(super) candidates: Vec<StoredCandidate>,
}

#[derive(Debug, Clone)]
pub(super) enum QuantizedStorage {
    Owned(Arc<[u8]>),
    Mapped(Arc<Mmap>),
}

impl QuantizedStorage {
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
pub struct QuantizedSegment {
    pub(super) descriptor: QuantizedDescriptor,
    pub(super) metadata: SegmentMetadata,
    pub(super) payload_offset: usize,
    pub(super) storage: QuantizedStorage,
}

impl PartialEq for QuantizedSegment {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor == other.descriptor && self.as_bytes() == other.as_bytes()
    }
}
