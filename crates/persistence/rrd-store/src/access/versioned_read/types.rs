//! Versioned-read budgets, sources, access paths, evidence and results.

use super::*;

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

    pub(super) fn path(&self) -> Result<RuntimeReadAccessPath> {
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
    /// One vector source's committed pointer deltas in cursor order.
    VectorSourceDeltas,
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

/// One vector source's committed pointer deltas in (`after`, read cursor],
/// with the complete authenticated history of every identity they name, all
/// from one read stamp under one key budget. Every pointer matches its
/// authenticated version; the deltas are an index, never a change-log read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeVectorDeltaRead {
    pub read: ReadStamp,
    pub source: VectorSourceAddress,
    pub after: u64,
    pub deltas: Vec<VectorSourceDelta>,
    pub changes: Vec<RuntimeChange>,
    pub evidence: RuntimeReadEvidence,
}
