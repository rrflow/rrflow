//! Canonical identifiers, resource paths, correlation and request context.

use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeSet;
use std::fmt;

/// A canonical public identifier component.
///
/// IDs are lowercase ASCII and intentionally URL/path safe. Human-facing
/// labels are separate data and may use arbitrary Unicode.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[schemars(transparent)]
pub struct CanonicalId(String);

impl CanonicalId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_ID_BYTES {
            return invalid(format!(
                "canonical id length must be in 1..={MAX_ID_BYTES} bytes"
            ));
        }
        if !value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || (index > 0 && matches!(byte, b'-' | b'_' | b'.'))
        }) {
            return invalid(
                "canonical id must start with lowercase ASCII or a digit and contain only lowercase ASCII, digits, '-', '_', or '.'",
            );
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for CanonicalId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for CanonicalId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CanonicalId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Organization,
    Estate,
    Project,
    Instance,
    Tenant,
    Namespace,
    Database,
    Table,
    Collection,
    Record,
    Point,
    Relation,
    Alias,
    Cluster,
    Node,
    Shard,
    Replica,
    Segment,
    Transaction,
    Snapshot,
    Backup,
    Operation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceId {
    pub kind: ResourceKind,
    pub id: CanonicalId,
}

impl ResourceId {
    pub fn new(kind: ResourceKind, id: impl Into<String>) -> Result<Self> {
        Ok(Self {
            kind,
            id: CanonicalId::new(id)?,
        })
    }
}

/// A fully explicit hierarchical identity. No field is inferred from process
/// cwd, connection state, or a human label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourcePath {
    pub segments: Vec<ResourceId>,
}

impl ResourcePath {
    pub fn validate(&self) -> Result<()> {
        if self.segments.is_empty() {
            return invalid("resource path must contain at least one segment");
        }
        if self.segments.len() > 16 {
            return invalid("resource path must contain at most 16 segments");
        }
        let mut kinds = BTreeSet::new();
        for segment in &self.segments {
            if !kinds.insert(segment.kind) {
                return invalid("resource path must not repeat a resource kind");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[schemars(transparent)]
pub struct CorrelationId(String);

impl CorrelationId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_ID_BYTES {
            return invalid(format!(
                "correlation id length must be in 1..={MAX_ID_BYTES} bytes"
            ));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
        {
            return invalid(
                "correlation id must contain only ASCII letters, digits, '-', '_', '.', or ':'",
            );
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for CorrelationId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CorrelationId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestContext {
    pub request_id: CorrelationId,
    pub operation_id: CorrelationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<CorrelationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_unix_ms: Option<u64>,
}

impl RequestContext {
    pub fn validate(&self, mutation: bool) -> Result<()> {
        if mutation && self.idempotency_key.is_none() {
            return invalid("mutating requests require an idempotency key");
        }
        if self.deadline_unix_ms == Some(0) {
            return invalid("deadline_unix_ms must be greater than zero when present");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdempotencyBinding {
    pub key: CorrelationId,
    pub operation_sha256: String,
}

impl IdempotencyBinding {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.operation_sha256, "operation_sha256")
    }
}
