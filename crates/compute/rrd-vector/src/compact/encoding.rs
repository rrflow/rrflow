//! Headers, decoding, scoring and integer helpers.

use super::*;

impl From<&VectorCandidate> for DenseCandidateMetadata {
    fn from(candidate: &VectorCandidate) -> Self {
        Self {
            scope: candidate.scope.clone(),
            source_cursor: candidate.source_cursor,
            reference: candidate.vector.reference.clone(),
            subject: candidate.vector.subject.clone(),
            collection: candidate.vector.collection.clone(),
            field: candidate.vector.field.clone(),
            valid_from: candidate.vector.valid_from,
            valid_to: candidate.vector.valid_to,
            provenance: candidate.vector.provenance.clone(),
            properties: candidate.vector.properties.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Header {
    pub(super) metadata_len: usize,
    pub(super) vector_offset: usize,
    pub(super) vector_len: usize,
    pub(super) row_stride: usize,
    pub(super) rows: usize,
    pub(super) dimensions: usize,
}

pub(super) fn write_header(
    bytes: &mut [u8],
    metadata_len: usize,
    vector_offset: usize,
    vector_len: usize,
    row_stride: usize,
    rows: usize,
    dimensions: usize,
) -> Result<()> {
    bytes[..8].copy_from_slice(MAGIC);
    put_u16(bytes, 8, COMPACT_DENSE_FORMAT_VERSION);
    put_u16(bytes, 10, 0);
    put_u32(bytes, 12, HEADER_BYTES as u32);
    put_u64(bytes, 16, HEADER_BYTES as u64);
    put_u64(bytes, 24, as_u64(metadata_len)?);
    put_u64(bytes, 32, as_u64(vector_offset)?);
    put_u64(bytes, 40, as_u64(vector_len)?);
    put_u64(bytes, 48, as_u64(row_stride)?);
    put_u64(bytes, 56, as_u64(rows)?);
    put_u32(bytes, 64, as_u32(dimensions)?);
    put_u32(bytes, 68, 0);
    Ok(())
}

pub(super) fn parse_header(bytes: &[u8]) -> Result<Header> {
    if bytes.len() < HEADER_BYTES || bytes.len() > MAX_ARTIFACT_BYTES {
        return invalid("compact dense artifact length is outside safety bounds");
    }
    if &bytes[..8] != MAGIC
        || read_u16(bytes, 8)? != COMPACT_DENSE_FORMAT_VERSION
        || read_u16(bytes, 10)? != 0
        || read_u32(bytes, 12)? as usize != HEADER_BYTES
        || read_u64(bytes, 16)? as usize != HEADER_BYTES
        || read_u32(bytes, 68)? != 0
        || bytes[104..HEADER_BYTES].iter().any(|byte| *byte != 0)
    {
        return invalid(
            "compact dense header magic, version, flags, or reserved bytes are invalid",
        );
    }
    let header = Header {
        metadata_len: as_usize(read_u64(bytes, 24)?)?,
        vector_offset: as_usize(read_u64(bytes, 32)?)?,
        vector_len: as_usize(read_u64(bytes, 40)?)?,
        row_stride: as_usize(read_u64(bytes, 48)?)?,
        rows: as_usize(read_u64(bytes, 56)?)?,
        dimensions: read_u32(bytes, 64)? as usize,
    };
    let metadata_end = HEADER_BYTES
        .checked_add(header.metadata_len)
        .ok_or_else(|| runtime_error("compact dense metadata offset overflow"))?;
    let vector_end = header
        .vector_offset
        .checked_add(header.vector_len)
        .ok_or_else(|| runtime_error("compact dense vector offset overflow"))?;
    if header.metadata_len == 0
        || header.dimensions == 0
        || header.dimensions > 1_048_576
        || header.rows > MAX_CANDIDATES
        || metadata_end > header.vector_offset
        || !header.vector_offset.is_multiple_of(ALIGNMENT)
        || !header.row_stride.is_multiple_of(ALIGNMENT)
        || header.row_stride < header.dimensions.saturating_mul(4)
        || header.vector_len != header.row_stride.saturating_mul(header.rows)
        || vector_end != bytes.len()
        || bytes[metadata_end..header.vector_offset]
            .iter()
            .any(|byte| *byte != 0)
    {
        return invalid("compact dense header offsets or dimensions are invalid");
    }
    Ok(header)
}

pub(super) fn validate_decoded(
    metadata: &DenseMetadata,
    header: &Header,
    bytes: &[u8],
) -> Result<()> {
    metadata.config.validate()?;
    if metadata.generation == 0
        || metadata.minimum_cursor != 0
        || metadata.config.dimensions != header.dimensions
        || metadata.candidates.len() != header.rows
    {
        return invalid("compact dense metadata differs from its physical header");
    }
    let mut versions = Vec::with_capacity(metadata.candidates.len());
    let row_bytes = header.dimensions * 4;
    for (row, candidate) in metadata.candidates.iter().enumerate() {
        if candidate.scope != metadata.config.scope
            || candidate.source_cursor == 0
            || candidate.source_cursor > metadata.source_cursor
            || candidate.field != metadata.config.field
        {
            return invalid("compact dense candidate violates metadata coverage");
        }
        let start = header.vector_offset + row * header.row_stride;
        let payload = &bytes[start..start + row_bytes];
        let values = decode_row(payload)?;
        let vector = RuntimeVector {
            reference: candidate.reference.clone(),
            subject: candidate.subject.clone(),
            collection: candidate.collection.clone(),
            field: candidate.field.clone(),
            valid_from: candidate.valid_from,
            valid_to: candidate.valid_to,
            value: VectorValue::Dense { values },
            provenance: candidate.provenance.clone(),
            properties: candidate.properties.clone(),
        };
        RuntimeCommit {
            scope: candidate.scope.clone(),
            at: candidate.valid_from,
            actor: "rrflow:compact-validator".into(),
            expected_cursor: 0,
            mutations: vec![RuntimeMutation::Vector {
                vector: vector.clone(),
            }],
        }
        .validate()?;
        let candidate_for_model = VectorCandidate {
            scope: candidate.scope.clone(),
            source_cursor: candidate.source_cursor,
            vector: vector.clone(),
        };
        if !candidate_for_model.matches_model(metadata.config.embedding_model.as_ref()) {
            return invalid("compact dense candidate differs from its configured embedding model");
        }
        if bytes[start + row_bytes..start + header.row_stride]
            .iter()
            .any(|byte| *byte != 0)
        {
            return invalid("compact dense row padding is non-zero");
        }
        versions.push(VectorCandidate {
            scope: candidate.scope.clone(),
            source_cursor: candidate.source_cursor,
            vector: RuntimeVector {
                reference: candidate.reference.clone(),
                subject: candidate.subject.clone(),
                collection: candidate.collection.clone(),
                field: candidate.field.clone(),
                valid_from: candidate.valid_from,
                valid_to: candidate.valid_to,
                value: VectorValue::Dense {
                    values: vec![0.0; header.dimensions],
                },
                provenance: None,
                properties: candidate.properties.clone(),
            },
        });
    }
    validate_candidate_versions(&versions)?;
    if metadata.candidates.windows(2).any(|pair| {
        (&pair[0].reference, pair[0].source_cursor) >= (&pair[1].reference, pair[1].source_cursor)
    }) {
        return invalid("compact dense candidates are not in canonical order");
    }
    Ok(())
}

pub(super) fn score_row(
    query: &[f32],
    row: &[u8],
    metric: ScoreMetric,
    _kernel: DenseKernel,
) -> Result<f64> {
    if row.len() != query.len() * 4 {
        return invalid("compact dense query and row dimensions differ");
    }
    #[cfg(target_arch = "x86_64")]
    if _kernel == DenseKernel::Auto && std::arch::is_x86_feature_detected!("avx2") {
        // SAFETY: AVX2 was checked at runtime; the implementation uses unaligned
        // loads and row/query lengths were proven equal above.
        return Ok(unsafe { score_avx2(query, row, metric) });
    }
    score_scalar(query, row, metric)
}

pub(super) fn score_scalar(query: &[f32], row: &[u8], metric: ScoreMetric) -> Result<f64> {
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    let mut squared_distance = 0.0;
    let mut manhattan = 0.0;
    for (left, chunk) in query.iter().zip(row.as_chunks::<4>().0.iter()) {
        let right = f32::from_le_bytes(*chunk);
        let left = f64::from(*left);
        let right = f64::from(right);
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
        squared_distance += (left - right).powi(2);
        manhattan += (left - right).abs();
    }
    Ok(match metric {
        ScoreMetric::Dot => dot,
        ScoreMetric::Cosine if right_norm == 0.0 => 0.0,
        ScoreMetric::Cosine => dot / (left_norm.sqrt() * right_norm.sqrt()),
        ScoreMetric::Euclidean => -squared_distance.sqrt(),
        ScoreMetric::Manhattan => -manhattan,
    })
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn score_avx2(query: &[f32], row: &[u8], metric: ScoreMetric) -> f64 {
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
        let right = _mm256_loadu_ps(row.as_ptr().add(index * 4).cast::<f32>());
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
    for (left, chunk) in query[index..]
        .iter()
        .zip(row[index * 4..].as_chunks::<4>().0.iter())
    {
        let right = f32::from_le_bytes(*chunk);
        let left = f64::from(*left);
        let right = f64::from(right);
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

pub(super) fn decode_row(row: &[u8]) -> Result<Vec<f32>> {
    row.as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| {
            let value = f32::from_le_bytes(*chunk);
            if !value.is_finite() {
                return invalid("compact dense vector contains a non-finite value");
            }
            Ok(value)
        })
        .collect()
}

pub(super) fn artifact_digest(bytes: &[u8]) -> Result<[u8; 32]> {
    if bytes.len() < HEADER_BYTES {
        return invalid("compact dense artifact is shorter than its header");
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
    serde_json::to_vec(value).map_err(|error| {
        runtime_error(format!("compact dense metadata cannot be encoded: {error}"))
    })
}

pub(super) fn align_up(value: usize, alignment: usize) -> Result<usize> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(|| runtime_error("compact dense alignment overflow"))
}

pub(super) fn as_u64(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| runtime_error("compact dense value exceeds u64"))
}

pub(super) fn as_u32(value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| runtime_error("compact dense value exceeds u32"))
}

pub(super) fn as_usize(value: u64) -> Result<usize> {
    usize::try_from(value).map_err(|_| runtime_error("compact dense value exceeds usize"))
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
        .and_then(|value| value.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| runtime_error("compact dense u16 header field is truncated"))
}

pub(super) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    bytes
        .get(offset..offset + 4)
        .and_then(|value| value.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| runtime_error("compact dense u32 header field is truncated"))
}

pub(super) fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    bytes
        .get(offset..offset + 8)
        .and_then(|value| value.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| runtime_error("compact dense u64 header field is truncated"))
}

pub(super) fn runtime_error(reason: impl Into<String>) -> rrd_core::Error {
    rrd_core::Error::InvalidRuntime {
        reason: reason.into(),
    }
}
