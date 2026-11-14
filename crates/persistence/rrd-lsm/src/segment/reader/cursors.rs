//! Run, memtable and segment projected cursors.

use super::*;

#[derive(Debug)]
pub(super) enum RunCursor {
    Memtable(MemtableProjectedCursor),
    Segment(SegmentProjectedCursor),
}

impl RunCursor {
    pub(super) fn next_candidate(
        &mut self,
        meter: &mut ProjectedReadMeter,
    ) -> Result<Option<ProjectedEntry>> {
        match self {
            Self::Memtable(cursor) => cursor.next_candidate(meter),
            Self::Segment(cursor) => cursor.next_candidate(meter),
        }
    }

    pub(super) fn prepare_value(
        &mut self,
        entry: ProjectedEntry,
        projection: ProjectedReadProjection,
        meter: &mut ProjectedReadMeter,
    ) -> Result<Option<PreparedRow>> {
        match self {
            Self::Memtable(cursor) => cursor.prepare_value(entry, projection),
            Self::Segment(cursor) => cursor.prepare_value(entry, projection, meter),
        }
    }

    pub(super) fn copy_value(
        &mut self,
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
pub(super) struct MemtableProjectedCursor {
    pub(super) table: Arc<crate::mutable_memory::MutableGeneration>,
    pub(super) ranges: Arc<[ProjectedReadRange]>,
    pub(super) read_sequence: u64,
    pub(super) after: Option<Vec<u8>>,
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
pub(super) struct SegmentProjectedCursor {
    pub(super) segment: Arc<Segment>,
    pub(super) ranges: Arc<[ProjectedReadRange]>,
    pub(super) read_sequence: u64,
    pub(super) cache_access: PageCacheAccess,
    pub(super) row_group: usize,
    pub(super) row: usize,
    pub(super) loaded_spine: Option<KeySpine>,
    // The row group each value page was last charged for. Rows reach each kind in
    // row-group order, so a row group's rows pay for its page's bytes once, as the
    // spine does. Each row still counts a request and touches the cache.
    pub(super) charged_validity: Option<usize>,
    pub(super) charged_offsets: Option<usize>,
    pub(super) charged_data: Option<usize>,
    pub(super) lookahead: Option<RawSegmentRow>,
}

impl SegmentProjectedCursor {
    pub(super) fn new(
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
            charged_validity: None,
            charged_offsets: None,
            charged_data: None,
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
        &mut self,
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
        let validity = self.segment.load_projected_value_page(
            &mut self.charged_validity,
            *row_group,
            PageKind::ValueValidity,
            meter,
            self.cache_access,
        )?;
        if !valid_at(validity.buffer.as_slice(), *row)? {
            return Ok(None);
        }
        let value_len = if projection == ProjectedReadProjection::KeyValue {
            let offsets = self.segment.load_projected_value_page(
                &mut self.charged_offsets,
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
        &mut self,
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
        let data = self.segment.load_projected_value_page(
            &mut self.charged_data,
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

    fn load_projected_value_page(
        &self,
        charged: &mut Option<usize>,
        row_group: usize,
        kind: PageKind,
        meter: &mut ProjectedReadMeter,
        cache_access: PageCacheAccess,
    ) -> Result<Arc<super::super::LoadedPage>> {
        let first_visit = *charged != Some(row_group);
        *charged = Some(row_group);
        self.load_metered_page(row_group, kind, first_visit, meter, cache_access)
    }

    fn load_projected_page(
        &self,
        row_group: usize,
        kind: PageKind,
        meter: &mut ProjectedReadMeter,
        cache_access: PageCacheAccess,
    ) -> Result<Arc<super::super::LoadedPage>> {
        self.load_metered_page(row_group, kind, true, meter, cache_access)
    }

    fn load_metered_page(
        &self,
        row_group: usize,
        kind: PageKind,
        charge_bytes: bool,
        meter: &mut ProjectedReadMeter,
        cache_access: PageCacheAccess,
    ) -> Result<Arc<super::super::LoadedPage>> {
        let descriptor = self
            .row_groups
            .get(row_group)
            .ok_or_else(|| Error::InvalidSegment("row-group index is outside the segment".into()))?
            .page(kind);
        let logical_bytes = if charge_bytes {
            descriptor.logical_bytes as u64
        } else {
            0
        };
        meter.request_page(kind, logical_bytes)?;
        let (page, evidence) = self.load_page_with_access(row_group, kind, cache_access)?;
        meter.record_page_load(evidence)?;
        Ok(page)
    }
}
