//! Installed engram identity, deployment profile, reasoning and recall limits and clock policy.

use super::*;

pub const DEPLOYMENT_PROFILE_CONTRACT_VERSION: u16 = 1;
pub const ENGRAM_CONFIGURATION_FORMAT_VERSION: u16 = 3;
pub const MAX_CLOCK_ROLLBACK_MS: u64 = 3_600_000;
pub const MAX_REASONING_RUN_ELAPSED_MS: u64 = 3_600_000;
pub const MAX_REASONING_STEPS: u64 = 10_000;
pub const MAX_REASONING_STEP_ELAPSED_MS: u64 = 300_000;

/// Stable project-local identity. Organization is deliberately absent: a
/// local installation must not fabricate a tenant merely to address itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstalledEngramIdentity {
    pub project_id: CanonicalId,
    pub engram_id: CanonicalId,
    pub instance_id: CanonicalId,
}

impl InstalledEngramIdentity {
    pub fn validate(&self) -> Result<()> {
        if self.project_id == self.engram_id
            || self.project_id == self.instance_id
            || self.engram_id == self.instance_id
        {
            return invalid("installed project, engram, and instance ids must be distinct");
        }
        Ok(())
    }
}

impl fmt::Display for InstalledEngramIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}/{}/{}",
            self.project_id, self.engram_id, self.instance_id
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
