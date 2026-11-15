//! Explicit, inactive observation-source configuration; no collector is activated here.
use crate::{invalid, sha256_bytes, CanonicalId, CorrelationId, InstalledEngramIdentity, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const OBSERVATION_SOURCE_CONFIGURATION_VERSION: u16 = 1;
pub const OBSERVATION_SOURCE_PREVIEW_TTL_MS: u64 = 300_000;
const MAX_SOURCES: usize = 64;
const MAX_CONFIGURATION_BYTES: usize = 1_048_576;
const MAX_EVENT_BYTES: u64 = 1_048_576;
const MAX_BATCH_BYTES: u64 = 8_388_608;
const MAX_BATCH_EVENTS: u32 = 1_024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservationTransport {
    OtlpLogs,
    AcpStream,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservationContentScope {
    MetadataOnly,
    ContentAuthorized,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationSourceBinding {
    pub id: CanonicalId,
    pub producer: CanonicalId,
    pub collector_principal: CanonicalId,
    pub transport: ObservationTransport,
    pub content_scope: ObservationContentScope,
    pub max_event_bytes: u64,
    pub max_batch_events: u32,
    pub max_batch_bytes: u64,
}
fn validate_sources(sources: &[ObservationSourceBinding]) -> Result<()> {
    if sources.len() > MAX_SOURCES {
        return invalid("too many observation sources");
    }
    let mut previous: Option<&str> = None;
    for source in sources {
        if previous.is_some_and(|id| id >= source.id.as_str()) {
            return invalid("observation source ids must be sorted and unique");
        }
        previous = Some(source.id.as_str());
        if source.max_event_bytes == 0
            || source.max_event_bytes > MAX_EVENT_BYTES
            || source.max_batch_events == 0
            || source.max_batch_events > MAX_BATCH_EVENTS
            || source.max_batch_bytes == 0
            || source.max_batch_bytes > MAX_BATCH_BYTES
            || source.max_event_bytes > source.max_batch_bytes
        {
            return invalid("invalid observation source limits");
        }
    }
    Ok(())
}
fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn bounded(value: &impl Serialize) -> Result<()> {
    if serde_json::to_vec(value)
        .map_err(|e| crate::ContractError(e.to_string()))?
        .len()
        > MAX_CONFIGURATION_BYTES
    {
        return invalid("observation configuration exceeds byte limit");
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationSourceConfiguration {
    pub format_version: u16,
    pub identity: InstalledEngramIdentity,
    pub revision: u64,
    pub sources: Vec<ObservationSourceBinding>,
    pub last_applied_plan_sha256: Option<String>,
}
impl ObservationSourceConfiguration {
    pub fn validate(&self) -> Result<()> {
        self.identity.validate()?;
        validate_sources(&self.sources)?;
        if self.format_version != OBSERVATION_SOURCE_CONFIGURATION_VERSION {
            return invalid("unsupported observation configuration version");
        }
        if self.revision == 0 {
            if !self.sources.is_empty() || self.last_applied_plan_sha256.is_some() {
                return invalid("initial observation configuration must be empty");
            }
        } else if !self
            .last_applied_plan_sha256
            .as_ref()
            .is_some_and(|d| is_digest(d))
        {
            return invalid("configured revision requires plan digest");
        }
        bounded(self)
    }
    pub fn sha256(&self) -> Result<String> {
        self.validate()?;
        Ok(sha256_bytes(
            &serde_json::to_vec(self).map_err(|e| crate::ContractError(e.to_string()))?,
        ))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewObservationSourceConfiguration {
    pub expected_revision: u64,
    pub sources: Vec<ObservationSourceBinding>,
}
impl PreviewObservationSourceConfiguration {
    pub fn validate(&self) -> Result<()> {
        if self.expected_revision == u64::MAX {
            return invalid("observation configuration revision exhausted");
        }
        validate_sources(&self.sources)?;
        bounded(self)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationSourceConfigurationPlan {
    pub format_version: u16,
    pub identity: InstalledEngramIdentity,
    pub session_id: CorrelationId,
    pub expected_revision: u64,
    pub expected_configuration_sha256: String,
    pub sources: Vec<ObservationSourceBinding>,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub plan_sha256: String,
    /// Engine-issued MAC over the plan digest; not an external credential.
    pub engine_authenticator: Vec<u8>,
}
impl ObservationSourceConfigurationPlan {
    pub fn sha256(&self) -> Result<String> {
        let mut content = self.clone();
        content.plan_sha256.clear();
        content.engine_authenticator.clear();
        Ok(sha256_bytes(
            &serde_json::to_vec(&content).map_err(|e| crate::ContractError(e.to_string()))?,
        ))
    }
    pub fn validate(&self) -> Result<()> {
        self.identity.validate()?;
        validate_sources(&self.sources)?;
        if self.format_version != OBSERVATION_SOURCE_CONFIGURATION_VERSION
            || self.expected_revision == u64::MAX
            || !is_digest(&self.expected_configuration_sha256)
            || !is_digest(&self.plan_sha256)
            || self.engine_authenticator.len() != 32
            || self
                .issued_at_unix_ms
                .checked_add(OBSERVATION_SOURCE_PREVIEW_TTL_MS)
                != Some(self.expires_at_unix_ms)
        {
            return invalid("invalid observation configuration plan");
        }
        bounded(self)?;
        if self.plan_sha256 != self.sha256()? {
            return invalid("observation plan digest does not match content");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyObservationSourceConfiguration {
    pub plan: ObservationSourceConfigurationPlan,
    pub approved_plan_sha256: String,
}
impl ApplyObservationSourceConfiguration {
    pub fn validate(&self) -> Result<()> {
        self.plan.validate()?;
        if self.approved_plan_sha256 != self.plan.plan_sha256 {
            return invalid("exact observation plan approval is required");
        }
        bounded(self)
    }
}
