//! Version and adjacency entry validation, version addresses and change authentication.

use super::*;

pub(super) fn validate_version_entry(
    source: &RuntimeVersionedSource,
    read: &ReadStamp,
    stored_key: &[u8],
    change: &RuntimeChange,
) -> Result<()> {
    if change.scope != read.scope || change.cursor == 0 || change.cursor > read.commit_cursor {
        return Err(Error::ReadStampMismatch(read.manifest_id.clone()));
    }
    if !change.verify_digest() {
        return Err(Error::Substrate(format!(
            "runtime semantic version {} failed digest verification",
            change.cursor
        )));
    }
    if let RuntimeVersionedSource::Adjacency {
        relation,
        node,
        direction,
    } = source
    {
        return validate_adjacency_entry(relation, node, *direction, stored_key, change);
    }
    let expected = expected_version_key(source, change)?;
    if stored_key != expected {
        return Err(Error::Codec(format!(
            "runtime semantic version {} does not match its typed key",
            change.cursor
        )));
    }
    Ok(())
}

/// A relation version is re-keyed from its own endpoints. A retirement does
/// not carry endpoints, so its strictly decoded key must name the selected
/// node and relation type and the retirement's identity, time, and cursor.
pub(super) fn validate_adjacency_entry(
    relation_kind: &RuntimeType,
    node: &RuntimeRef,
    direction: RuntimeAdjacencyDirection,
    stored_key: &[u8],
    change: &RuntimeChange,
) -> Result<()> {
    let space = adjacency_space(direction);
    let matches = match &change.mutation {
        RuntimeMutation::Relation { relation } => {
            let near = match direction {
                RuntimeAdjacencyDirection::Outgoing => &relation.from,
                RuntimeAdjacencyDirection::Incoming => &relation.to,
            };
            &relation.reference.kind == relation_kind
                && near == node
                && stored_key
                    == keyspaces::runtime_adjacency_version_key(
                        space,
                        &change.scope,
                        relation,
                        relation.valid_from,
                        change.cursor,
                    )
        }
        RuntimeMutation::Retire { retirement }
            if retirement.model == RuntimeLogicalModel::GraphRelation =>
        {
            let key = keyspaces::parse_runtime_adjacency_version_key(KeyCodec, space, stored_key)?;
            key.scope == change.scope
                && &key.node == node
                && &retirement.reference.kind == relation_kind
                && key.relation == retirement.reference
                && key.effective_at == retirement.effective_at
                && key.cursor == change.cursor
        }
        _ => false,
    };
    if !matches {
        return Err(Error::Codec(format!(
            "runtime adjacency version {} does not match its typed key",
            change.cursor
        )));
    }
    Ok(())
}

pub(super) fn expected_version_key(
    source: &RuntimeVersionedSource,
    change: &RuntimeChange,
) -> Result<Vec<u8>> {
    let scope = &change.scope;
    let cursor = change.cursor;
    match (source, &change.mutation) {
        (RuntimeVersionedSource::Schema, RuntimeMutation::Schema { .. }) => {
            return Ok(keyspaces::runtime_schema_version_key(scope, cursor));
        }
        (RuntimeVersionedSource::Claims { predicate }, RuntimeMutation::Claim { claim })
            if predicate
                .as_ref()
                .is_none_or(|value| value == &claim.predicate) =>
        {
            return Ok(keyspaces::runtime_claim_version_key(scope, claim, cursor));
        }
        _ => {}
    }

    let address = version_address(change)?;
    if !source_accepts_address(source, &address)? {
        let path = source.path()?;
        return Err(Error::Codec(format!(
            "runtime semantic version {} escaped its selected {:?} family",
            cursor, path
        )));
    }
    if address.space == keyspaces::RUNTIME_VECTOR_VERSIONS {
        Ok(keyspaces::runtime_vector_version_key(
            scope,
            &address.reference,
            address.effective_at,
            cursor,
        ))
    } else {
        Ok(keyspaces::runtime_version_key(
            address.space,
            scope,
            &address.reference,
            address.effective_at,
            cursor,
        ))
    }
}

pub(super) struct VersionAddress {
    pub(super) space: Space,
    pub(super) reference: RuntimeRef,
    pub(super) effective_at: u64,
}

pub(super) fn version_address(change: &RuntimeChange) -> Result<VersionAddress> {
    let (space, reference, effective_at) = match &change.mutation {
        RuntimeMutation::Record { record } => (
            keyspaces::RUNTIME_RECORD_VERSIONS,
            record.reference.clone(),
            record.valid_from,
        ),
        RuntimeMutation::Relation { relation } => (
            keyspaces::RUNTIME_RELATION_VERSIONS,
            relation.reference.clone(),
            relation.valid_from,
        ),
        RuntimeMutation::Event { event } => (
            keyspaces::RUNTIME_EVENT_VERSIONS,
            keyspaces::runtime_event_reference(event, change.cursor),
            change.at,
        ),
        RuntimeMutation::Vector { vector } => (
            keyspaces::RUNTIME_VECTOR_VERSIONS,
            vector.reference.clone(),
            vector.valid_from,
        ),
        RuntimeMutation::SeriesSample { sample } => (
            keyspaces::RUNTIME_SERIES_VERSIONS,
            sample.reference.clone(),
            sample.observed_at,
        ),
        RuntimeMutation::Geo { geo } => (
            keyspaces::RUNTIME_GEO_VERSIONS,
            geo.reference.clone(),
            geo.valid_from,
        ),
        RuntimeMutation::Object { object } => (
            keyspaces::RUNTIME_OBJECT_VERSIONS,
            object.reference.clone(),
            change.at,
        ),
        RuntimeMutation::Retire { retirement } => (
            version_space_for_model(retirement.model)?,
            retirement.reference.clone(),
            retirement.effective_at,
        ),
        RuntimeMutation::Schema { .. } | RuntimeMutation::Claim { .. } => {
            return Err(Error::Codec(format!(
                "runtime semantic version {} escaped its selected family",
                change.cursor
            )));
        }
    };
    Ok(VersionAddress {
        space,
        reference,
        effective_at,
    })
}

pub(super) fn source_accepts_address(
    source: &RuntimeVersionedSource,
    address: &VersionAddress,
) -> Result<bool> {
    let (space, kind) = match source {
        RuntimeVersionedSource::Records { kind } => {
            (keyspaces::RUNTIME_RECORD_VERSIONS, kind.as_ref())
        }
        RuntimeVersionedSource::Relations { kind } => {
            (keyspaces::RUNTIME_RELATION_VERSIONS, kind.as_ref())
        }
        RuntimeVersionedSource::Events { kind } => {
            (keyspaces::RUNTIME_EVENT_VERSIONS, kind.as_ref())
        }
        RuntimeVersionedSource::Vectors { kind } => {
            (keyspaces::RUNTIME_VECTOR_VERSIONS, kind.as_ref())
        }
        RuntimeVersionedSource::Series { kind } => {
            (keyspaces::RUNTIME_SERIES_VERSIONS, kind.as_ref())
        }
        RuntimeVersionedSource::Geo { kind } => (keyspaces::RUNTIME_GEO_VERSIONS, kind.as_ref()),
        RuntimeVersionedSource::Objects { kind } => {
            (keyspaces::RUNTIME_OBJECT_VERSIONS, kind.as_ref())
        }
        // One physical identity has one history. A retirement recorded under
        // an earlier schema's logical model for the same family remains part
        // of it, exactly as a family range would select it.
        RuntimeVersionedSource::Identity { model, reference } => {
            return Ok(version_space_for_model(*model)? == address.space
                && reference == &address.reference);
        }
        RuntimeVersionedSource::IdentityRange {
            model,
            kind,
            lower,
            upper,
        } => {
            let id = &address.reference.id;
            let above = match lower {
                Bound::Unbounded => true,
                Bound::Included(lower) => id >= lower,
                Bound::Excluded(lower) => id > lower,
            };
            let below = match upper {
                Bound::Unbounded => true,
                Bound::Included(upper) => id <= upper,
                Bound::Excluded(upper) => id < upper,
            };
            return Ok(version_space_for_model(*model)? == address.space
                && kind == &address.reference.kind
                && above
                && below);
        }
        RuntimeVersionedSource::Schema
        | RuntimeVersionedSource::Claims { .. }
        | RuntimeVersionedSource::Adjacency { .. } => return Ok(false),
    };
    Ok(space == address.space && kind.is_none_or(|expected| expected == &address.reference.kind))
}

pub(super) fn access_path_for_model(model: RuntimeLogicalModel) -> Result<RuntimeReadAccessPath> {
    Ok(match model {
        RuntimeLogicalModel::Document
        | RuntimeLogicalModel::Relational
        | RuntimeLogicalModel::GraphNode
        | RuntimeLogicalModel::KeyValue
        | RuntimeLogicalModel::ReasoningRecord
        | RuntimeLogicalModel::LifecycleRecord => RuntimeReadAccessPath::RecordVersions,
        RuntimeLogicalModel::GraphRelation => RuntimeReadAccessPath::RelationVersions,
        RuntimeLogicalModel::Event
        | RuntimeLogicalModel::ReasoningEvent
        | RuntimeLogicalModel::LifecycleEvent => RuntimeReadAccessPath::EventVersions,
        RuntimeLogicalModel::Vector => RuntimeReadAccessPath::VectorVersions,
        RuntimeLogicalModel::TimeSeries => RuntimeReadAccessPath::SeriesVersions,
        RuntimeLogicalModel::Geo => RuntimeReadAccessPath::GeoVersions,
        RuntimeLogicalModel::Object => RuntimeReadAccessPath::ObjectVersions,
        RuntimeLogicalModel::ReasoningClaim => {
            return Err(Error::Substrate(
                "claim identities require subject/predicate selection".into(),
            ));
        }
    })
}

pub(super) fn version_space_for_model(model: RuntimeLogicalModel) -> Result<Space> {
    Ok(match access_path_for_model(model)? {
        RuntimeReadAccessPath::RecordVersions => keyspaces::RUNTIME_RECORD_VERSIONS,
        RuntimeReadAccessPath::RelationVersions => keyspaces::RUNTIME_RELATION_VERSIONS,
        RuntimeReadAccessPath::EventVersions => keyspaces::RUNTIME_EVENT_VERSIONS,
        RuntimeReadAccessPath::VectorVersions => keyspaces::RUNTIME_VECTOR_VERSIONS,
        RuntimeReadAccessPath::SeriesVersions => keyspaces::RUNTIME_SERIES_VERSIONS,
        RuntimeReadAccessPath::GeoVersions => keyspaces::RUNTIME_GEO_VERSIONS,
        RuntimeReadAccessPath::ObjectVersions => keyspaces::RUNTIME_OBJECT_VERSIONS,
        RuntimeReadAccessPath::ReadStamp
        | RuntimeReadAccessPath::SchemaVersions
        | RuntimeReadAccessPath::ClaimVersions
        | RuntimeReadAccessPath::AdjacencyVersions
        | RuntimeReadAccessPath::VectorSourceDeltas
        | RuntimeReadAccessPath::AccumulatorProof => {
            return Err(Error::Substrate(
                "logical model has no identity-version space".into(),
            ));
        }
    })
}

pub(super) fn open_accumulator(
    reader: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    nodes: &mut ProofNodes,
) -> Result<RuntimeLogAccumulator> {
    let root = read
        .accumulator_root
        .as_deref()
        .ok_or_else(|| Error::ReadStampMismatch(read.manifest_id.clone()))?;
    Ok(RuntimeLogAccumulator::from_nodes(
        read.commit_cursor,
        root,
        |level, index| nodes.read(level, index, || read_accumulator_node(reader, level, index)),
    )?)
}

pub(super) fn authenticate_changes<'a>(
    reader: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    accumulator: &RuntimeLogAccumulator,
    nodes: &mut ProofNodes,
    changes: impl Iterator<Item = &'a RuntimeChange>,
) -> Result<()> {
    let root = read
        .accumulator_root
        .as_deref()
        .ok_or_else(|| Error::ReadStampMismatch(read.manifest_id.clone()))?;
    for change in changes {
        let proof = accumulator.inclusion_proof(change.cursor - 1, |level, index| {
            nodes.read(level, index, || read_accumulator_node(reader, level, index))
        })?;
        proof.verify_change(change, root)?;
    }
    Ok(())
}
