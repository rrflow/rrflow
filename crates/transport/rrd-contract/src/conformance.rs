//! Deployment conformance corpora and service capabilities.

use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One checked-in logical corpus used unchanged by every deployment adapter.
/// Physical cursors, transport evidence, and storage receipts remain
/// mode-specific; the expected logical identities may not diverge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConformanceCorpus {
    pub format_version: u16,
    pub documents: Vec<DeploymentConformanceDocument>,
    pub query: DeploymentConformanceQuery,
    pub expected_ids: Vec<CanonicalId>,
}

impl DeploymentConformanceCorpus {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != DEPLOYMENT_CONFORMANCE_FORMAT_VERSION {
            return invalid(format!(
                "deployment conformance format must be {DEPLOYMENT_CONFORMANCE_FORMAT_VERSION}"
            ));
        }
        if self.documents.is_empty() || self.documents.len() > MAX_DEPLOYMENT_CONFORMANCE_DOCUMENTS
        {
            return invalid(format!(
                "deployment conformance documents must contain 1..={MAX_DEPLOYMENT_CONFORMANCE_DOCUMENTS} entries"
            ));
        }
        let mut document_ids = BTreeSet::new();
        for document in &self.documents {
            document.validate()?;
            if !document_ids.insert(&document.id) {
                return invalid("deployment conformance document identities must be unique");
            }
        }
        self.query.validate()?;
        if self.expected_ids.is_empty()
            || self.expected_ids.len() > usize::from(self.query.top_k)
            || self
                .expected_ids
                .iter()
                .any(|identity| !document_ids.contains(identity))
        {
            return invalid(
                "deployment conformance expected identities must be non-empty, bounded by top_k, and present in the corpus",
            );
        }
        if self.expected_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return invalid("deployment conformance expected identities must be unique and sorted");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConformanceDocument {
    pub id: CanonicalId,
    pub text: String,
}

impl DeploymentConformanceDocument {
    pub fn validate(&self) -> Result<()> {
        if self.text.is_empty() || self.text.len() > MAX_MESSAGE_BYTES {
            return invalid(format!(
                "deployment conformance document text must contain 1..={MAX_MESSAGE_BYTES} bytes"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConformanceQuery {
    pub rrflowql: String,
    pub edge_text: String,
    pub valid_at: u64,
    pub top_k: u16,
}

impl DeploymentConformanceQuery {
    pub fn validate(&self) -> Result<()> {
        if self.rrflowql.is_empty() || self.rrflowql.len() > MAX_QUERY_BYTES {
            return invalid(format!(
                "deployment conformance rrflowQL must contain 1..={MAX_QUERY_BYTES} bytes"
            ));
        }
        if self.edge_text.is_empty() || self.edge_text.len() > MAX_MESSAGE_BYTES {
            return invalid(format!(
                "deployment conformance edge query must contain 1..={MAX_MESSAGE_BYTES} bytes"
            ));
        }
        if self.valid_at == 0 || self.top_k == 0 {
            return invalid("deployment conformance valid_at and top_k must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Unavailable,
    Experimental,
    Available,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDescriptor {
    pub name: CanonicalId,
    pub contract_version: u16,
    pub status: CapabilityStatus,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub limits: BTreeMap<CanonicalId, u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limitation: Option<String>,
}

impl CapabilityDescriptor {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version == 0 {
            return invalid("capability contract version must be greater than zero");
        }
        if self
            .limitation
            .as_ref()
            .is_some_and(|value| value.is_empty() || value.len() > MAX_MESSAGE_BYTES)
        {
            return invalid(format!(
                "capability limitation length must be in 1..={MAX_MESSAGE_BYTES} bytes"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceCapabilities {
    pub protocol: String,
    pub protocol_version: u16,
    pub implementation: CanonicalId,
    pub implementation_version: String,
    pub deployment: DeploymentProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_engram: Option<InstalledEngramIdentity>,
    pub configuration: EngramConfiguration,
    pub instance: ResourceId,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub product_capabilities: ProductCapabilityCatalogue,
}

impl ServiceCapabilities {
    pub fn validate(&self) -> Result<()> {
        validate_protocol(&self.protocol, self.protocol_version)?;
        self.deployment.validate()?;
        self.configuration.validate()?;
        if self.instance.kind != ResourceKind::Instance {
            return invalid("service capability identity must be an instance resource");
        }
        if let Some(installed) = &self.installed_engram {
            installed.validate()?;
            if installed.instance_id != self.instance.id {
                return invalid(
                    "service capability instance differs from installed engram identity",
                );
            }
        }
        if self.implementation_version.is_empty()
            || self.implementation_version.len() > MAX_ID_BYTES
            || !self.implementation_version.is_ascii()
        {
            return invalid(format!(
                "implementation version must be 1..={MAX_ID_BYTES} ASCII bytes"
            ));
        }
        if self.capabilities.len() > MAX_CAPABILITIES {
            return invalid(format!(
                "service may advertise at most {MAX_CAPABILITIES} capabilities"
            ));
        }
        let mut names = BTreeSet::new();
        for capability in &self.capabilities {
            capability.validate()?;
            if !names.insert(&capability.name) {
                return invalid("service capability names must be unique");
            }
        }
        if self
            .capabilities
            .windows(2)
            .any(|pair| pair[0].name > pair[1].name)
        {
            return invalid("service capabilities must be sorted by canonical name");
        }
        self.product_capabilities.validate()?;
        Ok(())
    }
}
