//! Candidate ordering and canonical artifact encoding.

use super::*;

pub(super) fn sort_candidates(candidates: &mut [VectorCandidate]) {
    candidates.sort_by(|left, right| {
        left.vector
            .reference
            .cmp(&right.vector.reference)
            .then_with(|| left.source_cursor.cmp(&right.source_cursor))
    });
}

/// One version as a generation stores it, with its packed codes.
pub(super) fn encode_candidate(
    config: &TurboQuantSegmentConfig,
    projection: Option<&QjlProjection>,
    candidate: VectorCandidate,
) -> Result<(StoredCandidate, Vec<u8>)> {
    if candidate.scope != config.scope
        || candidate.vector.field != config.field
        || candidate.vector.value.dimensions() != config.dimensions
        || !candidate.matches_model(config.embedding_model.as_ref())
    {
        return invalid("TurboQuant candidate violates configuration or coverage");
    }
    let VectorValue::Dense { values } = &candidate.vector.value else {
        return invalid("TurboQuant artifacts currently require dense vectors");
    };
    let (original_norm, centroid_norm, residual_norm, packed) = match projection {
        None => {
            let encoded = TurboQuantVector::encode(values, config.bits, config.seed)?;
            (
                encoded.original_norm,
                encoded.centroid_norm,
                None,
                encoded.packed,
            )
        }
        Some(projection) => {
            let encoded =
                TurboQuantInnerProductCode::encode(values, config.bits, config.seed, projection)?;
            (
                encoded.original_norm,
                encoded.centroid_norm,
                Some(encoded.residual_norm),
                encoded.packed,
            )
        }
    };
    Ok((
        StoredCandidate {
            reference: candidate.vector.reference,
            subject: candidate.vector.subject,
            collection: candidate.vector.collection,
            source_cursor: candidate.source_cursor,
            valid_from: candidate.vector.valid_from,
            valid_to: candidate.vector.valid_to,
            properties: candidate.vector.properties,
            original_norm,
            centroid_norm,
            residual_norm,
        },
        packed,
    ))
}

pub(super) fn runtime_error(reason: impl Into<String>) -> rrd_core::Error {
    rrd_core::Error::InvalidRuntime {
        reason: reason.into(),
    }
}

pub(super) fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| rrd_core::Error::InvalidRuntime {
        reason: format!("TurboQuant artifact cannot be encoded: {error}"),
    })
}

pub(super) fn artifact_digest(header: &[u8], payload: &[u8]) -> String {
    let mut bytes = b"rrflow-turboquant-segment-v1\0".to_vec();
    bytes.extend_from_slice(header);
    bytes.extend_from_slice(payload);
    digest::sha256_hex(&bytes)
}

pub(super) fn encode_artifact(
    header: &[u8],
    payload: &[u8],
    artifact_digest: &str,
) -> Result<Vec<u8>> {
    let header_len = u32::try_from(header.len()).map_err(|_| rrd_core::Error::InvalidRuntime {
        reason: "TurboQuant header exceeds u32".into(),
    })?;
    let payload_len =
        u64::try_from(payload.len()).map_err(|_| rrd_core::Error::InvalidRuntime {
            reason: "TurboQuant payload exceeds u64".into(),
        })?;
    let mut bytes = Vec::with_capacity(HEADER_PREFIX_BYTES + header.len() + payload.len());
    bytes.extend_from_slice(TURBOQUANT_SEGMENT_MAGIC);
    bytes.extend_from_slice(&TURBOQUANT_SEGMENT_FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&header_len.to_le_bytes());
    bytes.extend_from_slice(&payload_len.to_le_bytes());
    bytes.extend_from_slice(artifact_digest.as_bytes());
    bytes.extend_from_slice(header);
    bytes.extend_from_slice(payload);
    if bytes.len() > MAX_TURBOQUANT_SEGMENT_BYTES {
        return invalid("TurboQuant artifact exceeds the 1 GiB safety limit");
    }
    Ok(bytes)
}
