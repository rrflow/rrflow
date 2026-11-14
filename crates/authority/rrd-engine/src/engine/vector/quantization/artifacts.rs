//! Public quantization artifacts and version reads.

use super::*;

/// The current snapshot of the artifact the operation under the context's
/// idempotency key acted on.
pub(super) fn recorded_artifact(
    catalogue: &rrd_vector::QuantizationArtifactCatalogue,
    context: &RequestContext,
) -> Result<VectorQuantizationArtifactSnapshot> {
    let outcome = context
        .idempotency_key
        .as_ref()
        .and_then(|key| catalogue.operation(key.as_str()))
        .ok_or_else(|| {
            ServiceError::Vector("the recorded quantization operation is absent".into())
        })?;
    let artifact = catalogue
        .artifacts
        .get(&(outcome.projection_id.clone(), outcome.generation))
        .ok_or_else(|| {
            ServiceError::Vector("the recorded quantization artifact is absent".into())
        })?;
    public_quantization_artifact(&artifact.entry, artifact.state, catalogue.revision)
}

pub(in super::super) fn quantization_artifact_id(
    method: &str,
    collection_id: &CanonicalId,
    vector_name: &CanonicalId,
) -> Result<ProjectionId> {
    ProjectionId::new(format!("quant-{method}-{collection_id}-{vector_name}")).map_err(core_vector)
}

pub(super) fn internal_quantization_method(
    method: &VectorQuantizationMethod,
) -> Result<rrd_vector::QuantizationMethod> {
    Ok(match method {
        VectorQuantizationMethod::Scalar => rrd_vector::QuantizationMethod::Scalar,
        VectorQuantizationMethod::Product { compression } => {
            rrd_vector::QuantizationMethod::Product {
                compression: match compression {
                    VectorProductCompression::X4 => rrd_vector::ProductCompression::X4,
                    VectorProductCompression::X8 => rrd_vector::ProductCompression::X8,
                    VectorProductCompression::X16 => rrd_vector::ProductCompression::X16,
                    VectorProductCompression::X32 => rrd_vector::ProductCompression::X32,
                    VectorProductCompression::X64 => rrd_vector::ProductCompression::X64,
                },
            }
        }
        VectorQuantizationMethod::Binary => rrd_vector::QuantizationMethod::Binary,
        VectorQuantizationMethod::TurboQuant { .. } => {
            return Err(ServiceError::Vector(
                "TurboQuant does not use the scalar/product/binary codec".into(),
            ));
        }
    })
}

/// The read behind a build: the vector versions it selected and its evidence.
pub(super) struct VersionsRead {
    pub(super) selected_versions: u64,
    pub(super) read_evidence: rrd_contract::ReadEvidence,
}

pub(super) fn internal_turboquant_bits(bits: VectorQuantizationBits) -> rrd_vector::TurboQuantBits {
    match bits {
        VectorQuantizationBits::Bits4 => rrd_vector::TurboQuantBits::Bits4,
        VectorQuantizationBits::Bits2 => rrd_vector::TurboQuantBits::Bits2,
        VectorQuantizationBits::Bits1_5 => rrd_vector::TurboQuantBits::Bits1_5,
        VectorQuantizationBits::Bits1 => rrd_vector::TurboQuantBits::Bits1,
    }
}

pub(super) fn public_quantization_artifact(
    entry: &rrd_vector::QuantizationArtifactEntry,
    state: rrd_vector::QuantizationArtifactState,
    lifecycle_revision: u64,
) -> Result<VectorQuantizationArtifactSnapshot> {
    let stamp = entry.descriptor.stamp();
    let (method, indexed_vectors, packed, full, auxiliary) = match &entry.descriptor {
        rrd_vector::VectorProjectionDescriptor::Quantized { descriptor } => (
            match descriptor.method {
                rrd_vector::QuantizationMethod::Scalar => VectorQuantizationMethod::Scalar,
                rrd_vector::QuantizationMethod::Product { compression } => {
                    VectorQuantizationMethod::Product {
                        compression: match compression {
                            rrd_vector::ProductCompression::X4 => VectorProductCompression::X4,
                            rrd_vector::ProductCompression::X8 => VectorProductCompression::X8,
                            rrd_vector::ProductCompression::X16 => VectorProductCompression::X16,
                            rrd_vector::ProductCompression::X32 => VectorProductCompression::X32,
                            rrd_vector::ProductCompression::X64 => VectorProductCompression::X64,
                        },
                    }
                }
                rrd_vector::QuantizationMethod::Binary => VectorQuantizationMethod::Binary,
            },
            descriptor.candidate_versions,
            descriptor.packed_vector_bytes,
            descriptor.full_precision_vector_bytes,
            descriptor.auxiliary_bytes,
        ),
        rrd_vector::VectorProjectionDescriptor::TurboQuant { descriptor } => (
            VectorQuantizationMethod::TurboQuant {
                bits: match descriptor.bits {
                    rrd_vector::TurboQuantBits::Bits4 => VectorQuantizationBits::Bits4,
                    rrd_vector::TurboQuantBits::Bits2 => VectorQuantizationBits::Bits2,
                    rrd_vector::TurboQuantBits::Bits1_5 => VectorQuantizationBits::Bits1_5,
                    rrd_vector::TurboQuantBits::Bits1 => VectorQuantizationBits::Bits1,
                },
                seed: descriptor.seed,
                estimator: match descriptor.estimator {
                    rrd_vector::TurboQuantEstimator::Mse => {
                        rrd_contract::VectorQuantizationEstimator::Mse
                    }
                    rrd_vector::TurboQuantEstimator::InnerProduct => {
                        rrd_contract::VectorQuantizationEstimator::InnerProduct
                    }
                },
            },
            descriptor.candidate_versions,
            descriptor.packed_vector_bytes,
            descriptor.full_precision_vector_bytes,
            0,
        ),
        _ => {
            return Err(ServiceError::Vector(
                "quantization lifecycle entry has a non-quantized descriptor".into(),
            ));
        }
    };
    let as_u64 = |value: usize, label: &str| {
        u64::try_from(value)
            .map_err(|_| ServiceError::Vector(format!("quantization {label} exceeds u64")))
    };
    Ok(VectorQuantizationArtifactSnapshot {
        artifact_id: CanonicalId::new(stamp.id.as_str())
            .map_err(|error| ServiceError::Vector(error.to_string()))?,
        collection_id: CanonicalId::new(entry.collection_id.as_str())
            .map_err(|error| ServiceError::Vector(error.to_string()))?,
        vector_name: CanonicalId::new(entry.vector_name.as_str())
            .map_err(|error| ServiceError::Vector(error.to_string()))?,
        maximum_compression_ratio: method.maximum_compression_ratio(),
        method,
        state: match state {
            rrd_vector::QuantizationArtifactState::Ready => VectorQuantizationArtifactState::Ready,
            rrd_vector::QuantizationArtifactState::Active => {
                VectorQuantizationArtifactState::Active
            }
            rrd_vector::QuantizationArtifactState::Retired => {
                VectorQuantizationArtifactState::Retired
            }
        },
        generation: stamp.generation,
        source_cursor: stamp.source_cursor,
        indexed_vectors: as_u64(indexed_vectors, "indexed vectors")?,
        packed_vector_bytes: as_u64(packed, "packed bytes")?,
        full_precision_vector_bytes: as_u64(full, "full-precision bytes")?,
        auxiliary_bytes: as_u64(auxiliary, "auxiliary bytes")?,
        configuration_sha256: stamp.config_digest.clone(),
        artifact_sha256: stamp.artifact_digest.clone(),
        object_sha256: entry.object.sha256.clone(),
        object_length: entry.object.length,
        built_at_unix_ms: entry.built_at,
        lifecycle_revision,
    })
}
