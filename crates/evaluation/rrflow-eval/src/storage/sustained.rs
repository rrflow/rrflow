//! Bounded sustained evidence built on the existing storage manifest/ledger.
//! These records describe qualification; they never configure the engine.
use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const NANOSECONDS_PER_MS: u64 = 1_000_000;
const MAX_SAMPLES: usize = 4096;

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
    #[schemars(length(min = 1, max = 32))]
    pub seeds: Vec<u64>,
    #[schemars(length(min = 1, max = 32))]
    pub points: Vec<OperatingPoint>,
    #[schemars(length(min = 1, max = 5))]
    pub required_lanes: Vec<CapacityLane>,
    pub evidence: EvidenceLimits,
}

impl StorageCampaign {
    pub fn validate(&self) -> Result<()> {
        self.storage.validate()?;
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
            self.schedule.operation_deadline_ms.get() <= self.storage.budgets.run_ms.get(),
            "operation deadline exceeds segment run bound",
        )?;
        require(
            self.schedule.sample_ms.get() <= MAX_EXACT_INTEGER / (2 * NANOSECONDS_PER_MS),
            "sampling interval exceeds exact nanosecond bound",
        )?;
        if self.purpose == CampaignPurpose::Qualification {
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AllocationEnforcement {
    ObservedOnly,
    CgroupV2 { receipt_sha256: Digest },
    WindowsJob { receipt_sha256: Digest },
    ExternalVm { receipt_sha256: Digest },
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
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LaneEvidence {
    pub lane: CapacityLane,
    pub repetitions: Limit,
    pub receipt_sha256: Digest,
    pub acknowledged_readback_verified: bool,
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
}

impl CampaignWriter {
    pub fn create(directory: &Path, campaign: StorageCampaign) -> Result<Self> {
        let identity = campaign.sha256()?;
        let bytes = encode_bounded(&campaign, MAX_DOCUMENT_BYTES)?;
        require(
            (bytes.len() as u64)
                .checked_add(MAX_DOCUMENT_BYTES)
                .is_some_and(|n| n <= campaign.evidence.total_bytes.get()),
            "campaign identity exceeds artifact budget",
        )?;
        std::fs::create_dir(directory)
            .map_err(|e| format!("campaign requires absent evidence directory: {e}"))?;
        write_new(&directory.join("campaign.json"), &bytes)?;
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
                .and_then(|n| n.checked_add(MAX_DOCUMENT_BYTES))
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
            next.checked_add(MAX_DOCUMENT_BYTES)
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
        merge_summary(&mut self.totals, summary)?;
        Ok(hash)
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
        let mut result = CampaignResult {
            format_version: 1,
            campaign_sha256: self.identity,
            status,
            segments: self.count,
            last_segment_sha256: self.previous,
            artifact_bytes: self.bytes,
            failed_segment: self.failed,
            operating_points: self.totals.into_values().collect(),
            limitations,
            qualification_passed: false,
        };
        result.qualification_passed = qualifies(&self.campaign, &result)?;
        let bytes = encode_bounded(&result, MAX_DOCUMENT_BYTES)?;
        require(
            self.bytes
                .checked_add(bytes.len() as u64)
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
    };
    let mut last = None;
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
    let sample_bound = campaign.schedule.sample_ms.get() * NANOSECONDS_PER_MS * 2;
    for sample in &segment.resources {
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
) -> Result<()> {
    let key = (value.point.clone(), value.seed, value.phase);
    if let Some(total) = totals.get_mut(&key) {
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
    merge_summary(&mut totals.clone(), summary.clone())?;
    Ok(summary)
}

fn qualifies(campaign: &StorageCampaign, result: &CampaignResult) -> Result<bool> {
    if campaign.purpose != CampaignPurpose::Qualification
        || result.status != CampaignStatus::Completed
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
            for phase in phases() {
                let Some(row) = result
                    .operating_points
                    .iter()
                    .find(|r| r.point == point.id && r.seed == *seed && r.phase == phase)
                else {
                    return Ok(false);
                };
                if row.elapsed_ns < campaign.schedule.duration_ms(phase) * NANOSECONDS_PER_MS
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
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn encode_bounded(value: &impl Serialize, limit: u64) -> Result<Vec<u8>> {
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
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
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
        .len();
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
        let summary = validate_next(
            &campaign,
            &segment,
            index,
            &previous,
            last_finished,
            &totals,
        )?;
        merge_summary(&mut totals, summary)?;
        previous = Some(digest("rrflow-storage-segment-v1", &segment)?);
        last_finished = segment.finished_ns;
    }
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
            .checked_add(result_bytes.len() as u64)
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
    require(
        result.qualification_passed == qualifies(&campaign, &result)?,
        "campaign qualification claim differs from evidence",
    )?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limit(value: u64) -> Limit {
        value.try_into().unwrap()
    }
    fn fixture() -> (StorageCampaign, SustainedSegment) {
        // Contract fixtures deliberately do not represent native observations.
        let mut manifest: StorageManifest =
            serde_json::from_str(include_str!("../../tests/fixtures/storage/manifest.json"))
                .unwrap();
        manifest.evidence_class = EvidenceClass::NativeCapacity;
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
            manifest,
            result,
            attempts: vec![LoadAttempt {
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
    fn missing_sequence_and_changed_offered_schedule_are_rejected() {
        let (campaign, segment) = fixture();
        let mut totals = BTreeMap::new();
        merge_summary(&mut totals, validate_segment(&campaign, &segment).unwrap()).unwrap();
        let mut repeated = validate_segment(&campaign, &segment).unwrap();
        assert!(merge_summary(&mut totals, repeated.clone()).is_err());
        repeated.first_sequence = 1;
        repeated.next_sequence = 2;
        repeated.schedule_origin_ns = Some(1);
        assert!(merge_summary(&mut totals, repeated).is_err());
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
        campaign.validate().unwrap();
        let mut result = CampaignResult {
            format_version: 1,
            campaign_sha256: campaign.sha256().unwrap(),
            status: CampaignStatus::Completed,
            segments: 15,
            last_segment_sha256: None,
            artifact_bytes: 0,
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
                row.attempts = if phase == LoadPhase::Drain {
                    0
                } else {
                    campaign.schedule.duration_ms(phase) / 1000
                        * if phase == LoadPhase::Burst { 2 } else { 1 }
                };
                result.operating_points.push(row);
            }
        }
        assert!(qualifies(&campaign, &result).unwrap());
        result.operating_points[0].attempts = 1;
        assert!(!qualifies(&campaign, &result).unwrap());
        result.operating_points[0].attempts = 300;
        result.operating_points[0].latency_over_budget = 4;
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
}
