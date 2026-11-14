//! Codec, filter and cache observations.

use super::*;

pub(super) fn observe_codec(codec: &str, pages: &[PageSample]) -> Result<CodecObservation> {
    let mut accumulator = CodecAccumulator::default();
    for page in pages {
        accumulator.pages += 1;
        accumulator.logical_bytes = accumulator
            .logical_bytes
            .checked_add(page.bytes.len() as u64)
            .ok_or_else(|| Error::InvalidSegment("codec logical byte total overflow".into()))?;
        let encode_started = Instant::now();
        let encoded = match codec {
            "none" => page.bytes.clone(),
            "lz4" => lz4_flex::block::compress_prepend_size(&page.bytes),
            "zstd" => zstd::bulk::compress(&page.bytes, 1)
                .map_err(|error| Error::InvalidSegment(format!("zstd encode failed: {error}")))?,
            _ => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown lab codec {codec}"
                )))
            }
        };
        accumulator.encode_nanoseconds = accumulator.encode_nanoseconds.saturating_add(
            encode_started
                .elapsed()
                .as_nanos()
                .try_into()
                .unwrap_or(u64::MAX),
        );
        black_box(&encoded);
        let decode_started = Instant::now();
        let decoded = match codec {
            "none" => encoded.clone(),
            "lz4" => lz4_flex::block::decompress_size_prepended(&encoded).map_err(|error| {
                Error::InvalidSegment(format!("lz4 candidate decode failed: {error}"))
            })?,
            "zstd" => zstd::bulk::decompress(&encoded, page.bytes.len()).map_err(|error| {
                Error::InvalidSegment(format!("zstd candidate decode failed: {error}"))
            })?,
            _ => unreachable!(),
        };
        accumulator.decode_nanoseconds = accumulator.decode_nanoseconds.saturating_add(
            decode_started
                .elapsed()
                .as_nanos()
                .try_into()
                .unwrap_or(u64::MAX),
        );
        black_box(&decoded);
        if decoded != page.bytes {
            return Err(Error::InvalidSegment(format!(
                "{codec} candidate changed {} page {}",
                page.family, page.kind
            )));
        }
        accumulator.codec_bytes = accumulator
            .codec_bytes
            .checked_add(encoded.len() as u64)
            .ok_or_else(|| Error::InvalidSegment("codec byte total overflow".into()))?;
        let adaptive = if codec != "none" && codec_saves_enough(page.bytes.len(), encoded.len()) {
            accumulator.selected_pages += 1;
            encoded.len().saturating_add(ADAPTIVE_CODEC_FRAME_BYTES)
        } else {
            page.bytes.len()
        };
        accumulator.adaptive_stored_bytes = accumulator
            .adaptive_stored_bytes
            .checked_add(adaptive as u64)
            .ok_or_else(|| Error::InvalidSegment("adaptive codec byte total overflow".into()))?;
    }
    let savings_basis_points =
        savings_basis_points(accumulator.logical_bytes, accumulator.adaptive_stored_bytes);
    Ok(CodecObservation {
        codec: codec.into(),
        pages: accumulator.pages,
        logical_bytes: accumulator.logical_bytes,
        codec_bytes: accumulator.codec_bytes,
        adaptive_stored_bytes: accumulator.adaptive_stored_bytes,
        adaptive_selected_pages: accumulator.selected_pages,
        savings_basis_points,
        encode_nanoseconds: accumulator.encode_nanoseconds,
        decode_nanoseconds: accumulator.decode_nanoseconds,
        round_trip_exact: true,
    })
}

pub(super) fn codec_saves_enough(logical: usize, encoded: usize) -> bool {
    let Some(stored) = encoded.checked_add(ADAPTIVE_CODEC_FRAME_BYTES) else {
        return false;
    };
    stored < logical
        && savings_basis_points(logical as u64, stored as u64) >= ADAPTIVE_CODEC_MINIMUM_SAVINGS_BPS
}

pub(super) fn savings_basis_points(logical: u64, stored: u64) -> u64 {
    if logical == 0 || stored >= logical {
        0
    } else {
        logical.saturating_sub(stored).saturating_mul(10_000) / logical
    }
}

pub(super) fn observe_filters(
    encoded: &[u8],
    row_groups: &[super::super::format::RowGroupDescriptor],
    config: PhysicalPolicyConfig,
) -> Result<FilterObservation> {
    let mut filters = 0u64;
    let mut unique_keys = 0u64;
    let mut serialized_bytes = 0u64;
    let mut member_queries = 0u64;
    let mut member_false_negatives = 0u64;
    let mut absent_queries = 0u64;
    let mut false_positives = 0u64;
    for group in row_groups {
        let keys = keys_for_group(encoded, group)?;
        if keys.len() as u32 != group.unique_key_count {
            return Err(Error::InvalidSegment(
                "persisted filter unique-key count disagrees with key pages".into(),
            ));
        }
        let filter = &group.filter;
        filters += 1;
        unique_keys = unique_keys.saturating_add(keys.len() as u64);
        serialized_bytes = serialized_bytes.saturating_add(filter.byte_len() as u64);
        for key in &keys {
            member_queries += 1;
            if !filter.may_contain(key) {
                member_false_negatives += 1;
            }
        }
        let probes = config.point_misses.div_ceil(row_groups.len()).max(1);
        for probe in 0..probes {
            let mut absent = keys[probe % keys.len()].clone();
            absent.extend_from_slice(b"/absent/");
            absent.extend_from_slice(&(probe as u64).to_le_bytes());
            if keys.binary_search(&absent).is_ok() {
                return Err(Error::InvalidSegment(
                    "generated filter probe unexpectedly exists".into(),
                ));
            }
            absent_queries += 1;
            if filter.may_contain(&absent) {
                false_positives += 1;
            }
        }
    }
    let false_positive_parts_per_million = false_positives
        .saturating_mul(1_000_000)
        .checked_div(absent_queries)
        .unwrap_or(0);
    Ok(FilterObservation {
        format: "segment-v6-row-group-bloom-v1".into(),
        filters,
        unique_keys,
        serialized_bytes,
        bits_per_key: FILTER_BITS_PER_KEY as u64,
        hash_functions: FILTER_HASH_FUNCTIONS as u64,
        member_queries,
        member_false_negatives,
        absent_queries,
        false_positives,
        false_positive_parts_per_million,
        passes_policy_threshold: member_false_negatives == 0
            && false_positive_parts_per_million <= FILTER_FALSE_POSITIVE_LIMIT_PPM,
    })
}

pub(super) fn validate_persisted_filter_integration(
    row_group_count: usize,
    filter: &FilterObservation,
    reopened: &ReopenedPointMissObservation,
) -> Result<()> {
    let row_group_count = u64::try_from(row_group_count)
        .map_err(|_| Error::InvalidSegment("row-group count exceeds u64".into()))?;
    require_integration(
        filter.filters == row_group_count,
        "candidate and segment row-group counts differ",
    )?;
    require_integration(
        filter.member_false_negatives == 0,
        "persisted filter produced a member false negative",
    )?;
    require_integration(
        filter.passes_policy_threshold,
        "persisted filter exceeded its accepted false-positive threshold",
    )?;
    require_integration(
        reopened.open_persisted_filter_count == filter.filters,
        "reopen filter count differs from the candidate observation",
    )?;
    require_integration(
        reopened.open_persisted_filter_bytes == filter.serialized_bytes,
        "reopen filter bytes differ from the candidate observation",
    )?;
    require_integration(
        reopened.open_semantic_page_operations == 0 && reopened.open_semantic_page_bytes == 0,
        "metadata-only reopen performed semantic page work",
    )?;
    require_integration(
        reopened.open_none_policy_segment_count == 0
            && reopened.open_adaptive_lz4_policy_segment_count > 0,
        "reopen did not authenticate only adaptive-LZ4 segments",
    )?;
    require_integration(
        reopened.open_raw_page_count > 0 && reopened.open_compressed_page_count > 0,
        "corpus did not exercise both raw and compressed production pages",
    )?;
    require_integration(
        reopened.open_stored_page_bytes < reopened.open_logical_page_bytes,
        "adaptive production pages did not reduce stored bytes",
    )?;
    require_integration(
        reopened.bytes_decompressed > 0,
        "present-key reads did not exercise production decompression",
    )?;
    require_integration(
        reopened.filter_checks > 0 && reopened.filter_negatives > 0,
        "point-miss workload did not exercise persisted filter pruning",
    )?;
    require_integration(
        reopened.page_loads < reopened.filter_checks,
        "point-miss workload did not avoid semantic page loads",
    )?;
    Ok(())
}

pub(super) fn require_integration(condition: bool, invariant: &'static str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::InvalidSegment(format!(
            "adaptive page-compression integration evidence failed: {invariant}"
        )))
    }
}

pub(super) fn keys_for_group(
    encoded: &[u8],
    group: &super::super::format::RowGroupDescriptor,
) -> Result<Vec<Vec<u8>>> {
    let offsets_page = group.page(PageKind::KeyOffsets);
    let data_page = group.page(PageKind::KeyData);
    let offsets = page_slice(encoded, offsets_page)?;
    let data = page_slice(encoded, data_page)?;
    let expected_offsets = (group.row_count as usize)
        .checked_add(1)
        .and_then(|count| count.checked_mul(std::mem::size_of::<i64>()))
        .ok_or_else(|| Error::InvalidSegment("key-offset shape overflow".into()))?;
    if offsets.len() != expected_offsets {
        return Err(Error::InvalidSegment(
            "candidate filter saw an invalid key-offset page".into(),
        ));
    }
    let mut keys = BTreeSet::new();
    for row in 0..group.row_count as usize {
        let start = decode_offset(offsets, row)?;
        let end = decode_offset(offsets, row + 1)?;
        if start > end || end > data.len() {
            return Err(Error::InvalidSegment(
                "candidate filter key offsets escaped key data".into(),
            ));
        }
        keys.insert(data[start..end].to_vec());
    }
    if keys.is_empty() {
        return Err(Error::InvalidSegment(
            "candidate filter received an empty row group".into(),
        ));
    }
    Ok(keys.into_iter().collect())
}

pub(super) fn page_slice<'a>(
    encoded: &'a [u8],
    descriptor: &super::super::format::PageDescriptor,
) -> Result<&'a [u8]> {
    let start = usize::try_from(descriptor.offset)
        .map_err(|_| Error::InvalidSegment("page offset exceeds usize".into()))?;
    let end = start
        .checked_add(descriptor.physical_bytes)
        .ok_or_else(|| Error::InvalidSegment("page offset overflow".into()))?;
    encoded
        .get(start..end)
        .ok_or_else(|| Error::InvalidSegment("page slice is outside segment".into()))
}

pub(super) fn decode_offset(bytes: &[u8], index: usize) -> Result<usize> {
    let start = index
        .checked_mul(std::mem::size_of::<i64>())
        .ok_or_else(|| Error::InvalidSegment("offset index overflow".into()))?;
    let end = start + std::mem::size_of::<i64>();
    let raw: [u8; 8] = bytes
        .get(start..end)
        .ok_or_else(|| Error::InvalidSegment("offset page ended early".into()))?
        .try_into()
        .expect("fixed offset width was checked");
    usize::try_from(i64::from_le_bytes(raw))
        .map_err(|_| Error::InvalidSegment("offset is negative or exceeds usize".into()))
}

pub(super) fn observe_caches(
    pages: &[PageSample],
    capacity: usize,
) -> Result<Vec<CachePolicyObservation>> {
    let trace = cache_trace(pages)?;
    let mut lru = ExactLru::new(capacity);
    let mut segmented = SegmentedLru::new(capacity);
    for access in trace {
        lru.access(access)?;
        segmented.access(access)?;
    }
    Ok(vec![lru.finish(), segmented.finish()?])
}

pub(super) fn cache_trace(pages: &[PageSample]) -> Result<Vec<CacheAccess>> {
    if pages.is_empty() {
        return Err(Error::InvalidSegment("cache trace has no pages".into()));
    }
    let hot = pages.iter().take(12).collect::<Vec<_>>();
    let mut trace = Vec::new();
    for _ in 0..8 {
        for page in &hot {
            trace.push(CacheAccess {
                page_id: page.id,
                bytes: page.bytes.len().max(1),
                post_scan_hot: false,
            });
        }
    }
    for page in pages {
        trace.push(CacheAccess {
            page_id: page.id,
            bytes: page.bytes.len().max(1),
            post_scan_hot: false,
        });
    }
    for _ in 0..8 {
        for page in &hot {
            trace.push(CacheAccess {
                page_id: page.id,
                bytes: page.bytes.len().max(1),
                post_scan_hot: true,
            });
        }
    }
    Ok(trace)
}
