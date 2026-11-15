use crate::{
    invalid, transaction_operation_sha256, validate_sha256, CanonicalId, ContextPacket,
    ContextReadStamp, DataReference, ReadEvidence, Result, TransactionMutation,
    MAX_CONTEXT_GRAPH_DEPTH, MAX_CONTEXT_ITEMS, MAX_CONTEXT_OUTPUT_BYTES, MAX_CONTEXT_QUERY_BYTES,
    MAX_CONTEXT_STORAGE_KEYS,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MEMORY_SEAT_KIND: &str = "rrflow-seat";
pub const MEMORY_PROVIDER_IDENTITY_KIND: &str = "rrflow-provider-identity";
pub const MEMORY_REPRESENTS_KIND: &str = "rrflow-represents";
pub const MAX_PROVIDER_REPRESENTATIONS: usize = 64;

/// A durable seat is the provider-independent identity RRFlow resolves as
/// self. Provider accounts may represent it, but never replace its identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemorySeatDefinition {
    pub id: CanonicalId,
    pub display_name: String,
    pub purpose: String,
}

impl MemorySeatDefinition {
    pub fn validate(&self) -> Result<()> {
        validate_bounded_text(&self.display_name, "seat display_name", 256, false)?;
        validate_bounded_text(&self.purpose, "seat purpose", 4_096, true)
    }
}

/// One provider-neutral representation of a durable seat. Only the opaque
/// provider name and a digest of the provider subject are persisted; provider
/// credentials and runtime sessions are deliberately outside this contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderRepresentationDefinition {
    pub id: CanonicalId,
    pub provider_identity: CanonicalId,
    pub provider: CanonicalId,
    pub subject_sha256: String,
}

impl ProviderRepresentationDefinition {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.subject_sha256, "provider subject_sha256")
    }
}

/// Input for a canonical memory-engram mutation plan. The plan is committed
/// with the normal authenticated data transaction capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PersistMemoryEngram {
    pub scope: String,
    pub seat: MemorySeatDefinition,
    #[serde(default)]
    pub representations: Vec<ProviderRepresentationDefinition>,
    pub valid_from: u64,
}

impl PersistMemoryEngram {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        self.seat.validate()?;
        if self.valid_from == 0 {
            return invalid("memory engram valid_from must be greater than zero");
        }
        if self.representations.len() > MAX_PROVIDER_REPRESENTATIONS {
            return invalid("memory engram exceeds its provider representation bound");
        }
        let mut ids = BTreeSet::new();
        let mut provider_identities = BTreeSet::new();
        for representation in &self.representations {
            representation.validate()?;
            if !ids.insert(&representation.id) {
                return invalid("provider representation ids must be unique");
            }
            if !provider_identities.insert(&representation.provider_identity) {
                return invalid("provider identities must be unique within a memory engram plan");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemoryEngramPlan {
    pub mutations: Vec<TransactionMutation>,
    pub operation_sha256: String,
}

impl MemoryEngramPlan {
    pub fn validate(&self) -> Result<()> {
        if self.mutations.is_empty() {
            return invalid("memory engram plan must contain at least one mutation");
        }
        for mutation in &self.mutations {
            mutation.validate()?;
        }
        validate_sha256(&self.operation_sha256, "memory engram operation_sha256")?;
        if self.operation_sha256 != transaction_operation_sha256(&self.mutations) {
            return invalid("memory engram operation digest does not match its mutations");
        }
        Ok(())
    }
}

/// Stable, path-safe coordinate into one canonical RRFlow data record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemoryWarp {
    pub instance: CanonicalId,
    pub target: DataReference,
}

impl MemoryWarp {
    pub fn new(instance: CanonicalId, target: DataReference) -> Self {
        Self { instance, target }
    }

    pub fn uri(&self) -> String {
        format!(
            "rrflow://{}/data/{}/{}",
            self.instance, self.target.kind, self.target.id
        )
    }

    pub fn parse(uri: &str) -> Result<Self> {
        if uri.as_bytes().contains(&0) {
            return invalid("memory warp URI must not contain NUL");
        }
        let remainder = uri
            .strip_prefix("rrflow://")
            .ok_or_else(|| crate::ContractError("memory warp URI must use rrflow://".into()))?;
        let segments = remainder.split('/').collect::<Vec<_>>();
        if segments.len() != 4 || segments[1] != "data" {
            return invalid("memory warp URI must be rrflow://<instance>/data/<kind>/<id>");
        }
        Ok(Self {
            instance: CanonicalId::new(segments[0])?,
            target: DataReference {
                kind: CanonicalId::new(segments[2])?,
                id: CanonicalId::new(segments[3])?,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveMemoryWarp {
    pub scope: String,
    pub uri: String,
    #[serde(default)]
    pub query: String,
    pub valid_at: u64,
    pub max_graph_depth: u8,
    pub max_items: u64,
    pub max_output_bytes: u64,
    pub max_storage_keys: u64,
}

impl ResolveMemoryWarp {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        MemoryWarp::parse(&self.uri)?;
        if self.query.len() > MAX_CONTEXT_QUERY_BYTES || self.query.as_bytes().contains(&0) {
            return invalid("memory warp query exceeds its byte bound or contains NUL");
        }
        if self.valid_at == 0 {
            return invalid("memory warp valid_at must be greater than zero");
        }
        if self.max_graph_depth > MAX_CONTEXT_GRAPH_DEPTH {
            return invalid("memory warp graph depth exceeds its bound");
        }
        if self.max_items == 0 || self.max_items > MAX_CONTEXT_ITEMS {
            return invalid("memory warp max_items exceeds its bound");
        }
        if self.max_output_bytes == 0 || self.max_output_bytes > MAX_CONTEXT_OUTPUT_BYTES {
            return invalid("memory warp max_output_bytes exceeds its bound");
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_CONTEXT_STORAGE_KEYS {
            return invalid("memory warp max_storage_keys exceeds its bound");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedMemoryWarp {
    pub uri: String,
    pub target: DataReference,
    pub context: ContextPacket,
}

impl ResolvedMemoryWarp {
    pub fn validate(&self) -> Result<()> {
        let warp = MemoryWarp::parse(&self.uri)?;
        if warp.target != self.target {
            return invalid("resolved memory warp target differs from its URI");
        }
        self.context.validate()?;
        let identity = format!("record:{}:{}", self.target.kind, self.target.id);
        if !self
            .context
            .items
            .iter()
            .any(|item| item.identity == identity)
        {
            return invalid("resolved memory warp context does not contain its target");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveSeatIdentity {
    pub scope: String,
    pub seat_id: CanonicalId,
    pub valid_at: u64,
    pub max_storage_keys: u64,
}

impl ResolveSeatIdentity {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        if self.valid_at == 0 {
            return invalid("seat resolution valid_at must be greater than zero");
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_CONTEXT_STORAGE_KEYS {
            return invalid("seat resolution max_storage_keys exceeds its bound");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderRepresentation {
    pub id: CanonicalId,
    pub provider_identity: CanonicalId,
    pub provider: CanonicalId,
    pub subject_sha256: String,
}

impl ProviderRepresentation {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.subject_sha256, "provider subject_sha256")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeatIdentity {
    pub uri: String,
    pub seat_id: CanonicalId,
    pub display_name: String,
    pub purpose: String,
    pub representations: Vec<ProviderRepresentation>,
    pub read: ContextReadStamp,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
}

impl SeatIdentity {
    pub fn validate(&self) -> Result<()> {
        validate_bounded_text(&self.display_name, "seat display_name", 256, false)?;
        validate_bounded_text(&self.purpose, "seat purpose", 4_096, true)?;
        let warp = MemoryWarp::parse(&self.uri)?;
        if warp.target.kind.as_str() != MEMORY_SEAT_KIND || warp.target.id != self.seat_id {
            return invalid("seat identity URI does not address the resolved seat");
        }
        if self.representations.is_empty()
            || self.representations.len() > MAX_PROVIDER_REPRESENTATIONS
        {
            return invalid("seat identity must have a bounded non-empty representation set");
        }
        let mut ids = BTreeSet::new();
        for representation in &self.representations {
            representation.validate()?;
            if !ids.insert(&representation.id) {
                return invalid("seat identity representation ids must be unique");
            }
        }
        if self
            .representations
            .windows(2)
            .any(|pair| pair[0].id >= pair[1].id)
        {
            return invalid("seat identity representations must be sorted");
        }
        validate_sha256(
            &self.read.runtime_manifest_sha256,
            "seat read runtime_manifest_sha256",
        )?;
        self.read_evidence.validate()
    }
}

fn validate_scope(scope: &str) -> Result<()> {
    if scope.is_empty() || scope.len() > 256 || scope.as_bytes().contains(&0) {
        return invalid("memory engram scope must be a non-empty bounded string without NUL");
    }
    Ok(())
}

fn validate_bounded_text(value: &str, name: &str, maximum: usize, allow_empty: bool) -> Result<()> {
    if (!allow_empty && value.trim().is_empty())
        || value.len() > maximum
        || value.as_bytes().contains(&0)
    {
        return invalid(format!("{name} is empty, oversized, or contains NUL"));
    }
    Ok(())
}
