use crate::{Error, IndexCatalogue, IndexCatalogueRepository, Query, Result, Source};
use rrd_core::{
    Predicate, ReadStamp, RuntimeLogicalModel, RuntimeMutation, RuntimeRef, RuntimeSchemaRegistry,
    RuntimeType, ScopeId,
};
use rrd_store::{RuntimeReadBudget, RuntimeReadEvidence, RuntimeVersionedSource, StorageEngine};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub cursor: u64,
    pub registry: RuntimeSchemaRegistry,
}

/// Schema history and the immutable read stamp against which it was captured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    pub read: ReadStamp,
    /// Number of authenticated semantic versions selected before catalogue
    /// reduction.
    pub selected_versions: usize,
    pub schemas: Vec<SchemaVersion>,
    pub indexes: IndexCatalogue,
    pub source_watermarks: SourceWatermarks,
    /// Exact typed semantic ranges used to construct this catalogue.
    pub version_sources: Vec<RuntimeVersionedSource>,
    /// Logical storage evidence for the stamped catalogue read.
    pub read_evidence: RuntimeReadEvidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceWatermarks {
    pub schema: u64,
    pub records: BTreeMap<RuntimeType, u64>,
    pub relations: BTreeMap<RuntimeType, u64>,
    pub events: BTreeMap<RuntimeType, u64>,
    pub series: BTreeMap<RuntimeType, u64>,
    pub geo: BTreeMap<RuntimeType, u64>,
    pub claims: BTreeMap<Predicate, u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub series_targets: BTreeMap<RuntimeRef, RuntimeType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[doc(hidden)]
    pub schema_history: Vec<u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub record_history: BTreeMap<RuntimeType, Vec<u64>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub relation_history: BTreeMap<RuntimeType, Vec<u64>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub event_history: BTreeMap<RuntimeType, Vec<u64>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub series_history: BTreeMap<RuntimeType, Vec<u64>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub geo_history: BTreeMap<RuntimeType, Vec<u64>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[doc(hidden)]
    pub claim_history: BTreeMap<Predicate, Vec<u64>>,
}

impl SourceWatermarks {
    fn observe(&mut self, cursor: u64, mutation: &RuntimeMutation) {
        match mutation {
            RuntimeMutation::Schema { .. } => {
                self.schema = cursor;
                self.schema_history.push(cursor);
            }
            RuntimeMutation::Record { record } => {
                self.records.insert(record.reference.kind.clone(), cursor);
                self.record_history
                    .entry(record.reference.kind.clone())
                    .or_default()
                    .push(cursor);
            }
            RuntimeMutation::Relation { relation } => {
                self.relations
                    .insert(relation.reference.kind.clone(), cursor);
                self.relation_history
                    .entry(relation.reference.kind.clone())
                    .or_default()
                    .push(cursor);
            }
            RuntimeMutation::Event { event } => {
                self.events.insert(event.kind.clone(), cursor);
                self.event_history
                    .entry(event.kind.clone())
                    .or_default()
                    .push(cursor);
            }
            RuntimeMutation::SeriesSample { sample } => {
                self.series_targets
                    .insert(sample.reference.clone(), sample.series.kind.clone());
                self.series.insert(sample.series.kind.clone(), cursor);
                self.series_history
                    .entry(sample.series.kind.clone())
                    .or_default()
                    .push(cursor);
            }
            RuntimeMutation::Geo { geo } => {
                self.geo.insert(geo.reference.kind.clone(), cursor);
                self.geo_history
                    .entry(geo.reference.kind.clone())
                    .or_default()
                    .push(cursor);
            }
            RuntimeMutation::Claim { claim } => {
                self.claims.insert(claim.predicate.clone(), cursor);
                self.claim_history
                    .entry(claim.predicate.clone())
                    .or_default()
                    .push(cursor);
            }
            RuntimeMutation::Retire { retirement } => {
                if retirement.model.is_record_like() {
                    self.records
                        .insert(retirement.reference.kind.clone(), cursor);
                    self.record_history
                        .entry(retirement.reference.kind.clone())
                        .or_default()
                        .push(cursor);
                } else if retirement.model == rrd_core::RuntimeLogicalModel::GraphRelation {
                    self.relations
                        .insert(retirement.reference.kind.clone(), cursor);
                    self.relation_history
                        .entry(retirement.reference.kind.clone())
                        .or_default()
                        .push(cursor);
                } else if retirement.model.is_event_like() {
                    self.events
                        .insert(retirement.reference.kind.clone(), cursor);
                    self.event_history
                        .entry(retirement.reference.kind.clone())
                        .or_default()
                        .push(cursor);
                } else if retirement.model == rrd_core::RuntimeLogicalModel::TimeSeries {
                    if let Some(kind) = self.series_targets.get(&retirement.reference).cloned() {
                        self.series.insert(kind.clone(), cursor);
                        self.series_history.entry(kind).or_default().push(cursor);
                    }
                } else if retirement.model == rrd_core::RuntimeLogicalModel::Geo {
                    self.geo.insert(retirement.reference.kind.clone(), cursor);
                    self.geo_history
                        .entry(retirement.reference.kind.clone())
                        .or_default()
                        .push(cursor);
                }
            }
            RuntimeMutation::Vector { .. } | RuntimeMutation::Object { .. } => {}
        }
    }

    /// Last source or schema change visible at `known_at_cursor`.
    ///
    /// A latest-only watermark is not a valid historical coordinate: a source
    /// may have advanced after the requested cursor. Retaining the cursor
    /// history keeps `KNOWN n` planning and exact-index selection bounded by
    /// the same transaction-time prefix as the direct version read.
    pub fn for_source_at(&self, source: &Source, known_at_cursor: u64) -> u64 {
        let data = match source {
            Source::Record { kind } => historical(
                self.record_history.get(kind),
                self.records.get(kind).copied(),
                known_at_cursor,
            ),
            Source::Relation { kind } => historical(
                self.relation_history.get(kind),
                self.relations.get(kind).copied(),
                known_at_cursor,
            ),
            Source::Event { kind } => historical(
                self.event_history.get(kind),
                self.events.get(kind).copied(),
                known_at_cursor,
            ),
            Source::Series { kind } => historical(
                self.series_history.get(kind),
                self.series.get(kind).copied(),
                known_at_cursor,
            ),
            Source::Geo { kind } => historical(
                self.geo_history.get(kind),
                self.geo.get(kind).copied(),
                known_at_cursor,
            ),
            // No family-wide watermark is captured for a traversal: its
            // source is the neighbourhood visited at execution, current
            // through the requested known cursor.
            Source::Traversal { .. } => known_at_cursor,
            Source::Claim {
                predicate: Some(predicate),
            } => historical(
                self.claim_history.get(predicate),
                self.claims.get(predicate).copied(),
                known_at_cursor,
            ),
            Source::Claim { predicate: None } => self
                .claim_history
                .values()
                .map(|history| latest(history, known_at_cursor))
                .max()
                .unwrap_or_else(|| {
                    self.claims
                        .values()
                        .copied()
                        .filter(|cursor| *cursor <= known_at_cursor)
                        .max()
                        .unwrap_or(0)
                }),
        };
        data.max(historical(
            Some(&self.schema_history),
            Some(self.schema),
            known_at_cursor,
        ))
    }
}

fn historical(history: Option<&Vec<u64>>, fallback: Option<u64>, known_at_cursor: u64) -> u64 {
    history
        .and_then(|history| {
            let cursor = latest(history, known_at_cursor);
            (cursor != 0).then_some(cursor)
        })
        .unwrap_or_else(|| {
            fallback
                .filter(|cursor| *cursor <= known_at_cursor)
                .unwrap_or(0)
        })
}

fn latest(history: &[u64], known_at_cursor: u64) -> u64 {
    history
        .iter()
        .rev()
        .copied()
        .find(|cursor| *cursor <= known_at_cursor)
        .unwrap_or(0)
}

impl Catalog {
    /// Captures the schema and only the semantic families required by `query`.
    pub fn capture_for_query<E: StorageEngine>(
        engine: &E,
        scope: &ScopeId,
        query: &Query,
        budget: RuntimeReadBudget,
    ) -> Result<Self> {
        let read = engine.runtime().read_stamp(scope)?;
        Self::capture_for_query_at(engine, read, query, budget)
    }

    /// Captures a query catalogue against a caller-owned immutable read stamp.
    pub fn capture_for_query_at<E: StorageEngine>(
        engine: &E,
        read: ReadStamp,
        query: &Query,
        budget: RuntimeReadBudget,
    ) -> Result<Self> {
        let mut sources = vec![query.source.clone()];
        if let Some(join) = &query.join {
            sources.push(join.source.clone());
        }
        Self::capture_for_sources_at(engine, read, &sources, budget)
    }

    /// Captures schema history and the explicitly selected source families.
    /// An empty `sources` slice is a schema-only catalogue, not an implicit
    /// whole-estate scan.
    pub fn capture_for_sources<E: StorageEngine>(
        engine: &E,
        scope: &ScopeId,
        sources: &[Source],
        budget: RuntimeReadBudget,
    ) -> Result<Self> {
        let read = engine.runtime().read_stamp(scope)?;
        Self::capture_for_sources_at(engine, read, sources, budget)
    }

    /// Observer-safe source capture at one immutable transaction coordinate.
    pub fn capture_for_sources_at<E: StorageEngine>(
        engine: &E,
        read: ReadStamp,
        sources: &[Source],
        budget: RuntimeReadBudget,
    ) -> Result<Self> {
        read.validate()
            .map_err(|error| Error::Catalog(error.to_string()))?;
        let mut version_sources = vec![RuntimeVersionedSource::Schema];
        version_sources.extend(version_sources_for_sources(sources));
        let version_sources = normalize_version_sources(version_sources);
        let versioned = engine
            .runtime()
            .read_versioned(&read, &version_sources, budget)?;
        let selected_versions = versioned.changes.len();
        let mut source_watermarks = SourceWatermarks::default();
        let mut schemas = Vec::new();
        for change in versioned.changes {
            source_watermarks.observe(change.cursor, &change.mutation);
            if let RuntimeMutation::Schema { registry } = change.mutation {
                schemas.push(SchemaVersion {
                    cursor: change.cursor,
                    registry,
                });
            }
        }
        let indexes = IndexCatalogueRepository::new(engine, read.scope.clone()).load()?;
        Ok(Self {
            read,
            selected_versions,
            schemas,
            indexes,
            source_watermarks,
            version_sources,
            read_evidence: versioned.evidence,
        })
    }

    pub(crate) fn require_version_sources(
        &self,
        required: &[RuntimeVersionedSource],
    ) -> Result<()> {
        if let Some(missing) = required.iter().find(|required| {
            !self
                .version_sources
                .iter()
                .any(|available| version_source_covers(available, required))
        }) {
            return Err(Error::Catalog(format!(
                "catalogue did not capture required semantic source {missing:?}"
            )));
        }
        Ok(())
    }

    pub fn schema_at(&self, cursor: u64) -> Option<&RuntimeSchemaRegistry> {
        self.schemas
            .iter()
            .rev()
            .find(|version| version.cursor <= cursor)
            .map(|version| &version.registry)
    }

    pub fn source_cursor(&self, source: &Source) -> u64 {
        self.source_watermarks
            .for_source_at(source, self.read.commit_cursor)
    }
}

pub(crate) fn version_sources_for_sources(sources: &[Source]) -> Vec<RuntimeVersionedSource> {
    normalize_version_sources(
        sources
            .iter()
            .flat_map(version_sources_for_source)
            .collect(),
    )
}

fn version_sources_for_source(source: &Source) -> Vec<RuntimeVersionedSource> {
    match source {
        Source::Record { kind } => vec![RuntimeVersionedSource::Records {
            kind: Some(kind.clone()),
        }],
        Source::Relation { kind } => vec![RuntimeVersionedSource::Relations {
            kind: Some(kind.clone()),
        }],
        Source::Event { kind } => vec![RuntimeVersionedSource::Events {
            kind: Some(kind.clone()),
        }],
        // A sample is physically addressed by its own type while rrflowQL
        // selects the referenced series type. Until Gate E adds the native
        // series-target index, the complete scoped series family is exact.
        Source::Series { .. } => vec![RuntimeVersionedSource::Series { kind: None }],
        Source::Geo { kind } => vec![RuntimeVersionedSource::Geo {
            kind: Some(kind.clone()),
        }],
        // A bounded traversal reads its start identity and the adjacency of
        // the nodes it visits at execution; its catalogue needs only schema.
        Source::Traversal { .. } => Vec::new(),
        Source::Claim { predicate } => vec![RuntimeVersionedSource::Claims {
            predicate: predicate.clone(),
        }],
    }
}

pub(crate) fn normalize_version_sources(
    sources: Vec<RuntimeVersionedSource>,
) -> Vec<RuntimeVersionedSource> {
    let mut normalized = Vec::<RuntimeVersionedSource>::new();
    for source in sources {
        if normalized
            .iter()
            .any(|available| version_source_covers(available, &source))
        {
            continue;
        }
        normalized.retain(|available| !version_source_covers(&source, available));
        normalized.push(source);
    }
    normalized
}

pub(crate) fn version_source_covers(
    available: &RuntimeVersionedSource,
    required: &RuntimeVersionedSource,
) -> bool {
    use RuntimeVersionedSource as Version;
    match (available, required) {
        (Version::Schema, Version::Schema) => true,
        (
            Version::Claims {
                predicate: available,
            },
            Version::Claims {
                predicate: required,
            },
        ) => available.is_none() || available == required,
        (Version::Records { kind: available }, Version::Records { kind: required })
        | (Version::Relations { kind: available }, Version::Relations { kind: required })
        | (Version::Events { kind: available }, Version::Events { kind: required })
        | (Version::Vectors { kind: available }, Version::Vectors { kind: required })
        | (Version::Series { kind: available }, Version::Series { kind: required })
        | (Version::Geo { kind: available }, Version::Geo { kind: required })
        | (Version::Objects { kind: available }, Version::Objects { kind: required }) => {
            available.is_none() || available == required
        }
        (
            Version::Identity {
                model: available_model,
                reference: available_reference,
            },
            Version::Identity {
                model: required_model,
                reference: required_reference,
            },
        ) => available_model == required_model && available_reference == required_reference,
        (Version::Records { kind }, Version::Identity { model, reference })
            if model.is_record_like() =>
        {
            kind.as_ref().is_none_or(|kind| kind == &reference.kind)
        }
        (Version::Relations { kind }, Version::Identity { model, reference })
            if *model == RuntimeLogicalModel::GraphRelation =>
        {
            kind.as_ref().is_none_or(|kind| kind == &reference.kind)
        }
        (Version::Events { kind }, Version::Identity { model, reference })
            if model.is_event_like() =>
        {
            kind.as_ref().is_none_or(|kind| kind == &reference.kind)
        }
        (Version::Vectors { kind }, Version::Identity { model, reference })
            if *model == RuntimeLogicalModel::Vector =>
        {
            kind.as_ref().is_none_or(|kind| kind == &reference.kind)
        }
        (Version::Series { kind }, Version::Identity { model, reference })
            if *model == RuntimeLogicalModel::TimeSeries =>
        {
            kind.as_ref().is_none_or(|kind| kind == &reference.kind)
        }
        (Version::Geo { kind }, Version::Identity { model, reference })
            if *model == RuntimeLogicalModel::Geo =>
        {
            kind.as_ref().is_none_or(|kind| kind == &reference.kind)
        }
        (Version::Objects { kind }, Version::Identity { model, reference })
            if *model == RuntimeLogicalModel::Object =>
        {
            kind.as_ref().is_none_or(|kind| kind == &reference.kind)
        }
        _ => false,
    }
}
