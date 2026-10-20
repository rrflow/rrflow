//! Canonical installed-estate, deployment, and operator-configuration contracts.
//!
//! These values separate durable identity and resource policy from adapter
//! topology. They govern durable effects; they do not describe or constrain a
//! model's private reasoning process.

use crate::{
    invalid, sha256_bytes, AssembleContext, CanonicalId, QueryBudget, Result,
    MAX_CONTEXT_GRAPH_DEPTH, MAX_CONTEXT_ITEMS, MAX_CONTEXT_OUTPUT_BYTES, MAX_CONTEXT_STORAGE_KEYS,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const DEPLOYMENT_PROFILE_CONTRACT_VERSION: u16 = 1;
pub const ESTATE_CONFIGURATION_FORMAT_VERSION: u16 = 2;
pub const MAX_CLOCK_ROLLBACK_MS: u64 = 3_600_000;
pub const MAX_REASONING_RUN_ELAPSED_MS: u64 = 3_600_000;
pub const MAX_REASONING_STEPS: u64 = 10_000;
pub const MAX_REASONING_STEP_ELAPSED_MS: u64 = 300_000;

/// Stable project-local identity. Organization is deliberately absent: a
/// local installation must not fabricate a tenant merely to address itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstalledEstateIdentity {
    pub project_id: CanonicalId,
    pub estate_id: CanonicalId,
    pub instance_id: CanonicalId,
}

impl InstalledEstateIdentity {
    pub fn validate(&self) -> Result<()> {
        if self.project_id == self.estate_id
            || self.project_id == self.instance_id
            || self.estate_id == self.instance_id
        {
            return invalid("installed project, estate, and instance ids must be distinct");
        }
        Ok(())
    }
}

impl fmt::Display for InstalledEstateIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}/{}/{}",
            self.project_id, self.estate_id, self.instance_id
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentForm {
    Embedded,
    SingleNodeServer,
    ClusteredServer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageProfileKind {
    RrflowMx,
    RrflowKv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointPresentation {
    InProcess,
    LoopbackHttpWebsocket,
    NetworkHttpWebsocket,
}

/// Actual service composition. The three axes cannot be collapsed into one
/// mode because storage, process shape, and listener reachability vary
/// independently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentProfile {
    pub contract_version: u16,
    pub deployment_form: DeploymentForm,
    pub storage_profile: StorageProfileKind,
    pub endpoint_presentation: EndpointPresentation,
}

impl DeploymentProfile {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != DEPLOYMENT_PROFILE_CONTRACT_VERSION {
            return invalid(format!(
                "deployment profile contract version must be {DEPLOYMENT_PROFILE_CONTRACT_VERSION}"
            ));
        }
        if self.deployment_form == DeploymentForm::ClusteredServer
            && self.storage_profile != StorageProfileKind::RrflowKv
        {
            return invalid("clustered deployment requires durable rrflowKV storage");
        }
        match (self.deployment_form, self.endpoint_presentation) {
            (DeploymentForm::Embedded, EndpointPresentation::InProcess)
            | (
                DeploymentForm::SingleNodeServer,
                EndpointPresentation::LoopbackHttpWebsocket
                | EndpointPresentation::NetworkHttpWebsocket,
            )
            | (DeploymentForm::ClusteredServer, EndpointPresentation::NetworkHttpWebsocket) => {
                Ok(())
            }
            _ => invalid("deployment form and endpoint presentation are inconsistent"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReasoningLimits {
    /// Wall-clock ceiling for a future governed reasoning run. This is not a
    /// limit on private analysis performed before a durable/effect boundary.
    pub max_run_elapsed_ms: u64,
    pub max_steps: u64,
    pub max_step_elapsed_ms: u64,
}

impl ReasoningLimits {
    pub fn validate(&self) -> Result<()> {
        validate_nonzero_max(
            "reasoning max_run_elapsed_ms",
            self.max_run_elapsed_ms,
            MAX_REASONING_RUN_ELAPSED_MS,
        )?;
        validate_nonzero_max("reasoning max_steps", self.max_steps, MAX_REASONING_STEPS)?;
        validate_nonzero_max(
            "reasoning max_step_elapsed_ms",
            self.max_step_elapsed_ms,
            MAX_REASONING_STEP_ELAPSED_MS,
        )?;
        if self.max_step_elapsed_ms > self.max_run_elapsed_ms {
            return invalid("reasoning step ceiling cannot exceed the run ceiling");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecallLimits {
    pub max_graph_depth: u8,
    pub max_items: u64,
    pub max_output_bytes: u64,
    pub max_storage_keys: u64,
}

/// Policy for wall-clock observations at governed durable-effect boundaries.
///
/// A value of zero is strict: any observed rollback from a persisted anchor is
/// rejected. A nonzero value is an explicit operator-selected allowance, not
/// evidence that the host clock is synchronized or trustworthy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClockPolicy {
    pub maximum_rollback_ms: u64,
}

impl ClockPolicy {
    pub fn validate(&self) -> Result<()> {
        if self.maximum_rollback_ms > MAX_CLOCK_ROLLBACK_MS {
            return invalid(format!(
                "clock maximum_rollback_ms must be in 0..={MAX_CLOCK_ROLLBACK_MS}"
            ));
        }
        Ok(())
    }
}

impl RecallLimits {
    pub fn validate(&self) -> Result<()> {
        if self.max_graph_depth > MAX_CONTEXT_GRAPH_DEPTH {
            return invalid(format!(
                "recall max_graph_depth must be in 0..={MAX_CONTEXT_GRAPH_DEPTH}"
            ));
        }
        validate_nonzero_max("recall max_items", self.max_items, MAX_CONTEXT_ITEMS)?;
        validate_nonzero_max(
            "recall max_output_bytes",
            self.max_output_bytes,
            MAX_CONTEXT_OUTPUT_BYTES,
        )?;
        validate_nonzero_max(
            "recall max_storage_keys",
            self.max_storage_keys,
            MAX_CONTEXT_STORAGE_KEYS,
        )
    }

    pub fn validate_request(&self, request: &AssembleContext) -> Result<()> {
        request.validate()?;
        for (name, requested, configured) in [
            (
                "max_graph_depth",
                u64::from(request.max_graph_depth),
                u64::from(self.max_graph_depth),
            ),
            ("max_items", request.max_items, self.max_items),
            (
                "max_output_bytes",
                request.max_output_bytes,
                self.max_output_bytes,
            ),
            (
                "max_storage_keys",
                request.max_storage_keys,
                self.max_storage_keys,
            ),
        ] {
            if requested > configured {
                return invalid(format!(
                    "recall request {name}={requested} exceeds configured ceiling {configured}"
                ));
            }
        }
        Ok(())
    }
}

/// Operator-authored input. Revision and digest are assigned by the engine,
/// so neither can be forged or silently retained across an edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateConfigurationInput {
    pub format_version: u16,
    pub clock: ClockPolicy,
    pub reasoning: ReasoningLimits,
    pub recall: RecallLimits,
    pub query: QueryBudget,
}

impl EstateConfigurationInput {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != ESTATE_CONFIGURATION_FORMAT_VERSION {
            return invalid(format!(
                "estate configuration format version must be {ESTATE_CONFIGURATION_FORMAT_VERSION}"
            ));
        }
        self.clock.validate()?;
        self.reasoning.validate()?;
        self.recall.validate()?;
        self.query.validate()
    }
}

/// Effective, immutable configuration bound to an installation plan and
/// surfaced to every client. Mutating it requires a future governed
/// configure-plan/configure-apply operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateConfiguration {
    pub format_version: u16,
    pub revision: u64,
    pub clock: ClockPolicy,
    pub reasoning: ReasoningLimits,
    pub recall: RecallLimits,
    pub query: QueryBudget,
    pub configuration_sha256: String,
}

impl EstateConfiguration {
    pub fn from_input(revision: u64, input: EstateConfigurationInput) -> Result<Self> {
        input.validate()?;
        if revision == 0 {
            return invalid("estate configuration revision must be greater than zero");
        }
        let mut configuration = Self {
            format_version: input.format_version,
            revision,
            clock: input.clock,
            reasoning: input.reasoning,
            recall: input.recall,
            query: input.query,
            configuration_sha256: String::new(),
        };
        configuration.configuration_sha256 = estate_configuration_sha256(&configuration)?;
        Ok(configuration)
    }

    pub fn validate(&self) -> Result<()> {
        EstateConfigurationInput {
            format_version: self.format_version,
            clock: self.clock.clone(),
            reasoning: self.reasoning.clone(),
            recall: self.recall.clone(),
            query: self.query.clone(),
        }
        .validate()?;
        if self.revision == 0 {
            return invalid("estate configuration revision must be greater than zero");
        }
        if self.configuration_sha256 != estate_configuration_sha256(self)? {
            return invalid("estate configuration digest does not match its content");
        }
        Ok(())
    }

    pub fn validate_query_budget(&self, requested: &QueryBudget) -> Result<()> {
        requested.validate()?;
        let configured = &self.query;
        for (name, requested, ceiling) in [
            (
                "max_storage_keys",
                requested.max_storage_keys,
                configured.max_storage_keys,
            ),
            ("max_rows", requested.max_rows, configured.max_rows),
            (
                "max_output_bytes",
                requested.max_output_bytes,
                configured.max_output_bytes,
            ),
            (
                "max_batch_rows",
                requested.max_batch_rows,
                configured.max_batch_rows,
            ),
            (
                "max_memory_bytes",
                requested.max_memory_bytes,
                configured.max_memory_bytes,
            ),
            (
                "max_spill_bytes",
                requested.max_spill_bytes,
                configured.max_spill_bytes,
            ),
            (
                "max_elapsed_ms",
                requested.max_elapsed_ms,
                configured.max_elapsed_ms,
            ),
        ] {
            if requested > ceiling {
                return invalid(format!(
                    "query request {name}={requested} exceeds configured ceiling {ceiling}"
                ));
            }
        }
        Ok(())
    }
}

impl Default for EstateConfiguration {
    fn default() -> Self {
        Self::from_input(
            1,
            EstateConfigurationInput {
                format_version: ESTATE_CONFIGURATION_FORMAT_VERSION,
                clock: ClockPolicy {
                    maximum_rollback_ms: 0,
                },
                reasoning: ReasoningLimits {
                    max_run_elapsed_ms: 900_000,
                    max_steps: 256,
                    max_step_elapsed_ms: 60_000,
                },
                recall: RecallLimits {
                    max_graph_depth: 4,
                    max_items: 128,
                    max_output_bytes: 512 * 1024,
                    max_storage_keys: 100_000,
                },
                query: QueryBudget::default(),
            },
        )
        .expect("default estate configuration is valid")
    }
}

pub fn estate_configuration_sha256(configuration: &EstateConfiguration) -> Result<String> {
    let encoded = serde_json::to_vec(&(
        configuration.format_version,
        configuration.revision,
        &configuration.clock,
        &configuration.reasoning,
        &configuration.recall,
        &configuration.query,
    ))
    .map_err(|error| {
        crate::ContractError(format!("configuration digest encoding failed: {error}"))
    })?;
    let mut bytes = Vec::with_capacity(40 + encoded.len());
    bytes.extend_from_slice(b"rrflow-estate-configuration-v2\0");
    bytes.extend_from_slice(&encoded);
    Ok(sha256_bytes(&bytes))
}

fn validate_nonzero_max(name: &str, value: u64, maximum: u64) -> Result<()> {
    if value == 0 || value > maximum {
        return invalid(format!("{name} must be in 1..={maximum}"));
    }
    Ok(())
}
