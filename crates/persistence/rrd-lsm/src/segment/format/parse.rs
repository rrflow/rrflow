//! Parsing and validating segment metadata, compression policy and row-group budget.

use super::*;

pub(in super::super) fn parse_metadata(
    reader: &mut (impl Read + Seek),
    physical_bytes: u64,
    checksum: String,
) -> Result<ParsedMetadata> {
    if physical_bytes < (SEGMENT_HEADER_BYTES + INDEX_HEADER_BYTES + FOOTER_BYTES) as u64 {
        return invalid("v6 segment is shorter than its framing");
    }
    let mut header = [0; SEGMENT_HEADER_BYTES];
    reader.seek(SeekFrom::Start(0))?;
    reader.read_exact(&mut header)?;
    require_current_format(&header)?;
    if read_u16(&header, 10)? as usize != SEGMENT_HEADER_BYTES
        || read_u32(&header, 12)? != 0
        || read_u16(&header, 44)? as usize != PAGES_PER_ROW_GROUP
        || read_u16(&header, 46)? as usize != PAGE_ALIGNMENT
        || header[188..].iter().any(|byte| *byte != 0)
    {
        return invalid("v6 segment header contains unsupported framing or flags");
    }
    let entries = read_u64(&header, 16)?;
    let minimum_sequence = read_u64(&header, 24)?;
    let maximum_sequence = read_u64(&header, 32)?;
    let row_group_count = read_u32(&header, 40)? as usize;
    let index_offset = read_u64(&header, 48)?;
    let index_bytes = usize::try_from(read_u64(&header, 56)?)
        .map_err(|_| Error::InvalidSegment("v6 index length exceeds usize".into()))?;
    let row_group_budget = decode_row_group_budget(&header)?;
    let compression_policy = decode_compression_policy(&header)?;
    let content_end = physical_bytes - FOOTER_BYTES as u64;
    if entries == 0
        || minimum_sequence == 0
        || minimum_sequence > maximum_sequence
        || row_group_count == 0
        || index_offset < SEGMENT_HEADER_BYTES as u64
        || index_offset % PAGE_ALIGNMENT as u64 != 0
        || !(INDEX_HEADER_BYTES..=MAX_INDEX_BYTES).contains(&index_bytes)
        || index_offset.checked_add(index_bytes as u64) != Some(content_end)
    {
        return invalid("v6 segment header contract is invalid");
    }
    if encode_digest(&header[64..96]) != SEGMENT_SCHEMA_DIGEST
        || encode_digest(&header[96..128]) != SEGMENT_KEY_CODEC_DIGEST
        || encode_digest(&header[128..160]) != SEGMENT_PAGE_FORMAT_DIGEST
    {
        return invalid("v6 segment schema, key codec, or page format digest is unknown");
    }

    reader.seek(SeekFrom::Start(index_offset))?;
    let mut index = vec![0; index_bytes];
    reader.read_exact(&mut index)?;
    if index.get(..8) != Some(INDEX_MAGIC.as_slice())
        || read_u32(&index, 8)? as usize != row_group_count
        || read_u16(&index, 12)? as usize != PAGES_PER_ROW_GROUP
        || read_u16(&index, 14)? as usize != ROW_GROUP_HEADER_BYTES
        || read_u16(&index, 16)? as usize != PAGE_DESCRIPTOR_BYTES
        || index[18] != FILTER_FORMAT_VERSION
        || index[19] as usize != FILTER_BITS_PER_KEY
        || index[20] as usize != FILTER_HASH_FUNCTIONS
        || index[21..32].iter().any(|byte| *byte != 0)
    {
        return invalid("v6 segment index header or filter policy is invalid");
    }

    let minimum_group_bytes = ROW_GROUP_HEADER_BYTES
        .checked_add(PAGES_PER_ROW_GROUP * PAGE_DESCRIPTOR_BYTES)
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<u64>()))
        .expect("fixed v6 row-group metadata size fits usize");
    if row_group_count
        > index_bytes
            .saturating_sub(INDEX_HEADER_BYTES)
            .checked_div(minimum_group_bytes)
            .unwrap_or(0)
    {
        return invalid("v6 row-group count cannot fit in the bounded index");
    }

    let mut cursor = INDEX_HEADER_BYTES;
    let mut row_groups = Vec::with_capacity(row_group_count);
    let mut expected_row_start = 0u64;
    let mut prior_last_key: Option<Vec<u8>> = None;
    let mut prior_page_end = SEGMENT_HEADER_BYTES as u64;
    for _ in 0..row_group_count {
        let fixed = slice(
            &index,
            cursor,
            ROW_GROUP_HEADER_BYTES,
            "v6 row-group header",
        )?;
        let row_start = read_u64(fixed, 0)?;
        let row_count = read_u32(fixed, 8)?;
        let first_key_bytes = read_u32(fixed, 12)? as usize;
        let last_key_bytes = read_u32(fixed, 16)? as usize;
        let unique_key_count = read_u32(fixed, 24)?;
        let filter_word_count = read_u32(fixed, 28)? as usize;
        if row_start != expected_row_start
            || row_count == 0
            || read_u16(fixed, 20)? as usize != PAGES_PER_ROW_GROUP
            || fixed[22..24].iter().any(|byte| *byte != 0)
            || unique_key_count == 0
            || unique_key_count > row_count
            || filter_word_count != RowGroupFilter::word_count(unique_key_count)?
            || first_key_bytes == 0
            || first_key_bytes > MAX_KEY_BYTES
            || last_key_bytes == 0
            || last_key_bytes > MAX_KEY_BYTES
        {
            return invalid("v6 row-group framing is invalid");
        }
        cursor += ROW_GROUP_HEADER_BYTES;
        let first_key = slice(&index, cursor, first_key_bytes, "v6 first key")?.to_vec();
        cursor += first_key_bytes;
        let last_key = slice(&index, cursor, last_key_bytes, "v6 last key")?.to_vec();
        cursor += last_key_bytes;
        if first_key > last_key
            || prior_last_key
                .as_ref()
                .is_some_and(|prior| prior >= &first_key)
        {
            return invalid("v6 row-group key ranges are not strictly ordered");
        }
        let mut pages = Vec::with_capacity(PAGES_PER_ROW_GROUP);
        for expected_kind in PageKind::ORDERED {
            let encoded = slice(&index, cursor, PAGE_DESCRIPTOR_BYTES, "v6 page descriptor")?;
            let page = decode_page_descriptor(encoded, compression_policy)?;
            if page.kind != expected_kind
                || page.row_start != row_start
                || page.row_count != row_count
                || page.offset % PAGE_ALIGNMENT as u64 != 0
                || page.offset < prior_page_end
                || page.offset.saturating_sub(prior_page_end) >= PAGE_ALIGNMENT as u64
                || page
                    .offset
                    .checked_add(page.physical_bytes as u64)
                    .is_none_or(|end| end > index_offset)
            {
                return invalid("v6 page descriptor violates ordering or bounds");
            }
            prior_page_end = page.offset + page.physical_bytes as u64;
            pages.push(page);
            cursor += PAGE_DESCRIPTOR_BYTES;
        }
        let filter_bytes = filter_word_count
            .checked_mul(std::mem::size_of::<u64>())
            .ok_or_else(|| Error::InvalidSegment("v6 filter byte count overflow".into()))?;
        let encoded_filter = slice(&index, cursor, filter_bytes, "v6 row-group filter")?;
        let filter = RowGroupFilter::from_words(
            unique_key_count,
            encoded_filter
                .as_chunks::<8>()
                .0
                .iter()
                .map(|word| u64::from_le_bytes(*word))
                .collect(),
        )?;
        cursor = cursor
            .checked_add(filter_bytes)
            .ok_or_else(|| Error::InvalidSegment("v6 filter cursor overflow".into()))?;
        validate_page_shapes(&pages, row_count)?;
        expected_row_start = expected_row_start
            .checked_add(u64::from(row_count))
            .ok_or_else(|| Error::InvalidSegment("v6 row count overflow".into()))?;
        prior_last_key = Some(last_key.clone());
        row_groups.push(RowGroupDescriptor {
            row_start,
            row_count,
            unique_key_count,
            first_key,
            last_key,
            pages: pages.try_into().expect("v6 page count was fixed"),
            filter,
        });
    }
    if cursor != index.len() || expected_row_start != entries {
        return invalid("v6 index does not exactly describe all rows");
    }
    validate_logical_envelope(&row_groups, index_bytes as u64)?;
    let padding_bytes_read = validate_zero_padding(reader, &row_groups, index_offset)?;
    let first_key = row_groups
        .first()
        .expect("validated v6 segment has a row group")
        .first_key
        .clone();
    let last_key = row_groups
        .last()
        .expect("validated v6 segment has a row group")
        .last_key
        .clone();
    Ok(ParsedMetadata {
        descriptor: SegmentDescriptor {
            id: checksum.clone(),
            level: 0,
            format_version: SEGMENT_FORMAT_VERSION,
            schema_digest: SEGMENT_SCHEMA_DIGEST.into(),
            key_codec_digest: SEGMENT_KEY_CODEC_DIGEST.into(),
            page_format_digest: SEGMENT_PAGE_FORMAT_DIGEST.into(),
            first_key,
            last_key,
            minimum_sequence,
            maximum_sequence,
            entries,
            bytes: physical_bytes,
            checksum,
        },
        row_group_budget,
        compression_policy,
        row_groups,
        header_bytes_read: SEGMENT_HEADER_BYTES as u64,
        index_bytes_read: index_bytes as u64,
        padding_bytes_read,
    })
}

pub(super) fn encode_compression_policy(output: &mut [u8], policy: SegmentCompressionPolicy) {
    match policy {
        SegmentCompressionPolicy::None => {
            output[176] = 0;
            output[177] = 0;
            output[178..180].copy_from_slice(&0u16.to_le_bytes());
            output[180..188].copy_from_slice(&0u64.to_le_bytes());
        }
        SegmentCompressionPolicy::AdaptiveLz4 {
            minimum_savings_basis_points,
            maximum_page_logical_bytes,
        } => {
            output[176] = 1;
            output[177] = PageCompression::Lz4Block as u8;
            output[178..180].copy_from_slice(&minimum_savings_basis_points.to_le_bytes());
            output[180..188].copy_from_slice(&maximum_page_logical_bytes.to_le_bytes());
        }
    }
}

pub(super) fn decode_compression_policy(header: &[u8]) -> Result<SegmentCompressionPolicy> {
    let minimum_savings_basis_points = read_u16(header, 178)?;
    let maximum_page_logical_bytes = read_u64(header, 180)?;
    match (
        header[176],
        header[177],
        minimum_savings_basis_points,
        maximum_page_logical_bytes,
    ) {
        (0, 0, 0, 0) => Ok(SegmentCompressionPolicy::None),
        (1, 1, minimum_savings_basis_points, maximum_page_logical_bytes) => {
            SegmentCompressionPolicy::AdaptiveLz4 {
                minimum_savings_basis_points,
                maximum_page_logical_bytes,
            }
            .validate()
            .map_err(|_| Error::InvalidSegment("v6 compression policy is invalid".into()))
        }
        _ => invalid("v6 compression policy metadata is unknown or noncanonical"),
    }
}

pub(super) fn validate_logical_envelope(
    row_groups: &[RowGroupDescriptor],
    index_bytes: u64,
) -> Result<()> {
    let mut cursor = SEGMENT_HEADER_BYTES as u64;
    for page in row_groups.iter().flat_map(|group| group.pages.iter()) {
        cursor = align_u64(cursor)?;
        cursor = cursor
            .checked_add(
                u64::try_from(page.logical_bytes).map_err(|_| {
                    Error::InvalidSegment("v6 logical page length exceeds u64".into())
                })?,
            )
            .ok_or_else(|| Error::InvalidSegment("v6 logical segment length overflow".into()))?;
    }
    cursor = align_u64(cursor)?;
    cursor = cursor
        .checked_add(index_bytes)
        .and_then(|bytes| bytes.checked_add(FOOTER_BYTES as u64))
        .ok_or_else(|| Error::InvalidSegment("v6 logical segment envelope overflow".into()))?;
    if cursor > MAX_SEGMENT_BYTES {
        return invalid("v6 reconstructed logical segment exceeds the 1 GiB safety limit");
    }
    Ok(())
}

pub(super) fn align_u64(value: u64) -> Result<u64> {
    value
        .checked_add(PAGE_ALIGNMENT as u64 - 1)
        .map(|value| value / PAGE_ALIGNMENT as u64 * PAGE_ALIGNMENT as u64)
        .ok_or_else(|| Error::InvalidSegment("v6 logical alignment overflow".into()))
}

pub(super) fn decode_row_group_budget(header: &[u8]) -> Result<SegmentRowGroupBudget> {
    let target_bytes = usize::try_from(read_u64(header, 160)?)
        .map_err(|_| Error::InvalidSegment("v6 row-group byte target exceeds usize".into()))?;
    let max_rows = usize::try_from(read_u64(header, 168)?)
        .map_err(|_| Error::InvalidSegment("v6 row-group row limit exceeds usize".into()))?;
    if target_bytes == 0
        || target_bytes as u64 > MAX_SEGMENT_BYTES
        || max_rows == 0
        || max_rows > u32::MAX as usize
    {
        return invalid("v6 segment row-group budget is invalid");
    }
    Ok(SegmentRowGroupBudget {
        max_rows,
        target_bytes,
    })
}

pub(in super::super) fn require_current_format(bytes: &[u8]) -> Result<()> {
    if let Some(version) = match bytes.get(..8) {
        Some(b"RRDSEG01") => Some(1),
        Some(b"RRDSEG02") => Some(2),
        Some(b"RRDSEG03") => Some(3),
        Some(b"RRDSEG04") => Some(4),
        Some(b"RRDSEG05") => Some(5),
        _ => None,
    } {
        return Err(Error::UnsupportedVersion {
            object: "segment",
            version,
        });
    }
    let version = bytes
        .get(8..10)
        .map(|field| u16::from_le_bytes(field.try_into().expect("fixed version field")))
        .ok_or_else(|| Error::InvalidSegment("segment has no complete format header".into()))?;
    if version != SEGMENT_FORMAT_VERSION {
        return Err(Error::UnsupportedVersion {
            object: "segment",
            version,
        });
    }
    if bytes.get(..8) != Some(SEGMENT_MAGIC.as_slice()) {
        return invalid("segment magic does not match its format version");
    }
    Ok(())
}
