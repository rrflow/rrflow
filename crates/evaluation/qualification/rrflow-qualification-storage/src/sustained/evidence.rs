//! Native allocation receipts, resource samples, load attempts, lane, corpus, reopen and backup evidence, and results.

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeMetric {
    Observed { value: u64 },
    Unavailable { reason: Id },
}
impl NativeMetric {
    pub fn observed(&self) -> Option<u64> {
        match self {
            Self::Observed { value } => Some(*value),
            Self::Unavailable { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AllocationEnforcement {
    ObservedOnly,
    CgroupV2 { receipt_sha256: Digest },
    WindowsJob { receipt_sha256: Digest },
    ExternalVm { receipt_sha256: Digest },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NativeAllocationKind {
    CgroupV2,
    WindowsJob,
}

/// Launcher receipt, retained with samples. Its digest binds declared native
/// controls; the installed driver separately checks live kernel membership.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeAllocationReceipt {
    pub format_version: u16,
    pub kind: NativeAllocationKind,
    pub nonce: Digest,
    pub engine_bytes: Limit,
    pub host_bytes: Limit,
    pub harness_sha256: Digest,
    pub vcpus: Limit,
    pub host_logical_cpus: Limit,
}
impl NativeAllocationReceipt {
    pub fn sha256(&self) -> Result<Digest> {
        digest("rrflow-native-allocation-v1", self)
    }
    pub fn validate(&self, campaign: &StorageCampaign) -> Result<()> {
        require(
            self.format_version == 1
                && self.engine_bytes == campaign.allocation.engine_bytes
                && self.host_bytes == campaign.allocation.host_bytes,
            "native allocation differs from frozen campaign",
        )?;
        require(
            self.vcpus == campaign.allocation.vcpus
                && self.vcpus.get() <= self.host_logical_cpus.get()
                && self.host_logical_cpus.get() <= 65536,
            "native CPU allocation differs from frozen campaign",
        )
    }
    pub fn enforcement(&self) -> Result<AllocationEnforcement> {
        let receipt_sha256 = self.sha256()?;
        Ok(match self.kind {
            NativeAllocationKind::CgroupV2 => AllocationEnforcement::CgroupV2 { receipt_sha256 },
            NativeAllocationKind::WindowsJob => {
                AllocationEnforcement::WindowsJob { receipt_sha256 }
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceSample {
    pub elapsed_ns: u64,
    pub pid: u32,
    pub resident_bytes: NativeMetric,
    /// Linux private resident pages; Windows private committed memory. Native
    /// platforms must be compared separately; neither is process RSS.
    pub private_bytes: NativeMetric,
    pub mapped_bytes: NativeMetric,
    pub host_resident_bytes: NativeMetric,
    pub read_bytes: NativeMetric,
    pub written_bytes: NativeMetric,
    pub cpu_time_ns: NativeMetric,
    /// Missing logical ownership metrics remain explicit, never zero-filled.
    pub retained_bytes: NativeMetric,
    pub queue_requests: NativeMetric,
    pub maintenance_debt_segments: NativeMetric,
    pub allocation_enforcement: AllocationEnforcement,
    pub allocation_receipt: Option<NativeAllocationReceipt>,
    pub storage: Option<rrd_contract::DiagnosticStorageSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LoadOutcome {
    Acknowledged,
    Rejected,
    Cancelled,
    Failed,
    Uncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LoadAttempt {
    pub sequence: u64,
    pub kind: StorageLoadKind,
    pub scheduled_ns: u64,
    pub started_ns: u64,
    pub finished_ns: u64,
    /// End of independent readback while the admitted client slot is still held.
    /// Absent in historical evidence; never infer zero verification cost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_finished_ns: Option<u64>,
    pub outcome: LoadOutcome,
    pub logical_id: Id,
    pub request_sha256: Digest,
    pub result_sha256: Digest,
    pub write: Option<WriteTransition>,
}

/// Bounded load-slot identity, separate from immutable seeded read coordinates.
/// Generation advances only after acknowledged independent readback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteTransition {
    pub slot: u64,
    pub generation: u64,
}

impl WriteTransition {
    /// Insert, update, retire, recreate. These are evidence counters only.
    pub fn counter_index(self) -> usize {
        if self.generation == 0 {
            0
        } else {
            match self.generation % 3 {
                1 => 1,
                2 => 2,
                _ => 3,
            }
        }
    }
    pub fn retired(self) -> bool {
        self.counter_index() == 2
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LaneEvidence {
    pub lane: CapacityLane,
    pub repetitions: Limit,
    pub receipt_sha256: Digest,
    pub acknowledged_readback_verified: bool,
}

/// Owned-file cache eviction verified before process restart. The measured
/// interval includes startup and first read; subsequent reads may be warm.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ColdStartEvidence {
    pub started_ns: u64,
    pub evicted_ns: u64,
    pub ready_ns: u64,
    pub finished_ns: u64,
    pub files: u64,
    pub file_bytes: u64,
    pub pages: u64,
    pub resident_pages: u64,
    pub page_bytes: u64,
    pub inventory_sha256: Digest,
    pub readback_sha256: Digest,
}

/// Streaming corpus receipts bind generated input, acknowledged writes and a
/// second complete readback pass without retaining the corpus in driver memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CorpusEvidence {
    /// Project documents; the separately counted graph anchor is not workload data.
    pub records: u64,
    pub anchor_records: u64,
    pub edges: u64,
    pub vectors: u64,
    pub vector_dimensions: u64,
    /// UTF-8 source payload only; vectors and protocol encoding are additional.
    pub logical_source_bytes: u64,
    pub encoded_mutation_bytes: u64,
    pub verified_records: u64,
    pub started_ns: u64,
    pub finished_ns: u64,
    pub input_sha256: Digest,
    pub acknowledgement_sha256: Digest,
    pub readback_sha256: Digest,
}

impl CorpusEvidence {
    pub(super) fn validate(&self, campaign: &StorageCampaign, before_ns: u64) -> Result<()> {
        let allocation = &campaign.allocation;
        require(
            self.records == allocation.records.get()
                && self.anchor_records == 1
                && self.edges >= allocation.edges.get()
                && self.vectors >= allocation.vectors.get()
                && self.vector_dimensions == allocation.vector_dimensions.get()
                && self.logical_source_bytes >= allocation.logical_source_bytes.get()
                && self.encoded_mutation_bytes >= self.logical_source_bytes
                && self.verified_records == self.records
                && self.started_ns <= self.finished_ns
                && self.finished_ns <= before_ns
                && self.finished_ns - self.started_ns
                    <= campaign.schedule.setup_ms.get() * NANOSECONDS_PER_MS
                && [
                    self.records,
                    self.edges,
                    self.vectors,
                    self.logical_source_bytes,
                    self.encoded_mutation_bytes,
                    self.finished_ns,
                ]
                .iter()
                .all(|v| *v <= MAX_EXACT_INTEGER),
            "corpus is incomplete, exceeds setup admission or differs from allocation",
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReopenObservation {
    pub cycle: u64,
    pub killed_pid: u32,
    pub reopened_pid: u32,
    pub killed_ns: u64,
    pub ready_ns: u64,
    pub finished_ns: u64,
    pub checked_operations: u64,
    pub expected_sha256: Digest,
    pub original_receipts_sha256: Digest,
    pub replay_receipts_sha256: Digest,
    pub readback_sha256: Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReopenEvidence {
    pub started_ns: u64,
    pub finished_ns: u64,
    pub corpus_records_verified: u64,
    pub corpus_readback_sha256: Digest,
    #[schemars(length(min = 1, max = 1000))]
    pub cycles: Vec<ReopenObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BackupEvidence {
    pub started_ns: u64,
    pub finished_ns: u64,
    pub verified_ns: u64,
    pub receipt: rrd_contract::CreateInstanceBackupResult,
    pub verification_catalogue_sha256: Digest,
}

impl ReopenEvidence {
    pub(super) fn validate(&self, campaign: &StorageCampaign, before_ns: u64) -> Result<()> {
        require(
            self.started_ns <= self.finished_ns
                && self.finished_ns <= before_ns
                && self.finished_ns - self.started_ns
                    <= campaign.schedule.lifecycle_ms.get() * NANOSECONDS_PER_MS
                && self.corpus_records_verified == campaign.allocation.records.get()
                && self.cycles.len() as u64 == campaign.schedule.forced_reopens.get(),
            "reopen corpus count or lifecycle deadline differs",
        )?;
        let mut prior = self.started_ns;
        for (index, cycle) in self.cycles.iter().enumerate() {
            require(
                cycle.cycle == index as u64
                    && cycle.checked_operations == index as u64 + 1
                    && cycle.killed_pid > 0
                    && cycle.reopened_pid > 0
                    && cycle.killed_ns >= prior
                    && cycle.killed_ns <= cycle.ready_ns
                    && cycle.ready_ns <= cycle.finished_ns
                    && cycle.finished_ns <= self.finished_ns
                    && cycle.original_receipts_sha256 == cycle.replay_receipts_sha256,
                "reopen omitted a prior ACK, changed receipts or reordered native events",
            )?;
            prior = cycle.finished_ns;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SustainedSegment {
    pub format_version: u16,
    pub campaign_sha256: Digest,
    pub index: u64,
    pub previous_sha256: Option<Digest>,
    pub point: Id,
    pub seed: u64,
    pub phase: LoadPhase,
    pub started_ns: u64,
    pub finished_ns: u64,
    pub corpus: Option<CorpusEvidence>,
    pub cold_start: Option<ColdStartEvidence>,
    pub reopens: Option<ReopenEvidence>,
    /// Launch time for the one bounded backup, possibly completed in a later segment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_started_ns: Option<u64>,
    pub backup: Option<BackupEvidence>,
    pub reader: Option<ReaderInterferenceEvidence>,
    /// Frozen before this segment's operations; full candidate/environment and
    /// installed configuration must equal the campaign's template identities.
    pub manifest: StorageManifest,
    pub result: StorageResult,
    #[schemars(length(max = 10000))]
    pub attempts: Vec<LoadAttempt>,
    #[schemars(length(max = 4096))]
    pub resources: Vec<ResourceSample>,
    #[schemars(length(max = 5))]
    pub lanes: Vec<LaneEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperatingPointResult {
    pub point: Id,
    pub seed: u64,
    pub phase: LoadPhase,
    pub elapsed_ns: u64,
    pub started_ns: u64,
    pub finished_ns: u64,
    pub attempts: u64,
    pub acknowledged_transitions: [u64; 4],
    pub first_sequence: u64,
    pub next_sequence: u64,
    pub schedule_origin_ns: Option<u64>,
    pub failures: u64,
    pub unresolved: u64,
    pub latency_over_budget: u64,
    pub maximum_scheduled_latency_ns: u64,
    pub maximum_resident_bytes: u64,
    pub missing_metrics: Vec<Id>,
    pub lanes: Vec<CapacityLane>,
    pub forced_reopens: u64,
    pub corpus: Option<CorpusEvidence>,
    pub ownership_baseline: Option<super::super::DrainSample>,
    #[schemars(length(max = 64))]
    pub drain_tail: Vec<super::super::DrainSample>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    Completed,
    Failed,
    Interrupted,
    NotRun,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CampaignResult {
    pub format_version: u16,
    pub campaign_sha256: Digest,
    pub status: CampaignStatus,
    pub segments: u64,
    pub last_segment_sha256: Option<Digest>,
    /// Campaign and segment bytes, excluding this self-describing result file.
    /// The total evidence quota still includes the result's encoded bytes.
    pub artifact_bytes: u64,
    pub progress: ProgressReceipt,
    pub failed_segment: Option<FailedSegment>,
    pub operating_points: Vec<OperatingPointResult>,
    pub limitations: Vec<Id>,
    pub qualification_passed: bool,
}

/// The first rejected operating point remains independently inspectable. This
/// reference is not an accepted segment and cannot contribute passing totals.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FailedSegment {
    pub sha256: Digest,
    pub reason: String,
}

/// Holds only one segment plus fixed-count point/seed/phase aggregates. Artifact
/// count/bytes are enforced before writes; create_new refuses replacement.
pub struct CampaignWriter {
    pub(super) directory: PathBuf,
    pub(super) campaign: StorageCampaign,
    pub(super) identity: Digest,
    pub(super) count: u64,
    pub(super) bytes: u64,
    pub(super) previous: Option<Digest>,
    pub(super) last_finished_ns: u64,
    pub(super) totals: BTreeMap<(Id, u64, LoadPhase), OperatingPointResult>,
    pub(super) failed: Option<FailedSegment>,
    pub(super) progress: Mutex<ProgressJournal>,
    pub(super) backups: BackupTracker,
}
