//! Campaign purposes, load phases and kinds, capacity lanes, operating points, schedules and campaigns.

use super::*;

pub(super) const NANOSECONDS_PER_MS: u64 = 1_000_000;
pub(super) const MAX_SAMPLES: usize = 4096;
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

/// A reviewed selection of records actually created by the workload adapter.
/// Omission preserves the historical full-range reader and campaign identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReaderSelection {
    WorkloadRecords {},
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
    pub drain: super::super::DrainPolicy,
    #[schemars(length(min = 1, max = 32))]
    pub seeds: Vec<u64>,
    #[schemars(length(min = 1, max = 32))]
    pub points: Vec<OperatingPoint>,
    #[schemars(length(min = 1, max = 5))]
    pub required_lanes: Vec<CapacityLane>,
    pub evidence: EvidenceLimits,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reader_selection: Option<ReaderSelection>,
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
            self.reader_selection.is_none()
                || self
                    .required_lanes
                    .contains(&CapacityLane::ReaderPinInterference),
            "reader selection requires the reader interference lane",
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
            super::super::capacity::validate_capacity_profile(self)?;
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

pub(super) fn phases() -> [LoadPhase; 5] {
    [
        LoadPhase::Warmup,
        LoadPhase::Measured,
        LoadPhase::Burst,
        LoadPhase::Drain,
        LoadPhase::Soak,
    ]
}
pub(super) fn all_lanes() -> BTreeSet<CapacityLane> {
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
