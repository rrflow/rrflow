//! Source ranges, batched scans and version ranges.

use super::*;

/// Every nonempty half-open physical interval admitted into one read. With
/// no prior overlap, only its sorted neighbours can overlap a new interval.
#[derive(Default)]
pub(super) struct SourceRanges {
    admitted: BTreeMap<Vec<u8>, (Vec<u8>, RuntimeVersionedSource)>,
}

impl SourceRanges {
    pub(super) fn admit(
        &mut self,
        sources: &[RuntimeVersionedSource],
        scope: &rrd_core::ScopeId,
    ) -> Result<Vec<RuntimeVersionedSource>> {
        let mut step = Vec::new();
        for source in sources {
            let (_, start, end) = source_interval(source, scope)?;
            if start == end {
                continue;
            }
            if let Some((_, admitted)) = self.admitted.get(&start) {
                if admitted == source {
                    continue;
                }
                return Err(overlapping_ranges(source));
            }
            let previous = self
                .admitted
                .range::<[u8], _>((Bound::Unbounded, Bound::Excluded(start.as_slice())))
                .next_back()
                .is_some_and(|(_, (admitted_end, _))| admitted_end > &start);
            let next = self
                .admitted
                .range::<[u8], _>((Bound::Excluded(start.as_slice()), Bound::Unbounded))
                .next()
                .is_some_and(|(admitted_start, _)| admitted_start < &end);
            if previous || next {
                return Err(overlapping_ranges(source));
            }
            self.admitted.insert(start, (end, source.clone()));
            step.push(source.clone());
        }
        Ok(step)
    }
}

pub(super) fn overlapping_ranges(source: &RuntimeVersionedSource) -> Error {
    match source.path() {
        Ok(path) => Error::Substrate(format!(
            "runtime versioned read contains overlapping {path:?} ranges"
        )),
        Err(error) => error,
    }
}

pub(super) fn scan_source(
    reader: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    source: &RuntimeVersionedSource,
    visit: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> Result<()>,
) -> Result<()> {
    let (_, start, end) = source_interval(source, &read.scope)?;
    scan_in_batches(reader, start, &end, visit)
}

/// Rows one versioned scan asks storage for at a time. `projected_output_bytes`
/// bounds what a scan holds, so this is what lets a read outgrow that limit.
/// A smaller batch costs more seeks, and a larger one refuses rows it could read.
pub(super) const VERSIONED_SCAN_BATCH_ROWS: usize = 1024;

/// Visits `[start, end)` in key order, holding one batch of stored rows at a
/// time. Every batch reads the same snapshot, because the reader does.
pub(super) fn scan_in_batches(
    reader: &(impl AccessRead + ?Sized),
    mut start: Vec<u8>,
    end: &[u8],
    visit: &mut dyn FnMut(Vec<u8>, Vec<u8>) -> Result<()>,
) -> Result<()> {
    loop {
        let rows = reader.scan_range(&start, end, VERSIONED_SCAN_BATCH_ROWS)?;
        let complete = rows.len() < VERSIONED_SCAN_BATCH_ROWS;
        let Some((last, _)) = rows.last() else {
            return Ok(());
        };
        // The smallest key after the last one, which is the next batch's start.
        let mut next = Vec::with_capacity(last.len() + 1);
        next.extend_from_slice(last);
        next.push(0);
        for (key, value) in rows {
            visit(key, value)?;
        }
        if complete || next.as_slice() >= end {
            return Ok(());
        }
        start = next;
    }
}

pub(super) fn source_interval(
    source: &RuntimeVersionedSource,
    scope: &rrd_core::ScopeId,
) -> Result<(Space, Vec<u8>, Vec<u8>)> {
    let end_of = |prefix: &[u8]| {
        prefix_end(prefix)
            .ok_or_else(|| Error::Substrate("version range has no upper bound".into()))
    };
    if let RuntimeVersionedSource::IdentityRange {
        model,
        kind,
        lower,
        upper,
    } = source
    {
        if matches!((lower, upper), (Bound::Unbounded, Bound::Unbounded))
            || matches!((lower, upper),
                (Bound::Included(a) | Bound::Excluded(a), Bound::Included(b) | Bound::Excluded(b)) if a > b)
        {
            return Err(Error::Substrate("invalid runtime identity interval".into()));
        }
        let space = version_space_for_model(*model)?;
        let family = keyspaces::runtime_kind_prefix(space, scope, kind);
        let identity = |id: &RuntimeId| {
            keyspaces::runtime_reference_prefix(
                space,
                scope,
                &RuntimeRef {
                    kind: kind.clone(),
                    id: id.clone(),
                },
            )
        };
        let start = match lower {
            Bound::Unbounded => family.clone(),
            Bound::Included(id) => identity(id),
            Bound::Excluded(id) => end_of(&identity(id))?,
        };
        let end = match upper {
            Bound::Unbounded => end_of(&family)?,
            Bound::Excluded(id) => identity(id),
            Bound::Included(id) => end_of(&identity(id))?,
        };
        // Equal semantic endpoints with either strict side select no identity.
        let start = start.min(end.clone());
        return Ok((space, start, end));
    }
    let (space, prefix) = source_range(source, scope)?;
    let end = end_of(&prefix)?;
    Ok((space, prefix, end))
}

pub(super) fn source_range(
    source: &RuntimeVersionedSource,
    scope: &rrd_core::ScopeId,
) -> Result<(Space, Vec<u8>)> {
    Ok(match source {
        RuntimeVersionedSource::Schema => (
            keyspaces::RUNTIME_SCHEMA_VERSIONS,
            keyspaces::runtime_scope_prefix(keyspaces::RUNTIME_SCHEMA_VERSIONS, scope),
        ),
        RuntimeVersionedSource::Claims { predicate } => (
            keyspaces::RUNTIME_CLAIM_VERSIONS,
            predicate.as_ref().map_or_else(
                || keyspaces::runtime_scope_prefix(keyspaces::RUNTIME_CLAIM_VERSIONS, scope),
                |predicate| keyspaces::runtime_claim_predicate_prefix(scope, predicate),
            ),
        ),
        RuntimeVersionedSource::Records { kind } => {
            version_range(keyspaces::RUNTIME_RECORD_VERSIONS, scope, kind.as_ref())
        }
        RuntimeVersionedSource::Relations { kind } => {
            version_range(keyspaces::RUNTIME_RELATION_VERSIONS, scope, kind.as_ref())
        }
        RuntimeVersionedSource::Events { kind } => {
            version_range(keyspaces::RUNTIME_EVENT_VERSIONS, scope, kind.as_ref())
        }
        RuntimeVersionedSource::Vectors { kind } => {
            version_range(keyspaces::RUNTIME_VECTOR_VERSIONS, scope, kind.as_ref())
        }
        RuntimeVersionedSource::Series { kind } => {
            version_range(keyspaces::RUNTIME_SERIES_VERSIONS, scope, kind.as_ref())
        }
        RuntimeVersionedSource::Geo { kind } => {
            version_range(keyspaces::RUNTIME_GEO_VERSIONS, scope, kind.as_ref())
        }
        RuntimeVersionedSource::Objects { kind } => {
            version_range(keyspaces::RUNTIME_OBJECT_VERSIONS, scope, kind.as_ref())
        }
        RuntimeVersionedSource::Identity { model, reference } => {
            let space = version_space_for_model(*model)?;
            (
                space,
                keyspaces::runtime_reference_prefix(space, scope, reference),
            )
        }
        RuntimeVersionedSource::IdentityRange { .. } => {
            return Err(Error::Substrate(
                "identity interval is not a prefix source".into(),
            ));
        }
        RuntimeVersionedSource::Adjacency {
            relation,
            node,
            direction,
        } => {
            let space = adjacency_space(*direction);
            (
                space,
                keyspaces::runtime_adjacency_prefix(space, scope, node, relation),
            )
        }
    })
}

pub(super) const fn adjacency_space(direction: RuntimeAdjacencyDirection) -> Space {
    match direction {
        RuntimeAdjacencyDirection::Outgoing => keyspaces::RUNTIME_OUTGOING_EDGE_VERSIONS,
        RuntimeAdjacencyDirection::Incoming => keyspaces::RUNTIME_INCOMING_EDGE_VERSIONS,
    }
}

pub(super) fn version_range(
    space: Space,
    scope: &rrd_core::ScopeId,
    kind: Option<&RuntimeType>,
) -> (Space, Vec<u8>) {
    let prefix = kind.map_or_else(
        || keyspaces::runtime_scope_prefix(space, scope),
        |kind| keyspaces::runtime_kind_prefix(space, scope, kind),
    );
    (space, prefix)
}
