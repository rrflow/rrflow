use crate::catalog::{normalize_version_sources, version_sources_for_sources};
use crate::{Catalog, Error, IndexKind, Result};
use crate::{
    ComparisonOperator, CursorExpr, Join, Projection, Query, Source, TimeExpr, ValueExpr,
    QUERY_CONTRACT_VERSION,
};
use rrd_core::{
    digest, ProjectionId, ProjectionState, ReadStamp, RuntimeSchemaRegistry, RuntimeValue,
    RuntimeValueType,
};
use rrd_store::RuntimeVersionedSource;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Parameters = BTreeMap<String, RuntimeValue>;
type FieldTypes = Vec<RuntimeValueType>;
pub type QueryFieldTypes = BTreeMap<String, FieldTypes>;
type FieldCatalog = QueryFieldTypes;
type BuiltinFields = &'static [(&'static str, &'static [RuntimeValueType])];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundFilter {
    pub field: String,
    #[serde(default, skip_serializing_if = "ComparisonOperator::is_equal")]
    pub comparison: ComparisonOperator,
    pub value: RuntimeValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundQuery {
    pub contract_version: u16,
    pub read: ReadStamp,
    pub source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join: Option<Join>,
    pub valid_at: u64,
    pub known_at_cursor: u64,
    pub source_cursor: u64,
    pub version_sources: Vec<RuntimeVersionedSource>,
    pub field_types: QueryFieldTypes,
    pub filters: Vec<BoundFilter>,
    pub projection: Projection,
    pub limit: Option<usize>,
    pub explain_contract: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub explain_analyze: bool,
    pub schema_revision: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub index_candidates: Vec<BoundIndexCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundIndexCandidate {
    pub id: ProjectionId,
    pub generation: u64,
    pub source_cursor: u64,
    pub state: ProjectionState,
    pub config_digest: String,
    pub artifact_digest: String,
    pub built_valid_at: Option<u64>,
    pub artifact_rows: Option<u64>,
    pub fields: Vec<String>,
    pub matched_prefix: usize,
    pub kind: IndexKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub predicates: Vec<BoundFilter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case")]
pub enum LogicalOperator {
    Scan {
        source: Source,
    },
    Join {
        source: Source,
        left_field: String,
        right_field: String,
    },
    Temporal {
        valid_at: u64,
        known_at_cursor: u64,
    },
    Filter {
        predicates: Vec<BoundFilter>,
    },
    Project {
        projection: Projection,
    },
    Limit {
        rows: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalPlan {
    pub contract_version: u16,
    pub read: ReadStamp,
    pub schema_revision: u64,
    pub source_cursor: u64,
    pub version_sources: Vec<RuntimeVersionedSource>,
    pub field_types: QueryFieldTypes,
    pub operators: Vec<LogicalOperator>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub explain_analyze: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case")]
pub enum PhysicalOperator {
    VersionedRead {
        sources: Vec<RuntimeVersionedSource>,
        exact: bool,
        stable_order: String,
    },
    /// Reads `sources` (each traversal's start identity plus any joined
    /// family), then expands every bounded traversal one breadth-first level
    /// at a time through temporal adjacency ranges and the identity
    /// histories of the relations they name.
    AdjacencyTraversal {
        sources: Vec<RuntimeVersionedSource>,
        exact: bool,
        stable_order: String,
    },
    MaterializedIndex {
        id: ProjectionId,
        kind: IndexKind,
        generation: u64,
        source_cursor: u64,
        schema_revision: u64,
        valid_at: u64,
        config_digest: String,
        artifact_digest: String,
        artifact_rows: u64,
        exact: bool,
        stable_order: String,
    },
    DataFusionEvaluate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePath {
    pub name: String,
    pub selected: bool,
    pub exact: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContract {
    pub read_manifest: String,
    pub scope: String,
    pub valid_at: u64,
    pub known_at_cursor: u64,
    pub source_cursor: u64,
    pub schema_revision: u64,
    pub exact: bool,
    pub deterministic_order: String,
    pub read_authentication: String,
    pub read_budget_unit: String,
    pub network_required: bool,
    pub gpu_required: bool,
    pub authorization_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanExplanation {
    pub contract: ExecutionContract,
    pub candidates: Vec<CandidatePath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalPlan {
    pub logical: LogicalPlan,
    pub operators: Vec<PhysicalOperator>,
    pub explanation: PlanExplanation,
    pub digest: String,
}

pub fn bind(query: &Query, parameters: &Parameters, catalog: &Catalog) -> Result<BoundQuery> {
    query
        .validate()
        .map_err(|error| Error::Binding(error.to_string()))?;
    let valid_at = resolve_u64_time(&query.temporal.valid_at, parameters, "valid time")?;
    let known_at_cursor = match &query.temporal.known_at {
        CursorExpr::Head => catalog.read.commit_cursor,
        CursorExpr::Literal(value) => *value,
        CursorExpr::Parameter(name) => parameter_u64(parameters, name, "known cursor")?,
    };
    if known_at_cursor > catalog.read.commit_cursor {
        return Err(Error::Binding(format!(
            "known cursor {known_at_cursor} exceeds captured head {}",
            catalog.read.commit_cursor
        )));
    }
    let schema = catalog.schema_at(known_at_cursor).ok_or_else(|| {
        Error::Binding(format!(
            "no schema was visible at known cursor {known_at_cursor}"
        ))
    })?;
    let source_fields = fields_for_source(&query.source, schema)?;
    let fields = if let Some(join) = &query.join {
        let right_fields = fields_for_source(&join.source, schema)?;
        let left_types = ensure_field(&join.left_field, &source_fields)?;
        let right_types = ensure_field(&join.right_field, &right_fields)?;
        if !left_types.iter().any(|left| {
            *left != RuntimeValueType::Null && right_types.iter().any(|right| left == right)
        }) {
            return Err(Error::Binding(format!(
                "join fields {:?} and {:?} have incompatible types",
                join.left_field, join.right_field
            )));
        }
        joined_field_catalog(&source_fields, &right_fields)
    } else {
        source_fields
    };
    for filter in &query.filters {
        let accepted = ensure_field(&filter.field, &fields)?;
        if let ValueExpr::Literal(value) = &filter.value {
            ensure_value_type(&filter.field, value, accepted)?;
        }
    }
    if let Projection::Fields(projected) = &query.projection {
        for field in projected {
            let _ = ensure_field(field, &fields)?;
        }
    }
    let filters = query
        .filters
        .iter()
        .map(|filter| {
            let value = match &filter.value {
                ValueExpr::Literal(value) => value.clone(),
                ValueExpr::Parameter(name) => parameters
                    .get(name)
                    .cloned()
                    .ok_or_else(|| Error::Binding(format!("missing query parameter ${name}")))?,
            };
            ensure_value_type(
                &filter.field,
                &value,
                fields
                    .get(&filter.field)
                    .expect("filter field was checked before parameter resolution"),
            )?;
            ensure_comparison(&filter.field, filter.comparison, &value)?;
            Ok(BoundFilter {
                field: filter.field.clone(),
                comparison: filter.comparison,
                value,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if query.join.is_some()
        && query
            .filters
            .iter()
            .any(|filter| filter.comparison.is_match())
    {
        return Err(Error::Binding(
            "MATCH scoring is not defined over joined row identities".into(),
        ));
    }
    let filter_fields = query
        .filters
        .iter()
        .map(|filter| filter.field.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let index_candidates = if query.join.is_none() {
        catalog
            .indexes
            .entries
            .values()
            .filter(|entry| entry.definition.source == query.source)
            .filter(|entry| entry.maintenance.is_some())
            .filter_map(|entry| {
                let matched_prefix = entry
                    .definition
                    .fields
                    .iter()
                    .take_while(|field| filter_fields.contains(field.as_str()))
                    .count();
                let predicates = entry
                    .definition
                    .filters
                    .iter()
                    .map(|filter| match &filter.value {
                        ValueExpr::Literal(value) => Some(BoundFilter {
                            field: filter.field.clone(),
                            comparison: filter.comparison,
                            value: value.clone(),
                        }),
                        ValueExpr::Parameter(_) => None,
                    })
                    .collect::<Option<Vec<_>>>()?;
                (matched_prefix > 0).then(|| BoundIndexCandidate {
                    id: entry.definition.id.clone(),
                    generation: entry.stamp.generation,
                    source_cursor: entry.stamp.source_cursor,
                    state: entry.stamp.state,
                    config_digest: entry.stamp.config_digest.clone(),
                    artifact_digest: entry.stamp.artifact_digest.clone(),
                    built_valid_at: entry.built_valid_at,
                    artifact_rows: entry.artifact_rows,
                    fields: entry.definition.fields.clone(),
                    matched_prefix,
                    kind: entry.definition.kind.clone(),
                    predicates,
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let source_cursor = catalog
        .source_watermarks
        .for_source_at(&query.source, known_at_cursor);
    let source_cursor = query.join.as_ref().map_or(source_cursor, |join| {
        source_cursor.max(
            catalog
                .source_watermarks
                .for_source_at(&join.source, known_at_cursor),
        )
    });
    let mut queried_sources = vec![query.source.clone()];
    if let Some(join) = &query.join {
        queried_sources.push(join.source.clone());
    }
    let required_catalogue_sources = version_sources_for_sources(&queried_sources);
    catalog.require_version_sources(&required_catalogue_sources)?;
    let version_sources = bound_version_sources(query, &filters, schema)?;
    Ok(BoundQuery {
        contract_version: QUERY_CONTRACT_VERSION,
        read: catalog.read.clone(),
        source: query.source.clone(),
        join: query.join.clone(),
        valid_at,
        known_at_cursor,
        source_cursor,
        version_sources,
        field_types: fields,
        filters,
        projection: query.projection.clone(),
        limit: query.limit,
        explain_contract: query.explain_contract,
        explain_analyze: query.explain_analyze,
        schema_revision: schema.revision,
        index_candidates,
    })
}

pub fn plan(bound: &BoundQuery) -> Result<PhysicalPlan> {
    let mut operators = vec![
        LogicalOperator::Scan {
            source: bound.source.clone(),
        },
        LogicalOperator::Temporal {
            valid_at: bound.valid_at,
            known_at_cursor: bound.known_at_cursor,
        },
    ];
    if let Some(join) = &bound.join {
        operators.push(LogicalOperator::Join {
            source: join.source.clone(),
            left_field: join.left_field.clone(),
            right_field: join.right_field.clone(),
        });
    }
    if !bound.filters.is_empty() {
        operators.push(LogicalOperator::Filter {
            predicates: bound.filters.clone(),
        });
    }
    operators.push(LogicalOperator::Project {
        projection: bound.projection.clone(),
    });
    if let Some(rows) = bound.limit {
        operators.push(LogicalOperator::Limit { rows });
    }
    let logical = LogicalPlan {
        contract_version: bound.contract_version,
        read: bound.read.clone(),
        schema_revision: bound.schema_revision,
        source_cursor: bound.source_cursor,
        version_sources: bound.version_sources.clone(),
        field_types: bound.field_types.clone(),
        operators,
        explain_analyze: bound.explain_analyze,
    };
    let event_cursor = event_cursor_filter(bound);
    let traversal = std::iter::once(&bound.source)
        .chain(bound.join.as_ref().map(|join| &join.source))
        .any(|source| matches!(source, Source::Traversal { .. }));
    let match_field = bound
        .filters
        .iter()
        .find(|filter| filter.comparison.is_match())
        .map(|filter| filter.field.as_str());
    let selected_index = (event_cursor.is_none() && !traversal)
        .then(|| {
            bound
                .index_candidates
                .iter()
                .filter(|candidate| {
                    candidate.state == ProjectionState::Ready
                        && candidate.source_cursor == bound.source_cursor
                        && candidate.source_cursor <= bound.known_at_cursor
                        && candidate.built_valid_at == Some(bound.valid_at)
                        && candidate.artifact_rows.is_some()
                        && match (&candidate.kind, match_field) {
                            (IndexKind::Scalar | IndexKind::Geo, None) => {
                                candidate.predicates.is_empty()
                            }
                            (IndexKind::MaterializedView, None) => {
                                candidate.predicates == bound.filters
                            }
                            (IndexKind::Bm25 { .. }, Some(field)) => {
                                candidate.fields.as_slice() == [field]
                            }
                            _ => false,
                        }
                })
                .max_by(|left, right| {
                    left.matched_prefix
                        .cmp(&right.matched_prefix)
                        .then_with(|| right.id.cmp(&left.id))
                })
        })
        .flatten();
    let mut candidates = if traversal {
        vec![
            CandidatePath {
                name: "adjacency_traversal".into(),
                selected: true,
                exact: true,
                reason: "reads each start identity, then per breadth-first level the temporal adjacency of the frontier and the identity history of every relation it names".into(),
            },
            CandidatePath {
                name: "versioned_source_read".into(),
                selected: false,
                exact: true,
                reason: "rejected: relation and record family ranges grow with the estate, not with the visited neighbourhood".into(),
            },
            CandidatePath {
                name: "derived_projection".into(),
                selected: false,
                exact: false,
                reason: "rejected: no projection generation and grounding proof supplied".into(),
            },
        ]
    } else if let Some(cursor) = event_cursor {
        vec![
            CandidatePath {
                name: "versioned_identity_read".into(),
                selected: selected_index.is_none(),
                exact: true,
                reason: format!(
                    "selects the typed event identity for bound global cursor {cursor} at the captured read stamp"
                ),
            },
            CandidatePath {
                name: "versioned_source_read".into(),
                selected: false,
                exact: true,
                reason: "rejected: the typed identity range is narrower than the event-family range".into(),
            },
            CandidatePath {
                name: "derived_projection".into(),
                selected: false,
                exact: false,
                reason: "rejected: no projection generation and grounding proof supplied".into(),
            },
        ]
    } else {
        vec![
            CandidatePath {
                name: "versioned_source_read".into(),
                selected: selected_index.is_none(),
                exact: true,
                reason: if selected_index.is_none() {
                    "selects only the typed semantic version ranges required by the bound sources"
                        .into()
                } else {
                    "rejected: a verified exact scalar index requests fewer materialized rows"
                        .into()
                },
            },
            CandidatePath {
                name: "derived_projection".into(),
                selected: false,
                exact: false,
                reason: "rejected: no projection generation and grounding proof supplied".into(),
            },
        ]
    };
    candidates.extend(bound.index_candidates.iter().map(|index| {
        let state = format!("{:?}", index.state).to_ascii_lowercase();
        let freshness = if index.source_cursor == bound.source_cursor {
            "fresh"
        } else if index.source_cursor > bound.known_at_cursor {
            "newer-than-requested"
        } else {
            "stale"
        };
        let selected = selected_index.is_some_and(|selected| selected.id == index.id);
        let time = index
            .built_valid_at
            .map_or_else(|| "unbuilt".into(), |value| format!("valid-time {value}"));
        CandidatePath {
            name: format!("index:{}", index.id),
            selected,
            exact: selected,
            reason: if selected {
                format!(
                    "selected: verified generation {} is ready and exact at cursor {} and valid-time {}; matched prefix {}/{}",
                    index.generation,
                    index.source_cursor,
                    bound.valid_at,
                    index.matched_prefix,
                    index.fields.len()
                )
            } else {
                format!(
                    "rejected: generation {} is {state}, {freshness} at cursor {}, and {time}; matched prefix {}/{}",
                    index.generation,
                    index.source_cursor,
                    index.matched_prefix,
                    index.fields.len()
                )
            },
        }
    }));
    let explanation = PlanExplanation {
        contract: ExecutionContract {
            read_manifest: bound.read.manifest_id.clone(),
            scope: bound.read.scope.to_string(),
            valid_at: bound.valid_at,
            known_at_cursor: bound.known_at_cursor,
            source_cursor: bound.source_cursor,
            schema_revision: bound.schema_revision,
            exact: true,
            deterministic_order: if match_field.is_some() {
                "bm25_score_desc_identity_ascending".into()
            } else {
                "identity_ascending".into()
            },
            read_authentication: "rfc9162_authenticated_semantic_versions".into(),
            read_budget_unit: "physical_keys_examined".into(),
            network_required: false,
            gpu_required: false,
            authorization_boundary: format!("scope:{}", bound.read.scope),
        },
        candidates,
    };
    let access = if let Some(index) = selected_index {
        PhysicalOperator::MaterializedIndex {
            id: index.id.clone(),
            kind: index.kind.clone(),
            generation: index.generation,
            source_cursor: index.source_cursor,
            schema_revision: bound.schema_revision,
            valid_at: bound.valid_at,
            config_digest: index.config_digest.clone(),
            artifact_digest: index.artifact_digest.clone(),
            artifact_rows: index
                .artifact_rows
                .expect("selected indexes have a row count"),
            exact: true,
            stable_order: if matches!(index.kind, IndexKind::Bm25 { .. }) {
                "bm25_score_desc_identity_ascending".into()
            } else {
                "identity_ascending".into()
            },
        }
    } else if traversal {
        PhysicalOperator::AdjacencyTraversal {
            sources: bound.version_sources.clone(),
            exact: true,
            stable_order: "global_cursor".into(),
        }
    } else {
        PhysicalOperator::VersionedRead {
            sources: bound.version_sources.clone(),
            exact: true,
            stable_order: "global_cursor".into(),
        }
    };
    let physical_operators = vec![access, PhysicalOperator::DataFusionEvaluate];
    let digest = plan_digest(&logical, &physical_operators, &explanation)?;
    Ok(PhysicalPlan {
        logical,
        operators: physical_operators,
        explanation,
        digest,
    })
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn event_cursor_filter(bound: &BoundQuery) -> Option<u64> {
    if !matches!(&bound.source, Source::Event { .. }) {
        return None;
    }
    bound.filters.iter().find_map(|filter| {
        (filter.field == "cursor" && filter.comparison == ComparisonOperator::Equal)
            .then_some(&filter.value)
            .and_then(|value| match value {
                RuntimeValue::Unsigned(cursor) => Some(*cursor),
                _ => None,
            })
    })
}

fn bound_version_sources(
    query: &Query,
    filters: &[BoundFilter],
    schema: &RuntimeSchemaRegistry,
) -> Result<Vec<RuntimeVersionedSource>> {
    if query.join.is_none() {
        if let Source::Event { kind } = &query.source {
            if let Some(cursor) = filters.iter().find_map(|filter| {
                (filter.field == "cursor" && filter.comparison == ComparisonOperator::Equal)
                    .then_some(&filter.value)
                    .and_then(|value| match value {
                        RuntimeValue::Unsigned(cursor) => Some(*cursor),
                        _ => None,
                    })
            }) {
                let model = schema
                    .logical_model(kind)
                    .map_err(|error| Error::Binding(error.to_string()))?;
                if !model.is_event_like() {
                    return Err(Error::Binding(format!(
                        "event source {kind} resolves to non-event model {model:?}"
                    )));
                }
                return Ok(vec![RuntimeVersionedSource::Identity {
                    model,
                    reference: rrd_core::RuntimeRef::new(
                        kind.to_string(),
                        format!("cursor:{cursor}"),
                    )
                    .map_err(|error| Error::Binding(error.to_string()))?,
                }]);
            }
        }
    }
    let mut sources = Vec::new();
    for source in std::iter::once(&query.source).chain(query.join.as_ref().map(|join| &join.source))
    {
        match source {
            Source::Traversal { start, .. } => {
                let model = schema
                    .logical_model(&start.kind)
                    .map_err(|error| Error::Binding(error.to_string()))?;
                if !model.is_record_like() {
                    return Err(Error::Binding(format!(
                        "traversal start type {} resolves to non-record model {model:?}",
                        start.kind
                    )));
                }
                sources.push(RuntimeVersionedSource::Identity {
                    model,
                    reference: start.clone(),
                });
            }
            source => sources.extend(version_sources_for_sources(std::slice::from_ref(source))),
        }
    }
    Ok(normalize_version_sources(sources))
}

impl PhysicalPlan {
    pub fn verify(&self) -> Result<()> {
        if self.logical.contract_version != QUERY_CONTRACT_VERSION {
            return Err(Error::Integrity(format!(
                "unsupported query contract version {}",
                self.logical.contract_version
            )));
        }
        self.logical
            .read
            .validate()
            .map_err(|error| Error::Integrity(error.to_string()))?;
        let expected = plan_digest(&self.logical, &self.operators, &self.explanation)?;
        if self.digest != expected {
            return Err(Error::Integrity(
                "physical plan digest does not match its contract".into(),
            ));
        }
        if !self.explanation.contract.exact {
            return Err(Error::Integrity(
                "DataFusion executor requires an exact plan".into(),
            ));
        }
        Ok(())
    }
}

fn plan_digest(
    logical: &LogicalPlan,
    physical: &[PhysicalOperator],
    explanation: &PlanExplanation,
) -> Result<String> {
    let bytes = serde_json::to_vec(&(logical, physical, explanation))?;
    Ok(digest::sha256_hex(&bytes))
}

fn resolve_u64_time(value: &TimeExpr, parameters: &Parameters, label: &str) -> Result<u64> {
    match value {
        TimeExpr::Literal(value) => Ok(*value),
        TimeExpr::Parameter(name) => parameter_u64(parameters, name, label),
    }
}

fn parameter_u64(parameters: &Parameters, name: &str, label: &str) -> Result<u64> {
    match parameters.get(name) {
        Some(RuntimeValue::Unsigned(value)) => Ok(*value),
        Some(_) => Err(Error::Binding(format!(
            "{label} parameter ${name} must be unsigned"
        ))),
        None => Err(Error::Binding(format!("missing query parameter ${name}"))),
    }
}

fn ensure_field<'a>(field: &str, fields: &'a FieldCatalog) -> Result<&'a [RuntimeValueType]> {
    fields.get(field).map(Vec::as_slice).ok_or_else(|| {
        Error::Binding(format!(
            "field {field:?} is not present in the selected source"
        ))
    })
}

fn joined_field_catalog(left: &FieldCatalog, right: &FieldCatalog) -> FieldCatalog {
    left.iter()
        .map(|(field, types)| (format!("left.{field}"), types.clone()))
        .chain(
            right
                .iter()
                .map(|(field, types)| (format!("right.{field}"), types.clone())),
        )
        .collect()
}

fn ensure_value_type(
    field: &str,
    value: &RuntimeValue,
    accepted: &[RuntimeValueType],
) -> Result<()> {
    if accepted.iter().any(|expected| expected.matches(value)) {
        Ok(())
    } else {
        Err(Error::Binding(format!(
            "filter value for {field:?} does not match accepted types {accepted:?}"
        )))
    }
}

fn ensure_comparison(
    field: &str,
    comparison: ComparisonOperator,
    value: &RuntimeValue,
) -> Result<()> {
    if comparison.is_match() && !matches!(value, RuntimeValue::String(_)) {
        return Err(Error::Binding(format!(
            "MATCH requires a string query for {field:?}"
        )));
    }
    if !comparison.is_ordering()
        || matches!(
            value,
            RuntimeValue::Integer(_) | RuntimeValue::Unsigned(_) | RuntimeValue::String(_)
        )
    {
        Ok(())
    } else {
        Err(Error::Binding(format!(
            "ordering comparison {} is not supported for the value type of {field:?}",
            comparison_label(comparison)
        )))
    }
}

fn comparison_label(comparison: ComparisonOperator) -> &'static str {
    match comparison {
        ComparisonOperator::Equal => "=",
        ComparisonOperator::NotEqual => "!=",
        ComparisonOperator::LessThan => "<",
        ComparisonOperator::LessThanOrEqual => "<=",
        ComparisonOperator::GreaterThan => ">",
        ComparisonOperator::GreaterThanOrEqual => ">=",
        ComparisonOperator::Match => "MATCH",
    }
}

pub(crate) fn fields_for_source(
    source: &Source,
    schema: &RuntimeSchemaRegistry,
) -> Result<FieldCatalog> {
    let (builtins, properties): (BuiltinFields, Vec<(String, RuntimeValueType)>) = match source {
        Source::Record { kind } => {
            let definition = schema.records.get(kind).ok_or_else(|| {
                Error::Binding(format!(
                    "record type {kind} is not registered at this cursor"
                ))
            })?;
            (
                &[
                    ("id", &[RuntimeValueType::String]),
                    ("kind", &[RuntimeValueType::String]),
                    ("valid_from", &[RuntimeValueType::Unsigned]),
                    (
                        "valid_to",
                        &[RuntimeValueType::Null, RuntimeValueType::Unsigned],
                    ),
                ],
                definition
                    .properties
                    .iter()
                    .map(|(name, rule)| (name.clone(), rule.value_type))
                    .collect(),
            )
        }
        Source::Relation { kind } => {
            let definition = schema.relations.get(kind).ok_or_else(|| {
                Error::Binding(format!(
                    "relation type {kind} is not registered at this cursor"
                ))
            })?;
            (
                &[
                    ("id", &[RuntimeValueType::String]),
                    ("kind", &[RuntimeValueType::String]),
                    ("from_kind", &[RuntimeValueType::String]),
                    ("from_id", &[RuntimeValueType::String]),
                    ("to_kind", &[RuntimeValueType::String]),
                    ("to_id", &[RuntimeValueType::String]),
                    ("valid_from", &[RuntimeValueType::Unsigned]),
                    (
                        "valid_to",
                        &[RuntimeValueType::Null, RuntimeValueType::Unsigned],
                    ),
                ],
                definition
                    .properties
                    .iter()
                    .map(|(name, rule)| (name.clone(), rule.value_type))
                    .collect(),
            )
        }
        Source::Event { kind } => {
            let definition = schema.events.get(kind).ok_or_else(|| {
                Error::Binding(format!(
                    "event type {kind} is not registered at this cursor"
                ))
            })?;
            (
                &[
                    ("cursor", &[RuntimeValueType::Unsigned]),
                    ("kind", &[RuntimeValueType::String]),
                    (
                        "subject_kind",
                        &[RuntimeValueType::Null, RuntimeValueType::String],
                    ),
                    (
                        "subject_id",
                        &[RuntimeValueType::Null, RuntimeValueType::String],
                    ),
                    ("at", &[RuntimeValueType::Unsigned]),
                    ("actor", &[RuntimeValueType::String]),
                ],
                definition
                    .properties
                    .iter()
                    .map(|(name, rule)| (name.clone(), rule.value_type))
                    .collect(),
            )
        }
        Source::Series { .. } => (
            &[
                ("id", &[RuntimeValueType::String]),
                ("kind", &[RuntimeValueType::String]),
                ("series_kind", &[RuntimeValueType::String]),
                ("series_id", &[RuntimeValueType::String]),
                ("observed_at", &[RuntimeValueType::Unsigned]),
                (
                    "value",
                    &[
                        RuntimeValueType::Integer,
                        RuntimeValueType::Unsigned,
                        RuntimeValueType::Decimal,
                        RuntimeValueType::Bool,
                        RuntimeValueType::String,
                    ],
                ),
            ],
            Vec::new(),
        ),
        Source::Geo { .. } => (
            &[
                ("id", &[RuntimeValueType::String]),
                ("kind", &[RuntimeValueType::String]),
                ("subject_kind", &[RuntimeValueType::String]),
                ("subject_id", &[RuntimeValueType::String]),
                ("field", &[RuntimeValueType::String]),
                ("valid_from", &[RuntimeValueType::Unsigned]),
                (
                    "valid_to",
                    &[RuntimeValueType::Null, RuntimeValueType::Unsigned],
                ),
                ("geometry_kind", &[RuntimeValueType::String]),
                (
                    "longitude",
                    &[RuntimeValueType::Null, RuntimeValueType::Decimal],
                ),
                (
                    "latitude",
                    &[RuntimeValueType::Null, RuntimeValueType::Decimal],
                ),
                (
                    "southwest_longitude",
                    &[RuntimeValueType::Null, RuntimeValueType::Decimal],
                ),
                (
                    "southwest_latitude",
                    &[RuntimeValueType::Null, RuntimeValueType::Decimal],
                ),
                (
                    "northeast_longitude",
                    &[RuntimeValueType::Null, RuntimeValueType::Decimal],
                ),
                (
                    "northeast_latitude",
                    &[RuntimeValueType::Null, RuntimeValueType::Decimal],
                ),
            ],
            Vec::new(),
        ),
        Source::Traversal {
            relation, start, ..
        } => {
            let definition = schema.relations.get(relation).ok_or_else(|| {
                Error::Binding(format!(
                    "relation type {relation} is not registered at this cursor"
                ))
            })?;
            // A start is any record-like table in the catalogue; a schemaless
            // table is as much a graph node as one with a strict schema.
            let start_is_record = schema.records.contains_key(&start.kind)
                || schema
                    .tables
                    .get(&start.kind)
                    .is_some_and(|table| table.model.is_record_like());
            if !start_is_record {
                return Err(Error::Binding(format!(
                    "traversal start type {} is not registered at this cursor",
                    start.kind
                )));
            }
            if !definition.from.contains(&start.kind) && !definition.to.contains(&start.kind) {
                return Err(Error::Binding(format!(
                    "traversal start type {} is not an endpoint of relation {relation}",
                    start.kind
                )));
            }
            (
                &[
                    ("depth", &[RuntimeValueType::Unsigned]),
                    ("node_kind", &[RuntimeValueType::String]),
                    ("node_id", &[RuntimeValueType::String]),
                    ("relation_kind", &[RuntimeValueType::String]),
                    ("relation_id", &[RuntimeValueType::String]),
                    ("from_kind", &[RuntimeValueType::String]),
                    ("from_id", &[RuntimeValueType::String]),
                    ("to_kind", &[RuntimeValueType::String]),
                    ("to_id", &[RuntimeValueType::String]),
                    ("path", &[RuntimeValueType::List]),
                ],
                Vec::new(),
            )
        }
        Source::Claim { .. } => (
            &[
                ("subject", &[RuntimeValueType::String]),
                ("predicate", &[RuntimeValueType::String]),
                ("object", &[RuntimeValueType::String]),
                ("valid_from", &[RuntimeValueType::Unsigned]),
                (
                    "valid_to",
                    &[RuntimeValueType::Null, RuntimeValueType::Unsigned],
                ),
                ("tx_time", &[RuntimeValueType::Unsigned]),
                ("actor", &[RuntimeValueType::String]),
            ],
            Vec::new(),
        ),
    };
    Ok(builtins
        .iter()
        .map(|(field, accepted)| ((*field).to_owned(), accepted.to_vec()))
        .chain(
            properties
                .into_iter()
                .map(|(field, accepted)| (field, vec![accepted])),
        )
        .collect())
}

pub(crate) fn field_types_for_bound_source(
    source: &Source,
    catalog: &Catalog,
    field: &str,
) -> Result<Vec<RuntimeValueType>> {
    let schema = catalog
        .schema_at(catalog.read.commit_cursor)
        .ok_or_else(|| Error::Binding("no schema is visible at the catalogue head".into()))?;
    fields_for_source(source, schema)?
        .remove(field)
        .ok_or_else(|| Error::Binding(format!("field {field:?} is not present in the source")))
}
