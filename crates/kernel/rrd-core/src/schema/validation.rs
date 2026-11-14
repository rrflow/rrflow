//! Mutation families, logical-model rules and table and property validation.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RuntimeMutationFamily {
    Record,
    Relation,
    Event,
    Vector,
    Series,
    Geo,
    Object,
    Claim,
}

impl RuntimeLogicalModel {
    pub const fn is_record_like(self) -> bool {
        matches!(
            self,
            Self::Document
                | Self::Relational
                | Self::GraphNode
                | Self::KeyValue
                | Self::ReasoningRecord
                | Self::LifecycleRecord
        )
    }

    pub const fn is_event_like(self) -> bool {
        matches!(
            self,
            Self::Event | Self::ReasoningEvent | Self::LifecycleEvent
        )
    }

    pub(super) fn family(self) -> RuntimeMutationFamily {
        match self {
            Self::Document
            | Self::Relational
            | Self::GraphNode
            | Self::KeyValue
            | Self::ReasoningRecord
            | Self::LifecycleRecord => RuntimeMutationFamily::Record,
            Self::GraphRelation => RuntimeMutationFamily::Relation,
            Self::Event | Self::ReasoningEvent | Self::LifecycleEvent => {
                RuntimeMutationFamily::Event
            }
            Self::Vector => RuntimeMutationFamily::Vector,
            Self::TimeSeries => RuntimeMutationFamily::Series,
            Self::Geo => RuntimeMutationFamily::Geo,
            Self::Object => RuntimeMutationFamily::Object,
            Self::ReasoningClaim => RuntimeMutationFamily::Claim,
        }
    }
}

impl RuntimeSchemaRegistry {
    pub fn logical_model(&self, kind: &RuntimeType) -> Result<RuntimeLogicalModel> {
        self.catalogue_tables()?
            .get(kind)
            .map(|table| table.model)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: format!("table type {kind} is not registered in the catalogue"),
            })
    }
}

pub(super) fn validate_specialized_table(
    tables: &BTreeMap<RuntimeType, RuntimeTableSchema>,
    kind: &RuntimeType,
    family: RuntimeMutationFamily,
) -> Result<()> {
    let table = require_table(kind, family, tables)?;
    if table.mode != RuntimeSchemaMode::Strict {
        return invalid(format!(
            "table type {kind} is schemaless but also has a strict {family:?} schema"
        ));
    }
    Ok(())
}

pub(super) fn validate_table(kind: &RuntimeType, table: &RuntimeTableSchema) -> Result<()> {
    validate_property_schema(kind, &table.properties, table.allow_additional_properties)?;
    if table.mode == RuntimeSchemaMode::Schemaless
        && (!table.properties.is_empty() || table.allow_additional_properties)
    {
        return invalid(format!(
            "schemaless table type {kind} cannot also declare a strict property contract"
        ));
    }
    if matches!(
        table.model.family(),
        RuntimeMutationFamily::Record
            | RuntimeMutationFamily::Relation
            | RuntimeMutationFamily::Event
    ) && (!table.properties.is_empty() || table.allow_additional_properties)
    {
        return invalid(format!(
            "table type {kind} must keep record/relation/event properties in its specialized schema"
        ));
    }
    Ok(())
}

pub(super) fn require_table<'a>(
    kind: &RuntimeType,
    family: RuntimeMutationFamily,
    tables: &'a BTreeMap<RuntimeType, RuntimeTableSchema>,
) -> Result<&'a RuntimeTableSchema> {
    let table = tables.get(kind).ok_or_else(|| Error::InvalidRuntime {
        reason: format!("{family:?} table type {kind} is not registered in the catalogue"),
    })?;
    if table.model.family() != family {
        return invalid(format!(
            "table type {kind} is catalogued as {:?}, not {family:?}",
            table.model
        ));
    }
    Ok(table)
}

pub(super) fn validate_structural_table(
    object: &str,
    kind: &RuntimeType,
    properties: &RuntimeProperties,
    family: RuntimeMutationFamily,
    tables: &BTreeMap<RuntimeType, RuntimeTableSchema>,
) -> Result<()> {
    let table = require_table(kind, family, tables)?;
    if table.mode == RuntimeSchemaMode::Schemaless {
        return Ok(());
    }
    validate_properties(
        object,
        kind,
        properties,
        &table.properties,
        table.allow_additional_properties,
    )
}

pub(super) fn validate_property_schema(
    kind: &RuntimeType,
    properties: &BTreeMap<String, RuntimePropertySchema>,
    _allow_additional: bool,
) -> Result<()> {
    for name in properties.keys() {
        if name.is_empty() || name.as_bytes().contains(&0) {
            return invalid(format!(
                "type {kind} contains invalid property name {name:?}"
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_properties(
    object: &str,
    kind: &RuntimeType,
    actual: &RuntimeProperties,
    declared: &BTreeMap<String, RuntimePropertySchema>,
    allow_additional: bool,
) -> Result<()> {
    for (name, property) in declared {
        match actual.get(name) {
            Some(value) if property.value_type.matches(value) => {}
            Some(_) => {
                return invalid(format!(
                    "{object} type {kind} property {name:?} has the wrong value type"
                ));
            }
            None if property.required => {
                return invalid(format!(
                    "{object} type {kind} is missing required property {name:?}"
                ));
            }
            None => {}
        }
    }
    if !allow_additional {
        if let Some(name) = actual.keys().find(|name| !declared.contains_key(*name)) {
            return invalid(format!(
                "{object} type {kind} contains undeclared property {name:?}"
            ));
        }
    }
    Ok(())
}

pub(super) fn max_overlap(relations: &[&RuntimeRelation]) -> u64 {
    let mut points = Vec::with_capacity(relations.len() * 2);
    for relation in relations {
        points.push((relation.valid_from, 1_i8));
        if let Some(end) = relation.valid_to {
            points.push((end, -1_i8));
        }
    }
    // Half-open intervals: ends are processed before starts at the same time.
    points.sort_by_key(|(at, delta)| (*at, *delta));
    let mut current = 0_i64;
    let mut maximum = 0_i64;
    for (_, delta) in points {
        current += i64::from(delta);
        maximum = maximum.max(current);
    }
    maximum as u64
}

pub(super) fn windows_overlap(
    left_from: u64,
    left_to: Option<u64>,
    right_from: u64,
    right_to: Option<u64>,
) -> bool {
    left_from < right_to.unwrap_or(u64::MAX) && right_from < left_to.unwrap_or(u64::MAX)
}

pub(super) fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::InvalidRuntime {
        reason: reason.into(),
    })
}
