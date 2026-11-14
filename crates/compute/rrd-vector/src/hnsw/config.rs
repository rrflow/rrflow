//! HNSW format constants, configuration, descriptors and the persisted graph body.

use super::*;

pub const HNSW_FORMAT_VERSION: u16 = 2;
pub(super) const HNSW_MAGIC: &str = "RRDHNS02";
pub(super) const MAX_HNSW_BYTES: usize = 1 << 30;
pub(super) const MAX_HNSW_NODES: usize = 10_000_000;
// A graph visit performs vector scoring plus heap/navigation work. Four
// score-equivalent units is deliberately conservative at filter crossovers;
// callers may still force ANN through `RequireApproximate`.
pub(super) const HNSW_NAVIGATION_COST_MULTIPLIER: usize = 4;
/// At most this many identities inform one cost estimate. A planner compares costs roughly, and
/// visiting every identity on every query took 44% of a search's time on 20,000 points.
pub(super) const COST_SAMPLE_IDENTITIES: usize = 512;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HnswMaintenanceKind {
    #[default]
    FullBuild,
    Incremental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HnswKernel {
    Scalar,
    /// Runtime-dispatched AVX2 on supported x86_64 hosts, NEON on aarch64,
    /// otherwise scalar. Lanes are f32, so traversal can round differently
    /// from scalar, and final exact reranking stays authoritative.
    Auto,
}

pub(super) fn is_full_build(value: &HnswMaintenanceKind) -> bool {
    *value == HnswMaintenanceKind::FullBuild
}

pub(super) fn is_zero(value: &usize) -> bool {
    *value == 0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HnswConfig {
    pub id: ProjectionId,
    pub scope: ScopeId,
    pub field: String,
    pub dimensions: usize,
    pub metric: ScoreMetric,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<EmbeddingModelBinding>,
    pub m: usize,
    pub ef_construction: usize,
    pub max_level: u8,
    pub seed: u64,
    #[serde(default)]
    pub filter_properties: BTreeSet<String>,
}

impl HnswConfig {
    pub fn validate(&self) -> Result<()> {
        if self.field.trim().is_empty() || self.field.as_bytes().contains(&0) {
            return invalid("HNSW field must be non-empty and contain no NUL bytes");
        }
        if self.dimensions == 0 || self.dimensions > 1_048_576 {
            return invalid("HNSW dimensions must be in 1..=1048576");
        }
        if !(2..=128).contains(&self.m) {
            return invalid("HNSW m must be in 2..=128");
        }
        if self.ef_construction < self.m || self.ef_construction > 1_000_000 {
            return invalid("HNSW ef_construction must be in m..=1000000");
        }
        if self.max_level == 0 || self.max_level > 32 {
            return invalid("HNSW max_level must be in 1..=32");
        }
        if self
            .filter_properties
            .iter()
            .any(|property| property.trim().is_empty() || property.as_bytes().contains(&0))
        {
            return invalid("HNSW filter properties must be valid names");
        }
        if let Some(model) = &self.embedding_model {
            model.validate()?;
        }
        Ok(())
    }

    pub(super) fn digest(&self) -> Result<String> {
        self.validate()?;
        Ok(digest::sha256_hex(&encode_json(self)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HnswDescriptor {
    pub stamp: ProjectionStamp,
    pub scope: ScopeId,
    pub field: String,
    pub dimensions: usize,
    pub metric: ScoreMetric,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<EmbeddingModelBinding>,
    pub m: usize,
    pub ef_construction: usize,
    pub max_level: u8,
    pub nodes: usize,
    #[serde(default)]
    pub filter_properties: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "is_full_build")]
    pub maintenance: HnswMaintenanceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_generation: Option<u64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub indexed_delta_vectors: usize,
}

impl HnswDescriptor {
    pub fn validate(&self) -> Result<()> {
        self.stamp.validate()?;
        if self.field.trim().is_empty()
            || self.field.as_bytes().contains(&0)
            || self.dimensions == 0
            || !(2..=128).contains(&self.m)
            || self.ef_construction < self.m
            || self.ef_construction > 1_000_000
            || self.max_level == 0
            || self.max_level > 32
            || self.nodes > MAX_HNSW_NODES
            || self
                .filter_properties
                .iter()
                .any(|property| property.trim().is_empty() || property.as_bytes().contains(&0))
        {
            return invalid("HNSW descriptor contains invalid build parameters");
        }
        if let Some(model) = &self.embedding_model {
            model.validate()?;
        }
        match self.maintenance {
            HnswMaintenanceKind::FullBuild if self.previous_generation.is_some() => {
                return invalid("full-build HNSW descriptor cannot name a previous generation");
            }
            HnswMaintenanceKind::Incremental
                if self.previous_generation != self.stamp.generation.checked_sub(1)
                    || self.indexed_delta_vectors == 0
                    || self.indexed_delta_vectors > self.nodes =>
            {
                return invalid("incremental HNSW descriptor has incoherent maintenance evidence");
            }
            HnswMaintenanceKind::FullBuild | HnswMaintenanceKind::Incremental => {}
        }
        Ok(())
    }

    pub fn candidate_path(&self, estimated_cost: u64) -> CandidatePath {
        CandidatePath {
            stamp: self.stamp.clone(),
            kind: AccessPathKind::Hnsw,
            field: self.field.clone(),
            dimensions: self.dimensions,
            metric: self.metric,
            embedding_model: self.embedding_model.clone(),
            filter_properties: self.filter_properties.clone(),
            estimated_candidates: self.nodes as u64,
            estimated_cost,
            overlay_source_cursor: None,
            overlay_candidates: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HnswNode {
    pub(super) candidate: VectorCandidate,
    pub(super) level: u8,
    /// Neighbor ids by layer, layer zero first.
    pub(super) neighbors: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HnswBody {
    pub(super) config: HnswConfig,
    pub(super) generation: u64,
    pub(super) source_cursor: u64,
    pub(super) entrypoint: Option<usize>,
    pub(super) nodes: Vec<HnswNode>,
    #[serde(default, skip_serializing_if = "is_full_build")]
    pub(super) maintenance: HnswMaintenanceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) previous_generation: Option<u64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub(super) indexed_delta_vectors: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HnswEnvelope {
    pub(super) magic: String,
    pub(super) format_version: u16,
    pub(super) artifact_digest: String,
    pub(super) body: HnswBody,
}
