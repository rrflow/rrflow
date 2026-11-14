//! Payload encoding, quantized and dense scoring, headers, digests and integer helpers.

use super::*;

pub(super) fn encode_payload(
    method: QuantizationMethod,
    values: &[&[f32]],
) -> Result<(QuantizedLayout, usize, Vec<u8>)> {
    match method {
        QuantizationMethod::Scalar => {
            let maximum = values
                .iter()
                .flat_map(|row| row.iter())
                .map(|value| value.abs())
                .fold(0.0_f32, f32::max);
            let scale = if maximum == 0.0 { 0.0 } else { maximum / 127.0 };
            let row_bytes = values[0].len();
            let mut payload = Vec::with_capacity(row_bytes * values.len());
            for row in values {
                payload.extend(row.iter().map(|value| {
                    if scale == 0.0 {
                        0_u8
                    } else {
                        ((value / scale).round().clamp(-127.0, 127.0) as i8) as u8
                    }
                }));
            }
            Ok((QuantizedLayout::Scalar { scale }, row_bytes, payload))
        }
        QuantizationMethod::Binary => {
            let row_bytes = values[0].len().div_ceil(8);
            let mut payload = vec![0_u8; row_bytes * values.len()];
            for (row_index, row) in values.iter().enumerate() {
                for (dimension, value) in row.iter().enumerate() {
                    if *value >= 0.0 {
                        payload[row_index * row_bytes + dimension / 8] |= 1 << (dimension % 8);
                    }
                }
            }
            Ok((QuantizedLayout::Binary, row_bytes, payload))
        }
        QuantizationMethod::Product { compression } => {
            let block_dimensions = compression.block_dimensions();
            let dimensions = values[0].len();
            let subspaces = dimensions.div_ceil(block_dimensions);
            let codebook_size = values.len().min(256);
            let mut codebooks = Vec::with_capacity(subspaces);
            for subspace in 0..subspaces {
                let mut codebook = Vec::with_capacity(codebook_size * block_dimensions);
                for code in 0..codebook_size {
                    let sample = code * values.len() / codebook_size;
                    for offset in 0..block_dimensions {
                        codebook.push(
                            values[sample]
                                .get(subspace * block_dimensions + offset)
                                .copied()
                                .unwrap_or(0.0),
                        );
                    }
                }
                codebooks.push(codebook);
            }
            let row_bytes = subspaces;
            let mut payload = Vec::with_capacity(row_bytes * values.len());
            for row in values {
                for (subspace, codebook) in codebooks.iter().enumerate() {
                    let mut best = (0_usize, f64::INFINITY);
                    for code in 0..codebook_size {
                        let mut distance = 0.0_f64;
                        for offset in 0..block_dimensions {
                            let left = row
                                .get(subspace * block_dimensions + offset)
                                .copied()
                                .unwrap_or(0.0);
                            let right = codebook[code * block_dimensions + offset];
                            distance += f64::from(left - right).powi(2);
                        }
                        if distance < best.1 {
                            best = (code, distance);
                        }
                    }
                    payload.push(best.0 as u8);
                }
            }
            Ok((
                QuantizedLayout::Product {
                    block_dimensions,
                    subspaces,
                    codebook_size,
                    codebooks,
                },
                row_bytes,
                payload,
            ))
        }
    }
}

pub(super) fn score_quantized(
    query: &[f32],
    row: &[u8],
    layout: &QuantizedLayout,
    metric: ScoreMetric,
    kernel: QuantizedKernel,
) -> Result<f64> {
    match layout {
        QuantizedLayout::Scalar { scale } => {
            if row.len() != query.len() {
                return invalid("scalar quantized row dimensions differ from the query");
            }
            let decoded = row
                .iter()
                .map(|value| f32::from(*value as i8) * *scale)
                .collect::<Vec<_>>();
            score_dense(query, &decoded, metric, kernel)
        }
        QuantizedLayout::Product {
            block_dimensions,
            subspaces,
            codebook_size,
            codebooks,
        } => {
            if row.len() != *subspaces || codebooks.len() != *subspaces {
                return invalid("product quantized row differs from its codebooks");
            }
            let mut decoded = Vec::with_capacity(query.len());
            for (subspace, encoded) in row.iter().enumerate() {
                let code = usize::from(*encoded);
                if code >= *codebook_size {
                    return invalid("product quantized row contains an unknown centroid");
                }
                let start = code * *block_dimensions;
                let remaining = query.len() - decoded.len();
                decoded.extend_from_slice(
                    &codebooks[subspace][start..start + (*block_dimensions).min(remaining)],
                );
            }
            score_dense(query, &decoded, metric, kernel)
        }
        QuantizedLayout::Binary => {
            if row.len() != query.len().div_ceil(8) {
                return invalid("binary quantized row dimensions differ from the query");
            }
            let mut matches = 0_u64;
            for (dimension, value) in query.iter().enumerate() {
                let query_bit = *value >= 0.0;
                let row_bit = row[dimension / 8] & (1 << (dimension % 8)) != 0;
                matches += u64::from(query_bit == row_bit);
            }
            let signed = 2.0 * matches as f64 - query.len() as f64;
            Ok(match metric {
                ScoreMetric::Dot => signed,
                ScoreMetric::Cosine => signed / query.len() as f64,
                ScoreMetric::Euclidean => -(query.len() as f64 - matches as f64).sqrt(),
                ScoreMetric::Manhattan => -(query.len() as f64 - matches as f64),
            })
        }
    }
}

pub(super) fn score_dense(
    query: &[f32],
    row: &[f32],
    metric: ScoreMetric,
    kernel: QuantizedKernel,
) -> Result<f64> {
    if query.len() != row.len()
        || query.iter().chain(row).any(|value| !value.is_finite())
        || (metric == ScoreMetric::Cosine && query.iter().all(|value| *value == 0.0))
    {
        return invalid("quantized dense score requires finite matching vectors");
    }
    match kernel {
        QuantizedKernel::Scalar => Ok(score_dense_scalar(query, row, metric)),
        QuantizedKernel::Auto => {
            #[cfg(target_arch = "x86_64")]
            if std::arch::is_x86_feature_detected!("avx2") {
                // SAFETY: AVX2 support was checked and both slices have equal
                // lengths.
                return Ok(unsafe { score_dense_avx2(query, row, metric) });
            }
            Ok(score_dense_scalar(query, row, metric))
        }
    }
}

pub(super) fn score_dense_scalar(query: &[f32], row: &[f32], metric: ScoreMetric) -> f64 {
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    let mut squared_distance = 0.0;
    let mut manhattan = 0.0;
    for (left, right) in query.iter().zip(row) {
        let left = f64::from(*left);
        let right = f64::from(*right);
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
        squared_distance += (left - right).powi(2);
        manhattan += (left - right).abs();
    }
    match metric {
        ScoreMetric::Dot => dot,
        ScoreMetric::Cosine if right_norm == 0.0 => 0.0,
        ScoreMetric::Cosine => dot / (left_norm.sqrt() * right_norm.sqrt()),
        ScoreMetric::Euclidean => -squared_distance.sqrt(),
        ScoreMetric::Manhattan => -manhattan,
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn score_dense_avx2(query: &[f32], row: &[f32], metric: ScoreMetric) -> f64 {
    use std::arch::x86_64::*;
    let mut dot = _mm256_setzero_ps();
    let mut left_norm = _mm256_setzero_ps();
    let mut right_norm = _mm256_setzero_ps();
    let mut distance = _mm256_setzero_ps();
    let mut manhattan = _mm256_setzero_ps();
    let sign_mask = _mm256_set1_ps(-0.0);
    let mut index = 0;
    while index + 8 <= query.len() {
        let left = _mm256_loadu_ps(query.as_ptr().add(index));
        let right = _mm256_loadu_ps(row.as_ptr().add(index));
        let delta = _mm256_sub_ps(left, right);
        dot = _mm256_add_ps(dot, _mm256_mul_ps(left, right));
        left_norm = _mm256_add_ps(left_norm, _mm256_mul_ps(left, left));
        right_norm = _mm256_add_ps(right_norm, _mm256_mul_ps(right, right));
        distance = _mm256_add_ps(distance, _mm256_mul_ps(delta, delta));
        manhattan = _mm256_add_ps(manhattan, _mm256_andnot_ps(sign_mask, delta));
        index += 8;
    }
    let mut lanes = [0.0_f32; 8];
    let reduce = |value: __m256, lanes: &mut [f32; 8]| {
        _mm256_storeu_ps(lanes.as_mut_ptr(), value);
        lanes.iter().map(|value| f64::from(*value)).sum::<f64>()
    };
    let mut dot = reduce(dot, &mut lanes);
    let mut left_norm = reduce(left_norm, &mut lanes);
    let mut right_norm = reduce(right_norm, &mut lanes);
    let mut distance = reduce(distance, &mut lanes);
    let mut manhattan = reduce(manhattan, &mut lanes);
    for (left, right) in query[index..].iter().zip(&row[index..]) {
        let left = f64::from(*left);
        let right = f64::from(*right);
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
        distance += (left - right).powi(2);
        manhattan += (left - right).abs();
    }
    match metric {
        ScoreMetric::Dot => dot,
        ScoreMetric::Cosine if right_norm == 0.0 => 0.0,
        ScoreMetric::Cosine => dot / (left_norm.sqrt() * right_norm.sqrt()),
        ScoreMetric::Euclidean => -distance.sqrt(),
        ScoreMetric::Manhattan => -manhattan,
    }
}

pub(super) fn validate_decoded(
    metadata: &SegmentMetadata,
    header: &Header,
    bytes: &[u8],
) -> Result<()> {
    metadata.config.validate()?;
    let metadata_end = HEADER_BYTES + header.metadata_len;
    if metadata.generation == 0
        || metadata.minimum_cursor != 0
        || metadata.config.dimensions != header.dimensions
        || metadata.candidates.len() != header.rows
        || metadata.row_bytes != header.row_bytes
        || bytes[metadata_end..header.payload_offset]
            .iter()
            .any(|byte| *byte != 0)
    {
        return invalid("quantized metadata differs from its physical header");
    }
    let expected_row_bytes = match &metadata.layout {
        QuantizedLayout::Scalar { scale } => {
            if !scale.is_finite() || *scale < 0.0 {
                return invalid("scalar quantization scale is invalid");
            }
            metadata.config.dimensions
        }
        QuantizedLayout::Binary => metadata.config.dimensions.div_ceil(8),
        QuantizedLayout::Product {
            block_dimensions,
            subspaces,
            codebook_size,
            codebooks,
        } => {
            let QuantizationMethod::Product { compression } = metadata.config.method else {
                return invalid("product layout differs from configured method");
            };
            if *block_dimensions != compression.block_dimensions()
                || *subspaces != metadata.config.dimensions.div_ceil(*block_dimensions)
                || *codebook_size == 0
                || *codebook_size > 256
                || codebooks.len() != *subspaces
                || codebooks.iter().any(|codebook| {
                    codebook.len() != codebook_size * block_dimensions
                        || codebook.iter().any(|value| !value.is_finite())
                })
            {
                return invalid("product quantization codebook shape is invalid");
            }
            *subspaces
        }
    };
    if expected_row_bytes != header.row_bytes
        || header.payload_len != header.rows.saturating_mul(header.row_bytes)
        || !matches!(
            (&metadata.layout, metadata.config.method),
            (QuantizedLayout::Scalar { .. }, QuantizationMethod::Scalar)
                | (
                    QuantizedLayout::Product { .. },
                    QuantizationMethod::Product { .. }
                )
                | (QuantizedLayout::Binary, QuantizationMethod::Binary)
        )
    {
        return invalid("quantized payload shape differs from configured method");
    }
    let mut versions = BTreeSet::new();
    for candidate in &metadata.candidates {
        if candidate.source_cursor == 0
            || candidate.source_cursor > metadata.source_cursor
            || !versions.insert((candidate.reference.clone(), candidate.source_cursor))
        {
            return invalid("quantized candidate metadata is invalid");
        }
    }
    if metadata.candidates.windows(2).any(|pair| {
        (&pair[0].reference, pair[0].source_cursor) >= (&pair[1].reference, pair[1].source_cursor)
    }) {
        return invalid("quantized candidates are not in canonical order");
    }
    Ok(())
}

pub(super) fn write_header(
    bytes: &mut [u8],
    metadata_len: usize,
    payload_offset: usize,
    payload_len: usize,
    row_bytes: usize,
    rows: usize,
    dimensions: usize,
) -> Result<()> {
    bytes[..8].copy_from_slice(MAGIC);
    put_u16(bytes, 8, QUANTIZED_SEGMENT_FORMAT_VERSION);
    put_u16(bytes, 10, 0);
    put_u32(bytes, 12, as_u32(HEADER_BYTES)?);
    put_u64(bytes, 16, as_u64(metadata_len)?);
    put_u64(bytes, 24, as_u64(payload_offset)?);
    put_u64(bytes, 32, as_u64(payload_len)?);
    put_u64(bytes, 40, as_u64(row_bytes)?);
    put_u64(bytes, 48, as_u64(rows)?);
    put_u32(bytes, 56, as_u32(dimensions)?);
    put_u32(bytes, 60, 0);
    put_u64(bytes, 64, as_u64(HEADER_BYTES)?);
    Ok(())
}

pub(super) fn read_header(bytes: &[u8]) -> Result<Header> {
    if bytes.len() < HEADER_BYTES || bytes.len() > MAX_ARTIFACT_BYTES {
        return invalid("quantized artifact byte length is outside bounds");
    }
    if &bytes[..8] != MAGIC
        || read_u16(bytes, 8)? != QUANTIZED_SEGMENT_FORMAT_VERSION
        || read_u16(bytes, 10)? != 0
        || read_u32(bytes, 12)? as usize != HEADER_BYTES
        || read_u64(bytes, 64)? as usize != HEADER_BYTES
        || bytes[104..HEADER_BYTES].iter().any(|byte| *byte != 0)
    {
        return invalid("quantized artifact header magic, version, or reserved bytes are invalid");
    }
    let header = Header {
        metadata_len: as_usize(read_u64(bytes, 16)?)?,
        payload_offset: as_usize(read_u64(bytes, 24)?)?,
        payload_len: as_usize(read_u64(bytes, 32)?)?,
        row_bytes: as_usize(read_u64(bytes, 40)?)?,
        rows: as_usize(read_u64(bytes, 48)?)?,
        dimensions: read_u32(bytes, 56)? as usize,
    };
    let metadata_end = HEADER_BYTES
        .checked_add(header.metadata_len)
        .ok_or_else(|| runtime_error("quantized metadata offset overflow"))?;
    let payload_end = header
        .payload_offset
        .checked_add(header.payload_len)
        .ok_or_else(|| runtime_error("quantized payload offset overflow"))?;
    if header.metadata_len == 0
        || header.row_bytes == 0
        || header.rows == 0
        || header.rows > MAX_CANDIDATES
        || header.dimensions == 0
        || header.dimensions > MAX_DIMENSIONS
        || metadata_end > header.payload_offset
        || !header.payload_offset.is_multiple_of(ALIGNMENT)
        || payload_end != bytes.len()
    {
        return invalid("quantized artifact header offsets or dimensions are invalid");
    }
    Ok(header)
}

pub(super) fn artifact_digest(bytes: &[u8]) -> Result<[u8; 32]> {
    if bytes.len() < HEADER_BYTES {
        return invalid("quantized artifact is shorter than its header");
    }
    let mut root = [0_u8; HEADER_BYTES + DIGEST_BYTES];
    root[..HEADER_BYTES].copy_from_slice(&bytes[..HEADER_BYTES]);
    root[DIGEST_OFFSET..DIGEST_OFFSET + DIGEST_BYTES].fill(0);
    root[HEADER_BYTES..].copy_from_slice(&digest::sha256(&bytes[HEADER_BYTES..]));
    Ok(digest::sha256(&root))
}

pub(super) fn hex_digest(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value)
        .map_err(|error| runtime_error(format!("quantized metadata cannot be encoded: {error}")))
}

pub(super) fn align_up(value: usize, alignment: usize) -> Result<usize> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(|| runtime_error("quantized alignment overflow"))
}

pub(super) fn as_u64(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| runtime_error("quantized value exceeds u64"))
}

pub(super) fn as_u32(value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| runtime_error("quantized value exceeds u32"))
}

pub(super) fn as_usize(value: u64) -> Result<usize> {
    usize::try_from(value).map_err(|_| runtime_error("quantized value exceeds usize"))
}

pub(super) fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub(super) fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(super) fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

pub(super) fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    bytes
        .get(offset..offset + 2)
        .and_then(|slice| slice.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| runtime_error("quantized u16 header read is out of bounds"))
}

pub(super) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    bytes
        .get(offset..offset + 4)
        .and_then(|slice| slice.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| runtime_error("quantized u32 header read is out of bounds"))
}

pub(super) fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    bytes
        .get(offset..offset + 8)
        .and_then(|slice| slice.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| runtime_error("quantized u64 header read is out of bounds"))
}

pub(super) fn runtime_error(reason: impl Into<String>) -> rrd_core::Error {
    rrd_core::Error::InvalidRuntime {
        reason: reason.into(),
    }
}
