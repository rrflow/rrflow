//! Writing segments from a memtable or version stream, and physical file checks.

use super::*;

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
        Self::write_encoded_with_cache(
            directory,
            encode(table, row_group_budget, compression_policy)?,
            compression_policy,
            cache,
            io,
        )
    }

    pub(crate) fn write_from_versions_with_cache<'a>(
        directory: &AnchoredPath,
        versions: impl Iterator<Item = (&'a [u8], &'a [VersionedValue])> + Clone,
        row_group_budget: SegmentRowGroupBudget,
        compression_policy: SegmentCompressionPolicy,
        cache: SharedPageCache,
        io: SharedIoContext,
    ) -> Result<(Self, AnchoredPath)> {
        Self::write_encoded_with_cache(
            directory,
            format::encode_versions(versions, row_group_budget, compression_policy)?,
            compression_policy,
            cache,
            io,
        )
    }

    pub(super) fn write_encoded_with_cache(
        directory: &AnchoredPath,
        encoded: format::EncodedSegment,
        compression_policy: SegmentCompressionPolicy,
        cache: SharedPageCache,
        io: SharedIoContext,
    ) -> Result<(Self, AnchoredPath)> {
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
}

pub(super) fn build_segment(
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

pub(super) fn validate_physical_file(mut file: &File) -> Result<u64> {
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
