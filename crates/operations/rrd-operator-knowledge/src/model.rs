//! Operator knowledge access paths, adapter descriptors, bindings, revisions, search controls, requests, hits and results.

use super::*;

pub const OPERATOR_KNOWLEDGE_CONTRACT_VERSION: u16 = 1;
pub(super) const MAX_NAME_BYTES: usize = 160;
pub(super) const MAX_HITS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorAccessPath {
    Exact,
    Hnsw,
    IvfFlat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorVectorKind {
    Dense,
    Sparse,
    MultiDense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterativeScanMode {
    Off,
    StrictOrder,
    RelaxedOrder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorAdapterDescriptor {
    pub contract_version: u16,
    pub adapter: String,
    pub implementation_digest: String,
    pub max_dimensions: u32,
    pub vector_kinds: BTreeSet<OperatorVectorKind>,
    pub search_capabilities: BTreeMap<OperatorAccessPath, BTreeSet<ScoreMetric>>,
    pub supports_tenant_filter: bool,
    pub supports_payload_filter: bool,
    pub supports_stable_revision: bool,
}

impl OperatorAdapterDescriptor {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        validate_name("operator adapter", &self.adapter)?;
        validate_digest("operator implementation", &self.implementation_digest)?;
        if self.max_dimensions == 0 {
            return invalid("operator adapter max dimensions must be greater than zero");
        }
        if self.vector_kinds.is_empty()
            || self.search_capabilities.is_empty()
            || self.search_capabilities.values().any(BTreeSet::is_empty)
        {
            return invalid("operator adapter must declare vector kinds and path-specific metrics");
        }
        Ok(())
    }
}

/// Immutable project-to-external-source binding. Human-readable database,
/// schema, table, column, and tenant names stay in deployment configuration;
/// only their canonical digests cross the kernel boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorKnowledgeBinding {
    pub contract_version: u16,
    pub adapter: String,
    pub project_id: String,
    pub member: String,
    pub scope: ScopeId,
    pub config_digest: String,
    pub source_identity_digest: String,
    pub relation_digest: String,
    pub tenant_digest: String,
    pub model: EmbeddingModelBinding,
    pub dimensions: u32,
    pub projection: ProjectionStamp,
}

impl OperatorKnowledgeBinding {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        validate_name("operator adapter", &self.adapter)?;
        validate_name("operator project", &self.project_id)?;
        validate_name("operator member", &self.member)?;
        validate_digest("operator config", &self.config_digest)?;
        validate_digest("operator source identity", &self.source_identity_digest)?;
        validate_digest("operator relation", &self.relation_digest)?;
        validate_digest("operator tenant", &self.tenant_digest)?;
        self.model.validate()?;
        if self.dimensions == 0 {
            return invalid("operator binding dimensions must be greater than zero");
        }
        self.projection.validate()?;
        if self.projection.source_cursor == 0
            || self.projection.state != ProjectionState::Ready
            || self.projection.config_digest != self.config_digest
        {
            return invalid(
                "operator projection must be ready, configuration-bound, and cover a non-zero source cursor",
            );
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        content_digest(b"rrd-operator-knowledge-binding-v1\0", self)
    }
}

/// The exact external visibility coordinate observed by one search. Snapshot
/// identity is mandatory; WAL position is optional supporting evidence and is
/// never treated as an equivalent snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSourceRevision {
    pub contract_version: u16,
    pub adapter: String,
    pub project_id: String,
    pub source_identity_digest: String,
    pub snapshot_digest: String,
    pub catalog_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stable_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wal_lsn_digest: Option<String>,
}

impl OperatorSourceRevision {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        validate_name("operator revision adapter", &self.adapter)?;
        validate_name("operator revision project", &self.project_id)?;
        validate_digest("operator source identity", &self.source_identity_digest)?;
        validate_digest("operator snapshot", &self.snapshot_digest)?;
        validate_digest("operator catalog", &self.catalog_digest)?;
        if let Some(revision) = &self.stable_revision {
            validate_name("operator stable revision", revision)?;
        }
        if let Some(lsn) = &self.wal_lsn_digest {
            validate_digest("operator WAL LSN", lsn)?;
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        content_digest(b"rrd-operator-knowledge-source-revision-v1\0", self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSearchControls {
    pub requested_path: OperatorAccessPath,
    pub iterative_scan: IterativeScanMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hnsw_ef_search: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hnsw_max_scan_tuples: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ivfflat_probes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ivfflat_max_probes: Option<u32>,
}

impl OperatorSearchControls {
    pub fn exact() -> Self {
        Self {
            requested_path: OperatorAccessPath::Exact,
            iterative_scan: IterativeScanMode::Off,
            hnsw_ef_search: None,
            hnsw_max_scan_tuples: None,
            ivfflat_probes: None,
            ivfflat_max_probes: None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self.requested_path {
            OperatorAccessPath::Exact => {
                if self.iterative_scan != IterativeScanMode::Off
                    || self.hnsw_ef_search.is_some()
                    || self.hnsw_max_scan_tuples.is_some()
                    || self.ivfflat_probes.is_some()
                    || self.ivfflat_max_probes.is_some()
                {
                    return invalid("exact operator search cannot declare ANN controls");
                }
            }
            OperatorAccessPath::Hnsw => {
                bounded_positive("hnsw ef_search", self.hnsw_ef_search, 1_000_000)?;
                bounded_positive(
                    "hnsw max_scan_tuples",
                    self.hnsw_max_scan_tuples,
                    100_000_000,
                )?;
                if self.ivfflat_probes.is_some() || self.ivfflat_max_probes.is_some() {
                    return invalid("HNSW operator search cannot declare IVFFlat controls");
                }
            }
            OperatorAccessPath::IvfFlat => {
                if self.iterative_scan == IterativeScanMode::StrictOrder {
                    return invalid(
                        "IVFFlat operator search supports off or relaxed-order iteration only",
                    );
                }
                bounded_positive("ivfflat probes", self.ivfflat_probes, 1_000_000)?;
                bounded_positive("ivfflat max_probes", self.ivfflat_max_probes, 1_000_000)?;
                if let (Some(probes), Some(max_probes)) =
                    (self.ivfflat_probes, self.ivfflat_max_probes)
                {
                    if max_probes < probes {
                        return invalid("ivfflat max_probes cannot be lower than probes");
                    }
                }
                if self.hnsw_ef_search.is_some() || self.hnsw_max_scan_tuples.is_some() {
                    return invalid("IVFFlat operator search cannot declare HNSW controls");
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSearchRequest {
    pub contract_version: u16,
    pub binding_digest: String,
    /// Lowest canonical RRD cursor the external projection must cover.
    pub required_source_cursor: u64,
    pub search: SearchRequest,
    pub controls: OperatorSearchControls,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_stable_revision: Option<String>,
}

impl OperatorSearchRequest {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        validate_digest("operator binding", &self.binding_digest)?;
        self.search.validate()?;
        if self.required_source_cursor == 0
            || self.required_source_cursor > self.search.read.commit_cursor
        {
            return invalid(
                "operator required source cursor must be non-zero and within the read stamp",
            );
        }
        self.controls.validate()?;
        match (self.search.mode, self.controls.requested_path) {
            (SearchMode::Exact, OperatorAccessPath::Exact)
            | (
                SearchMode::AllowApproximate { .. } | SearchMode::RequireApproximate { .. },
                OperatorAccessPath::Hnsw | OperatorAccessPath::IvfFlat,
            ) => {}
            _ => {
                return invalid("operator vector mode and requested external access path disagree")
            }
        }
        if let Some(revision) = &self.expected_stable_revision {
            validate_name("expected operator revision", revision)?;
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        content_digest(b"rrd-operator-knowledge-search-request-v1\0", self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSearchHit {
    pub external_id: String,
    pub subject_id: String,
    pub score: f64,
}

impl OperatorSearchHit {
    fn validate(&self) -> Result<()> {
        validate_name("operator hit identity", &self.external_id)?;
        validate_name("operator hit subject", &self.subject_id)?;
        if !self.score.is_finite() {
            return invalid("operator hit score must be finite");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorPlanEvidence {
    pub selected_path: OperatorAccessPath,
    pub plan_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason_digest: Option<String>,
    pub controls: OperatorSearchControls,
    pub filter_applied_after_ann: bool,
    pub ordering_exact: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidates_examined: Option<u64>,
}

impl OperatorPlanEvidence {
    fn validate(&self) -> Result<()> {
        validate_digest("operator plan", &self.plan_digest)?;
        if let Some(index) = &self.index_digest {
            validate_digest("operator index", index)?;
        }
        if let Some(reason) = &self.fallback_reason_digest {
            validate_digest("operator fallback reason", reason)?;
        }
        self.controls.validate()?;
        if self.controls.requested_path != self.selected_path
            && self.fallback_reason_digest.is_none()
        {
            return invalid("operator access-path fallback requires a reason digest");
        }
        if self.selected_path == OperatorAccessPath::Exact && self.filter_applied_after_ann {
            return invalid("exact operator search cannot report post-ANN filtering");
        }
        if self.selected_path == OperatorAccessPath::Exact && !self.ordering_exact {
            return invalid("exact operator search must report exact ordering");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSearchResult {
    pub contract_version: u16,
    pub request_digest: String,
    pub revision: OperatorSourceRevision,
    pub plan: OperatorPlanEvidence,
    pub hits: Vec<OperatorSearchHit>,
    pub elapsed_micros: u64,
}

impl OperatorSearchResult {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        validate_digest("operator request", &self.request_digest)?;
        self.revision.validate()?;
        self.plan.validate()?;
        if self.hits.len() > MAX_HITS {
            return invalid("operator result exceeds the hit budget");
        }
        let mut seen = BTreeSet::new();
        for hit in &self.hits {
            hit.validate()?;
            if !seen.insert((&hit.external_id, &hit.subject_id)) {
                return invalid("operator result contains a duplicate hit identity");
            }
        }
        Ok(())
    }
}

pub trait OperatorKnowledgeAdapter {
    fn descriptor(&self) -> &OperatorAdapterDescriptor;

    fn search(
        &mut self,
        binding: &OperatorKnowledgeBinding,
        request: &OperatorSearchRequest,
    ) -> Result<OperatorSearchResult>;
}
