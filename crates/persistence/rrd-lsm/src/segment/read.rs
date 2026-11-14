//! Point, version, range and cursor reads within a segment.

use super::*;

impl Segment {
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
        let Some(selected) = self.select_version(key, read_sequence)? else {
            return Ok(None);
        };
        Ok(Some(SegmentVersion {
            sequence: selected.sequence(),
            value: selected.value()?.map(Box::<[u8]>::from),
        }))
    }

    // Conflict validation needs presence and sequence, including tombstones,
    // but must not load/copy payload columns simply to discard their values.
    pub(crate) fn get_sequence(&self, key: &[u8], read_sequence: u64) -> Result<Option<u64>> {
        Ok(self
            .select_version(key, read_sequence)?
            .map(|selected| selected.sequence()))
    }

    pub(crate) fn select_version(
        &self,
        key: &[u8],
        read_sequence: u64,
    ) -> Result<Option<SelectedVersion<'_>>> {
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
        Ok(selected.map(|(row, sequence)| SelectedVersion {
            segment: self,
            row_group: index,
            row,
            sequence,
        }))
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

    pub(super) fn load_key_spine(&self, row_group: usize) -> Result<KeySpine> {
        Ok(KeySpine {
            offsets: self.load_page(row_group, PageKind::KeyOffsets)?,
            data: self.load_page(row_group, PageKind::KeyData)?,
            sequences: self.load_page(row_group, PageKind::SequenceValues)?,
        })
    }

    pub(super) fn load_row_group(&self, row_group: usize) -> Result<LoadedRowGroup> {
        Ok(LoadedRowGroup {
            spine: self.load_key_spine(row_group)?,
            validity: self.load_page(row_group, PageKind::ValueValidity)?,
            value_offsets: self.load_page(row_group, PageKind::ValueOffsets)?,
            values: self.load_page(row_group, PageKind::ValueData)?,
        })
    }

    pub(super) fn read_row_group_uncached(&self, row_group: usize) -> Result<LoadedRowGroup> {
        let descriptor = self.row_groups.get(row_group).ok_or_else(|| {
            Error::InvalidSegment("row-group index is outside the segment".into())
        })?;
        Ok(LoadedRowGroup {
            spine: KeySpine {
                offsets: Arc::new(read_page(
                    &self.source,
                    descriptor.page(PageKind::KeyOffsets),
                    &self.cache,
                )?),
                data: Arc::new(read_page(
                    &self.source,
                    descriptor.page(PageKind::KeyData),
                    &self.cache,
                )?),
                sequences: Arc::new(read_page(
                    &self.source,
                    descriptor.page(PageKind::SequenceValues),
                    &self.cache,
                )?),
            },
            validity: Arc::new(read_page(
                &self.source,
                descriptor.page(PageKind::ValueValidity),
                &self.cache,
            )?),
            value_offsets: Arc::new(read_page(
                &self.source,
                descriptor.page(PageKind::ValueOffsets),
                &self.cache,
            )?),
            values: Arc::new(read_page(
                &self.source,
                descriptor.page(PageKind::ValueData),
                &self.cache,
            )?),
        })
    }

    pub(super) fn value_at(&self, row_group: usize, row: usize) -> Result<Option<Vec<u8>>> {
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

    pub(super) fn load_page(&self, row_group: usize, kind: PageKind) -> Result<Arc<LoadedPage>> {
        self.load_page_with_evidence(row_group, kind)
            .map(|(page, _)| page)
    }

    pub(super) fn load_page_with_evidence(
        &self,
        row_group: usize,
        kind: PageKind,
    ) -> Result<(Arc<LoadedPage>, PageLoadEvidence)> {
        self.load_page_with_access(row_group, kind, PageCacheAccess::OrdinaryOperation)
    }

    pub(super) fn load_page_with_access(
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
        let loaded = Arc::new(read_page(&self.source, descriptor, &self.cache)?);
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

    pub(super) fn load_value_columns(&self, row_group: usize) -> Result<LoadedValueColumns> {
        let _descriptor = self.row_groups.get(row_group).ok_or_else(|| {
            Error::InvalidSegment("row-group index is outside the segment".into())
        })?;
        Ok(LoadedValueColumns {
            validity: self.load_page(row_group, PageKind::ValueValidity)?,
            offsets: self.load_page(row_group, PageKind::ValueOffsets)?,
            values: self.load_page(row_group, PageKind::ValueData)?,
        })
    }
}
