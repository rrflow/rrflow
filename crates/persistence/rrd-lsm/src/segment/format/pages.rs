//! Page encoding, descriptors, compression checks and shape validation.

use super::*;

pub(super) fn append_page(
    output: &mut Vec<u8>,
    metadata: PageWriteMetadata,
    bytes: &[u8],
    compression_policy: SegmentCompressionPolicy,
    scratch: &mut Vec<u8>,
    evidence: &mut SegmentWriteEvidence,
) -> Result<PageDescriptor> {
    align(output);
    let offset = output.len() as u64;
    let logical_bytes = u64::try_from(bytes.len())
        .map_err(|_| Error::InvalidSegment("v6 logical page length exceeds u64".into()))?;
    evidence.logical_page_bytes = evidence
        .logical_page_bytes
        .checked_add(logical_bytes)
        .ok_or_else(|| Error::InvalidSegment("v6 logical page evidence overflow".into()))?;

    let (compression, stored) = match compression_policy {
        SegmentCompressionPolicy::None => (PageCompression::None, bytes),
        SegmentCompressionPolicy::AdaptiveLz4 {
            minimum_savings_basis_points,
            maximum_page_logical_bytes,
        } if !bytes.is_empty() && logical_bytes <= maximum_page_logical_bytes => {
            let maximum_output = lz4_flex::block::get_maximum_output_size(bytes.len());
            if scratch.len() < maximum_output {
                scratch
                    .try_reserve_exact(maximum_output - scratch.len())
                    .map_err(|_| {
                        Error::InvalidSegment("v6 LZ4 compression scratch allocation failed".into())
                    })?;
                scratch.resize(maximum_output, 0);
            }
            evidence.compression_scratch_high_water_bytes = evidence
                .compression_scratch_high_water_bytes
                .max(u64::try_from(scratch.capacity()).map_err(|_| {
                    Error::InvalidSegment("v6 LZ4 compression scratch exceeds u64".into())
                })?);
            let compressed_bytes =
                lz4_flex::block::compress_into(bytes, &mut scratch[..maximum_output]).map_err(
                    |error| Error::InvalidSegment(format!("v6 LZ4 encode failed: {error}")),
                )?;
            let compressed = scratch.get(..compressed_bytes).ok_or_else(|| {
                Error::InvalidSegment("v6 LZ4 encoder returned an invalid length".into())
            })?;
            if compression_saves(
                logical_bytes,
                u64::try_from(compressed_bytes).map_err(|_| {
                    Error::InvalidSegment("v6 compressed page length exceeds u64".into())
                })?,
                minimum_savings_basis_points,
            )? {
                (PageCompression::Lz4Block, compressed)
            } else {
                (PageCompression::None, bytes)
            }
        }
        SegmentCompressionPolicy::AdaptiveLz4 { .. } => (PageCompression::None, bytes),
    };
    output.extend_from_slice(stored);
    let stored_bytes = u64::try_from(stored.len())
        .map_err(|_| Error::InvalidSegment("v6 stored page length exceeds u64".into()))?;
    evidence.stored_page_bytes = evidence
        .stored_page_bytes
        .checked_add(stored_bytes)
        .ok_or_else(|| Error::InvalidSegment("v6 stored page evidence overflow".into()))?;
    match compression {
        PageCompression::None => {
            evidence.raw_page_count = evidence
                .raw_page_count
                .checked_add(1)
                .ok_or_else(|| Error::InvalidSegment("v6 raw page count overflow".into()))?;
        }
        PageCompression::Lz4Block => {
            evidence.compressed_page_count = evidence
                .compressed_page_count
                .checked_add(1)
                .ok_or_else(|| Error::InvalidSegment("v6 compressed page count overflow".into()))?;
        }
    }
    Ok(PageDescriptor {
        kind: metadata.kind,
        compression,
        row_start: metadata.row_start,
        row_count: metadata.row_count,
        null_count: metadata.statistics.null_count,
        offset,
        physical_bytes: stored.len(),
        logical_bytes: bytes.len(),
        statistic_min: metadata.statistics.minimum,
        statistic_max: metadata.statistics.maximum,
        digest: sha256(stored),
    })
}

pub(super) fn encode_page_descriptor(output: &mut Vec<u8>, page: &PageDescriptor) {
    output.push(page.kind as u8);
    output.push(page.kind.column());
    output.push(page.kind.buffer_kind());
    output.push(page.kind.logical_type());
    output.push(page.kind.physical_type());
    output.push(0); // plain Arrow-layout encoding
    output.push(page.compression as u8);
    output.push(0);
    output.extend_from_slice(&page.row_start.to_le_bytes());
    output.extend_from_slice(&page.row_count.to_le_bytes());
    output.extend_from_slice(&page.null_count.to_le_bytes());
    output.extend_from_slice(&page.offset.to_le_bytes());
    output.extend_from_slice(&(page.physical_bytes as u64).to_le_bytes());
    output.extend_from_slice(&(page.logical_bytes as u64).to_le_bytes());
    output.extend_from_slice(&page.statistic_min.to_le_bytes());
    output.extend_from_slice(&page.statistic_max.to_le_bytes());
    output.extend_from_slice(&page.digest);
}

pub(super) fn decode_page_descriptor(
    bytes: &[u8],
    compression_policy: SegmentCompressionPolicy,
) -> Result<PageDescriptor> {
    let kind = PageKind::from_byte(bytes[0])?;
    if bytes[1] != kind.column()
        || bytes[2] != kind.buffer_kind()
        || bytes[3] != kind.logical_type()
        || bytes[4] != kind.physical_type()
        || bytes[5] != 0
        || bytes[7] != 0
    {
        return invalid("v6 page type, encoding, or reserved metadata is unknown");
    }
    let compression = PageCompression::from_byte(bytes[6])?;
    let physical_bytes = usize::try_from(read_u64(bytes, 32)?)
        .map_err(|_| Error::InvalidSegment("v6 physical page length exceeds usize".into()))?;
    let logical_bytes = usize::try_from(read_u64(bytes, 40)?)
        .map_err(|_| Error::InvalidSegment("v6 logical page length exceeds usize".into()))?;
    validate_page_compression(
        compression_policy,
        compression,
        u64::try_from(physical_bytes)
            .map_err(|_| Error::InvalidSegment("v6 physical page length exceeds u64".into()))?,
        u64::try_from(logical_bytes)
            .map_err(|_| Error::InvalidSegment("v6 logical page length exceeds u64".into()))?,
    )?;
    Ok(PageDescriptor {
        kind,
        compression,
        row_start: read_u64(bytes, 8)?,
        row_count: read_u32(bytes, 16)?,
        null_count: read_u32(bytes, 20)?,
        offset: read_u64(bytes, 24)?,
        physical_bytes,
        logical_bytes,
        statistic_min: read_u64(bytes, 48)?,
        statistic_max: read_u64(bytes, 56)?,
        digest: bytes[64..96].try_into().expect("fixed page digest"),
    })
}

pub(super) fn compression_saves(logical: u64, stored: u64, minimum_savings: u16) -> Result<bool> {
    if stored >= logical {
        return Ok(false);
    }
    let stored_scaled = u128::from(stored)
        .checked_mul(u128::from(COMPRESSION_BASIS_POINTS))
        .ok_or_else(|| Error::InvalidSegment("v6 compression threshold overflow".into()))?;
    let retained_basis_points = COMPRESSION_BASIS_POINTS
        .checked_sub(minimum_savings)
        .ok_or_else(|| Error::InvalidSegment("v6 compression threshold is invalid".into()))?;
    let logical_scaled = u128::from(logical)
        .checked_mul(u128::from(retained_basis_points))
        .ok_or_else(|| Error::InvalidSegment("v6 compression threshold overflow".into()))?;
    Ok(stored_scaled <= logical_scaled)
}

pub(super) fn validate_page_compression(
    policy: SegmentCompressionPolicy,
    compression: PageCompression,
    physical_bytes: u64,
    logical_bytes: u64,
) -> Result<()> {
    match (policy, compression) {
        (_, PageCompression::None) if physical_bytes == logical_bytes => Ok(()),
        (_, PageCompression::None) => {
            invalid("v6 raw page has different physical and logical lengths")
        }
        (SegmentCompressionPolicy::None, PageCompression::Lz4Block) => {
            invalid("v6 none policy cannot contain an LZ4 page")
        }
        (
            SegmentCompressionPolicy::AdaptiveLz4 {
                minimum_savings_basis_points,
                maximum_page_logical_bytes,
            },
            PageCompression::Lz4Block,
        ) if physical_bytes > 0
            && logical_bytes <= maximum_page_logical_bytes
            && compression_saves(logical_bytes, physical_bytes, minimum_savings_basis_points)? =>
        {
            Ok(())
        }
        (SegmentCompressionPolicy::AdaptiveLz4 { .. }, PageCompression::Lz4Block) => {
            invalid("v6 LZ4 page violates its authenticated lengths or saving policy")
        }
    }
}

pub(super) fn validate_page_shapes(pages: &[PageDescriptor], rows: u32) -> Result<()> {
    let rows = rows as usize;
    let expected_offsets = rows
        .checked_add(1)
        .and_then(|count| count.checked_mul(8))
        .ok_or_else(|| Error::InvalidSegment("v6 offset page length overflow".into()))?;
    if pages[0].logical_bytes != expected_offsets
        || pages[2].logical_bytes != rows.saturating_mul(8)
        || pages[3].logical_bytes != rows.div_ceil(8)
        || pages[4].logical_bytes != expected_offsets
        || pages.iter().any(|page| page.null_count > page.row_count)
        || pages[0].null_count != 0
        || pages[1].null_count != 0
        || pages[2].null_count != 0
        || pages[3].null_count != pages[4].null_count
        || pages[3].null_count != pages[5].null_count
    {
        return invalid("v6 Arrow-compatible page lengths or null counts are invalid");
    }
    Ok(())
}

pub(super) fn validate_zero_padding(
    reader: &mut (impl Read + Seek),
    row_groups: &[RowGroupDescriptor],
    index_offset: u64,
) -> Result<u64> {
    let mut expected = SEGMENT_HEADER_BYTES as u64;
    let mut bytes_read = 0u64;
    for page in row_groups.iter().flat_map(|group| group.pages.iter()) {
        bytes_read = bytes_read
            .checked_add(validate_zero_range(reader, expected, page.offset)?)
            .ok_or_else(|| Error::InvalidSegment("v6 padding read count overflow".into()))?;
        expected = page
            .offset
            .checked_add(page.physical_bytes as u64)
            .ok_or_else(|| Error::InvalidSegment("v6 page end overflow".into()))?;
    }
    bytes_read
        .checked_add(validate_zero_range(reader, expected, index_offset)?)
        .ok_or_else(|| Error::InvalidSegment("v6 padding read count overflow".into()))
}

pub(super) fn validate_zero_range(
    reader: &mut (impl Read + Seek),
    start: u64,
    end: u64,
) -> Result<u64> {
    let length = usize::try_from(
        end.checked_sub(start)
            .ok_or_else(|| Error::InvalidSegment("v6 padding range is inverted".into()))?,
    )
    .map_err(|_| Error::InvalidSegment("v6 padding range exceeds usize".into()))?;
    if length >= PAGE_ALIGNMENT {
        return invalid("v6 page padding exceeds the alignment contract");
    }
    if length == 0 {
        return Ok(0);
    }
    let mut padding = [0; PAGE_ALIGNMENT - 1];
    reader.seek(SeekFrom::Start(start))?;
    reader.read_exact(&mut padding[..length])?;
    if padding[..length].iter().any(|byte| *byte != 0) {
        return invalid("v6 page alignment padding is non-zero");
    }
    Ok(length as u64)
}

pub(super) fn validate_key_value(key: &[u8], value: &[u8]) -> Result<()> {
    if key.is_empty() || key.len() > MAX_KEY_BYTES || value.len() > MAX_VALUE_BYTES {
        return invalid("key or value exceeds the v6 segment contract");
    }
    Ok(())
}
