//! Encoding versions into row groups and a segment.

use super::*;

pub(in super::super) fn encode(
    table: &Memtable,
    row_group_budget: SegmentRowGroupBudget,
    compression_policy: SegmentCompressionPolicy,
) -> Result<EncodedSegment> {
    encode_versions(table.all_versions(), row_group_budget, compression_policy)
}

pub(in super::super) fn encode_versions<'a>(
    versions: impl Iterator<Item = (&'a [u8], &'a [VersionedValue])> + Clone,
    row_group_budget: SegmentRowGroupBudget,
    compression_policy: SegmentCompressionPolicy,
) -> Result<EncodedSegment> {
    let row_group_budget = row_group_budget.validate()?;
    let compression_policy = compression_policy.validate()?;
    let (entries, minimum_sequence, maximum_sequence) = versions
        .clone()
        .flat_map(|(_, versions)| versions.iter())
        .try_fold(
            (0u64, u64::MAX, 0u64),
            |(entries, minimum, maximum), version| {
                let entries = entries
                    .checked_add(1)
                    .ok_or_else(|| Error::InvalidSegment("v6 entry count exceeds u64".into()))?;
                Ok::<_, Error>((
                    entries,
                    minimum.min(version.sequence),
                    maximum.max(version.sequence),
                ))
            },
        )?;
    if entries == 0 {
        return invalid("cannot write an empty segment");
    }

    let mut output = vec![0; SEGMENT_HEADER_BYTES];
    let mut write_evidence = SegmentWriteEvidence::default();
    let mut row_groups = Vec::new();
    // The encoder owns one live row group and one bounded compression buffer.
    // Earlier payload columns are released/reused before reading the next group.
    let mut scratch = Vec::new();
    let mut current = RowGroupBuilder::new(0);
    for (key, versions) in versions {
        let key_bytes = versions.iter().try_fold(0usize, |total, version| {
            let value = version.value.as_deref().unwrap_or_default();
            validate_key_value(key, value)?;
            total
                .checked_add(key.len())
                .and_then(|total| total.checked_add(value.len()))
                .and_then(|total| total.checked_add(24))
                .ok_or_else(|| Error::InvalidSegment("v6 row-group estimate overflow".into()))
        })?;
        if current.rows() > 0
            && (current.rows().saturating_add(versions.len()) > row_group_budget.max_rows
                || current.estimated_bytes().saturating_add(key_bytes)
                    > row_group_budget.target_bytes)
        {
            let next = current
                .row_start
                .checked_add(current.rows() as u64)
                .ok_or_else(|| Error::InvalidSegment("v6 row offset overflow".into()))?;
            row_groups.push(append_row_group(
                &current,
                &mut output,
                compression_policy,
                &mut scratch,
                &mut write_evidence,
            )?);
            current.reset(next);
        }
        for version in versions {
            current.append(key, version)?;
        }
    }
    if current.rows() > 0 {
        row_groups.push(append_row_group(
            &current,
            &mut output,
            compression_policy,
            &mut scratch,
            &mut write_evidence,
        )?);
    }

    align(&mut output);
    let index_offset = output.len() as u64;
    output.extend_from_slice(INDEX_MAGIC);
    output.extend_from_slice(&(row_groups.len() as u32).to_le_bytes());
    output.extend_from_slice(&(PAGES_PER_ROW_GROUP as u16).to_le_bytes());
    output.extend_from_slice(&(ROW_GROUP_HEADER_BYTES as u16).to_le_bytes());
    output.extend_from_slice(&(PAGE_DESCRIPTOR_BYTES as u16).to_le_bytes());
    output.push(FILTER_FORMAT_VERSION);
    output.push(FILTER_BITS_PER_KEY as u8);
    output.push(FILTER_HASH_FUNCTIONS as u8);
    output.extend_from_slice(&[0; 11]);
    for group in &row_groups {
        let filter_word_count = u32::try_from(group.filter.words().len())
            .map_err(|_| Error::InvalidSegment("v6 filter word count exceeds u32".into()))?;
        output.extend_from_slice(&group.row_start.to_le_bytes());
        output.extend_from_slice(&group.row_count.to_le_bytes());
        output.extend_from_slice(&(group.first_key.len() as u32).to_le_bytes());
        output.extend_from_slice(&(group.last_key.len() as u32).to_le_bytes());
        output.extend_from_slice(&(PAGES_PER_ROW_GROUP as u16).to_le_bytes());
        output.extend_from_slice(&[0; 2]);
        output.extend_from_slice(&group.unique_key_count.to_le_bytes());
        output.extend_from_slice(&filter_word_count.to_le_bytes());
        output.extend_from_slice(&group.first_key);
        output.extend_from_slice(&group.last_key);
        for page in &group.pages {
            encode_page_descriptor(&mut output, page);
        }
        for word in group.filter.words() {
            output.extend_from_slice(&word.to_le_bytes());
        }
    }
    let index_bytes = output.len() as u64 - index_offset;
    if index_bytes as usize > MAX_INDEX_BYTES {
        return invalid("v6 index exceeds its bounded contract");
    }
    if output.len() as u64 > MAX_SEGMENT_BYTES - FOOTER_BYTES as u64 {
        return invalid("encoded v6 segment exceeds the 1 GiB safety limit");
    }

    output[..8].copy_from_slice(SEGMENT_MAGIC);
    output[8..10].copy_from_slice(&SEGMENT_FORMAT_VERSION.to_le_bytes());
    output[10..12].copy_from_slice(&(SEGMENT_HEADER_BYTES as u16).to_le_bytes());
    output[12..16].copy_from_slice(&0u32.to_le_bytes());
    output[16..24].copy_from_slice(&entries.to_le_bytes());
    output[24..32].copy_from_slice(&minimum_sequence.to_le_bytes());
    output[32..40].copy_from_slice(&maximum_sequence.to_le_bytes());
    output[40..44].copy_from_slice(&(row_groups.len() as u32).to_le_bytes());
    output[44..46].copy_from_slice(&(PAGES_PER_ROW_GROUP as u16).to_le_bytes());
    output[46..48].copy_from_slice(&(PAGE_ALIGNMENT as u16).to_le_bytes());
    output[48..56].copy_from_slice(&index_offset.to_le_bytes());
    output[56..64].copy_from_slice(&index_bytes.to_le_bytes());
    output[64..96].copy_from_slice(&decode_fixed_digest(SEGMENT_SCHEMA_DIGEST));
    output[96..128].copy_from_slice(&decode_fixed_digest(SEGMENT_KEY_CODEC_DIGEST));
    output[128..160].copy_from_slice(&decode_fixed_digest(SEGMENT_PAGE_FORMAT_DIGEST));
    output[160..168].copy_from_slice(&(row_group_budget.target_bytes as u64).to_le_bytes());
    output[168..176].copy_from_slice(&(row_group_budget.max_rows as u64).to_le_bytes());
    encode_compression_policy(&mut output, compression_policy);

    validate_logical_envelope(&row_groups, index_bytes)?;

    let checksum = sha256_hex(&output);
    output.extend_from_slice(checksum.as_bytes());
    Ok(EncodedSegment {
        bytes: output,
        checksum,
        evidence: write_evidence,
    })
}

pub(super) fn append_row_group(
    builder: &RowGroupBuilder,
    output: &mut Vec<u8>,
    compression_policy: SegmentCompressionPolicy,
    scratch: &mut Vec<u8>,
    write_evidence: &mut SegmentWriteEvidence,
) -> Result<RowGroupDescriptor> {
    let row_count = u32::try_from(builder.rows())
        .map_err(|_| Error::InvalidSegment("v6 row-group count exceeds u32".into()))?;
    let unique_key_count = builder.unique_key_count;
    let filter = builder.filter()?;
    let sequence_min = *builder
        .sequences
        .iter()
        .min()
        .expect("non-empty row group has sequences");
    let sequence_max = *builder
        .sequences
        .iter()
        .max()
        .expect("non-empty row group has sequences");
    let key_offsets = encode_i64_values(&builder.key_offsets);
    let sequences = encode_u64_values(&builder.sequences);
    let value_offsets = encode_i64_values(&builder.value_offsets);
    let pages = [
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::KeyOffsets,
                builder.row_start,
                row_count,
                PageStatistics::new(0, builder.minimum_key_bytes, builder.maximum_key_bytes),
            ),
            &key_offsets,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::KeyData,
                builder.row_start,
                row_count,
                PageStatistics::new(0, builder.minimum_key_bytes, builder.maximum_key_bytes),
            ),
            &builder.keys,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::SequenceValues,
                builder.row_start,
                row_count,
                PageStatistics::new(0, sequence_min, sequence_max),
            ),
            &sequences,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::ValueValidity,
                builder.row_start,
                row_count,
                PageStatistics::new(builder.null_count, 0, 1),
            ),
            &builder.value_validity,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::ValueOffsets,
                builder.row_start,
                row_count,
                PageStatistics::new(
                    builder.null_count,
                    builder.minimum_value_bytes,
                    builder.maximum_value_bytes,
                ),
            ),
            &value_offsets,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::ValueData,
                builder.row_start,
                row_count,
                PageStatistics::new(
                    builder.null_count,
                    builder.minimum_value_bytes,
                    builder.maximum_value_bytes,
                ),
            ),
            &builder.values,
            compression_policy,
            scratch,
            write_evidence,
        )?,
    ];
    Ok(RowGroupDescriptor {
        row_start: builder.row_start,
        row_count,
        unique_key_count,
        first_key: builder.first_key.clone(),
        last_key: builder.last_key.clone(),
        pages,
        filter,
    })
}
