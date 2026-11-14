//! Opening, validating and installing segments.

use super::*;

impl Segment {
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
        let source_file = io.open_segment(path)?;
        let lease = source_file.lease()?;
        let mut file = &*lease;
        let physical_bytes = validate_physical_file(file)?;
        let total_started = Instant::now();
        let verify_started = Instant::now();
        let checksum = verify_file_digest(file, physical_bytes)?;
        let verify_ms = verify_started.elapsed().as_millis() as u64;
        let metadata_started = Instant::now();
        let metadata = parse_metadata(&mut file, physical_bytes, checksum)?;
        let metadata_ms = metadata_started.elapsed().as_millis() as u64;
        let open_evidence = SegmentOpenEvidence::for_metadata(physical_bytes, &metadata, true)?;
        let source_started = Instant::now();
        drop(lease);
        let source = open_page_source(source_file, io)?;
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
        let source_file = io.open_segment(path)?;
        let lease = source_file.lease()?;
        let mut file = &*lease;
        let physical_bytes = validate_physical_file(file)?;
        if physical_bytes != expected.bytes {
            return invalid("segment physical size differs from its manifest descriptor");
        }
        let checksum = verify_file_digest(file, physical_bytes)?;
        if checksum != expected.checksum || checksum != expected.id {
            return invalid("segment content digest differs from its manifest descriptor");
        }
        let metadata = parse_metadata(&mut file, physical_bytes, checksum)?;
        let open_evidence = SegmentOpenEvidence::for_metadata(physical_bytes, &metadata, false)?;
        drop(lease);
        let source = open_page_source(source_file, io)?;
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

    pub(super) fn validate_all_pages(&self) -> Result<()> {
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
