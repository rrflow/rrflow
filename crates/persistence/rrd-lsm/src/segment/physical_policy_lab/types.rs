//! Physical-policy trial configuration, observations, corpus and cache models.

use super::*;

pub const PHYSICAL_POLICY_EVIDENCE_VERSION: u16 = 4;

pub(super) const FAMILY_PREFIXES: [(&str, &[u8]); 8] = [
    ("audit", b"audit/"),
    ("edge-in", b"edge/in/"),
    ("edge-out", b"edge/out/"),
    ("record", b"record/"),
    ("runtime", b"runtime/"),
    ("scalar", b"scalar/"),
    ("term", b"term/"),
    ("vector", b"vector/"),
];
pub(super) const FILTER_FALSE_POSITIVE_LIMIT_PPM: u64 = 20_000;
pub(super) const ADAPTIVE_CODEC_MINIMUM_SAVINGS_BPS: u64 = 1_250;
pub(super) const ADAPTIVE_CODEC_FRAME_BYTES: usize = 8;
pub(super) const VALUE_POINTER_BYTES: usize = 16;
pub(super) const MAX_RECORDS_PER_FAMILY: usize = 16_384;
pub(super) const MAX_VERSIONS_PER_KEY: usize = 4;
pub(super) const MAX_VALUE_BYTES: usize = 1024 * 1024;
pub(super) const MAX_POINT_MISSES: usize = 1_000_000;
pub(super) const MAX_CACHE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalPolicyConfig {
    pub seed: u64,
    pub records_per_family: usize,
    pub versions_per_key: usize,
    pub value_bytes: usize,
    pub point_misses: usize,
    pub cache_bytes: usize,
}

impl PhysicalPolicyConfig {
    pub fn validate(self) -> Result<Self> {
        if self.records_per_family == 0
            || self.records_per_family > MAX_RECORDS_PER_FAMILY
            || self.versions_per_key == 0
            || self.versions_per_key > MAX_VERSIONS_PER_KEY
            || self.value_bytes == 0
            || self.value_bytes > MAX_VALUE_BYTES
            || self.point_misses == 0
            || self.point_misses > MAX_POINT_MISSES
            || self.cache_bytes == 0
            || self.cache_bytes > MAX_CACHE_BYTES
        {
            return Err(Error::InvalidConfiguration(format!(
                "physical-policy lab requires records_per_family=1..={MAX_RECORDS_PER_FAMILY}, versions_per_key=1..={MAX_VERSIONS_PER_KEY}, value_bytes=1..={MAX_VALUE_BYTES}, point_misses=1..={MAX_POINT_MISSES}, and cache_bytes=1..={MAX_CACHE_BYTES}"
            )));
        }
        let operations = self
            .records_per_family
            .checked_mul(FAMILY_PREFIXES.len())
            .and_then(|value| value.checked_mul(self.versions_per_key))
            .ok_or_else(|| {
                Error::InvalidConfiguration("physical-policy operation count overflow".into())
            })?;
        let estimated_payload = operations
            .checked_mul(self.value_bytes.saturating_add(64))
            .ok_or_else(|| {
                Error::InvalidConfiguration("physical-policy payload estimate overflow".into())
            })?;
        if estimated_payload > crate::WAL_MAX_PAYLOAD_BYTES {
            return Err(Error::InvalidConfiguration(format!(
                "physical-policy corpus estimate {estimated_payload} exceeds one bounded WAL payload of {} bytes",
                crate::WAL_MAX_PAYLOAD_BYTES
            )));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateDecision {
    pub candidate: String,
    pub decision: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodecObservation {
    pub codec: String,
    pub pages: u64,
    pub logical_bytes: u64,
    pub codec_bytes: u64,
    pub adaptive_stored_bytes: u64,
    pub adaptive_selected_pages: u64,
    pub savings_basis_points: u64,
    pub encode_nanoseconds: u64,
    pub decode_nanoseconds: u64,
    pub round_trip_exact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterObservation {
    pub format: String,
    pub filters: u64,
    pub unique_keys: u64,
    pub serialized_bytes: u64,
    pub bits_per_key: u64,
    pub hash_functions: u64,
    pub member_queries: u64,
    pub member_false_negatives: u64,
    pub absent_queries: u64,
    pub false_positives: u64,
    pub false_positive_parts_per_million: u64,
    pub passes_policy_threshold: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachePolicyObservation {
    pub policy: String,
    pub capacity_bytes: u64,
    pub peak_resident_bytes: u64,
    pub final_resident_bytes: u64,
    pub hits: u64,
    pub misses: u64,
    pub admissions: u64,
    pub evictions: u64,
    pub post_scan_hot_hits: u64,
    pub post_scan_hot_requests: u64,
    pub semantic_identity_exact: bool,
    pub exact_capacity_respected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValuePlacementObservation {
    pub evidence_kind: String,
    pub non_null_versions: u64,
    pub inline_value_bytes: u64,
    pub latest_live_value_bytes: u64,
    pub obsolete_value_bytes: u64,
    pub separated_pointer_bytes: u64,
    pub append_only_value_log_bytes: u64,
    pub modeled_compaction_bytes_avoided: u64,
    pub production_retainable: bool,
    pub missing_proofs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyObservation {
    pub family: String,
    pub rows: u64,
    pub page_count: u64,
    pub logical_page_bytes: u64,
    pub lz4_adaptive_bytes: u64,
    pub zstd_adaptive_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReopenedPointMissObservation {
    pub integrated_rrflowkv: bool,
    pub open_none_policy_segment_count: u64,
    pub open_adaptive_lz4_policy_segment_count: u64,
    pub open_raw_page_count: u64,
    pub open_compressed_page_count: u64,
    pub open_stored_page_bytes: u64,
    pub open_logical_page_bytes: u64,
    pub open_persisted_filter_count: u64,
    pub open_persisted_filter_bytes: u64,
    pub open_semantic_page_operations: u64,
    pub open_semantic_page_bytes: u64,
    pub misses_verified: u64,
    pub hit_samples_verified: u64,
    pub elapsed_nanoseconds: u64,
    pub segment_physical_bytes: u64,
    pub filter_checks: u64,
    pub filter_negatives: u64,
    pub page_cache_hits: u64,
    pub page_cache_misses: u64,
    pub page_loads: u64,
    pub bytes_read: u64,
    pub bytes_decoded: u64,
    pub bytes_decompressed: u64,
    pub final_cache_resident_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegratedCachePolicyObservation {
    pub policy: String,
    pub protected_capacity_basis_points: u64,
    pub snapshot_sequence: u64,
    pub manifest_digest: String,
    pub semantic_digest: String,
    pub projected_row_digest: String,
    pub hot_family_count: u64,
    pub hot_requests_per_pass: u64,
    pub projected_rows: u64,
    pub capacity_bytes: u64,
    pub final_resident_bytes: u64,
    pub probationary_resident_bytes: u64,
    pub protected_resident_bytes: u64,
    pub final_entries: u64,
    pub probationary_entries: u64,
    pub protected_entries: u64,
    pub warm_hits: u64,
    pub warm_misses: u64,
    pub warm_loads: u64,
    pub scan_hits: u64,
    pub scan_misses: u64,
    pub scan_loads: u64,
    pub post_scan_hot_hits: u64,
    pub post_scan_hot_misses: u64,
    pub post_scan_hot_loads: u64,
    pub admissions: u64,
    pub admission_rejections: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub same_scope_hits: u64,
    pub evictions: u64,
    pub duplicate_loads: u64,
    pub bytes_read: u64,
    pub bytes_decoded: u64,
    pub bytes_decompressed: u64,
    pub semantic_identity_exact: bool,
    pub exact_capacity_respected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalPolicyTrial {
    pub evidence_version: u16,
    pub evidence_scope: String,
    pub config: PhysicalPolicyConfig,
    pub corpus_digest: String,
    pub operation_count: u64,
    pub key_count: u64,
    pub row_group_count: u64,
    pub page_count: u64,
    pub segment_format_version: u16,
    pub segment_physical_bytes: u64,
    pub logical_page_bytes: u64,
    pub elapsed_nanoseconds: u64,
    pub peak_rss_bytes: Option<u64>,
    pub codecs: Vec<CodecObservation>,
    pub persisted_row_group_filter: FilterObservation,
    pub cache_candidates: Vec<CachePolicyObservation>,
    pub value_placement_candidate: ValuePlacementObservation,
    pub families: Vec<FamilyObservation>,
    pub reopened_point_miss: ReopenedPointMissObservation,
    pub integrated_cache_policies: Vec<IntegratedCachePolicyObservation>,
    pub decisions: Vec<CandidateDecision>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct Corpus {
    pub(super) mutations: Vec<Mutation>,
    pub(super) expected_latest: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    pub(super) digest: String,
}

#[derive(Debug, Clone)]
pub(super) struct PageSample {
    pub(super) id: usize,
    pub(super) row_group: usize,
    pub(super) row_count: u64,
    pub(super) family: String,
    pub(super) kind: &'static str,
    pub(super) bytes: Vec<u8>,
}

#[derive(Debug, Default)]
pub(super) struct CodecAccumulator {
    pub(super) pages: u64,
    pub(super) logical_bytes: u64,
    pub(super) codec_bytes: u64,
    pub(super) adaptive_stored_bytes: u64,
    pub(super) selected_pages: u64,
    pub(super) encode_nanoseconds: u64,
    pub(super) decode_nanoseconds: u64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CacheAccess {
    pub(super) page_id: usize,
    pub(super) bytes: usize,
    pub(super) post_scan_hot: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CacheEntry {
    pub(super) bytes: usize,
    pub(super) stamp: u64,
}

#[derive(Debug)]
pub(super) struct ExactLru {
    pub(super) capacity: usize,
    pub(super) resident: usize,
    pub(super) peak: usize,
    pub(super) clock: u64,
    pub(super) entries: BTreeMap<usize, CacheEntry>,
    pub(super) observation: CachePolicyObservation,
}

#[derive(Debug)]
pub(super) struct SegmentedLru {
    pub(super) capacity: usize,
    pub(super) protected_capacity: usize,
    pub(super) probation_capacity: usize,
    pub(super) protected_resident: usize,
    pub(super) probation_resident: usize,
    pub(super) peak: usize,
    pub(super) clock: u64,
    pub(super) protected: BTreeMap<usize, CacheEntry>,
    pub(super) probation: BTreeMap<usize, CacheEntry>,
    pub(super) observation: CachePolicyObservation,
}
