//! Installation identity, nested-repository discovery and planning.

use super::*;

impl RrdEngine {
    /// Allocate one explicit, non-secret identity input before deterministic
    /// planning. Callers retain it across previews and generate a new value for
    /// each independent engram; allocation creates no project or engine state.
    pub fn generate_installation_identity() -> Result<String> {
        let entropy = crate::engine::token_key::generate_token_key()
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        Ok(digest::sha256_hex(&entropy))
    }

    /// Discover present tracked submodule directories inside the selected root.
    /// This neither follows child Gitfiles nor authorizes or changes installation.
    pub fn discover_nested_repositories(
        project: &Path,
        profile_id: &str,
    ) -> Result<NestedRepositoryDiscovery> {
        let (profile, profile_sha256) = DefaultInstallProfile::load(profile_id)?;
        let project = AnchoredPath::open_root(project).map_err(storage_error)?;
        inventory::discover_gitlinks(&project, &profile, profile_sha256)
    }

    pub fn plan_installation(
        project: &Path,
        target_kind: InstallationTargetKind,
        installation_identity: &str,
        profile_id: &str,
        configuration_path: Option<&Path>,
        executable: &Path,
    ) -> Result<InstallationPreview> {
        Self::plan_installation_with_options(
            project,
            target_kind,
            installation_identity,
            profile_id,
            configuration_path,
            InstallationPlanningOptions::default(),
            executable,
        )
    }

    /// Produce an exact installation preview for a caller-selected proposal.
    pub fn plan_installation_with_proposal(
        project: &Path,
        target_kind: InstallationTargetKind,
        installation_identity: &str,
        profile_id: &str,
        configuration_path: Option<&Path>,
        proposal: Option<&AttunementProposal>,
        executable: &Path,
    ) -> Result<InstallationPreview> {
        Self::plan_installation_with_options(
            project,
            target_kind,
            installation_identity,
            profile_id,
            configuration_path,
            InstallationPlanningOptions {
                attunement_proposal: proposal.cloned(),
                allow_project_source_read: false,
                attunement_follow: None,
            },
            executable,
        )
    }

    /// Plan installation with explicit authorization decisions included in the
    /// immutable preview and its installation digest.
    pub fn plan_installation_with_options(
        project: &Path,
        target_kind: InstallationTargetKind,
        installation_identity: &str,
        profile_id: &str,
        configuration_path: Option<&Path>,
        options: InstallationPlanningOptions,
        executable: &Path,
    ) -> Result<InstallationPreview> {
        let proposal = options.attunement_proposal.as_ref();
        if let Some(proposal) = proposal {
            proposal.validate()?;
        }
        if !is_sha256(installation_identity) {
            return Err(ServiceError::Contract(
                "installation identity must contain 64 lowercase hexadecimal characters".into(),
            ));
        }
        let project = canonical_installing_project(project, target_kind)?;
        reject_existing_engram(&project)?;
        let (profile, profile_sha256) = DefaultInstallProfile::load(profile_id)?;
        let mut configuration = load_engram_configuration(configuration_path)?;
        if let Some(follow) = options.attunement_follow {
            configuration = configuration
                .with_attunement_follow(Some(follow))
                .map_err(|error| ServiceError::Contract(error.to_string()))?;
        }
        let executable_sha256 = sha256_file(executable)?;
        let project_precondition_sha256 = project_inventory_sha256(project.project(), &profile)?;
        let identity_seed = installation_identity_seed(
            installation_identity,
            target_kind,
            &project_precondition_sha256,
            &profile_sha256,
            &executable_sha256,
            &configuration.configuration_sha256,
        )?;
        let install_id = canonical_prefixed("install", &identity_seed)?;
        let attunement_id = canonical_prefixed("attune", &identity_seed)?;
        let instance_id = canonical_prefixed("instance", &identity_seed)?;
        let engram_id = canonical_prefixed("engram", &identity_seed)?;
        let project_id = canonical_prefixed("project", &identity_seed)?;
        let target = InstalledEngramIdentity {
            project_id,
            engram_id,
            instance_id,
        };
        let (steps, minimum, maximum) = match proposal {
            Some(proposal) => (
                proposal.normalized_steps(),
                proposal.estimated_min_duration_ms,
                proposal.estimated_max_duration_ms,
            ),
            None => (
                profile.attunement_phases.clone(),
                profile.estimated_min_duration_ms,
                profile.estimated_max_duration_ms,
            ),
        };
        profile.require_available(&steps)?;
        validate_parse_policy_limits(&steps, &profile, &configuration)?;
        let phases = planned_attunement_steps(&steps)?;
        let mut attunement = AttunementPlan {
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
            id: attunement_id,
            target: target.clone(),
            source_sha256: project_precondition_sha256.clone(),
            phases,
            estimated_min_duration_ms: minimum,
            estimated_max_duration_ms: maximum,
            plan_sha256: zero_digest(),
        };
        attunement.plan_sha256 = attunement_plan_sha256(&attunement)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;

        let retained_engram = project
            .project()
            .join(ENGRAM_DIRECTORY)
            .try_exists()
            .map_err(storage_error)?;
        let layout = EngramLayout::with_retained_engram(
            &install_id,
            &profile.initial_principal_id,
            retained_engram,
        )?;
        let absent = digest::sha256_hex(b"rrflow-install-managed-path-absent-v1");
        let mut managed_paths = layout.managed_paths(&absent);
        bind_existing_ancestors(&project, &mut managed_paths)?;
        admission::validate_new_paths(&project, &managed_paths)?;
        let bootstrap = bootstrap::build(&profile, &layout, &managed_paths)?;
        let actions = installation_actions(
            &configuration.configuration_sha256,
            &attunement.plan_sha256,
            profile.credential_bytes,
        )?;
        let mut installation = InstallationPlan {
            contract_version: INSTALLATION_PLAN_CONTRACT_VERSION,
            installation_identity: installation_identity.into(),
            project_root_sha256: initial_project_root_sha256(&project)?,
            id: install_id.clone(),
            target,
            target_kind,
            deployment: profile.deployment,
            product_version: env!("CARGO_PKG_VERSION").into(),
            executable_sha256,
            profile_id: profile.id,
            profile_sha256,
            project_precondition_sha256,
            storage_root_id: install_id,
            configuration,
            initial_seat: profile.specialization.seat.clone(),
            bootstrap,
            initial_principal_id: profile.initial_principal_id,
            initial_grants: rrd_contract::installation_initial_grants(
                options.allow_project_source_read,
            ),
            credential_bytes: profile.credential_bytes,
            inactive_capabilities: profile.inactive_capabilities,
            managed_paths,
            attunement_plan_id: attunement.id.clone(),
            attunement_plan_sha256: attunement.plan_sha256.clone(),
            actions,
            plan_sha256: zero_digest(),
        };
        installation.plan_sha256 = installation_plan_sha256(&installation)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let preview = InstallationPreview {
            preview_format_version: INSTALLATION_PREVIEW_FORMAT_VERSION,
            installation,
            attunement,
            attunement_proposal: proposal.cloned(),
            source_read_resource_scope: options
                .allow_project_source_read
                .then(|| "instance_prefix".to_owned()),
        };
        preview.validate()?;
        Ok(preview)
    }
}
