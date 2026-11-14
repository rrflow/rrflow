//! Public and internal collection mappings, payload-index validation and collection history.

use super::*;

pub(super) fn validate_payload_index_values(
    collection: &rrd_vector::CollectionEntry,
    properties: &DataProperties,
) -> Result<()> {
    for index in collection.payload_indexes.values() {
        let Some(value) = properties.get(index.definition.field.as_str()) else {
            continue;
        };
        let valid = matches!(
            (index.definition.kind, value),
            (rrd_vector::PayloadIndexKind::Boolean, QueryValue::Bool(_))
                | (
                    rrd_vector::PayloadIndexKind::Integer,
                    QueryValue::Integer(_)
                )
                | (
                    rrd_vector::PayloadIndexKind::Unsigned,
                    QueryValue::Unsigned(_)
                )
                | (
                    rrd_vector::PayloadIndexKind::Decimal,
                    QueryValue::Decimal(_)
                )
                | (rrd_vector::PayloadIndexKind::Keyword, QueryValue::String(_))
                | (rrd_vector::PayloadIndexKind::Digest, QueryValue::Digest(_))
        );
        if !valid {
            return Err(ServiceError::Vector(format!(
                "payload property {} differs from its {:?} index contract",
                index.definition.field, index.definition.kind
            )));
        }
    }
    Ok(())
}

pub(in crate::engine) fn vector_collection_error(
    error: rrd_vector::CollectionError,
) -> ServiceError {
    if matches!(error, rrd_vector::CollectionError::IdempotencyConflict) {
        ServiceError::IdempotencyConflict
    } else {
        ServiceError::Vector(error.to_string())
    }
}

pub(in crate::engine) fn internal_vector_metric(
    metric: VectorSearchMetric,
) -> rrd_vector::ScoreMetric {
    match metric {
        VectorSearchMetric::Cosine => rrd_vector::ScoreMetric::Cosine,
        VectorSearchMetric::Dot => rrd_vector::ScoreMetric::Dot,
        VectorSearchMetric::Euclidean => rrd_vector::ScoreMetric::Euclidean,
        VectorSearchMetric::Manhattan => rrd_vector::ScoreMetric::Manhattan,
    }
}

pub(super) fn public_vector_metric(metric: rrd_vector::ScoreMetric) -> VectorSearchMetric {
    match metric {
        rrd_vector::ScoreMetric::Cosine => VectorSearchMetric::Cosine,
        rrd_vector::ScoreMetric::Dot => VectorSearchMetric::Dot,
        rrd_vector::ScoreMetric::Euclidean => VectorSearchMetric::Euclidean,
        rrd_vector::ScoreMetric::Manhattan => VectorSearchMetric::Manhattan,
    }
}

pub(super) fn internal_vector_collection(
    request: &EnsureVectorCollection,
) -> Result<rrd_vector::VectorCollectionDefinition> {
    let vectors = request
        .vectors
        .iter()
        .map(|vector| {
            let name = ProjectionId::new(vector.name.as_str()).map_err(core_vector)?;
            Ok((
                name.clone(),
                rrd_vector::NamedVectorConfig {
                    name,
                    field: vector.field.as_str().into(),
                    kind: match vector.kind {
                        VectorValueKind::Dense => rrd_vector::VectorValueKind::Dense,
                        VectorValueKind::Sparse => rrd_vector::VectorValueKind::Sparse,
                        VectorValueKind::MultiDense => rrd_vector::VectorValueKind::MultiDense,
                    },
                    dimensions: vector.dimensions,
                    metric: internal_vector_metric(vector.metric),
                    embedding_model: vector.embedding_model.as_ref().map(|model| {
                        rrd_vector::EmbeddingModelBinding {
                            name: model.name.clone(),
                            digest: model.digest.clone(),
                        }
                    }),
                    memory_tier: match vector.memory_tier {
                        VectorMemoryTier::Pinned => rrd_vector::VectorMemoryTier::Pinned,
                        VectorMemoryTier::Cached => rrd_vector::VectorMemoryTier::Cached,
                        VectorMemoryTier::Cold => rrd_vector::VectorMemoryTier::Cold,
                    },
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok(rrd_vector::VectorCollectionDefinition {
        id: ProjectionId::new(request.collection_id.as_str()).map_err(core_vector)?,
        vectors,
    })
}

pub(in crate::engine) fn public_vector_collection(
    entry: &rrd_vector::CollectionEntry,
) -> Result<VectorCollectionSnapshot> {
    Ok(VectorCollectionSnapshot {
        collection_id: CanonicalId::new(entry.definition.id.as_str())
            .map_err(|error| ServiceError::Vector(error.to_string()))?,
        vectors: entry
            .definition
            .vectors
            .values()
            .map(|vector| {
                Ok(NamedVectorDefinition {
                    name: CanonicalId::new(vector.name.as_str())
                        .map_err(|error| ServiceError::Vector(error.to_string()))?,
                    field: CanonicalId::new(&vector.field)
                        .map_err(|error| ServiceError::Vector(error.to_string()))?,
                    kind: match vector.kind {
                        rrd_vector::VectorValueKind::Dense => VectorValueKind::Dense,
                        rrd_vector::VectorValueKind::Sparse => VectorValueKind::Sparse,
                        rrd_vector::VectorValueKind::MultiDense => VectorValueKind::MultiDense,
                    },
                    dimensions: vector.dimensions,
                    metric: public_vector_metric(vector.metric),
                    embedding_model: vector.embedding_model.as_ref().map(|model| {
                        VectorEmbeddingModel {
                            name: model.name.clone(),
                            digest: model.digest.clone(),
                        }
                    }),
                    memory_tier: match vector.memory_tier {
                        rrd_vector::VectorMemoryTier::Pinned => VectorMemoryTier::Pinned,
                        rrd_vector::VectorMemoryTier::Cached => VectorMemoryTier::Cached,
                        rrd_vector::VectorMemoryTier::Cold => VectorMemoryTier::Cold,
                    },
                })
            })
            .collect::<Result<_>>()?,
        payload_indexes: entry
            .payload_indexes
            .values()
            .map(public_payload_index)
            .collect::<Result<_>>()?,
        generation: entry.generation,
        created_at_unix_ms: entry.created_at,
        updated_at_unix_ms: entry.updated_at,
        configuration_sha256: entry.configuration_digest.clone(),
    })
}

pub(super) fn collection_mutation_context(
    session_id: &CorrelationId,
    context: &RequestContext,
    now: u64,
) -> rrd_vector::CollectionMutationContext {
    rrd_vector::CollectionMutationContext {
        at: now,
        actor: format!("session:{}", session_id.as_str()),
        request_id: context.request_id.as_str().into(),
        operation_id: context.operation_id.as_str().into(),
    }
}

pub(super) fn internal_payload_index_kind(
    kind: VectorPayloadIndexKind,
) -> rrd_vector::PayloadIndexKind {
    match kind {
        VectorPayloadIndexKind::Boolean => rrd_vector::PayloadIndexKind::Boolean,
        VectorPayloadIndexKind::Integer => rrd_vector::PayloadIndexKind::Integer,
        VectorPayloadIndexKind::Unsigned => rrd_vector::PayloadIndexKind::Unsigned,
        VectorPayloadIndexKind::Decimal => rrd_vector::PayloadIndexKind::Decimal,
        VectorPayloadIndexKind::Keyword => rrd_vector::PayloadIndexKind::Keyword,
        VectorPayloadIndexKind::Digest => rrd_vector::PayloadIndexKind::Digest,
    }
}

pub(super) fn public_payload_index_kind(
    kind: rrd_vector::PayloadIndexKind,
) -> VectorPayloadIndexKind {
    match kind {
        rrd_vector::PayloadIndexKind::Boolean => VectorPayloadIndexKind::Boolean,
        rrd_vector::PayloadIndexKind::Integer => VectorPayloadIndexKind::Integer,
        rrd_vector::PayloadIndexKind::Unsigned => VectorPayloadIndexKind::Unsigned,
        rrd_vector::PayloadIndexKind::Decimal => VectorPayloadIndexKind::Decimal,
        rrd_vector::PayloadIndexKind::Keyword => VectorPayloadIndexKind::Keyword,
        rrd_vector::PayloadIndexKind::Digest => VectorPayloadIndexKind::Digest,
    }
}

pub(super) fn public_payload_index(
    entry: &rrd_vector::PayloadIndexEntry,
) -> Result<VectorPayloadIndexSnapshot> {
    Ok(VectorPayloadIndexSnapshot {
        field: CanonicalId::new(entry.definition.field.as_str())
            .map_err(|error| ServiceError::Vector(error.to_string()))?,
        kind: public_payload_index_kind(entry.definition.kind),
        generation: entry.generation,
        created_at_unix_ms: entry.created_at,
        updated_at_unix_ms: entry.updated_at,
        configuration_sha256: entry.configuration_digest.clone(),
    })
}

pub(super) fn require_no_collection_artifacts(
    engine: &RrdEngine,
    scope: &ScopeId,
    collection_id: &CanonicalId,
    collection: &rrd_vector::CollectionEntry,
) -> Result<()> {
    let entries = crate::vector_artifact_catalog_entries(&engine.storage, scope)
        .map_err(|error| ServiceError::Vector(error.to_string()))?;
    for vector in collection.definition.vectors.values() {
        if entries
            .iter()
            .any(|entry| hnsw_may_belong_to_source(entry, collection_id, &vector.name))
        {
            return Err(ServiceError::Vector(format!(
                "vector collection {collection_id} still owns an active HNSW artifact; retire it before deletion"
            )));
        }
    }
    let quantization = crate::quantization_artifact_catalogue(&engine.storage, scope)
        .map_err(|error| ServiceError::Vector(error.to_string()))?;
    if quantization.artifacts.values().any(|artifact| {
        artifact.entry.collection_id.as_str() == collection_id.as_str()
            && artifact.state != rrd_vector::QuantizationArtifactState::Retired
    }) {
        return Err(ServiceError::Vector(format!(
            "vector collection {collection_id} still owns ready or active quantization artifacts; retire them before deletion"
        )));
    }
    Ok(())
}

pub(super) fn require_payload_index_not_in_use(
    engine: &RrdEngine,
    scope: &ScopeId,
    collection_id: &CanonicalId,
    collection: &rrd_vector::CollectionEntry,
    field: &str,
) -> Result<()> {
    let entries = crate::vector_artifact_catalog_entries(&engine.storage, scope)
        .map_err(|error| ServiceError::Vector(error.to_string()))?;
    for vector in collection.definition.vectors.values() {
        let kind = "hnsw";
        for entry in entries
            .iter()
            .filter(|entry| hnsw_may_belong_to_source(entry, collection_id, &vector.name))
        {
            let rrd_vector::VectorProjectionDescriptor::Hnsw { descriptor } = &entry.descriptor
            else {
                return Err(ServiceError::Vector(
                    "generic vector index catalogue contains a non-HNSW named index".into(),
                ));
            };
            let properties = &descriptor.filter_properties;
            if properties.contains(field) {
                return Err(ServiceError::Vector(format!(
                    "payload index {field} is required by active {kind} artifact {}; retire or replace the artifact before deletion",
                    entry.descriptor.stamp().id
                )));
            }
        }
    }
    let quantization = crate::quantization_artifact_catalogue(&engine.storage, scope)
        .map_err(|error| ServiceError::Vector(error.to_string()))?;
    for artifact in quantization.artifacts.values().filter(|artifact| {
        artifact.entry.collection_id.as_str() == collection_id.as_str()
            && artifact.state != rrd_vector::QuantizationArtifactState::Retired
    }) {
        let properties = match &artifact.entry.descriptor {
            rrd_vector::VectorProjectionDescriptor::Quantized { descriptor } => {
                &descriptor.filter_properties
            }
            rrd_vector::VectorProjectionDescriptor::TurboQuant { descriptor } => {
                &descriptor.filter_properties
            }
            _ => continue,
        };
        if properties.contains(field) {
            return Err(ServiceError::Vector(format!(
                "payload index {field} is required by quantization artifact {}; retire it before deletion",
                artifact.entry.descriptor.stamp().id
            )));
        }
    }
    Ok(())
}

pub(super) fn hnsw_may_belong_to_source(
    entry: &rrd_vector::VectorArtifactCatalogEntry,
    collection_id: &CanonicalId,
    vector_name: &ProjectionId,
) -> bool {
    if !matches!(
        entry.descriptor,
        rrd_vector::VectorProjectionDescriptor::Hnsw { .. }
    ) {
        return false;
    }
    match entry.source() {
        Some(source) => {
            source.collection_id == collection_id.as_str()
                && source.vector_name == vector_name.as_str()
        }
        // The former name is ambiguous, so use it only to conservatively
        // refuse deletion, never to authorize a read or establish ownership.
        None if entry.contract_version == 2 => {
            entry.descriptor.stamp().id.as_str() == format!("hnsw-{collection_id}-{vector_name}")
        }
        None => false,
    }
}

pub(in crate::engine) fn validate_collection_query(
    config: &rrd_vector::NamedVectorConfig,
    query: &VectorSearchQuery,
) -> Result<()> {
    let (kind, dimensions) = match query {
        VectorSearchQuery::Dense { values } => (
            rrd_vector::VectorValueKind::Dense,
            u32::try_from(values.len())
                .map_err(|_| ServiceError::Vector("query dimensions exceed u32".into()))?,
        ),
        VectorSearchQuery::Sparse { dimensions, .. } => {
            (rrd_vector::VectorValueKind::Sparse, *dimensions)
        }
        VectorSearchQuery::MultiDense { dimensions, .. } => {
            (rrd_vector::VectorValueKind::MultiDense, *dimensions)
        }
    };
    if config.kind != kind || config.dimensions != dimensions {
        return Err(ServiceError::Vector(
            "query vector kind or dimensions differ from the named-vector contract".into(),
        ));
    }
    Ok(())
}

pub(in crate::engine) fn vector_matches_collection(
    vector: &RuntimeVector,
    collection_id: &CanonicalId,
    vector_name: &CanonicalId,
) -> bool {
    vector.collection.collection_id == collection_id.as_str()
        && vector.collection.vector_name == vector_name.as_str()
}

/// What a generation built for `collection_id`/`vector_name` holds of
/// `candidates`, which must be complete histories in every source, and the
/// cursor it covers (POAM-048). A move out of the address ends what the
/// generation holds of the identity where the move takes effect.
pub(in crate::engine) fn source_history(
    candidates: impl IntoIterator<Item = rrd_vector::VectorCandidate>,
    collection_id: &CanonicalId,
    vector_name: &CanonicalId,
) -> rrd_vector::SourceHistory {
    rrd_vector::source_history(candidates, |candidate| {
        vector_matches_collection(&candidate.vector, collection_id, vector_name)
    })
}

/// The versions in `collection_id`/`vector_name` of every identity the
/// collection holds at `visibility`'s valid time and read cursor: those whose
/// latest version there names this address. An identity whose latest version
/// names another collection or named vector has moved, and none of its
/// versions here may answer (POAM-045). `candidates` must hold each
/// identity's complete history.
pub(in crate::engine) fn collection_versions(
    visibility: &rrd_vector::VectorVisibilityRequest,
    candidates: Vec<rrd_vector::VectorCandidate>,
    collection_id: &CanonicalId,
    vector_name: &CanonicalId,
) -> Result<Vec<rrd_vector::VectorCandidate>> {
    let held = rrd_vector::latest_versions(visibility, &candidates)
        .map_err(core_vector)?
        .into_values()
        .filter(|latest| vector_matches_collection(&latest.vector, collection_id, vector_name))
        .map(|latest| latest.vector.reference.clone())
        .collect::<std::collections::BTreeSet<_>>();
    Ok(candidates
        .into_iter()
        .filter(|candidate| {
            held.contains(&candidate.vector.reference)
                && vector_matches_collection(&candidate.vector, collection_id, vector_name)
        })
        .collect())
}
