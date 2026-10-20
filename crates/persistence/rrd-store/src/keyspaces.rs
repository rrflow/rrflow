//! Canonical semantic key shapes layered on the rrflowKV tuple codec.
//!
//! Callers name a semantic shape; they never concatenate physical delimiters.
//! The codec places every shape below the manifest-authenticated
//! `format / tenant / scope / family` prefix and this module freezes the first
//! component that subdivides a family.

use crate::error::{Error, Result};
use crate::key_codec::{
    CatalogueSubfamily, DecodedKeyPart, KeyAddress, KeyCodec, KeyFamily, KeyPart,
};
use rrd_core::{
    Millis, Predicate, ProjectionFamily, ProjectionId, ProjectionWork, Reader, RuntimeEvent,
    RuntimeRef, RuntimeRelation, RuntimeValue, ScopeId, Subject,
};

pub(crate) use crate::key_codec::APPLICATION_FORMAT as RRFLOW_KV_FORMAT;
pub(crate) type RrflowKvKeyCodec = KeyCodec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Space {
    Claims,
    SequenceIndex,
    Access,
    System,
    Invocations,
    FunctionArtifacts,
    FunctionDefinitions,
    TransactionFunctionBindings,
    FunctionCatalogueMemberships,
    FunctionCatalogueHeads,
    Projections,
    RuntimeChanges,
    RuntimeClaimVersions,
    RuntimeRecords,
    RuntimeRecordVersions,
    RuntimeRelations,
    RuntimeRelationVersions,
    RuntimeEventVersions,
    RuntimeOutgoingEdges,
    RuntimeOutgoingEdgeVersions,
    RuntimeIncomingEdges,
    RuntimeIncomingEdgeVersions,
    RuntimeVectors,
    RuntimeVectorVersions,
    RuntimeSeries,
    RuntimeSeriesVersions,
    RuntimeGeo,
    RuntimeGeoVersions,
    RuntimeObjects,
    RuntimeObjectVersions,
    RuntimeProjectionDeltas,
    RuntimeIndexSourceDeltas,
    RuntimeVectorSourceDeltas,
    RuntimeIndexBindings,
    RuntimeIndexBindingVersions,
    RuntimeIndexBindingSets,
    RuntimeScalarEntries,
    RuntimeUniqueEntries,
    RuntimeOutbox,
    RuntimeAudit,
    RuntimeCommits,
    RuntimeSchemas,
    RuntimeSchemaVersions,
    RuntimeSnapshots,
}

pub(crate) const CLAIMS: Space = Space::Claims;
pub(crate) const SEQUENCE_INDEX: Space = Space::SequenceIndex;
pub(crate) const ACCESS: Space = Space::Access;
pub(crate) const META: Space = Space::System;
pub(crate) const INVOCATIONS: Space = Space::Invocations;
pub(crate) const FUNCTION_ARTIFACTS: Space = Space::FunctionArtifacts;
pub(crate) const FUNCTION_DEFINITIONS: Space = Space::FunctionDefinitions;
pub(crate) const TRANSACTION_FUNCTION_BINDINGS: Space = Space::TransactionFunctionBindings;
pub(crate) const FUNCTION_CATALOGUE_MEMBERSHIPS: Space = Space::FunctionCatalogueMemberships;
pub(crate) const FUNCTION_CATALOGUE_HEADS: Space = Space::FunctionCatalogueHeads;
pub(crate) const PROJECTIONS: Space = Space::Projections;
pub(crate) const RUNTIME_CHANGES: Space = Space::RuntimeChanges;
pub(crate) const RUNTIME_CLAIM_VERSIONS: Space = Space::RuntimeClaimVersions;
pub(crate) const RUNTIME_RECORDS: Space = Space::RuntimeRecords;
pub(crate) const RUNTIME_RECORD_VERSIONS: Space = Space::RuntimeRecordVersions;
pub(crate) const RUNTIME_RELATIONS: Space = Space::RuntimeRelations;
pub(crate) const RUNTIME_RELATION_VERSIONS: Space = Space::RuntimeRelationVersions;
pub(crate) const RUNTIME_EVENT_VERSIONS: Space = Space::RuntimeEventVersions;
pub(crate) const RUNTIME_OUTGOING_EDGES: Space = Space::RuntimeOutgoingEdges;
pub(crate) const RUNTIME_OUTGOING_EDGE_VERSIONS: Space = Space::RuntimeOutgoingEdgeVersions;
pub(crate) const RUNTIME_INCOMING_EDGES: Space = Space::RuntimeIncomingEdges;
pub(crate) const RUNTIME_INCOMING_EDGE_VERSIONS: Space = Space::RuntimeIncomingEdgeVersions;
pub(crate) const RUNTIME_VECTORS: Space = Space::RuntimeVectors;
pub(crate) const RUNTIME_VECTOR_VERSIONS: Space = Space::RuntimeVectorVersions;
pub(crate) const RUNTIME_SERIES: Space = Space::RuntimeSeries;
pub(crate) const RUNTIME_SERIES_VERSIONS: Space = Space::RuntimeSeriesVersions;
pub(crate) const RUNTIME_GEO: Space = Space::RuntimeGeo;
pub(crate) const RUNTIME_GEO_VERSIONS: Space = Space::RuntimeGeoVersions;
pub(crate) const RUNTIME_OBJECTS: Space = Space::RuntimeObjects;
pub(crate) const RUNTIME_OBJECT_VERSIONS: Space = Space::RuntimeObjectVersions;
pub(crate) const RUNTIME_PROJECTION_DELTAS: Space = Space::RuntimeProjectionDeltas;
pub(crate) const RUNTIME_INDEX_SOURCE_DELTAS: Space = Space::RuntimeIndexSourceDeltas;
pub(crate) const RUNTIME_VECTOR_SOURCE_DELTAS: Space = Space::RuntimeVectorSourceDeltas;
pub(crate) const RUNTIME_INDEX_BINDINGS: Space = Space::RuntimeIndexBindings;
pub(crate) const RUNTIME_INDEX_BINDING_VERSIONS: Space = Space::RuntimeIndexBindingVersions;
pub(crate) const RUNTIME_INDEX_BINDING_SETS: Space = Space::RuntimeIndexBindingSets;
pub(crate) const RUNTIME_SCALAR_ENTRIES: Space = Space::RuntimeScalarEntries;
pub(crate) const RUNTIME_UNIQUE_ENTRIES: Space = Space::RuntimeUniqueEntries;
pub(crate) const RUNTIME_OUTBOX: Space = Space::RuntimeOutbox;
pub(crate) const RUNTIME_AUDIT: Space = Space::RuntimeAudit;
pub(crate) const RUNTIME_COMMITS: Space = Space::RuntimeCommits;
pub(crate) const RUNTIME_SCHEMAS: Space = Space::RuntimeSchemas;
pub(crate) const RUNTIME_SCHEMA_VERSIONS: Space = Space::RuntimeSchemaVersions;
pub(crate) const RUNTIME_SNAPSHOTS: Space = Space::RuntimeSnapshots;

impl Space {
    const fn family(self) -> KeyFamily {
        match self {
            Self::Claims
            | Self::SequenceIndex
            | Self::RuntimeClaimVersions
            | Self::RuntimeRecordVersions
            | Self::RuntimeRelationVersions
            | Self::RuntimeSeriesVersions
            | Self::RuntimeGeoVersions
            | Self::RuntimeObjectVersions => KeyFamily::Temporal,
            Self::Access | Self::RuntimeAudit => KeyFamily::Audit,
            Self::System => KeyFamily::System,
            Self::Invocations
            | Self::FunctionArtifacts
            | Self::FunctionDefinitions
            | Self::TransactionFunctionBindings
            | Self::FunctionCatalogueMemberships
            | Self::FunctionCatalogueHeads
            | Self::RuntimeSchemas
            | Self::RuntimeSchemaVersions
            | Self::RuntimeSnapshots
            | Self::RuntimeIndexBindings
            | Self::RuntimeIndexBindingVersions
            | Self::RuntimeIndexBindingSets => KeyFamily::Catalogue,
            Self::Projections
            | Self::RuntimeProjectionDeltas
            | Self::RuntimeIndexSourceDeltas
            | Self::RuntimeVectorSourceDeltas => KeyFamily::ProjectionDelta,
            Self::RuntimeChanges | Self::RuntimeEventVersions => KeyFamily::EngineEvent,
            Self::RuntimeRecords
            | Self::RuntimeRelations
            | Self::RuntimeSeries
            | Self::RuntimeGeo
            | Self::RuntimeObjects => KeyFamily::Current,
            Self::RuntimeVectors | Self::RuntimeVectorVersions => KeyFamily::Vector,
            Self::RuntimeScalarEntries => KeyFamily::Scalar,
            Self::RuntimeUniqueEntries => KeyFamily::Unique,
            Self::RuntimeOutgoingEdges | Self::RuntimeOutgoingEdgeVersions => {
                KeyFamily::OutgoingEdge
            }
            Self::RuntimeIncomingEdges | Self::RuntimeIncomingEdgeVersions => {
                KeyFamily::IncomingEdge
            }
            Self::RuntimeOutbox => KeyFamily::Outbox,
            Self::RuntimeCommits => KeyFamily::RuntimeCommit,
        }
    }

    const fn subspace(self) -> u8 {
        match self {
            Self::Claims => 0x01,
            Self::SequenceIndex => 0x02,
            Self::RuntimeRecordVersions => 0x03,
            Self::RuntimeRelationVersions => 0x04,
            Self::RuntimeClaimVersions => 0x05,
            Self::RuntimeSeriesVersions => 0x06,
            Self::RuntimeGeoVersions => 0x07,
            Self::RuntimeObjectVersions => 0x08,
            Self::Access => 0x01,
            Self::System => 0x01,
            // This is the canonical catalogue receipt subfamily, not a second
            // invocation namespace.
            Self::Invocations => CatalogueSubfamily::InvocationReceipt as u8,
            Self::FunctionArtifacts => CatalogueSubfamily::FunctionArtifact as u8,
            Self::FunctionDefinitions => CatalogueSubfamily::FunctionDefinition as u8,
            Self::TransactionFunctionBindings => {
                CatalogueSubfamily::TransactionFunctionBinding as u8
            }
            Self::FunctionCatalogueMemberships => CatalogueSubfamily::Membership as u8,
            Self::FunctionCatalogueHeads => CatalogueSubfamily::Head as u8,
            Self::RuntimeSchemas => 0x20,
            Self::RuntimeSnapshots => 0x21,
            Self::RuntimeSchemaVersions => 0x22,
            Self::RuntimeIndexBindings => 0x30,
            Self::RuntimeIndexBindingVersions => 0x31,
            Self::RuntimeIndexBindingSets => 0x32,
            Self::Projections => 0x01,
            Self::RuntimeProjectionDeltas => 0x02,
            Self::RuntimeIndexSourceDeltas => 0x03,
            Self::RuntimeVectorSourceDeltas => 0x04,
            Self::RuntimeChanges => 0x01,
            Self::RuntimeEventVersions => 0x02,
            Self::RuntimeRecords => 0x01,
            Self::RuntimeRelations => 0x02,
            Self::RuntimeSeries => 0x03,
            Self::RuntimeGeo => 0x04,
            Self::RuntimeObjects => 0x05,
            Self::RuntimeVectors => 0x01,
            Self::RuntimeVectorVersions => 0x02,
            Self::RuntimeScalarEntries | Self::RuntimeUniqueEntries => 0x01,
            Self::RuntimeOutgoingEdges | Self::RuntimeIncomingEdges => 0x01,
            Self::RuntimeOutgoingEdgeVersions | Self::RuntimeIncomingEdgeVersions => 0x02,
            Self::RuntimeOutbox => 0x01,
            Self::RuntimeAudit => 0x02,
            Self::RuntimeCommits => 0x01,
        }
    }
}

fn encode(space: Space, parts: &[KeyPart<'_>]) -> Vec<u8> {
    let mut tuple = Vec::with_capacity(parts.len() + 1);
    tuple.push(KeyPart::U8(space.subspace()));
    tuple.extend_from_slice(parts);
    KeyCodec.encode(KeyAddress::GLOBAL, space.family(), &tuple)
}

pub(crate) fn space_prefix(space: Space) -> Vec<u8> {
    encode(space, &[])
}

fn decode_space(codec: KeyCodec, space: Space, stored: &[u8]) -> Result<Vec<DecodedKeyPart>> {
    let decoded = codec.decode(stored)?;
    if decoded.tenant.is_some()
        || decoded.scope.is_some()
        || decoded.family != space.family()
        || decoded.parts.first() != Some(&DecodedKeyPart::U8(space.subspace()))
    {
        return Err(Error::Codec(format!(
            "key escaped canonical rrflowKV space {space:?}"
        )));
    }
    Ok(decoded.parts.into_iter().skip(1).collect())
}

pub(crate) fn validate_space(codec: KeyCodec, space: Space, stored: &[u8]) -> Result<()> {
    decode_space(codec, space, stored).map(|_| ())
}

fn expect_parts<const N: usize>(
    codec: KeyCodec,
    space: Space,
    stored: &[u8],
) -> Result<[DecodedKeyPart; N]> {
    decode_space(codec, space, stored)?
        .try_into()
        .map_err(|_| Error::Codec(format!("{space:?} key has the wrong field count")))
}

pub(crate) fn claim_key(
    subject: &Subject,
    predicate: &Predicate,
    valid_from: Millis,
    tx_time: Millis,
) -> Vec<u8> {
    encode(
        CLAIMS,
        &[
            KeyPart::Text(subject.as_str()),
            KeyPart::Text(predicate.as_str()),
            KeyPart::DescU64(valid_from),
            KeyPart::DescU64(tx_time),
        ],
    )
}

pub(crate) fn claim_subject_prefix(subject: &Subject) -> Vec<u8> {
    encode(CLAIMS, &[KeyPart::Text(subject.as_str())])
}

pub(crate) fn claim_version_prefix(subject: &Subject, predicate: &Predicate) -> Vec<u8> {
    encode(
        CLAIMS,
        &[
            KeyPart::Text(subject.as_str()),
            KeyPart::Text(predicate.as_str()),
        ],
    )
}

pub(crate) fn claim_seek_key(subject: &Subject, predicate: &Predicate, as_of: Millis) -> Vec<u8> {
    encode(
        CLAIMS,
        &[
            KeyPart::Text(subject.as_str()),
            KeyPart::Text(predicate.as_str()),
            KeyPart::DescU64(as_of),
        ],
    )
}

pub(crate) fn parse_claim_key(
    codec: KeyCodec,
    stored: &[u8],
) -> Result<(Subject, Predicate, Millis, Millis)> {
    let [DecodedKeyPart::Text(subject), DecodedKeyPart::Text(predicate), DecodedKeyPart::DescU64(valid_from), DecodedKeyPart::DescU64(tx_time)] =
        expect_parts(codec, CLAIMS, stored)?
    else {
        return Err(Error::Codec("claim key has invalid field types".into()));
    };
    Ok((
        Subject::new(subject)?,
        Predicate::new(predicate)?,
        valid_from,
        tx_time,
    ))
}

pub(crate) fn sequence_key(sequence: u64) -> Vec<u8> {
    encode(SEQUENCE_INDEX, &[KeyPart::U64(sequence)])
}

pub(crate) fn parse_sequence_key(codec: KeyCodec, stored: &[u8]) -> Result<u64> {
    let [DecodedKeyPart::U64(sequence)] = expect_parts(codec, SEQUENCE_INDEX, stored)? else {
        return Err(Error::Codec(
            "claim sequence key has invalid field types".into(),
        ));
    };
    Ok(sequence)
}

pub(crate) fn access_key(
    at: Millis,
    reader: &Reader,
    subject: &Subject,
    predicate: &Predicate,
) -> Vec<u8> {
    encode(
        ACCESS,
        &[
            KeyPart::U64(at),
            KeyPart::Text(reader.as_str()),
            KeyPart::Text(subject.as_str()),
            KeyPart::Text(predicate.as_str()),
        ],
    )
}

pub(crate) fn access_bound(at: Millis) -> Vec<u8> {
    encode(ACCESS, &[KeyPart::U64(at)])
}

pub(crate) fn parse_access_key(
    codec: KeyCodec,
    stored: &[u8],
) -> Result<(Millis, Reader, Subject, Predicate)> {
    let [DecodedKeyPart::U64(at), DecodedKeyPart::Text(reader), DecodedKeyPart::Text(subject), DecodedKeyPart::Text(predicate)] =
        expect_parts(codec, ACCESS, stored)?
    else {
        return Err(Error::Codec("access key has invalid field types".into()));
    };
    Ok((
        at,
        Reader::new(reader)?,
        Subject::new(subject)?,
        Predicate::new(predicate)?,
    ))
}

pub(crate) fn invocation_key(at: Millis, ordinal: u64) -> Vec<u8> {
    encode(INVOCATIONS, &[KeyPart::U64(at), KeyPart::U64(ordinal)])
}

pub(crate) fn invocation_bound(at: Millis) -> Vec<u8> {
    encode(INVOCATIONS, &[KeyPart::U64(at)])
}

pub(crate) fn function_artifact_key(instance: &str, content_sha256: &str) -> Result<Vec<u8>> {
    let digest = decode_sha256(content_sha256)?;
    Ok(encode(
        FUNCTION_ARTIFACTS,
        &[KeyPart::Text(instance), KeyPart::Bytes(&digest)],
    ))
}

pub(crate) fn function_definition_key(
    instance: &str,
    function_id: &str,
    definition_sha256: &str,
) -> Result<Vec<u8>> {
    let digest = decode_sha256(definition_sha256)?;
    Ok(encode(
        FUNCTION_DEFINITIONS,
        &[
            KeyPart::Text(instance),
            KeyPart::Text(function_id),
            KeyPart::Bytes(&digest),
        ],
    ))
}

pub(crate) fn transaction_function_binding_key(
    instance: &str,
    binding_id: &str,
    binding_sha256: &str,
) -> Result<Vec<u8>> {
    let digest = decode_sha256(binding_sha256)?;
    Ok(encode(
        TRANSACTION_FUNCTION_BINDINGS,
        &[
            KeyPart::Text(instance),
            KeyPart::Text(binding_id),
            KeyPart::Bytes(&digest),
        ],
    ))
}

pub(crate) fn function_catalogue_membership_key(instance: &str, revision: u64) -> Vec<u8> {
    encode(
        FUNCTION_CATALOGUE_MEMBERSHIPS,
        &[KeyPart::Text(instance), KeyPart::U64(revision)],
    )
}

pub(crate) fn function_catalogue_head_key(instance: &str) -> Vec<u8> {
    encode(FUNCTION_CATALOGUE_HEADS, &[KeyPart::Text(instance)])
}

pub(crate) fn function_invocation_receipt_key(instance: &str, invocation_id: &str) -> Vec<u8> {
    encode(
        INVOCATIONS,
        &[KeyPart::Text(instance), KeyPart::Text(invocation_id)],
    )
}

fn decode_sha256(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64 {
        return Err(Error::Codec(
            "SHA-256 key coordinate has invalid length".into(),
        ));
    }
    let mut digest = [0_u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        digest[index] = (decode_hex_digit(pair[0])? << 4) | decode_hex_digit(pair[1])?;
    }
    Ok(digest)
}

fn decode_hex_digit(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(Error::Codec(
            "SHA-256 key coordinate is not lowercase hexadecimal".into(),
        )),
    }
}

pub(crate) fn projection_key(name: &str) -> Vec<u8> {
    encode(PROJECTIONS, &[KeyPart::Text(name)])
}

pub(crate) fn runtime_change_key(cursor: u64) -> Vec<u8> {
    encode(RUNTIME_CHANGES, &[KeyPart::U64(cursor)])
}

pub(crate) fn runtime_event_reference(event: &RuntimeEvent, cursor: u64) -> Result<RuntimeRef> {
    RuntimeRef::new(event.kind.as_str(), format!("cursor:{cursor}")).map_err(Error::from)
}

pub(crate) fn runtime_identity_key(
    space: Space,
    scope: &ScopeId,
    reference: &RuntimeRef,
) -> Vec<u8> {
    debug_assert!(matches!(
        space,
        Space::RuntimeRecords
            | Space::RuntimeRelations
            | Space::RuntimeVectors
            | Space::RuntimeSeries
            | Space::RuntimeGeo
            | Space::RuntimeObjects
    ));
    encode(
        space,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(reference.kind.as_str()),
            KeyPart::Text(reference.id.as_str()),
        ],
    )
}

pub(crate) fn runtime_version_key(
    space: Space,
    scope: &ScopeId,
    reference: &RuntimeRef,
    effective_at: Millis,
    cursor: u64,
) -> Vec<u8> {
    debug_assert!(matches!(
        space,
        Space::RuntimeRecordVersions
            | Space::RuntimeRelationVersions
            | Space::RuntimeEventVersions
            | Space::RuntimeSeriesVersions
            | Space::RuntimeGeoVersions
            | Space::RuntimeObjectVersions
    ));
    encode(
        space,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(reference.kind.as_str()),
            KeyPart::Text(reference.id.as_str()),
            KeyPart::DescU64(effective_at),
            KeyPart::DescU64(cursor),
        ],
    )
}

pub(crate) fn runtime_kind_prefix(
    space: Space,
    scope: &ScopeId,
    kind: &rrd_core::RuntimeType,
) -> Vec<u8> {
    debug_assert!(matches!(
        space,
        Space::RuntimeRecordVersions
            | Space::RuntimeRelationVersions
            | Space::RuntimeEventVersions
            | Space::RuntimeVectorVersions
            | Space::RuntimeSeriesVersions
            | Space::RuntimeGeoVersions
            | Space::RuntimeObjectVersions
    ));
    encode(
        space,
        &[KeyPart::Text(scope.as_str()), KeyPart::Text(kind.as_str())],
    )
}

pub(crate) fn runtime_reference_prefix(
    space: Space,
    scope: &ScopeId,
    reference: &RuntimeRef,
) -> Vec<u8> {
    debug_assert!(matches!(
        space,
        Space::RuntimeRecordVersions
            | Space::RuntimeRelationVersions
            | Space::RuntimeEventVersions
            | Space::RuntimeVectorVersions
            | Space::RuntimeSeriesVersions
            | Space::RuntimeGeoVersions
            | Space::RuntimeObjectVersions
    ));
    encode(
        space,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(reference.kind.as_str()),
            KeyPart::Text(reference.id.as_str()),
        ],
    )
}

pub(crate) fn runtime_claim_version_key(
    scope: &ScopeId,
    claim: &rrd_core::Claim,
    cursor: u64,
) -> Vec<u8> {
    encode(
        RUNTIME_CLAIM_VERSIONS,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(claim.predicate.as_str()),
            KeyPart::Text(claim.subject.as_str()),
            KeyPart::DescU64(claim.valid_from),
            KeyPart::DescU64(claim.tx_time),
            KeyPart::DescU64(cursor),
        ],
    )
}

pub(crate) fn runtime_claim_predicate_prefix(scope: &ScopeId, predicate: &Predicate) -> Vec<u8> {
    encode(
        RUNTIME_CLAIM_VERSIONS,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(predicate.as_str()),
        ],
    )
}

pub(crate) fn runtime_adjacency_key(
    space: Space,
    scope: &ScopeId,
    relation: &RuntimeRelation,
) -> Vec<u8> {
    debug_assert!(matches!(
        space,
        Space::RuntimeOutgoingEdges | Space::RuntimeIncomingEdges
    ));
    let (source, target) = adjacency_endpoints(space, relation);
    encode(
        space,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(source.kind.as_str()),
            KeyPart::Text(source.id.as_str()),
            KeyPart::Text(relation.reference.kind.as_str()),
            KeyPart::Text(target.kind.as_str()),
            KeyPart::Text(target.id.as_str()),
            KeyPart::Text(relation.reference.id.as_str()),
        ],
    )
}

pub(crate) fn runtime_adjacency_version_key(
    space: Space,
    scope: &ScopeId,
    relation: &RuntimeRelation,
    effective_at: Millis,
    cursor: u64,
) -> Vec<u8> {
    debug_assert!(matches!(
        space,
        Space::RuntimeOutgoingEdgeVersions | Space::RuntimeIncomingEdgeVersions
    ));
    let (source, target) = adjacency_endpoints(space, relation);
    encode(
        space,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(source.kind.as_str()),
            KeyPart::Text(source.id.as_str()),
            KeyPart::Text(relation.reference.kind.as_str()),
            KeyPart::Text(target.kind.as_str()),
            KeyPart::Text(target.id.as_str()),
            KeyPart::Text(relation.reference.id.as_str()),
            KeyPart::DescU64(effective_at),
            KeyPart::DescU64(cursor),
        ],
    )
}

/// Temporal adjacency range for one node and relation type. Every version
/// whose relation had `node` at the space's near endpoint, plus every
/// retirement of such a relation, sorts under this prefix.
pub(crate) fn runtime_adjacency_prefix(
    space: Space,
    scope: &ScopeId,
    node: &RuntimeRef,
    relation_kind: &rrd_core::RuntimeType,
) -> Vec<u8> {
    debug_assert!(matches!(
        space,
        Space::RuntimeOutgoingEdgeVersions | Space::RuntimeIncomingEdgeVersions
    ));
    encode(
        space,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(node.kind.as_str()),
            KeyPart::Text(node.id.as_str()),
            KeyPart::Text(relation_kind.as_str()),
        ],
    )
}

/// Strictly decoded temporal adjacency key. `node` is the space's near
/// endpoint and `other` the far endpoint recorded when the version was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeAdjacencyVersionKey {
    pub(crate) scope: ScopeId,
    pub(crate) node: RuntimeRef,
    pub(crate) relation: RuntimeRef,
    pub(crate) other: RuntimeRef,
    pub(crate) effective_at: Millis,
    pub(crate) cursor: u64,
}

pub(crate) fn parse_runtime_adjacency_version_key(
    codec: KeyCodec,
    space: Space,
    stored: &[u8],
) -> Result<RuntimeAdjacencyVersionKey> {
    debug_assert!(matches!(
        space,
        Space::RuntimeOutgoingEdgeVersions | Space::RuntimeIncomingEdgeVersions
    ));
    let [DecodedKeyPart::Text(scope), DecodedKeyPart::Text(node_kind), DecodedKeyPart::Text(node_id), DecodedKeyPart::Text(relation_kind), DecodedKeyPart::Text(other_kind), DecodedKeyPart::Text(other_id), DecodedKeyPart::Text(relation_id), DecodedKeyPart::DescU64(effective_at), DecodedKeyPart::DescU64(cursor)] =
        expect_parts(codec, space, stored)?
    else {
        return Err(Error::Codec(
            "runtime adjacency version key has invalid field types".into(),
        ));
    };
    Ok(RuntimeAdjacencyVersionKey {
        scope: ScopeId::new(scope)?,
        node: RuntimeRef::new(node_kind, node_id)?,
        relation: RuntimeRef::new(relation_kind, relation_id)?,
        other: RuntimeRef::new(other_kind, other_id)?,
        effective_at,
        cursor,
    })
}

fn adjacency_endpoints(space: Space, relation: &RuntimeRelation) -> (&RuntimeRef, &RuntimeRef) {
    match space {
        Space::RuntimeOutgoingEdges | Space::RuntimeOutgoingEdgeVersions => {
            (&relation.from, &relation.to)
        }
        Space::RuntimeIncomingEdges | Space::RuntimeIncomingEdgeVersions => {
            (&relation.to, &relation.from)
        }
        _ => panic!("adjacency key requires an outgoing or incoming edge space"),
    }
}

pub(crate) fn runtime_projection_delta_key(work: &ProjectionWork) -> Vec<u8> {
    encode(
        RUNTIME_PROJECTION_DELTAS,
        &[
            KeyPart::U64(work.source_cursor),
            KeyPart::U64(work.commit_ordinal),
            KeyPart::U8(projection_family_tag(work.family)),
            KeyPart::Text(work.scope.as_str()),
        ],
    )
}

pub(crate) fn runtime_projection_delta_start(cursor: u64) -> Vec<u8> {
    encode(RUNTIME_PROJECTION_DELTAS, &[KeyPart::U64(cursor)])
}

pub(crate) fn runtime_index_binding_key(scope: &ScopeId, index: &ProjectionId) -> Vec<u8> {
    encode(
        RUNTIME_INDEX_BINDINGS,
        &[KeyPart::Text(scope.as_str()), KeyPart::Text(index.as_str())],
    )
}

pub(crate) fn runtime_index_binding_scope_prefix(scope: &ScopeId) -> Vec<u8> {
    encode(RUNTIME_INDEX_BINDINGS, &[KeyPart::Text(scope.as_str())])
}

pub(crate) fn runtime_index_binding_set_key(scope: &ScopeId) -> Vec<u8> {
    encode(RUNTIME_INDEX_BINDING_SETS, &[KeyPart::Text(scope.as_str())])
}

pub(crate) fn runtime_index_binding_version_key(
    scope: &ScopeId,
    index: &ProjectionId,
    catalogue_revision: u64,
    schema_revision: u64,
) -> Vec<u8> {
    encode(
        RUNTIME_INDEX_BINDING_VERSIONS,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(index.as_str()),
            KeyPart::U64(catalogue_revision),
            KeyPart::U64(schema_revision),
        ],
    )
}

pub(crate) fn runtime_index_entry_prefix(
    space: Space,
    scope: &ScopeId,
    index: &ProjectionId,
) -> Vec<u8> {
    debug_assert!(matches!(
        space,
        Space::RuntimeScalarEntries | Space::RuntimeUniqueEntries
    ));
    encode(
        space,
        &[KeyPart::Text(scope.as_str()), KeyPart::Text(index.as_str())],
    )
}

pub(crate) fn runtime_index_value_prefix<'a>(
    space: Space,
    scope: &'a ScopeId,
    index: &'a ProjectionId,
    values: &'a [RuntimeValue],
) -> Result<Vec<u8>> {
    let mut parts = vec![KeyPart::Text(scope.as_str()), KeyPart::Text(index.as_str())];
    append_index_values(&mut parts, values)?;
    Ok(encode(space, &parts))
}

pub(crate) fn runtime_index_entry_key<'a>(
    space: Space,
    scope: &'a ScopeId,
    index: &'a ProjectionId,
    values: &'a [RuntimeValue],
    reference: &'a RuntimeRef,
    valid_from: Millis,
) -> Result<Vec<u8>> {
    let mut parts = vec![KeyPart::Text(scope.as_str()), KeyPart::Text(index.as_str())];
    append_index_values(&mut parts, values)?;
    parts.extend([
        KeyPart::Text(reference.kind.as_str()),
        KeyPart::Text(reference.id.as_str()),
        KeyPart::DescU64(valid_from),
    ]);
    Ok(encode(space, &parts))
}

fn append_index_values<'a>(parts: &mut Vec<KeyPart<'a>>, values: &'a [RuntimeValue]) -> Result<()> {
    for value in values {
        match value {
            RuntimeValue::Null => parts.push(KeyPart::U8(0)),
            RuntimeValue::Bool(value) => {
                parts.extend([KeyPart::U8(1), KeyPart::Bool(*value)]);
            }
            RuntimeValue::Integer(value) => {
                parts.extend([KeyPart::U8(2), KeyPart::I64(*value)]);
            }
            RuntimeValue::Unsigned(value) => {
                parts.extend([KeyPart::U8(3), KeyPart::U64(*value)]);
            }
            RuntimeValue::Decimal(value) => {
                parts.extend([KeyPart::U8(4), KeyPart::Text(value)]);
            }
            RuntimeValue::String(value) => {
                parts.extend([KeyPart::U8(5), KeyPart::Text(value)]);
            }
            RuntimeValue::Digest(value) => {
                parts.extend([KeyPart::U8(6), KeyPart::Text(value)]);
            }
            RuntimeValue::List(_) | RuntimeValue::Map(_) => {
                return Err(Error::Codec(
                    "scalar index keys cannot contain list or map values".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn runtime_index_source_delta_key(
    scope: &ScopeId,
    index: &ProjectionId,
    cursor: u64,
    ordinal: u64,
) -> Vec<u8> {
    encode(
        RUNTIME_INDEX_SOURCE_DELTAS,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(index.as_str()),
            KeyPart::U64(cursor),
            KeyPart::U64(ordinal),
        ],
    )
}

pub(crate) fn runtime_index_source_delta_start(
    scope: &ScopeId,
    index: &ProjectionId,
    cursor: u64,
) -> Vec<u8> {
    runtime_index_source_delta_key(scope, index, cursor, 0)
}

pub(crate) fn runtime_index_source_delta_prefix(scope: &ScopeId, index: &ProjectionId) -> Vec<u8> {
    encode(
        RUNTIME_INDEX_SOURCE_DELTAS,
        &[KeyPart::Text(scope.as_str()), KeyPart::Text(index.as_str())],
    )
}

pub(crate) fn runtime_vector_version_key(
    scope: &ScopeId,
    reference: &RuntimeRef,
    effective_at: Millis,
    cursor: u64,
) -> Vec<u8> {
    encode(
        RUNTIME_VECTOR_VERSIONS,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(reference.kind.as_str()),
            KeyPart::Text(reference.id.as_str()),
            KeyPart::DescU64(effective_at),
            KeyPart::DescU64(cursor),
        ],
    )
}

pub(crate) fn runtime_vector_source_delta_key(
    scope: &ScopeId,
    collection_id: &str,
    vector_name: &str,
    field: &str,
    cursor: u64,
    ordinal: u64,
) -> Result<Vec<u8>> {
    validate_vector_source_key_parts(collection_id, vector_name, field)?;
    Ok(encode(
        RUNTIME_VECTOR_SOURCE_DELTAS,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(collection_id),
            KeyPart::Text(vector_name),
            KeyPart::Text(field),
            KeyPart::U64(cursor),
            KeyPart::U64(ordinal),
        ],
    ))
}

pub(crate) fn runtime_vector_source_delta_start(
    scope: &ScopeId,
    collection_id: &str,
    vector_name: &str,
    field: &str,
    cursor: u64,
) -> Result<Vec<u8>> {
    runtime_vector_source_delta_key(scope, collection_id, vector_name, field, cursor, 0)
}

pub(crate) fn runtime_vector_source_delta_prefix(
    scope: &ScopeId,
    collection_id: &str,
    vector_name: &str,
    field: &str,
) -> Result<Vec<u8>> {
    validate_vector_source_key_parts(collection_id, vector_name, field)?;
    Ok(encode(
        RUNTIME_VECTOR_SOURCE_DELTAS,
        &[
            KeyPart::Text(scope.as_str()),
            KeyPart::Text(collection_id),
            KeyPart::Text(vector_name),
            KeyPart::Text(field),
        ],
    ))
}

fn validate_vector_source_key_parts(
    collection_id: &str,
    vector_name: &str,
    field: &str,
) -> Result<()> {
    if collection_id.trim().is_empty() || vector_name.trim().is_empty() || field.trim().is_empty() {
        return Err(Error::IndexConstraint(
            "vector source key identity must not contain an empty part".into(),
        ));
    }
    Ok(())
}

fn projection_family_tag(family: ProjectionFamily) -> u8 {
    match family {
        ProjectionFamily::Scalar => 0,
        ProjectionFamily::Graph => 1,
        ProjectionFamily::Text => 2,
        ProjectionFamily::Vector => 3,
        ProjectionFamily::TimeSeries => 4,
        ProjectionFamily::Geo => 5,
        ProjectionFamily::Object => 6,
    }
}

pub(crate) fn runtime_scope_prefix(space: Space, scope: &ScopeId) -> Vec<u8> {
    encode(space, &[KeyPart::Text(scope.as_str())])
}

pub(crate) fn parse_runtime_identity_key(
    codec: KeyCodec,
    space: Space,
    stored: &[u8],
) -> Result<(ScopeId, RuntimeRef)> {
    let [DecodedKeyPart::Text(scope), DecodedKeyPart::Text(kind), DecodedKeyPart::Text(id)] =
        expect_parts(codec, space, stored)?
    else {
        return Err(Error::Codec(
            "runtime identity key has invalid field types".into(),
        ));
    };
    Ok((ScopeId::new(scope)?, RuntimeRef::new(kind, id)?))
}

pub(crate) fn runtime_outbox_key(cursor: u64) -> Vec<u8> {
    encode(RUNTIME_OUTBOX, &[KeyPart::U64(cursor)])
}

pub(crate) fn runtime_audit_key(commit_id: &str) -> Vec<u8> {
    encode(RUNTIME_AUDIT, &[KeyPart::Text(commit_id)])
}

pub(crate) fn runtime_commit_key(commit_id: &str) -> Vec<u8> {
    encode(RUNTIME_COMMITS, &[KeyPart::Text(commit_id)])
}

pub(crate) fn runtime_schema_key(scope: &ScopeId) -> Vec<u8> {
    encode(RUNTIME_SCHEMAS, &[KeyPart::Text(scope.as_str())])
}

pub(crate) fn runtime_schema_version_key(scope: &ScopeId, cursor: u64) -> Vec<u8> {
    encode(
        RUNTIME_SCHEMA_VERSIONS,
        &[KeyPart::Text(scope.as_str()), KeyPart::DescU64(cursor)],
    )
}

pub(crate) fn runtime_snapshot_key(id: &str) -> Vec<u8> {
    encode(RUNTIME_SNAPSHOTS, &[KeyPart::Text(id)])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum SystemEntry {
    ClaimSequence = 0x01,
    AcceptedClaimAppend = 0x02,
    ControlValue = 0x03,
    ControlJournalSequence = 0x04,
    ControlJournalLastDigest = 0x05,
    ControlJournalEntry = 0x06,
    CatalogueRevision = 0x07,
    InvocationOrdinal = 0x08,
    RuntimeCursor = 0x09,
    RuntimeLastDigest = 0x0a,
    RuntimeLastAuditDigest = 0x0b,
    RuntimeAccumulatorState = 0x0c,
    RuntimeAccumulatorNode = 0x0d,
    ControlRestoreProgress = 0x0e,
}

fn system_key(entry: SystemEntry, parts: &[KeyPart<'_>]) -> Vec<u8> {
    let mut tuple = Vec::with_capacity(parts.len() + 1);
    tuple.push(KeyPart::U8(entry as u8));
    tuple.extend_from_slice(parts);
    encode(META, &tuple)
}

pub(crate) fn sequence_watermark_key() -> Vec<u8> {
    system_key(SystemEntry::ClaimSequence, &[])
}

pub(crate) fn accepted_append_key(idempotency_key: &str) -> Vec<u8> {
    system_key(
        SystemEntry::AcceptedClaimAppend,
        &[KeyPart::Text(idempotency_key)],
    )
}

pub(crate) fn control_record_key(key: &str) -> Vec<u8> {
    system_key(SystemEntry::ControlValue, &[KeyPart::Text(key)])
}

pub(crate) fn control_journal_sequence_key() -> Vec<u8> {
    system_key(SystemEntry::ControlJournalSequence, &[])
}

pub(crate) fn control_journal_last_digest_key() -> Vec<u8> {
    system_key(SystemEntry::ControlJournalLastDigest, &[])
}

pub(crate) fn control_journal_key(sequence: u64) -> Vec<u8> {
    system_key(SystemEntry::ControlJournalEntry, &[KeyPart::U64(sequence)])
}

pub(crate) fn is_control_journal_key(codec: KeyCodec, stored: &[u8]) -> bool {
    matches!(
        decode_space(codec, META, stored).as_deref(),
        Ok([DecodedKeyPart::U8(tag), DecodedKeyPart::U64(_)])
            if *tag == SystemEntry::ControlJournalEntry as u8
    )
}

pub(crate) fn catalog_revision_key(scope: &ScopeId) -> Vec<u8> {
    system_key(
        SystemEntry::CatalogueRevision,
        &[KeyPart::Text(scope.as_str())],
    )
}

pub(crate) fn invocation_watermark_key() -> Vec<u8> {
    system_key(SystemEntry::InvocationOrdinal, &[])
}

pub(crate) fn runtime_cursor_key() -> Vec<u8> {
    system_key(SystemEntry::RuntimeCursor, &[])
}

pub(crate) fn runtime_last_digest_key() -> Vec<u8> {
    system_key(SystemEntry::RuntimeLastDigest, &[])
}

pub(crate) fn runtime_last_audit_digest_key() -> Vec<u8> {
    system_key(SystemEntry::RuntimeLastAuditDigest, &[])
}

pub(crate) fn runtime_accumulator_state_key() -> Vec<u8> {
    system_key(SystemEntry::RuntimeAccumulatorState, &[])
}

pub(crate) fn runtime_accumulator_node_key(level: u8, index: u64) -> Vec<u8> {
    system_key(
        SystemEntry::RuntimeAccumulatorNode,
        &[KeyPart::U8(level), KeyPart::U64(index)],
    )
}

/// Progress of a control-journal archive replay into this root. Written in
/// the same physical transaction as each replayed control transition, so a
/// resumed restore never re-applies or skips an entry.
pub(crate) fn control_restore_progress_key() -> Vec<u8> {
    system_key(SystemEntry::ControlRestoreProgress, &[])
}

/// Persistence policy for a write transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// Authoritative writes. Journal is flushed and synced before commit returns.
    Authoritative,
    /// Telemetry writes. Buffered; a periodic flush or a later authoritative
    /// write carries them to disk.
    Buffered,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_codec::prefix_end;

    const DIRECT_VERSION_KEY_FIXTURE: &str =
        include_str!("../fixtures/rrflow-kv-direct-version-keys-v1.hex");

    fn sp(subject: &str, predicate: &str) -> (Subject, Predicate) {
        (
            Subject::new(subject).unwrap(),
            Predicate::new(predicate).unwrap(),
        )
    }

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(DIGITS[usize::from(byte >> 4)] as char);
            encoded.push(DIGITS[usize::from(byte & 0x0f)] as char);
        }
        encoded
    }

    #[test]
    fn spaces_have_frozen_unique_family_subspace_coordinates() {
        let spaces = [
            CLAIMS,
            SEQUENCE_INDEX,
            ACCESS,
            META,
            INVOCATIONS,
            FUNCTION_ARTIFACTS,
            FUNCTION_DEFINITIONS,
            TRANSACTION_FUNCTION_BINDINGS,
            FUNCTION_CATALOGUE_MEMBERSHIPS,
            FUNCTION_CATALOGUE_HEADS,
            PROJECTIONS,
            RUNTIME_CHANGES,
            RUNTIME_CLAIM_VERSIONS,
            RUNTIME_RECORDS,
            RUNTIME_RECORD_VERSIONS,
            RUNTIME_RELATIONS,
            RUNTIME_RELATION_VERSIONS,
            RUNTIME_EVENT_VERSIONS,
            RUNTIME_OUTGOING_EDGES,
            RUNTIME_OUTGOING_EDGE_VERSIONS,
            RUNTIME_INCOMING_EDGES,
            RUNTIME_INCOMING_EDGE_VERSIONS,
            RUNTIME_VECTORS,
            RUNTIME_VECTOR_VERSIONS,
            RUNTIME_SERIES,
            RUNTIME_SERIES_VERSIONS,
            RUNTIME_GEO,
            RUNTIME_GEO_VERSIONS,
            RUNTIME_OBJECTS,
            RUNTIME_OBJECT_VERSIONS,
            RUNTIME_PROJECTION_DELTAS,
            RUNTIME_INDEX_SOURCE_DELTAS,
            RUNTIME_VECTOR_SOURCE_DELTAS,
            RUNTIME_INDEX_BINDINGS,
            RUNTIME_INDEX_BINDING_VERSIONS,
            RUNTIME_INDEX_BINDING_SETS,
            RUNTIME_SCALAR_ENTRIES,
            RUNTIME_UNIQUE_ENTRIES,
            RUNTIME_OUTBOX,
            RUNTIME_AUDIT,
            RUNTIME_COMMITS,
            RUNTIME_SCHEMAS,
            RUNTIME_SCHEMA_VERSIONS,
            RUNTIME_SNAPSHOTS,
        ];
        let mut coordinates = spaces
            .map(|space| (space.family() as u8, space.subspace()))
            .to_vec();
        coordinates.sort_unstable();
        coordinates.dedup();
        assert_eq!(coordinates.len(), spaces.len());
    }

    #[test]
    fn claim_keys_are_bitemporal_newest_first_and_isolated() {
        let codec = KeyCodec;
        let (subject, predicate) = sp("wp3", "status");
        let original = claim_key(&subject, &predicate, 100, 100);
        let correction = claim_key(&subject, &predicate, 100, 200);
        let newer = claim_key(&subject, &predicate, 300, 300);
        assert!(newer < correction && correction < original);
        assert_eq!(
            parse_claim_key(codec, &correction).unwrap(),
            (subject.clone(), predicate.clone(), 100, 200)
        );

        let prefix = claim_version_prefix(&subject, &predicate);
        let end = prefix_end(&prefix).unwrap();
        assert!(correction >= prefix && correction < end);
        for (other_subject, other_predicate) in
            [("wp3x", "status"), ("wp3", "statusx"), ("wp", "status")]
        {
            let (other_subject, other_predicate) = sp(other_subject, other_predicate);
            let neighbour = claim_key(&other_subject, &other_predicate, 100, 100);
            assert!(neighbour < prefix || neighbour >= end);
        }
    }

    #[test]
    fn seek_ordering_matches_as_of_semantics() {
        let (subject, predicate) = sp("wp3", "status");
        let v1 = claim_key(&subject, &predicate, 100, 100);
        let v2 = claim_key(&subject, &predicate, 200, 200);
        let seek = claim_seek_key(&subject, &predicate, 150);
        assert!(v2 < seek);
        assert!(seek <= v1);
    }

    #[test]
    fn access_and_sequence_keys_round_trip_in_order() {
        let codec = KeyCodec;
        let reader = Reader::new("agent:clyffy/worker").unwrap();
        let (subject, predicate) = sp("a/b", "c");
        let access = access_key(500, &reader, &subject, &predicate);
        assert_eq!(
            parse_access_key(codec, &access).unwrap(),
            (500, reader, subject, predicate)
        );
        assert!(
            access_key(
                9,
                &Reader::new("r").unwrap(),
                &Subject::new("s").unwrap(),
                &Predicate::new("p").unwrap()
            ) < access_bound(10)
        );
        assert!(sequence_key(9) < sequence_key(10));
        assert_eq!(parse_sequence_key(codec, &sequence_key(42)).unwrap(), 42);
    }

    #[test]
    fn malformed_semantic_shapes_are_rejected() {
        let codec = KeyCodec;
        assert!(parse_claim_key(codec, &space_prefix(CLAIMS)).is_err());
        assert!(parse_access_key(codec, &sequence_key(1)).is_err());
        let malformed = KeyCodec.encode(
            KeyAddress::GLOBAL,
            KeyFamily::Temporal,
            &[KeyPart::U8(Space::Claims.subspace()), KeyPart::U64(1)],
        );
        assert!(parse_claim_key(codec, &malformed).is_err());
    }

    #[test]
    fn temporal_and_adjacency_shapes_preserve_direction_and_newest_first_order() {
        let scope = ScopeId::new("project:graph-order").unwrap();
        let relation = RuntimeRelation {
            reference: RuntimeRef::new("imports", "a-b").unwrap(),
            from: RuntimeRef::new("file", "src/a.rs").unwrap(),
            to: RuntimeRef::new("file", "src/b.rs").unwrap(),
            valid_from: 100,
            valid_to: None,
            properties: Default::default(),
        };
        let outgoing = runtime_adjacency_key(RUNTIME_OUTGOING_EDGES, &scope, &relation);
        let incoming = runtime_adjacency_key(RUNTIME_INCOMING_EDGES, &scope, &relation);
        assert_ne!(outgoing, incoming);
        let outgoing_history = space_prefix(RUNTIME_OUTGOING_EDGE_VERSIONS);
        let incoming_history = space_prefix(RUNTIME_INCOMING_EDGE_VERSIONS);
        assert_ne!(space_prefix(RUNTIME_OUTGOING_EDGES), outgoing_history);
        assert_ne!(space_prefix(RUNTIME_INCOMING_EDGES), incoming_history);
        assert!(
            runtime_adjacency_version_key(
                RUNTIME_OUTGOING_EDGE_VERSIONS,
                &scope,
                &relation,
                200,
                9,
            ) < runtime_adjacency_version_key(
                RUNTIME_OUTGOING_EDGE_VERSIONS,
                &scope,
                &relation,
                100,
                8,
            )
        );
        assert!(
            runtime_version_key(
                RUNTIME_RELATION_VERSIONS,
                &scope,
                &relation.reference,
                200,
                9,
            ) < runtime_version_key(
                RUNTIME_RELATION_VERSIONS,
                &scope,
                &relation.reference,
                100,
                8,
            )
        );
    }

    #[test]
    fn adjacency_prefixes_isolate_node_and_relation_type_and_keys_round_trip() {
        let codec = KeyCodec;
        let scope = ScopeId::new("project:adjacency-prefix").unwrap();
        let relation = RuntimeRelation {
            reference: RuntimeRef::new("imports", "a-b").unwrap(),
            from: RuntimeRef::new("file", "a").unwrap(),
            to: RuntimeRef::new("module", "b").unwrap(),
            valid_from: 100,
            valid_to: None,
            properties: Default::default(),
        };
        let imports = rrd_core::RuntimeType::new("imports").unwrap();
        for (space, near, far) in [
            (RUNTIME_OUTGOING_EDGE_VERSIONS, &relation.from, &relation.to),
            (RUNTIME_INCOMING_EDGE_VERSIONS, &relation.to, &relation.from),
        ] {
            let key = runtime_adjacency_version_key(space, &scope, &relation, 100, 7);
            let prefix = runtime_adjacency_prefix(space, &scope, near, &imports);
            let end = prefix_end(&prefix).unwrap();
            assert!(key >= prefix && key < end);
            for (node_id, kind) in [
                (format!("{}x", near.id), "imports"),
                (format!("{}-b", near.id), "imports"),
                (near.id.to_string(), "importsx"),
                (near.id.to_string(), "import"),
            ] {
                let mut neighbour = relation.clone();
                neighbour.reference = RuntimeRef::new(kind, "a-b").unwrap();
                let endpoint = RuntimeRef::new(near.kind.as_str(), node_id.as_str()).unwrap();
                if space == RUNTIME_OUTGOING_EDGE_VERSIONS {
                    neighbour.from = endpoint;
                } else {
                    neighbour.to = endpoint;
                }
                let neighbour = runtime_adjacency_version_key(space, &scope, &neighbour, 100, 7);
                assert!(neighbour < prefix || neighbour >= end, "{node_id} {kind}");
            }
            assert_eq!(
                parse_runtime_adjacency_version_key(codec, space, &key).unwrap(),
                RuntimeAdjacencyVersionKey {
                    scope: scope.clone(),
                    node: near.clone(),
                    relation: relation.reference.clone(),
                    other: far.clone(),
                    effective_at: 100,
                    cursor: 7,
                }
            );
            assert!(parse_runtime_adjacency_version_key(codec, space, &prefix).is_err());
        }
        assert!(parse_runtime_adjacency_version_key(
            codec,
            RUNTIME_INCOMING_EDGE_VERSIONS,
            &runtime_adjacency_version_key(
                RUNTIME_OUTGOING_EDGE_VERSIONS,
                &scope,
                &relation,
                100,
                7
            ),
        )
        .is_err());
    }

    #[test]
    fn every_direct_version_family_is_scope_bound_and_newest_first() {
        let scope = ScopeId::new("project:direct-version-keys").unwrap();
        let other = ScopeId::new("project:direct-version-keys-other").unwrap();
        let reference = RuntimeRef::new("document", "alpha").unwrap();
        for space in [
            RUNTIME_RECORD_VERSIONS,
            RUNTIME_RELATION_VERSIONS,
            RUNTIME_EVENT_VERSIONS,
            RUNTIME_SERIES_VERSIONS,
            RUNTIME_GEO_VERSIONS,
            RUNTIME_OBJECT_VERSIONS,
        ] {
            let newer = runtime_version_key(space, &scope, &reference, 200, 9);
            let older = runtime_version_key(space, &scope, &reference, 100, 8);
            assert!(newer < older);
            assert!(newer.starts_with(&runtime_kind_prefix(space, &scope, &reference.kind)));
            assert!(!newer.starts_with(&runtime_scope_prefix(space, &other)));
        }

        let claim = rrd_core::Claim::new(
            Subject::new("document:alpha").unwrap(),
            Predicate::new("status").unwrap(),
            "ready",
            100,
            200,
            rrd_core::Producer {
                actor: "test:direct-version-keys".into(),
                on_behalf_of: None,
                session: None,
            },
        );
        let claim_key = runtime_claim_version_key(&scope, &claim, 9);
        assert!(claim_key.starts_with(&runtime_claim_predicate_prefix(&scope, &claim.predicate)));
        assert!(!claim_key.starts_with(&runtime_scope_prefix(RUNTIME_CLAIM_VERSIONS, &other)));
        assert!(runtime_schema_version_key(&scope, 9) < runtime_schema_version_key(&scope, 8));
        assert!(runtime_schema_version_key(&scope, 9)
            .starts_with(&runtime_scope_prefix(RUNTIME_SCHEMA_VERSIONS, &scope)));
    }

    #[test]
    fn direct_version_key_shapes_match_the_frozen_fixture() {
        let scope = ScopeId::new("project:version-fixture").unwrap();
        let reference = RuntimeRef::new("document", "alpha").unwrap();
        let claim = rrd_core::Claim::new(
            Subject::new("document:alpha").unwrap(),
            Predicate::new("status").unwrap(),
            "ready",
            100,
            200,
            rrd_core::Producer {
                actor: "test:version-fixture".into(),
                on_behalf_of: None,
                session: None,
            },
        );
        let entries = [
            ("schema", runtime_schema_version_key(&scope, 1)),
            ("claim", runtime_claim_version_key(&scope, &claim, 2)),
            (
                "record",
                runtime_version_key(RUNTIME_RECORD_VERSIONS, &scope, &reference, 100, 3),
            ),
            (
                "relation",
                runtime_version_key(RUNTIME_RELATION_VERSIONS, &scope, &reference, 100, 4),
            ),
            (
                "event",
                runtime_version_key(RUNTIME_EVENT_VERSIONS, &scope, &reference, 100, 5),
            ),
            (
                "vector",
                runtime_vector_version_key(&scope, &reference, 100, 6),
            ),
            (
                "series",
                runtime_version_key(RUNTIME_SERIES_VERSIONS, &scope, &reference, 100, 7),
            ),
            (
                "geo",
                runtime_version_key(RUNTIME_GEO_VERSIONS, &scope, &reference, 100, 8),
            ),
            (
                "object",
                runtime_version_key(RUNTIME_OBJECT_VERSIONS, &scope, &reference, 100, 9),
            ),
        ];
        let actual = entries
            .into_iter()
            .map(|(label, key)| format!("{label} {}", hex(&key)))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(actual, DIRECT_VERSION_KEY_FIXTURE.trim_end());
    }

    #[test]
    fn native_index_and_vector_delta_keys_are_typed_ordered_and_source_isolated() {
        let scope = ScopeId::new("project:index-key-order").unwrap();
        let index = ProjectionId::new("by-score").unwrap();
        let reference = RuntimeRef::new("document", "alpha").unwrap();
        let negative = vec![RuntimeValue::Integer(-7)];
        let positive = vec![RuntimeValue::Integer(9)];
        let negative_key = runtime_index_entry_key(
            RUNTIME_SCALAR_ENTRIES,
            &scope,
            &index,
            &negative,
            &reference,
            10,
        )
        .unwrap();
        let positive_key = runtime_index_entry_key(
            RUNTIME_SCALAR_ENTRIES,
            &scope,
            &index,
            &positive,
            &reference,
            10,
        )
        .unwrap();
        assert!(negative_key < positive_key);

        let unique_prefix =
            runtime_index_value_prefix(RUNTIME_UNIQUE_ENTRIES, &scope, &index, &positive).unwrap();
        let unique_key = runtime_index_entry_key(
            RUNTIME_UNIQUE_ENTRIES,
            &scope,
            &index,
            &positive,
            &reference,
            10,
        )
        .unwrap();
        assert!(unique_key.starts_with(&unique_prefix));
        assert!(
            runtime_index_source_delta_key(&scope, &index, 9, 0)
                < runtime_index_source_delta_key(&scope, &index, 10, 0)
        );
        assert!(
            runtime_vector_version_key(&scope, &reference, 20, 9)
                < runtime_vector_version_key(&scope, &reference, 10, 8)
        );
        let documents =
            runtime_vector_source_delta_prefix(&scope, "documents", "semantic", "title").unwrap();
        assert_ne!(
            documents,
            runtime_vector_source_delta_prefix(&scope, "archive", "semantic", "title").unwrap()
        );
        assert_ne!(
            documents,
            runtime_vector_source_delta_prefix(&scope, "documents", "summary", "title").unwrap()
        );
        assert!(runtime_vector_source_delta_prefix(&scope, "", "semantic", "title").is_err());
        assert!(runtime_vector_source_delta_prefix(&scope, "documents", "", "title").is_err());
    }
}
