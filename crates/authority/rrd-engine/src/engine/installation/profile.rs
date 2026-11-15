//! The default install profile, attunement proposals and engram and estate configuration loading.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultInstallProfile {
    pub format_version: u16,
    pub id: CanonicalId,
    pub configuration_asset: String,
    pub deployment: DeploymentProfile,
    pub template_asset: String,
    pub specialization: rrd_contract::InstallationSpecialization,
    pub initial_principal_id: CanonicalId,
    pub credential_bytes: u16,
    pub initial_grant_policy: CanonicalId,
    pub inactive_capabilities: Vec<CanonicalId>,
    /// Capabilities this profile offers. The engine intersects them with its
    /// installed executors when it derives a plan from evidence; the initial
    /// `attunement_phases` must be among them.
    pub available_capabilities: Vec<AttunementPhase>,
    pub attunement_phases: Vec<DefaultAttunementPhase>,
    pub estimated_min_duration_ms: u64,
    pub estimated_max_duration_ms: u64,
    pub(in super::super) inventory_policy: policy::InventoryPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultAttunementPhase {
    pub phase: AttunementPhase,
    pub configuration: String,
    pub estimated_items: u64,
    pub estimated_input_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_policy: Option<rrd_contract::SourceParsePolicy>,
}

/// One proposed step. Description is metadata and does not configure execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementProposalStep {
    pub phase: AttunementPhase,
    #[schemars(schema_with = "attunement_description_schema")]
    pub description: String,
    pub estimated_items: u64,
    pub estimated_input_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_policy: Option<rrd_contract::SourceParsePolicy>,
}

/// A bounded, explicitly proposed sequence of recognized attunement effects.
/// The proposal is a public plan input, not a record of private reasoning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementProposal {
    #[schemars(schema_with = "attunement_proposal_version_schema")]
    pub format_version: u16,
    #[schemars(schema_with = "attunement_proposal_steps_schema")]
    pub steps: Vec<AttunementProposalStep>,
    pub estimated_min_duration_ms: u64,
    pub estimated_max_duration_ms: u64,
}

pub(super) fn attunement_description_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    // JSON Schema counts characters; engine validation also enforces the byte bound.
    schemars::json_schema!({"type": "string", "minLength": 1, "maxLength": MAX_ATTUNEMENT_DESCRIPTION_BYTES})
}

pub(super) fn attunement_proposal_version_schema(
    _: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    schemars::json_schema!({"type": "integer", "const": ATTUNEMENT_PROPOSAL_FORMAT_VERSION})
}

pub(super) fn attunement_proposal_steps_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    let mut schema = <Vec<AttunementProposalStep> as schemars::JsonSchema>::json_schema(generator);
    schema.insert("minItems".into(), serde_json::json!(1));
    schema.insert("maxItems".into(), serde_json::json!(MAX_ATTUNEMENT_PHASES));
    schema
}

/// Mechanical proposal shape; semantic and effect validation remain in the engine.
pub fn attunement_proposal_schema() -> Result<serde_json::Value> {
    serde_json::to_value(schemars::schema_for!(AttunementProposal)).map_err(contract_json)
}

impl AttunementProposal {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != ATTUNEMENT_PROPOSAL_FORMAT_VERSION {
            return Err(ServiceError::Contract(
                "unsupported attunement proposal version".into(),
            ));
        }
        // Reject oversized caller-owned input before cloning descriptions.
        if self.steps.is_empty()
            || self.steps.len() > MAX_ATTUNEMENT_PHASES
            || self
                .steps
                .iter()
                .any(|step| step.description.len() > MAX_ATTUNEMENT_DESCRIPTION_BYTES)
        {
            return Err(ServiceError::Contract(
                "attunement proposal exceeds its bounds".into(),
            ));
        }
        // Validate caller-owned policy sizes before normalized_steps clones them.
        for step in &self.steps {
            if let Some(policy) = &step.parse_policy {
                policy
                    .validate()
                    .map_err(|error| ServiceError::Contract(error.to_string()))?;
            }
        }
        validate_attunement_steps(
            &self.normalized_steps(),
            self.estimated_min_duration_ms,
            self.estimated_max_duration_ms,
        )
    }

    pub(super) fn normalized_steps(&self) -> Vec<DefaultAttunementPhase> {
        self.steps
            .iter()
            .map(|step| DefaultAttunementPhase {
                phase: step.phase,
                configuration: step.description.clone(),
                estimated_items: step.estimated_items,
                estimated_input_bytes: step.estimated_input_bytes,
                parse_policy: step.parse_policy.clone(),
            })
            .collect()
    }
}

pub(super) fn validate_attunement_steps(
    steps: &[DefaultAttunementPhase],
    minimum: u64,
    maximum: u64,
) -> Result<()> {
    if steps.is_empty()
        || steps.len() > MAX_ATTUNEMENT_PHASES
        || minimum > maximum
        || steps[..steps.len().saturating_sub(1)]
            .iter()
            .any(|step| step.phase == AttunementPhase::Verify)
        || steps.iter().any(|step| {
            step.configuration.trim().is_empty()
                || step.configuration.len() > MAX_ATTUNEMENT_DESCRIPTION_BYTES
                || step.configuration.as_bytes().contains(&0)
        })
    {
        return Err(ServiceError::Contract(
            "attunement proposal steps or bounds are invalid".into(),
        ));
    }
    for step in steps {
        if let Some(policy) = &step.parse_policy {
            if step.phase != AttunementPhase::Parse {
                return Err(ServiceError::Contract(
                    "source parse policy requires a Parse step".into(),
                ));
            }
            policy
                .validate()
                .map_err(|error| ServiceError::Contract(error.to_string()))?;
        }
    }
    Ok(())
}

pub(super) fn planned_attunement_steps(
    steps: &[DefaultAttunementPhase],
) -> Result<Vec<AttunementPhasePlan>> {
    steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            Ok(AttunementPhasePlan {
                phase: step.phase,
                sequence: u16::try_from(index + 1)
                    .map_err(|_| ServiceError::Contract("attunement step index overflow".into()))?,
                description: step.configuration.clone(),
                configuration_sha256: json_sha256(step)?,
                estimated_items: step.estimated_items,
                estimated_input_bytes: step.estimated_input_bytes,
                parse_policy: step.parse_policy.clone(),
            })
        })
        .collect()
}

pub(in super::super) fn validate_parse_policy_limits(
    steps: &[DefaultAttunementPhase],
    profile: &DefaultInstallProfile,
    configuration: &rrd_contract::EstateConfiguration,
) -> Result<()> {
    validate_parse_policies(
        steps.iter().filter_map(|step| step.parse_policy.as_ref()),
        profile,
        configuration,
    )
}

/// The same bound for a derived plan, whose steps are contract phase plans.
pub(in super::super) fn validate_parse_policies<'a>(
    policies: impl Iterator<Item = &'a rrd_contract::SourceParsePolicy>,
    profile: &DefaultInstallProfile,
    configuration: &rrd_contract::EstateConfiguration,
) -> Result<()> {
    let inventory = &profile.inventory_policy.limits;
    for policy in policies {
        if policy.max_files > inventory.entries
            || policy.max_file_bytes > inventory.file_bytes
            || policy.max_input_bytes > inventory.content_bytes
            || policy.max_commit_bytes > configuration.storage.transaction_admission_bytes
        {
            return Err(ServiceError::ConfigurationLimit(
                "source parse policy exceeds installed inventory or transaction limits".into(),
            ));
        }
    }
    Ok(())
}

/// Read only an explicitly selected bounded regular proposal during planning.
pub fn load_attunement_proposal(path: &Path) -> Result<AttunementProposal> {
    let bytes = read_regular_bounded(
        &AnchoredPath::from_path(path).map_err(storage_error)?,
        MAX_ATTUNEMENT_PROPOSAL_BYTES,
    )?;
    let proposal: AttunementProposal = serde_json::from_slice(&bytes).map_err(contract_json)?;
    proposal.validate()?;
    Ok(proposal)
}

impl DefaultInstallProfile {
    pub(in super::super) fn load(requested: &str) -> Result<(Self, String)> {
        let bytes = match requested {
            "default" => INSTALL_PROFILE_BYTES,
            "clyffy" => CLYFFY_PROFILE_BYTES,
            _ => {
                return Err(ServiceError::Contract(format!(
                    "unsupported installation profile {requested:?}"
                )))
            }
        };
        let profile: Self = serde_json::from_slice(bytes).map_err(contract_json)?;
        profile.validate()?;
        if profile.id.as_str() != requested {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        Ok((profile, digest::sha256_hex(bytes)))
    }

    fn validate(&self) -> Result<()> {
        if self.format_version != INSTALL_PROFILE_FORMAT
            || !matches!(self.id.as_str(), "default" | "clyffy")
            || self.template_asset != "templates/generic-project-v1.md"
            || self.configuration_asset != "default-estate-configuration-v3.toml"
            || self.initial_grant_policy.as_str() != "local-owner-all-actions"
            || !(32..=4_096).contains(&self.credential_bytes)
            || self.estimated_min_duration_ms > self.estimated_max_duration_ms
        {
            return Err(ServiceError::Contract(
                "default installation profile has invalid bounds or identity".into(),
            ));
        }
        self.inventory_policy.compile()?;
        self.specialization
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.deployment
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if self.deployment.storage_profile != StorageProfileKind::RrflowKv {
            return Err(ServiceError::Contract(
                "default installation profile must use durable rrflowKV".into(),
            ));
        }
        if self
            .inactive_capabilities
            .windows(2)
            .any(|capabilities| capabilities[0] >= capabilities[1])
        {
            return Err(ServiceError::Contract(
                "default installation profile capabilities are non-canonical".into(),
            ));
        }
        if self.available_capabilities.is_empty()
            || self
                .available_capabilities
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || !self
                .available_capabilities
                .contains(&AttunementPhase::Connect)
            || !self
                .available_capabilities
                .contains(&AttunementPhase::Inventory)
        {
            return Err(ServiceError::Contract(
                "default installation profile capabilities must be canonical and include connect and inventory".into(),
            ));
        }
        validate_attunement_steps(
            &self.attunement_phases,
            self.estimated_min_duration_ms,
            self.estimated_max_duration_ms,
        )?;
        self.require_available(&self.attunement_phases)
    }

    /// Every planned step must be a capability this profile offers.
    pub(in super::super) fn require_available(
        &self,
        steps: &[DefaultAttunementPhase],
    ) -> Result<()> {
        if steps
            .iter()
            .any(|step| !self.available_capabilities.contains(&step.phase))
        {
            return Err(ServiceError::Contract(
                "attunement step selects a capability the installation profile does not offer"
                    .into(),
            ));
        }
        Ok(())
    }
}

/// Loads the bundled default or one explicitly selected, bounded operator
/// configuration. Planning is the only lifecycle stage that reads this file;
/// apply consumes the effective configuration sealed into the plan.
pub fn load_engram_configuration(path: Option<&Path>) -> Result<EstateConfiguration> {
    let bytes = match path {
        None => DEFAULT_CONFIGURATION_BYTES.to_vec(),
        Some(path) => read_regular_bounded(
            &AnchoredPath::from_path(path).map_err(storage_error)?,
            MAX_CONFIGURATION_BYTES,
        )?,
    };
    let input: EstateConfigurationInput = toml::from_str(
        std::str::from_utf8(&bytes).map_err(|error| ServiceError::Contract(error.to_string()))?,
    )
    .map_err(|error| ServiceError::Contract(error.to_string()))?;
    let configuration = EstateConfiguration::from_input(1, input)
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    super::super::core::installed_storage_options(&configuration.storage)?;
    Ok(configuration)
}

/// A read-only suggestion; neither a complete repository inventory nor a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NestedRepositoryDiscovery {
    pub format_version: u16,
    pub profile_sha256: String,
    pub git_inputs_sha256: String,
    pub git_index_verified: bool,
    pub coverage: &'static str,
    pub recommended_layout: &'static str,
    pub selection_available: bool,
    pub candidates: Vec<String>,
}

/// Retained source-level name for existing engine callers; the engram-facing
/// installer and the established configuration contract share one loader.
pub fn load_estate_configuration(path: Option<&Path>) -> Result<EstateConfiguration> {
    load_engram_configuration(path)
}
