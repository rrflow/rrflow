//! Vector artifacts and their kinds.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum VectorArtifact {
    ExactSegment(ImmutableVectorSegment),
    CompactDense(CompactDenseSegment),
    Hnsw(HnswIndex),
    Quantized(QuantizedSegment),
    TurboQuant(TurboQuantSegment),
}

/// Exact on-disk codec identity for a cataloged vector artifact. A projection
/// descriptor alone cannot distinguish the JSON exact segment from the compact
/// dense representation because both intentionally plan as `ExactSegment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorArtifactKind {
    ExactSegment,
    CompactDense,
    Hnsw,
    ScalarQuantized,
    ProductQuantized,
    BinaryQuantized,
    TurboQuant,
}

impl VectorArtifactKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExactSegment => "exact_segment",
            Self::CompactDense => "compact_dense",
            Self::Hnsw => "hnsw",
            Self::ScalarQuantized => "scalar_quantized",
            Self::ProductQuantized => "product_quantized",
            Self::BinaryQuantized => "binary_quantized",
            Self::TurboQuant => "turboquant",
        }
    }

    pub const fn media_type(self) -> &'static str {
        match self {
            Self::ExactSegment => "application/vnd.rrflow.vector-exact-segment+json",
            Self::CompactDense => "application/vnd.rrflow.vector-compact-dense",
            Self::Hnsw => "application/vnd.rrflow.vector-hnsw+json",
            Self::ScalarQuantized => "application/vnd.rrflow.vector-quantized-scalar",
            Self::ProductQuantized => "application/vnd.rrflow.vector-quantized-product",
            Self::BinaryQuantized => "application/vnd.rrflow.vector-quantized-binary",
            Self::TurboQuant => "application/vnd.rrflow.vector-turboquant",
        }
    }
}

impl VectorArtifact {
    pub fn kind(&self) -> VectorArtifactKind {
        match self {
            Self::ExactSegment(_) => VectorArtifactKind::ExactSegment,
            Self::CompactDense(_) => VectorArtifactKind::CompactDense,
            Self::Hnsw(_) => VectorArtifactKind::Hnsw,
            Self::Quantized(segment) => match segment.descriptor().method {
                QuantizationMethod::Scalar => VectorArtifactKind::ScalarQuantized,
                QuantizationMethod::Product { .. } => VectorArtifactKind::ProductQuantized,
                QuantizationMethod::Binary => VectorArtifactKind::BinaryQuantized,
            },
            Self::TurboQuant(_) => VectorArtifactKind::TurboQuant,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::ExactSegment(segment) => segment.as_bytes(),
            Self::CompactDense(segment) => segment.as_bytes(),
            Self::Hnsw(index) => index.as_bytes(),
            Self::Quantized(segment) => segment.as_bytes(),
            Self::TurboQuant(segment) => segment.as_bytes(),
        }
    }

    pub fn from_bytes(kind: VectorArtifactKind, bytes: &[u8]) -> Result<Self> {
        match kind {
            VectorArtifactKind::ExactSegment => {
                ImmutableVectorSegment::from_bytes(bytes).map(Self::ExactSegment)
            }
            VectorArtifactKind::CompactDense => {
                CompactDenseSegment::from_bytes(bytes).map(Self::CompactDense)
            }
            VectorArtifactKind::Hnsw => HnswIndex::from_bytes(bytes).map(Self::Hnsw),
            VectorArtifactKind::ScalarQuantized
            | VectorArtifactKind::ProductQuantized
            | VectorArtifactKind::BinaryQuantized => {
                let artifact = QuantizedSegment::from_bytes(bytes).map(Self::Quantized)?;
                if artifact.kind() != kind {
                    return invalid("quantized artifact method differs from its codec kind");
                }
                Ok(artifact)
            }
            VectorArtifactKind::TurboQuant => {
                TurboQuantSegment::from_bytes(bytes).map(Self::TurboQuant)
            }
        }
    }

    /// Opens codecs with a native read-only mmap representation. JSON codecs
    /// deliberately return `None`; callers retain the verified owned-byte
    /// fallback rather than pretending they are physically mapped.
    pub fn open_mmap(kind: VectorArtifactKind, path: impl AsRef<Path>) -> Result<Option<Self>> {
        if matches!(
            kind,
            VectorArtifactKind::ExactSegment | VectorArtifactKind::Hnsw
        ) {
            return Ok(None);
        }
        let file = std::fs::File::open(path).map_err(|error| Error::InvalidRuntime {
            reason: format!("cannot open vector artifact: {error}"),
        })?;
        Self::open_mmap_file(kind, file)
    }

    pub fn open_mmap_file(kind: VectorArtifactKind, file: std::fs::File) -> Result<Option<Self>> {
        match kind {
            VectorArtifactKind::CompactDense => CompactDenseSegment::open_mmap_file(file)
                .map(Self::CompactDense)
                .map(Some),
            VectorArtifactKind::ScalarQuantized
            | VectorArtifactKind::ProductQuantized
            | VectorArtifactKind::BinaryQuantized => {
                let artifact = QuantizedSegment::open_mmap_file(file).map(Self::Quantized)?;
                if artifact.kind() != kind {
                    return invalid("mapped quantized artifact method differs from its codec kind");
                }
                Ok(Some(artifact))
            }
            VectorArtifactKind::TurboQuant => TurboQuantSegment::open_mmap_file(file)
                .map(Self::TurboQuant)
                .map(Some),
            VectorArtifactKind::ExactSegment | VectorArtifactKind::Hnsw => Ok(None),
        }
    }

    pub fn descriptor(&self) -> VectorProjectionDescriptor {
        match self {
            Self::ExactSegment(segment) => segment.descriptor().clone().into(),
            Self::CompactDense(segment) => segment.descriptor().clone().into(),
            Self::Hnsw(index) => index.descriptor().clone().into(),
            Self::Quantized(segment) => segment.descriptor().clone().into(),
            Self::TurboQuant(segment) => segment.descriptor().clone().into(),
        }
    }
}

impl From<ImmutableVectorSegment> for VectorArtifact {
    fn from(segment: ImmutableVectorSegment) -> Self {
        Self::ExactSegment(segment)
    }
}

impl From<HnswIndex> for VectorArtifact {
    fn from(index: HnswIndex) -> Self {
        Self::Hnsw(index)
    }
}

impl From<CompactDenseSegment> for VectorArtifact {
    fn from(segment: CompactDenseSegment) -> Self {
        Self::CompactDense(segment)
    }
}

impl From<QuantizedSegment> for VectorArtifact {
    fn from(segment: QuantizedSegment) -> Self {
        Self::Quantized(segment)
    }
}

impl From<TurboQuantSegment> for VectorArtifact {
    fn from(segment: TurboQuantSegment) -> Self {
        Self::TurboQuant(segment)
    }
}
