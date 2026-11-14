mod format;

mod output;

mod source_memory;

#[cfg(test)]
mod source_memory_tests;

pub use source_memory::{SourcePageMemory, SourcePageStats, DEFAULT_SOURCE_PAGE_BYTES};

#[cfg(feature = "physical-policy-lab")]
mod physical_policy_lab;

mod reader;

mod decode;
mod evidence;
mod open;
mod page;
mod page_cache;
mod read;
mod write;

use decode::*;
pub use evidence::*;
pub(crate) use page::*;
pub use page_cache::*;
use write::*;

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

pub const DEFAULT_PAGE_CACHE_BYTES: usize = 64 * 1024 * 1024;

pub const DEFAULT_PAGE_CACHE_PROTECTED_CAPACITY_BASIS_POINTS: u16 = 8_000;

const PAGE_CACHE_CAPACITY_BASIS_POINTS: usize = 10_000;

pub const SEGMENT_OPEN_EVIDENCE_VERSION: u16 = 3;

const COPY_BUFFER_BYTES: usize = 64 * 1024;

static TEMPORARY_ID: AtomicU64 = AtomicU64::new(1);

static CACHE_ID: AtomicU64 = AtomicU64::new(1);

static CACHE_ACCESS_SCOPE_ID: AtomicU64 = AtomicU64::new(1);

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

/// Authenticated row identity without a retained page or copied value. Keeping
/// the segment here prevents a selected row from being read from another file.
pub(crate) struct SelectedVersion<'a> {
    segment: &'a Segment,
    row_group: usize,
    row: usize,
    sequence: u64,
}

impl SelectedVersion<'_> {
    pub(crate) fn sequence(&self) -> u64 {
        self.sequence
    }

    pub(crate) fn belongs_to(&self, segment: &Segment) -> bool {
        std::ptr::eq(self.segment, segment)
    }

    pub(crate) fn value(self) -> Result<Option<Vec<u8>>> {
        self.segment.value_at(self.row_group, self.row)
    }
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

#[cfg(test)]
mod tests;
