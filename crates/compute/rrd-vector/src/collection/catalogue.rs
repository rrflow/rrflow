//! Collection entries, the collection catalogue, errors and mutation context.

use super::*;

pub const VECTOR_COLLECTION_CATALOGUE_VERSION: u16 = 1;
pub(super) const MAX_COLLECTIONS: usize = 4_096;
pub(super) const MAX_NAMED_VECTORS: usize = 64;
pub(super) const MAX_PAYLOAD_INDEXES: usize = 256;
pub(super) const MAX_OPERATION_RECEIPTS: usize = 100_000;

pub type CollectionResult<T> = std::result::Result<T, CollectionError>;

#[derive(Debug)]
pub enum CollectionError {
    Store(rrd_store::Error),
    Invalid(String),
    Integrity(String),
    IdempotencyConflict,
    Budget(String),
}

impl fmt::Display for CollectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(formatter, "vector collection storage failed: {error}"),
            Self::Invalid(message) => write!(formatter, "invalid vector collection: {message}"),
            Self::Integrity(message) => {
                write!(formatter, "vector collection integrity failed: {message}")
            }
            Self::IdempotencyConflict => formatter
                .write_str("vector collection idempotency key is bound to another operation"),
            Self::Budget(message) => {
                write!(formatter, "vector collection budget exceeded: {message}")
            }
        }
    }
}

impl std::error::Error for CollectionError {}

impl From<rrd_store::Error> for CollectionError {
    fn from(value: rrd_store::Error) -> Self {
        Self::Store(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorValueKind {
    Dense,
    Sparse,
    MultiDense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorMemoryTier {
    Pinned,
    Cached,
    Cold,
}

/// Exact scalar representation admitted by one collection payload index.
///
/// This deliberately follows RRD's canonical property vocabulary instead of
/// accepting transport-specific aliases that the authoritative runtime cannot
/// validate after reopen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadIndexKind {
    Boolean,
    Integer,
    Unsigned,
    Decimal,
    Keyword,
    Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadIndexDefinition {
    pub field: ProjectionId,
    pub kind: PayloadIndexKind,
}

impl PayloadIndexDefinition {
    pub fn config_digest(&self) -> CollectionResult<String> {
        let encoded = serde_json::to_vec(self)
            .map_err(|error| CollectionError::Invalid(error.to_string()))?;
        Ok(digest::sha256_hex(&encoded))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadIndexEntry {
    pub definition: PayloadIndexDefinition,
    pub generation: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub configuration_digest: String,
}

impl PayloadIndexEntry {
    pub fn validate(&self) -> CollectionResult<()> {
        if self.generation == 0 || self.created_at == 0 || self.updated_at < self.created_at {
            return integrity("payload-index generation or timestamps are invalid");
        }
        if self.configuration_digest != self.definition.config_digest()? {
            return integrity("payload-index configuration digest differs from its definition");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedVectorConfig {
    pub name: ProjectionId,
    pub field: String,
    pub kind: VectorValueKind,
    pub dimensions: u32,
    pub metric: ScoreMetric,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<EmbeddingModelBinding>,
    pub memory_tier: VectorMemoryTier,
}

impl NamedVectorConfig {
    pub fn validate(&self) -> CollectionResult<()> {
        if self.field.trim().is_empty() || self.field.as_bytes().contains(&0) {
            return invalid("named-vector field must be non-empty and contain no NUL");
        }
        if self.dimensions == 0 || self.dimensions > 1_048_576 {
            return invalid("named-vector dimensions must be in 1..=1048576");
        }
        if let Some(model) = &self.embedding_model {
            model
                .validate()
                .map_err(|error| CollectionError::Invalid(error.to_string()))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorCollectionDefinition {
    pub id: ProjectionId,
    pub vectors: BTreeMap<ProjectionId, NamedVectorConfig>,
}

impl VectorCollectionDefinition {
    pub fn validate(&self) -> CollectionResult<()> {
        if self.vectors.is_empty() || self.vectors.len() > MAX_NAMED_VECTORS {
            return invalid(format!(
                "collection must contain 1..={MAX_NAMED_VECTORS} named vectors"
            ));
        }
        let mut fields = BTreeSet::new();
        for (name, vector) in &self.vectors {
            vector.validate()?;
            if name != &vector.name {
                return invalid("named-vector map identity differs from its definition");
            }
            if !fields.insert(vector.field.as_str()) {
                return invalid("collection vector fields must be unique");
            }
        }
        Ok(())
    }

    pub fn config_digest(&self) -> CollectionResult<String> {
        self.validate()?;
        let encoded = serde_json::to_vec(self)
            .map_err(|error| CollectionError::Invalid(error.to_string()))?;
        Ok(digest::sha256_hex(&encoded))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionEntry {
    pub definition: VectorCollectionDefinition,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub payload_indexes: BTreeMap<ProjectionId, PayloadIndexEntry>,
    pub generation: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub configuration_digest: String,
}

impl CollectionEntry {
    pub fn validate(&self) -> CollectionResult<()> {
        self.definition.validate()?;
        if self.generation == 0 || self.created_at == 0 || self.updated_at < self.created_at {
            return integrity("collection generation or timestamps are invalid");
        }
        if self.configuration_digest != self.definition.config_digest()? {
            return integrity("collection configuration digest differs from its definition");
        }
        if self.payload_indexes.len() > MAX_PAYLOAD_INDEXES {
            return integrity("collection payload-index bound is exceeded");
        }
        for (field, index) in &self.payload_indexes {
            index.validate()?;
            if field != &index.definition.field {
                return integrity("payload-index map identity differs from its definition");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionOperationReceipt {
    pub operation_digest: String,
    pub entry: CollectionEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionDeletionReceipt {
    pub operation_digest: String,
    pub deleted_entry: CollectionEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadIndexOperationReceipt {
    pub operation_digest: String,
    pub collection: CollectionEntry,
    pub index: PayloadIndexEntry,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionCatalogue {
    pub contract_version: u16,
    pub scope: ScopeId,
    pub revision: u64,
    pub collections: BTreeMap<ProjectionId, CollectionEntry>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub operations: BTreeMap<String, CollectionOperationReceipt>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub deletions: BTreeMap<String, CollectionDeletionReceipt>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub payload_operations: BTreeMap<String, PayloadIndexOperationReceipt>,
}

impl CollectionCatalogue {
    pub(super) fn empty(scope: ScopeId) -> Self {
        Self {
            contract_version: VECTOR_COLLECTION_CATALOGUE_VERSION,
            scope,
            revision: 0,
            collections: BTreeMap::new(),
            operations: BTreeMap::new(),
            deletions: BTreeMap::new(),
            payload_operations: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> CollectionResult<()> {
        if self.contract_version != VECTOR_COLLECTION_CATALOGUE_VERSION {
            return integrity("unsupported vector collection catalogue version");
        }
        let operation_count = self
            .operations
            .len()
            .checked_add(self.deletions.len())
            .and_then(|count| count.checked_add(self.payload_operations.len()))
            .ok_or_else(|| CollectionError::Integrity("operation receipt count overflow".into()))?;
        if self.collections.len() > MAX_COLLECTIONS || operation_count > MAX_OPERATION_RECEIPTS {
            return integrity("vector collection catalogue bounds are exceeded");
        }
        for (id, entry) in &self.collections {
            entry.validate()?;
            if id != &entry.definition.id {
                return integrity("collection map identity differs from its definition");
            }
        }
        for (key, receipt) in &self.operations {
            if key.is_empty() || !valid_digest(&receipt.operation_digest) {
                return integrity("vector collection operation receipt is invalid");
            }
            receipt.entry.validate()?;
        }
        for (key, receipt) in &self.deletions {
            if key.is_empty() || !valid_digest(&receipt.operation_digest) {
                return integrity("vector collection deletion receipt is invalid");
            }
            receipt.deleted_entry.validate()?;
        }
        for (key, receipt) in &self.payload_operations {
            if key.is_empty() || !valid_digest(&receipt.operation_digest) {
                return integrity("payload-index operation receipt is invalid");
            }
            receipt.collection.validate()?;
            receipt.index.validate()?;
            let field = &receipt.index.definition.field;
            if receipt.deleted == receipt.collection.payload_indexes.contains_key(field) {
                return integrity("payload-index receipt outcome differs from its collection");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionMutationContext {
    pub at: u64,
    pub actor: String,
    pub request_id: String,
    pub operation_id: String,
}

impl CollectionMutationContext {
    pub(super) fn validate(&self) -> CollectionResult<()> {
        if self.at == 0
            || self.actor.is_empty()
            || self.request_id.is_empty()
            || self.operation_id.is_empty()
        {
            return invalid("collection mutation requires time, actor, request, and operation IDs");
        }
        Ok(())
    }
}

pub struct VectorCollectionRepository<'a, E: StorageEngine + ?Sized> {
    pub(super) engine: &'a E,
    pub(super) scope: ScopeId,
    pub(super) key: String,
}
