//! Projected-read requests, ranges, budgets, resources, outcomes, evidence and batches.

use super::*;

pub const PROJECTED_READ_CONTRACT_VERSION: u16 = 1;

pub const DEFAULT_MAX_ACTIVE_READ_VIEWS: usize = 1_024;
pub const DEFAULT_MAX_PROJECTED_READ_RANGES: usize = 1_024;
pub const DEFAULT_MAX_PROJECTED_READ_RUNS: usize = 4_096;
pub const DEFAULT_MAX_PINNED_READ_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_VERSIONS_EXAMINED: u64 = 16_000_000;
pub const DEFAULT_MAX_PAGE_REQUESTS: u64 = 4_000_000;
pub const DEFAULT_MAX_PAGE_LOGICAL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_OUTPUT_ROWS: u64 = 1_000_000;
pub const DEFAULT_MAX_OUTPUT_BUFFER_BYTES: u64 = 1024 * 1024 * 1024;
pub const DEFAULT_MAX_BATCH_ROWS: usize = 8_192;
pub const DEFAULT_MAX_BATCH_ALLOCATED_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedReadRange {
    pub start: Vec<u8>,
    pub end: Option<Vec<u8>>,
}

impl ProjectedReadRange {
    pub fn new(start: Vec<u8>, end: Option<Vec<u8>>) -> Result<Self> {
        let range = Self { start, end };
        range.validate()?;
        Ok(range)
    }

    pub fn all() -> Self {
        Self {
            start: Vec::new(),
            end: None,
        }
    }

    fn validate(&self) -> Result<()> {
        if self
            .end
            .as_ref()
            .is_some_and(|end| self.start.as_slice() >= end.as_slice())
        {
            return Err(Error::InvalidProjectedRead(
                "projected read ranges must be non-empty and half-open".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn contains(&self, key: &[u8]) -> bool {
        key >= self.start.as_slice() && self.end.as_ref().is_none_or(|end| key < end.as_slice())
    }

    pub(crate) fn overlaps(&self, first: &[u8], last: &[u8]) -> bool {
        last >= self.start.as_slice() && self.end.as_ref().is_none_or(|end| first < end.as_slice())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectedReadProjection {
    Key,
    KeyValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedReadBudget {
    pub max_active_views: usize,
    pub max_ranges: usize,
    pub max_runs: usize,
    /// Ceiling for distinct retained segment and mutable generations.
    /// This is a logical ownership charge, not process memory usage.
    pub max_pinned_bytes: u64,
    pub max_versions_examined: u64,
    pub max_page_requests: u64,
    pub max_page_logical_bytes: u64,
    pub max_output_rows: u64,
    pub max_output_buffer_bytes: u64,
    pub max_batch_rows: usize,
    pub max_batch_allocated_bytes: usize,
}

impl Default for ProjectedReadBudget {
    fn default() -> Self {
        Self {
            max_active_views: DEFAULT_MAX_ACTIVE_READ_VIEWS,
            max_ranges: DEFAULT_MAX_PROJECTED_READ_RANGES,
            max_runs: DEFAULT_MAX_PROJECTED_READ_RUNS,
            max_pinned_bytes: DEFAULT_MAX_PINNED_READ_BYTES,
            max_versions_examined: DEFAULT_MAX_VERSIONS_EXAMINED,
            max_page_requests: DEFAULT_MAX_PAGE_REQUESTS,
            max_page_logical_bytes: DEFAULT_MAX_PAGE_LOGICAL_BYTES,
            max_output_rows: DEFAULT_MAX_OUTPUT_ROWS,
            max_output_buffer_bytes: DEFAULT_MAX_OUTPUT_BUFFER_BYTES,
            max_batch_rows: DEFAULT_MAX_BATCH_ROWS,
            max_batch_allocated_bytes: DEFAULT_MAX_BATCH_ALLOCATED_BYTES,
        }
    }
}

impl ProjectedReadBudget {
    fn validate(self) -> Result<Self> {
        for (resource, value) in [
            (ProjectedReadResource::ActiveViews, self.max_active_views),
            (ProjectedReadResource::Ranges, self.max_ranges),
            (ProjectedReadResource::Runs, self.max_runs),
            (ProjectedReadResource::BatchRows, self.max_batch_rows),
            (
                ProjectedReadResource::BatchAllocatedBytes,
                self.max_batch_allocated_bytes,
            ),
        ] {
            if value == 0 {
                return Err(Error::InvalidProjectedReadBudget { resource });
            }
        }
        for (resource, value) in [
            (ProjectedReadResource::PinnedBytes, self.max_pinned_bytes),
            (
                ProjectedReadResource::VersionsExamined,
                self.max_versions_examined,
            ),
            (ProjectedReadResource::PageRequests, self.max_page_requests),
            (
                ProjectedReadResource::PageLogicalBytes,
                self.max_page_logical_bytes,
            ),
            (ProjectedReadResource::OutputRows, self.max_output_rows),
            (
                ProjectedReadResource::OutputBufferBytes,
                self.max_output_buffer_bytes,
            ),
        ] {
            if value == 0 {
                return Err(Error::InvalidProjectedReadBudget { resource });
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedReadRequest {
    pub ranges: Vec<ProjectedReadRange>,
    pub projection: ProjectedReadProjection,
    pub snapshot: Snapshot,
    pub budget: ProjectedReadBudget,
}

impl ProjectedReadRequest {
    pub(crate) fn validate(&self, current_sequence: u64) -> Result<()> {
        self.budget.validate()?;
        if self.ranges.is_empty() {
            return Err(Error::InvalidProjectedRead(
                "projected read requires at least one half-open range".into(),
            ));
        }
        check_limit(
            ProjectedReadResource::Ranges,
            self.budget.max_ranges as u64,
            self.ranges.len() as u64,
        )?;
        for range in &self.ranges {
            range.validate()?;
        }
        for adjacent in self.ranges.windows(2) {
            let previous_end = adjacent[0].end.as_ref().ok_or_else(|| {
                Error::InvalidProjectedRead(
                    "an unbounded projected range must be the final range".into(),
                )
            })?;
            if previous_end.as_slice() > adjacent[1].start.as_slice() {
                return Err(Error::InvalidProjectedRead(
                    "projected read ranges must be sorted and disjoint".into(),
                ));
            }
        }
        if self.snapshot.sequence > current_sequence {
            return Err(Error::InvalidProjectedRead(format!(
                "snapshot sequence {} is newer than current sequence {current_sequence}",
                self.snapshot.sequence
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectedReadResource {
    ActiveViews,
    Ranges,
    Runs,
    PinnedBytes,
    VersionsExamined,
    PageRequests,
    PageLogicalBytes,
    OutputRows,
    OutputBufferBytes,
    BatchRows,
    BatchAllocatedBytes,
    RetainedOutputBytes,
}

impl ProjectedReadResource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ActiveViews => "active_views",
            Self::Ranges => "ranges",
            Self::Runs => "runs",
            Self::PinnedBytes => "pinned_bytes",
            Self::VersionsExamined => "versions_examined",
            Self::PageRequests => "page_requests",
            Self::PageLogicalBytes => "page_logical_bytes",
            Self::OutputRows => "output_rows",
            Self::OutputBufferBytes => "output_buffer_bytes",
            Self::BatchRows => "batch_rows",
            Self::BatchAllocatedBytes => "batch_allocated_bytes",
            Self::RetainedOutputBytes => "retained_output_bytes",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectedReadOutcome {
    #[default]
    Creating,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedReadEvidence {
    pub contract_version: u16,
    pub manifest: String,
    pub snapshot_sequence: u64,
    pub projection: Option<ProjectedReadProjection>,
    pub selected_ranges: u64,
    pub selected_runs: u64,
    pub selected_row_groups: u64,
    /// Logical footprint of this view; overlapping views can share a generation.
    pub pinned_bytes: u64,
    pub versions_examined: u64,
    pub page_requests: u64,
    pub page_logical_bytes: u64,
    pub key_offset_page_requests: u64,
    pub key_data_page_requests: u64,
    pub sequence_page_requests: u64,
    pub validity_page_requests: u64,
    pub value_offset_page_requests: u64,
    pub value_data_page_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_loads: u64,
    pub mmap_read_operations: u64,
    pub io_uring_read_operations: u64,
    pub bounded_read_operations: u64,
    pub physical_bytes_read: u64,
    pub bytes_borrowed: u64,
    pub bytes_decoded: u64,
    pub bytes_decompressed: u64,
    pub bytes_allocated: u64,
    pub bytes_copied: u64,
    pub emitted_rows: u64,
    pub emitted_key_bytes: u64,
    pub emitted_value_bytes: u64,
    pub emitted_buffer_bytes: u64,
    pub emitted_batches: u64,
    pub peak_batch_rows: u64,
    pub peak_batch_allocated_bytes: u64,
    pub outcome: ProjectedReadOutcome,
}

/// Immutable copied output. Clones and slices of its Arrow buffers keep the
/// originating database-open instance's output reservation until the last
/// allocation owner drops. These custom-owned buffers cannot be converted
/// in place to mutable buffers; caller-created copies are outside that pool.
#[derive(Debug, Clone)]
pub struct ProjectedReadBatch {
    pub(super) projection: ProjectedReadProjection,
    pub(super) rows: usize,
    pub(super) key_offsets: Buffer,
    pub(super) key_data: Buffer,
    pub(super) value_offsets: Option<Buffer>,
    pub(super) value_data: Option<Buffer>,
}

impl ProjectedReadBatch {
    pub fn len(&self) -> usize {
        self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    pub fn projection(&self) -> ProjectedReadProjection {
        self.projection
    }

    pub fn key_offsets_buffer(&self) -> &Buffer {
        &self.key_offsets
    }

    pub fn key_data_buffer(&self) -> &Buffer {
        &self.key_data
    }

    pub fn value_offsets_buffer(&self) -> Option<&Buffer> {
        self.value_offsets.as_ref()
    }

    pub fn value_data_buffer(&self) -> Option<&Buffer> {
        self.value_data.as_ref()
    }

    pub fn key(&self, row: usize) -> Option<&[u8]> {
        binary_value(&self.key_offsets, &self.key_data, row, self.rows)
    }

    pub fn value(&self, row: usize) -> Option<&[u8]> {
        binary_value(
            self.value_offsets.as_ref()?,
            self.value_data.as_ref()?,
            row,
            self.rows,
        )
    }

    pub fn allocated_bytes(&self) -> usize {
        self.key_offsets
            .capacity()
            .saturating_add(self.key_data.capacity())
            .saturating_add(self.value_offsets.as_ref().map_or(0, Buffer::capacity))
            .saturating_add(self.value_data.as_ref().map_or(0, Buffer::capacity))
    }
}
