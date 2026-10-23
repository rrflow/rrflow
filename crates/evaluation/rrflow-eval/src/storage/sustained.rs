//! Bounded sustained evidence built on the existing storage manifest/ledger.
//! These records describe qualification; they never configure the engine.
use super::progress::ProgressJournal;
use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const NANOSECONDS_PER_MS: u64 = 1_000_000;
const MAX_SAMPLES: usize = 4096;
/// Reserved outside workload artifacts for the outer process cleanup receipt.
pub const MAX_LAUNCH_RECEIPT_BYTES: u64 = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CampaignPurpose {
    Characterization,
    Qualification,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum LoadPhase {
    Warmup,
    Measured,
    Burst,
    Drain,
    Soak,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum StorageLoadKind {
    PointRead,
    RangeRead,
    MixedWrite,
    Maintenance,
}

/// Exactly the C-07 storage projection; no random sampling can silently change
/// the proportions. Key/operation seeds are independently bound by the manifest.
pub fn storage_load_kind(sequence: u64) -> StorageLoadKind {
    match sequence % 20 {
        0..=9 => StorageLoadKind::PointRead,
        10..=13 => StorageLoadKind::RangeRead,
        14..=18 => StorageLoadKind::MixedWrite,
        _ => StorageLoadKind::Maintenance,
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CapacityLane {
    Cold,
    Warm,
    BackupUnderWrite,
    ReaderPinInterference,
    ForcedReopens,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapacityAllocation {
    /// Exact digest of the owning J-04 profile projection, not a second table.
    pub owner_projection_sha256: Digest,
    pub vcpus: Limit,
    pub host_bytes: Limit,
    pub engine_bytes: Limit,
    pub estates: Limit,
    pub records: Limit,
    pub edges: Limit,
    pub vectors: Limit,
    pub vector_dimensions: Limit,
    pub logical_source_bytes: Limit,
}

/// Frozen read-key selection; mutation slots have their separate generation
/// contract. Hot-set traffic outside the selected fraction uses the cold set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReadKeyDistribution {
    Uniform {},
    HotKey {
        ordinal: u64,
    },
    HotSet {
        keys: Limit,
        traffic_basis_points: u16,
    },
}

impl ReadKeyDistribution {
    pub fn validate(&self, records: u64) -> Result<()> {
        require(
            (1..=MAX_EXACT_INTEGER).contains(&records),
            "invalid read-key corpus",
        )?;
        match self {
            Self::Uniform {} => Ok(()),
            Self::HotKey { ordinal } => require(*ordinal < records, "hot key outside corpus"),
            Self::HotSet {
                keys,
                traffic_basis_points,
            } => require(
                keys.get() <= records
                    && (1..=10_000).contains(traffic_basis_points)
                    && (keys.get() < records || *traffic_basis_points == 10_000),
                "hot-set size or traffic fraction outside corpus",
            ),
        }
    }

    pub fn ordinal(&self, seed: u64, sequence: u64, records: u64) -> Result<u64> {
        self.validate(records)?;
        // Stable SplitMix64 coordinates; this selects workload keys, never secrets.
        let mix = |mut value: u64| {
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
            value ^ (value >> 31)
        };
        let coordinate = seed.wrapping_add(sequence.wrapping_mul(0x9e3779b97f4a7c15));
        let key = mix(coordinate);
        Ok(match self {
            Self::Uniform {} => key % records,
            Self::HotKey { ordinal } => *ordinal,
            Self::HotSet {
                keys,
                traffic_basis_points,
            } => {
                if mix(coordinate ^ 0xa0761d6478bd642f) % 10_000 < u64::from(*traffic_basis_points)
                {
                    key % keys.get()
                } else {
                    keys.get() + key % (records - keys.get())
                }
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperatingPoint {
    pub id: Id,
    pub clients: Limit,
    pub requests_per_second: Limit,
    pub maximum_error_basis_points: u16,
    pub maximum_p99_ns: Limit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SustainedSchedule {
    /// Separate finite admission for corpus ingestion and full independent readback.
    pub setup_ms: Limit,
    /// Installed kill/reopen and interference preparation, outside offered load.
    pub lifecycle_ms: Limit,
    pub warmup_ms: Limit,
    pub measured_ms: Limit,
    pub burst_ms: Limit,
    pub drain_ms: Limit,
    pub soak_ms: Limit,
    pub sample_ms: Limit,
    pub operation_deadline_ms: Limit,
    pub forced_reopens: Limit,
}
impl SustainedSchedule {
    pub fn duration_ms(&self, phase: LoadPhase) -> u64 {
        match phase {
            LoadPhase::Warmup => self.warmup_ms,
            LoadPhase::Measured => self.measured_ms,
            LoadPhase::Burst => self.burst_ms,
            LoadPhase::Drain => self.drain_ms,
            LoadPhase::Soak => self.soak_ms,
        }
        .get()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceLimits {
    pub segment_attempts: Limit,
    pub segment_bytes: Limit,
    pub total_bytes: Limit,
    pub segments: Limit,
    pub samples_per_segment: Limit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorageCampaign {
    pub format_version: u16,
    pub purpose: CampaignPurpose,
    /// The existing C-07a contract is the per-segment validation template.
    pub storage: StorageManifest,
    pub allocation: CapacityAllocation,
    pub schedule: SustainedSchedule,
    pub read_distribution: ReadKeyDistribution,
    pub drain: super::DrainPolicy,
    #[schemars(length(min = 1, max = 32))]
    pub seeds: Vec<u64>,
    #[schemars(length(min = 1, max = 32))]
    pub points: Vec<OperatingPoint>,
    #[schemars(length(min = 1, max = 5))]
    pub required_lanes: Vec<CapacityLane>,
    pub evidence: EvidenceLimits,
}

impl StorageCampaign {
    /// Idle samples and the executed verification probe share one segment's
    /// existing deadline. Reserve probe time before choosing an idle span.
    pub fn idle_segment_ms(&self, requested_ms: u64) -> Result<u64> {
        let available = self
            .storage
            .budgets
            .run_ms
            .get()
            .checked_sub(self.schedule.operation_deadline_ms.get())
            .filter(|value| *value > 0)
            .ok_or("idle segment must reserve an operation deadline")?;
        let sampled = self
            .schedule
            .sample_ms
            .get()
            .checked_mul(self.evidence.samples_per_segment.get().saturating_sub(2))
            .filter(|value| *value > 0)
            .ok_or("idle segment needs bounded resource samples")?;
        let duration = requested_ms.min(available).min(sampled).min(120_000);
        require(duration > 0, "idle segment requires a positive duration")?;
        Ok(duration)
    }

    pub fn validate(&self) -> Result<()> {
        self.storage.validate()?;
        self.drain.validate(self)?;
        self.read_distribution
            .validate(self.allocation.records.get())?;
        require(self.format_version == 1, "unsupported campaign format")?;
        require(
            matches!(self.storage.evidence_class, EvidenceClass::NativeCapacity),
            "campaign requires native capacity evidence",
        )?;
        require(
            !self.seeds.is_empty()
                && self.seeds.len() <= 32
                && self.seeds.iter().all(|s| *s <= MAX_EXACT_INTEGER)
                && self.seeds.iter().collect::<BTreeSet<_>>().len() == self.seeds.len(),
            "invalid or repeated campaign seeds",
        )?;
        require(
            !self.points.is_empty()
                && self.points.len() <= 32
                && self
                    .points
                    .iter()
                    .map(|p| &p.id)
                    .collect::<BTreeSet<_>>()
                    .len()
                    == self.points.len(),
            "invalid operating points",
        )?;
        for point in &self.points {
            require(
                point.clients.get() <= self.storage.budgets.queue_requests.get()
                    && point.maximum_error_basis_points <= 10_000
                    && point.requests_per_second.get() <= 1_000_000,
                "operating point exceeds declared bounded driver admission",
            )?;
        }
        require(
            self.required_lanes
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                == self.required_lanes.len()
                && !self.required_lanes.is_empty(),
            "missing or repeated lane",
        )?;
        require(
            self.evidence.segment_attempts.get() <= MAX_ATTEMPTS as u64
                && self.evidence.segment_attempts.get() <= self.storage.budgets.attempts.get()
                && self.evidence.segment_bytes.get() <= MAX_DOCUMENT_BYTES
                && self.evidence.segment_bytes.get() <= self.evidence.total_bytes.get()
                && self.evidence.samples_per_segment.get() <= MAX_SAMPLES as u64
                && self.evidence.segments.get() <= 1_000_000,
            "campaign evidence must have finite admitted segment and total bounds",
        )?;
        require(
            self.allocation.engine_bytes.get() <= self.allocation.host_bytes.get()
                && self.allocation.estates.get() == self.storage.budgets.estates.get()
                && self.allocation.engine_bytes.get() == self.storage.budgets.process_bytes.get()
                && self
                    .storage
                    .budgets
                    .aggregate_process_bytes
                    .is_none_or(|pool| pool == self.allocation.engine_bytes)
                && (self.allocation.estates.get() == 1
                    || self.storage.budgets.aggregate_process_bytes.is_some())
                && self.allocation.host_bytes.get() == self.storage.budgets.host_bytes.get(),
            "allocation and storage budget identity differ",
        )?;
        for phase in phases() {
            let duration = self.schedule.duration_ms(phase);
            require(
                duration
                    .checked_mul(NANOSECONDS_PER_MS)
                    .is_some_and(|v| v <= MAX_EXACT_INTEGER)
                    && self.schedule.sample_ms.get() <= duration,
                "phase duration or sampling interval is invalid",
            )?;
        }
        require(
            self.schedule.setup_ms.get() <= 7 * 86_400_000,
            "corpus setup exceeds the seven-day admission ceiling",
        )?;
        require(
            self.schedule.lifecycle_ms.get() <= 1_800_000
                && self.schedule.forced_reopens.get() <= 1000,
            "installed lifecycle preparation exceeds bounded admission",
        )?;
        require(
            self.schedule.operation_deadline_ms.get() <= self.storage.budgets.run_ms.get(),
            "operation deadline exceeds segment run bound",
        )?;
        require(
            self.schedule.sample_ms.get() <= MAX_EXACT_INTEGER / (2 * NANOSECONDS_PER_MS),
            "sampling interval exceeds exact nanosecond bound",
        )?;
        if self.purpose == CampaignPurpose::Qualification {
            super::capacity::validate_capacity_profile(self)?;
            require(
                self.schedule.warmup_ms.get() >= 300_000
                    && self.schedule.measured_ms.get() >= 1_800_000
                    && self.schedule.soak_ms.get() >= 86_400_000
                    && self.seeds.len() >= 3
                    && self.schedule.forced_reopens.get() >= 100,
                "short campaign cannot qualify the J-04/C-07 envelope",
            )?;
            require(
                self.required_lanes.iter().copied().collect::<BTreeSet<_>>() == all_lanes(),
                "full qualification requires every lifecycle lane",
            )?;
        }
        Ok(())
    }
    pub fn sha256(&self) -> Result<Digest> {
        self.validate()?;
        digest("rrflow-storage-campaign-v1", self)
    }
}

fn phases() -> [LoadPhase; 5] {
    [
        LoadPhase::Warmup,
        LoadPhase::Measured,
        LoadPhase::Burst,
        LoadPhase::Drain,
        LoadPhase::Soak,
    ]
}
fn all_lanes() -> BTreeSet<CapacityLane> {
    [
        CapacityLane::Cold,
        CapacityLane::Warm,
        CapacityLane::BackupUnderWrite,
        CapacityLane::ReaderPinInterference,
        CapacityLane::ForcedReopens,
    ]
    .into_iter()
    .collect()
}

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
    fn validate(&self, campaign: &StorageCampaign, before_ns: u64) -> Result<()> {
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
    fn validate(&self, campaign: &StorageCampaign, before_ns: u64) -> Result<()> {
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
    pub ownership_baseline: Option<super::DrainSample>,
    #[schemars(length(max = 64))]
    pub drain_tail: Vec<super::DrainSample>,
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
    directory: PathBuf,
    campaign: StorageCampaign,
    identity: Digest,
    count: u64,
    bytes: u64,
    previous: Option<Digest>,
    last_finished_ns: u64,
    totals: BTreeMap<(Id, u64, LoadPhase), OperatingPointResult>,
    failed: Option<FailedSegment>,
    progress: Mutex<ProgressJournal>,
}

impl CampaignWriter {
    pub fn create(directory: &Path, campaign: StorageCampaign) -> Result<Self> {
        let identity = campaign.sha256()?;
        let bytes = encode_bounded(&campaign, MAX_DOCUMENT_BYTES)?;
        require(
            (bytes.len() as u64)
                .checked_add(MAX_DOCUMENT_BYTES + MAX_LAUNCH_RECEIPT_BYTES)
                .and_then(|n| n.checked_add(4096))
                .is_some_and(|n| n <= campaign.evidence.total_bytes.get()),
            "campaign identity exceeds artifact budget",
        )?;
        std::fs::create_dir(directory)
            .map_err(|e| format!("campaign requires absent evidence directory: {e}"))?;
        write_new(&directory.join("campaign.json"), &bytes)?;
        let progress = Mutex::new(ProgressJournal::create(directory, &identity)?);
        Ok(Self {
            directory: directory.to_owned(),
            campaign,
            identity,
            count: 0,
            bytes: bytes.len() as u64,
            previous: None,
            last_finished_ns: 0,
            totals: BTreeMap::new(),
            failed: None,
            progress,
        })
    }
    pub fn next_index(&self) -> u64 {
        self.count
    }
    pub fn previous_sha256(&self) -> Option<Digest> {
        self.previous.clone()
    }
    pub fn identity(&self) -> &Digest {
        &self.identity
    }
    pub fn record_event(&self, event: &ProgressEvent) -> Result<()> {
        let maximum = self
            .campaign
            .evidence
            .total_bytes
            .get()
            .checked_sub(self.bytes)
            .and_then(|n| n.checked_sub(MAX_DOCUMENT_BYTES + MAX_LAUNCH_RECEIPT_BYTES))
            .and_then(|n| n.checked_sub(self.campaign.evidence.segment_bytes.get()))
            .ok_or("progress has no reserved evidence capacity")?;
        self.progress
            .lock()
            .map_err(|_| "progress writer poisoned")?
            .append(event, maximum)
    }
    pub fn checkpoint_progress(&self) -> Result<()> {
        self.progress
            .lock()
            .map_err(|_| "progress writer poisoned")?
            .checkpoint()
    }
    fn progress_bytes(&self) -> Result<u64> {
        Ok(self
            .progress
            .lock()
            .map_err(|_| "progress writer poisoned")?
            .bytes())
    }
    /// Reserve enough evidence capacity before invoking the next operating
    /// point. Actual append still checks its encoded size and native writes.
    pub fn admit_next(&self) -> Result<()> {
        require(
            self.failed.is_none() && self.count < self.campaign.evidence.segments.get(),
            "campaign cannot admit another segment",
        )?;
        require(
            self.bytes
                .checked_add(self.campaign.evidence.segment_bytes.get())
                .and_then(|n| n.checked_add(self.progress_bytes().ok()?))
                .and_then(|n| n.checked_add(MAX_DOCUMENT_BYTES + MAX_LAUNCH_RECEIPT_BYTES))
                .is_some_and(|n| n <= self.campaign.evidence.total_bytes.get()),
            "next operating point has no reserved evidence capacity",
        )
    }
    pub fn append(&mut self, segment: &SustainedSegment) -> Result<Digest> {
        require(
            self.failed.is_none(),
            "campaign already retained a failed segment",
        )?;
        require(
            self.count < self.campaign.evidence.segments.get(),
            "campaign segment quota exhausted",
        )?;
        let bytes = encode_bounded(segment, self.campaign.evidence.segment_bytes.get())?;
        let next = self
            .bytes
            .checked_add(bytes.len() as u64)
            .ok_or("campaign artifact size overflow")?;
        // Reserve one bounded final result so a quota failure cannot consume its
        // reporting space. Failed OS writes remain failed, never counted complete.
        require(
            next.checked_add(self.progress_bytes()?)
                .and_then(|n| n.checked_add(MAX_DOCUMENT_BYTES + MAX_LAUNCH_RECEIPT_BYTES))
                .is_some_and(|n| n <= self.campaign.evidence.total_bytes.get()),
            "campaign artifact quota exhausted",
        )?;
        let hash = digest("rrflow-storage-segment-v1", segment)?;
        let validation = validate_next(
            &self.campaign,
            segment,
            self.count,
            &self.previous,
            self.last_finished_ns,
            &self.totals,
        );
        let summary = match validation {
            Ok(summary) => summary,
            Err(reason) => {
                write_new(&self.directory.join("failed-segment.json"), &bytes)?;
                self.bytes = next;
                self.failed = Some(FailedSegment {
                    sha256: hash,
                    reason: reason.clone(),
                });
                return Err(reason);
            }
        };
        write_new(
            &self
                .directory
                .join(format!("segment-{:020}.json", self.count)),
            &bytes,
        )?;
        self.bytes = next;
        self.count += 1;
        self.previous = Some(hash.clone());
        self.last_finished_ns = segment.finished_ns;
        merge_summary(
            &mut self.totals,
            summary,
            self.campaign.drain.stable_samples.get() as usize,
        )?;
        Ok(hash)
    }
    pub fn verify_drain(&self, point: &Id, seed: u64) -> Result<()> {
        super::drain::check(
            &self.campaign,
            &self.totals.values().collect::<Vec<_>>(),
            point,
            seed,
        )
    }
    pub fn finish(self, status: CampaignStatus, limitations: Vec<Id>) -> Result<CampaignResult> {
        require(
            limitations.len() <= MAX_CASES,
            "too many campaign limitations",
        )?;
        require(
            self.failed.is_none()
                || matches!(status, CampaignStatus::Failed | CampaignStatus::Interrupted),
            "failed operating point cannot be reported complete",
        )?;
        let progress = self
            .progress
            .into_inner()
            .map_err(|_| "progress writer poisoned")?
            .finish()?;
        let progress_audit =
            super::progress::verify_progress(&self.directory, &progress, &self.campaign, status)?;
        for row in self.totals.values() {
            progress_audit.check_readback_order(&row.point, row.seed, row.finished_ns)?;
        }
        let artifact_bytes = self
            .bytes
            .checked_add(progress.bytes)
            .ok_or("progress artifact overflow")?;
        let mut result = CampaignResult {
            format_version: 1,
            campaign_sha256: self.identity,
            status,
            segments: self.count,
            last_segment_sha256: self.previous,
            artifact_bytes,
            progress,
            failed_segment: self.failed,
            operating_points: self.totals.into_values().collect(),
            limitations,
            qualification_passed: false,
        };
        if status == CampaignStatus::Completed {
            super::drain::check_completed(&self.campaign, &result.operating_points)?;
        }
        result.qualification_passed = qualifies(&self.campaign, &result)?;
        let bytes = encode_bounded(&result, MAX_DOCUMENT_BYTES)?;
        require(
            artifact_bytes
                .checked_add(bytes.len() as u64 + MAX_LAUNCH_RECEIPT_BYTES)
                .is_some_and(|n| n <= self.campaign.evidence.total_bytes.get()),
            "final result exceeds artifact quota",
        )?;
        write_new(&self.directory.join("result.json"), &bytes)?;
        Ok(result)
    }
}

pub fn validate_segment(
    campaign: &StorageCampaign,
    segment: &SustainedSegment,
) -> Result<OperatingPointResult> {
    require(
        segment.format_version == 1 && segment.campaign_sha256 == campaign.sha256()?,
        "segment campaign identity differs",
    )?;
    let point = campaign
        .points
        .iter()
        .find(|p| p.id == segment.point)
        .ok_or("unknown operating point")?;
    require(
        campaign.seeds.contains(&segment.seed)
            && segment.started_ns < segment.finished_ns
            && segment.finished_ns <= MAX_EXACT_INTEGER,
        "invalid segment seed/time",
    )?;
    require(
        digest("candidate", &segment.manifest.candidate)?
            == digest("candidate", &campaign.storage.candidate)?
            && matches!(
                segment.manifest.evidence_class,
                EvidenceClass::NativeCapacity
            )
            && segment.manifest.durability == campaign.storage.durability
            && segment.manifest.corpus_sha256 == campaign.storage.corpus_sha256
            && digest("configuration", &segment.manifest.installed_configuration)?
                == digest("configuration", &campaign.storage.installed_configuration)?
            && digest("bindings", &segment.manifest.bindings)?
                == digest("bindings", &campaign.storage.bindings)?
            && digest("environment", &segment.manifest.environment)?
                == digest("environment", &campaign.storage.environment)?
            && digest("budgets", &segment.manifest.budgets)?
                == digest("budgets", &campaign.storage.budgets)?,
        "segment changed candidate, environment or ceilings",
    )?;
    let report = validate_result(&segment.manifest, &segment.result)?;
    require(
        segment
            .result
            .cases
            .iter()
            .all(|case| case.seed == segment.seed),
        "segment changed the frozen workload seed",
    )?;
    require(
        report.contract_passed,
        "segment failed independent storage ledger",
    )?;
    require(
        (segment.phase == LoadPhase::Drain || !segment.attempts.is_empty())
            && (segment.phase != LoadPhase::Drain || segment.attempts.is_empty())
            && segment.attempts.len() as u64 <= campaign.evidence.segment_attempts.get()
            && !segment.resources.is_empty()
            && segment.resources.len() as u64 <= campaign.evidence.samples_per_segment.get()
            && segment.lanes.len() <= 5,
        "segment evidence missing or exceeds bound",
    )?;
    let mut summary = OperatingPointResult {
        point: point.id.clone(),
        seed: segment.seed,
        phase: segment.phase,
        elapsed_ns: segment.finished_ns - segment.started_ns,
        started_ns: segment.started_ns,
        finished_ns: segment.finished_ns,
        attempts: segment.attempts.len() as u64,
        acknowledged_transitions: [0; 4],
        first_sequence: segment.attempts.first().map_or(0, |a| a.sequence),
        next_sequence: 0,
        schedule_origin_ns: None,
        failures: 0,
        unresolved: 0,
        latency_over_budget: 0,
        maximum_scheduled_latency_ns: 0,
        maximum_resident_bytes: 0,
        missing_metrics: Vec::new(),
        lanes: Vec::new(),
        forced_reopens: 0,
        corpus: segment.corpus.clone(),
        ownership_baseline: None,
        drain_tail: Vec::new(),
    };
    if segment.phase == LoadPhase::Warmup && summary.first_sequence == 0 {
        let first = &segment.resources[0];
        require(
            segment
                .attempts
                .iter()
                .all(|attempt| first.elapsed_ns <= attempt.started_ns),
            "ownership baseline follows a warmup effect",
        )?;
        summary.ownership_baseline = Some(first.into());
    }
    if segment.phase == LoadPhase::Drain {
        let keep = campaign.drain.stable_samples.get() as usize;
        summary.drain_tail = segment
            .resources
            .iter()
            .skip(segment.resources.len().saturating_sub(keep))
            .map(super::DrainSample::from)
            .collect();
    }
    if let Some(corpus) = &segment.corpus {
        require(
            segment.phase == LoadPhase::Warmup && summary.first_sequence == 0,
            "corpus receipt must precede the first warmup segment",
        )?;
        corpus.validate(campaign, segment.started_ns)?;
    }
    if let Some(reopens) = &segment.reopens {
        require(
            segment.phase == LoadPhase::Warmup && summary.first_sequence == 0,
            "reopen receipt must precede first warmup segment",
        )?;
        reopens.validate(campaign, segment.started_ns)?;
        require(
            segment.lanes.iter().any(|lane| {
                lane.lane == CapacityLane::ForcedReopens
                    && lane.repetitions.get() == reopens.cycles.len() as u64
                    && digest("c07g.reopen-evidence", reopens).as_ref() == Ok(&lane.receipt_sha256)
            }),
            "reopen lane does not bind its raw observations",
        )?;
    } else {
        require(
            !segment
                .lanes
                .iter()
                .any(|lane| lane.lane == CapacityLane::ForcedReopens),
            "reopen lane omitted its raw observations",
        )?;
    }
    if let Some(cold) = &segment.cold_start {
        require(
            segment.phase == LoadPhase::Warmup
                && summary.first_sequence == 0
                && cold.started_ns <= cold.evicted_ns
                && cold.evicted_ns <= cold.ready_ns
                && cold.ready_ns <= cold.finished_ns
                && cold.finished_ns <= segment.started_ns
                && cold.finished_ns - cold.started_ns
                    <= campaign.schedule.lifecycle_ms.get() * NANOSECONDS_PER_MS
                && cold.files > 0
                && cold.files
                    <= campaign
                        .storage
                        .installed_configuration
                        .storage
                        .recovery_files
                && cold.file_bytes > 0
                && cold.file_bytes
                    <= campaign
                        .storage
                        .installed_configuration
                        .storage
                        .recovery_read_bytes
                && cold.pages > 0
                && cold.resident_pages == 0
                && cold.page_bytes.is_power_of_two()
                && (4096..=65536).contains(&cold.page_bytes)
                && (cold.pages as u128) * (cold.page_bytes as u128) >= cold.file_bytes as u128
                && (cold.pages as u128) * (cold.page_bytes as u128)
                    <= cold.file_bytes as u128 + cold.files as u128 * (cold.page_bytes - 1) as u128
                && segment.lanes.iter().any(|lane| {
                    lane.lane == CapacityLane::Cold
                        && lane.repetitions.get() == 1
                        && digest("c07g.cold-start", cold).as_ref() == Ok(&lane.receipt_sha256)
                }),
            "cold lane lacks verified eviction, bounded startup or matching readback",
        )?;
    } else {
        require(
            !segment
                .lanes
                .iter()
                .any(|lane| lane.lane == CapacityLane::Cold),
            "cold lane omitted its native residency evidence",
        )?;
    }
    if let Some(reader) = &segment.reader {
        reader.validate()?;
        require(
            reader.opened_ns >= segment.started_ns && reader.released_ns <= segment.finished_ns,
            "reader interference lies outside its segment",
        )?;
    }
    for lane in segment
        .lanes
        .iter()
        .filter(|lane| lane.lane == CapacityLane::ReaderPinInterference)
    {
        let reader = segment
            .reader
            .as_ref()
            .ok_or("reader lane omitted raw ownership evidence")?;
        require(
            reader.qualifies(&segment.attempts)
                && lane.repetitions.get() == 1
                && digest("c07g.reader-interference", reader)? == lane.receipt_sha256,
            "reader lane lacks overlapping ACK, maintenance or matching receipt",
        )?;
    }
    if let Some(backup) = &segment.backup {
        require(
            backup.started_ns >= segment.started_ns
                && backup.started_ns <= backup.finished_ns
                && backup.finished_ns <= backup.verified_ns
                && backup.verified_ns <= segment.finished_ns
                && backup.receipt.backup.application_complete
                && segment.attempts.iter().any(|attempt| {
                    attempt.kind == StorageLoadKind::MixedWrite
                        && attempt.outcome == LoadOutcome::Acknowledged
                        && attempt.started_ns < backup.finished_ns
                        && attempt.finished_ns > backup.started_ns
                })
                && segment.lanes.iter().any(|lane| {
                    lane.lane == CapacityLane::BackupUnderWrite
                        && lane.repetitions.get() == 1
                        && digest("c07g.backup-evidence", backup).as_ref()
                            == Ok(&lane.receipt_sha256)
                }),
            "backup lane lacks a complete archive, overlapping ACK or matching raw receipt",
        )?;
    } else {
        require(
            !segment
                .lanes
                .iter()
                .any(|lane| lane.lane == CapacityLane::BackupUnderWrite),
            "backup lane omitted its raw receipt",
        )?;
    }
    for lane in &segment.lanes {
        if lane.lane == CapacityLane::Warm {
            require(
                segment.phase == LoadPhase::Measured
                    && summary.first_sequence == 0
                    && digest("c07g.warm-readback", &segment.result)? == lane.receipt_sha256,
                "warm lane lacks the first measured independent ledger",
            )?;
        }
    }
    let mut last = None;
    let mut written_slots = BTreeSet::new();
    let rate = point.requests_per_second.get()
        * if segment.phase == LoadPhase::Burst {
            2
        } else {
            1
        };
    for attempt in &segment.attempts {
        let offset = (attempt.sequence as u128 * 1_000_000_000 / rate as u128)
            .try_into()
            .map_err(|_| "offered schedule overflow")?;
        let origin = attempt
            .scheduled_ns
            .checked_sub(offset)
            .ok_or("offered schedule precedes time origin")?;
        require(
            summary
                .schedule_origin_ns
                .is_none_or(|value| value == origin),
            "offered schedule changed or coordinated omission",
        )?;
        summary.schedule_origin_ns = Some(origin);
        require(
            attempt.kind == storage_load_kind(attempt.sequence)
                && attempt.sequence < MAX_EXACT_INTEGER
                && last.is_none_or(|n| attempt.sequence == n + 1)
                && attempt.scheduled_ns >= segment.started_ns
                && attempt.scheduled_ns <= attempt.started_ns
                && attempt.started_ns <= attempt.finished_ns
                && attempt.finished_ns <= segment.finished_ns,
            "attempt mix, sequence or monotonic interval differs",
        )?;
        match attempt.write {
            Some(write) => {
                require(
                    attempt.kind == StorageLoadKind::MixedWrite
                        && write.slot < campaign.evidence.segment_attempts.get()
                        && written_slots.insert(write.slot)
                        && write.generation <= MAX_EXACT_INTEGER,
                    "write transition exceeds frozen slot admission or repeats within segment",
                )?;
                if attempt.outcome == LoadOutcome::Acknowledged {
                    summary.acknowledged_transitions[write.counter_index()] += 1;
                }
            }
            None => require(
                attempt.kind != StorageLoadKind::MixedWrite,
                "mixed write omitted its slot transition",
            )?,
        }
        let recorded = segment
            .result
            .cases
            .iter()
            .flat_map(|case| &case.ledger.attempts)
            .find(|record| {
                record.logical_id == attempt.logical_id
                    && record.request_sha256 == attempt.request_sha256
                    && record.started_ns == attempt.started_ns - segment.started_ns
                    && record.finished_ns == attempt.finished_ns - segment.started_ns
            })
            .ok_or("scheduled attempt missing from independent ledger")?;
        require(
            match (&attempt.outcome, &recorded.outcome) {
                (LoadOutcome::Acknowledged, AttemptOutcome::Acknowledged { receipt_sha256 }) => {
                    receipt_sha256 == &attempt.result_sha256
                }
                (
                    LoadOutcome::Rejected | LoadOutcome::Cancelled,
                    AttemptOutcome::Rejected { .. },
                ) => true,
                (
                    LoadOutcome::Uncertain | LoadOutcome::Failed,
                    AttemptOutcome::Uncertain { .. },
                ) => true,
                _ => false,
            },
            "scheduled attempt outcome differs from ledger",
        )?;
        last = Some(attempt.sequence);
        summary.next_sequence = attempt.sequence + 1;
        if attempt.finished_ns - attempt.scheduled_ns > point.maximum_p99_ns.get() {
            summary.latency_over_budget += 1;
        }
        summary.maximum_scheduled_latency_ns = summary
            .maximum_scheduled_latency_ns
            .max(attempt.finished_ns - attempt.scheduled_ns);
        if attempt.outcome != LoadOutcome::Acknowledged {
            summary.failures += 1;
        }
        if attempt.outcome == LoadOutcome::Uncertain {
            summary.unresolved += 1;
        }
        require(
            attempt.finished_ns - attempt.started_ns
                <= campaign.schedule.operation_deadline_ms.get() * NANOSECONDS_PER_MS,
            "operation exceeded frozen deadline",
        )?;
    }
    let mut missing = BTreeSet::new();
    let mut previous = segment.started_ns;
    let mut allocation_identity = None;
    let sample_bound = campaign.schedule.sample_ms.get() * NANOSECONDS_PER_MS * 2;
    for sample in &segment.resources {
        if let Some(receipt) = &sample.allocation_receipt {
            receipt.validate(campaign)?;
            require(
                digest("allocation-enforcement", &sample.allocation_enforcement)?
                    == digest("allocation-enforcement", &receipt.enforcement()?)?,
                "native allocation receipt digest differs",
            )?;
            let identity = receipt.sha256()?;
            require(
                allocation_identity
                    .as_ref()
                    .is_none_or(|prior| prior == &identity),
                "native allocation changed within measured segment",
            )?;
            allocation_identity = Some(identity);
        } else {
            require(
                matches!(
                    sample.allocation_enforcement,
                    AllocationEnforcement::ObservedOnly
                ),
                "native enforcement claim omitted its launcher receipt",
            )?;
        }
        require(
            sample.pid > 0
                && sample.elapsed_ns >= previous
                && sample.elapsed_ns <= segment.finished_ns
                && sample.elapsed_ns - previous <= sample_bound,
            "resource series has a gap or invalid process identity",
        )?;
        previous = sample.elapsed_ns;
        for (name, metric) in [
            ("resident", &sample.resident_bytes),
            ("private", &sample.private_bytes),
            ("mapped", &sample.mapped_bytes),
            ("host-resident", &sample.host_resident_bytes),
            ("read-io", &sample.read_bytes),
            ("write-io", &sample.written_bytes),
            ("cpu", &sample.cpu_time_ns),
            ("retained", &sample.retained_bytes),
            ("queue", &sample.queue_requests),
            ("maintenance-debt", &sample.maintenance_debt_segments),
        ] {
            match metric.observed() {
                Some(value) => {
                    require(value <= MAX_EXACT_INTEGER, "resource exceeds exact integer")?
                }
                None => {
                    missing.insert(name.to_owned().try_into()?);
                }
            }
        }
        if matches!(
            sample.allocation_enforcement,
            AllocationEnforcement::ObservedOnly
        ) {
            missing.insert("native-allocation-enforcement".to_owned().try_into()?);
        }
        if let Some(rss) = sample.resident_bytes.observed() {
            require(
                rss <= campaign.allocation.engine_bytes.get(),
                "engine RSS ceiling exceeded",
            )?;
            summary.maximum_resident_bytes = summary.maximum_resident_bytes.max(rss);
        }
        if let Some(host) = sample.host_resident_bytes.observed() {
            require(
                host <= campaign.allocation.host_bytes.get(),
                "host ceiling exceeded",
            )?;
        }
        if let Some(retained) = sample.retained_bytes.observed() {
            require(
                retained <= campaign.storage.budgets.retained_bytes.get(),
                "retained-owner ceiling exceeded",
            )?;
        }
        if let Some(queue) = sample.queue_requests.observed() {
            require(
                queue <= campaign.storage.budgets.queue_requests.get(),
                "queue ceiling exceeded",
            )?;
        }
        if let Some(storage) = &sample.storage {
            storage.validate().map_err(|e| e.to_string())?;
            if let Some(admission) = &storage.admission {
                let limits = &campaign.storage.installed_configuration.storage;
                require(
                    admission.native_transactions <= limits.native_transactions
                        && admission.native_transaction_bytes <= limits.native_transaction_bytes
                        && admission.active_read_views <= limits.active_read_views
                        && admission.pinned_read_bytes <= limits.pinned_read_bytes,
                    "physical read owners exceed installed retention admission",
                )?;
            } else {
                missing.insert("physical-read-ownership".to_owned().try_into()?);
            }
        } else {
            missing.insert("engine-storage-snapshot".to_owned().try_into()?);
        }
    }
    require(
        segment.finished_ns - previous <= sample_bound,
        "resource series ends before measured interval",
    )?;
    summary.missing_metrics = missing.into_iter().collect();
    let mut lanes = BTreeSet::new();
    for lane in &segment.lanes {
        require(
            lanes.insert(lane.lane) && lane.acknowledged_readback_verified,
            "lane is duplicated or lacks exact readback",
        )?;
        if lane.lane == CapacityLane::ForcedReopens {
            summary.forced_reopens = lane.repetitions.get();
        }
    }
    summary.lanes = lanes.into_iter().collect();
    Ok(summary)
}

fn merge_summary(
    totals: &mut BTreeMap<(Id, u64, LoadPhase), OperatingPointResult>,
    value: OperatingPointResult,
    drain_samples: usize,
) -> Result<()> {
    let key = (value.point.clone(), value.seed, value.phase);
    if let Some(total) = totals.get_mut(&key) {
        require(
            value.ownership_baseline.is_none(),
            "ownership baseline repeated",
        )?;
        // Keep only the frozen finite tail, including missing observations.
        // Every segment uses the same policy; short segments accumulate until
        // the maximum 64-sample window is available.
        total.drain_tail.extend(value.drain_tail.iter().cloned());
        if total.drain_tail.len() > drain_samples {
            total
                .drain_tail
                .drain(..total.drain_tail.len() - drain_samples);
        }
        require(
            value.corpus.is_none(),
            "corpus receipt repeated within operating point",
        )?;
        require(
            total.next_sequence == value.first_sequence
                && total.schedule_origin_ns == value.schedule_origin_ns,
            "operating point omitted/repeated attempts or changed offered schedule",
        )?;
        total.next_sequence = value.next_sequence;
        require(
            value.started_ns >= total.started_ns && value.finished_ns >= total.finished_ns,
            "operating point monotonic interval moved backwards",
        )?;
        total.finished_ns = value.finished_ns;
        total.elapsed_ns = total.finished_ns - total.started_ns;
        for (a, b) in [
            (&mut total.attempts, value.attempts),
            (&mut total.failures, value.failures),
            (&mut total.unresolved, value.unresolved),
            (&mut total.latency_over_budget, value.latency_over_budget),
            (&mut total.forced_reopens, value.forced_reopens),
        ] {
            *a = a
                .checked_add(b)
                .filter(|n| *n <= MAX_EXACT_INTEGER)
                .ok_or("campaign aggregate overflow")?;
        }
        for (total, count) in total
            .acknowledged_transitions
            .iter_mut()
            .zip(value.acknowledged_transitions)
        {
            *total = total
                .checked_add(count)
                .filter(|n| *n <= MAX_EXACT_INTEGER)
                .ok_or("transition count overflow")?;
        }
        total.maximum_scheduled_latency_ns = total
            .maximum_scheduled_latency_ns
            .max(value.maximum_scheduled_latency_ns);
        total.maximum_resident_bytes = total
            .maximum_resident_bytes
            .max(value.maximum_resident_bytes);
        total.missing_metrics.extend(value.missing_metrics);
        total.missing_metrics.sort();
        total.missing_metrics.dedup();
        total.lanes.extend(value.lanes);
        total.lanes.sort();
        total.lanes.dedup();
    } else {
        require(
            value.first_sequence == 0,
            "operating point does not start at sequence zero",
        )?;
        totals.insert(key, value);
    }
    Ok(())
}

fn validate_next(
    campaign: &StorageCampaign,
    segment: &SustainedSegment,
    index: u64,
    previous: &Option<Digest>,
    last_finished: u64,
    totals: &BTreeMap<(Id, u64, LoadPhase), OperatingPointResult>,
) -> Result<OperatingPointResult> {
    require(
        segment.index == index
            && &segment.previous_sha256 == previous
            && segment.finished_ns >= last_finished,
        "segment chain reordered or changed",
    )?;
    let summary = validate_segment(campaign, segment)?;
    // Fixed-count aggregates, bounded to 32 points * 32 seeds * five phases.
    // Validate on a copy before publishing evidence or mutating totals.
    merge_summary(
        &mut totals.clone(),
        summary.clone(),
        campaign.drain.stable_samples.get() as usize,
    )?;
    Ok(summary)
}

fn qualifies(campaign: &StorageCampaign, result: &CampaignResult) -> Result<bool> {
    if campaign.purpose != CampaignPurpose::Qualification || campaign.allocation.estates.get() != 1
    {
        return Ok(false);
    }
    satisfies_operating_requirements(campaign, result)
}

pub(super) fn satisfies_operating_requirements(
    campaign: &StorageCampaign,
    result: &CampaignResult,
) -> Result<bool> {
    if result.status != CampaignStatus::Completed
        || !result.limitations.is_empty()
        || result.segments == 0
        || result.failed_segment.is_some()
    {
        return Ok(false);
    }
    for point in &campaign.points {
        for seed in &campaign.seeds {
            let mut lanes = BTreeSet::new();
            let mut reopens = 0u64;
            let mut transitions = [false; 4];
            for phase in phases() {
                let Some(row) = result
                    .operating_points
                    .iter()
                    .find(|r| r.point == point.id && r.seed == *seed && r.phase == phase)
                else {
                    return Ok(false);
                };
                if row.elapsed_ns < campaign.schedule.duration_ms(phase) * NANOSECONDS_PER_MS
                    || (phase == LoadPhase::Warmup && row.corpus.is_none())
                    || row.unresolved != 0
                    || !row.missing_metrics.is_empty()
                    || (phase != LoadPhase::Drain && row.attempts == 0)
                    || row.latency_over_budget as u128 * 100 > row.attempts as u128
                    || (phase != LoadPhase::Drain
                        && (row.attempts as u128)
                            < (campaign.schedule.duration_ms(phase) as u128
                                * point.requests_per_second.get() as u128
                                * if phase == LoadPhase::Burst { 2 } else { 1 })
                            .div_ceil(1000))
                    || row.failures as u128 * 10_000
                        > row.attempts as u128 * point.maximum_error_basis_points as u128
                {
                    return Ok(false);
                }
                for (seen, count) in transitions.iter_mut().zip(row.acknowledged_transitions) {
                    *seen |= count > 0;
                }
                lanes.extend(row.lanes.iter().copied());
                reopens = reopens
                    .checked_add(row.forced_reopens)
                    .ok_or("reopen count overflow")?;
            }
            if !campaign
                .required_lanes
                .iter()
                .all(|lane| lanes.contains(lane))
                || reopens < campaign.schedule.forced_reopens.get()
                || transitions.iter().any(|seen| !seen)
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub(super) fn encode_bounded(value: &impl Serialize, limit: u64) -> Result<Vec<u8>> {
    struct Capped {
        bytes: Vec<u8>,
        limit: u64,
    }
    impl Write for Capped {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if self.bytes.len() as u64 + b.len() as u64 > self.limit {
                return Err(std::io::Error::other("evidence encoding exceeds bound"));
            }
            self.bytes.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Capped {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
    Ok(output.bytes)
}
pub(super) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}
pub(super) fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= limit,
        "evidence is not a bounded regular file",
    )?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    require(
        bytes.len() as u64 == metadata.len(),
        "evidence changed while reading",
    )?;
    Ok(bytes)
}

/// Stream the declared sequence from disk; never enumerate/retain a day of
/// documents. A missing/altered/extra indexed segment or false completion fails.
pub fn verify_campaign(directory: &Path) -> Result<CampaignResult> {
    let campaign: StorageCampaign = decode(&read_bounded(
        &directory.join("campaign.json"),
        MAX_DOCUMENT_BYTES,
    )?)?;
    let identity = campaign.sha256()?;
    let result_bytes = read_bounded(&directory.join("result.json"), MAX_DOCUMENT_BYTES)?;
    let result: CampaignResult = decode(&result_bytes)?;
    let mut progress_audit =
        super::progress::verify_progress(directory, &result.progress, &campaign, result.status)?;
    require(
        result.format_version == 1
            && result.campaign_sha256 == identity
            && result.segments <= campaign.evidence.segments.get()
            && result.limitations.len() <= MAX_CASES
            && result.operating_points.len() <= campaign.points.len() * campaign.seeds.len() * 5,
        "campaign result identity/count differs",
    )?;
    let mut previous = None;
    let mut last_finished = 0;
    let mut totals = BTreeMap::new();
    let mut bytes = std::fs::metadata(directory.join("campaign.json"))
        .map_err(|e| e.to_string())?
        .len()
        .checked_add(result.progress.bytes)
        .ok_or("progress artifact byte overflow")?;
    for index in 0..result.segments {
        let raw = read_bounded(
            &directory.join(format!("segment-{index:020}.json")),
            campaign.evidence.segment_bytes.get(),
        )?;
        bytes = bytes
            .checked_add(raw.len() as u64)
            .ok_or("artifact byte overflow")?;
        require(
            bytes <= campaign.evidence.total_bytes.get(),
            "campaign artifact quota exceeded",
        )?;
        let segment: SustainedSegment = decode(&raw)?;
        progress_audit.bind_segment(&segment)?;
        let summary = validate_next(
            &campaign,
            &segment,
            index,
            &previous,
            last_finished,
            &totals,
        )?;
        merge_summary(
            &mut totals,
            summary,
            campaign.drain.stable_samples.get() as usize,
        )?;
        previous = Some(digest("rrflow-storage-segment-v1", &segment)?);
        last_finished = segment.finished_ns;
    }
    progress_audit.require_bound(result.status)?;
    if let Some(failed) = &result.failed_segment {
        require(
            matches!(
                result.status,
                CampaignStatus::Failed | CampaignStatus::Interrupted
            ),
            "failed segment reported complete",
        )?;
        let raw = read_bounded(
            &directory.join("failed-segment.json"),
            campaign.evidence.segment_bytes.get(),
        )?;
        bytes = bytes
            .checked_add(raw.len() as u64)
            .ok_or("artifact byte overflow")?;
        let segment: SustainedSegment = decode(&raw)?;
        require(
            digest("rrflow-storage-segment-v1", &segment)? == failed.sha256,
            "failed operating point digest differs",
        )?;
        let reason = validate_next(
            &campaign,
            &segment,
            result.segments,
            &previous,
            last_finished,
            &totals,
        )
        .err()
        .ok_or("failed segment no longer fails validation")?;
        require(
            reason == failed.reason,
            "failed operating point reason differs",
        )?;
    } else {
        require(
            !directory.join("failed-segment.json").exists(),
            "result omitted failed operating point",
        )?;
    }
    require(
        bytes
            .checked_add(result_bytes.len() as u64 + MAX_LAUNCH_RECEIPT_BYTES)
            .is_some_and(|n| n <= campaign.evidence.total_bytes.get()),
        "campaign total evidence quota exceeded",
    )?;
    require(
        !directory
            .join(format!("segment-{:020}.json", result.segments))
            .exists(),
        "result omitted a written segment",
    )?;
    require(
        bytes == result.artifact_bytes
            && previous == result.last_segment_sha256
            && digest("point-results", &totals.into_values().collect::<Vec<_>>())?
                == digest("point-results", &result.operating_points)?,
        "result aggregates or segment digest differ",
    )?;
    if result.status == CampaignStatus::Completed {
        super::drain::check_completed(&campaign, &result.operating_points)?;
    }
    require(
        result.qualification_passed == qualifies(&campaign, &result)?,
        "campaign qualification claim differs from evidence",
    )?;
    Ok(result)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    fn limit(value: u64) -> Limit {
        value.try_into().unwrap()
    }
    #[test]
    fn idle_segments_reserve_probe_time_and_respect_sample_capacity() {
        let (mut campaign, _) = fixture();
        campaign.storage.budgets.run_ms = limit(120_000);
        campaign.schedule.operation_deadline_ms = limit(30_000);
        campaign.schedule.sample_ms = limit(1000);
        campaign.evidence.samples_per_segment = limit(256);
        assert_eq!(campaign.idle_segment_ms(120_000).unwrap(), 90_000);
        assert_eq!(campaign.idle_segment_ms(20_000).unwrap(), 20_000);
        campaign.evidence.samples_per_segment = limit(4);
        assert_eq!(campaign.idle_segment_ms(120_000).unwrap(), 2000);
        campaign.evidence.samples_per_segment = limit(2);
        assert!(campaign.idle_segment_ms(120_000).is_err());
        campaign.evidence.samples_per_segment = limit(256);
        campaign.schedule.operation_deadline_ms = limit(120_000);
        assert!(campaign.idle_segment_ms(120_000).is_err());
        campaign.schedule.operation_deadline_ms = limit(30_000);
        assert!(campaign.idle_segment_ms(0).is_err());
    }

    pub(crate) fn fixture() -> (StorageCampaign, SustainedSegment) {
        // Contract fixtures deliberately do not represent native observations.
        let mut manifest: StorageManifest =
            serde_json::from_str(include_str!("../../tests/fixtures/storage/manifest.json"))
                .unwrap();
        manifest.evidence_class = EvidenceClass::NativeCapacity;
        manifest.budgets.estates = limit(1);
        let mut result: StorageResult =
            serde_json::from_str(include_str!("../../tests/fixtures/storage/result.json")).unwrap();
        result.manifest_sha256 = manifest_digest(&manifest).unwrap();
        let campaign = StorageCampaign {
            format_version: 1,
            purpose: CampaignPurpose::Characterization,
            allocation: CapacityAllocation {
                owner_projection_sha256: digest("fixture", &1).unwrap(),
                vcpus: limit(1),
                host_bytes: manifest.budgets.host_bytes,
                engine_bytes: manifest.budgets.process_bytes,
                estates: manifest.budgets.estates,
                records: limit(1),
                edges: limit(1),
                vectors: limit(1),
                vector_dimensions: limit(1),
                logical_source_bytes: limit(1),
            },
            schedule: SustainedSchedule {
                setup_ms: limit(1000),
                lifecycle_ms: limit(1000),
                warmup_ms: limit(1000),
                measured_ms: limit(1000),
                burst_ms: limit(1000),
                drain_ms: limit(1000),
                soak_ms: limit(1000),
                sample_ms: limit(1000),
                operation_deadline_ms: limit(1000),
                forced_reopens: limit(1),
            },
            seeds: vec![7],
            read_distribution: ReadKeyDistribution::Uniform {},
            drain: super::super::DrainPolicy {
                maximum_mutable_growth_bytes: 0,
                maximum_projected_growth_bytes: 0,
                maximum_debt_segments: 0,
                stable_samples: limit(2),
            },
            points: vec![OperatingPoint {
                id: "one".to_owned().try_into().unwrap(),
                clients: limit(1),
                requests_per_second: limit(1),
                maximum_error_basis_points: 0,
                maximum_p99_ns: limit(1000),
            }],
            required_lanes: vec![CapacityLane::Warm],
            evidence: EvidenceLimits {
                segment_attempts: limit(100),
                segment_bytes: limit(MAX_DOCUMENT_BYTES),
                total_bytes: limit(32 * 1024 * 1024),
                segments: limit(100),
                samples_per_segment: limit(100),
            },
            storage: manifest.clone(),
        };
        let observed = NativeMetric::Observed { value: 0 };
        let sample = ResourceSample {
            elapsed_ns: 0,
            pid: 1,
            resident_bytes: observed.clone(),
            private_bytes: observed.clone(),
            mapped_bytes: observed.clone(),
            host_resident_bytes: observed.clone(),
            read_bytes: observed.clone(),
            written_bytes: observed.clone(),
            cpu_time_ns: observed.clone(),
            retained_bytes: observed.clone(),
            queue_requests: observed.clone(),
            maintenance_debt_segments: observed,
            allocation_enforcement: AllocationEnforcement::ObservedOnly,
            allocation_receipt: None,
            storage: Some(manifest.storage_observation.clone()),
        };
        let recorded = result.cases[0].ledger.attempts[0].clone();
        let AttemptOutcome::Acknowledged { receipt_sha256 } = recorded.outcome else {
            panic!("ACK fixture")
        };
        let segment = SustainedSegment {
            format_version: 1,
            campaign_sha256: campaign.sha256().unwrap(),
            index: 0,
            previous_sha256: None,
            point: campaign.points[0].id.clone(),
            seed: 7,
            phase: LoadPhase::Measured,
            started_ns: 0,
            finished_ns: 1_000_000_000,
            corpus: None,
            cold_start: None,
            reopens: None,
            backup: None,
            reader: None,
            manifest,
            result,
            attempts: vec![LoadAttempt {
                write: None,
                sequence: 0,
                kind: StorageLoadKind::PointRead,
                scheduled_ns: 0,
                started_ns: recorded.started_ns,
                finished_ns: recorded.finished_ns,
                outcome: LoadOutcome::Acknowledged,
                logical_id: recorded.logical_id,
                request_sha256: recorded.request_sha256,
                result_sha256: receipt_sha256,
            }],
            resources: vec![sample],
            lanes: vec![],
        };
        (campaign, segment)
    }
    #[test]
    fn short_run_is_characterization_and_failed_measurement_survives() {
        let (campaign, mut segment) = fixture();
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().join("evidence");
        let mut writer = CampaignWriter::create(&directory, campaign.clone()).unwrap();
        writer.append(&segment).unwrap();
        segment.index = writer.next_index();
        segment.previous_sha256 = writer.previous_sha256();
        segment.started_ns = 1_000_000_000;
        segment.finished_ns = 2_000_000_000;
        segment.attempts[0].sequence = 1;
        segment.attempts[0].scheduled_ns = 1_000_000_000;
        segment.attempts[0].started_ns += 1_000_000_000;
        segment.attempts[0].finished_ns += 1_000_000_000;
        segment.resources[0].elapsed_ns = 1_000_000_000;
        segment.resources[0].resident_bytes = NativeMetric::Observed {
            value: campaign.allocation.engine_bytes.get() + 1,
        };
        assert!(writer.append(&segment).unwrap_err().contains("RSS"));
        let result = writer.finish(CampaignStatus::Failed, vec![]).unwrap();
        assert!(!result.qualification_passed);
        assert!(result.failed_segment.is_some());
        assert_eq!(
            verify_campaign(&directory).unwrap().status,
            CampaignStatus::Failed
        );
        let mut changed: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("failed-segment.json")).unwrap())
                .unwrap();
        changed["resources"][0]["resident_bytes"]["value"] = 0.into();
        std::fs::write(
            directory.join("failed-segment.json"),
            serde_json::to_vec(&changed).unwrap(),
        )
        .unwrap();
        assert!(verify_campaign(&directory).is_err());
    }

    #[test]
    fn drain_requires_consecutive_ownership_recovery_and_exact_baseline() {
        use super::super::{DrainOwnership, DrainSample};
        let (campaign, segment) = fixture();
        let owner = DrainOwnership {
            native_transactions: 0,
            native_transaction_bytes: 0,
            active_read_views: 0,
            pinned_read_bytes: 0,
            mutable_bytes: 10,
            projected_bytes: 20,
            debt_segments: 0,
            outstanding_requests: 0,
            outstanding_bytes: 0,
            queued_requests: 0,
            active: false,
            fenced: false,
        };
        let mut warmup = validate_segment(&campaign, &segment).unwrap();
        warmup.phase = LoadPhase::Warmup;
        warmup.ownership_baseline = Some(DrainSample {
            elapsed_ns: 0,
            ownership: Some(owner.clone()),
        });
        let mut drain = warmup.clone();
        drain.phase = LoadPhase::Drain;
        drain.ownership_baseline = None;
        drain.started_ns = warmup.finished_ns;
        drain.finished_ns = drain.started_ns + 10;
        drain.first_sequence = 0;
        drain.next_sequence = 0;
        drain.schedule_origin_ns = None;
        drain.drain_tail = vec![
            DrainSample {
                elapsed_ns: 1,
                ownership: Some(owner.clone()),
            },
            DrainSample {
                elapsed_ns: 2,
                ownership: Some(owner),
            },
        ];
        let check = |row: &OperatingPointResult| {
            super::super::drain::check(&campaign, &[&warmup, row], &row.point, row.seed)
        };
        check(&drain).unwrap();
        for field in 0..12 {
            let mut bad = drain.clone();
            let value = bad.drain_tail[0].ownership.as_mut().unwrap();
            match field {
                0 => value.mutable_bytes += 1,
                1 => value.projected_bytes += 1,
                2 => value.debt_segments = 1,
                3 => value.outstanding_requests = 1,
                4 => value.outstanding_bytes = 1,
                5 => value.queued_requests = 1,
                6 => value.active = true,
                7 => value.fenced = true,
                8 => value.active_read_views = 1,
                9 => value.pinned_read_bytes = 1,
                10 => value.native_transactions = 1,
                _ => value.native_transaction_bytes = 1,
            }
            assert!(
                check(&bad).is_err(),
                "failed first sample cannot be hidden by settled last sample"
            );
        }
        let mut missing = drain.clone();
        missing.drain_tail[0].ownership = None;
        assert!(check(&missing).is_err());
        missing.drain_tail.remove(0);
        assert!(check(&missing).is_err());
        missing = drain.clone();
        missing.drain_tail[1].elapsed_ns = missing.drain_tail[0].elapsed_ns;
        assert!(check(&missing).is_err());
        assert!(
            super::super::drain::check(&campaign, &[&drain], &drain.point, drain.seed).is_err()
        );
        let mut totals = BTreeMap::new();
        merge_summary(&mut totals, drain.clone(), 2).unwrap();
        drain.started_ns = drain.finished_ns;
        drain.finished_ns += 1;
        drain.drain_tail[0].elapsed_ns = 3;
        drain.drain_tail[1].elapsed_ns = 4;
        merge_summary(&mut totals, drain, 2).unwrap();
        let tail = &totals.values().next().unwrap().drain_tail;
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0].elapsed_ns, 3);
    }

    #[test]
    fn drain_configuration_and_preload_observation_are_frozen() {
        let (mut campaign, mut segment) = fixture();
        let original = campaign.sha256().unwrap();
        campaign.drain.maximum_mutable_growth_bytes = 1;
        assert_ne!(original, campaign.sha256().unwrap());
        campaign.drain.stable_samples = limit(65);
        assert!(campaign.validate().is_err());
        campaign.drain.stable_samples = limit(1);
        assert!(campaign.validate().is_err());
        campaign.drain.stable_samples = limit(3);
        assert!(
            campaign.validate().is_err(),
            "window exceeds frozen drain time"
        );
        campaign.drain.stable_samples = limit(2);
        segment.campaign_sha256 = campaign.sha256().unwrap();
        segment.phase = LoadPhase::Warmup;
        segment.resources[0].elapsed_ns = segment.attempts[0].started_ns + 1;
        assert!(validate_segment(&campaign, &segment)
            .unwrap_err()
            .contains("baseline"));
    }

    #[test]
    fn read_distribution_is_frozen_bounded_and_selects_the_requested_region() {
        let uniform = ReadKeyDistribution::Uniform {};
        let hot = ReadKeyDistribution::HotKey { ordinal: 99 };
        let skew = ReadKeyDistribution::HotSet {
            keys: limit(4),
            traffic_basis_points: 2500,
        };
        let all_hot = ReadKeyDistribution::HotSet {
            keys: limit(4),
            traffic_basis_points: 10_000,
        };
        let mut uniform_keys = BTreeSet::new();
        let mut hot_reads = 0;
        for sequence in 0..1024 {
            let ordinal = uniform.ordinal(73, sequence, 100).unwrap();
            assert!(ordinal < 100);
            uniform_keys.insert(ordinal);
            assert_eq!(hot.ordinal(73, sequence, 100).unwrap(), 99);
            assert!(all_hot.ordinal(73, sequence, 100).unwrap() < 4);
            let ordinal = skew.ordinal(73, sequence, 100).unwrap();
            assert!(ordinal < 100);
            hot_reads += usize::from(ordinal < 4);
        }
        assert_eq!(uniform_keys.len(), 100);
        assert!((180..=330).contains(&hot_reads));
        for invalid in [
            ReadKeyDistribution::HotKey { ordinal: 100 },
            ReadKeyDistribution::HotSet {
                keys: limit(101),
                traffic_basis_points: 2500,
            },
            ReadKeyDistribution::HotSet {
                keys: limit(100),
                traffic_basis_points: 9999,
            },
            ReadKeyDistribution::HotSet {
                keys: limit(1),
                traffic_basis_points: 0,
            },
            ReadKeyDistribution::HotSet {
                keys: limit(1),
                traffic_basis_points: 10001,
            },
        ] {
            assert!(invalid.ordinal(0, 0, 100).is_err());
        }
        assert!(uniform.ordinal(0, 0, 0).is_err());
        let (mut campaign, _) = fixture();
        let original = campaign.sha256().unwrap();
        campaign.read_distribution = ReadKeyDistribution::HotKey { ordinal: 0 };
        assert_ne!(campaign.sha256().unwrap(), original);
        assert!(decode::<ReadKeyDistribution>(br#"{"kind":"uniform","keys":1}"#).is_err());
    }
    #[test]
    fn missing_sequence_and_changed_offered_schedule_are_rejected() {
        let (campaign, segment) = fixture();
        let mut totals = BTreeMap::new();
        merge_summary(
            &mut totals,
            validate_segment(&campaign, &segment).unwrap(),
            2,
        )
        .unwrap();
        let mut repeated = validate_segment(&campaign, &segment).unwrap();
        assert!(merge_summary(&mut totals, repeated.clone(), 2).is_err());
        repeated.first_sequence = 1;
        repeated.next_sequence = 2;
        repeated.schedule_origin_ns = Some(1);
        assert!(merge_summary(&mut totals, repeated, 2).is_err());
        let mut changed = segment.clone();
        changed.attempts[0].finished_ns = 1001;
        changed.result.cases[0].ledger.attempts[0].finished_ns = 1001;
        assert_eq!(
            validate_segment(&campaign, &changed)
                .unwrap()
                .latency_over_budget,
            1
        );
    }

    #[test]
    fn cold_lane_requires_bounded_native_residency_and_matching_raw_receipt() {
        let (campaign, mut segment) = fixture();
        segment.phase = LoadPhase::Warmup;
        let cold = ColdStartEvidence {
            started_ns: 0,
            evicted_ns: 0,
            ready_ns: 0,
            finished_ns: 0,
            files: 2,
            file_bytes: 4097,
            pages: 2,
            resident_pages: 0,
            page_bytes: 4096,
            inventory_sha256: digest("fixture-inventory", &1).unwrap(),
            readback_sha256: digest("fixture-readback", &1).unwrap(),
        };
        let bind = |segment: &mut SustainedSegment, cold: ColdStartEvidence| {
            segment.lanes = vec![LaneEvidence {
                lane: CapacityLane::Cold,
                repetitions: limit(1),
                receipt_sha256: digest("c07g.cold-start", &cold).unwrap(),
                acknowledged_readback_verified: true,
            }];
            segment.cold_start = Some(cold);
        };
        bind(&mut segment, cold.clone());
        validate_segment(&campaign, &segment).unwrap();
        segment.cold_start = None;
        assert!(validate_segment(&campaign, &segment)
            .unwrap_err()
            .contains("residency"));
        for invalid in 0..6 {
            let mut changed = cold.clone();
            match invalid {
                0 => changed.resident_pages = 1,
                1 => {
                    changed.files = campaign
                        .storage
                        .installed_configuration
                        .storage
                        .recovery_files
                        + 1
                }
                2 => {
                    changed.file_bytes = campaign
                        .storage
                        .installed_configuration
                        .storage
                        .recovery_read_bytes
                        + 1
                }
                3 => changed.evicted_ns = 1,
                4 => changed.pages = 10,
                _ => changed.page_bytes = 0,
            }
            bind(&mut segment, changed);
            assert!(validate_segment(&campaign, &segment)
                .unwrap_err()
                .contains("cold lane"));
        }
        bind(&mut segment, cold);
        segment.lanes[0].receipt_sha256 = digest("changed", &1).unwrap();
        assert!(validate_segment(&campaign, &segment)
            .unwrap_err()
            .contains("cold lane"));
    }

    #[test]
    fn campaign_completion_requires_final_readback_and_raw_cold_reconciliation() {
        use super::super::progress::{verify_progress, ProgressJournal};
        let (mut campaign, mut segment) = fixture();
        campaign.required_lanes.push(CapacityLane::Cold);
        let point = campaign.points[0].id.clone();
        let seed = campaign.seeds[0];
        let file_identity = digest("native-file-fixture", &1).unwrap();
        let cold = ColdStartEvidence {
            started_ns: 0,
            evicted_ns: 1,
            ready_ns: 2,
            finished_ns: 3,
            files: 1,
            file_bytes: 4097,
            pages: 2,
            resident_pages: 0,
            page_bytes: 4096,
            inventory_sha256: digest(
                "c07g.cold-inventory",
                &(
                    digest("c07g.cold-inventory", &seed).unwrap(),
                    &file_identity,
                ),
            )
            .unwrap(),
            readback_sha256: digest("readback", &1).unwrap(),
        };
        let file = ProgressEvent::ColdFile {
            point: point.clone(),
            seed,
            file_identity_sha256: file_identity,
            bytes: 4097,
            page_bytes: 4096,
            pages: 2,
            resident_pages: 0,
        };
        let observed = ProgressEvent::ColdStart {
            point: point.clone(),
            seed,
            observation: cold.clone(),
        };
        let final_readback = ProgressEvent::FinalReadback {
            point,
            seed,
            killed_pid: 1,
            reopened_pid: 2,
            seed_records: campaign.allocation.records.get(),
            working_slots: 0,
            states_sha256: digest("states", &1).unwrap(),
            started_ns: 10,
            finished_ns: 20,
        };
        let write = |events: &[ProgressEvent]| {
            let directory = tempfile::tempdir().unwrap();
            let mut journal =
                ProgressJournal::create(directory.path(), &campaign.sha256().unwrap()).unwrap();
            for event in events {
                journal.append(event, MAX_DOCUMENT_BYTES).unwrap();
            }
            (journal.finish().unwrap(), directory)
        };
        let events = vec![file.clone(), observed.clone(), final_readback.clone()];
        let (receipt, directory) = write(&events);
        let mut audit = verify_progress(
            directory.path(),
            &receipt,
            &campaign,
            CampaignStatus::Completed,
        )
        .unwrap();
        assert!(audit.require_bound(CampaignStatus::Completed).is_err());
        assert!(audit
            .check_readback_order(&segment.point, segment.seed, 21)
            .is_err());
        segment.finished_ns = 10;
        segment.cold_start = Some(cold.clone());
        audit.bind_segment(&segment).unwrap();
        audit.require_bound(CampaignStatus::Completed).unwrap();
        assert!(audit.bind_segment(&segment).is_err());

        for omission in 0..3 {
            let mut missing = events.clone();
            missing.remove(omission);
            let (receipt, directory) = write(&missing);
            assert!(verify_progress(
                directory.path(),
                &receipt,
                &campaign,
                CampaignStatus::Completed
            )
            .is_err());
        }
        let (receipt, directory) = write(std::slice::from_ref(&file));
        verify_progress(
            directory.path(),
            &receipt,
            &campaign,
            CampaignStatus::Interrupted,
        )
        .unwrap();
        for invalid in 0..6 {
            let mut changed = events.clone();
            match invalid {
                0 => {
                    let ProgressEvent::ColdFile { resident_pages, .. } = &mut changed[0] else {
                        unreachable!()
                    };
                    *resident_pages = 1;
                }
                1 => {
                    let ProgressEvent::ColdStart { observation, .. } = &mut changed[1] else {
                        unreachable!()
                    };
                    observation.file_bytes += 1;
                }
                2 => {
                    let ProgressEvent::ColdStart { observation, .. } = &mut changed[1] else {
                        unreachable!()
                    };
                    observation.inventory_sha256 = digest("wrong", &1).unwrap();
                }
                3 => changed.push(final_readback.clone()),
                4 => {
                    let ProgressEvent::FinalReadback { seed_records, .. } = &mut changed[2] else {
                        unreachable!()
                    };
                    *seed_records = 0;
                }
                _ => changed.push(ProgressEvent::Failure {
                    reason: "failed-after-readback".to_owned().try_into().unwrap(),
                }),
            }
            let (receipt, directory) = write(&changed);
            assert!(verify_progress(
                directory.path(),
                &receipt,
                &campaign,
                CampaignStatus::Completed
            )
            .is_err());
        }
    }
    fn bind_desktop(campaign: &mut StorageCampaign) {
        let projection = capacity_profiles().unwrap();
        let profile = &projection.profiles[0];
        assert_eq!(profile.id, "desktop");
        campaign.storage.profile = profile.id.clone().try_into().unwrap();
        campaign.allocation = CapacityAllocation {
            owner_projection_sha256: projection.owner_projection_sha256,
            vcpus: limit(profile.vcpus),
            host_bytes: limit(profile.host_bytes),
            engine_bytes: limit(profile.engine_bytes),
            estates: limit(profile.estates),
            records: limit(profile.records_min),
            edges: limit(profile.edges_min),
            vectors: limit(profile.vectors_min),
            vector_dimensions: limit(profile.vector_dimensions),
            logical_source_bytes: limit(profile.logical_source_bytes_min),
        };
        campaign.storage.budgets.host_bytes = campaign.allocation.host_bytes;
        campaign.storage.budgets.process_bytes = campaign.allocation.engine_bytes;
        campaign.storage.budgets.estates = campaign.allocation.estates;
        campaign.points[0].requests_per_second = limit(profile.rates[0]);
    }

    #[test]
    fn qualification_binds_owning_allocation_corpus_and_load_coordinates() {
        let (mut campaign, _) = fixture();
        bind_desktop(&mut campaign);
        super::super::capacity::validate_capacity_profile(&campaign).unwrap();
        for mutation in 0..12 {
            let mut invalid = campaign.clone();
            match mutation {
                0 => invalid.allocation.owner_projection_sha256 = digest("wrong", &0).unwrap(),
                1 => invalid.storage.profile = "unknown".to_owned().try_into().unwrap(),
                2 => invalid.allocation.vcpus = limit(1),
                3 => invalid.allocation.host_bytes = limit(1),
                4 => invalid.allocation.engine_bytes = limit(1),
                5 => invalid.allocation.estates = limit(2),
                6 => invalid.allocation.records = limit(1),
                7 => invalid.allocation.edges = limit(1),
                8 => invalid.allocation.vectors = limit(1),
                9 => invalid.allocation.vector_dimensions = limit(1),
                10 => invalid.allocation.logical_source_bytes = limit(1),
                _ => invalid.points[0].requests_per_second = limit(1),
            }
            assert!(
                super::super::capacity::validate_capacity_profile(&invalid).is_err(),
                "accepted changed owning coordinate {mutation}"
            );
        }
        campaign.points[0].clients = limit(2);
        assert!(super::super::capacity::validate_capacity_profile(&campaign).is_err());
        campaign.storage.profile = "fleet-isolation".to_owned().try_into().unwrap();
        assert!(super::super::capacity::validate_capacity_profile(&campaign)
            .unwrap_err()
            .contains("allocation differs"));
        // Smaller arbitrary characterization remains representable, without qualification.
        fixture().0.validate().unwrap();
    }

    #[test]
    fn observed_only_shortened_or_omitted_work_cannot_qualify() {
        let (mut campaign, segment) = fixture();
        campaign.purpose = CampaignPurpose::Qualification;
        assert!(campaign.validate().is_err());
        campaign.schedule.warmup_ms = limit(300_000);
        campaign.schedule.measured_ms = limit(1_800_000);
        campaign.schedule.soak_ms = limit(86_400_000);
        campaign.schedule.forced_reopens = limit(100);
        campaign.seeds = vec![1, 2, 3];
        campaign.required_lanes = all_lanes().into_iter().collect();
        bind_desktop(&mut campaign);
        campaign.validate().unwrap();
        let mut result = CampaignResult {
            format_version: 1,
            campaign_sha256: campaign.sha256().unwrap(),
            status: CampaignStatus::Completed,
            segments: 15,
            last_segment_sha256: None,
            artifact_bytes: 0,
            progress: ProgressReceipt {
                bytes: 0,
                records: 1,
                sha256: digest("fixture", &1).unwrap(),
            },
            failed_segment: None,
            operating_points: vec![],
            limitations: vec![],
            qualification_passed: false,
        };
        for seed in &campaign.seeds {
            for phase in phases() {
                let mut row = validate_segment(&fixture().0, &segment).unwrap();
                row.seed = *seed;
                row.phase = phase;
                row.elapsed_ns = campaign.schedule.duration_ms(phase) * NANOSECONDS_PER_MS;
                row.missing_metrics.clear();
                row.lanes = all_lanes().into_iter().collect();
                row.forced_reopens = 100;
                if phase == LoadPhase::Warmup {
                    row.corpus = Some(CorpusEvidence {
                        records: campaign.allocation.records.get(),
                        anchor_records: 1,
                        edges: campaign.allocation.edges.get(),
                        vectors: campaign.allocation.vectors.get(),
                        vector_dimensions: campaign.allocation.vector_dimensions.get(),
                        logical_source_bytes: campaign.allocation.logical_source_bytes.get(),
                        encoded_mutation_bytes: 2048,
                        verified_records: campaign.allocation.records.get(),
                        started_ns: 0,
                        finished_ns: 0,
                        input_sha256: digest("fixture", &1).unwrap(),
                        acknowledgement_sha256: digest("fixture", &2).unwrap(),
                        readback_sha256: digest("fixture", &3).unwrap(),
                    });
                }
                row.acknowledged_transitions = [1; 4];
                row.attempts = if phase == LoadPhase::Drain {
                    0
                } else {
                    campaign.schedule.duration_ms(phase) / 1000
                        * campaign.points[0].requests_per_second.get()
                        * if phase == LoadPhase::Burst { 2 } else { 1 }
                };
                result.operating_points.push(row);
            }
        }
        assert!(qualifies(&campaign, &result).unwrap());
        result.operating_points[0].attempts = 1;
        assert!(!qualifies(&campaign, &result).unwrap());
        result.operating_points[0].attempts = 1500;
        result.operating_points[0].latency_over_budget = 16;
        assert!(!qualifies(&campaign, &result).unwrap());
        result.operating_points[0].latency_over_budget = 0;
        result.operating_points[0].missing_metrics.push(
            "native-allocation-enforcement"
                .to_owned()
                .try_into()
                .unwrap(),
        );
        assert!(!qualifies(&campaign, &result).unwrap());
    }
    #[test]
    fn evidence_admission_precedes_filesystem_effect() {
        let (mut campaign, _) = fixture();
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().join("no-effect");
        campaign.evidence.total_bytes = limit(MAX_DOCUMENT_BYTES);
        assert!(CampaignWriter::create(&directory, campaign).is_err());
        assert!(!directory.exists());
    }

    #[test]
    fn working_set_transitions_preserve_identity_and_repeat_retirement_cycles() {
        assert_eq!(
            (0..8)
                .map(|generation| WriteTransition {
                    slot: 7,
                    generation
                }
                .counter_index())
                .collect::<Vec<_>>(),
            [0, 1, 2, 3, 1, 2, 3, 1]
        );
        assert!(WriteTransition {
            slot: 7,
            generation: 5
        }
        .retired());
        assert!(!WriteTransition {
            slot: 7,
            generation: 6
        }
        .retired());
        let (campaign, mut segment) = fixture();
        segment.attempts[0].write = Some(WriteTransition {
            slot: 0,
            generation: 0,
        });
        assert!(validate_segment(&campaign, &segment)
            .unwrap_err()
            .contains("write transition"));
    }

    #[test]
    fn corpus_receipt_requires_full_readback_and_frozen_setup_budget() {
        let (campaign, _) = fixture();
        let mut corpus = CorpusEvidence {
            records: 1,
            anchor_records: 1,
            edges: 1,
            vectors: 1,
            vector_dimensions: 1,
            logical_source_bytes: 1024,
            encoded_mutation_bytes: 2048,
            verified_records: 1,
            started_ns: 0,
            finished_ns: 1_000_000_000,
            input_sha256: digest("fixture", &1).unwrap(),
            acknowledgement_sha256: digest("fixture", &2).unwrap(),
            readback_sha256: digest("fixture", &3).unwrap(),
        };
        assert!(corpus.validate(&campaign, 1_000_000_000).is_ok());
        corpus.verified_records = 0;
        assert!(corpus.validate(&campaign, 1_000_000_000).is_err());
        corpus.verified_records = 1;
        corpus.finished_ns += 1;
        assert!(corpus.validate(&campaign, 2_000_000_000).is_err());
        corpus.finished_ns = 0;
        corpus.edges = 0;
        assert!(corpus.validate(&campaign, 1_000_000_000).is_err());
    }

    #[test]
    fn reopen_receipt_denies_missing_ack_and_changed_replay() {
        let (campaign, _) = fixture();
        let hash = digest("fixture", &1).unwrap();
        let mut proof = ReopenEvidence {
            started_ns: 0,
            finished_ns: 1000,
            corpus_records_verified: 1,
            corpus_readback_sha256: hash.clone(),
            cycles: vec![ReopenObservation {
                cycle: 0,
                killed_pid: 1,
                reopened_pid: 2,
                killed_ns: 100,
                ready_ns: 200,
                finished_ns: 300,
                checked_operations: 1,
                expected_sha256: hash.clone(),
                original_receipts_sha256: hash.clone(),
                replay_receipts_sha256: hash.clone(),
                readback_sha256: hash,
            }],
        };
        assert!(proof.validate(&campaign, 1000).is_ok());
        proof.cycles[0].checked_operations = 0;
        assert!(proof.validate(&campaign, 1000).is_err());
        proof.cycles[0].checked_operations = 1;
        proof.cycles[0].replay_receipts_sha256 = digest("different", &1).unwrap();
        assert!(proof.validate(&campaign, 1000).is_err());
    }

    #[test]
    fn native_allocation_claim_requires_matching_retained_receipt() {
        let (campaign, mut segment) = fixture();
        let receipt = NativeAllocationReceipt {
            format_version: 1,
            kind: NativeAllocationKind::WindowsJob,
            nonce: digest("nonce", &1).unwrap(),
            engine_bytes: campaign.allocation.engine_bytes,
            host_bytes: campaign.allocation.host_bytes,
            harness_sha256: digest("harness", &1).unwrap(),
            vcpus: campaign.allocation.vcpus,
            host_logical_cpus: limit(8),
        };
        segment.resources[0].allocation_enforcement = receipt.enforcement().unwrap();
        assert!(validate_segment(&campaign, &segment).is_err());
        segment.resources[0].allocation_receipt = Some(receipt.clone());
        assert!(validate_segment(&campaign, &segment).is_ok());
        segment.resources[0]
            .allocation_receipt
            .as_mut()
            .unwrap()
            .engine_bytes = limit(1);
        assert!(validate_segment(&campaign, &segment).is_err());
        segment.resources[0].allocation_receipt = Some(receipt);
        segment.resources[0].allocation_enforcement = AllocationEnforcement::CgroupV2 {
            receipt_sha256: digest("different", &1).unwrap(),
        };
        assert!(validate_segment(&campaign, &segment).is_err());
    }
}
