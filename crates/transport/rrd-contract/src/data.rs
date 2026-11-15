//! Data values, schemas, organisation and snapshots.

use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type DataValue = QueryValue;
pub type DataProperties = BTreeMap<String, DataValue>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataReference {
    pub kind: CanonicalId,
    pub id: CanonicalId,
}

/// Stable target identity for CRUD and retirement. Append-only events are
/// addressed by their authenticated-log cursor instead of a fabricated user
/// identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "identity", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataTarget {
    Reference { reference: DataReference },
    Event { kind: CanonicalId, cursor: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataValueType {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataPropertySchema {
    pub value_type: DataValueType,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRecordSchema {
    #[serde(default)]
    pub properties: BTreeMap<String, DataPropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
    #[serde(default)]
    pub unique_properties: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRelationSchema {
    #[serde(default)]
    pub from: BTreeSet<CanonicalId>,
    #[serde(default)]
    pub to: BTreeSet<CanonicalId>,
    #[serde(default)]
    pub properties: BTreeMap<String, DataPropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
    #[serde(default)]
    pub unique_pair: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_outgoing: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_incoming: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataEventSchema {
    #[serde(default)]
    pub subject_required: bool,
    #[serde(default)]
    pub subject_types: BTreeSet<CanonicalId>,
    #[serde(default)]
    pub properties: BTreeMap<String, DataPropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataLogicalModel {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataSchemaMode {
    Strict,
    Schemaless,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataTableSchema {
    pub model: DataLogicalModel,
    pub mode: DataSchemaMode,
    #[serde(default)]
    pub properties: BTreeMap<String, DataPropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
}

/// Built-in domain holding every kind and collection not placed elsewhere.
pub const DATA_DEFAULT_DOMAIN: &str = "default";
/// Built-in domain holding the engine-owned kinds and collections.
pub const DATA_ENGINE_DOMAIN: &str = "rrflow";
/// Identity prefix that marks a kind or collection as engine-owned.
pub const DATA_ENGINE_OWNED_PREFIX: &str = "rrflow-";
pub const MAX_DATA_DOMAINS: usize = 256;
pub const MAX_DATA_DOMAIN_DEPTH: usize = 8;
pub const MAX_DATA_CATEGORIES: usize = 64;
pub const MAX_DATA_TAGS_PER_CATEGORY: usize = 4_096;
pub const MAX_DATA_TAGS_PER_KIND: usize = 64;
pub const MAX_DATA_ORGANISATION_TITLE_BYTES: usize = 128;
pub const MAX_DATA_ORGANISATION_DESCRIPTION_BYTES: usize = 1_024;

/// One configurable part of an engram's catalogue, such as a service, package
/// or subject area. The engram organisation contract owns its rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataDomainSchema {
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Omitted, or `default`, for a top-level domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<CanonicalId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataCategoryTargets {
    Kinds,
    Records,
    KindsAndRecords,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataTagCardinality {
    Single,
    Multiple,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataTagSchema {
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// A governed vocabulary of tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataCategorySchema {
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub targets: DataCategoryTargets,
    pub cardinality: DataTagCardinality,
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub tags: BTreeMap<CanonicalId, DataTagSchema>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct DataTagReference {
    pub category: CanonicalId,
    pub tag: CanonicalId,
}

/// Domains, categories and memberships of one engram catalogue. A kind or
/// collection without an entry belongs to `default`, or to `rrflow` when its
/// identity starts with `rrflow-`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataEngramOrganisation {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub domains: BTreeMap<CanonicalId, DataDomainSchema>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub categories: BTreeMap<CanonicalId, DataCategorySchema>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub kind_domains: BTreeMap<CanonicalId, CanonicalId>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub collection_domains: BTreeMap<CanonicalId, CanonicalId>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub kind_tags: BTreeMap<CanonicalId, BTreeSet<DataTagReference>>,
}

impl DataEngramOrganisation {
    pub fn is_default(&self) -> bool {
        self.domains.is_empty()
            && self.categories.is_empty()
            && self.kind_domains.is_empty()
            && self.collection_domains.is_empty()
            && self.kind_tags.is_empty()
    }

    /// The one domain a kind belongs to.
    pub fn domain_of_kind<'a>(&'a self, kind: &'a CanonicalId) -> &'a str {
        self.kind_domains
            .get(kind)
            .map_or_else(|| default_data_domain(kind.as_str()), CanonicalId::as_str)
    }

    /// The one domain a vector collection belongs to.
    pub fn domain_of_collection<'a>(&'a self, collection: &'a CanonicalId) -> &'a str {
        self.collection_domains.get(collection).map_or_else(
            || default_data_domain(collection.as_str()),
            CanonicalId::as_str,
        )
    }
}

pub(crate) fn default_data_domain(identity: &str) -> &'static str {
    if identity.starts_with(DATA_ENGINE_OWNED_PREFIX) {
        DATA_ENGINE_DOMAIN
    } else {
        DATA_DEFAULT_DOMAIN
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataSchemaRegistry {
    pub revision: u64,
    pub migration: String,
    #[serde(default, skip_serializing_if = "DataEngramOrganisation::is_default")]
    pub organisation: DataEngramOrganisation,
    pub tables: BTreeMap<CanonicalId, DataTableSchema>,
    #[serde(default)]
    pub records: BTreeMap<CanonicalId, DataRecordSchema>,
    #[serde(default)]
    pub relations: BTreeMap<CanonicalId, DataRelationSchema>,
    #[serde(default)]
    pub events: BTreeMap<CanonicalId, DataEventSchema>,
}

/// Bounded, authenticated catalogue discovery for database administration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadDataSchema {
    pub scope: String,
    pub max_storage_keys: u32,
}

impl ReadDataSchema {
    pub fn validate(&self) -> Result<()> {
        if self.scope.is_empty() || self.scope.len() > MAX_ID_BYTES || self.scope.contains('\0') {
            return invalid("data schema scope is invalid");
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_DATA_SNAPSHOT_STORAGE_KEYS {
            return invalid("data schema storage key budget is out of bounds");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataSchemaSnapshot {
    pub scope: String,
    pub known_at_cursor: u64,
    pub read_manifest_sha256: String,
    pub schema: Option<DataSchemaRegistry>,
    pub read_evidence: ReadEvidence,
}

impl DataSchemaSnapshot {
    pub fn validate(&self) -> Result<()> {
        ReadDataSchema {
            scope: self.scope.clone(),
            max_storage_keys: 1,
        }
        .validate()?;
        validate_sha256(&self.read_manifest_sha256, "schema read manifest")?;
        self.read_evidence.validate()?;
        if let Some(schema) = &self.schema {
            if schema.revision == 0 {
                return invalid("schema revision must be non-zero");
            }
            TransactionMutation::PutSchema {
                registry: schema.clone(),
            }
            .validate()?;
        }
        Ok(())
    }
}

/// Bounded request for one all-model snapshot at the engine's current
/// authenticated transaction cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadDataSnapshot {
    pub valid_at: u64,
    pub max_storage_keys: u32,
}

impl ReadDataSnapshot {
    pub fn validate(&self) -> Result<()> {
        if self.valid_at == 0 {
            return invalid("data snapshot valid_at must be greater than zero");
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_DATA_SNAPSHOT_STORAGE_KEYS {
            return invalid(format!(
                "data snapshot max_storage_keys must be in 1..={MAX_DATA_SNAPSHOT_STORAGE_KEYS}"
            ));
        }
        Ok(())
    }
}

/// One current value in an all-model snapshot. `value` uses the same exact
/// typed vocabulary as writes; the wrapper supplies the catalogue model and
/// stable reference (including cursor-derived event identities).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataSnapshotEntry {
    pub model: DataLogicalModel,
    pub target: DataTarget,
    pub value: TransactionMutation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataSnapshot {
    pub scope: String,
    pub valid_at: u64,
    pub known_at_cursor: u64,
    pub schema_revision: u64,
    pub read_manifest_sha256: String,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub entries: Vec<DataSnapshotEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataVectorValue {
    Dense {
        values: Vec<f32>,
    },
    Sparse {
        dimensions: u32,
        indices: Vec<u32>,
        values: Vec<f32>,
    },
    MultiDense {
        dimensions: u32,
        vectors: Vec<Vec<f32>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataVectorNormalization {
    None,
    UnitL2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataEmbeddingProvenance {
    pub source_sha256: String,
    pub model: String,
    pub model_sha256: String,
    pub dimensions: u32,
    pub normalization: DataVectorNormalization,
    #[serde(default)]
    pub generation_parameters: DataProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum DataSeriesValue {
    Integer(i64),
    Unsigned(u64),
    Decimal(String),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataGeoPoint {
    pub longitude: f64,
    pub latitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataGeoValue {
    Point {
        point: DataGeoPoint,
    },
    BoundingBox {
        southwest: DataGeoPoint,
        northeast: DataGeoPoint,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataObjectReceipt {
    pub backend: String,
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}
