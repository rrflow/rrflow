mod format;
mod output;
#[cfg(feature = "physical-policy-lab")]
mod physical_policy_lab;
mod reader;

use self::format::{
    encode, parse_metadata, require_current_format, sha256_hex, PageDescriptor, PageKind,
    ParsedMetadata, RowGroupDescriptor, RowGroupFilter, FOOTER_BYTES, INDEX_HEADER_BYTES,
    MAX_KEY_BYTES, MAX_SEGMENT_BYTES, MAX_VALUE_BYTES, SEGMENT_HEADER_BYTES,
};
use crate::filesystem::{AnchoredPath, OpenOptions};
use crate::io::{IoContext, SelectedIo, SharedIoContext};
use crate::{Error, Memtable, Result, SegmentDescriptor, SegmentIoPolicy, VersionedValue};
use arrow_buffer::{alloc::Allocation, Buffer, MutableBuffer};
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub use self::format::{
    SegmentCompressionPolicy, SegmentRowGroupBudget, COMPRESSION_BASIS_POINTS,
    DEFAULT_COMPRESSION_MAXIMUM_PAGE_LOGICAL_BYTES,
    DEFAULT_COMPRESSION_MINIMUM_SAVINGS_BASIS_POINTS, DEFAULT_ROW_GROUP_MAX_ROWS,
    DEFAULT_ROW_GROUP_TARGET_BYTES, DEFAULT_SEGMENT_COMPRESSION_POLICY, SEGMENT_FORMAT_VERSION,
    SEGMENT_KEY_CODEC_DIGEST, SEGMENT_PAGE_FORMAT_DIGEST, SEGMENT_SCHEMA_DIGEST,
};
pub use self::output::{
    ProjectedOutputMemory, ProjectedOutputStats, DEFAULT_PROJECTED_OUTPUT_BYTES,
};
#[cfg(feature = "physical-policy-lab")]
pub use self::physical_policy_lab::{
    run_physical_policy_trial, CachePolicyObservation, CandidateDecision, CodecObservation,
    FamilyObservation, FilterObservation, IntegratedCachePolicyObservation, PhysicalPolicyConfig,
    PhysicalPolicyTrial, ReopenedPointMissObservation, ValuePlacementObservation,
    PHYSICAL_POLICY_EVIDENCE_VERSION,
};
pub(crate) use self::reader::{ActiveReadViews, ReadView};
pub use self::reader::{
    ProjectedReadBatch, ProjectedReadBudget, ProjectedReadEvidence, ProjectedReadOutcome,
    ProjectedReadProjection, ProjectedReadRange, ProjectedReadRequest, ProjectedReadResource,
    ProjectedReadStream, DEFAULT_MAX_ACTIVE_READ_VIEWS, DEFAULT_MAX_BATCH_ALLOCATED_BYTES,
    DEFAULT_MAX_BATCH_ROWS, DEFAULT_MAX_OUTPUT_BUFFER_BYTES, DEFAULT_MAX_OUTPUT_ROWS,
    DEFAULT_MAX_PAGE_LOGICAL_BYTES, DEFAULT_MAX_PAGE_REQUESTS, DEFAULT_MAX_PINNED_READ_BYTES,
    DEFAULT_MAX_PROJECTED_READ_RANGES, DEFAULT_MAX_PROJECTED_READ_RUNS,
    DEFAULT_MAX_VERSIONS_EXAMINED, PROJECTED_READ_CONTRACT_VERSION,
};

pub const DEFAULT_PAGE_CACHE_BYTES: usize = 4 * 1024 * 1024;
pub const DEFAULT_PAGE_CACHE_PROTECTED_CAPACITY_BASIS_POINTS: u16 = 8_000;
const PAGE_CACHE_CAPACITY_BASIS_POINTS: usize = 10_000;
pub const SEGMENT_OPEN_EVIDENCE_VERSION: u16 = 3;
const COPY_BUFFER_BYTES: usize = 64 * 1024;
static TEMPORARY_ID: AtomicU64 = AtomicU64::new(1);
static CACHE_ID: AtomicU64 = AtomicU64::new(1);
static CACHE_ACCESS_SCOPE_ID: AtomicU64 = AtomicU64::new(1);

/// Process-local immutable-page replacement policy. This policy is
/// configuration, not durable segment or manifest identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PageCachePolicy {
    ExactLru,
    ScanResistantLru {
        protected_capacity_basis_points: u16,
    },
}

impl Default for PageCachePolicy {
    fn default() -> Self {
        Self::ScanResistantLru {
            protected_capacity_basis_points: DEFAULT_PAGE_CACHE_PROTECTED_CAPACITY_BASIS_POINTS,
        }
    }
}

impl PageCachePolicy {
    pub fn validate(self) -> Result<Self> {
        if let Self::ScanResistantLru {
            protected_capacity_basis_points,
        } = self
        {
            if !(1..10_000).contains(&protected_capacity_basis_points) {
                return Err(Error::InvalidConfiguration(
                    "scan-resistant page cache protected capacity must be between 1 and 9,999 basis points"
                        .into(),
                ));
            }
        }
        Ok(self)
    }

    pub fn kind(self) -> &'static str {
        match self {
            Self::ExactLru => "exact-lru",
            Self::ScanResistantLru { .. } => "scan-resistant-lru",
        }
    }

    pub fn protected_capacity_basis_points(self) -> u16 {
        match self {
            Self::ExactLru => 0,
            Self::ScanResistantLru {
                protected_capacity_basis_points,
            } => protected_capacity_basis_points,
        }
    }

    pub(crate) fn protected_capacity_bytes(self, capacity_bytes: usize) -> Result<usize> {
        self.validate()?;
        capacity_bytes
            .checked_mul(usize::from(self.protected_capacity_basis_points()))
            .map(|bytes| bytes / PAGE_CACHE_CAPACITY_BASIS_POINTS)
            .ok_or_else(|| {
                Error::InvalidConfiguration(
                    "page-cache protected-capacity calculation overflow".into(),
                )
            })
    }
}

/// Process-local immutable-page cache evidence. Counters distinguish physical
/// page reads from bytes borrowed, allocated, copied, or decompressed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageCacheStats {
    pub policy: PageCachePolicy,
    pub capacity_bytes: usize,
    pub resident_bytes: usize,
    pub entries: usize,
    pub probationary_resident_bytes: usize,
    pub protected_resident_bytes: usize,
    pub probationary_entries: usize,
    pub protected_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub admissions: u64,
    pub admission_rejections: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub same_scope_hits: u64,
    pub duplicate_loads: u64,
    pub loads: u64,
    pub bytes_read: u64,
    pub bytes_decoded: u64,
    pub bytes_borrowed: u64,
    pub bytes_allocated: u64,
    pub bytes_copied: u64,
    pub bytes_decompressed: u64,
    pub filter_checks: u64,
    pub filter_negatives: u64,
}

/// Immutable process-local evidence for bytes read while opening authenticated
/// segment files. It is diagnostic state, never part of a manifest or segment
/// identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentOpenEvidence {
    pub contract_version: u16,
    pub segment_count: u64,
    pub physical_bytes: u64,
    pub format_probe_operations: u64,
    pub format_probe_bytes: u64,
    pub full_checksum_operations: u64,
    pub full_checksum_bytes: u64,
    pub metadata_operations: u64,
    pub metadata_bytes: u64,
    pub persisted_filter_count: u64,
    pub persisted_filter_bytes: u64,
    pub none_policy_segment_count: u64,
    pub adaptive_lz4_policy_segment_count: u64,
    pub raw_page_count: u64,
    pub compressed_page_count: u64,
    pub stored_page_bytes: u64,
    pub logical_page_bytes: u64,
    pub semantic_page_operations: u64,
    pub semantic_page_bytes: u64,
}

impl Default for SegmentOpenEvidence {
    fn default() -> Self {
        Self {
            contract_version: SEGMENT_OPEN_EVIDENCE_VERSION,
            segment_count: 0,
            physical_bytes: 0,
            format_probe_operations: 0,
            format_probe_bytes: 0,
            full_checksum_operations: 0,
            full_checksum_bytes: 0,
            metadata_operations: 0,
            metadata_bytes: 0,
            persisted_filter_count: 0,
            persisted_filter_bytes: 0,
            none_policy_segment_count: 0,
            adaptive_lz4_policy_segment_count: 0,
            raw_page_count: 0,
            compressed_page_count: 0,
            stored_page_bytes: 0,
            logical_page_bytes: 0,
            semantic_page_operations: 0,
            semantic_page_bytes: 0,
        }
    }
}

impl SegmentOpenEvidence {
    fn for_metadata(
        physical_bytes: u64,
        metadata: &ParsedMetadata,
        validate_semantic_pages: bool,
    ) -> Result<Self> {
        let metadata_bytes = metadata
            .header_bytes_read
            .checked_add(metadata.index_bytes_read)
            .and_then(|bytes| bytes.checked_add(metadata.padding_bytes_read))
            .ok_or_else(|| Error::InvalidSegment("segment-open metadata bytes overflow".into()))?;
        let (semantic_page_operations, semantic_page_bytes) = if validate_semantic_pages {
            metadata
                .row_groups
                .iter()
                .flat_map(|group| &group.pages)
                .try_fold((0u64, 0u64), |(operations, bytes), page| {
                    Ok::<_, Error>((
                        operations.checked_add(1).ok_or_else(|| {
                            Error::InvalidSegment(
                                "segment-open semantic page count overflow".into(),
                            )
                        })?,
                        bytes
                            .checked_add(page.physical_bytes as u64)
                            .ok_or_else(|| {
                                Error::InvalidSegment(
                                    "segment-open semantic page bytes overflow".into(),
                                )
                            })?,
                    ))
                })?
        } else {
            (0, 0)
        };
        let persisted_filter_count = u64::try_from(metadata.row_groups.len())
            .map_err(|_| Error::InvalidSegment("persisted filter count exceeds u64".into()))?;
        let persisted_filter_bytes =
            metadata.row_groups.iter().try_fold(0u64, |total, group| {
                total
                    .checked_add(group.filter.byte_len() as u64)
                    .ok_or_else(|| Error::InvalidSegment("persisted filter bytes overflow".into()))
            })?;
        let (none_policy_segment_count, adaptive_lz4_policy_segment_count) =
            match metadata.compression_policy {
                SegmentCompressionPolicy::None => (1, 0),
                SegmentCompressionPolicy::AdaptiveLz4 { .. } => (0, 1),
            };
        let (raw_page_count, compressed_page_count, stored_page_bytes, logical_page_bytes) =
            metadata
                .row_groups
                .iter()
                .flat_map(|group| &group.pages)
                .try_fold(
                    (0u64, 0u64, 0u64, 0u64),
                    |(raw, compressed, stored, logical), page| {
                        let (raw_delta, compressed_delta) = match page.compression {
                            format::PageCompression::None => (1, 0),
                            format::PageCompression::Lz4Block => (0, 1),
                        };
                        Ok::<_, Error>((
                            raw.checked_add(raw_delta).ok_or_else(|| {
                                Error::InvalidSegment("segment-open raw page count overflow".into())
                            })?,
                            compressed.checked_add(compressed_delta).ok_or_else(|| {
                                Error::InvalidSegment(
                                    "segment-open compressed page count overflow".into(),
                                )
                            })?,
                            stored
                                .checked_add(u64::try_from(page.physical_bytes).map_err(|_| {
                                    Error::InvalidSegment(
                                        "segment-open stored page bytes exceed u64".into(),
                                    )
                                })?)
                                .ok_or_else(|| {
                                    Error::InvalidSegment(
                                        "segment-open stored page bytes overflow".into(),
                                    )
                                })?,
                            logical
                                .checked_add(u64::try_from(page.logical_bytes).map_err(|_| {
                                    Error::InvalidSegment(
                                        "segment-open logical page bytes exceed u64".into(),
                                    )
                                })?)
                                .ok_or_else(|| {
                                    Error::InvalidSegment(
                                        "segment-open logical page bytes overflow".into(),
                                    )
                                })?,
                        ))
                    },
                )?;
        Ok(Self {
            contract_version: SEGMENT_OPEN_EVIDENCE_VERSION,
            segment_count: 1,
            physical_bytes,
            format_probe_operations: 1,
            format_probe_bytes: 10,
            full_checksum_operations: 1,
            full_checksum_bytes: physical_bytes,
            metadata_operations: 1,
            metadata_bytes,
            persisted_filter_count,
            persisted_filter_bytes,
            none_policy_segment_count,
            adaptive_lz4_policy_segment_count,
            raw_page_count,
            compressed_page_count,
            stored_page_bytes,
            logical_page_bytes,
            semantic_page_operations,
            semantic_page_bytes,
        })
    }

    pub(crate) fn merge(&mut self, other: &Self) -> Result<()> {
        if other.contract_version != SEGMENT_OPEN_EVIDENCE_VERSION
            || self.contract_version != SEGMENT_OPEN_EVIDENCE_VERSION
        {
            return Err(Error::InvalidSegment(
                "segment-open evidence version is unsupported".into(),
            ));
        }
        self.contract_version = SEGMENT_OPEN_EVIDENCE_VERSION;
        for (field, value, name) in [
            (
                &mut self.segment_count,
                other.segment_count,
                "segment count",
            ),
            (
                &mut self.physical_bytes,
                other.physical_bytes,
                "physical bytes",
            ),
            (
                &mut self.format_probe_operations,
                other.format_probe_operations,
                "format-probe operations",
            ),
            (
                &mut self.format_probe_bytes,
                other.format_probe_bytes,
                "format-probe bytes",
            ),
            (
                &mut self.full_checksum_operations,
                other.full_checksum_operations,
                "checksum operations",
            ),
            (
                &mut self.full_checksum_bytes,
                other.full_checksum_bytes,
                "checksum bytes",
            ),
            (
                &mut self.metadata_operations,
                other.metadata_operations,
                "metadata operations",
            ),
            (
                &mut self.metadata_bytes,
                other.metadata_bytes,
                "metadata bytes",
            ),
            (
                &mut self.persisted_filter_count,
                other.persisted_filter_count,
                "persisted filter count",
            ),
            (
                &mut self.persisted_filter_bytes,
                other.persisted_filter_bytes,
                "persisted filter bytes",
            ),
            (
                &mut self.none_policy_segment_count,
                other.none_policy_segment_count,
                "none-policy segment count",
            ),
            (
                &mut self.adaptive_lz4_policy_segment_count,
                other.adaptive_lz4_policy_segment_count,
                "adaptive-LZ4 segment count",
            ),
            (
                &mut self.raw_page_count,
                other.raw_page_count,
                "raw page count",
            ),
            (
                &mut self.compressed_page_count,
                other.compressed_page_count,
                "compressed page count",
            ),
            (
                &mut self.stored_page_bytes,
                other.stored_page_bytes,
                "stored page bytes",
            ),
            (
                &mut self.logical_page_bytes,
                other.logical_page_bytes,
                "logical page bytes",
            ),
            (
                &mut self.semantic_page_operations,
                other.semantic_page_operations,
                "semantic-page operations",
            ),
            (
                &mut self.semantic_page_bytes,
                other.semantic_page_bytes,
                "semantic-page bytes",
            ),
        ] {
            *field = field
                .checked_add(value)
                .ok_or_else(|| Error::InvalidSegment(format!("segment-open {name} overflow")))?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct PageCache {
    policy: PageCachePolicy,
    capacity_bytes: usize,
    protected_capacity_bytes: usize,
    resident_bytes: usize,
    probationary_resident_bytes: usize,
    protected_resident_bytes: usize,
    values: HashMap<(u64, usize), CacheEntry>,
    exact_order: BinaryHeap<Reverse<(u64, (u64, usize))>>,
    probationary_order: BinaryHeap<Reverse<(u64, (u64, usize))>>,
    protected_order: BinaryHeap<Reverse<(u64, (u64, usize))>>,
    clock: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
    admissions: u64,
    admission_rejections: u64,
    promotions: u64,
    demotions: u64,
    same_scope_hits: u64,
    duplicate_loads: u64,
    loads: u64,
    bytes_read: u64,
    bytes_decoded: u64,
    bytes_borrowed: u64,
    bytes_allocated: u64,
    bytes_copied: u64,
    bytes_decompressed: u64,
    filter_checks: u64,
    filter_negatives: u64,
}

#[derive(Debug)]
struct CacheEntry {
    value: Arc<LoadedPage>,
    last_used: u64,
    region: CacheRegion,
    last_access: PageCacheAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheRegion {
    Exact,
    Probationary,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Engine-owned logical access provenance used only by the process-local page
/// replacement policy. It is neither serialized nor accepted from callers.
pub(crate) enum PageCacheAccess {
    OrdinaryOperation,
    ProjectedScope(u64),
}

impl PageCacheAccess {
    fn suppresses_promotion_from(self, previous: Self) -> bool {
        matches!(
            (previous, self),
            (Self::ProjectedScope(previous), Self::ProjectedScope(current))
                if previous == current
        )
    }
}

pub(crate) fn next_projected_cache_access() -> Result<PageCacheAccess> {
    CACHE_ACCESS_SCOPE_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map(PageCacheAccess::ProjectedScope)
        .map_err(|_| Error::InvalidProjectedRead("page-cache reuse scope exhausted".into()))
}

pub(crate) type SharedPageCache = Arc<Mutex<PageCache>>;

pub(crate) fn new_page_cache(capacity_bytes: usize) -> SharedPageCache {
    new_page_cache_with_policy(capacity_bytes, PageCachePolicy::default())
        .expect("default page-cache policy is valid")
}

pub(crate) fn new_page_cache_with_policy(
    capacity_bytes: usize,
    policy: PageCachePolicy,
) -> Result<SharedPageCache> {
    let protected_capacity_bytes = policy.protected_capacity_bytes(capacity_bytes)?;
    Ok(Arc::new(Mutex::new(PageCache {
        policy,
        capacity_bytes,
        protected_capacity_bytes,
        resident_bytes: 0,
        probationary_resident_bytes: 0,
        protected_resident_bytes: 0,
        values: HashMap::new(),
        exact_order: BinaryHeap::new(),
        probationary_order: BinaryHeap::new(),
        protected_order: BinaryHeap::new(),
        clock: 0,
        hits: 0,
        misses: 0,
        evictions: 0,
        admissions: 0,
        admission_rejections: 0,
        promotions: 0,
        demotions: 0,
        same_scope_hits: 0,
        duplicate_loads: 0,
        loads: 0,
        bytes_read: 0,
        bytes_decoded: 0,
        bytes_borrowed: 0,
        bytes_allocated: 0,
        bytes_copied: 0,
        bytes_decompressed: 0,
        filter_checks: 0,
        filter_negatives: 0,
    })))
}

pub(crate) fn page_cache_stats(cache: &SharedPageCache) -> PageCacheStats {
    let cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let probationary_entries = cache
        .values
        .values()
        .filter(|entry| entry.region == CacheRegion::Probationary)
        .count();
    let protected_entries = cache
        .values
        .values()
        .filter(|entry| entry.region == CacheRegion::Protected)
        .count();
    PageCacheStats {
        policy: cache.policy,
        capacity_bytes: cache.capacity_bytes,
        resident_bytes: cache.resident_bytes,
        entries: cache.values.len(),
        probationary_resident_bytes: cache.probationary_resident_bytes,
        protected_resident_bytes: cache.protected_resident_bytes,
        probationary_entries,
        protected_entries,
        hits: cache.hits,
        misses: cache.misses,
        evictions: cache.evictions,
        admissions: cache.admissions,
        admission_rejections: cache.admission_rejections,
        promotions: cache.promotions,
        demotions: cache.demotions,
        same_scope_hits: cache.same_scope_hits,
        duplicate_loads: cache.duplicate_loads,
        loads: cache.loads,
        bytes_read: cache.bytes_read,
        bytes_decoded: cache.bytes_decoded,
        bytes_borrowed: cache.bytes_borrowed,
        bytes_allocated: cache.bytes_allocated,
        bytes_copied: cache.bytes_copied,
        bytes_decompressed: cache.bytes_decompressed,
        filter_checks: cache.filter_checks,
        filter_negatives: cache.filter_negatives,
    }
}

fn record_filter_probe(cache: &SharedPageCache, negative: bool) -> Result<()> {
    let mut cache = cache
        .lock()
        .map_err(|_| Error::InvalidSegment("immutable page cache lock poisoned".into()))?;
    cache.filter_checks = cache.filter_checks.saturating_add(1);
    if negative {
        cache.filter_negatives = cache.filter_negatives.saturating_add(1);
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum PageSource {
    File {
        file: Arc<File>,
        selected: SelectedIo,
        io: SharedIoContext,
    },
    Mapped {
        bytes: Arc<Mmap>,
        io: SharedIoContext,
    },
    Bytes(Arc<Vec<u8>>),
}

#[derive(Debug)]
struct LoadedPage {
    buffer: Buffer,
    borrowed: bool,
    allocated_bytes: usize,
    copied_bytes: usize,
    decompressed_bytes: usize,
    actual_io: Option<SelectedIo>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PageReadContext {
    raw_borrowed: bool,
    source_allocated_bytes: usize,
    raw_copied_bytes: usize,
    actual_io: Option<SelectedIo>,
}

impl LoadedPage {
    fn resident_bytes(&self) -> usize {
        self.buffer.capacity().max(1)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PageLoadEvidence {
    pub cache_hit: bool,
    pub cache_miss: bool,
    pub cache_load: bool,
    pub actual_io: Option<SelectedIo>,
    pub physical_bytes: u64,
    pub borrowed_bytes: u64,
    pub decoded_bytes: u64,
    pub decompressed_bytes: u64,
    pub allocated_bytes: u64,
    pub copied_bytes: u64,
}

pub struct Segment {
    pub descriptor: SegmentDescriptor,
    row_group_budget: SegmentRowGroupBudget,
    compression_policy: SegmentCompressionPolicy,
    source: PageSource,
    row_groups: Vec<RowGroupDescriptor>,
    cache: SharedPageCache,
    cache_id: u64,
    open_evidence: SegmentOpenEvidence,
}

impl fmt::Debug for Segment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Segment")
            .field("descriptor", &self.descriptor)
            .field("row_group_budget", &self.row_group_budget)
            .field("compression_policy", &self.compression_policy)
            .field("row_group_count", &self.row_group_count())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SegmentVersion {
    pub sequence: u64,
    pub value: Option<Box<[u8]>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SegmentRecord {
    pub key: Vec<u8>,
    pub version: VersionedValue,
}

#[derive(Debug)]
struct KeySpine {
    offsets: Arc<LoadedPage>,
    data: Arc<LoadedPage>,
    sequences: Arc<LoadedPage>,
}

struct LoadedValueColumns {
    validity: Arc<LoadedPage>,
    offsets: Arc<LoadedPage>,
    values: Arc<LoadedPage>,
}

struct LoadedRowGroup {
    spine: KeySpine,
    validity: Arc<LoadedPage>,
    value_offsets: Arc<LoadedPage>,
    values: Arc<LoadedPage>,
}

pub(crate) struct SegmentRecordCursor<'a> {
    segment: &'a Segment,
    row_group_index: usize,
    row_index: usize,
    loaded: Option<LoadedRowGroup>,
}

impl Segment {
    pub fn write_from_memtable(directory: &Path, table: &Memtable) -> Result<(Self, PathBuf)> {
        Self::write_from_memtable_with_policy(directory, table, DEFAULT_SEGMENT_COMPRESSION_POLICY)
    }

    pub fn write_from_memtable_with_policy(
        directory: &Path,
        table: &Memtable,
        compression_policy: SegmentCompressionPolicy,
    ) -> Result<(Self, PathBuf)> {
        let directory = AnchoredPath::from_path(directory)?;
        let (segment, path) = Self::write_from_memtable_with_cache(
            &directory,
            table,
            SegmentRowGroupBudget::default(),
            compression_policy,
            new_page_cache(DEFAULT_PAGE_CACHE_BYTES),
            IoContext::new(SegmentIoPolicy::default())?,
        )?;
        Ok((segment, path.diagnostic_path().to_owned()))
    }

    pub(crate) fn write_from_memtable_with_cache(
        directory: &AnchoredPath,
        table: &Memtable,
        row_group_budget: SegmentRowGroupBudget,
        compression_policy: SegmentCompressionPolicy,
        cache: SharedPageCache,
        io: SharedIoContext,
    ) -> Result<(Self, AnchoredPath)> {
        let encoded = encode(table, row_group_budget, compression_policy)?;
        let write_evidence = encoded.evidence;
        let path = directory.join(format!("{}.seg", encoded.checksum));
        directory.create_dir_all()?;
        if path.try_exists()? {
            let segment = Self::open_with_cache_and_io(&path, cache, io)?;
            if segment.descriptor.id != encoded.checksum {
                return invalid("existing content-addressed segment has another identity");
            }
            // A previous rename can have taken effect despite a failed parent
            // barrier. Reuse is publication too: stabilize the existing name.
            directory.sync_directory()?;
            tracing::debug!(
                target: "rrd_lsm::segment_write",
                compression_policy = compression_policy.kind(),
                compression_minimum_savings_basis_points =
                    compression_policy.minimum_savings_basis_points(),
                compression_maximum_page_logical_bytes =
                    compression_policy.maximum_page_logical_bytes(),
                raw_page_count = write_evidence.raw_page_count,
                compressed_page_count = write_evidence.compressed_page_count,
                stored_page_bytes = write_evidence.stored_page_bytes,
                logical_page_bytes = write_evidence.logical_page_bytes,
                compression_scratch_high_water_bytes =
                    write_evidence.compression_scratch_high_water_bytes,
                deduplicated = true,
                "immutable v6 segment output resolved"
            );
            return Ok((segment, path));
        }
        let temporary = directory.join(format!(
            ".{}.{}.{}.tmp",
            encoded.checksum,
            std::process::id(),
            TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        temporary.admit_disk(encoded.bytes.len() as u64, true)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        if let Err(error) = (|| -> std::io::Result<()> {
            crate::filesystem::file_io::write_all(
                &mut file,
                temporary.diagnostic_path(),
                &encoded.bytes,
            )?;
            crate::filesystem::file_io::sync_all(&file, temporary.diagnostic_path())?;
            temporary.rename_to(&path)
        })() {
            let _ = temporary.remove_file();
            return Err(Error::Io(error));
        }
        let segment = Self::open_with_cache_and_io(&path, cache, io)?;
        tracing::debug!(
            target: "rrd_lsm::segment_write",
            compression_policy = compression_policy.kind(),
            compression_minimum_savings_basis_points =
                compression_policy.minimum_savings_basis_points(),
            compression_maximum_page_logical_bytes =
                compression_policy.maximum_page_logical_bytes(),
            raw_page_count = write_evidence.raw_page_count,
            compressed_page_count = write_evidence.compressed_page_count,
            stored_page_bytes = write_evidence.stored_page_bytes,
            logical_page_bytes = write_evidence.logical_page_bytes,
            compression_scratch_high_water_bytes =
                write_evidence.compression_scratch_high_water_bytes,
            deduplicated = false,
            "immutable v6 segment output published"
        );
        Ok((segment, path))
    }

    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with_cache_and_io(
            &AnchoredPath::from_path(path)?,
            new_page_cache(DEFAULT_PAGE_CACHE_BYTES),
            IoContext::new(SegmentIoPolicy::default())?,
        )
    }

    pub(crate) fn open_with_cache_and_io(
        path: &AnchoredPath,
        cache: SharedPageCache,
        io: SharedIoContext,
    ) -> Result<Self> {
        let mut file = path.open_read()?;
        let physical_bytes = validate_physical_file(&mut file)?;
        let total_started = Instant::now();
        let verify_started = Instant::now();
        let checksum = verify_file_digest(&mut file, physical_bytes)?;
        let verify_ms = verify_started.elapsed().as_millis() as u64;
        let metadata_started = Instant::now();
        let metadata = parse_metadata(&mut file, physical_bytes, checksum)?;
        let metadata_ms = metadata_started.elapsed().as_millis() as u64;
        let open_evidence = SegmentOpenEvidence::for_metadata(physical_bytes, &metadata, true)?;
        let source_started = Instant::now();
        let source = open_page_source(file, io)?;
        let source_ms = source_started.elapsed().as_millis() as u64;
        let row_group_count = metadata.row_groups.len();
        let row_group_budget = metadata.row_group_budget;
        let segment = build_segment(metadata, source, cache, open_evidence)?;
        segment.validate_all_pages()?;
        tracing::debug!(
            target: "rrd_lsm::open",
            path = %path.display(),
            physical_bytes,
            row_group_count,
            row_group_target_bytes = row_group_budget.target_bytes,
            row_group_max_rows = row_group_budget.max_rows,
            compression_policy = segment.compression_policy.kind(),
            compression_minimum_savings_basis_points = segment
                .compression_policy
                .minimum_savings_basis_points(),
            compression_maximum_page_logical_bytes = segment
                .compression_policy
                .maximum_page_logical_bytes(),
            persisted_filter_count = segment.open_evidence.persisted_filter_count,
            persisted_filter_bytes = segment.open_evidence.persisted_filter_bytes,
            raw_page_count = segment.open_evidence.raw_page_count,
            compressed_page_count = segment.open_evidence.compressed_page_count,
            stored_page_bytes = segment.open_evidence.stored_page_bytes,
            logical_page_bytes = segment.open_evidence.logical_page_bytes,
            verify_ms,
            metadata_ms,
            source_ms,
            total_ms = total_started.elapsed().as_millis() as u64,
            "immutable v6 columnar segment open phases completed"
        );
        Ok(segment)
    }

    pub(crate) fn open_expected_with_cache_and_io(
        path: &AnchoredPath,
        expected: &SegmentDescriptor,
        cache: SharedPageCache,
        io: SharedIoContext,
    ) -> Result<Self> {
        let mut file = path.open_read()?;
        let physical_bytes = validate_physical_file(&mut file)?;
        if physical_bytes != expected.bytes {
            return invalid("segment physical size differs from its manifest descriptor");
        }
        let checksum = verify_file_digest(&mut file, physical_bytes)?;
        if checksum != expected.checksum || checksum != expected.id {
            return invalid("segment content digest differs from its manifest descriptor");
        }
        let metadata = parse_metadata(&mut file, physical_bytes, checksum)?;
        let open_evidence = SegmentOpenEvidence::for_metadata(physical_bytes, &metadata, false)?;
        let source = open_page_source(file, io)?;
        let mut segment = build_segment(metadata, source, cache, open_evidence)?;
        segment.descriptor.level = expected.level;
        if &segment.descriptor != expected {
            return invalid("v6 segment differs from its manifest descriptor");
        }
        Ok(segment)
    }

    pub(crate) fn validate_snapshot_bytes(
        expected: &SegmentDescriptor,
        bytes: &[u8],
    ) -> Result<Self> {
        Self::validate_snapshot_owned(expected, bytes.to_vec())
    }

    pub(crate) fn validate_snapshot_owned(
        expected: &SegmentDescriptor,
        bytes: Vec<u8>,
    ) -> Result<Self> {
        if bytes.len() as u64 > MAX_SEGMENT_BYTES {
            return invalid("snapshot segment exceeds the 1 GiB physical safety limit");
        }
        require_current_format(&bytes)?;
        let content_end = bytes
            .len()
            .checked_sub(FOOTER_BYTES)
            .ok_or_else(|| Error::InvalidSegment("v6 snapshot footer underflow".into()))?;
        let footer = std::str::from_utf8(&bytes[content_end..])
            .map_err(|_| Error::InvalidSegment("segment footer is not ASCII".into()))?;
        let checksum = sha256_hex(&bytes[..content_end]);
        if footer != checksum {
            return invalid("segment content checksum does not match");
        }
        let mut cursor = std::io::Cursor::new(&bytes);
        let metadata = parse_metadata(&mut cursor, bytes.len() as u64, checksum)?;
        let mut segment = build_segment(
            metadata,
            PageSource::Bytes(Arc::new(bytes)),
            new_page_cache(DEFAULT_PAGE_CACHE_BYTES),
            SegmentOpenEvidence::default(),
        )?;
        segment.validate_all_pages()?;
        segment.descriptor.level = expected.level;
        if &segment.descriptor != expected {
            return invalid(format!(
                "snapshot segment {} differs from its descriptor",
                expected.id
            ));
        }
        Ok(segment)
    }

    pub(crate) fn install_snapshot_bytes_with_cache(
        directory: &AnchoredPath,
        expected: &SegmentDescriptor,
        bytes: &[u8],
        cache: SharedPageCache,
        io: SharedIoContext,
    ) -> Result<Self> {
        Self::validate_snapshot_bytes(expected, bytes)?;
        directory.create_dir_all()?;
        let path = directory.join(format!("{}.seg", expected.id));
        if path.try_exists()? {
            let existing = Self::open_expected_with_cache_and_io(&path, expected, cache, io)?;
            if !file_equals_bytes(&path, bytes)? {
                return invalid(format!(
                    "existing snapshot segment {} has different bytes",
                    expected.id
                ));
            }
            directory.sync_directory()?;
            return Ok(existing);
        }
        let temporary = directory.join(format!(
            ".{}.{}.{}.snapshot.tmp",
            expected.id,
            std::process::id(),
            TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        temporary.admit_disk(bytes.len() as u64, true)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        if let Err(error) = (|| -> std::io::Result<()> {
            crate::filesystem::file_io::write_all(&mut file, temporary.diagnostic_path(), bytes)?;
            crate::filesystem::file_io::sync_all(&file, temporary.diagnostic_path())?;
            temporary.rename_to(&path)
        })() {
            let _ = temporary.remove_file();
            return Err(Error::Io(error));
        }
        Self::open_expected_with_cache_and_io(&path, expected, cache, io)
    }

    pub fn get(&self, key: &[u8], read_sequence: u64) -> Result<Option<Vec<u8>>> {
        Ok(self
            .get_version(key, read_sequence)?
            .and_then(|version| version.value.map(|value| value.into_vec())))
    }

    pub(crate) fn get_version(
        &self,
        key: &[u8],
        read_sequence: u64,
    ) -> Result<Option<SegmentVersion>> {
        if key < self.descriptor.first_key.as_slice()
            || key > self.descriptor.last_key.as_slice()
            || read_sequence < self.descriptor.minimum_sequence
        {
            return Ok(None);
        }
        let index = self
            .row_groups
            .partition_point(|group| group.last_key.as_slice() < key);
        let Some(group) = self.row_groups.get(index) else {
            return Ok(None);
        };
        if key < group.first_key.as_slice() {
            return Ok(None);
        }
        let negative = !group.filter.may_contain(key);
        record_filter_probe(&self.cache, negative)?;
        if negative {
            return Ok(None);
        }
        let spine = self.load_key_spine(index)?;
        let mut row = lower_bound(&spine, group.row_count as usize, key)?;
        let mut selected = None;
        while row < group.row_count as usize {
            let actual = key_at(&spine, row)?;
            if actual != key {
                break;
            }
            let sequence = sequence_at(&spine, row)?;
            if sequence <= read_sequence {
                selected = Some((row, sequence));
            }
            row += 1;
        }
        let Some((row, sequence)) = selected else {
            return Ok(None);
        };
        Ok(Some(SegmentVersion {
            sequence,
            value: self.value_at(index, row)?.map(Box::<[u8]>::from),
        }))
    }

    pub(crate) fn get_versions(
        &self,
        keys: &[&[u8]],
        read_sequence: u64,
    ) -> Result<Vec<Option<SegmentVersion>>> {
        keys.iter()
            .map(|key| self.get_version(key, read_sequence))
            .collect()
    }

    pub fn scan(
        &self,
        start: &[u8],
        end: Option<&[u8]>,
        read_sequence: u64,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .visible_from(start, end, read_sequence)?
            .into_iter()
            .filter_map(|(key, version)| version.value.map(|value| (key, value.into_vec())))
            .collect())
    }

    pub fn visible_versions(&self, read_sequence: u64) -> Result<Vec<(Vec<u8>, VersionedValue)>> {
        self.visible_from(&[], None, read_sequence)
    }

    /// Sum authenticated decoded page lengths without loading page contents.
    /// This includes key spines, offsets, sequence columns and value payloads.
    pub(crate) fn compaction_logical_bytes(&self) -> Result<u64> {
        self.row_groups
            .iter()
            .flat_map(|group| &group.pages)
            .try_fold(0u64, |total, page| {
                total.checked_add(page.logical_bytes as u64).ok_or_else(|| {
                    Error::InvalidSegment("compaction decoded input overflow".into())
                })
            })
    }

    pub(crate) fn record_cursor(&self) -> SegmentRecordCursor<'_> {
        SegmentRecordCursor {
            segment: self,
            row_group_index: 0,
            row_index: 0,
            loaded: None,
        }
    }

    pub fn row_group_count(&self) -> usize {
        self.row_groups.len()
    }

    pub fn open_evidence(&self) -> &SegmentOpenEvidence {
        &self.open_evidence
    }

    /// Returns the authenticated writer budget carried by this immutable
    /// segment. Historical segments remain self-describing when the configured
    /// budget for future flushes changes.
    pub fn row_group_budget(&self) -> SegmentRowGroupBudget {
        self.row_group_budget
    }

    /// Returns the authenticated physical writer policy carried by this
    /// immutable segment. It describes durable bytes, not current process
    /// configuration for future outputs.
    pub fn compression_policy(&self) -> SegmentCompressionPolicy {
        self.compression_policy
    }

    pub(crate) fn visible_from(
        &self,
        start: &[u8],
        end: Option<&[u8]>,
        read_sequence: u64,
    ) -> Result<Vec<(Vec<u8>, VersionedValue)>> {
        let mut grouped = BTreeMap::<Vec<u8>, SegmentVersion>::new();
        let first = self
            .row_groups
            .partition_point(|group| group.last_key.as_slice() < start);
        for index in first..self.row_groups.len() {
            let descriptor = &self.row_groups[index];
            if end.is_some_and(|end| descriptor.first_key.as_slice() >= end) {
                break;
            }
            let spine = self.load_key_spine(index)?;
            let mut values = None;
            for row in 0..descriptor.row_count as usize {
                let key = key_at(&spine, row)?;
                if key < start || end.is_some_and(|end| key >= end) {
                    continue;
                }
                let sequence = sequence_at(&spine, row)?;
                if sequence <= read_sequence {
                    let value = match values.as_ref() {
                        Some(value) => value,
                        None => values.insert(self.load_value_columns(index)?),
                    };
                    let version = SegmentVersion {
                        sequence,
                        value: value_from_columns(value, row)?.map(Box::<[u8]>::from),
                    };
                    if grouped
                        .get(key)
                        .is_none_or(|prior| sequence > prior.sequence)
                    {
                        grouped.insert(key.to_vec(), version);
                    }
                }
            }
        }
        Ok(grouped
            .into_iter()
            .map(|(key, version)| {
                (
                    key,
                    VersionedValue {
                        sequence: version.sequence,
                        value: version.value,
                    },
                )
            })
            .collect())
    }

    pub(crate) fn visible_ranges(
        &self,
        ranges: &[(Vec<u8>, Vec<u8>)],
        read_sequence: u64,
    ) -> Result<Vec<(Vec<u8>, VersionedValue)>> {
        if ranges.is_empty() || read_sequence < self.descriptor.minimum_sequence {
            return Ok(Vec::new());
        }
        let mut grouped = BTreeMap::<Vec<u8>, SegmentVersion>::new();
        let mut selected_groups = BTreeSet::new();
        for (start, end) in ranges {
            let first = self
                .row_groups
                .partition_point(|group| group.last_key.as_slice() < start.as_slice());
            for (index, group) in self.row_groups.iter().enumerate().skip(first) {
                if group.first_key.as_slice() >= end.as_slice() {
                    break;
                }
                selected_groups.insert(index);
            }
        }
        for index in selected_groups {
            let descriptor = &self.row_groups[index];
            let spine = self.load_key_spine(index)?;
            let mut values = None;
            for row in 0..descriptor.row_count as usize {
                let key = key_at(&spine, row)?;
                let after_start = ranges.partition_point(|(start, _)| start.as_slice() <= key);
                let Some(range_index) = after_start.checked_sub(1) else {
                    continue;
                };
                if key >= ranges[range_index].1.as_slice() {
                    continue;
                }
                let sequence = sequence_at(&spine, row)?;
                if sequence <= read_sequence {
                    let value = match values.as_ref() {
                        Some(value) => value,
                        None => values.insert(self.load_value_columns(index)?),
                    };
                    let version = SegmentVersion {
                        sequence,
                        value: value_from_columns(value, row)?.map(Box::<[u8]>::from),
                    };
                    if grouped
                        .get(key)
                        .is_none_or(|prior| sequence > prior.sequence)
                    {
                        grouped.insert(key.to_vec(), version);
                    }
                }
            }
        }
        Ok(grouped
            .into_iter()
            .map(|(key, version)| {
                (
                    key,
                    VersionedValue {
                        sequence: version.sequence,
                        value: version.value,
                    },
                )
            })
            .collect())
    }

    fn load_key_spine(&self, row_group: usize) -> Result<KeySpine> {
        Ok(KeySpine {
            offsets: self.load_page(row_group, PageKind::KeyOffsets)?,
            data: self.load_page(row_group, PageKind::KeyData)?,
            sequences: self.load_page(row_group, PageKind::SequenceValues)?,
        })
    }

    fn load_row_group(&self, row_group: usize) -> Result<LoadedRowGroup> {
        Ok(LoadedRowGroup {
            spine: self.load_key_spine(row_group)?,
            validity: self.load_page(row_group, PageKind::ValueValidity)?,
            value_offsets: self.load_page(row_group, PageKind::ValueOffsets)?,
            values: self.load_page(row_group, PageKind::ValueData)?,
        })
    }

    fn read_row_group_uncached(&self, row_group: usize) -> Result<LoadedRowGroup> {
        let descriptor = self.row_groups.get(row_group).ok_or_else(|| {
            Error::InvalidSegment("row-group index is outside the segment".into())
        })?;
        Ok(LoadedRowGroup {
            spine: KeySpine {
                offsets: Arc::new(read_page(
                    &self.source,
                    descriptor.page(PageKind::KeyOffsets),
                )?),
                data: Arc::new(read_page(&self.source, descriptor.page(PageKind::KeyData))?),
                sequences: Arc::new(read_page(
                    &self.source,
                    descriptor.page(PageKind::SequenceValues),
                )?),
            },
            validity: Arc::new(read_page(
                &self.source,
                descriptor.page(PageKind::ValueValidity),
            )?),
            value_offsets: Arc::new(read_page(
                &self.source,
                descriptor.page(PageKind::ValueOffsets),
            )?),
            values: Arc::new(read_page(
                &self.source,
                descriptor.page(PageKind::ValueData),
            )?),
        })
    }

    fn value_at(&self, row_group: usize, row: usize) -> Result<Option<Vec<u8>>> {
        let validity = self.load_page(row_group, PageKind::ValueValidity)?;
        if !valid_at(validity.buffer.as_slice(), row)? {
            return Ok(None);
        }
        let offsets = self.load_page(row_group, PageKind::ValueOffsets)?;
        let values = self.load_page(row_group, PageKind::ValueData)?;
        Ok(Some(
            binary_at(offsets.buffer.as_slice(), values.buffer.as_slice(), row)?.to_vec(),
        ))
    }

    fn load_page(&self, row_group: usize, kind: PageKind) -> Result<Arc<LoadedPage>> {
        self.load_page_with_evidence(row_group, kind)
            .map(|(page, _)| page)
    }

    fn load_page_with_evidence(
        &self,
        row_group: usize,
        kind: PageKind,
    ) -> Result<(Arc<LoadedPage>, PageLoadEvidence)> {
        self.load_page_with_access(row_group, kind, PageCacheAccess::OrdinaryOperation)
    }

    fn load_page_with_access(
        &self,
        row_group: usize,
        kind: PageKind,
        access: PageCacheAccess,
    ) -> Result<(Arc<LoadedPage>, PageLoadEvidence)> {
        let ordinal = row_group
            .checked_mul(format::PAGES_PER_ROW_GROUP)
            .and_then(|value| value.checked_add(kind as usize - 1))
            .ok_or_else(|| Error::InvalidSegment("v6 page cache ordinal overflow".into()))?;
        let key = (self.cache_id, ordinal);
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| Error::InvalidSegment("immutable page cache lock poisoned".into()))?;
            if let Some(value) = cache.lookup(key, access)? {
                return Ok((
                    Arc::clone(&value),
                    PageLoadEvidence {
                        cache_hit: true,
                        actual_io: None,
                        borrowed_bytes: if value.borrowed {
                            value.buffer.len() as u64
                        } else {
                            0
                        },
                        ..PageLoadEvidence::default()
                    },
                ));
            }
        }
        let descriptor = self
            .row_groups
            .get(row_group)
            .ok_or_else(|| Error::InvalidSegment("row-group index is outside the segment".into()))?
            .page(kind);
        let loaded = Arc::new(read_page(&self.source, descriptor)?);
        let evidence = PageLoadEvidence {
            cache_miss: true,
            cache_load: true,
            actual_io: loaded.actual_io,
            physical_bytes: descriptor.physical_bytes as u64,
            borrowed_bytes: if loaded.borrowed {
                descriptor.logical_bytes as u64
            } else {
                0
            },
            decoded_bytes: descriptor.logical_bytes as u64,
            decompressed_bytes: loaded.decompressed_bytes as u64,
            allocated_bytes: loaded.allocated_bytes as u64,
            copied_bytes: loaded.copied_bytes as u64,
            ..PageLoadEvidence::default()
        };
        let resident_bytes = loaded.resident_bytes();
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| Error::InvalidSegment("immutable page cache lock poisoned".into()))?;
        cache.loads = cache.loads.saturating_add(1);
        cache.bytes_read = cache
            .bytes_read
            .saturating_add(descriptor.physical_bytes as u64);
        cache.bytes_decoded = cache
            .bytes_decoded
            .saturating_add(descriptor.logical_bytes as u64);
        cache.bytes_allocated = cache
            .bytes_allocated
            .saturating_add(loaded.allocated_bytes as u64);
        cache.bytes_copied = cache
            .bytes_copied
            .saturating_add(loaded.copied_bytes as u64);
        cache.bytes_decompressed = cache
            .bytes_decompressed
            .checked_add(loaded.decompressed_bytes as u64)
            .ok_or_else(|| {
                Error::InvalidSegment("page-cache decompression evidence overflow".into())
            })?;
        if loaded.borrowed {
            cache.bytes_borrowed = cache
                .bytes_borrowed
                .saturating_add(descriptor.logical_bytes as u64);
        }
        let value = cache.complete_load(key, loaded, resident_bytes, access)?;
        Ok((value, evidence))
    }

    fn load_value_columns(&self, row_group: usize) -> Result<LoadedValueColumns> {
        let _descriptor = self.row_groups.get(row_group).ok_or_else(|| {
            Error::InvalidSegment("row-group index is outside the segment".into())
        })?;
        Ok(LoadedValueColumns {
            validity: self.load_page(row_group, PageKind::ValueValidity)?,
            offsets: self.load_page(row_group, PageKind::ValueOffsets)?,
            values: self.load_page(row_group, PageKind::ValueData)?,
        })
    }

    fn validate_all_pages(&self) -> Result<()> {
        let mut previous: Option<(Vec<u8>, u64)> = None;
        let mut observed_rows = 0u64;
        let mut observed_minimum = u64::MAX;
        let mut observed_maximum = 0u64;
        for (index, descriptor) in self.row_groups.iter().enumerate() {
            let group = self.read_row_group_uncached(index)?;
            validate_offsets(
                group.spine.offsets.buffer.as_slice(),
                group.spine.data.buffer.as_slice(),
                descriptor.row_count as usize,
                MAX_KEY_BYTES,
                false,
            )?;
            validate_offsets(
                group.value_offsets.buffer.as_slice(),
                group.values.buffer.as_slice(),
                descriptor.row_count as usize,
                MAX_VALUE_BYTES,
                true,
            )?;
            validate_validity(
                group.validity.buffer.as_slice(),
                descriptor.row_count as usize,
            )?;
            let mut filter = RowGroupFilter::new(descriptor.unique_key_count)?;
            let mut unique_key_count = 0u32;
            let mut previous_group_key: Option<Vec<u8>> = None;
            let mut actual_first = None;
            let mut actual_last = None;
            let mut null_count = 0u32;
            let mut key_min = u64::MAX;
            let mut key_max = 0u64;
            let mut value_min = u64::MAX;
            let mut value_max = 0u64;
            let mut sequence_min = u64::MAX;
            let mut sequence_max = 0u64;
            for row in 0..descriptor.row_count as usize {
                let key = key_at(&group.spine, row)?;
                let sequence = sequence_at(&group.spine, row)?;
                if sequence == 0
                    || sequence < self.descriptor.minimum_sequence
                    || sequence > self.descriptor.maximum_sequence
                    || previous
                        .as_ref()
                        .is_some_and(|(prior_key, prior_sequence)| {
                            key < prior_key.as_slice()
                                || (key == prior_key.as_slice() && sequence <= *prior_sequence)
                        })
                {
                    return invalid("v6 spine is not in canonical key/sequence order");
                }
                previous = Some((key.to_vec(), sequence));
                if previous_group_key.as_deref() != Some(key) {
                    filter.insert(key);
                    unique_key_count = unique_key_count.checked_add(1).ok_or_else(|| {
                        Error::InvalidSegment("v6 unique-key validation count overflow".into())
                    })?;
                    previous_group_key = Some(key.to_vec());
                }
                actual_first.get_or_insert_with(|| key.to_vec());
                actual_last = Some(key.to_vec());
                key_min = key_min.min(key.len() as u64);
                key_max = key_max.max(key.len() as u64);
                sequence_min = sequence_min.min(sequence);
                sequence_max = sequence_max.max(sequence);
                observed_minimum = observed_minimum.min(sequence);
                observed_maximum = observed_maximum.max(sequence);
                match value_from_loaded(&group, row)? {
                    Some(value) => {
                        value_min = value_min.min(value.len() as u64);
                        value_max = value_max.max(value.len() as u64);
                    }
                    None => {
                        null_count = null_count.saturating_add(1);
                        value_min = 0;
                    }
                }
            }
            if actual_first.as_deref() != Some(descriptor.first_key.as_slice())
                || actual_last.as_deref() != Some(descriptor.last_key.as_slice())
                || descriptor.page(PageKind::KeyOffsets).statistic_min != key_min
                || descriptor.page(PageKind::KeyOffsets).statistic_max != key_max
                || descriptor.page(PageKind::SequenceValues).statistic_min != sequence_min
                || descriptor.page(PageKind::SequenceValues).statistic_max != sequence_max
                || descriptor.page(PageKind::ValueValidity).null_count != null_count
                || descriptor.page(PageKind::ValueOffsets).statistic_min != value_min
                || descriptor.page(PageKind::ValueOffsets).statistic_max != value_max
            {
                return invalid("v6 page statistics or row-group bounds disagree with data");
            }
            if unique_key_count != descriptor.unique_key_count || filter != descriptor.filter {
                return invalid("v6 persisted row-group filter disagrees with decoded keys");
            }
            observed_rows = observed_rows
                .checked_add(u64::from(descriptor.row_count))
                .ok_or_else(|| Error::InvalidSegment("v6 observed row count overflow".into()))?;
        }
        if observed_rows != self.descriptor.entries
            || observed_minimum != self.descriptor.minimum_sequence
            || observed_maximum != self.descriptor.maximum_sequence
        {
            return invalid("v6 header counts or sequence range disagree with pages");
        }
        Ok(())
    }
}

impl SegmentRecordCursor<'_> {
    pub(crate) fn next_record(&mut self) -> Result<Option<SegmentRecord>> {
        loop {
            if self.row_group_index >= self.segment.row_groups.len() {
                return Ok(None);
            }
            if self.loaded.is_none() {
                self.loaded = Some(self.segment.load_row_group(self.row_group_index)?);
                self.row_index = 0;
            }
            let descriptor = &self.segment.row_groups[self.row_group_index];
            if self.row_index < descriptor.row_count as usize {
                let loaded = self.loaded.as_ref().expect("row group was loaded");
                let row = self.row_index;
                self.row_index += 1;
                return Ok(Some(SegmentRecord {
                    key: key_at(&loaded.spine, row)?.to_vec(),
                    version: VersionedValue {
                        sequence: sequence_at(&loaded.spine, row)?,
                        value: value_from_loaded(loaded, row)?.map(Box::<[u8]>::from),
                    },
                }));
            }
            self.row_group_index += 1;
            self.loaded = None;
        }
    }
}

impl PageCache {
    fn lookup(
        &mut self,
        key: (u64, usize),
        access: PageCacheAccess,
    ) -> Result<Option<Arc<LoadedPage>>> {
        let Some((region, previous_access)) = self
            .values
            .get(&key)
            .map(|entry| (entry.region, entry.last_access))
        else {
            self.misses = self.misses.saturating_add(1);
            return Ok(None);
        };

        self.hits = self.hits.saturating_add(1);
        match (self.policy, region) {
            (PageCachePolicy::ExactLru, CacheRegion::Exact)
            | (PageCachePolicy::ScanResistantLru { .. }, CacheRegion::Protected) => {
                self.refresh(key, region, access);
            }
            (PageCachePolicy::ScanResistantLru { .. }, CacheRegion::Probationary) => {
                if access.suppresses_promotion_from(previous_access) {
                    self.same_scope_hits = self.same_scope_hits.saturating_add(1);
                    self.refresh(key, region, access);
                } else {
                    self.promote(key, access)?;
                    self.demote_to_protected_target()?;
                }
            }
            _ => {
                return Err(Error::InvalidSegment(
                    "page-cache entry region conflicts with replacement policy".into(),
                ));
            }
        }
        self.maybe_rebuild_orders();
        self.validate_residency()?;
        Ok(Some(Arc::clone(
            &self
                .values
                .get(&key)
                .expect("cache hit remains resident")
                .value,
        )))
    }

    fn complete_load(
        &mut self,
        key: (u64, usize),
        loaded: Arc<LoadedPage>,
        resident_bytes: usize,
        access: PageCacheAccess,
    ) -> Result<Arc<LoadedPage>> {
        if let Some(entry) = self.values.get(&key) {
            let region = entry.region;
            let value = Arc::clone(&entry.value);
            self.duplicate_loads = self.duplicate_loads.saturating_add(1);
            self.refresh(key, region, access);
            self.maybe_rebuild_orders();
            self.validate_residency()?;
            return Ok(value);
        }
        if self.capacity_bytes == 0 || resident_bytes > self.capacity_bytes {
            self.admission_rejections = self.admission_rejections.saturating_add(1);
            return Ok(loaded);
        }

        while resident_bytes > self.capacity_bytes.saturating_sub(self.resident_bytes) {
            let removed = match self.policy {
                PageCachePolicy::ExactLru => self.evict_oldest(CacheRegion::Exact)?,
                PageCachePolicy::ScanResistantLru { .. } => {
                    if self.evict_oldest(CacheRegion::Probationary)? {
                        true
                    } else {
                        self.evict_oldest(CacheRegion::Protected)?
                    }
                }
            };
            if !removed {
                return Err(Error::InvalidSegment(
                    "page-cache capacity pressure found no resident eviction victim".into(),
                ));
            }
        }

        let region = match self.policy {
            PageCachePolicy::ExactLru => CacheRegion::Exact,
            PageCachePolicy::ScanResistantLru { .. } => CacheRegion::Probationary,
        };
        let stamp = self.next_stamp();
        self.resident_bytes = self
            .resident_bytes
            .checked_add(resident_bytes)
            .ok_or_else(|| Error::InvalidSegment("page-cache residency overflow".into()))?;
        self.add_region_bytes(region, resident_bytes)?;
        self.values.insert(
            key,
            CacheEntry {
                value: Arc::clone(&loaded),
                last_used: stamp,
                region,
                last_access: access,
            },
        );
        self.push_order(region, stamp, key);
        self.admissions = self.admissions.saturating_add(1);
        self.maybe_rebuild_orders();
        self.validate_residency()?;
        Ok(loaded)
    }

    fn next_stamp(&mut self) -> u64 {
        self.clock = self.clock.wrapping_add(1);
        if self.clock == 0 {
            self.renumber();
        }
        self.clock
    }

    fn refresh(&mut self, key: (u64, usize), region: CacheRegion, access: PageCacheAccess) {
        let stamp = self.next_stamp();
        let entry = self.values.get_mut(&key).expect("cache key was checked");
        debug_assert_eq!(entry.region, region);
        entry.last_used = stamp;
        entry.last_access = access;
        self.push_order(region, stamp, key);
    }

    fn promote(&mut self, key: (u64, usize), access: PageCacheAccess) -> Result<()> {
        let resident_bytes = self
            .values
            .get(&key)
            .filter(|entry| entry.region == CacheRegion::Probationary)
            .map(|entry| entry.value.resident_bytes())
            .ok_or_else(|| {
                Error::InvalidSegment("page-cache promotion source is not probationary".into())
            })?;
        self.subtract_region_bytes(CacheRegion::Probationary, resident_bytes)?;
        self.add_region_bytes(CacheRegion::Protected, resident_bytes)?;
        let stamp = self.next_stamp();
        let entry = self.values.get_mut(&key).expect("cache key was checked");
        entry.region = CacheRegion::Protected;
        entry.last_used = stamp;
        entry.last_access = access;
        self.push_order(CacheRegion::Protected, stamp, key);
        self.promotions = self.promotions.saturating_add(1);
        Ok(())
    }

    fn demote_to_protected_target(&mut self) -> Result<()> {
        while self.protected_resident_bytes > self.protected_capacity_bytes {
            let Some((key, stamp)) = self.pop_oldest_valid(CacheRegion::Protected) else {
                return Err(Error::InvalidSegment(
                    "page-cache protected bytes have no demotion victim".into(),
                ));
            };
            let resident_bytes = self
                .values
                .get(&key)
                .expect("valid cache victim remains resident")
                .value
                .resident_bytes();
            self.subtract_region_bytes(CacheRegion::Protected, resident_bytes)?;
            self.add_region_bytes(CacheRegion::Probationary, resident_bytes)?;
            self.values
                .get_mut(&key)
                .expect("valid cache victim remains resident")
                .region = CacheRegion::Probationary;
            self.probationary_order.push(Reverse((stamp, key)));
            self.demotions = self.demotions.saturating_add(1);
        }
        Ok(())
    }

    fn evict_oldest(&mut self, region: CacheRegion) -> Result<bool> {
        let Some((key, _)) = self.pop_oldest_valid(region) else {
            return Ok(false);
        };
        let removed = self
            .values
            .remove(&key)
            .expect("valid cache victim remains resident");
        let resident_bytes = removed.value.resident_bytes();
        self.resident_bytes = self
            .resident_bytes
            .checked_sub(resident_bytes)
            .ok_or_else(|| Error::InvalidSegment("page-cache residency underflow".into()))?;
        self.subtract_region_bytes(region, resident_bytes)?;
        self.evictions = self.evictions.saturating_add(1);
        Ok(true)
    }

    fn pop_oldest_valid(&mut self, region: CacheRegion) -> Option<((u64, usize), u64)> {
        loop {
            let candidate = match region {
                CacheRegion::Exact => self.exact_order.pop(),
                CacheRegion::Probationary => self.probationary_order.pop(),
                CacheRegion::Protected => self.protected_order.pop(),
            };
            let Reverse((stamp, key)) = candidate?;
            if self
                .values
                .get(&key)
                .is_some_and(|entry| entry.region == region && entry.last_used == stamp)
            {
                return Some((key, stamp));
            }
        }
    }

    fn push_order(&mut self, region: CacheRegion, stamp: u64, key: (u64, usize)) {
        let entry = Reverse((stamp, key));
        match region {
            CacheRegion::Exact => self.exact_order.push(entry),
            CacheRegion::Probationary => self.probationary_order.push(entry),
            CacheRegion::Protected => self.protected_order.push(entry),
        }
    }

    fn add_region_bytes(&mut self, region: CacheRegion, resident_bytes: usize) -> Result<()> {
        let field = match region {
            CacheRegion::Exact => return Ok(()),
            CacheRegion::Probationary => &mut self.probationary_resident_bytes,
            CacheRegion::Protected => &mut self.protected_resident_bytes,
        };
        *field = field
            .checked_add(resident_bytes)
            .ok_or_else(|| Error::InvalidSegment("page-cache region residency overflow".into()))?;
        Ok(())
    }

    fn subtract_region_bytes(&mut self, region: CacheRegion, resident_bytes: usize) -> Result<()> {
        let field = match region {
            CacheRegion::Exact => return Ok(()),
            CacheRegion::Probationary => &mut self.probationary_resident_bytes,
            CacheRegion::Protected => &mut self.protected_resident_bytes,
        };
        *field = field
            .checked_sub(resident_bytes)
            .ok_or_else(|| Error::InvalidSegment("page-cache region residency underflow".into()))?;
        Ok(())
    }

    fn validate_residency(&self) -> Result<()> {
        if self.resident_bytes > self.capacity_bytes {
            return Err(Error::InvalidSegment(
                "page-cache residency exceeds configured capacity".into(),
            ));
        }
        let classified = self
            .probationary_resident_bytes
            .checked_add(self.protected_resident_bytes)
            .ok_or_else(|| {
                Error::InvalidSegment("page-cache classified residency overflow".into())
            })?;
        match self.policy {
            PageCachePolicy::ExactLru if classified != 0 => Err(Error::InvalidSegment(
                "exact-LRU cache contains scan-resistant region bytes".into(),
            )),
            PageCachePolicy::ScanResistantLru { .. } if classified != self.resident_bytes => {
                Err(Error::InvalidSegment(
                    "scan-resistant page-cache region bytes disagree with total residency".into(),
                ))
            }
            _ => Ok(()),
        }
    }

    fn maybe_rebuild_orders(&mut self) {
        let order_entries = self
            .exact_order
            .len()
            .saturating_add(self.probationary_order.len())
            .saturating_add(self.protected_order.len());
        if order_entries > self.values.len().saturating_mul(2).max(32) {
            self.rebuild_orders();
        }
    }

    fn rebuild_orders(&mut self) {
        self.exact_order.clear();
        self.probationary_order.clear();
        self.protected_order.clear();
        let entries = self
            .values
            .iter()
            .map(|(key, entry)| (entry.region, entry.last_used, *key))
            .collect::<Vec<_>>();
        for (region, stamp, key) in entries {
            self.push_order(region, stamp, key);
        }
    }

    fn renumber(&mut self) {
        let mut ordered = self
            .values
            .iter()
            .map(|(key, entry)| (entry.last_used, *key))
            .collect::<Vec<_>>();
        ordered.sort_unstable();
        for (index, (_, key)) in ordered.into_iter().enumerate() {
            self.values
                .get_mut(&key)
                .expect("cache key exists")
                .last_used = u64::try_from(index + 1).expect("cache size fits u64");
        }
        self.clock = u64::try_from(self.values.len()).expect("cache size fits u64");
        self.rebuild_orders();
    }
}

fn build_segment(
    metadata: ParsedMetadata,
    source: PageSource,
    cache: SharedPageCache,
    open_evidence: SegmentOpenEvidence,
) -> Result<Segment> {
    let compression_policy = metadata.compression_policy;
    Ok(Segment {
        descriptor: metadata.descriptor,
        row_group_budget: metadata.row_group_budget,
        compression_policy,
        source,
        row_groups: metadata.row_groups,
        cache,
        cache_id: CACHE_ID.fetch_add(1, Ordering::Relaxed),
        open_evidence,
    })
}

fn validate_physical_file(file: &mut File) -> Result<u64> {
    let physical_bytes = file.metadata()?.len();
    if physical_bytes > MAX_SEGMENT_BYTES {
        return invalid("segment exceeds the 1 GiB physical safety limit");
    }
    file.seek(std::io::SeekFrom::Start(0))?;
    let mut prefix = [0; 10];
    file.read_exact(&mut prefix)
        .map_err(|_| Error::InvalidSegment("segment has no complete format header".into()))?;
    require_current_format(&prefix)?;
    if physical_bytes < (SEGMENT_HEADER_BYTES + INDEX_HEADER_BYTES + FOOTER_BYTES) as u64 {
        return invalid("segment is shorter than its framing");
    }
    Ok(physical_bytes)
}

fn open_page_source(file: File, io: SharedIoContext) -> Result<PageSource> {
    let selected = io.file_backend()?;
    if selected == SelectedIo::Mmap {
        let mapped = unsafe { MmapOptions::new().map(&file) };
        match mapped {
            Ok(bytes) => {
                io.record_segment(SelectedIo::Mmap);
                return Ok(PageSource::Mapped {
                    bytes: Arc::new(bytes),
                    io,
                });
            }
            Err(error) if io.policy().allow_fallback => {
                io.record_fallback(&format!("mmap failed: {error}"));
            }
            Err(error) => return Err(Error::Io(error)),
        }
    }
    let selected = if selected == SelectedIo::Mmap {
        SelectedIo::Bounded
    } else {
        selected
    };
    io.record_segment(selected);
    Ok(PageSource::File {
        file: Arc::new(file),
        selected,
        io,
    })
}

fn read_page(source: &PageSource, descriptor: &PageDescriptor) -> Result<LoadedPage> {
    let start = usize::try_from(descriptor.offset)
        .map_err(|_| Error::InvalidSegment("v6 page offset exceeds usize".into()))?;
    let end = start
        .checked_add(descriptor.physical_bytes)
        .ok_or_else(|| Error::InvalidSegment("v6 page range overflow".into()))?;
    match source {
        PageSource::File { file, selected, io } => {
            let mut bytes = MutableBuffer::new(descriptor.physical_bytes);
            bytes.resize(descriptor.physical_bytes, 0);
            // A row group retains a complete key/version chain and can exceed
            // the soft page target. The I/O policy bounds each native request,
            // not that authenticated page. Do not expose any bytes until every
            // chunk has been acquired and the full stored-page digest matches.
            let mut actual_io = *selected;
            let mut offset = descriptor.offset;
            if descriptor.physical_bytes == 0 {
                actual_io = io.read_exact_at(actual_io, file, offset, bytes.as_slice_mut())?;
            } else {
                for chunk in bytes
                    .as_slice_mut()
                    .chunks_mut(io.policy().max_request_bytes)
                {
                    actual_io = io.read_exact_at(actual_io, file, offset, chunk)?;
                    offset = offset.checked_add(chunk.len() as u64).ok_or_else(|| {
                        Error::InvalidSegment("v6 page read offset overflow".into())
                    })?;
                }
            }
            let buffer = Buffer::from(bytes);
            let allocated = buffer.capacity();
            verify_stored_page(buffer.as_slice(), descriptor)?;
            finish_page(
                buffer.as_slice(),
                descriptor,
                Some(buffer.clone()),
                PageReadContext {
                    source_allocated_bytes: allocated,
                    actual_io: Some(actual_io),
                    ..PageReadContext::default()
                },
            )
        }
        PageSource::Mapped { bytes, io } => {
            let slice = bytes
                .get(start..end)
                .ok_or_else(|| Error::InvalidSegment("v6 mapped page range is absent".into()))?;
            io.record_read(SelectedIo::Mmap, slice.len());
            verify_stored_page(slice, descriptor)?;
            let raw = if descriptor.compression == format::PageCompression::None {
                Some(borrow_mmap(bytes, start, slice.len())?)
            } else {
                None
            };
            finish_page(
                slice,
                descriptor,
                raw,
                PageReadContext {
                    raw_borrowed: true,
                    actual_io: Some(SelectedIo::Mmap),
                    ..PageReadContext::default()
                },
            )
        }
        PageSource::Bytes(bytes) => {
            let slice = bytes
                .get(start..end)
                .ok_or_else(|| Error::InvalidSegment("v6 snapshot page range is absent".into()))?;
            verify_stored_page(slice, descriptor)?;
            if descriptor.compression == format::PageCompression::None {
                let buffer = Buffer::from(slice);
                let allocated = buffer.capacity();
                finish_page(
                    buffer.as_slice(),
                    descriptor,
                    Some(buffer.clone()),
                    PageReadContext {
                        source_allocated_bytes: allocated,
                        raw_copied_bytes: slice.len(),
                        ..PageReadContext::default()
                    },
                )
            } else {
                finish_page(slice, descriptor, None, PageReadContext::default())
            }
        }
    }
}

fn verify_stored_page(stored: &[u8], descriptor: &PageDescriptor) -> Result<()> {
    if ring::digest::digest(&ring::digest::SHA256, stored).as_ref() != descriptor.digest {
        return invalid("v6 stored-page checksum does not match");
    }
    Ok(())
}

fn finish_page(
    stored: &[u8],
    descriptor: &PageDescriptor,
    raw_buffer: Option<Buffer>,
    context: PageReadContext,
) -> Result<LoadedPage> {
    match descriptor.compression {
        format::PageCompression::None => {
            let buffer = raw_buffer
                .ok_or_else(|| Error::InvalidSegment("v6 raw page has no source buffer".into()))?;
            Ok(LoadedPage {
                buffer,
                borrowed: context.raw_borrowed,
                allocated_bytes: context.source_allocated_bytes,
                copied_bytes: context.raw_copied_bytes,
                decompressed_bytes: 0,
                actual_io: context.actual_io,
            })
        }
        format::PageCompression::Lz4Block => {
            let mut decoded = MutableBuffer::new(descriptor.logical_bytes);
            decoded.resize(descriptor.logical_bytes, 0);
            let decoded_bytes = lz4_flex::block::decompress_into(stored, decoded.as_slice_mut())
                .map_err(|error| {
                    Error::InvalidSegment(format!("v6 LZ4 page decode failed: {error}"))
                })?;
            if decoded_bytes != descriptor.logical_bytes {
                return invalid(format!(
                    "v6 LZ4 page decoded {decoded_bytes} bytes, expected {}",
                    descriptor.logical_bytes
                ));
            }
            let buffer = Buffer::from(decoded);
            let allocated_bytes = context
                .source_allocated_bytes
                .checked_add(buffer.capacity())
                .ok_or_else(|| {
                    Error::InvalidSegment("v6 page allocation evidence overflow".into())
                })?;
            Ok(LoadedPage {
                buffer,
                borrowed: false,
                allocated_bytes,
                copied_bytes: 0,
                decompressed_bytes: descriptor.logical_bytes,
                actual_io: context.actual_io,
            })
        }
    }
}

fn borrow_mmap(bytes: &Arc<Mmap>, start: usize, length: usize) -> Result<Buffer> {
    if length == 0 {
        return Ok(Buffer::default());
    }
    let pointer = unsafe { bytes.as_ptr().add(start) as *mut u8 };
    let pointer = NonNull::new(pointer)
        .ok_or_else(|| Error::InvalidSegment("v6 mapped page has a null pointer".into()))?;
    let owner: Arc<dyn Allocation> = Arc::clone(bytes) as Arc<dyn Allocation>;
    Ok(unsafe { Buffer::from_custom_allocation(pointer, length, owner) })
}

fn lower_bound(spine: &KeySpine, rows: usize, key: &[u8]) -> Result<usize> {
    let mut left = 0;
    let mut right = rows;
    while left < right {
        let middle = left + (right - left) / 2;
        if key_at(spine, middle)? < key {
            left = middle + 1;
        } else {
            right = middle;
        }
    }
    Ok(left)
}

fn key_at(spine: &KeySpine, row: usize) -> Result<&[u8]> {
    binary_at(
        spine.offsets.buffer.as_slice(),
        spine.data.buffer.as_slice(),
        row,
    )
}

fn sequence_at(spine: &KeySpine, row: usize) -> Result<u64> {
    let offset = row
        .checked_mul(8)
        .ok_or_else(|| Error::InvalidSegment("v6 sequence offset overflow".into()))?;
    let bytes = spine
        .sequences
        .buffer
        .as_slice()
        .get(offset..offset + 8)
        .ok_or_else(|| Error::InvalidSegment("v6 sequence row is absent".into()))?;
    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
}

fn value_from_loaded(group: &LoadedRowGroup, row: usize) -> Result<Option<&[u8]>> {
    if !valid_at(group.validity.buffer.as_slice(), row)? {
        return Ok(None);
    }
    Ok(Some(binary_at(
        group.value_offsets.buffer.as_slice(),
        group.values.buffer.as_slice(),
        row,
    )?))
}

fn value_from_columns(columns: &LoadedValueColumns, row: usize) -> Result<Option<&[u8]>> {
    if !valid_at(columns.validity.buffer.as_slice(), row)? {
        return Ok(None);
    }
    Ok(Some(binary_at(
        columns.offsets.buffer.as_slice(),
        columns.values.buffer.as_slice(),
        row,
    )?))
}

fn valid_at(validity: &[u8], row: usize) -> Result<bool> {
    let byte = validity
        .get(row / 8)
        .ok_or_else(|| Error::InvalidSegment("v6 validity row is absent".into()))?;
    Ok(byte & (1 << (row % 8)) != 0)
}

fn binary_at<'a>(offsets: &[u8], data: &'a [u8], row: usize) -> Result<&'a [u8]> {
    let start = offset_at(offsets, row)?;
    let end = offset_at(offsets, row + 1)?;
    if start > end || end > data.len() {
        return invalid("v6 Arrow binary offsets are not monotonic or in bounds");
    }
    Ok(&data[start..end])
}

fn offset_at(offsets: &[u8], index: usize) -> Result<usize> {
    let offset = index
        .checked_mul(8)
        .ok_or_else(|| Error::InvalidSegment("v6 Arrow offset index overflow".into()))?;
    let bytes = offsets
        .get(offset..offset + 8)
        .ok_or_else(|| Error::InvalidSegment("v6 Arrow offset is absent".into()))?;
    let value = i64::from_le_bytes(bytes.try_into().unwrap());
    usize::try_from(value)
        .map_err(|_| Error::InvalidSegment("v6 Arrow offset is negative or too large".into()))
}

fn validate_offsets(
    offsets: &[u8],
    data: &[u8],
    rows: usize,
    maximum_item_bytes: usize,
    allow_empty: bool,
) -> Result<()> {
    if offsets.len() != (rows + 1).saturating_mul(8) || offset_at(offsets, 0)? != 0 {
        return invalid("v6 Arrow binary offset page has the wrong shape");
    }
    for row in 0..rows {
        let start = offset_at(offsets, row)?;
        let end = offset_at(offsets, row + 1)?;
        if start > end
            || end > data.len()
            || end - start > maximum_item_bytes
            || (!allow_empty && start == end)
        {
            return invalid("v6 Arrow binary offsets violate item bounds");
        }
    }
    if offset_at(offsets, rows)? != data.len() {
        return invalid("v6 Arrow binary offsets do not terminate at the data length");
    }
    Ok(())
}

fn validate_validity(validity: &[u8], rows: usize) -> Result<()> {
    if validity.len() != rows.div_ceil(8) {
        return invalid("v6 Arrow validity bitmap has the wrong length");
    }
    let used = rows % 8;
    if used != 0 && validity.last().is_some_and(|byte| byte >> used != 0) {
        return invalid("v6 Arrow validity bitmap has non-zero padding bits");
    }
    Ok(())
}

fn verify_file_digest(file: &mut File, physical_bytes: u64) -> Result<String> {
    let content_bytes = physical_bytes
        .checked_sub(FOOTER_BYTES as u64)
        .ok_or_else(|| Error::InvalidSegment("segment footer underflow".into()))?;
    file.seek(std::io::SeekFrom::Start(0))?;
    let mut hasher = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = vec![0; COPY_BUFFER_BYTES];
    let mut remaining = content_bytes;
    while remaining > 0 {
        let take = usize::try_from(remaining.min(buffer.len() as u64)).unwrap();
        file.read_exact(&mut buffer[..take])?;
        hasher.update(&buffer[..take]);
        remaining -= take as u64;
    }
    let mut footer = [0; FOOTER_BYTES];
    file.read_exact(&mut footer)?;
    let expected = std::str::from_utf8(&footer)
        .map_err(|_| Error::InvalidSegment("segment footer is not ASCII".into()))?;
    let actual = digest_hex(hasher.finish().as_ref());
    if expected != actual {
        return invalid("segment content checksum does not match");
    }
    Ok(actual)
}

fn digest_hex(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = Vec::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize]);
        encoded.push(HEX[(byte & 0x0f) as usize]);
    }
    String::from_utf8(encoded).expect("hex digest is ASCII")
}

fn file_equals_bytes(path: &AnchoredPath, expected: &[u8]) -> Result<bool> {
    let mut file = path.open_read()?;
    if file.metadata()?.len() != expected.len() as u64 {
        return Ok(false);
    }
    let mut buffer = vec![0; COPY_BUFFER_BYTES];
    let mut cursor = 0;
    while cursor < expected.len() {
        let end = cursor.saturating_add(buffer.len()).min(expected.len());
        file.read_exact(&mut buffer[..end - cursor])?;
        if buffer[..end - cursor] != expected[cursor..end] {
            return Ok(false);
        }
        cursor = end;
    }
    Ok(true)
}

fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::InvalidSegment(reason.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Durability, Mutation, WalWriter, WriteBatch};
    use std::collections::BTreeMap;

    fn cache_page(bytes: usize, marker: u8) -> Arc<LoadedPage> {
        Arc::new(LoadedPage {
            buffer: Buffer::from(vec![marker; bytes]),
            borrowed: false,
            allocated_bytes: bytes,
            copied_bytes: bytes,
            decompressed_bytes: 0,
            actual_io: None,
        })
    }

    #[test]
    fn scan_resistant_cache_demotes_large_promotions_without_changing_the_page() {
        let shared = new_page_cache_with_policy(
            128,
            PageCachePolicy::ScanResistantLru {
                protected_capacity_basis_points: 5_000,
            },
        )
        .unwrap();
        let mut cache = shared.lock().unwrap();
        let page = cache_page(80, 1);
        let resident_bytes = page.resident_bytes();
        let inserted = cache
            .complete_load(
                (1, 1),
                Arc::clone(&page),
                resident_bytes,
                PageCacheAccess::OrdinaryOperation,
            )
            .unwrap();
        assert!(Arc::ptr_eq(&page, &inserted));

        let promoted = cache
            .lookup((1, 1), PageCacheAccess::OrdinaryOperation)
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&page, &promoted));
        assert_eq!(cache.promotions, 1);
        assert_eq!(cache.demotions, 1);
        assert_eq!(
            cache.values.get(&(1, 1)).unwrap().region,
            CacheRegion::Probationary
        );
        assert_eq!(cache.protected_resident_bytes, 0);
        assert_eq!(cache.probationary_resident_bytes, resident_bytes);
        cache.validate_residency().unwrap();

        let oversize = cache_page(256, 2);
        let returned = cache
            .complete_load(
                (1, 2),
                Arc::clone(&oversize),
                oversize.resident_bytes(),
                PageCacheAccess::OrdinaryOperation,
            )
            .unwrap();
        assert!(Arc::ptr_eq(&oversize, &returned));
        assert!(!cache.values.contains_key(&(1, 2)));
        assert_eq!(cache.admission_rejections, 1);
        cache.validate_residency().unwrap();
    }

    #[test]
    fn exact_lru_ignores_stale_heap_entries_and_renumbers_at_clock_wrap() {
        let shared = new_page_cache_with_policy(128, PageCachePolicy::ExactLru).unwrap();
        let mut cache = shared.lock().unwrap();
        for (ordinal, marker) in [(1, 1), (2, 2)] {
            let page = cache_page(64, marker);
            cache
                .complete_load(
                    (1, ordinal),
                    Arc::clone(&page),
                    page.resident_bytes(),
                    PageCacheAccess::OrdinaryOperation,
                )
                .unwrap();
        }
        assert!(cache
            .lookup((1, 1), PageCacheAccess::OrdinaryOperation)
            .unwrap()
            .is_some());
        cache.clock = u64::MAX;
        assert!(cache
            .lookup((1, 1), PageCacheAccess::OrdinaryOperation)
            .unwrap()
            .is_some());
        assert!(cache.clock > 0);

        let page = cache_page(64, 3);
        cache
            .complete_load(
                (1, 3),
                Arc::clone(&page),
                page.resident_bytes(),
                PageCacheAccess::OrdinaryOperation,
            )
            .unwrap();
        assert!(cache.values.contains_key(&(1, 1)));
        assert!(!cache.values.contains_key(&(1, 2)));
        assert!(cache.values.contains_key(&(1, 3)));
        assert_eq!(cache.resident_bytes, 128);
        cache.validate_residency().unwrap();
    }

    #[test]
    fn projected_scope_requires_cross_operation_reuse_before_promotion() {
        let shared = new_page_cache_with_policy(
            128,
            PageCachePolicy::ScanResistantLru {
                protected_capacity_basis_points: 5_000,
            },
        )
        .unwrap();
        let mut cache = shared.lock().unwrap();
        let page = cache_page(64, 1);
        cache
            .complete_load(
                (1, 1),
                Arc::clone(&page),
                page.resident_bytes(),
                PageCacheAccess::ProjectedScope(7),
            )
            .unwrap();

        let same_scope = cache
            .lookup((1, 1), PageCacheAccess::ProjectedScope(7))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&page, &same_scope));
        assert_eq!(cache.promotions, 0);
        assert_eq!(cache.same_scope_hits, 1);
        assert_eq!(
            cache.values.get(&(1, 1)).unwrap().region,
            CacheRegion::Probationary
        );

        let later_scope = cache
            .lookup((1, 1), PageCacheAccess::ProjectedScope(8))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&page, &later_scope));
        assert_eq!(cache.promotions, 1);
        assert_eq!(cache.same_scope_hits, 1);
        assert_eq!(
            cache.values.get(&(1, 1)).unwrap().region,
            CacheRegion::Protected
        );
        cache.validate_residency().unwrap();
    }

    fn two_key_sample_memtable() -> Memtable {
        let directory = tempfile::tempdir().unwrap();
        let wal_path = directory.path().join("span.wal");
        let mut wal = WalWriter::create(&wal_path).unwrap();
        let operations = vec![
            Mutation::Put {
                key: b"key:100".to_vec(),
                value: b"alpha".to_vec(),
            },
            Mutation::Put {
                key: b"key:200".to_vec(),
                value: b"beta".to_vec(),
            },
        ];
        wal.append_write_batch(
            &WriteBatch::new(operations).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
        drop(wal);

        let recovery = crate::recover(&wal_path).unwrap();
        Memtable::recover(&recovery.batches).unwrap()
    }

    fn mixed_family_memtable() -> Memtable {
        let mut versions = BTreeMap::<Vec<u8>, Vec<VersionedValue>>::new();
        versions.insert(
            b"ctl:alpha".to_vec(),
            vec![
                VersionedValue {
                    sequence: 1,
                    value: Some(b"ctl-v1".as_ref().into()),
                },
                VersionedValue {
                    sequence: 7,
                    value: None,
                },
            ],
        );
        versions.insert(
            b"doc:alpha".to_vec(),
            vec![VersionedValue {
                sequence: 2,
                value: Some(b"doc-v1".as_ref().into()),
            }],
        );
        versions.insert(
            b"doc:beta".to_vec(),
            vec![
                VersionedValue {
                    sequence: 3,
                    value: None,
                },
                VersionedValue {
                    sequence: 5,
                    value: Some(b"doc-v2".as_ref().into()),
                },
            ],
        );
        versions.insert(
            b"edge:alpha".to_vec(),
            vec![VersionedValue {
                sequence: 4,
                value: Some(b"edge-v1".as_ref().into()),
            }],
        );
        versions.insert(
            b"vec:alpha".to_vec(),
            vec![VersionedValue {
                sequence: 6,
                value: Some(b"vec-v1".as_ref().into()),
            }],
        );
        Memtable::from_versions(versions, 7).unwrap()
    }

    fn visible_ranges_oracle(
        table: &Memtable,
        ranges: &[(Vec<u8>, Vec<u8>)],
        read_sequence: u64,
    ) -> Vec<(Vec<u8>, VersionedValue)> {
        table
            .visible_versions(read_sequence)
            .into_iter()
            .filter_map(|(key, version)| {
                ranges
                    .iter()
                    .find(|(start, end)| {
                        key.as_slice() >= start.as_slice() && key.as_slice() < end.as_slice()
                    })
                    .map(|_| (key, version))
            })
            .collect()
    }

    #[test]
    fn visible_scan_with_no_matching_keys_loads_only_spine_pages() {
        let directory = tempfile::tempdir().unwrap();
        let segments = directory.path().join("segments");
        let cache = new_page_cache(DEFAULT_PAGE_CACHE_BYTES);
        let io = IoContext::new(SegmentIoPolicy::default()).unwrap();
        let table = two_key_sample_memtable();
        let (segment, _path) = Segment::write_from_memtable_with_cache(
            &AnchoredPath::from_path(&segments).unwrap(),
            &table,
            SegmentRowGroupBudget::default(),
            DEFAULT_SEGMENT_COMPRESSION_POLICY,
            Arc::clone(&cache),
            io,
        )
        .unwrap();

        assert_eq!(segment.row_group_count(), 1);

        let before = super::page_cache_stats(&cache);
        let visible = segment
            .visible_from(
                b"key:150",
                Some(b"key:160"),
                segment.descriptor.maximum_sequence,
            )
            .unwrap();
        let after = super::page_cache_stats(&cache);
        let row_groups = u64::try_from(segment.row_group_count()).unwrap_or_default();

        assert!(visible.is_empty());
        assert_eq!(before.loads + 3 * row_groups, after.loads);
    }

    #[test]
    fn visible_scan_with_matching_keys_loads_value_columns() {
        let directory = tempfile::tempdir().unwrap();
        let segments = directory.path().join("segments");
        let cache = new_page_cache(DEFAULT_PAGE_CACHE_BYTES);
        let io = IoContext::new(SegmentIoPolicy::default()).unwrap();
        let table = two_key_sample_memtable();
        let (segment, _path) = Segment::write_from_memtable_with_cache(
            &AnchoredPath::from_path(&segments).unwrap(),
            &table,
            SegmentRowGroupBudget::default(),
            DEFAULT_SEGMENT_COMPRESSION_POLICY,
            Arc::clone(&cache),
            io,
        )
        .unwrap();

        let before = super::page_cache_stats(&cache);
        let visible = segment
            .visible_from(
                b"key:150",
                Some(b"key:250"),
                segment.descriptor.maximum_sequence,
            )
            .unwrap();
        let after = super::page_cache_stats(&cache);
        let row_groups = u64::try_from(segment.row_group_count()).unwrap_or_default();

        assert_eq!(visible.len(), 1);
        assert_eq!(before.loads + 6 * row_groups, after.loads);
    }

    #[test]
    fn visible_scan_matches_memtable_visibility_across_ranges_and_sequences() {
        let directory = tempfile::tempdir().unwrap();
        let segments = directory.path().join("segments");
        let cache = new_page_cache(DEFAULT_PAGE_CACHE_BYTES);
        let io = IoContext::new(SegmentIoPolicy::default()).unwrap();
        let table = mixed_family_memtable();
        let (segment, _path) = Segment::write_from_memtable_with_cache(
            &AnchoredPath::from_path(&segments).unwrap(),
            &table,
            SegmentRowGroupBudget::default(),
            DEFAULT_SEGMENT_COMPRESSION_POLICY,
            Arc::clone(&cache),
            io,
        )
        .unwrap();

        let queries = [
            (b"".as_ref(), None),
            (b"ctl:".as_ref(), Some(b"doc:".as_ref())),
            (b"doc:".as_ref(), Some(b"edge:".as_ref())),
            (b"edge:".as_ref(), Some(b"vec:".as_ref())),
            (b"vec:".as_ref(), Some(b"zzz".as_ref())),
        ];

        for read_sequence in 0..=table.maximum_sequence() {
            for (start, end) in queries {
                let expected = table.visible_from(start, end, read_sequence);
                let actual = segment.visible_from(start, end, read_sequence).unwrap();
                assert_eq!(actual, expected, "sequence {read_sequence}");
            }
        }
    }

    #[test]
    fn visible_ranges_match_memtable_oracle_for_mixed_families() {
        let directory = tempfile::tempdir().unwrap();
        let segments = directory.path().join("segments");
        let cache = new_page_cache(DEFAULT_PAGE_CACHE_BYTES);
        let io = IoContext::new(SegmentIoPolicy::default()).unwrap();
        let table = mixed_family_memtable();
        let (segment, _path) = Segment::write_from_memtable_with_cache(
            &AnchoredPath::from_path(&segments).unwrap(),
            &table,
            SegmentRowGroupBudget::default(),
            DEFAULT_SEGMENT_COMPRESSION_POLICY,
            Arc::clone(&cache),
            io,
        )
        .unwrap();

        let ranges = vec![
            (b"ctl:".to_vec(), b"doc:".to_vec()),
            (b"edge:".to_vec(), b"vec:".to_vec()),
            (b"vec:".to_vec(), b"zzz".to_vec()),
        ];

        for read_sequence in 0..=table.maximum_sequence() {
            let expected = visible_ranges_oracle(&table, &ranges, read_sequence);
            let actual = segment.visible_ranges(&ranges, read_sequence).unwrap();
            assert_eq!(actual, expected, "sequence {read_sequence}");
        }
    }
}
