//! Projected entries, the merge heap, the read meter and value helpers.

use super::*;

#[derive(Debug, Clone)]
pub(super) struct ProjectedEntry {
    pub(super) key: Vec<u8>,
    pub(super) sequence: u64,
    pub(super) location: ProjectedLocation,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ProjectedLocation {
    Memtable,
    Segment {
        row_group: usize,
        row: usize,
        value_start: Option<usize>,
        value_end: Option<usize>,
    },
}

#[derive(Debug)]
pub(super) struct PreparedRow {
    pub(super) run: usize,
    pub(super) key: Vec<u8>,
    pub(super) value_len: usize,
    pub(super) location: ProjectedLocation,
}

#[derive(Debug)]
pub(super) struct RawSegmentRow {
    pub(super) key: Vec<u8>,
    pub(super) sequence: u64,
    pub(super) row_group: usize,
    pub(super) row: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct HeapItem {
    pub(super) key: Vec<u8>,
    pub(super) run: usize,
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
pub(super) struct ProjectedReadMeter {
    pub(super) evidence: ProjectedReadEvidence,
    pub(super) budget: ProjectedReadBudget,
}

impl ProjectedReadMeter {
    pub(super) fn record_versions(&mut self, count: u64) -> Result<()> {
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

    pub(super) fn record_selected_row_group(&mut self) -> Result<()> {
        self.evidence.selected_row_groups = checked_add(
            self.evidence.selected_row_groups,
            1,
            "selected row-group counter",
        )?;
        Ok(())
    }

    pub(super) fn request_page(&mut self, kind: PageKind, logical_bytes: u64) -> Result<()> {
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

    pub(super) fn record_page_load(&mut self, page: PageLoadEvidence) -> Result<()> {
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
            Some(super::super::SelectedIo::Mmap) => {
                self.evidence.mmap_read_operations =
                    checked_add(self.evidence.mmap_read_operations, 1, "mmap read counter")?;
            }
            Some(super::super::SelectedIo::IoUring) => {
                self.evidence.io_uring_read_operations = checked_add(
                    self.evidence.io_uring_read_operations,
                    1,
                    "io_uring read counter",
                )?;
            }
            Some(super::super::SelectedIo::Bounded) => {
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

    pub(super) fn record_batch_allocation(&mut self, bytes: usize) -> Result<()> {
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

    pub(super) fn record_emitted_batch(
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

pub(super) fn check_limit(
    resource: ProjectedReadResource,
    limit: u64,
    observed: u64,
) -> Result<()> {
    if observed > limit {
        return Err(Error::ProjectedReadLimit {
            resource,
            limit,
            observed,
        });
    }
    Ok(())
}

pub(super) fn checked_add(left: u64, right: u64, name: &'static str) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| Error::InvalidProjectedRead(format!("{name} overflow")))
}

pub(super) fn batch_allocation_bytes(
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

pub(super) fn write_offset(output: &mut [u8], index: usize, value: usize) -> Result<()> {
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

pub(super) fn binary_value<'a>(
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

pub(super) fn read_offset(offsets: &[u8], index: usize) -> Option<usize> {
    let start = index.checked_mul(8)?;
    let bytes: [u8; 8] = offsets.get(start..start + 8)?.try_into().ok()?;
    usize::try_from(i64::from_le_bytes(bytes)).ok()
}
