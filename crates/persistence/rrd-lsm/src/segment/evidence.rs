//! Evidence recorded when a segment is opened and validated.

use super::*;

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
    pub(super) fn for_metadata(
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
