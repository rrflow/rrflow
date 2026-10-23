use super::format::PageKind;
use super::{
    key_at, offset_at, sequence_at, valid_at, KeySpine, PageCacheAccess, PageLoadEvidence, Segment,
};
use crate::{Error, Manifest, ProjectedOutputMemory, Result, Snapshot};
use arrow_buffer::{Buffer, MutableBuffer};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};
use std::sync::{Arc, Mutex};

#[cfg(test)]
#[path = "reader_lifetime_tests.rs"]
mod lifetime_tests;

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
    projection: ProjectedReadProjection,
    rows: usize,
    key_offsets: Buffer,
    key_data: Buffer,
    value_offsets: Option<Buffer>,
    value_data: Option<Buffer>,
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

#[derive(Debug, Default)]
pub(crate) struct ActiveReadViews {
    active: usize,
    pinned_bytes: u64,
    manifests: BTreeMap<String, ActiveManifest>,
}

#[derive(Debug)]
struct ActiveManifest {
    manifest: Manifest,
    references: usize,
    pinned_bytes: u64,
}

impl ActiveReadViews {
    pub(crate) fn ownership(registry: &Arc<Mutex<Self>>) -> Result<(u64, u64)> {
        let views = registry
            .lock()
            .map_err(|_| Error::PoisonedReadViewRegistry)?;
        Ok((views.active as u64, views.pinned_bytes))
    }

    pub(crate) fn acquire(
        registry: &Arc<Mutex<Self>>,
        manifest: Manifest,
        pinned_bytes: u64,
        budget: ProjectedReadBudget,
    ) -> Result<ReadViewLease> {
        let mut views = registry
            .lock()
            .map_err(|_| Error::PoisonedReadViewRegistry)?;
        let next_active = views
            .active
            .checked_add(1)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        check_limit(
            ProjectedReadResource::ActiveViews,
            budget.max_active_views as u64,
            next_active as u64,
        )?;
        let next_pinned = views
            .pinned_bytes
            .checked_add(pinned_bytes)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        check_limit(
            ProjectedReadResource::PinnedBytes,
            budget.max_pinned_bytes,
            next_pinned,
        )?;
        let digest = manifest.digest.clone();
        let (next_references, next_manifest_pinned) = match views.manifests.get(&digest) {
            Some(entry) => {
                if entry.manifest != manifest {
                    return Err(Error::PoisonedReadViewRegistry);
                }
                (
                    entry
                        .references
                        .checked_add(1)
                        .ok_or(Error::PoisonedReadViewRegistry)?,
                    entry
                        .pinned_bytes
                        .checked_add(pinned_bytes)
                        .ok_or(Error::PoisonedReadViewRegistry)?,
                )
            }
            None => (1, pinned_bytes),
        };
        match views.manifests.get_mut(&digest) {
            Some(entry) => {
                entry.references = next_references;
                entry.pinned_bytes = next_manifest_pinned;
            }
            None => {
                views.manifests.insert(
                    digest.clone(),
                    ActiveManifest {
                        manifest,
                        references: next_references,
                        pinned_bytes: next_manifest_pinned,
                    },
                );
            }
        }
        views.active = next_active;
        views.pinned_bytes = next_pinned;
        Ok(ReadViewLease {
            registry: Arc::clone(registry),
            manifest: digest,
            pinned_bytes,
            released: false,
        })
    }

    pub(crate) fn visit_manifests(
        registry: &Arc<Mutex<Self>>,
        mut visit: impl FnMut(&Manifest) -> Result<()>,
    ) -> Result<()> {
        let views = registry
            .lock()
            .map_err(|_| Error::PoisonedReadViewRegistry)?;
        for entry in views.manifests.values() {
            visit(&entry.manifest)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ReadViewLease {
    registry: Arc<Mutex<ActiveReadViews>>,
    manifest: String,
    pinned_bytes: u64,
    released: bool,
}

impl ReadViewLease {
    fn release(&mut self) -> Result<()> {
        if self.released {
            return Ok(());
        }
        let mut views = self
            .registry
            .lock()
            .map_err(|_| Error::PoisonedReadViewRegistry)?;
        let next_active = views
            .active
            .checked_sub(1)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        let next_pinned = views
            .pinned_bytes
            .checked_sub(self.pinned_bytes)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        let entry = views
            .manifests
            .get(&self.manifest)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        let next_references = entry
            .references
            .checked_sub(1)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        let next_manifest_pinned = entry
            .pinned_bytes
            .checked_sub(self.pinned_bytes)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        let remove = next_references == 0;
        // Empty views legitimately hold a reference with zero pinned bytes.
        // Only the last reference requires all of this manifest's charge gone.
        if remove && next_manifest_pinned != 0 {
            return Err(Error::PoisonedReadViewRegistry);
        }
        if remove {
            views.manifests.remove(&self.manifest);
        } else {
            let entry = views
                .manifests
                .get_mut(&self.manifest)
                .expect("active read manifest was validated");
            entry.references = next_references;
            entry.pinned_bytes = next_manifest_pinned;
        }
        views.active = next_active;
        views.pinned_bytes = next_pinned;
        self.released = true;
        Ok(())
    }
}

impl Drop for ReadViewLease {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[derive(Debug)]
pub(crate) struct ReadView {
    pub sequence: u64,
    pub manifest: String,
    pub memtable: Arc<crate::mutable_memory::MutableGeneration>,
    pub segments: Vec<Arc<Segment>>,
    pub pinned_bytes: u64,
}

#[derive(Debug)]
pub struct ProjectedReadStream {
    projection: ProjectedReadProjection,
    budget: ProjectedReadBudget,
    meter: ProjectedReadMeter,
    cursors: Vec<RunCursor>,
    heads: Vec<Option<ProjectedEntry>>,
    heap: BinaryHeap<HeapItem>,
    pending: Option<PreparedRow>,
    initialized: bool,
    terminal: bool,
    lease: Option<ReadViewLease>,
    output_memory: ProjectedOutputMemory,
}

impl ProjectedReadStream {
    pub(crate) fn new(
        view: ReadView,
        ranges: Vec<ProjectedReadRange>,
        projection: ProjectedReadProjection,
        budget: ProjectedReadBudget,
        cache_access: PageCacheAccess,
        lease: ReadViewLease,
        output_memory: ProjectedOutputMemory,
    ) -> Self {
        let ranges: Arc<[ProjectedReadRange]> = ranges.into();
        let mut cursors = Vec::with_capacity(
            view.segments
                .len()
                .saturating_add(usize::from(view.memtable.version_count() != 0)),
        );
        if view.memtable.version_count() != 0 {
            cursors.push(RunCursor::Memtable(MemtableProjectedCursor {
                table: Arc::clone(&view.memtable),
                ranges: Arc::clone(&ranges),
                read_sequence: view.sequence,
                after: None,
            }));
        }
        cursors.extend(view.segments.into_iter().map(|segment| {
            RunCursor::Segment(SegmentProjectedCursor::new(
                segment,
                Arc::clone(&ranges),
                view.sequence,
                cache_access,
            ))
        }));
        let evidence = ProjectedReadEvidence {
            contract_version: PROJECTED_READ_CONTRACT_VERSION,
            manifest: view.manifest,
            snapshot_sequence: view.sequence,
            projection: Some(projection),
            selected_ranges: ranges.len() as u64,
            selected_runs: cursors.len() as u64,
            pinned_bytes: view.pinned_bytes,
            outcome: ProjectedReadOutcome::Creating,
            ..ProjectedReadEvidence::default()
        };
        let heads = (0..cursors.len()).map(|_| None).collect();
        Self {
            projection,
            budget,
            meter: ProjectedReadMeter { evidence, budget },
            cursors,
            heads,
            heap: BinaryHeap::new(),
            pending: None,
            initialized: false,
            terminal: false,
            lease: Some(lease),
            output_memory,
        }
    }

    pub fn evidence(&self) -> &ProjectedReadEvidence {
        &self.meter.evidence
    }

    pub fn cancel(&mut self) -> Result<()> {
        if self.terminal {
            return Ok(());
        }
        self.finish(ProjectedReadOutcome::Cancelled)
    }

    pub fn next_batch(&mut self) -> Result<Option<ProjectedReadBatch>> {
        if self.terminal {
            return Ok(None);
        }
        self.meter.evidence.outcome = ProjectedReadOutcome::Running;
        let result = self.next_batch_inner();
        match result {
            Ok(Some(batch)) => Ok(Some(batch)),
            Ok(None) => {
                self.finish(ProjectedReadOutcome::Completed)?;
                Ok(None)
            }
            Err(error) => {
                let _ = self.finish(ProjectedReadOutcome::Failed);
                Err(error)
            }
        }
    }

    fn next_batch_inner(&mut self) -> Result<Option<ProjectedReadBatch>> {
        self.initialize()?;
        let mut rows = Vec::new();
        let mut key_bytes = 0usize;
        let mut value_bytes = 0usize;
        loop {
            let prepared = match self.pending.take() {
                Some(row) => Some(row),
                None => self.next_row()?,
            };
            let Some(prepared) = prepared else {
                break;
            };
            let next_rows = rows.len().checked_add(1).ok_or_else(|| {
                Error::InvalidProjectedRead("projected batch row count overflow".into())
            })?;
            let next_key_bytes = key_bytes.checked_add(prepared.key.len()).ok_or_else(|| {
                Error::InvalidProjectedRead("projected key byte count overflow".into())
            })?;
            let next_value_bytes =
                value_bytes.checked_add(prepared.value_len).ok_or_else(|| {
                    Error::InvalidProjectedRead("projected value byte count overflow".into())
                })?;
            let allocation = batch_allocation_bytes(
                next_rows,
                next_key_bytes,
                next_value_bytes,
                self.projection,
            )?;
            if next_rows > self.budget.max_batch_rows
                || allocation > self.budget.max_batch_allocated_bytes
            {
                if rows.is_empty() {
                    let (resource, limit, observed) = if next_rows > self.budget.max_batch_rows {
                        (
                            ProjectedReadResource::BatchRows,
                            self.budget.max_batch_rows as u64,
                            next_rows as u64,
                        )
                    } else {
                        (
                            ProjectedReadResource::BatchAllocatedBytes,
                            self.budget.max_batch_allocated_bytes as u64,
                            allocation as u64,
                        )
                    };
                    return Err(Error::ProjectedReadLimit {
                        resource,
                        limit,
                        observed,
                    });
                }
                self.pending = Some(prepared);
                break;
            }
            let projected_rows = self
                .meter
                .evidence
                .emitted_rows
                .checked_add(next_rows as u64)
                .ok_or_else(|| {
                    Error::InvalidProjectedRead("projected output row count overflow".into())
                })?;
            check_limit(
                ProjectedReadResource::OutputRows,
                self.budget.max_output_rows,
                projected_rows,
            )?;
            let projected_bytes = self
                .meter
                .evidence
                .emitted_buffer_bytes
                .checked_add(allocation as u64)
                .ok_or_else(|| {
                    Error::InvalidProjectedRead("projected output byte count overflow".into())
                })?;
            check_limit(
                ProjectedReadResource::OutputBufferBytes,
                self.budget.max_output_buffer_bytes,
                projected_bytes,
            )?;
            key_bytes = next_key_bytes;
            value_bytes = next_value_bytes;
            rows.push(prepared);
            if rows.len() == self.budget.max_batch_rows {
                break;
            }
        }
        if rows.is_empty() {
            return Ok(None);
        }
        self.build_batch(rows, key_bytes, value_bytes).map(Some)
    }

    fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        self.initialized = true;
        for run in 0..self.cursors.len() {
            self.advance(run)?;
        }
        Ok(())
    }

    fn advance(&mut self, run: usize) -> Result<()> {
        let entry = self.cursors[run].next_candidate(&mut self.meter)?;
        if let Some(entry) = entry {
            self.heap.push(HeapItem {
                key: entry.key.clone(),
                run,
            });
            self.heads[run] = Some(entry);
        }
        Ok(())
    }

    fn next_row(&mut self) -> Result<Option<PreparedRow>> {
        loop {
            let Some(first) = self.heap.pop() else {
                return Ok(None);
            };
            let key = first.key;
            let mut runs = vec![first.run];
            while self.heap.peek().is_some_and(|item| item.key == key) {
                runs.push(self.heap.pop().expect("heap item was checked").run);
            }
            let mut entries = Vec::with_capacity(runs.len());
            let mut sequences = BTreeMap::new();
            for run in &runs {
                let entry = self.heads[*run].take().ok_or_else(|| {
                    Error::InvalidProjectedRead("projected merge heap lost its run head".into())
                })?;
                if entry.key != key {
                    return Err(Error::InvalidProjectedRead(
                        "projected merge heap key differs from its run head".into(),
                    ));
                }
                if sequences.insert(entry.sequence, *run).is_some() {
                    return Err(Error::DuplicateProjectedSequence {
                        manifest: self.meter.evidence.manifest.clone(),
                        sequence: entry.sequence,
                    });
                }
                entries.push((*run, entry));
            }
            let winner = entries
                .iter()
                .max_by_key(|(_, entry)| entry.sequence)
                .expect("one heap item creates one entry");
            let mut prepared = self.cursors[winner.0].prepare_value(
                winner.1.clone(),
                self.projection,
                &mut self.meter,
            )?;
            if let Some(prepared) = prepared.as_mut() {
                prepared.run = winner.0;
            }
            for run in runs {
                self.advance(run)?;
            }
            if let Some(prepared) = prepared {
                return Ok(Some(prepared));
            }
        }
    }

    fn build_batch(
        &mut self,
        rows: Vec<PreparedRow>,
        key_bytes: usize,
        value_bytes: usize,
    ) -> Result<ProjectedReadBatch> {
        let allocation =
            batch_allocation_bytes(rows.len(), key_bytes, value_bytes, self.projection)?;
        let mut reservation = self.output_memory.reserve(allocation)?;
        self.meter.record_batch_allocation(allocation)?;
        let mut key_offsets = MutableBuffer::from_len_zeroed((rows.len() + 1) * 8);
        let mut key_data = MutableBuffer::from_len_zeroed(key_bytes);
        let mut value_offsets = (self.projection == ProjectedReadProjection::KeyValue)
            .then(|| MutableBuffer::from_len_zeroed((rows.len() + 1) * 8));
        let mut value_data = (self.projection == ProjectedReadProjection::KeyValue)
            .then(|| MutableBuffer::from_len_zeroed(value_bytes));
        let mut key_cursor = 0usize;
        let mut value_cursor = 0usize;
        write_offset(key_offsets.as_slice_mut(), 0, 0)?;
        if let Some(offsets) = value_offsets.as_mut() {
            write_offset(offsets.as_slice_mut(), 0, 0)?;
        }
        for (index, row) in rows.iter().enumerate() {
            let key_end = key_cursor.checked_add(row.key.len()).ok_or_else(|| {
                Error::InvalidProjectedRead("projected key output overflow".into())
            })?;
            key_data.as_slice_mut()[key_cursor..key_end].copy_from_slice(&row.key);
            key_cursor = key_end;
            write_offset(key_offsets.as_slice_mut(), index + 1, key_cursor)?;
            if self.projection == ProjectedReadProjection::KeyValue {
                let value_end = value_cursor.checked_add(row.value_len).ok_or_else(|| {
                    Error::InvalidProjectedRead("projected value output overflow".into())
                })?;
                let output = value_data
                    .as_mut()
                    .expect("value projection allocated value data")
                    .as_slice_mut()
                    .get_mut(value_cursor..value_end)
                    .ok_or_else(|| {
                        Error::InvalidProjectedRead("projected value output is absent".into())
                    })?;
                self.cursors[row.run].copy_value(row, output, &mut self.meter)?;
                value_cursor = value_end;
                write_offset(
                    value_offsets
                        .as_mut()
                        .expect("value projection allocated offsets")
                        .as_slice_mut(),
                    index + 1,
                    value_cursor,
                )?;
            }
        }
        let batch = ProjectedReadBatch {
            projection: self.projection,
            rows: rows.len(),
            key_offsets: reservation.wrap(key_offsets)?,
            key_data: reservation.wrap(key_data)?,
            value_offsets: value_offsets
                .map(|buffer| reservation.wrap(buffer))
                .transpose()?,
            value_data: value_data
                .map(|buffer| reservation.wrap(buffer))
                .transpose()?,
        };
        self.meter.record_emitted_batch(
            rows.len() as u64,
            key_bytes as u64,
            value_bytes as u64,
            batch.allocated_bytes() as u64,
        )?;
        Ok(batch)
    }

    fn finish(&mut self, outcome: ProjectedReadOutcome) -> Result<()> {
        if self.terminal {
            return Ok(());
        }
        self.terminal = true;
        self.meter.evidence.outcome = outcome;
        // Release captured generations and temporary buffers before the pin.
        // Returned batches own copied buffers independently of these sources.
        self.pending = None;
        self.heads = Vec::new();
        self.heap = BinaryHeap::new();
        self.cursors = Vec::new();
        let release = self
            .lease
            .take()
            .map_or(Ok(()), |mut lease| lease.release());
        tracing::debug!(
            target: "rrd_lsm::projected_read",
            manifest = %self.meter.evidence.manifest,
            snapshot_sequence = self.meter.evidence.snapshot_sequence,
            projection = ?self.projection,
            selected_ranges = self.meter.evidence.selected_ranges,
            selected_runs = self.meter.evidence.selected_runs,
            selected_row_groups = self.meter.evidence.selected_row_groups,
            versions_examined = self.meter.evidence.versions_examined,
            page_requests = self.meter.evidence.page_requests,
            physical_bytes_read = self.meter.evidence.physical_bytes_read,
            emitted_rows = self.meter.evidence.emitted_rows,
            emitted_buffer_bytes = self.meter.evidence.emitted_buffer_bytes,
            outcome = ?outcome,
            "projected physical read reached a terminal state"
        );
        release
    }
}

impl Drop for ProjectedReadStream {
    fn drop(&mut self) {
        if !self.terminal {
            let _ = self.finish(ProjectedReadOutcome::Cancelled);
        }
    }
}

#[derive(Debug)]
enum RunCursor {
    Memtable(MemtableProjectedCursor),
    Segment(SegmentProjectedCursor),
}

impl RunCursor {
    fn next_candidate(&mut self, meter: &mut ProjectedReadMeter) -> Result<Option<ProjectedEntry>> {
        match self {
            Self::Memtable(cursor) => cursor.next_candidate(meter),
            Self::Segment(cursor) => cursor.next_candidate(meter),
        }
    }

    fn prepare_value(
        &self,
        entry: ProjectedEntry,
        projection: ProjectedReadProjection,
        meter: &mut ProjectedReadMeter,
    ) -> Result<Option<PreparedRow>> {
        match self {
            Self::Memtable(cursor) => cursor.prepare_value(entry, projection),
            Self::Segment(cursor) => cursor.prepare_value(entry, projection, meter),
        }
    }

    fn copy_value(
        &self,
        row: &PreparedRow,
        output: &mut [u8],
        meter: &mut ProjectedReadMeter,
    ) -> Result<()> {
        match self {
            Self::Memtable(cursor) => cursor.copy_value(row, output),
            Self::Segment(cursor) => cursor.copy_value(row, output, meter),
        }
    }
}

#[derive(Debug)]
struct MemtableProjectedCursor {
    table: Arc<crate::mutable_memory::MutableGeneration>,
    ranges: Arc<[ProjectedReadRange]>,
    read_sequence: u64,
    after: Option<Vec<u8>>,
}

impl MemtableProjectedCursor {
    fn next_candidate(&mut self, meter: &mut ProjectedReadMeter) -> Result<Option<ProjectedEntry>> {
        let seek = self.table.seek_visible_after(
            &self.ranges,
            self.after.as_deref(),
            self.read_sequence,
        )?;
        meter.record_versions(seek.versions_examined)?;
        let Some((key, sequence)) = seek.next else {
            return Ok(None);
        };
        self.after = Some(key.clone());
        Ok(Some(ProjectedEntry {
            key,
            sequence,
            location: ProjectedLocation::Memtable,
        }))
    }

    fn prepare_value(
        &self,
        entry: ProjectedEntry,
        projection: ProjectedReadProjection,
    ) -> Result<Option<PreparedRow>> {
        let version = self
            .table
            .get_version(&entry.key, self.read_sequence)
            .filter(|version| version.sequence == entry.sequence)
            .ok_or_else(|| {
                Error::InvalidProjectedRead(
                    "captured memtable no longer contains its projected version".into(),
                )
            })?;
        let Some(value) = version.value.as_deref() else {
            return Ok(None);
        };
        Ok(Some(PreparedRow {
            run: 0,
            key: entry.key,
            value_len: if projection == ProjectedReadProjection::KeyValue {
                value.len()
            } else {
                0
            },
            location: ProjectedLocation::Memtable,
        }))
    }

    fn copy_value(&self, row: &PreparedRow, output: &mut [u8]) -> Result<()> {
        let version = self
            .table
            .get_version(&row.key, self.read_sequence)
            .filter(|version| {
                version
                    .value
                    .as_ref()
                    .is_some_and(|value| value.len() == row.value_len)
            })
            .ok_or_else(|| {
                Error::InvalidProjectedRead(
                    "captured memtable value differs from its projected length".into(),
                )
            })?;
        output.copy_from_slice(version.value.as_deref().expect("value was checked"));
        Ok(())
    }
}

#[derive(Debug)]
struct SegmentProjectedCursor {
    segment: Arc<Segment>,
    ranges: Arc<[ProjectedReadRange]>,
    read_sequence: u64,
    cache_access: PageCacheAccess,
    row_group: usize,
    row: usize,
    loaded_spine: Option<KeySpine>,
    lookahead: Option<RawSegmentRow>,
}

impl SegmentProjectedCursor {
    fn new(
        segment: Arc<Segment>,
        ranges: Arc<[ProjectedReadRange]>,
        read_sequence: u64,
        cache_access: PageCacheAccess,
    ) -> Self {
        let row_group = segment
            .row_groups
            .partition_point(|group| group.last_key.as_slice() < ranges[0].start.as_slice());
        Self {
            segment,
            ranges,
            read_sequence,
            cache_access,
            row_group,
            row: 0,
            loaded_spine: None,
            lookahead: None,
        }
    }

    fn next_candidate(&mut self, meter: &mut ProjectedReadMeter) -> Result<Option<ProjectedEntry>> {
        loop {
            let Some(first) = self.take_or_read_row(meter)? else {
                return Ok(None);
            };
            let key = first.key.clone();
            let mut best = (first.sequence <= self.read_sequence).then_some(first);
            while let Some(next) = self.read_row(meter)? {
                if next.key != key {
                    self.lookahead = Some(next);
                    break;
                }
                if next.sequence <= self.read_sequence
                    && best
                        .as_ref()
                        .is_none_or(|current| next.sequence > current.sequence)
                {
                    best = Some(next);
                }
            }
            if let Some(best) = best {
                return Ok(Some(ProjectedEntry {
                    key,
                    sequence: best.sequence,
                    location: ProjectedLocation::Segment {
                        row_group: best.row_group,
                        row: best.row,
                        value_start: None,
                        value_end: None,
                    },
                }));
            }
        }
    }

    fn take_or_read_row(
        &mut self,
        meter: &mut ProjectedReadMeter,
    ) -> Result<Option<RawSegmentRow>> {
        match self.lookahead.take() {
            Some(row) => Ok(Some(row)),
            None => self.read_row(meter),
        }
    }

    fn read_row(&mut self, meter: &mut ProjectedReadMeter) -> Result<Option<RawSegmentRow>> {
        loop {
            if self.row_group >= self.segment.row_groups.len() {
                return Ok(None);
            }
            let descriptor = &self.segment.row_groups[self.row_group];
            let selected = self
                .ranges
                .iter()
                .any(|range| range.overlaps(&descriptor.first_key, &descriptor.last_key));
            let sequence_selected =
                descriptor.page(PageKind::SequenceValues).statistic_min <= self.read_sequence;
            if !selected || !sequence_selected {
                self.row_group += 1;
                self.row = 0;
                self.loaded_spine = None;
                continue;
            }
            if self.loaded_spine.is_none() {
                self.loaded_spine = Some(self.segment.load_projected_key_spine(
                    self.row_group,
                    meter,
                    self.cache_access,
                )?);
                meter.record_selected_row_group()?;
                self.row = 0;
            }
            let descriptor = &self.segment.row_groups[self.row_group];
            if self.row >= descriptor.row_count as usize {
                self.row_group += 1;
                self.row = 0;
                self.loaded_spine = None;
                continue;
            }
            meter.record_versions(1)?;
            let row = self.row;
            self.row += 1;
            let spine = self.loaded_spine.as_ref().expect("spine was loaded");
            let key = key_at(spine, row)?;
            if !self.ranges.iter().any(|range| range.contains(key)) {
                continue;
            }
            return Ok(Some(RawSegmentRow {
                key: key.to_vec(),
                sequence: sequence_at(spine, row)?,
                row_group: self.row_group,
                row,
            }));
        }
    }

    fn prepare_value(
        &self,
        mut entry: ProjectedEntry,
        projection: ProjectedReadProjection,
        meter: &mut ProjectedReadMeter,
    ) -> Result<Option<PreparedRow>> {
        let ProjectedLocation::Segment {
            row_group,
            row,
            value_start,
            value_end,
        } = &mut entry.location
        else {
            return Err(Error::InvalidProjectedRead(
                "segment cursor received a memtable locator".into(),
            ));
        };
        let validity = self.segment.load_projected_page(
            *row_group,
            PageKind::ValueValidity,
            meter,
            self.cache_access,
        )?;
        if !valid_at(validity.buffer.as_slice(), *row)? {
            return Ok(None);
        }
        let value_len = if projection == ProjectedReadProjection::KeyValue {
            let offsets = self.segment.load_projected_page(
                *row_group,
                PageKind::ValueOffsets,
                meter,
                self.cache_access,
            )?;
            let start = offset_at(offsets.buffer.as_slice(), *row)?;
            let end = offset_at(offsets.buffer.as_slice(), *row + 1)?;
            if start > end {
                return Err(Error::InvalidSegment(
                    "projected value offsets are inverted".into(),
                ));
            }
            *value_start = Some(start);
            *value_end = Some(end);
            end - start
        } else {
            0
        };
        Ok(Some(PreparedRow {
            run: 0,
            key: entry.key,
            value_len,
            location: entry.location,
        }))
    }

    fn copy_value(
        &self,
        row: &PreparedRow,
        output: &mut [u8],
        meter: &mut ProjectedReadMeter,
    ) -> Result<()> {
        let ProjectedLocation::Segment {
            row_group,
            value_start: Some(start),
            value_end: Some(end),
            ..
        } = row.location
        else {
            return Err(Error::InvalidProjectedRead(
                "projected segment value has no authenticated byte range".into(),
            ));
        };
        let data = self.segment.load_projected_page(
            row_group,
            PageKind::ValueData,
            meter,
            self.cache_access,
        )?;
        let value = data
            .buffer
            .as_slice()
            .get(start..end)
            .ok_or_else(|| Error::InvalidSegment("projected value range is absent".into()))?;
        if value.len() != output.len() {
            return Err(Error::InvalidSegment(
                "projected value length changed after authenticated offset read".into(),
            ));
        }
        output.copy_from_slice(value);
        Ok(())
    }
}

impl Segment {
    fn load_projected_key_spine(
        &self,
        row_group: usize,
        meter: &mut ProjectedReadMeter,
        cache_access: PageCacheAccess,
    ) -> Result<KeySpine> {
        Ok(KeySpine {
            offsets: self.load_projected_page(
                row_group,
                PageKind::KeyOffsets,
                meter,
                cache_access,
            )?,
            data: self.load_projected_page(row_group, PageKind::KeyData, meter, cache_access)?,
            sequences: self.load_projected_page(
                row_group,
                PageKind::SequenceValues,
                meter,
                cache_access,
            )?,
        })
    }

    fn load_projected_page(
        &self,
        row_group: usize,
        kind: PageKind,
        meter: &mut ProjectedReadMeter,
        cache_access: PageCacheAccess,
    ) -> Result<Arc<super::LoadedPage>> {
        let descriptor = self
            .row_groups
            .get(row_group)
            .ok_or_else(|| Error::InvalidSegment("row-group index is outside the segment".into()))?
            .page(kind);
        meter.request_page(kind, descriptor.logical_bytes as u64)?;
        let (page, evidence) = self.load_page_with_access(row_group, kind, cache_access)?;
        meter.record_page_load(evidence)?;
        Ok(page)
    }
}

#[derive(Debug, Clone)]
struct ProjectedEntry {
    key: Vec<u8>,
    sequence: u64,
    location: ProjectedLocation,
}

#[derive(Debug, Clone, Copy)]
enum ProjectedLocation {
    Memtable,
    Segment {
        row_group: usize,
        row: usize,
        value_start: Option<usize>,
        value_end: Option<usize>,
    },
}

#[derive(Debug)]
struct PreparedRow {
    run: usize,
    key: Vec<u8>,
    value_len: usize,
    location: ProjectedLocation,
}

#[derive(Debug)]
struct RawSegmentRow {
    key: Vec<u8>,
    sequence: u64,
    row_group: usize,
    row: usize,
}

#[derive(Debug, Eq, PartialEq)]
struct HeapItem {
    key: Vec<u8>,
    run: usize,
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .key
            .cmp(&self.key)
            .then_with(|| other.run.cmp(&self.run))
    }
}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
struct ProjectedReadMeter {
    evidence: ProjectedReadEvidence,
    budget: ProjectedReadBudget,
}

impl ProjectedReadMeter {
    fn record_versions(&mut self, count: u64) -> Result<()> {
        let observed = checked_add(
            self.evidence.versions_examined,
            count,
            "projected version counter",
        )?;
        check_limit(
            ProjectedReadResource::VersionsExamined,
            self.budget.max_versions_examined,
            observed,
        )?;
        self.evidence.versions_examined = observed;
        Ok(())
    }

    fn record_selected_row_group(&mut self) -> Result<()> {
        self.evidence.selected_row_groups = checked_add(
            self.evidence.selected_row_groups,
            1,
            "selected row-group counter",
        )?;
        Ok(())
    }

    fn request_page(&mut self, kind: PageKind, logical_bytes: u64) -> Result<()> {
        let requests = checked_add(self.evidence.page_requests, 1, "page request counter")?;
        check_limit(
            ProjectedReadResource::PageRequests,
            self.budget.max_page_requests,
            requests,
        )?;
        let bytes = checked_add(
            self.evidence.page_logical_bytes,
            logical_bytes,
            "page logical-byte counter",
        )?;
        check_limit(
            ProjectedReadResource::PageLogicalBytes,
            self.budget.max_page_logical_bytes,
            bytes,
        )?;
        self.evidence.page_requests = requests;
        self.evidence.page_logical_bytes = bytes;
        let field = match kind {
            PageKind::KeyOffsets => &mut self.evidence.key_offset_page_requests,
            PageKind::KeyData => &mut self.evidence.key_data_page_requests,
            PageKind::SequenceValues => &mut self.evidence.sequence_page_requests,
            PageKind::ValueValidity => &mut self.evidence.validity_page_requests,
            PageKind::ValueOffsets => &mut self.evidence.value_offset_page_requests,
            PageKind::ValueData => &mut self.evidence.value_data_page_requests,
        };
        *field = checked_add(*field, 1, "page-family request counter")?;
        Ok(())
    }

    fn record_page_load(&mut self, page: PageLoadEvidence) -> Result<()> {
        self.evidence.cache_hits = checked_add(
            self.evidence.cache_hits,
            u64::from(page.cache_hit),
            "cache-hit counter",
        )?;
        self.evidence.cache_misses = checked_add(
            self.evidence.cache_misses,
            u64::from(page.cache_miss),
            "cache-miss counter",
        )?;
        self.evidence.cache_loads = checked_add(
            self.evidence.cache_loads,
            u64::from(page.cache_load),
            "cache-load counter",
        )?;
        match page.actual_io {
            Some(super::SelectedIo::Mmap) => {
                self.evidence.mmap_read_operations =
                    checked_add(self.evidence.mmap_read_operations, 1, "mmap read counter")?;
            }
            Some(super::SelectedIo::IoUring) => {
                self.evidence.io_uring_read_operations = checked_add(
                    self.evidence.io_uring_read_operations,
                    1,
                    "io_uring read counter",
                )?;
            }
            Some(super::SelectedIo::Bounded) => {
                self.evidence.bounded_read_operations = checked_add(
                    self.evidence.bounded_read_operations,
                    1,
                    "bounded read counter",
                )?;
            }
            None => {}
        }
        for (field, value, name) in [
            (
                &mut self.evidence.physical_bytes_read,
                page.physical_bytes,
                "physical read-byte counter",
            ),
            (
                &mut self.evidence.bytes_borrowed,
                page.borrowed_bytes,
                "borrowed-byte counter",
            ),
            (
                &mut self.evidence.bytes_decoded,
                page.decoded_bytes,
                "decoded-byte counter",
            ),
            (
                &mut self.evidence.bytes_decompressed,
                page.decompressed_bytes,
                "decompressed-byte counter",
            ),
            (
                &mut self.evidence.bytes_allocated,
                page.allocated_bytes,
                "allocated-byte counter",
            ),
            (
                &mut self.evidence.bytes_copied,
                page.copied_bytes,
                "copied-byte counter",
            ),
        ] {
            *field = checked_add(*field, value, name)?;
        }
        Ok(())
    }

    fn record_batch_allocation(&mut self, bytes: usize) -> Result<()> {
        check_limit(
            ProjectedReadResource::BatchAllocatedBytes,
            self.budget.max_batch_allocated_bytes as u64,
            bytes as u64,
        )?;
        self.evidence.bytes_allocated = checked_add(
            self.evidence.bytes_allocated,
            bytes as u64,
            "batch allocation counter",
        )?;
        self.evidence.bytes_copied = checked_add(
            self.evidence.bytes_copied,
            bytes as u64,
            "batch copy counter",
        )?;
        self.evidence.peak_batch_allocated_bytes =
            self.evidence.peak_batch_allocated_bytes.max(bytes as u64);
        Ok(())
    }

    fn record_emitted_batch(
        &mut self,
        rows: u64,
        key_bytes: u64,
        value_bytes: u64,
        buffer_bytes: u64,
    ) -> Result<()> {
        self.evidence.emitted_rows =
            checked_add(self.evidence.emitted_rows, rows, "emitted row counter")?;
        self.evidence.emitted_key_bytes = checked_add(
            self.evidence.emitted_key_bytes,
            key_bytes,
            "emitted key-byte counter",
        )?;
        self.evidence.emitted_value_bytes = checked_add(
            self.evidence.emitted_value_bytes,
            value_bytes,
            "emitted value-byte counter",
        )?;
        self.evidence.emitted_buffer_bytes = checked_add(
            self.evidence.emitted_buffer_bytes,
            buffer_bytes,
            "emitted buffer-byte counter",
        )?;
        self.evidence.emitted_batches =
            checked_add(self.evidence.emitted_batches, 1, "emitted batch counter")?;
        self.evidence.peak_batch_rows = self.evidence.peak_batch_rows.max(rows);
        Ok(())
    }
}

fn check_limit(resource: ProjectedReadResource, limit: u64, observed: u64) -> Result<()> {
    if observed > limit {
        return Err(Error::ProjectedReadLimit {
            resource,
            limit,
            observed,
        });
    }
    Ok(())
}

fn checked_add(left: u64, right: u64, name: &'static str) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| Error::InvalidProjectedRead(format!("{name} overflow")))
}

fn batch_allocation_bytes(
    rows: usize,
    key_bytes: usize,
    value_bytes: usize,
    projection: ProjectedReadProjection,
) -> Result<usize> {
    let offsets = rows
        .checked_add(1)
        .and_then(|count| count.checked_mul(8))
        .ok_or_else(|| Error::InvalidProjectedRead("projected offset bytes overflow".into()))?;
    let key = offsets
        .checked_add(key_bytes)
        .ok_or_else(|| Error::InvalidProjectedRead("projected key buffers overflow".into()))?;
    if projection == ProjectedReadProjection::Key {
        return Ok(key);
    }
    key.checked_add(offsets)
        .and_then(|bytes| bytes.checked_add(value_bytes))
        .ok_or_else(|| Error::InvalidProjectedRead("projected value buffers overflow".into()))
}

fn write_offset(output: &mut [u8], index: usize, value: usize) -> Result<()> {
    let value = i64::try_from(value)
        .map_err(|_| Error::InvalidProjectedRead("projected offset exceeds i64".into()))?;
    let offset = index
        .checked_mul(8)
        .ok_or_else(|| Error::InvalidProjectedRead("projected offset index overflow".into()))?;
    let target = output
        .get_mut(offset..offset + 8)
        .ok_or_else(|| Error::InvalidProjectedRead("projected offset output is absent".into()))?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn binary_value<'a>(
    offsets: &Buffer,
    data: &'a Buffer,
    row: usize,
    rows: usize,
) -> Option<&'a [u8]> {
    if row >= rows {
        return None;
    }
    let start = read_offset(offsets.as_slice(), row)?;
    let end = read_offset(offsets.as_slice(), row + 1)?;
    (start <= end)
        .then(|| data.as_slice().get(start..end))
        .flatten()
}

fn read_offset(offsets: &[u8], index: usize) -> Option<usize> {
    let start = index.checked_mul(8)?;
    let bytes: [u8; 8] = offsets.get(start..start + 8)?.try_into().ok()?;
    usize::try_from(i64::from_le_bytes(bytes)).ok()
}
