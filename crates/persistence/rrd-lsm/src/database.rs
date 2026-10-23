use crate::filesystem::{AnchoredPath, OpenOptions};
use crate::io::{IoContext, SharedIoContext};
use crate::mutable::MutableCharge;
use crate::segment::{
    new_page_cache_with_policy, next_projected_cache_access, page_cache_stats, ActiveReadViews,
    ReadView, SharedPageCache,
};
use crate::write_diagnostics::{PendingWriteDiagnostics, PhaseClock, WritePathCollector};
use crate::{
    AppendReceipt, Checkpoint, Durability, Error, Manifest, ManifestStore, Memtable,
    PageCachePolicy, ProjectedReadRequest, ProjectedReadResource, ProjectedReadStream, Result,
    Segment, SegmentCompressionPolicy, SegmentIoPolicy, SegmentIoStats, SegmentOpenEvidence,
    SegmentRowGroupBudget, SnapshotBundle, SnapshotBundleFile, SnapshotExportBoundary,
    SnapshotSegment, VersionedValue, WalWriter, WriteBatch, WriteLockWait, WritePathDiagnostics,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

pub(crate) const WAL_DIRECTORY: &str = "wal";
pub(crate) const SEGMENT_DIRECTORY: &str = "segments";
pub const DEFAULT_WAL_PAYLOAD_MAX_BYTES: usize = 64 * 1024 * 1024;
pub const DEFAULT_MEMTABLE_MAX_VERSIONS: usize = 524_288;
pub const DEFAULT_L0_COMPACTION_TRIGGER: usize = 8;
pub const DEFAULT_COMPACTION_MAX_INPUT_SEGMENTS: usize = 16;
pub const DEFAULT_COMPACTION_TARGET_SEGMENT_BYTES: usize = 8 * 1024 * 1024;
pub const DEFAULT_MAX_COMPACTION_LEVEL: u8 = 6;

/// Synchronous soft write-path maintenance targets. Crossing either target
/// stalls the next writer while the existing WAL-backed memtable is published.
/// One batch may exceed a soft target, but must fit the separate hard mutable
/// limits. Neither policy bounds retained generations or whole-process memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenancePolicy {
    pub wal_payload_max_bytes: usize,
    pub memtable_max_versions: usize,
}

impl Default for MaintenancePolicy {
    fn default() -> Self {
        Self {
            wal_payload_max_bytes: DEFAULT_WAL_PAYLOAD_MAX_BYTES,
            memtable_max_versions: DEFAULT_MEMTABLE_MAX_VERSIONS,
        }
    }
}

impl MaintenancePolicy {
    fn validate(self) -> Result<Self> {
        if self.wal_payload_max_bytes == 0 || self.memtable_max_versions == 0 {
            return Err(Error::InvalidConfiguration(
                "WAL payload and memtable version limits must be greater than zero".into(),
            ));
        }
        Ok(self)
    }
}

/// Deterministic bounds for one immutable maintenance step. The input bound
/// prevents a growing database from turning one compaction into an unbounded
/// all-segment rewrite; the output target is enforced at key boundaries so all
/// MVCC versions for one key remain together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionPolicy {
    pub l0_compaction_trigger: usize,
    pub max_input_segments: usize,
    pub target_segment_bytes: usize,
    pub max_level: u8,
}

impl Default for CompactionPolicy {
    fn default() -> Self {
        Self {
            l0_compaction_trigger: DEFAULT_L0_COMPACTION_TRIGGER,
            max_input_segments: DEFAULT_COMPACTION_MAX_INPUT_SEGMENTS,
            target_segment_bytes: DEFAULT_COMPACTION_TARGET_SEGMENT_BYTES,
            max_level: DEFAULT_MAX_COMPACTION_LEVEL,
        }
    }
}

impl CompactionPolicy {
    fn validate(self) -> Result<Self> {
        if self.l0_compaction_trigger < 2
            || self.max_input_segments < 2
            || self.target_segment_bytes == 0
            || self.max_level == 0
        {
            return Err(Error::InvalidConfiguration(
                "compaction requires an L0 trigger and input bound of at least two, a non-zero output target, and at least one level".into(),
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseOptions {
    pub transaction_limits: crate::TransactionLimits,
    /// Per-open aggregate ownership ceilings, independent of caller budgets.
    pub active_read_views: usize,
    pub pinned_read_bytes: u64,
    /// Available native bytes retained beyond each proposed WAL/immutable
    /// allocation. This is admission, not reserved disk space or a total quota.
    pub recovery_reserve_bytes: u64,
    /// Output allocation cap shared by projected streams of this open instance.
    pub projected_output_bytes: usize,
    pub page_cache_bytes: usize,
    pub page_cache_policy: PageCachePolicy,
    pub segment_io: SegmentIoPolicy,
    pub segment_row_group_budget: SegmentRowGroupBudget,
    pub segment_compression: SegmentCompressionPolicy,
    pub maintenance: MaintenancePolicy,
    pub mutable_limits: crate::MutableLimits,
    pub mutable_retention: crate::MutableRetentionLimits,
    pub compaction: CompactionPolicy,
    pub compaction_limits: crate::CompactionLimits,
    pub reclamation: crate::ReclamationLimits,
    pub recovery: crate::RecoveryLimits,
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            transaction_limits: crate::TransactionLimits::default(),
            active_read_views: crate::DEFAULT_MAX_ACTIVE_READ_VIEWS,
            pinned_read_bytes: crate::DEFAULT_MAX_PINNED_READ_BYTES,
            projected_output_bytes: crate::DEFAULT_PROJECTED_OUTPUT_BYTES,
            recovery_reserve_bytes: 16 * 1024 * 1024,
            page_cache_bytes: crate::DEFAULT_PAGE_CACHE_BYTES,
            page_cache_policy: PageCachePolicy::default(),
            segment_io: SegmentIoPolicy::default(),
            segment_row_group_budget: SegmentRowGroupBudget::default(),
            segment_compression: SegmentCompressionPolicy::default(),
            maintenance: MaintenancePolicy::default(),
            mutable_limits: crate::MutableLimits::default(),
            mutable_retention: crate::MutableRetentionLimits::default(),
            compaction: CompactionPolicy::default(),
            compaction_limits: crate::CompactionLimits::default(),
            reclamation: crate::ReclamationLimits::default(),
            recovery: crate::RecoveryLimits::default(),
        }
    }
}

impl DatabaseOptions {
    pub(crate) fn validate(self) -> Result<Self> {
        if self.projected_output_bytes == 0
            || self.recovery_reserve_bytes == 0
            || self.active_read_views == 0
            || self.pinned_read_bytes == 0
        {
            return Err(Error::InvalidConfiguration(
                "projected output, read retention and recovery reserve must be positive".into(),
            ));
        }
        self.page_cache_policy
            .protected_capacity_bytes(self.page_cache_bytes)?;
        self.segment_io.validate()?;
        self.segment_row_group_budget.validate()?;
        self.segment_compression.validate()?;
        self.transaction_limits.validate()?;
        self.maintenance.validate()?;
        self.mutable_limits.validate()?;
        if self.mutable_retention.accounted_bytes < self.mutable_limits.accounted_bytes
            || self.mutable_retention.versions < self.mutable_limits.versions
            || self.mutable_retention.generations == 0
        {
            return Err(Error::InvalidConfiguration(
                "mutable retention must cover active limits and at least one generation".into(),
            ));
        }
        self.compaction.validate()?;
        self.compaction_limits.validate()?;
        self.reclamation.validate()?;
        self.recovery.validate()?;
        if self.compaction_limits.l0_segments < self.compaction.l0_compaction_trigger as u64 {
            return Err(Error::InvalidConfiguration(
                "L0 hard capacity is below its compaction trigger".into(),
            ));
        }
        Ok(self)
    }
}

/// Process-local maintenance counters. Canonical state lives in the WAL,
/// segments, and manifests; these counters intentionally reset on reopen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceStats {
    pub automatic_flushes: u64,
    pub write_stalls: u64,
    pub failed_flushes: u64,
    pub oversized_batches: u64,
    pub peak_wal_payload_bytes: usize,
    pub peak_memtable_versions: usize,
    pub peak_accounted_memtable_bytes: usize,
    pub automatic_compactions: u64,
    pub failed_compactions: u64,
    pub compaction_input_bytes: u64,
    pub compaction_output_bytes: u64,
    pub peak_compaction_buffer_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionOutcome {
    pub previous_manifest: String,
    pub manifest: String,
    pub input_segments: usize,
    pub output_segments: usize,
    pub input_versions: u64,
    pub output_versions: u64,
    pub protected_sequences: Vec<u64>,
    pub source_level: u8,
    pub target_level: u8,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub peak_buffer_bytes: usize,
    pub history_pruned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompactionCandidate {
    indices: Vec<usize>,
    source_level: u8,
    target_level: u8,
}

struct CompactionMerge {
    segments: Vec<Segment>,
    output_segments: usize,
    input_versions: u64,
    output_versions: u64,
    output_bytes: u64,
    peak_buffer_bytes: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GarbageCollectionReport {
    pub retained_manifests: Vec<String>,
    pub retained_segments: Vec<String>,
    pub retained_wals: Vec<String>,
    pub removed_manifests: Vec<String>,
    pub removed_segments: Vec<String>,
    pub removed_wals: Vec<String>,
    pub examined_entries: u64,
    pub metadata_bytes: u64,
    pub removed_bytes: u64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureMode {
    Crash,
    StorageFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteBoundary {
    Prepared,
    WalAppended,
    WalSynced,
    Visible,
}

impl WriteBoundary {
    fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "write.prepared",
            Self::WalAppended => "write.wal_appended",
            Self::WalSynced => "write.wal_synced",
            Self::Visible => "write.visible",
        }
    }
}

impl FailureMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Crash => "crash",
            Self::StorageFull => "storage-full",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushBoundary {
    WalSynced,
    SegmentSynced,
    SuccessorWalSynced,
    ManifestPublished,
}

impl FlushBoundary {
    fn as_str(self) -> &'static str {
        match self {
            Self::WalSynced => "flush.wal_synced",
            Self::SegmentSynced => "flush.segment_synced",
            Self::SuccessorWalSynced => "flush.successor_wal_synced",
            Self::ManifestPublished => "flush.manifest_published",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionBoundary {
    SegmentSynced,
    ManifestPublished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotInstallBoundary {
    SegmentsSynced,
    SuccessorWalSynced,
    ManifestPublished,
}

impl SnapshotInstallBoundary {
    fn as_str(self) -> &'static str {
        match self {
            Self::SegmentsSynced => "snapshot.segments_synced",
            Self::SuccessorWalSynced => "snapshot.successor_wal_synced",
            Self::ManifestPublished => "snapshot.manifest_published",
        }
    }
}

impl CompactionBoundary {
    fn as_str(self) -> &'static str {
        match self {
            Self::SegmentSynced => "compaction.segment_synced",
            Self::ManifestPublished => "compaction.manifest_published",
        }
    }
}

/// Single-writer RRFlow LSM database with crash-ordered WAL → segment →
/// manifest publication. RRFlow's model, differential, recovery, and benchmark
/// gates define its correctness and performance requirements.
pub struct Database {
    root: AnchoredPath,
    manifests: ManifestStore,
    manifest: Manifest,
    wal: WalWriter,
    memtable: Arc<crate::mutable_memory::MutableGeneration>,
    mutable_memory: crate::MutableMemory,
    segments: Vec<Arc<Segment>>,
    page_cache: SharedPageCache,
    segment_io: SharedIoContext,
    segment_open_evidence: SegmentOpenEvidence,
    segment_row_group_budget: SegmentRowGroupBudget,
    segment_compression: SegmentCompressionPolicy,
    maintenance: MaintenancePolicy,
    mutable_limits: crate::MutableLimits,
    compaction: CompactionPolicy,
    compaction_limits: crate::CompactionLimits,
    reclamation: crate::ReclamationLimits,
    reclamation_pending: bool,
    maintenance_stats: MaintenanceStats,
    wal_payload_bytes: usize,
    writer_requires_reopen: Option<&'static str>,
    transaction_snapshots: Arc<crate::transaction::TransactionSnapshots>,
    active_read_views: Arc<std::sync::Mutex<ActiveReadViews>>,
    active_read_view_limit: usize,
    pinned_read_byte_limit: u64,
    projected_output: crate::ProjectedOutputMemory,
    write_path_diagnostics: Option<WritePathCollector>,
}

impl Database {
    pub fn create(root: &Path) -> Result<Self> {
        Self::create_with_options(root, DatabaseOptions::default())
    }

    pub fn create_with_page_cache(root: &Path, page_cache_bytes: usize) -> Result<Self> {
        Self::create_with_options(
            root,
            DatabaseOptions {
                page_cache_bytes,
                ..DatabaseOptions::default()
            },
        )
    }

    pub fn create_with_options(root: &Path, options: DatabaseOptions) -> Result<Self> {
        Self::create_with_options_and_application_format(root, options, None)
    }

    /// Creates a database whose manifests authenticate the higher-level key
    /// encoding interpreted by its owner. The physical engine treats this
    /// non-zero identity as opaque and preserves it across every publication.
    pub fn create_with_application_format(
        root: &Path,
        options: DatabaseOptions,
        application_format: u64,
    ) -> Result<Self> {
        if application_format == 0 {
            return Err(Error::InvalidConfiguration(
                "application format identity must be non-zero".into(),
            ));
        }
        Self::create_with_options_and_application_format(root, options, Some(application_format))
    }

    fn create_with_options_and_application_format(
        root: &Path,
        options: DatabaseOptions,
        application_format: Option<u64>,
    ) -> Result<Self> {
        Self::create_anchored(&AnchoredPath::from_path(root)?, options, application_format)
    }

    pub fn create_anchored(
        root: &AnchoredPath,
        options: DatabaseOptions,
        application_format: Option<u64>,
    ) -> Result<Self> {
        if application_format == Some(0) {
            return Err(Error::InvalidConfiguration(
                "application format identity must be non-zero".into(),
            ));
        }
        let options = options.validate()?;
        let segment_io = IoContext::new(options.segment_io)?;
        match root.metadata() {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || !metadata.is_dir()
                    || root.entries()?.next().is_some()
                {
                    return Err(Error::InvalidManifest(
                        "new database path exists and is not an empty non-symlink directory".into(),
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                root.create_dir()?;
            }
            Err(error) => return Err(error.into()),
        }
        let root = root.pin_directory()?;
        let root = root.with_recovery_reserve(options.recovery_reserve_bytes)?;
        let manifests = ManifestStore::create_anchored(&root)?;
        root.join(WAL_DIRECTORY).create_dir()?;
        root.join(SEGMENT_DIRECTORY).create_dir()?;
        let manifest = new_manifest(1, None, 0, 0, 1, Vec::new(), application_format)?;
        let wal = WalWriter::create_at_anchored(&wal_path(&root, 1), 1)?;
        manifests.publish(&manifest, None)?;
        let mutable_memory = crate::MutableMemory::new(options.mutable_retention);
        Ok(Self {
            root: root.to_owned(),
            manifests,
            manifest,
            wal,
            memtable: mutable_memory.empty(0),
            mutable_memory,
            segments: Vec::new(),
            page_cache: new_page_cache_with_policy(
                options.page_cache_bytes,
                options.page_cache_policy,
            )?,
            segment_io,
            segment_open_evidence: SegmentOpenEvidence::default(),
            segment_row_group_budget: options.segment_row_group_budget,
            segment_compression: options.segment_compression,
            maintenance: options.maintenance,
            mutable_limits: options.mutable_limits,
            compaction: options.compaction,
            compaction_limits: options.compaction_limits,
            reclamation: options.reclamation,
            reclamation_pending: true,
            maintenance_stats: MaintenanceStats::default(),
            wal_payload_bytes: 0,
            writer_requires_reopen: None,
            transaction_snapshots: Arc::new(crate::transaction::TransactionSnapshots::new(
                options.transaction_limits,
            )),
            active_read_views: Arc::new(std::sync::Mutex::new(ActiveReadViews::default())),
            active_read_view_limit: options.active_read_views,
            pinned_read_byte_limit: options.pinned_read_bytes,
            projected_output: crate::ProjectedOutputMemory::new(options.projected_output_bytes),
            write_path_diagnostics: None,
        })
    }

    pub fn open(root: &Path) -> Result<Self> {
        Self::open_with_options(root, DatabaseOptions::default())
    }

    pub fn open_with_page_cache(root: &Path, page_cache_bytes: usize) -> Result<Self> {
        Self::open_with_options(
            root,
            DatabaseOptions {
                page_cache_bytes,
                ..DatabaseOptions::default()
            },
        )
    }

    pub fn open_with_options(root: &Path, options: DatabaseOptions) -> Result<Self> {
        Self::open_anchored(&AnchoredPath::open_root(root)?, options)
    }

    pub fn open_anchored(root: &AnchoredPath, options: DatabaseOptions) -> Result<Self> {
        let root = root.pin_directory()?;
        let total_started = Instant::now();
        validate_existing_database_layout(&root)?;
        let options = options.validate()?;
        let root = root.with_recovery_reserve(options.recovery_reserve_bytes)?;
        let mut recovery_budget = crate::recovery_budget::RecoveryBudget::new(options.recovery);
        let segment_io = IoContext::new(options.segment_io)?;
        let manifest_started = Instant::now();
        let manifests = ManifestStore::open_existing_anchored(&root)?;
        let (_, manifest) = manifests
            .current_bounded(&mut recovery_budget)?
            .ok_or_else(|| Error::InvalidManifest("database has no CURRENT manifest".into()))?;
        let manifest_ms = manifest_started.elapsed().as_millis() as u64;
        let page_cache =
            new_page_cache_with_policy(options.page_cache_bytes, options.page_cache_policy)?;
        let mut segment_bytes = 0u64;
        for segment in &manifest.segments {
            recovery_budget.file(segment.bytes)?;
            segment_bytes = segment_bytes.checked_add(segment.bytes).ok_or_else(|| {
                Error::InvalidManifest("recovery segment byte count overflow".into())
            })?;
        }
        let segments_started = Instant::now();
        let mut segments = Vec::with_capacity(manifest.segments.len());
        let mut segment_open_evidence = SegmentOpenEvidence::default();
        for expected in &manifest.segments {
            recovery_budget.tick()?;
            let segment_path = root
                .join(SEGMENT_DIRECTORY)
                .join(format!("{}.seg", expected.id));
            require_existing_regular_file(&segment_path, "segment")?;
            let segment = Segment::open_expected_with_cache_and_io(
                &segment_path,
                expected,
                Arc::clone(&page_cache),
                Arc::clone(&segment_io),
            )?;
            if &segment.descriptor != expected {
                return Err(Error::InvalidManifest(format!(
                    "segment {} does not match its manifest descriptor",
                    expected.id
                )));
            }
            segment_open_evidence.merge(segment.open_evidence())?;
            segments.push(segment);
        }
        let segments_ms = segments_started.elapsed().as_millis() as u64;
        let path = wal_path(&root, manifest.wal_start_sequence);
        require_existing_regular_file(&path, "active WAL")?;
        let mut memtable = Memtable::at_sequence(manifest.durable_sequence);
        let wal_started = Instant::now();
        let mut wal_file = OpenOptions::new().read(true).write(true).open(&path)?;
        let recovery = crate::wal::replay_file_bounded(
            &mut wal_file,
            manifest.wal_start_sequence,
            &mut recovery_budget,
            |batch| {
                #[cfg(all(feature = "crash-test", not(test)))]
                crate::crash_test::boundary(
                    crate::crash_test::Point::RecoveryRead,
                    root.diagnostic_path(),
                );
                memtable.apply_owned_recovered(batch, options.mutable_limits)
            },
        )?;
        if let Some(offset) = recovery.torn_tail {
            return Err(Error::TornTail { offset });
        }
        let wal_recovery_ms = wal_started.elapsed().as_millis() as u64;
        let wal_payload_bytes = recovery.payload_bytes;
        let maintenance_stats = MaintenanceStats {
            peak_wal_payload_bytes: wal_payload_bytes,
            peak_memtable_versions: memtable.version_count(),
            peak_accounted_memtable_bytes: memtable.approximate_bytes(),
            ..MaintenanceStats::default()
        };
        let wal = WalWriter::from_recovered_file(&path, wal_file, recovery)?;
        tracing::info!(
            target: "rrd_lsm::open",
            path = %root.display(),
            manifest_ms,
            segment_count = manifest.segments.len(),
            segment_bytes,
            segments_ms,
            wal_recovery_ms,
            wal_payload_bytes,
            page_cache_policy = options.page_cache_policy.kind(),
            page_cache_protected_capacity_basis_points =
                options.page_cache_policy.protected_capacity_basis_points(),
            page_cache_capacity_bytes = options.page_cache_bytes,
            row_group_target_bytes = options.segment_row_group_budget.target_bytes,
            row_group_max_rows = options.segment_row_group_budget.max_rows,
            segment_compression = options.segment_compression.kind(),
            compression_minimum_savings_basis_points =
                options.segment_compression.minimum_savings_basis_points(),
            compression_maximum_page_logical_bytes =
                options.segment_compression.maximum_page_logical_bytes(),
            total_ms = total_started.elapsed().as_millis() as u64,
            "rrflowKV LSM open phases completed"
        );
        let mutable_memory = crate::MutableMemory::new(options.mutable_retention);
        Ok(Self {
            root: root.to_owned(),
            manifests,
            manifest,
            wal,
            memtable: Arc::new(mutable_memory.wrap(memtable)?),
            mutable_memory,
            segments: segments.into_iter().map(Arc::new).collect(),
            page_cache,
            segment_io,
            segment_open_evidence,
            segment_row_group_budget: options.segment_row_group_budget,
            segment_compression: options.segment_compression,
            maintenance: options.maintenance,
            mutable_limits: options.mutable_limits,
            compaction: options.compaction,
            compaction_limits: options.compaction_limits,
            reclamation: options.reclamation,
            reclamation_pending: true,
            maintenance_stats,
            wal_payload_bytes,
            writer_requires_reopen: None,
            transaction_snapshots: Arc::new(crate::transaction::TransactionSnapshots::new(
                options.transaction_limits,
            )),
            active_read_views: Arc::new(std::sync::Mutex::new(ActiveReadViews::default())),
            active_read_view_limit: options.active_read_views,
            pinned_read_byte_limit: options.pinned_read_bytes,
            projected_output: crate::ProjectedOutputMemory::new(options.projected_output_bytes),
            write_path_diagnostics: None,
        })
    }

    pub fn root(&self) -> &Path {
        self.root.diagnostic_path()
    }

    pub fn anchored_root(&self) -> &AnchoredPath {
        &self.root
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            sequence: self.memtable.maximum_sequence(),
        }
    }

    pub fn mutable_memory(&self) -> crate::MutableMemory {
        self.mutable_memory.clone()
    }

    /// Coherent aggregate native transaction owner/buffer charge observation.
    pub fn transaction_ownership(&self) -> Result<(u64, u64)> {
        self.transaction_snapshots.ownership()
    }

    /// Aggregate live stream ownership; returned batches own separate output reservations.
    pub fn read_view_ownership(&self) -> Result<(u64, u64)> {
        ActiveReadViews::ownership(&self.active_read_views)
    }

    pub fn projected_output_memory(&self) -> crate::ProjectedOutputMemory {
        self.projected_output.clone()
    }

    /// Captures one immutable logical/physical read view and returns a bounded
    /// projected stream that owns its memtable and segment generations. The
    /// caller can consume the stream without retaining a borrow of `Database`.
    pub fn begin_projected_read(
        &self,
        mut request: ProjectedReadRequest,
    ) -> Result<ProjectedReadStream> {
        let current_sequence = self.snapshot().sequence;
        request.validate(current_sequence)?;
        request.budget.max_active_views = request
            .budget
            .max_active_views
            .min(self.active_read_view_limit);
        request.budget.max_pinned_bytes = request
            .budget
            .max_pinned_bytes
            .min(self.pinned_read_byte_limit);
        let segments = self
            .segments
            .iter()
            .filter(|segment| {
                request.snapshot.sequence >= segment.descriptor.minimum_sequence
                    && request.ranges.iter().any(|range| {
                        range.overlaps(
                            segment.descriptor.first_key.as_slice(),
                            segment.descriptor.last_key.as_slice(),
                        )
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        let run_count = segments
            .len()
            .checked_add(usize::from(self.memtable.version_count() != 0))
            .ok_or_else(|| Error::InvalidProjectedRead("projected run count overflow".into()))?;
        if run_count > request.budget.max_runs {
            return Err(Error::ProjectedReadLimit {
                resource: ProjectedReadResource::Runs,
                limit: request.budget.max_runs as u64,
                observed: run_count as u64,
            });
        }
        let memtable_bytes = u64::try_from(self.memtable.approximate_bytes()).map_err(|_| {
            Error::InvalidProjectedRead("captured memtable bytes exceed u64".into())
        })?;
        let pinned_bytes =
            self.manifest
                .segments
                .iter()
                .try_fold(memtable_bytes, |total, segment| {
                    total.checked_add(segment.bytes).ok_or_else(|| {
                        Error::InvalidProjectedRead("pinned read-view bytes overflow".into())
                    })
                })?;
        let cache_access = next_projected_cache_access()?;
        let lease = ActiveReadViews::acquire(
            &self.active_read_views,
            self.manifest.clone(),
            pinned_bytes,
            request.budget,
        )?;
        let view = ReadView {
            sequence: request.snapshot.sequence,
            manifest: self.manifest.digest.clone(),
            memtable: Arc::clone(&self.memtable),
            segments,
            pinned_bytes,
        };
        Ok(ProjectedReadStream::new(
            view,
            request.ranges,
            request.projection,
            request.budget,
            cache_access,
            lease,
            self.projected_output.clone(),
        ))
    }

    /// Begins one snapshot-isolated physical transaction and pins its read
    /// sequence against history-pruning compaction until commit, rollback, or
    /// drop.
    pub fn begin_transaction(&self) -> Result<crate::Transaction> {
        let transaction =
            crate::Transaction::begin(self.snapshot(), Arc::clone(&self.transaction_snapshots))?;
        tracing::debug!(
            target: "rrd_lsm::transaction",
            snapshot_sequence = transaction.snapshot().sequence,
            outcome = "begun",
            "physical transaction began"
        );
        Ok(transaction)
    }

    /// Atomically conflict-checks and publishes one transaction write set.
    /// Only keys written by the transaction participate in conflict detection.
    pub fn commit_transaction(
        &mut self,
        transaction: crate::Transaction,
        durability: Durability,
    ) -> Result<crate::TransactionCommit> {
        self.commit_transaction_inner(transaction, durability, None, WriteLockWait::default())
    }

    /// Commits through the ordinary transaction path while attaching
    /// caller-measured adapter mutex waits to an explicitly enabled diagnostic
    /// sample. The context is ignored when collection is disabled.
    pub fn commit_transaction_with_write_context(
        &mut self,
        transaction: crate::Transaction,
        durability: Durability,
        lock_wait: WriteLockWait,
    ) -> Result<crate::TransactionCommit> {
        self.commit_transaction_inner(transaction, durability, None, lock_wait)
    }

    /// Commits through one deterministic write-boundary failure. This is the
    /// transaction-level fault surface used to prove that a semantic write
    /// set never becomes a collection of independently visible families.
    pub fn commit_transaction_with_failure(
        &mut self,
        transaction: crate::Transaction,
        durability: Durability,
        boundary: WriteBoundary,
        mode: FailureMode,
    ) -> Result<crate::TransactionCommit> {
        self.commit_transaction_inner(
            transaction,
            durability,
            Some((boundary, mode)),
            WriteLockWait::default(),
        )
    }

    fn commit_transaction_inner(
        &mut self,
        mut transaction: crate::Transaction,
        durability: Durability,
        failure: Option<(WriteBoundary, FailureMode)>,
        lock_wait: WriteLockWait,
    ) -> Result<crate::TransactionCommit> {
        let mut diagnostics = self.write_path_diagnostics.as_ref().map(|_| {
            PendingWriteDiagnostics::start(
                lock_wait,
                self.maintenance_stats,
                self.memtable.approximate_bytes(),
            )
        });
        let validation_started = diagnostics.is_some().then(PhaseClock::start);
        self.ensure_writer_ready()?;
        self.validate_transaction_owner(&transaction)?;
        for key in transaction.pending().keys() {
            if let Some(conflicting_sequence) = self.latest_sequence_for_key(key)? {
                if conflicting_sequence > transaction.snapshot().sequence {
                    tracing::warn!(
                        target: "rrd_lsm::transaction",
                        snapshot_sequence = transaction.snapshot().sequence,
                        conflicting_sequence,
                        mutation_count = transaction.write_count(),
                        outcome = "conflict",
                        "physical transaction commit denied"
                    );
                    return Err(Error::TransactionConflict {
                        snapshot_sequence: transaction.snapshot().sequence,
                        conflicting_sequence,
                    });
                }
            }
        }
        if let Some(started) = validation_started {
            diagnostics
                .as_mut()
                .expect("diagnostic recorder checked")
                .phases
                .transaction_validation = started.finish();
        }
        let mutation_count = transaction.write_count();
        if mutation_count == 0 {
            tracing::debug!(
                target: "rrd_lsm::transaction",
                snapshot_sequence = transaction.snapshot().sequence,
                mutation_count,
                outcome = "read_only",
                "physical transaction committed without a write"
            );
            return Ok(crate::TransactionCommit {
                snapshot_sequence: transaction.snapshot().sequence,
                mutation_count,
                receipt: None,
            });
        }
        let snapshot_sequence = transaction.snapshot().sequence;
        let batch_started = diagnostics.is_some().then(PhaseClock::start);
        let batch = WriteBatch::new(transaction.take_mutations())?;
        if let Some(started) = batch_started {
            diagnostics
                .as_mut()
                .expect("diagnostic recorder checked")
                .phases
                .batch_prepare = started.finish();
        }
        let receipt = self.write_owned_inner(batch, durability, failure, diagnostics)?;
        tracing::debug!(
            target: "rrd_lsm::transaction",
            snapshot_sequence,
            mutation_count,
            first_sequence = receipt.first_sequence,
            last_sequence = receipt.last_sequence,
            durable = receipt.durable,
            outcome = "committed",
            "physical transaction committed"
        );
        Ok(crate::TransactionCommit {
            snapshot_sequence,
            mutation_count,
            receipt: Some(receipt),
        })
    }

    pub(crate) fn validate_transaction_owner(
        &self,
        transaction: &crate::Transaction,
    ) -> Result<()> {
        if transaction.belongs_to(&self.transaction_snapshots) {
            Ok(())
        } else {
            Err(Error::TransactionDatabaseMismatch)
        }
    }

    fn latest_sequence_for_key(&self, key: &[u8]) -> Result<Option<u64>> {
        let read_sequence = self.snapshot().sequence;
        let mut latest = self
            .memtable
            .get_version(key, read_sequence)
            .map(|version| version.sequence);
        for segment in &self.segments {
            if let Some(version) = segment.get_version(key, read_sequence)? {
                if latest.is_none_or(|sequence| version.sequence > sequence) {
                    latest = Some(version.sequence);
                }
            }
        }
        Ok(latest)
    }

    pub fn write(&mut self, batch: &WriteBatch, durability: Durability) -> Result<AppendReceipt> {
        self.write_inner(batch, durability, None)
    }

    pub fn write_with_failure(
        &mut self,
        batch: &WriteBatch,
        durability: Durability,
        boundary: WriteBoundary,
        mode: FailureMode,
    ) -> Result<AppendReceipt> {
        self.write_inner(batch, durability, Some((boundary, mode)))
    }

    fn write_inner(
        &mut self,
        batch: &WriteBatch,
        durability: Durability,
        failure: Option<(WriteBoundary, FailureMode)>,
    ) -> Result<AppendReceipt> {
        self.ensure_writer_ready()?;
        let charge = MutableCharge::batch(batch)?;
        self.mutable_limits.check(charge)?;
        let payload = batch.encode()?;
        self.prepare_write(batch, payload.len(), charge)?;
        let prepared_mutable = self.mutable_memory.prepare(&self.memtable, charge)?;
        inject_write_failure(failure, WriteBoundary::Prepared)?;
        let append_durability = append_durability(durability, failure);
        let receipt = self
            .wal
            .append_encoded_write_batch(batch, &payload, append_durability)?;
        self.inject_post_append_failure(failure)?;
        self.inject_post_wal_failure(failure, &receipt)?;
        if prepared_mutable
            .apply(
                &mut self.memtable,
                batch,
                receipt.first_sequence,
                receipt.last_sequence,
            )
            .is_err()
        {
            // WAL may already be durable. Never report this as an absent write.
            self.writer_requires_reopen = Some("mutable-publication");
            return Err(Error::RecoveryRequired {
                boundary: "mutable-publication",
            });
        }
        self.wal_payload_bytes = self.wal_payload_bytes.saturating_add(payload.len());
        self.record_memtable_peak();
        self.inject_post_visibility_failure(failure)?;
        Ok(receipt)
    }

    /// Owned fast path used by adapters that build a batch for immediate
    /// commit. Keys and values move into the memtable after WAL encoding rather
    /// than being cloned a second time.
    pub fn write_owned(
        &mut self,
        batch: WriteBatch,
        durability: Durability,
    ) -> Result<AppendReceipt> {
        let diagnostics = self.write_path_diagnostics.as_ref().map(|_| {
            PendingWriteDiagnostics::start(
                WriteLockWait::default(),
                self.maintenance_stats,
                self.memtable.approximate_bytes(),
            )
        });
        self.write_owned_inner(batch, durability, None, diagnostics)
    }

    pub fn write_owned_with_failure(
        &mut self,
        batch: WriteBatch,
        durability: Durability,
        boundary: WriteBoundary,
        mode: FailureMode,
    ) -> Result<AppendReceipt> {
        let diagnostics = self.write_path_diagnostics.as_ref().map(|_| {
            PendingWriteDiagnostics::start(
                WriteLockWait::default(),
                self.maintenance_stats,
                self.memtable.approximate_bytes(),
            )
        });
        self.write_owned_inner(batch, durability, Some((boundary, mode)), diagnostics)
    }

    fn write_owned_inner(
        &mut self,
        batch: WriteBatch,
        durability: Durability,
        failure: Option<(WriteBoundary, FailureMode)>,
        mut diagnostics: Option<PendingWriteDiagnostics>,
    ) -> Result<AppendReceipt> {
        self.ensure_writer_ready()?;
        let charge = MutableCharge::batch(&batch)?;
        self.mutable_limits.check(charge)?;
        let mutation_count = batch.len();
        let encode_started = diagnostics.is_some().then(PhaseClock::start);
        let payload = batch.encode()?;
        if let Some(started) = encode_started {
            diagnostics
                .as_mut()
                .expect("diagnostic recorder checked")
                .phases
                .batch_encode = started.finish();
        }
        let payload_bytes = payload.len();
        let maintenance_started = diagnostics.is_some().then(PhaseClock::start);
        self.prepare_write(&batch, payload.len(), charge)?;
        if let Some(started) = maintenance_started {
            diagnostics
                .as_mut()
                .expect("diagnostic recorder checked")
                .phases
                .maintenance = started.finish();
        }
        let prepared_mutable = self.mutable_memory.prepare(&self.memtable, charge)?;
        inject_write_failure(failure, WriteBoundary::Prepared)?;
        let append_durability = append_durability(durability, failure);
        let receipt = if let Some(diagnostics) = diagnostics.as_mut() {
            let (receipt, wal) = self.wal.append_encoded_write_batch_profiled(
                &batch,
                &payload,
                append_durability,
            )?;
            diagnostics.phases.wal_record_prepare = wal.record_prepare;
            diagnostics.phases.wal_initial_reservation = wal.initial_reservation;
            diagnostics.phases.wal_write = wal.write;
            diagnostics.phases.wal_sync = wal.sync;
            receipt
        } else {
            self.wal
                .append_encoded_write_batch(&batch, &payload, append_durability)?
        };
        self.inject_post_append_failure(failure)?;
        self.inject_post_wal_failure(failure, &receipt)?;
        let apply_started = diagnostics.is_some().then(PhaseClock::start);
        if prepared_mutable
            .apply_owned(
                &mut self.memtable,
                batch,
                receipt.first_sequence,
                receipt.last_sequence,
            )
            .is_err()
        {
            // WAL may already be durable. Never report this as an absent write.
            self.writer_requires_reopen = Some("mutable-publication");
            return Err(Error::RecoveryRequired {
                boundary: "mutable-publication",
            });
        }
        if let Some(started) = apply_started {
            diagnostics
                .as_mut()
                .expect("diagnostic recorder checked")
                .phases
                .memtable_apply = started.finish();
        }
        let bookkeeping_started = diagnostics.is_some().then(PhaseClock::start);
        self.wal_payload_bytes = self.wal_payload_bytes.saturating_add(payload.len());
        self.record_memtable_peak();
        self.inject_post_visibility_failure(failure)?;
        if let Some(started) = bookkeeping_started {
            diagnostics
                .as_mut()
                .expect("diagnostic recorder checked")
                .phases
                .bookkeeping = started.finish();
        }
        if let Some(diagnostics) = diagnostics {
            let sample = diagnostics.finish(
                durability,
                mutation_count,
                payload_bytes,
                &receipt,
                self.maintenance_stats,
                self.memtable.approximate_bytes(),
            );
            self.write_path_diagnostics
                .as_mut()
                .expect("collector remains enabled for one mutable write")
                .record(sample);
        }
        Ok(receipt)
    }

    fn inject_post_append_failure(
        &mut self,
        failure: Option<(WriteBoundary, FailureMode)>,
    ) -> Result<()> {
        let Some((WriteBoundary::WalAppended, mode)) = failure else {
            return Ok(());
        };
        let boundary = WriteBoundary::WalAppended.as_str();
        self.writer_requires_reopen = Some(boundary);
        Err(Error::InjectedFailure {
            mode: mode.as_str(),
            boundary,
        })
    }

    fn inject_post_wal_failure(
        &mut self,
        failure: Option<(WriteBoundary, FailureMode)>,
        receipt: &AppendReceipt,
    ) -> Result<()> {
        let Some((WriteBoundary::WalSynced, mode)) = failure else {
            return Ok(());
        };
        if !receipt.durable {
            self.wal.sync()?;
        }
        let boundary = WriteBoundary::WalSynced.as_str();
        self.writer_requires_reopen = Some(boundary);
        Err(Error::InjectedFailure {
            mode: mode.as_str(),
            boundary,
        })
    }

    fn inject_post_visibility_failure(
        &mut self,
        failure: Option<(WriteBoundary, FailureMode)>,
    ) -> Result<()> {
        #[cfg(test)]
        crate::fault_io::crash::visible(self.root.diagnostic_path());
        #[cfg(all(feature = "crash-test", not(test)))]
        crate::crash_test::boundary(
            crate::crash_test::Point::Visible,
            self.root.diagnostic_path(),
        );
        let Some((WriteBoundary::Visible, mode)) = failure else {
            return Ok(());
        };
        let boundary = WriteBoundary::Visible.as_str();
        self.writer_requires_reopen = Some(boundary);
        Err(Error::InjectedFailure {
            mode: mode.as_str(),
            boundary,
        })
    }

    fn ensure_writer_ready(&self) -> Result<()> {
        self.manifests.ensure_publication_ready()?;
        match self.writer_requires_reopen {
            Some(boundary) => Err(Error::RecoveryRequired { boundary }),
            None => Ok(()),
        }
    }

    fn prepare_write(
        &mut self,
        batch: &WriteBatch,
        payload_bytes: usize,
        charge: MutableCharge,
    ) -> Result<()> {
        self.reclaim_if_pending()?;
        let projected_bytes = self.wal_payload_bytes.saturating_add(payload_bytes);
        let projected_versions = self.memtable.version_count().saturating_add(batch.len());
        let active = self.memtable.version_count() != 0;
        if active
            && (projected_bytes > self.maintenance.wal_payload_max_bytes
                || projected_versions > self.maintenance.memtable_max_versions
                || self
                    .mutable_limits
                    .needs_flush(self.memtable.charge(), charge))
        {
            self.maintenance_stats.write_stalls =
                self.maintenance_stats.write_stalls.saturating_add(1);
            match self.flush_memtable(self.manifest.created_at) {
                Ok(Some(_)) => {
                    self.maintenance_stats.automatic_flushes =
                        self.maintenance_stats.automatic_flushes.saturating_add(1);
                    match self.compact_if_needed(self.manifest.created_at) {
                        Ok(Some(outcome)) => {
                            self.maintenance_stats.automatic_compactions = self
                                .maintenance_stats
                                .automatic_compactions
                                .saturating_add(1);
                            self.maintenance_stats.compaction_input_bytes = self
                                .maintenance_stats
                                .compaction_input_bytes
                                .saturating_add(outcome.input_bytes);
                            self.maintenance_stats.compaction_output_bytes = self
                                .maintenance_stats
                                .compaction_output_bytes
                                .saturating_add(outcome.output_bytes);
                            self.maintenance_stats.peak_compaction_buffer_bytes = self
                                .maintenance_stats
                                .peak_compaction_buffer_bytes
                                .max(outcome.peak_buffer_bytes);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            self.maintenance_stats.failed_compactions =
                                self.maintenance_stats.failed_compactions.saturating_add(1);
                            return Err(error);
                        }
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    self.maintenance_stats.failed_flushes =
                        self.maintenance_stats.failed_flushes.saturating_add(1);
                    return Err(error);
                }
            }
        }
        let l0_segments = self
            .segments
            .iter()
            .filter(|segment| segment.descriptor.level == 0)
            .count() as u64;
        if l0_segments >= self.compaction_limits.l0_segments {
            self.compact_if_needed(self.manifest.created_at)?;
            let remaining = self
                .segments
                .iter()
                .filter(|segment| segment.descriptor.level == 0)
                .count() as u64;
            self.compaction_limits.check(
                crate::MaintenanceResource::L0Segments,
                remaining.saturating_add(1),
            )?;
        }
        self.reclaim_if_pending()?;
        self.mutable_limits
            .check(self.memtable.charge().checked_add(charge)?)?;
        if payload_bytes > self.maintenance.wal_payload_max_bytes
            || batch.len() > self.maintenance.memtable_max_versions
        {
            self.maintenance_stats.oversized_batches =
                self.maintenance_stats.oversized_batches.saturating_add(1);
        }
        Ok(())
    }

    fn record_memtable_peak(&mut self) {
        self.maintenance_stats.peak_accounted_memtable_bytes = self
            .maintenance_stats
            .peak_accounted_memtable_bytes
            .max(self.memtable.approximate_bytes());
        self.maintenance_stats.peak_wal_payload_bytes = self
            .maintenance_stats
            .peak_wal_payload_bytes
            .max(self.wal_payload_bytes);
        self.maintenance_stats.peak_memtable_versions = self
            .maintenance_stats
            .peak_memtable_versions
            .max(self.memtable.version_count());
    }

    pub fn sync(&mut self) -> Result<u64> {
        self.ensure_writer_ready()?;
        self.wal.sync()
    }

    /// Flushes the current WAL-backed memtable into one immutable segment.
    /// Publication order makes either the old or new manifest fully recoverable
    /// at every crash boundary.
    pub fn flush_memtable(&mut self, at: u64) -> Result<Option<Manifest>> {
        self.flush_memtable_inner(at, None)
    }

    /// Deterministic fault-injection entry point for recovery matrices. A
    /// `ManifestPublished` failure updates in-memory state before returning so
    /// callers that do not immediately simulate process death still fail safe.
    pub fn flush_memtable_with_failure(
        &mut self,
        at: u64,
        boundary: FlushBoundary,
        mode: FailureMode,
    ) -> Result<Option<Manifest>> {
        self.flush_memtable_inner(at, Some((boundary, mode)))
    }

    fn flush_memtable_inner(
        &mut self,
        at: u64,
        failure: Option<(FlushBoundary, FailureMode)>,
    ) -> Result<Option<Manifest>> {
        self.ensure_writer_ready()?;
        if self.memtable.version_count() == 0 {
            self.wal.sync()?;
            return Ok(None);
        }
        let l0_segments = self
            .segments
            .iter()
            .filter(|segment| segment.descriptor.level == 0)
            .count() as u64;
        self.compaction_limits.check(
            crate::MaintenanceResource::L0Segments,
            l0_segments.saturating_add(1),
        )?;
        let sequence = self.wal.sync()?;
        inject_failure(failure, FlushBoundary::WalSynced)?;
        let (segment, _) = Segment::write_from_memtable_with_cache(
            &self.root.join(SEGMENT_DIRECTORY),
            &self.memtable,
            self.segment_row_group_budget,
            self.segment_compression,
            Arc::clone(&self.page_cache),
            Arc::clone(&self.segment_io),
        )?;
        inject_failure(failure, FlushBoundary::SegmentSynced)?;
        let next_sequence = sequence
            .checked_add(1)
            .ok_or_else(|| Error::InvalidBatch("sequence overflow at flush".into()))?;
        let next_path = wal_path(&self.root, next_sequence);
        let successor = if next_path.try_exists()? {
            WalWriter::open_empty_at_anchored(&next_path, next_sequence)?
        } else {
            WalWriter::create_at_anchored(&next_path, next_sequence)?
        };
        inject_failure(failure, FlushBoundary::SuccessorWalSynced)?;
        let mut descriptors = self.manifest.segments.clone();
        descriptors.push(segment.descriptor.clone());
        let next = new_manifest(
            self.manifest
                .generation
                .checked_add(1)
                .ok_or_else(|| Error::InvalidManifest("manifest generation overflow".into()))?,
            Some(self.manifest.digest.clone()),
            at,
            sequence,
            next_sequence,
            descriptors,
            self.manifest.application_format,
        )?;
        self.manifests.publish(&next, Some(&self.manifest.digest))?;
        self.wal = successor;
        self.memtable = self.mutable_memory.empty(sequence);
        self.wal_payload_bytes = 0;
        self.segments.push(Arc::new(segment));
        self.manifest = next.clone();
        self.reclamation_pending = true;
        inject_failure(failure, FlushBoundary::ManifestPublished)?;
        Ok(Some(next))
    }

    pub fn checkpoint(&self, name: &str, at: u64) -> Result<Checkpoint> {
        self.manifests.checkpoint(name, &self.manifest.digest, at)
    }

    pub fn checkpoints(&self) -> Result<Vec<Checkpoint>> {
        self.manifests.checkpoints()
    }

    pub fn release_checkpoint(&self, name: &str) -> Result<bool> {
        self.manifests.release_checkpoint(name)
    }

    /// Flushes the active WAL-backed memtable and captures the resulting
    /// immutable manifest closure. The published manifest points at an empty
    /// successor WAL, so every state byte required by the snapshot is carried
    /// by the authenticated segment inventory.
    pub fn export_snapshot_bundle(&mut self, at: u64) -> Result<SnapshotBundle> {
        self.flush_memtable(at)?;
        let segments = self
            .manifest
            .segments
            .iter()
            .map(|descriptor| {
                let bytes = self
                    .root
                    .join(SEGMENT_DIRECTORY)
                    .join(format!("{}.seg", descriptor.id))
                    .read()?;
                Ok(SnapshotSegment {
                    descriptor: descriptor.clone(),
                    bytes,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        SnapshotBundle::new(self.manifest.clone(), segments)
    }

    /// Flushes and writes the authenticated snapshot bundle directly to a new
    /// file, retaining only a fixed copy buffer plus one segment during deep
    /// validation.
    pub fn export_snapshot_file(
        &mut self,
        at: u64,
        destination: impl crate::filesystem::AnchorInput,
    ) -> Result<SnapshotBundleFile> {
        self.flush_memtable(at)?;
        SnapshotBundleFile::create(
            self.manifest.clone(),
            &self.root.join(SEGMENT_DIRECTORY),
            destination,
        )
    }

    pub fn export_snapshot_file_with_failure(
        &mut self,
        at: u64,
        destination: impl crate::filesystem::AnchorInput,
        boundary: SnapshotExportBoundary,
        mode: FailureMode,
    ) -> Result<SnapshotBundleFile> {
        self.flush_memtable(at)?;
        SnapshotBundleFile::create_with_hook(
            self.manifest.clone(),
            &self.root.join(SEGMENT_DIRECTORY),
            destination,
            |reached| {
                if reached == boundary {
                    Err(Error::InjectedFailure {
                        mode: mode.as_str(),
                        boundary: boundary.as_str(),
                    })
                } else {
                    Ok(())
                }
            },
        )
    }

    /// Replaces the current logical database image with a newer authenticated
    /// snapshot. Segment files and an empty continuation WAL are durable before
    /// a single manifest-pointer publication makes the imported state visible.
    pub fn install_snapshot_bundle(
        &mut self,
        bundle: &SnapshotBundle,
        at: u64,
    ) -> Result<Manifest> {
        self.install_snapshot_bundle_inner(bundle, at, None)
    }

    /// Installs a file-backed bundle without decoding the whole transfer into
    /// one allocation. RRD LSM currently retains opened segments in memory, so
    /// the resident engine image remains a separately measured bound.
    pub fn install_snapshot_file(
        &mut self,
        bundle: &SnapshotBundleFile,
        at: u64,
    ) -> Result<Manifest> {
        self.install_snapshot_file_inner(bundle, at, None)
    }

    pub fn install_snapshot_file_with_failure(
        &mut self,
        bundle: &SnapshotBundleFile,
        at: u64,
        boundary: SnapshotInstallBoundary,
        mode: FailureMode,
    ) -> Result<Manifest> {
        self.install_snapshot_file_inner(bundle, at, Some((boundary, mode)))
    }

    fn install_snapshot_file_inner(
        &mut self,
        bundle: &SnapshotBundleFile,
        at: u64,
        failure: Option<(SnapshotInstallBoundary, FailureMode)>,
    ) -> Result<Manifest> {
        self.ensure_writer_ready()?;
        self.ensure_no_active_transactions("snapshot installation")?;
        if bundle.source_manifest.application_format != self.manifest.application_format {
            return Err(Error::InvalidManifest(
                "snapshot application format differs from the target database".into(),
            ));
        }
        let source_sequence = bundle.source_manifest.durable_sequence;
        let current_sequence = self.snapshot().sequence;
        if source_sequence == current_sequence
            && self.manifest.segments == bundle.source_manifest.segments
        {
            return Ok(self.manifest.clone());
        }
        if source_sequence <= current_sequence {
            return Err(Error::InvalidManifest(format!(
                "snapshot sequence {source_sequence} does not advance local sequence {current_sequence}"
            )));
        }

        let segment_directory = self.root.join(SEGMENT_DIRECTORY);
        let mut imported = Vec::with_capacity(bundle.source_manifest.segments.len());
        for (index, descriptor) in bundle.descriptors().enumerate() {
            let bytes = bundle.segment_bytes(index)?;
            imported.push(Arc::new(Segment::install_snapshot_bytes_with_cache(
                &segment_directory,
                descriptor,
                &bytes,
                Arc::clone(&self.page_cache),
                Arc::clone(&self.segment_io),
            )?));
        }
        inject_snapshot_install_failure(failure, SnapshotInstallBoundary::SegmentsSynced)?;

        let next_sequence = source_sequence
            .checked_add(1)
            .ok_or_else(|| Error::InvalidManifest("snapshot sequence overflowed".into()))?;
        let next_path = wal_path(&self.root, next_sequence);
        let successor = if next_path.try_exists()? {
            WalWriter::open_empty_at_anchored(&next_path, next_sequence)?
        } else {
            WalWriter::create_at_anchored(&next_path, next_sequence)?
        };
        inject_snapshot_install_failure(failure, SnapshotInstallBoundary::SuccessorWalSynced)?;

        let previous = self.manifest.digest.clone();
        let manifest = new_manifest(
            self.manifest
                .generation
                .checked_add(1)
                .ok_or_else(|| Error::InvalidManifest("manifest generation overflowed".into()))?,
            Some(previous.clone()),
            at.max(self.manifest.created_at)
                .max(bundle.source_manifest.created_at),
            source_sequence,
            next_sequence,
            bundle.source_manifest.segments.clone(),
            self.manifest.application_format,
        )?;
        self.manifests.publish(&manifest, Some(&previous))?;
        self.wal = successor;
        self.memtable = self.mutable_memory.empty(source_sequence);
        self.wal_payload_bytes = 0;
        self.segments = imported;
        self.manifest = manifest.clone();
        self.reclamation_pending = true;
        inject_snapshot_install_failure(failure, SnapshotInstallBoundary::ManifestPublished)?;
        Ok(manifest)
    }

    pub fn install_snapshot_bundle_with_failure(
        &mut self,
        bundle: &SnapshotBundle,
        at: u64,
        boundary: SnapshotInstallBoundary,
        mode: FailureMode,
    ) -> Result<Manifest> {
        self.install_snapshot_bundle_inner(bundle, at, Some((boundary, mode)))
    }

    fn install_snapshot_bundle_inner(
        &mut self,
        bundle: &SnapshotBundle,
        at: u64,
        failure: Option<(SnapshotInstallBoundary, FailureMode)>,
    ) -> Result<Manifest> {
        self.ensure_writer_ready()?;
        self.ensure_no_active_transactions("snapshot installation")?;
        bundle.validate()?;
        if bundle.source_manifest.application_format != self.manifest.application_format {
            return Err(Error::InvalidManifest(
                "snapshot application format differs from the target database".into(),
            ));
        }
        let source_sequence = bundle.source_manifest.durable_sequence;
        let current_sequence = self.snapshot().sequence;
        if source_sequence == current_sequence
            && self.manifest.segments == bundle.source_manifest.segments
        {
            return Ok(self.manifest.clone());
        }
        if source_sequence <= current_sequence {
            return Err(Error::InvalidManifest(format!(
                "snapshot sequence {source_sequence} does not advance local sequence {current_sequence}"
            )));
        }

        let segment_directory = self.root.join(SEGMENT_DIRECTORY);
        let mut imported = Vec::with_capacity(bundle.segments.len());
        for bundled in &bundle.segments {
            imported.push(Arc::new(Segment::install_snapshot_bytes_with_cache(
                &segment_directory,
                &bundled.descriptor,
                &bundled.bytes,
                Arc::clone(&self.page_cache),
                Arc::clone(&self.segment_io),
            )?));
        }
        inject_snapshot_install_failure(failure, SnapshotInstallBoundary::SegmentsSynced)?;

        let next_sequence = source_sequence
            .checked_add(1)
            .ok_or_else(|| Error::InvalidManifest("snapshot sequence overflowed".into()))?;
        let next_path = wal_path(&self.root, next_sequence);
        let successor = if next_path.try_exists()? {
            WalWriter::open_empty_at_anchored(&next_path, next_sequence)?
        } else {
            WalWriter::create_at_anchored(&next_path, next_sequence)?
        };
        inject_snapshot_install_failure(failure, SnapshotInstallBoundary::SuccessorWalSynced)?;

        let previous = self.manifest.digest.clone();
        let manifest = new_manifest(
            self.manifest
                .generation
                .checked_add(1)
                .ok_or_else(|| Error::InvalidManifest("manifest generation overflowed".into()))?,
            Some(previous.clone()),
            at.max(self.manifest.created_at)
                .max(bundle.source_manifest.created_at),
            source_sequence,
            next_sequence,
            bundle.source_manifest.segments.clone(),
            self.manifest.application_format,
        )?;
        self.manifests.publish(&manifest, Some(&previous))?;
        self.wal = successor;
        self.memtable = self.mutable_memory.empty(source_sequence);
        self.wal_payload_bytes = 0;
        self.segments = imported;
        self.manifest = manifest.clone();
        self.reclamation_pending = true;
        inject_snapshot_install_failure(failure, SnapshotInstallBoundary::ManifestPublished)?;
        Ok(manifest)
    }

    /// Executes one deterministic leveled compaction step while retaining
    /// exactly the versions observable at `protected` snapshots and at the
    /// current durable head. Input selection is bounded and records are merged
    /// through forward-only segment cursors. Output is partitioned at key
    /// boundaries. Hard input byte/version limits include every version of a
    /// hot key before decoding; the output target alone is not a hard bound.
    ///
    /// Named checkpoints keep their original manifests and segments and are
    /// handled independently by garbage collection.
    pub fn compact(
        &mut self,
        protected: &[Snapshot],
        at: u64,
    ) -> Result<Option<CompactionOutcome>> {
        self.compact_inner(protected, at, None, true, false)
    }

    pub fn compact_with_failure(
        &mut self,
        protected: &[Snapshot],
        at: u64,
        boundary: CompactionBoundary,
        mode: FailureMode,
    ) -> Result<Option<CompactionOutcome>> {
        self.compact_inner(protected, at, Some((boundary, mode)), true, false)
    }

    /// Runs one leveled step only when the configured L0 threshold or a lower
    /// level debt is present. Automatic maintenance conservatively retains all
    /// MVCC versions because a plain [`Snapshot`] is intentionally copyable and
    /// cannot act as a lifetime-tracked reclamation lease. Explicit compaction
    /// supplies the precise protected snapshot set and may prune history.
    pub fn compact_if_needed(&mut self, at: u64) -> Result<Option<CompactionOutcome>> {
        self.compact_inner(&[], at, None, false, true)
    }

    fn compact_inner(
        &mut self,
        protected: &[Snapshot],
        at: u64,
        failure: Option<(CompactionBoundary, FailureMode)>,
        force: bool,
        retain_all_versions: bool,
    ) -> Result<Option<CompactionOutcome>> {
        self.flush_memtable_inner(at, None)?;
        let started = Instant::now();
        let Some(candidate) = self.select_compaction_candidate(force, started)? else {
            return Ok(None);
        };
        let input_bytes = self.admit_compaction_input(&candidate, started)?;
        let durable = self.manifest.durable_sequence;
        let mut protected_sequences = protected
            .iter()
            .map(|snapshot| snapshot.sequence)
            .filter(|sequence| *sequence <= durable)
            .collect::<BTreeSet<_>>();
        protected_sequences.extend(self.transaction_snapshots.sequences()?);
        protected_sequences.insert(durable);
        let previous_manifest = self.manifest.digest.clone();
        let input_segments = candidate.indices.len();
        let merge = self.merge_compaction_candidate(
            &candidate,
            durable,
            &protected_sequences,
            retain_all_versions,
            started,
        )?;
        let CompactionMerge {
            segments: compacted_segments,
            output_segments,
            input_versions,
            output_versions,
            output_bytes,
            peak_buffer_bytes,
        } = merge;
        self.compaction_limits.check_elapsed(started)?;
        inject_compaction_failure(failure, CompactionBoundary::SegmentSynced)?;
        let selected = candidate.indices.iter().copied().collect::<BTreeSet<_>>();
        let mut descriptors = self
            .segments
            .iter()
            .enumerate()
            .filter(|(index, _)| !selected.contains(index))
            .map(|(_, segment)| segment.descriptor.clone())
            .collect::<Vec<_>>();
        descriptors.extend(
            compacted_segments
                .iter()
                .map(|segment| segment.descriptor.clone()),
        );
        let next = new_manifest(
            self.manifest
                .generation
                .checked_add(1)
                .ok_or_else(|| Error::InvalidManifest("manifest generation overflow".into()))?,
            Some(previous_manifest.clone()),
            at,
            durable,
            self.manifest.wal_start_sequence,
            descriptors,
            self.manifest.application_format,
        )?;
        self.compaction_limits.check_elapsed(started)?;
        self.manifests
            .publish(&next, Some(previous_manifest.as_str()))?;
        let mut retained_segments = std::mem::take(&mut self.segments)
            .into_iter()
            .enumerate()
            .filter_map(|(index, segment)| (!selected.contains(&index)).then_some(segment))
            .collect::<Vec<_>>();
        retained_segments.extend(compacted_segments.into_iter().map(Arc::new));
        self.segments = retained_segments;
        self.manifest = next.clone();
        self.reclamation_pending = true;
        inject_compaction_failure(failure, CompactionBoundary::ManifestPublished)?;
        Ok(Some(CompactionOutcome {
            previous_manifest,
            manifest: next.digest,
            input_segments,
            output_segments,
            input_versions,
            output_versions,
            protected_sequences: protected_sequences.into_iter().collect(),
            source_level: candidate.source_level,
            target_level: candidate.target_level,
            input_bytes,
            output_bytes,
            peak_buffer_bytes,
            history_pruned: !retain_all_versions,
        }))
    }

    /// Select a complete overlap closure that fits one maintenance quantum.
    /// A blocked upper level must not hide feasible work that clears its target.
    fn select_compaction_candidate(
        &self,
        force: bool,
        started: Instant,
    ) -> Result<Option<CompactionCandidate>> {
        let mut denied = None;
        for source_level in 0..self.compaction.max_level {
            self.compaction_limits.check_elapsed(started)?;
            let mut sources = self
                .segments
                .iter()
                .enumerate()
                .filter(|(_, segment)| segment.descriptor.level == source_level)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let blocked = denied.is_some();
            let threshold = if source_level == 0 {
                self.compaction.l0_compaction_trigger
            } else {
                2
            };
            if sources.is_empty() || (!force && !blocked && sources.len() < threshold) {
                continue;
            }
            sources.sort_by(|left, right| {
                let left = &self.segments[*left].descriptor;
                let right = &self.segments[*right].descriptor;
                if source_level == 0 {
                    left.maximum_sequence
                        .cmp(&right.maximum_sequence)
                        .then_with(|| left.first_key.cmp(&right.first_key))
                        .then_with(|| left.id.cmp(&right.id))
                } else {
                    left.first_key
                        .cmp(&right.first_key)
                        .then_with(|| left.id.cmp(&right.id))
                }
            });
            for take in (1..=sources.len().min(self.compaction.max_input_segments)).rev() {
                let Some(candidate) =
                    self.compaction_overlap_candidate(&sources[..take], source_level, started)?
                else {
                    continue;
                };
                // Do not disguise a rejected merge of multiple source segments
                // as a successful move of just one. A sole blocking target may
                // move down to free its parent; this is finite in max_level.
                if candidate.indices.len() < 2 && !(sources.len() == 1 && (force || blocked)) {
                    continue;
                }
                match self.admit_compaction_input(&candidate, started) {
                    Ok(_) => return Ok(Some(candidate)),
                    Err(
                        error @ Error::MaintenanceLimit {
                            resource:
                                crate::MaintenanceResource::InputBytes
                                | crate::MaintenanceResource::InputLogicalBytes
                                | crate::MaintenanceResource::InputVersions,
                            ..
                        },
                    ) => {
                        if denied.is_none() {
                            denied = Some(error);
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        match denied {
            Some(error) => Err(error),
            None => Ok(None),
        }
    }

    fn compaction_overlap_candidate(
        &self,
        sources: &[usize],
        source_level: u8,
        started: Instant,
    ) -> Result<Option<CompactionCandidate>> {
        let first_key = sources
            .iter()
            .map(|index| self.segments[*index].descriptor.first_key.as_slice())
            .min()
            .expect("nonempty source selection");
        let last_key = sources
            .iter()
            .map(|index| self.segments[*index].descriptor.last_key.as_slice())
            .max()
            .expect("nonempty source selection");
        let target_level = source_level + 1;
        let mut indices = sources.to_vec();
        for (index, segment) in self.segments.iter().enumerate() {
            self.compaction_limits.check_elapsed(started)?;
            if segment.descriptor.level == target_level
                && ranges_overlap(
                    first_key,
                    last_key,
                    &segment.descriptor.first_key,
                    &segment.descriptor.last_key,
                )
            {
                indices.push(index);
                if indices.len() > self.compaction.max_input_segments {
                    return Ok(None);
                }
            }
        }
        indices.sort_unstable();
        Ok(Some(CompactionCandidate {
            indices,
            source_level,
            target_level,
        }))
    }

    fn admit_compaction_input(
        &self,
        candidate: &CompactionCandidate,
        started: Instant,
    ) -> Result<u64> {
        let mut logical_bytes = 0u64;
        let mut versions = 0u64;
        let mut physical_bytes = 0u64;
        for index in &candidate.indices {
            let segment = &self.segments[*index];
            physical_bytes = physical_bytes
                .checked_add(segment.descriptor.bytes)
                .ok_or_else(|| Error::InvalidSegment("compaction physical byte overflow".into()))?;
            logical_bytes = logical_bytes
                .checked_add(segment.compaction_logical_bytes()?)
                .ok_or_else(|| Error::InvalidSegment("compaction logical byte overflow".into()))?;
            versions = versions
                .checked_add(segment.descriptor.entries)
                .ok_or_else(|| Error::InvalidSegment("compaction version count overflow".into()))?;
            self.compaction_limits
                .check(crate::MaintenanceResource::InputBytes, physical_bytes)?;
            self.compaction_limits
                .check(crate::MaintenanceResource::InputLogicalBytes, logical_bytes)?;
            self.compaction_limits
                .check(crate::MaintenanceResource::InputVersions, versions)?;
            self.compaction_limits.check_elapsed(started)?;
        }
        Ok(physical_bytes)
    }

    fn merge_compaction_candidate(
        &self,
        candidate: &CompactionCandidate,
        durable: u64,
        protected_sequences: &BTreeSet<u64>,
        retain_all_versions: bool,
        started: Instant,
    ) -> Result<CompactionMerge> {
        let selected = candidate.indices.iter().copied().collect::<BTreeSet<_>>();
        let mut cursors = candidate
            .indices
            .iter()
            .map(|index| self.segments[*index].record_cursor())
            .collect::<Vec<_>>();
        let mut current = cursors
            .iter_mut()
            .map(|cursor| cursor.next_record())
            .collect::<Result<Vec<_>>>()?;
        let mut buffered = BTreeMap::<Vec<u8>, Vec<VersionedValue>>::new();
        let mut buffered_bytes = 0usize;
        let mut peak_buffer_bytes = 0usize;
        let mut compacted_segments = Vec::new();
        let mut input_versions = 0u64;
        let mut output_versions = 0u64;
        let mut output_bytes = 0u64;

        while let Some(key) = current
            .iter()
            .filter_map(|record| record.as_ref().map(|record| record.key.as_slice()))
            .min()
            .map(<[u8]>::to_vec)
        {
            self.compaction_limits.check_elapsed(started)?;
            let mut versions = BTreeMap::<u64, Option<Box<[u8]>>>::new();
            for (cursor, record) in cursors.iter_mut().zip(current.iter_mut()) {
                while record.as_ref().is_some_and(|record| record.key == key) {
                    self.compaction_limits.check_elapsed(started)?;
                    let observed = record.take().expect("record was checked");
                    match versions.get(&observed.version.sequence) {
                        Some(existing) if existing != &observed.version.value => {
                            return Err(Error::InvalidSegment(format!(
                                "segments disagree for sequence {}",
                                observed.version.sequence
                            )));
                        }
                        Some(_) => {}
                        None => {
                            input_versions = input_versions.saturating_add(1);
                            versions.insert(observed.version.sequence, observed.version.value);
                        }
                    }
                    *record = cursor.next_record()?;
                }
            }
            let retained = if retain_all_versions {
                versions
                    .into_iter()
                    .map(|(sequence, value)| VersionedValue { sequence, value })
                    .collect::<Vec<_>>()
            } else {
                let retained_sequences = protected_sequences
                    .iter()
                    .filter_map(|sequence| {
                        versions
                            .range(..=*sequence)
                            .next_back()
                            .map(|(sequence, _)| *sequence)
                    })
                    .collect::<BTreeSet<_>>();
                versions
                    .into_iter()
                    .filter(|(sequence, _)| retained_sequences.contains(sequence))
                    .map(|(sequence, value)| VersionedValue { sequence, value })
                    .collect::<Vec<_>>()
            };
            let tombstone_only = retained.iter().all(|version| version.value.is_none());
            let shadows_unselected = self.segments.iter().enumerate().any(|(index, segment)| {
                !selected.contains(&index)
                    && key >= segment.descriptor.first_key
                    && key <= segment.descriptor.last_key
            });
            if retained.is_empty()
                || (!retain_all_versions && tombstone_only && !shadows_unselected)
            {
                continue;
            }
            let group_bytes = version_group_bytes(&key, &retained);
            if !buffered.is_empty()
                && buffered_bytes.saturating_add(group_bytes) > self.compaction.target_segment_bytes
            {
                self.compaction_limits.check_elapsed(started)?;
                self.compaction_limits.check(
                    crate::MaintenanceResource::OutputSegments,
                    compacted_segments.len() as u64 + 1,
                )?;
                let segment = self.write_compaction_partition(
                    std::mem::take(&mut buffered),
                    durable,
                    candidate.target_level,
                )?;
                output_bytes = output_bytes.saturating_add(segment.descriptor.bytes);
                compacted_segments.push(segment);
                buffered_bytes = 0;
            }
            output_versions = output_versions.saturating_add(retained.len() as u64);
            buffered_bytes = buffered_bytes.saturating_add(group_bytes);
            peak_buffer_bytes = peak_buffer_bytes.max(buffered_bytes);
            buffered.insert(key, retained);
        }
        if !buffered.is_empty() {
            self.compaction_limits.check_elapsed(started)?;
            self.compaction_limits.check(
                crate::MaintenanceResource::OutputSegments,
                compacted_segments.len() as u64 + 1,
            )?;
            let segment =
                self.write_compaction_partition(buffered, durable, candidate.target_level)?;
            output_bytes = output_bytes.saturating_add(segment.descriptor.bytes);
            compacted_segments.push(segment);
        }
        Ok(CompactionMerge {
            output_segments: compacted_segments.len(),
            segments: compacted_segments,
            input_versions,
            output_versions,
            output_bytes,
            peak_buffer_bytes,
        })
    }

    fn write_compaction_partition(
        &self,
        versions: BTreeMap<Vec<u8>, Vec<VersionedValue>>,
        durable: u64,
        target_level: u8,
    ) -> Result<Segment> {
        let table = Memtable::from_versions(versions, durable)?;
        let (mut segment, _) = Segment::write_from_memtable_with_cache(
            &self.root.join(SEGMENT_DIRECTORY),
            &table,
            self.segment_row_group_budget,
            self.segment_compression,
            Arc::clone(&self.page_cache),
            Arc::clone(&self.segment_io),
        )?;
        segment.descriptor.level = target_level;
        Ok(segment)
    }

    fn reclaim_if_pending(&mut self) -> Result<()> {
        if self.reclamation_pending {
            let report = self.garbage_collect()?;
            self.reclamation_pending = report.has_more;
            if report.has_more {
                // Finish finite cleanup across retries before creating more debt.
                return Err(Error::ReclamationPending);
            }
        }
        Ok(())
    }

    /// Reclaims a bounded prefix only after validating the complete admitted
    /// current/view/checkpoint reachability closure and candidate inventory.
    pub fn garbage_collect(&self) -> Result<GarbageCollectionReport> {
        self.ensure_writer_ready()?;
        let mut budget = crate::reclamation::ReclamationBudget::new(self.reclamation);
        let mut retained_manifests = BTreeSet::new();
        let mut retained_segments = BTreeMap::new();
        let mut retained_wals = BTreeSet::new();
        retain_manifest(
            &self.manifest,
            &mut retained_manifests,
            &mut retained_segments,
            &mut retained_wals,
            &mut budget,
        )?;
        ActiveReadViews::visit_manifests(&self.active_read_views, |manifest| {
            retain_manifest(
                manifest,
                &mut retained_manifests,
                &mut retained_segments,
                &mut retained_wals,
                &mut budget,
            )
        })?;
        for checkpoint in self.manifests.checkpoints_bounded(&mut budget)? {
            if !retained_manifests.contains(&checkpoint.manifest) {
                let manifest = self
                    .manifests
                    .load_bounded(&checkpoint.manifest, &mut budget)?;
                retain_manifest(
                    &manifest,
                    &mut retained_manifests,
                    &mut retained_segments,
                    &mut retained_wals,
                    &mut budget,
                )?;
            }
        }
        let manifest_directory = self.root.join("manifests");
        let segment_directory = self.root.join(SEGMENT_DIRECTORY);
        let wal_directory = self.root.join(WAL_DIRECTORY);
        // A missing protected object is damage, not permission to clean up the
        // remaining evidence. Validate references before the first delete.
        for identity in &retained_manifests {
            self.manifests.load_bounded(identity, &mut budget)?;
        }
        for (identity, bytes) in &retained_segments {
            budget.entry()?;
            let metadata = segment_directory
                .join(format!("{identity}.seg"))
                .metadata()?;
            if !metadata.is_file() || metadata.len() != *bytes {
                return Err(Error::InvalidManifest("protected reclamation segment is missing, non-regular or has a different length".into()));
            }
        }
        for name in &retained_wals {
            budget.entry()?;
            if !wal_directory.join(name).metadata()?.is_file() {
                return Err(Error::InvalidManifest(
                    "protected reclamation WAL is not a regular file".into(),
                ));
            }
        }
        let segment_ids = retained_segments.into_keys().collect::<BTreeSet<_>>();
        let manifest_candidates = unreachable_files(
            &manifest_directory,
            "json",
            &retained_manifests,
            &mut budget,
        )?;
        let segment_candidates =
            unreachable_files(&segment_directory, "seg", &segment_ids, &mut budget)?;
        let wal_candidates = unreachable_files(&wal_directory, "wal", &retained_wals, &mut budget)?;
        budget.tick()?;
        let mut report = GarbageCollectionReport {
            retained_manifests: retained_manifests.into_iter().collect(),
            retained_segments: segment_ids.into_iter().collect(),
            retained_wals: retained_wals.into_iter().collect(),
            ..GarbageCollectionReport::default()
        };
        report.has_more |= remove_candidates(
            &manifest_directory,
            manifest_candidates,
            &mut report.removed_manifests,
            &mut budget,
        )?;
        report.has_more |= remove_candidates(
            &segment_directory,
            segment_candidates,
            &mut report.removed_segments,
            &mut budget,
        )?;
        report.has_more |= remove_candidates(
            &wal_directory,
            wal_candidates,
            &mut report.removed_wals,
            &mut budget,
        )?;
        report.examined_entries = budget.entries;
        report.metadata_bytes = budget.metadata_bytes;
        report.removed_bytes = budget.deleted_bytes;
        Ok(report)
    }

    pub fn get(&self, key: &[u8], snapshot: Snapshot) -> Result<Option<Vec<u8>>> {
        // The active memtable always contains sequences newer than every
        // published segment. A visible value (including a tombstone) is
        // therefore authoritative for this snapshot and lets hot runtime keys
        // avoid immutable-page I/O entirely.
        if let Some(version) = self.memtable.get_version(key, snapshot.sequence) {
            return Ok(version.value.as_deref().map(<[u8]>::to_vec));
        }
        let mut best_sequence = 0;
        let mut best_value = None;
        for segment in &self.segments {
            if let Some(version) = segment.get_version(key, snapshot.sequence)? {
                if version.sequence > best_sequence {
                    best_sequence = version.sequence;
                    best_value = version.value;
                }
            }
        }
        Ok(best_value.map(|value| value.into_vec()))
    }

    pub fn get_many(&self, keys: &[Vec<u8>], snapshot: Snapshot) -> Result<Vec<Option<Vec<u8>>>> {
        let mut values = vec![None; keys.len()];
        let mut unresolved = Vec::with_capacity(keys.len());
        for (index, key) in keys.iter().enumerate() {
            if let Some(version) = self.memtable.get_version(key, snapshot.sequence) {
                values[index] = version.value.as_deref().map(<[u8]>::to_vec);
            } else {
                unresolved.push(index);
            }
        }
        if unresolved.is_empty() {
            return Ok(values);
        }
        let unresolved_keys = unresolved
            .iter()
            .map(|index| keys[*index].as_slice())
            .collect::<Vec<_>>();
        if self.segments.len() == 1 && self.memtable.version_count() == 0 {
            for (position, version) in self.segments[0]
                .get_versions(&unresolved_keys, snapshot.sequence)?
                .into_iter()
                .enumerate()
            {
                values[unresolved[position]] = version
                    .and_then(|version| version.value)
                    .map(|value| value.into_vec());
            }
            return Ok(values);
        }
        let mut best_sequences = vec![0u64; unresolved.len()];
        for segment in &self.segments {
            for (position, version) in segment
                .get_versions(&unresolved_keys, snapshot.sequence)?
                .into_iter()
                .enumerate()
            {
                if let Some(version) = version {
                    if version.sequence > best_sequences[position] {
                        best_sequences[position] = version.sequence;
                        values[unresolved[position]] = version.value.map(|value| value.into_vec());
                    }
                }
            }
        }
        Ok(values)
    }

    pub fn scan(
        &self,
        start: &[u8],
        end: Option<&[u8]>,
        snapshot: Snapshot,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        if self.segments.is_empty() {
            return Ok(self.memtable.scan(start, end, snapshot.sequence));
        }
        if self.segments.len() == 1 && self.memtable.version_count() == 0 {
            return self.segments[0].scan(start, end, snapshot.sequence);
        }
        let mut visible = BTreeMap::<Vec<u8>, VersionedValue>::new();
        for segment in &self.segments {
            for (key, version) in segment.visible_from(start, end, snapshot.sequence)? {
                if visible
                    .get(&key)
                    .is_none_or(|current| version.sequence > current.sequence)
                {
                    visible.insert(key, version);
                }
            }
        }
        for (key, version) in self.memtable.visible_from(start, end, snapshot.sequence) {
            if visible
                .get(&key)
                .is_none_or(|current| version.sequence > current.sequence)
            {
                visible.insert(key, version);
            }
        }
        Ok(visible
            .into_iter()
            .filter_map(|(key, version)| version.value.map(|value| (key, value.into_vec())))
            .collect())
    }

    /// Scans sorted, disjoint half-open key ranges at one snapshot. Immutable
    /// segments are visited once so ranges sharing a row group share
    /// one authenticated read and decode.
    pub fn scan_ranges(
        &self,
        ranges: &[(Vec<u8>, Vec<u8>)],
        snapshot: Snapshot,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        if ranges.iter().any(|(start, end)| start >= end)
            || ranges.windows(2).any(|ranges| ranges[0].1 > ranges[1].0)
        {
            return Err(Error::InvalidConfiguration(
                "multi-range scan requires sorted, disjoint, non-empty half-open ranges".into(),
            ));
        }
        if ranges.is_empty() {
            return Ok(Vec::new());
        }
        let mut visible = BTreeMap::<Vec<u8>, VersionedValue>::new();
        for segment in &self.segments {
            for (key, version) in segment.visible_ranges(ranges, snapshot.sequence)? {
                if visible
                    .get(&key)
                    .is_none_or(|current| version.sequence > current.sequence)
                {
                    visible.insert(key, version);
                }
            }
        }
        for (start, end) in ranges {
            for (key, version) in self
                .memtable
                .visible_from(start, Some(end), snapshot.sequence)
            {
                if visible
                    .get(&key)
                    .is_none_or(|current| version.sequence > current.sequence)
                {
                    visible.insert(key, version);
                }
            }
        }
        Ok(visible
            .into_iter()
            .filter_map(|(key, version)| version.value.map(|value| (key, value.into_vec())))
            .collect())
    }

    /// Visits visible rows in key order without requiring callers to retain a
    /// second materialized copy of the result. The mutable-only path borrows
    /// directly from the memtable; mixed immutable/mutable state retains the
    /// canonical merge behavior used by [`Self::scan`].
    pub fn scan_each<E, F>(
        &self,
        start: &[u8],
        end: Option<&[u8]>,
        snapshot: Snapshot,
        mut visit: F,
    ) -> std::result::Result<(), E>
    where
        E: From<Error>,
        F: FnMut(&[u8], &[u8]) -> std::result::Result<(), E>,
    {
        if self.segments.is_empty() {
            return self
                .memtable
                .scan_each(start, end, snapshot.sequence, visit);
        }
        for (key, value) in self.scan(start, end, snapshot).map_err(E::from)? {
            visit(&key, &value)?;
        }
        Ok(())
    }

    pub fn memtable(&self) -> &Memtable {
        &self.memtable
    }

    pub fn mutable_limits(&self) -> crate::MutableLimits {
        self.mutable_limits
    }

    pub fn maintenance_policy(&self) -> MaintenancePolicy {
        self.maintenance
    }

    pub fn compaction_policy(&self) -> CompactionPolicy {
        self.compaction
    }

    pub fn segment_row_group_budget(&self) -> SegmentRowGroupBudget {
        self.segment_row_group_budget
    }

    pub fn segment_compression_policy(&self) -> SegmentCompressionPolicy {
        self.segment_compression
    }

    pub fn l0_segment_count(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.descriptor.level == 0)
            .count()
    }

    pub fn compaction_debt_segments(&self) -> usize {
        self.l0_segment_count()
            .saturating_sub(self.compaction.l0_compaction_trigger.saturating_sub(1))
    }

    pub fn maintenance_stats(&self) -> MaintenanceStats {
        self.maintenance_stats
    }

    /// Enables bounded, process-local measurement for subsequently accepted
    /// owned write batches. An active buffer must be drained before another
    /// one is enabled so evidence cannot be discarded implicitly.
    pub fn enable_write_path_diagnostics(&mut self, capacity: usize) -> Result<()> {
        if self.write_path_diagnostics.is_some() {
            return Err(Error::InvalidConfiguration(
                "write path diagnostics are already enabled".into(),
            ));
        }
        self.write_path_diagnostics = Some(WritePathCollector::new(capacity)?);
        Ok(())
    }

    /// Disables and drains the current process-local write diagnostic buffer.
    pub fn take_write_path_diagnostics(&mut self) -> Option<WritePathDiagnostics> {
        self.write_path_diagnostics
            .take()
            .map(WritePathCollector::finish)
    }

    pub fn wal_payload_bytes(&self) -> usize {
        self.wal_payload_bytes
    }

    /// Current shared immutable-page residency and effectiveness counters.
    pub fn page_cache_stats(&self) -> crate::PageCacheStats {
        page_cache_stats(&self.page_cache)
    }

    pub fn segment_io_stats(&self) -> SegmentIoStats {
        self.segment_io.stats()
    }

    /// Immutable phase evidence captured while authenticating the segments
    /// referenced by `CURRENT`. Later page reads cannot mutate this record.
    pub fn segment_open_evidence(&self) -> &SegmentOpenEvidence {
        &self.segment_open_evidence
    }

    fn ensure_no_active_transactions(&self, operation: &'static str) -> Result<()> {
        if self.transaction_snapshots.is_empty()? {
            Ok(())
        } else {
            Err(Error::ActiveTransactions { operation })
        }
    }
}

fn new_manifest(
    generation: u64,
    parent: Option<String>,
    created_at: u64,
    durable_sequence: u64,
    wal_start_sequence: u64,
    segments: Vec<crate::SegmentDescriptor>,
    application_format: Option<u64>,
) -> Result<Manifest> {
    match application_format {
        Some(application_format) => Manifest::new_with_application_format(
            generation,
            parent,
            created_at,
            durable_sequence,
            wal_start_sequence,
            segments,
            application_format,
        ),
        None => Manifest::new(
            generation,
            parent,
            created_at,
            durable_sequence,
            wal_start_sequence,
            segments,
        ),
    }
}

fn validate_existing_database_layout(root: &AnchoredPath) -> Result<()> {
    for relative in ["", WAL_DIRECTORY, SEGMENT_DIRECTORY] {
        let path = root.join(relative);
        let metadata = path.metadata().map_err(|error| {
            Error::io(
                "validating database directory",
                path.diagnostic_path(),
                error,
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(Error::InvalidManifest(format!(
                "database directory {} is missing, symbolic, or invalid",
                path.display()
            )));
        }
    }
    Ok(())
}

fn require_existing_regular_file(path: &AnchoredPath, object: &str) -> Result<()> {
    let metadata = path
        .metadata()
        .map_err(|error| Error::io("validating database file", path.diagnostic_path(), error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::InvalidManifest(format!(
            "{object} {} is symbolic or invalid",
            path.display()
        )));
    }
    Ok(())
}

pub(crate) fn wal_path(root: &AnchoredPath, starting_sequence: u64) -> AnchoredPath {
    root.join(WAL_DIRECTORY)
        .join(format!("{starting_sequence:020}.wal"))
}

fn ranges_overlap(
    left_first: &[u8],
    left_last: &[u8],
    right_first: &[u8],
    right_last: &[u8],
) -> bool {
    left_first <= right_last && right_first <= left_last
}

fn version_group_bytes(key: &[u8], versions: &[VersionedValue]) -> usize {
    versions.iter().fold(0usize, |bytes, version| {
        bytes
            .saturating_add(key.len())
            .saturating_add(version.value.as_ref().map_or(0, |value| value.len()))
            .saturating_add(std::mem::size_of::<VersionedValue>())
    })
}

fn inject_failure(
    configured: Option<(FlushBoundary, FailureMode)>,
    reached: FlushBoundary,
) -> Result<()> {
    if let Some((boundary, mode)) = configured.filter(|(boundary, _)| *boundary == reached) {
        return Err(Error::InjectedFailure {
            mode: mode.as_str(),
            boundary: boundary.as_str(),
        });
    }
    Ok(())
}

fn inject_write_failure(
    configured: Option<(WriteBoundary, FailureMode)>,
    reached: WriteBoundary,
) -> Result<()> {
    if let Some((boundary, mode)) = configured.filter(|(boundary, _)| *boundary == reached) {
        return Err(Error::InjectedFailure {
            mode: mode.as_str(),
            boundary: boundary.as_str(),
        });
    }
    Ok(())
}

fn append_durability(
    requested: Durability,
    failure: Option<(WriteBoundary, FailureMode)>,
) -> Durability {
    if matches!(failure, Some((WriteBoundary::WalAppended, _))) {
        // The injected boundary must occur after a complete frame write but
        // before an authoritative sync. Recovery may therefore observe the
        // complete frame, but can never observe only part of its operations.
        Durability::Buffered
    } else {
        requested
    }
}

fn inject_snapshot_install_failure(
    configured: Option<(SnapshotInstallBoundary, FailureMode)>,
    reached: SnapshotInstallBoundary,
) -> Result<()> {
    if let Some((boundary, mode)) = configured.filter(|(boundary, _)| *boundary == reached) {
        return Err(Error::InjectedFailure {
            mode: mode.as_str(),
            boundary: boundary.as_str(),
        });
    }
    Ok(())
}

fn inject_compaction_failure(
    configured: Option<(CompactionBoundary, FailureMode)>,
    reached: CompactionBoundary,
) -> Result<()> {
    if let Some((boundary, mode)) = configured.filter(|(boundary, _)| *boundary == reached) {
        return Err(Error::InjectedFailure {
            mode: mode.as_str(),
            boundary: boundary.as_str(),
        });
    }
    Ok(())
}

fn retain_manifest(
    manifest: &Manifest,
    manifests: &mut BTreeSet<String>,
    segments: &mut BTreeMap<String, u64>,
    wals: &mut BTreeSet<String>,
    budget: &mut crate::reclamation::ReclamationBudget,
) -> Result<()> {
    budget.entry()?;
    if manifests.contains(&manifest.digest) {
        return Ok(());
    }
    budget.charge_entries(manifest.segments.len() as u64)?;
    manifest.validate()?;
    manifests.insert(manifest.digest.clone());
    wals.insert(format!("{:020}.wal", manifest.wal_start_sequence));
    for segment in &manifest.segments {
        budget.tick()?;
        if let Some(previous) = segments.insert(segment.id.clone(), segment.bytes) {
            if previous != segment.bytes {
                return Err(Error::InvalidManifest(
                    "protected manifests disagree about segment length".into(),
                ));
            }
        }
    }
    Ok(())
}

fn unreachable_files(
    directory: &AnchoredPath,
    extension: &str,
    retained: &BTreeSet<String>,
    budget: &mut crate::reclamation::ReclamationBudget,
) -> Result<Vec<(AnchoredPath, String, u64)>> {
    let mut candidates = Vec::new();
    for entry in directory.entries()? {
        budget.entry()?;
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_file()
            || entry
                .relative_path()
                .extension()
                .and_then(|value| value.to_str())
                != Some(extension)
        {
            continue;
        }
        let file_name = entry
            .relative_path()
            .file_name()
            .ok_or_else(|| Error::InvalidManifest("missing file name".into()))?
            .to_string_lossy()
            .into_owned();
        let identity = entry
            .relative_path()
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| Error::InvalidManifest("non-UTF-8 storage object name".into()))?;
        if !retained.contains(if extension == "wal" {
            file_name.as_str()
        } else {
            identity
        }) {
            candidates.push((entry, file_name, metadata.len()));
        }
    }
    candidates.sort_by(|left, right| left.1.cmp(&right.1));
    Ok(candidates)
}

fn remove_candidates(
    directory: &AnchoredPath,
    candidates: Vec<(AnchoredPath, String, u64)>,
    removed: &mut Vec<String>,
    budget: &mut crate::reclamation::ReclamationBudget,
) -> Result<bool> {
    let result = (|| {
        for (path, file_name, bytes) in candidates {
            if !budget.admit_delete(bytes)? {
                return Ok(true);
            }
            path.remove_file()?;
            removed.push(file_name);
        }
        Ok(false)
    })();
    // Even a partial or failed deletion cycle synchronizes completed removals.
    let sync = if removed.is_empty() {
        Ok(())
    } else {
        directory.sync_directory().map_err(Error::Io)
    };
    match result {
        Err(error) => {
            let _ = sync;
            Err(error)
        }
        Ok(more) => {
            sync?;
            Ok(more)
        }
    }
}
