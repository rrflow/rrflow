//! Runtime value types, property, record, relation, event and table schemas.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeValueType {
    Null,
    Bool,
    Integer,
    Unsigned,
    Decimal,
    String,
    Digest,
    List,
    Map,
}

impl RuntimeValueType {
    pub fn matches(self, value: &RuntimeValue) -> bool {
        matches!(
            (self, value),
            (Self::Null, RuntimeValue::Null)
                | (Self::Bool, RuntimeValue::Bool(_))
                | (Self::Integer, RuntimeValue::Integer(_))
                | (Self::Unsigned, RuntimeValue::Unsigned(_))
                | (Self::Decimal, RuntimeValue::Decimal(_))
                | (Self::String, RuntimeValue::String(_))
                | (Self::Digest, RuntimeValue::Digest(_))
                | (Self::List, RuntimeValue::List(_))
                | (Self::Map, RuntimeValue::Map(_))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePropertySchema {
    pub value_type: RuntimeValueType,
    #[serde(default)]
    pub required: bool,
}

impl RuntimePropertySchema {
    pub const fn required(value_type: RuntimeValueType) -> Self {
        Self {
            value_type,
            required: true,
        }
    }

    pub const fn optional(value_type: RuntimeValueType) -> Self {
        Self {
            value_type,
            required: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRecordSchema {
    #[serde(default)]
    pub properties: BTreeMap<String, RuntimePropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
    #[serde(default)]
    pub unique_properties: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRelationSchema {
    #[serde(default)]
    pub from: BTreeSet<RuntimeType>,
    #[serde(default)]
    pub to: BTreeSet<RuntimeType>,
    #[serde(default)]
    pub properties: BTreeMap<String, RuntimePropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
    #[serde(default)]
    pub unique_pair: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_outgoing: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_incoming: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEventSchema {
    #[serde(default)]
    pub subject_required: bool,
    #[serde(default)]
    pub subject_types: BTreeSet<RuntimeType>,
    #[serde(default)]
    pub properties: BTreeMap<String, RuntimePropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
}

/// Logical meaning of one table identity inside a database catalogue.
///
/// Record-like models deliberately share the canonical [`RuntimeRecord`]
/// identity. Their distinct catalogue kind prevents document, relational,
/// graph, key-value, reasoning, and lifecycle APIs from creating competing
/// persisted truths for the same table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLogicalModel {
    Document,
    Relational,
    GraphNode,
    GraphRelation,
    KeyValue,
    Vector,
    Event,
    TimeSeries,
    Geo,
    Object,
    ReasoningClaim,
    ReasoningRecord,
    ReasoningEvent,
    LifecycleRecord,
    LifecycleEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSchemaMode {
    Strict,
    Schemaless,
}

/// A single table entry in the revisioned logical catalogue.
///
/// Record, relation, and event tables keep their richer constraints in the
/// existing specialized maps below. The property contract here governs the
/// structurally typed vector, time-series, geo, and object families. In
/// schemaless mode these fields must remain empty so mode cannot be inferred or
/// contradicted by a second boolean contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeTableSchema {
    pub model: RuntimeLogicalModel,
    pub mode: RuntimeSchemaMode,
    #[serde(default)]
    pub properties: BTreeMap<String, RuntimePropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
}

impl RuntimeTableSchema {
    pub fn strict(model: RuntimeLogicalModel) -> Self {
        Self {
            model,
            mode: RuntimeSchemaMode::Strict,
            properties: BTreeMap::new(),
            allow_additional_properties: false,
        }
    }

    pub fn schemaless(model: RuntimeLogicalModel) -> Self {
        Self {
            model,
            mode: RuntimeSchemaMode::Schemaless,
            properties: BTreeMap::new(),
            allow_additional_properties: false,
        }
    }
}
