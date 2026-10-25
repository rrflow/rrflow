//! Project-local configuration, separate from external capability authorization.
use crate::{
    invalid, sha256_bytes, EndpointProfile, EndpointProfileInput, InstalledEstateIdentity, Result,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ENDPOINT_CONFIGURATION_VERSION: u16 = 1;
pub const MAX_ENDPOINT_CONFIGURATION_BYTES: usize = 1_048_576;
pub const ENDPOINT_CONFIGURATION_PREVIEW_TTL_MS: u64 = 300_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadEndpointConfiguration {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointConfiguration {
    pub format_version: u16,
    pub identity: InstalledEstateIdentity,
    /// Zero means never configured; revocation retains a nonzero revision.
    pub revision: u64,
    pub profile: Option<EndpointProfile>,
    pub last_applied_plan_sha256: Option<String>,
}

impl EndpointConfiguration {
    pub fn validate(&self) -> Result<()> {
        self.identity.validate()?;
        if self.format_version != ENDPOINT_CONFIGURATION_VERSION {
            return invalid("unsupported endpoint configuration version");
        }
        if let Some(profile) = &self.profile {
            profile.validate()?;
            if profile.revision != self.revision {
                return invalid("endpoint profile revision differs from configuration");
            }
        }
        if self.revision == 0 {
            if self.profile.is_some() || self.last_applied_plan_sha256.is_some() {
                return invalid("initial endpoint configuration must be empty");
            }
        } else if !self
            .last_applied_plan_sha256
            .as_ref()
            .is_some_and(|v| is_digest(v))
        {
            return invalid("configured endpoint revision requires an applied plan digest");
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
pub struct PreviewEndpointConfiguration {
    pub expected_revision: u64,
    /// None previews revocation. This neither deletes history nor contacts a provider.
    pub profile: Option<EndpointProfileInput>,
}
impl PreviewEndpointConfiguration {
    pub fn validate(&self) -> Result<()> {
        if let Some(profile) = &self.profile {
            profile.validate()?;
        }
        if self.expected_revision == u64::MAX {
            return invalid("endpoint configuration revision exhausted");
        }
        bounded(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointConfigurationPlan {
    pub format_version: u16,
    pub identity: InstalledEstateIdentity,
    pub expected_revision: u64,
    pub expected_configuration_sha256: String,
    pub profile: Option<EndpointProfile>,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub plan_sha256: String,
}
impl EndpointConfigurationPlan {
    pub fn sha256(&self) -> Result<String> {
        let mut content = self.clone();
        content.plan_sha256.clear();
        Ok(sha256_bytes(
            &serde_json::to_vec(&content).map_err(|e| crate::ContractError(e.to_string()))?,
        ))
    }
    pub fn validate(&self) -> Result<()> {
        self.identity.validate()?;
        let next = self.expected_revision.checked_add(1);
        if self.format_version != ENDPOINT_CONFIGURATION_VERSION
            || next.is_none()
            || !is_digest(&self.expected_configuration_sha256)
            || !is_digest(&self.plan_sha256)
            || self
                .issued_at_unix_ms
                .checked_add(ENDPOINT_CONFIGURATION_PREVIEW_TTL_MS)
                != Some(self.expires_at_unix_ms)
        {
            return invalid("invalid endpoint configuration plan");
        }
        if let Some(profile) = &self.profile {
            profile.validate()?;
            if Some(profile.revision) != next {
                return invalid("preview profile revision is not the next revision");
            }
        }
        bounded(self)?;
        if self.plan_sha256 != self.sha256()? {
            return invalid("endpoint plan digest does not match content");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyEndpointConfiguration {
    pub plan: EndpointConfigurationPlan,
    pub approved_plan_sha256: String,
}
impl ApplyEndpointConfiguration {
    pub fn validate(&self) -> Result<()> {
        self.plan.validate()?;
        if self.approved_plan_sha256 != self.plan.plan_sha256 {
            return invalid("exact endpoint plan approval is required");
        }
        bounded(self)
    }
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
        > MAX_ENDPOINT_CONFIGURATION_BYTES
    {
        return invalid("endpoint configuration exceeds byte limit");
    }
    Ok(())
}
