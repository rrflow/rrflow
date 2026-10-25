//! Read-stamp-bound access to canonical semantic version entries.
//!
//! This is the normal state-read substrate shared by rrflowMX and rrflowKV.
//! It selects scope-bound typed version ranges, validates every returned key
//! against its value, authenticates each value against the stamped RFC 9162
//! root, and reports the complete bounded access shape. The authoritative
//! change log is not a range source here; replay, changefeed, archive, and
//! recovery keep their explicit paged-log APIs.

use super::proof_nodes::ProofNodes;
use super::runtime_state::{read_accumulator_node, validate_read_stamp, AccessRead};
use crate::key_codec::{prefix_end, KeyCodec};
use crate::keyspaces::{self, Space};
use crate::{Error, Result};
use rrd_core::{
    Predicate, ReadStamp, RuntimeChange, RuntimeId, RuntimeLogAccumulator, RuntimeLogicalModel,
    RuntimeMutation, RuntimeReadValidation, RuntimeRef, RuntimeType,
};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ops::Bound;

pub const RUNTIME_VERSIONED_READ_CONTRACT_VERSION: u16 = 1;

/// Hard key-examination bound for one direct semantic read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeReadBudget {
    pub max_keys: u64,
}

impl RuntimeReadBudget {
    pub fn new(max_keys: u64) -> Result<Self> {
        let budget = Self { max_keys };
        budget.validate()?;
        Ok(budget)
    }

    pub fn validate(self) -> Result<()> {
        if self.max_keys == 0 {
            return Err(Error::Substrate(
                "runtime versioned-read key budget must be greater than zero".into(),
            ));
        }
        Ok(())
    }

    /// Mathematical upper bound for authenticating all non-overlapping
    /// semantic versions visible at one current commit head. This is used by
    /// write-path integrity checks that cannot accept an operator workload
    /// policy. Request/query paths must continue to supply their own budget.
    pub fn authenticated_history_integrity(commit_cursor: u64) -> Result<Self> {
        // Cursor, accumulator state, head digest, scope schema, and scope
        // catalogue revision are the current-stamp point reads.
        const CURRENT_STAMP_POINT_READS: u64 = 5;
        let proof_height = u64::from(u64::BITS);
        let version_and_proof = commit_cursor
            .checked_mul(proof_height.saturating_add(1))
            .ok_or(Error::SequenceOverflow)?;
        let max_keys = version_and_proof
            .checked_add(proof_height)
            .and_then(|value| value.checked_add(CURRENT_STAMP_POINT_READS))
            .ok_or(Error::SequenceOverflow)?;
        Self::new(max_keys)
    }
}

/// One typed semantic version range. An absent discriminator selects the
/// complete family within the stamped scope; a present discriminator narrows
/// the physical prefix before any value is decoded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeVersionedSource {
    Schema,
    Claims {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        predicate: Option<Predicate>,
    },
    Records {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<RuntimeType>,
    },
    Relations {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<RuntimeType>,
    },
    Events {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<RuntimeType>,
    },
    Vectors {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<RuntimeType>,
    },
    Series {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<RuntimeType>,
    },
    Geo {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<RuntimeType>,
    },
    Objects {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<RuntimeType>,
    },
    /// Exact temporal history for one non-claim semantic identity. `model`
    /// selects its physical family; the stamped schema remains authoritative
    /// for record-like and event-like logical-model resolution.
    Identity {
        model: RuntimeLogicalModel,
        reference: RuntimeRef,
    },
    /// Complete histories for a lexical interval of IDs in one typed family.
    /// Bounds select identities, never individual temporal versions. At least
    /// one bound is required; use the family source for an unbounded scan.
    IdentityRange {
        model: RuntimeLogicalModel,
        kind: RuntimeType,
        lower: Bound<RuntimeId>,
        upper: Bound<RuntimeId>,
    },
    /// Temporal adjacency history for one node and relation type: every
    /// relation version that placed `node` at the `direction` endpoint, plus
    /// every retirement of a relation while it did. The range selects the
    /// relations incident to the node; each relation's identity history
    /// remains authoritative for its state at a valid time.
    Adjacency {
        relation: RuntimeType,
        node: RuntimeRef,
        direction: RuntimeAdjacencyDirection,
    },
}

/// Endpoint of a relation that an adjacency range is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAdjacencyDirection {
    /// Relations whose `from` endpoint is the node.
    Outgoing,
    /// Relations whose `to` endpoint is the node.
    Incoming,
}

impl RuntimeVersionedSource {
    pub const fn all() -> [Self; 9] {
        [
            Self::Schema,
            Self::Claims { predicate: None },
            Self::Records { kind: None },
            Self::Relations { kind: None },
            Self::Events { kind: None },
            Self::Vectors { kind: None },
            Self::Series { kind: None },
            Self::Geo { kind: None },
            Self::Objects { kind: None },
        ]
    }

    fn path(&self) -> Result<RuntimeReadAccessPath> {
        match self {
            Self::Schema => Ok(RuntimeReadAccessPath::SchemaVersions),
            Self::Claims { .. } => Ok(RuntimeReadAccessPath::ClaimVersions),
            Self::Records { .. } => Ok(RuntimeReadAccessPath::RecordVersions),
            Self::Relations { .. } => Ok(RuntimeReadAccessPath::RelationVersions),
            Self::Events { .. } => Ok(RuntimeReadAccessPath::EventVersions),
            Self::Vectors { .. } => Ok(RuntimeReadAccessPath::VectorVersions),
            Self::Series { .. } => Ok(RuntimeReadAccessPath::SeriesVersions),
            Self::Geo { .. } => Ok(RuntimeReadAccessPath::GeoVersions),
            Self::Objects { .. } => Ok(RuntimeReadAccessPath::ObjectVersions),
            Self::Identity { model, .. } | Self::IdentityRange { model, .. } => {
                access_path_for_model(*model)
            }
            Self::Adjacency { .. } => Ok(RuntimeReadAccessPath::AdjacencyVersions),
        }
    }
}

/// Closed physical path vocabulary for direct-read evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeReadAccessPath {
    ReadStamp,
    SchemaVersions,
    ClaimVersions,
    RecordVersions,
    RelationVersions,
    AdjacencyVersions,
    EventVersions,
    VectorVersions,
    SeriesVersions,
    GeoVersions,
    ObjectVersions,
    AccumulatorProof,
}

/// Counters attributed to one closed direct-read access path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeReadPathEvidence {
    pub path: RuntimeReadAccessPath,
    pub point_reads: u64,
    pub range_scans: u64,
    pub keys_examined: u64,
    pub values_decoded: u64,
    pub decoded_bytes: u64,
}

/// Complete logical I/O evidence for one stamped direct read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeReadEvidence {
    pub contract_version: u16,
    pub key_budget: u64,
    pub point_reads: u64,
    pub range_scans: u64,
    pub keys_examined: u64,
    pub values_decoded: u64,
    pub decoded_bytes: u64,
    pub stamp_validation: RuntimeReadValidation,
    pub paths: Vec<RuntimeReadPathEvidence>,
}

impl RuntimeReadEvidence {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != RUNTIME_VERSIONED_READ_CONTRACT_VERSION
            || self.key_budget == 0
            || self.keys_examined > self.key_budget
            || self.values_decoded > self.keys_examined
            || self.paths.is_empty()
        {
            return Err(Error::Substrate(
                "runtime versioned-read evidence header is invalid".into(),
            ));
        }
        let mut prior = None;
        let mut totals = PathCounters::default();
        for path in &self.paths {
            if prior.is_some_and(|value| value >= path.path)
                || path.values_decoded > path.keys_examined
            {
                return Err(Error::Substrate(
                    "runtime versioned-read path evidence is invalid".into(),
                ));
            }
            prior = Some(path.path);
            totals.add(
                path.point_reads,
                path.range_scans,
                path.keys_examined,
                path.values_decoded,
                path.decoded_bytes,
            )?;
        }
        if totals.point_reads != self.point_reads
            || totals.range_scans != self.range_scans
            || totals.keys_examined != self.keys_examined
            || totals.values_decoded != self.values_decoded
            || totals.decoded_bytes != self.decoded_bytes
        {
            return Err(Error::Substrate(
                "runtime versioned-read evidence totals do not match its paths".into(),
            ));
        }
        Ok(())
    }
}

/// Authenticated semantic versions returned from exactly one read stamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeVersionedRead {
    pub read: ReadStamp,
    pub changes: Vec<RuntimeChange>,
    pub evidence: RuntimeReadEvidence,
}

pub(crate) fn read_versioned(
    transaction: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    sources: &[RuntimeVersionedSource],
    budget: RuntimeReadBudget,
) -> Result<RuntimeVersionedRead> {
    read_versioned_expanding(transaction, read, sources, budget, &mut |_| Vec::new())
}

/// Reads typed ranges in steps under one stamp validation, key budget, and
/// accumulator. Every step's changes are key-validated and authenticated
/// before `expand` sees them in global cursor order; `expand` returns the
/// next step's ranges. A range already read is not read again, a range that
/// overlaps a different read range is rejected, and an empty step ends the
/// read.
pub(crate) fn read_versioned_expanding(
    transaction: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    sources: &[RuntimeVersionedSource],
    budget: RuntimeReadBudget,
    expand: &mut dyn FnMut(&[RuntimeChange]) -> Vec<RuntimeVersionedSource>,
) -> Result<RuntimeVersionedRead> {
    budget.validate()?;
    if sources.is_empty() {
        return Err(Error::Substrate(
            "runtime versioned read requires at least one typed source".into(),
        ));
    }
    let mut ranges = SourceRanges::default();
    let mut step = ranges.admit(sources, &read.scope)?;
    if read.accumulator_root.is_none() {
        return Err(Error::ReadStampMismatch(read.manifest_id.clone()));
    }

    let reader = MeteredRead::new(transaction, budget);
    let stamp_validation = reader.with_path(RuntimeReadAccessPath::ReadStamp, || {
        validate_read_stamp(&reader, read)
    });
    let stamp_validation = reader.preserve_budget_error(stamp_validation)?;
    let mut accumulator = None;
    // Owned by this one immutable transaction/stamp, never by the store.
    let mut proof_nodes = ProofNodes::new();
    let mut changes = BTreeMap::<u64, RuntimeChange>::new();
    while !step.is_empty() {
        let mut selected = BTreeMap::<u64, RuntimeChange>::new();
        for source in &step {
            let path = source.path()?;
            let encoded = reader.with_path(path, || scan_source(&reader, read, source))?;
            for (stored_key, bytes) in encoded {
                let change: RuntimeChange = serde_json::from_slice(&bytes)?;
                if change.cursor > read.commit_cursor {
                    continue;
                }
                validate_version_entry(source, read, &stored_key, &change)?;
                insert_version(&mut selected, change)?;
            }
        }
        let authentication = reader.with_path(RuntimeReadAccessPath::AccumulatorProof, || {
            let accumulator = match &mut accumulator {
                Some(accumulator) => accumulator,
                empty => empty.insert(open_accumulator(&reader, read, &mut proof_nodes)?),
            };
            authenticate_changes(
                &reader,
                read,
                accumulator,
                &mut proof_nodes,
                selected
                    .values()
                    .filter(|change| !changes.contains_key(&change.cursor)),
            )
        });
        reader.preserve_budget_error(authentication)?;
        for change in selected.values() {
            insert_version(&mut changes, change.clone())?;
        }
        let next = expand(&selected.into_values().collect::<Vec<_>>());
        step = ranges.admit(&next, &read.scope)?;
    }
    let evidence = reader.finish(stamp_validation)?;
    let result = RuntimeVersionedRead {
        read: read.clone(),
        changes: changes.into_values().collect(),
        evidence,
    };
    result.evidence.validate()?;
    Ok(result)
}

fn insert_version(changes: &mut BTreeMap<u64, RuntimeChange>, change: RuntimeChange) -> Result<()> {
    match changes.entry(change.cursor) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(change);
        }
        std::collections::btree_map::Entry::Occupied(entry) => {
            if entry.get() != &change {
                return Err(Error::Substrate(format!(
                    "runtime cursor {} resolved to conflicting semantic versions",
                    change.cursor
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn schema_at_read(
    read: &ReadStamp,
    changes: &[RuntimeChange],
) -> Result<rrd_core::RuntimeSchemaRegistry> {
    let expected_revision = read
        .schema_revision
        .ok_or_else(|| Error::RuntimeSchemaMissing(read.scope.to_string()))?;
    let schema = changes
        .iter()
        .filter(|change| change.scope == read.scope && change.cursor <= read.commit_cursor)
        .filter_map(|change| match &change.mutation {
            RuntimeMutation::Schema { registry } => Some(registry),
            _ => None,
        })
        .next_back()
        .cloned()
        .ok_or_else(|| Error::RuntimeSchemaMissing(read.scope.to_string()))?;
    if schema.revision != expected_revision {
        return Err(Error::ReadStampUnavailable(format!(
            "schema revision {} for scope {} at cursor {}",
            expected_revision, read.scope, read.commit_cursor
        )));
    }
    Ok(schema)
}

/// Every nonempty half-open physical interval admitted into one read. With
/// no prior overlap, only its sorted neighbours can overlap a new interval.
#[derive(Default)]
struct SourceRanges {
    admitted: BTreeMap<Vec<u8>, (Vec<u8>, RuntimeVersionedSource)>,
}

impl SourceRanges {
    fn admit(
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

fn overlapping_ranges(source: &RuntimeVersionedSource) -> Error {
    match source.path() {
        Ok(path) => Error::Substrate(format!(
            "runtime versioned read contains overlapping {path:?} ranges"
        )),
        Err(error) => error,
    }
}

fn scan_source(
    reader: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    source: &RuntimeVersionedSource,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let (_, start, end) = source_interval(source, &read.scope)?;
    reader.scan_range(&start, &end, usize::MAX)
}

fn source_interval(
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

fn source_range(
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

const fn adjacency_space(direction: RuntimeAdjacencyDirection) -> Space {
    match direction {
        RuntimeAdjacencyDirection::Outgoing => keyspaces::RUNTIME_OUTGOING_EDGE_VERSIONS,
        RuntimeAdjacencyDirection::Incoming => keyspaces::RUNTIME_INCOMING_EDGE_VERSIONS,
    }
}

fn version_range(
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

fn validate_version_entry(
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
fn validate_adjacency_entry(
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

fn expected_version_key(
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

struct VersionAddress {
    space: Space,
    reference: RuntimeRef,
    effective_at: u64,
}

fn version_address(change: &RuntimeChange) -> Result<VersionAddress> {
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
            keyspaces::runtime_event_reference(event, change.cursor)?,
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

fn source_accepts_address(
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

fn access_path_for_model(model: RuntimeLogicalModel) -> Result<RuntimeReadAccessPath> {
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

fn version_space_for_model(model: RuntimeLogicalModel) -> Result<Space> {
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
        | RuntimeReadAccessPath::AccumulatorProof => {
            return Err(Error::Substrate(
                "logical model has no identity-version space".into(),
            ));
        }
    })
}

fn open_accumulator(
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

fn authenticate_changes<'a>(
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

#[derive(Debug, Clone, Copy, Default)]
struct PathCounters {
    point_reads: u64,
    range_scans: u64,
    keys_examined: u64,
    values_decoded: u64,
    decoded_bytes: u64,
}

impl PathCounters {
    fn add(
        &mut self,
        point_reads: u64,
        range_scans: u64,
        keys_examined: u64,
        values_decoded: u64,
        decoded_bytes: u64,
    ) -> Result<()> {
        self.point_reads = checked_add(self.point_reads, point_reads)?;
        self.range_scans = checked_add(self.range_scans, range_scans)?;
        self.keys_examined = checked_add(self.keys_examined, keys_examined)?;
        self.values_decoded = checked_add(self.values_decoded, values_decoded)?;
        self.decoded_bytes = checked_add(self.decoded_bytes, decoded_bytes)?;
        Ok(())
    }
}

struct MeteredRead<'a, R: AccessRead + ?Sized> {
    inner: &'a R,
    budget: RuntimeReadBudget,
    budget_observed: Cell<Option<u64>>,
    path: Cell<RuntimeReadAccessPath>,
    totals: RefCell<PathCounters>,
    paths: RefCell<BTreeMap<RuntimeReadAccessPath, PathCounters>>,
}

impl<'a, R: AccessRead + ?Sized> MeteredRead<'a, R> {
    fn new(inner: &'a R, budget: RuntimeReadBudget) -> Self {
        Self {
            inner,
            budget,
            budget_observed: Cell::new(None),
            path: Cell::new(RuntimeReadAccessPath::ReadStamp),
            totals: RefCell::new(PathCounters::default()),
            paths: RefCell::new(BTreeMap::new()),
        }
    }

    fn with_path<T>(&self, path: RuntimeReadAccessPath, operation: impl FnOnce() -> T) -> T {
        let prior = self.path.replace(path);
        let result = operation();
        self.path.set(prior);
        result
    }

    fn preserve_budget_error<T>(&self, result: Result<T>) -> Result<T> {
        if let Some(observed) = self.budget_observed.get() {
            return Err(Error::RuntimeReadBudgetExceeded {
                limit: self.budget.max_keys,
                observed,
            });
        }
        result
    }

    fn record(
        &self,
        point_reads: u64,
        range_scans: u64,
        keys_examined: u64,
        values_decoded: u64,
        decoded_bytes: u64,
    ) -> Result<()> {
        self.totals.borrow_mut().add(
            point_reads,
            range_scans,
            keys_examined,
            values_decoded,
            decoded_bytes,
        )?;
        self.paths
            .borrow_mut()
            .entry(self.path.get())
            .or_default()
            .add(
                point_reads,
                range_scans,
                keys_examined,
                values_decoded,
                decoded_bytes,
            )
    }

    fn finish(self, stamp_validation: RuntimeReadValidation) -> Result<RuntimeReadEvidence> {
        let totals = self.totals.into_inner();
        let paths = self
            .paths
            .into_inner()
            .into_iter()
            .map(|(path, counters)| RuntimeReadPathEvidence {
                path,
                point_reads: counters.point_reads,
                range_scans: counters.range_scans,
                keys_examined: counters.keys_examined,
                values_decoded: counters.values_decoded,
                decoded_bytes: counters.decoded_bytes,
            })
            .collect();
        Ok(RuntimeReadEvidence {
            contract_version: RUNTIME_VERSIONED_READ_CONTRACT_VERSION,
            key_budget: self.budget.max_keys,
            point_reads: totals.point_reads,
            range_scans: totals.range_scans,
            keys_examined: totals.keys_examined,
            values_decoded: totals.values_decoded,
            decoded_bytes: totals.decoded_bytes,
            stamp_validation,
            paths,
        })
    }
}

impl<R: AccessRead + ?Sized> AccessRead for MeteredRead<'_, R> {
    fn read_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let observed = checked_add(self.totals.borrow().keys_examined, 1)?;
        if observed > self.budget.max_keys {
            self.budget_observed.set(Some(observed));
            return Err(Error::RuntimeReadBudgetExceeded {
                limit: self.budget.max_keys,
                observed,
            });
        }
        let value = self.inner.read_key(key)?;
        let (decoded, bytes) = value.as_ref().map_or((0, 0), |value| {
            (1, u64::try_from(value.len()).unwrap_or(u64::MAX))
        });
        self.record(1, 0, 1, decoded, bytes)?;
        Ok(value)
    }

    fn scan_range(
        &self,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let examined = self.totals.borrow().keys_examined;
        let remaining = self.budget.max_keys.saturating_sub(examined);
        let probe_limit = usize::try_from(remaining.saturating_add(1))
            .unwrap_or(usize::MAX)
            .min(limit);
        let rows = self.inner.scan_range(start, end, probe_limit)?;
        let row_count = u64::try_from(rows.len()).unwrap_or(u64::MAX);
        let observed = checked_add(examined, row_count)?;
        if observed > self.budget.max_keys {
            self.budget_observed.set(Some(observed));
            return Err(Error::RuntimeReadBudgetExceeded {
                limit: self.budget.max_keys,
                observed,
            });
        }
        let decoded_bytes = rows.iter().try_fold(0_u64, |total, (_, value)| {
            checked_add(total, u64::try_from(value.len()).unwrap_or(u64::MAX))
        })?;
        self.record(0, 1, row_count, row_count, decoded_bytes)?;
        Ok(rows)
    }
}

fn checked_add(left: u64, right: u64) -> Result<u64> {
    left.checked_add(right).ok_or(Error::SequenceOverflow)
}

#[cfg(test)]
#[path = "versioned_identity_ranges.rs"]
mod identity_range_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use rrd_core::{RuntimeCommit, RuntimeSchemaRegistry, ScopeId};

    fn schema_change() -> (ReadStamp, RuntimeChange) {
        let scope = ScopeId::new("project:version-entry-validation").unwrap();
        let mut registry = RuntimeSchemaRegistry::empty(1, "version entry validation");
        registry
            .define_record_table(
                RuntimeType::new("document").unwrap(),
                RuntimeLogicalModel::Relational,
                Default::default(),
            )
            .unwrap();
        let commit = RuntimeCommit {
            scope: scope.clone(),
            at: 100,
            actor: "test:version-entry-validation".into(),
            expected_cursor: 0,
            mutations: vec![RuntimeMutation::Schema { registry }],
        };
        let change = RuntimeChange::committed(
            1,
            &commit,
            &commit.digest(),
            0,
            commit.mutations[0].clone(),
            None,
        );
        let read = ReadStamp::authenticated(
            scope,
            Some(1),
            0,
            1,
            Some(change.digest.clone()),
            "11".repeat(32),
        )
        .unwrap();
        (read, change)
    }

    #[test]
    fn a_version_value_cannot_move_to_another_typed_key_or_family() {
        let (read, change) = schema_change();
        let mut key = keyspaces::runtime_schema_version_key(&read.scope, change.cursor);
        validate_version_entry(&RuntimeVersionedSource::Schema, &read, &key, &change).unwrap();

        *key.last_mut().unwrap() ^= 1;
        assert!(matches!(
            validate_version_entry(&RuntimeVersionedSource::Schema, &read, &key, &change,),
            Err(Error::Codec(_))
        ));
        assert!(matches!(
            validate_version_entry(
                &RuntimeVersionedSource::Records { kind: None },
                &read,
                &keyspaces::runtime_schema_version_key(&read.scope, change.cursor),
                &change,
            ),
            Err(Error::Codec(_))
        ));
    }

    fn admit(sources: &[RuntimeVersionedSource]) -> Result<Vec<RuntimeVersionedSource>> {
        SourceRanges::default().admit(
            sources,
            &ScopeId::new("project:overlap-validation").unwrap(),
        )
    }

    fn adjacency(node: &str, relation: &str) -> RuntimeVersionedSource {
        RuntimeVersionedSource::Adjacency {
            relation: RuntimeType::new(relation).unwrap(),
            node: RuntimeRef::new("document", node).unwrap(),
            direction: RuntimeAdjacencyDirection::Outgoing,
        }
    }

    fn committed(mutation: RuntimeMutation, cursor: u64) -> RuntimeChange {
        let commit = RuntimeCommit {
            scope: ScopeId::new("project:adjacency-entry-validation").unwrap(),
            at: 300,
            actor: "test:adjacency-entry-validation".into(),
            expected_cursor: cursor - 1,
            mutations: vec![mutation],
        };
        RuntimeChange::committed(
            cursor,
            &commit,
            &commit.digest(),
            0,
            commit.mutations[0].clone(),
            None,
        )
    }

    #[test]
    fn an_adjacency_value_must_name_its_node_type_identity_time_and_cursor() {
        let relation = rrd_core::RuntimeRelation {
            reference: RuntimeRef::new("depends_on", "a-b").unwrap(),
            from: RuntimeRef::new("document", "a").unwrap(),
            to: RuntimeRef::new("document", "b").unwrap(),
            valid_from: 100,
            valid_to: None,
            properties: Default::default(),
        };
        let depends_on = RuntimeType::new("depends_on").unwrap();
        let outgoing = RuntimeAdjacencyDirection::Outgoing;
        let space = adjacency_space(outgoing);
        let put = committed(
            RuntimeMutation::Relation {
                relation: relation.clone(),
            },
            4,
        );
        let key = |relation: &rrd_core::RuntimeRelation, at: u64, cursor: u64| {
            keyspaces::runtime_adjacency_version_key(space, &put.scope, relation, at, cursor)
        };
        let validate = |node: &RuntimeRef,
                        direction: RuntimeAdjacencyDirection,
                        stored: &[u8],
                        change: &RuntimeChange| {
            validate_adjacency_entry(&depends_on, node, direction, stored, change)
        };
        validate(&relation.from, outgoing, &key(&relation, 100, 4), &put).unwrap();
        let mut moved = relation.clone();
        moved.to = RuntimeRef::new("document", "c").unwrap();
        for (node, direction, stored) in [
            (&relation.from, outgoing, key(&moved, 100, 4)),
            (&relation.from, outgoing, key(&relation, 101, 4)),
            (&relation.from, outgoing, key(&relation, 100, 5)),
            (&relation.to, outgoing, key(&relation, 100, 4)),
            (
                &relation.from,
                RuntimeAdjacencyDirection::Incoming,
                key(&relation, 100, 4),
            ),
        ] {
            assert!(matches!(
                validate(node, direction, &stored, &put),
                Err(Error::Codec(_))
            ));
        }

        let retirement = committed(
            RuntimeMutation::Retire {
                retirement: rrd_core::RuntimeRetirement {
                    model: RuntimeLogicalModel::GraphRelation,
                    reference: relation.reference.clone(),
                    effective_at: 300,
                },
            },
            5,
        );
        validate(
            &relation.from,
            outgoing,
            &key(&relation, 300, 5),
            &retirement,
        )
        .unwrap();
        // The far endpoint is not part of a retirement and is not checked.
        validate(&relation.from, outgoing, &key(&moved, 300, 5), &retirement).unwrap();
        let mut renamed = relation.clone();
        renamed.reference = RuntimeRef::new("depends_on", "a-c").unwrap();
        let mut retyped = relation.clone();
        retyped.reference = RuntimeRef::new("cites", "a-b").unwrap();
        for stored in [
            key(&relation, 299, 5),
            key(&relation, 300, 6),
            key(&renamed, 300, 5),
            key(&retyped, 300, 5),
        ] {
            assert!(matches!(
                validate(&relation.from, outgoing, &stored, &retirement),
                Err(Error::Codec(_))
            ));
        }
        assert!(validate(&relation.to, outgoing, &key(&relation, 300, 5), &retirement).is_err());
        let record_retirement = committed(
            RuntimeMutation::Retire {
                retirement: rrd_core::RuntimeRetirement {
                    model: RuntimeLogicalModel::Document,
                    reference: relation.reference.clone(),
                    effective_at: 300,
                },
            },
            5,
        );
        assert!(matches!(
            validate(
                &relation.from,
                outgoing,
                &key(&relation, 300, 5),
                &record_retirement
            ),
            Err(Error::Codec(_))
        ));
    }

    #[test]
    fn overlapping_family_ranges_are_rejected_before_storage_access() {
        assert!(admit(&[
            RuntimeVersionedSource::Records { kind: None },
            RuntimeVersionedSource::Records {
                kind: Some(RuntimeType::new("document").unwrap()),
            },
        ])
        .is_err());
        assert!(admit(&[
            RuntimeVersionedSource::Records {
                kind: Some(RuntimeType::new("document").unwrap()),
            },
            RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::Document,
                reference: RuntimeRef::new("document", "readme").unwrap(),
            },
        ])
        .is_err());
        // One physical identity selected under two record-like models is one
        // range, not two reads.
        assert!(admit(&[
            RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::Document,
                reference: RuntimeRef::new("document", "readme").unwrap(),
            },
            RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::Relational,
                reference: RuntimeRef::new("document", "readme").unwrap(),
            },
        ])
        .is_err());
    }

    #[test]
    fn admitted_ranges_deduplicate_and_find_overlap_in_any_order() {
        let a = adjacency("a", "depends_on");
        let ab = adjacency("ab", "depends_on");
        let other_kind = adjacency("a", "depends_onx");
        let incoming = RuntimeVersionedSource::Adjacency {
            relation: RuntimeType::new("depends_on").unwrap(),
            node: RuntimeRef::new("document", "a").unwrap(),
            direction: RuntimeAdjacencyDirection::Incoming,
        };
        assert_eq!(
            admit(&[
                a.clone(),
                ab.clone(),
                a.clone(),
                other_kind.clone(),
                incoming.clone()
            ])
            .unwrap(),
            vec![a.clone(), ab, other_kind, incoming]
        );

        let scope = ScopeId::new("project:overlap-validation").unwrap();
        let mut ranges = SourceRanges::default();
        let family = RuntimeVersionedSource::Records {
            kind: Some(RuntimeType::new("document").unwrap()),
        };
        let identity = RuntimeVersionedSource::Identity {
            model: RuntimeLogicalModel::Document,
            reference: RuntimeRef::new("document", "readme").unwrap(),
        };
        assert_eq!(
            ranges
                .admit(std::slice::from_ref(&identity), &scope)
                .unwrap(),
            vec![identity.clone()]
        );
        assert!(ranges
            .admit(std::slice::from_ref(&identity), &scope)
            .unwrap()
            .is_empty());
        assert!(ranges.admit(std::slice::from_ref(&family), &scope).is_err());
    }
}
