//! Diagnostic requests, authority, coverage, graph differences and section types.

use super::*;

pub const DIAGNOSTIC_SNAPSHOT_FORMAT_VERSION: u16 = 1;
pub const MAX_DIAGNOSTIC_AUDIT_RECORDS: u16 = 1_024;
pub const MAX_DIAGNOSTIC_RUNTIME_SCANNED_CHANGES: u64 = 1_000_000;
/// Most graph records one bounded diagnostic read may return.
pub const MAX_DIAGNOSTIC_GRAPH_RECORDS: u64 = 100_000;

/// Physical process-local counters only; no graph/history assembly or claim
/// that independently sampled counters form an authoritative read stamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadStorageDiagnostics {
    pub scope: String,
}

impl ReadStorageDiagnostics {
    pub fn validate(&self) -> crate::Result<()> {
        if self.scope.is_empty()
            || self.scope.len() > crate::MAX_ID_BYTES
            || self.scope.as_bytes().contains(&0)
        {
            return invalid("storage diagnostic scope is invalid");
        }
        Ok(())
    }
}

/// Coordinates a bounded diagnostic read. Cursors are explicit so a client can
/// resume without asking RRD to infer history from UI state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadDiagnosticSnapshot {
    pub scope: String,
    pub graph_valid_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_known_at_cursor: Option<u64>,
    pub graph_compare_cursor: u64,
    /// Most graph records to return, the most linked first; `None` returns
    /// every record. The difference then keeps only entries about returned
    /// records, and both say how much they left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_max_records: Option<u64>,
    pub runtime_max_scanned_changes: u64,
    pub changes_after_cursor: u64,
    pub change_limit: u64,
    pub audit_after_sequence: u64,
    pub audit_limit: u16,
}

impl ReadDiagnosticSnapshot {
    pub fn validate(&self) -> crate::Result<()> {
        if self.scope.is_empty()
            || self.scope.len() > crate::MAX_ID_BYTES
            || self.scope.as_bytes().contains(&0)
        {
            return invalid("diagnostic scope is invalid");
        }
        if self.change_limit == 0 || self.change_limit > MAX_CHANGEFEED_PAGE {
            return invalid(format!(
                "diagnostic change_limit must be in 1..={MAX_CHANGEFEED_PAGE}"
            ));
        }
        if self.runtime_max_scanned_changes == 0
            || self.runtime_max_scanned_changes > MAX_DIAGNOSTIC_RUNTIME_SCANNED_CHANGES
        {
            return invalid(format!(
                "diagnostic runtime_max_scanned_changes must be in 1..={MAX_DIAGNOSTIC_RUNTIME_SCANNED_CHANGES}"
            ));
        }
        if self
            .graph_known_at_cursor
            .is_some_and(|cursor| self.graph_compare_cursor > cursor)
        {
            return invalid("diagnostic graph_compare_cursor exceeds graph_known_at_cursor");
        }
        if self
            .graph_max_records
            .is_some_and(|max| max == 0 || max > MAX_DIAGNOSTIC_GRAPH_RECORDS)
        {
            return invalid(format!(
                "diagnostic graph_max_records must be in 1..={MAX_DIAGNOSTIC_GRAPH_RECORDS}"
            ));
        }
        if self.audit_limit == 0 || self.audit_limit > MAX_DIAGNOSTIC_AUDIT_RECORDS {
            return invalid(format!(
                "diagnostic audit_limit must be in 1..={MAX_DIAGNOSTIC_AUDIT_RECORDS}"
            ));
        }
        Ok(())
    }
}

/// The authoritative coordinates that remained stable while every diagnostic
/// section was assembled. RRD retries instead of returning a mixed-time view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticReadStamp {
    pub claim_sequence: u64,
    pub control_sequence: u64,
    pub runtime_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_revision: Option<u64>,
    pub catalogue_revision: u64,
    pub runtime_manifest_sha256: String,
    pub retention_sha256: String,
    pub assembly_attempts: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticAuthority {
    /// Canonical state read under the verified read stamp.
    Authoritative,
    /// Derived from authoritative state under the verified read stamp.
    Projection,
    /// A process-local measurement taken at `observed_at_unix_ms`. It is not
    /// state, is not bound to the read stamp, and resets when the process
    /// reopens its store.
    Observation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCoverage {
    Complete,
    Bounded,
}

/// Machine-readable disclosure for every section in the response. Connectome
/// renders these facts; it does not invent authority or completeness labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSectionSnapshot {
    pub id: CanonicalId,
    pub authority: DiagnosticAuthority,
    pub coverage: DiagnosticCoverage,
    pub row_count: u64,
    pub known_at_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_after: Option<u64>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticModelKind {
    Record,
    Relation,
    Event,
}

/// One stable logical model summary derived from the authoritative schema.
/// Connectome may render the full schema, but it does not classify model kinds
/// or recompute constraint counts itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticModelSnapshot {
    pub id: CanonicalId,
    pub kind: DiagnosticModelKind,
    pub property_count: u64,
    pub required_property_count: u64,
    pub constraint_count: u64,
    pub allow_additional_properties: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticModelCatalogueSnapshot {
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_revision: Option<u64>,
    pub models: Vec<DiagnosticModelSnapshot>,
}

/// Lossless runtime identity used by diagnostics. Runtime IDs intentionally
/// admit causal/generation separators such as `:` and `@`; they are not the
/// URL-safe public resource IDs represented by `CanonicalId`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRuntimeReference {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticGraphRecordSnapshot {
    pub reference: DiagnosticRuntimeReference,
    pub valid_from_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to_unix_ms: Option<u64>,
    pub properties: DataProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticGraphRelationSnapshot {
    pub reference: DiagnosticRuntimeReference,
    pub from: DiagnosticRuntimeReference,
    pub to: DiagnosticRuntimeReference,
    pub valid_from_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to_unix_ms: Option<u64>,
    pub properties: DataProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticGraphSnapshot {
    pub scope: String,
    pub valid_at_unix_ms: u64,
    pub known_at_cursor: u64,
    pub records: Vec<DiagnosticGraphRecordSnapshot>,
    pub relations: Vec<DiagnosticGraphRelationSnapshot>,
    /// Records and relations at this cursor before `graph_max_records`.
    pub records_total: u64,
    pub relations_total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticGraphRecordChange {
    pub before: DiagnosticGraphRecordSnapshot,
    pub after: DiagnosticGraphRecordSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticGraphRelationChange {
    pub before: DiagnosticGraphRelationSnapshot,
    pub after: DiagnosticGraphRelationSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticGraphDifference {
    pub valid_at_unix_ms: u64,
    pub from_cursor: u64,
    pub to_cursor: u64,
    pub added_records: Vec<DiagnosticGraphRecordSnapshot>,
    pub removed_records: Vec<DiagnosticGraphRecordSnapshot>,
    pub changed_records: Vec<DiagnosticGraphRecordChange>,
    pub added_relations: Vec<DiagnosticGraphRelationSnapshot>,
    pub removed_relations: Vec<DiagnosticGraphRelationSnapshot>,
    pub changed_relations: Vec<DiagnosticGraphRelationChange>,
    /// Entries left out because they concern records the bound did not return.
    pub omitted_entries: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSnapshotLease {
    pub id_sha256: String,
    pub scope: String,
    pub owner: String,
    pub created_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub runtime_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_revision: Option<u64>,
    pub catalogue_revision: u64,
    pub runtime_manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRetentionPin {
    pub id_sha256: String,
    pub snapshot_id_sha256: String,
    pub scope: String,
    pub runtime_manifest_sha256: String,
    pub minimum_cursor: u64,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRetentionSnapshot {
    pub observed_at_unix_ms: u64,
    pub leases: Vec<DiagnosticSnapshotLease>,
    pub pins: Vec<DiagnosticRetentionPin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oldest_retained_cursor: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticVectorArtifactKind {
    ExactSegment,
    CompactDense,
    Hnsw,
    TurboQuant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticVectorArtifactSnapshot {
    pub catalogue_revision: u64,
    pub kind: DiagnosticVectorArtifactKind,
    pub scope: String,
    pub projection_id: String,
    pub generation: u64,
    pub source_cursor: u64,
    pub config_sha256: String,
    pub artifact_sha256: String,
    pub object: DiagnosticRuntimeReference,
    pub object_sha256: String,
    pub object_length: u64,
    pub media_type: String,
    pub receipt: DataObjectReceipt,
    pub published_at_unix_ms: u64,
    pub entry_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticVectorArtifactCatalogueSnapshot {
    pub scope: String,
    pub revision: u64,
    pub artifacts: Vec<DiagnosticVectorArtifactSnapshot>,
}

/// How much physical evidence the instance's storage profile reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStorageEvidenceLevel {
    /// The profile has no stable physical counters (volatile rrflowMX).
    LogicalOnly,
    /// rrflowKV memtable, WAL, maintenance, segment, page-cache, and segment
    /// I/O counters.
    RrflowKvCounters,
}

/// The rrflowKV mutable write buffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticMemtableSnapshot {
    pub versions: u64,
    /// Version count that triggers an automatic flush.
    pub max_versions: u64,
    pub approximate_bytes: u64,
    pub keys: u64,
    pub key_payload_bytes: u64,
    pub value_payload_bytes: u64,
    pub tombstones: u64,
    pub spilled_chains: u64,
    pub spilled_version_capacity: u64,
    pub version_record_bytes: u64,
    pub owned_bytes_lower_bound: u64,
}

/// The rrflowKV write-ahead log tail not yet flushed into segments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticWriteAheadLogSnapshot {
    pub payload_bytes: u64,
    /// Payload size that triggers an automatic flush.
    pub payload_max_bytes: u64,
}

/// Cumulative flush and compaction work since the process opened the store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticMaintenanceSnapshot {
    pub automatic_flushes: u64,
    pub failed_flushes: u64,
    pub write_stalls: u64,
    pub oversized_batches: u64,
    pub automatic_compactions: u64,
    pub failed_compactions: u64,
    pub compaction_input_bytes: u64,
    pub compaction_output_bytes: u64,
    pub peak_compaction_buffer_bytes: u64,
}

/// The persisted segment set named by the current manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSegmentSetSnapshot {
    pub segments: u64,
    pub segment_bytes: u64,
    pub l0_segments: u64,
    /// L0 segment count that triggers compaction.
    pub l0_compaction_trigger: u64,
    pub compaction_debt_segments: u64,
    pub compaction_target_segment_bytes: u64,
}

/// The bounded decoded-page cache and the page reads it served or missed.
/// Hit, miss, eviction, load, byte, and filter counters are cumulative since
/// the process opened the store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticPageCacheSnapshot {
    pub capacity_bytes: u64,
    pub resident_bytes: u64,
    pub entries: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub page_loads: u64,
    pub bytes_read: u64,
    pub bytes_decoded: u64,
    pub bytes_borrowed: u64,
    pub bytes_allocated: u64,
    pub bytes_copied: u64,
    pub bytes_decompressed: u64,
    pub filter_checks: u64,
    pub filter_negatives: u64,
}

/// The segment read path rrflowKV requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSegmentIoMode {
    Auto,
    Mmap,
    IoUring,
    Bounded,
}

/// Segment file I/O by access path, cumulative since the store opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSegmentIoSnapshot {
    pub requested_mode: DiagnosticSegmentIoMode,
    pub mmap_segments: u64,
    pub io_uring_segments: u64,
    pub bounded_segments: u64,
    pub fallbacks: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_fallback_reason: Option<String>,
    pub read_operations: u64,
    pub mmap_reads: u64,
    pub io_uring_reads: u64,
    pub bounded_reads: u64,
    pub bytes_read: u64,
    pub max_request_bytes: u64,
    pub peak_request_bytes: u64,
    pub max_open_files: u64,
    pub open_files: u64,
    pub peak_open_files: u64,
    pub handle_evictions: u64,
    pub handle_reopens: u64,
    pub handle_denials: u64,
}

/// rrflowKV physical state and counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRrflowKvSnapshot {
    pub physical_sequence: u64,
    pub manifest_generation: u64,
    pub durable_sequence: u64,
    pub memtable: DiagnosticMemtableSnapshot,
    pub write_ahead_log: DiagnosticWriteAheadLogSnapshot,
    pub maintenance: DiagnosticMaintenanceSnapshot,
    pub segment_set: DiagnosticSegmentSetSnapshot,
    pub page_cache: DiagnosticPageCacheSnapshot,
    pub segment_io: DiagnosticSegmentIoSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rrflow_cld: Option<DiagnosticRrflowCldSnapshot>,
}

/// Process-local RRFlowCLD trigger observations for cold-served vectors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRrflowCldSnapshot {
    pub trigger_count: u64,
    pub cold_source_hits: u64,
    pub cold_mapped_hits: u64,
    pub cold_owned_hits: u64,
    pub cache_bypass_hits: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_trigger_at_unix_ms: Option<u64>,
}

/// Process-local physical storage observation for the instance's storage
/// profile. It carries `observation` authority: counters are measured at
/// `observed_at_unix_ms`, are not bound to the read stamp, and reset when the
/// process reopens its store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticStorageAdmission {
    pub checkpoint_entries: u64,
    pub checkpoint_limit: u64,
    pub native_transactions: u64,
    pub native_transaction_bytes: u64,
    /// Actual live physical read views and conservative aggregate pin charge.
    pub active_read_views: u64,
    pub pinned_read_bytes: u64,
    /// Logical owner charges; these are not allocator or process RSS bytes.
    pub retained_mutable_bytes: u64,
    pub retained_projected_bytes: u64,
    /// Owned source-page allocation charges; excludes borrowed mmap residency.
    pub retained_source_bytes: u64,
    pub peak_source_bytes: u64,
    pub denied_source_allocations: u64,
    /// Includes the active request plus admitted waiters, matching admission.
    pub outstanding_requests: u64,
    pub outstanding_logical_bytes: u64,
    pub queued_requests: u64,
    pub active: bool,
    pub fenced: bool,
}
