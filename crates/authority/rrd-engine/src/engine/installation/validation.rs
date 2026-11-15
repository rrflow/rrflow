//! Installation identity seeds, actions, sealed-preview and target validation.

use super::*;

pub(super) fn installation_identity_seed(
    installation_identity: &str,
    target_kind: InstallationTargetKind,
    project_precondition_sha256: &str,
    profile_sha256: &str,
    executable_sha256: &str,
    configuration_sha256: &str,
) -> Result<String> {
    json_sha256(&(
        "rrflow-installed-engram-v3",
        installation_identity,
        target_kind,
        project_precondition_sha256,
        profile_sha256,
        executable_sha256,
        configuration_sha256,
    ))
}

pub(super) fn initial_project_root_sha256(project: &EngramFilesystem) -> Result<String> {
    let canonical = project
        .project()
        .diagnostic_path()
        .to_str()
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    json_sha256(&("rrflow-initial-project-root-v1", canonical))
}

pub(super) fn installation_actions(
    configuration_sha256: &str,
    attunement_plan_sha256: &str,
    credential_bytes: u16,
) -> Result<Vec<InstallationPlanAction>> {
    let action_kinds = [
        InstallationActionKind::ValidateProject,
        InstallationActionKind::AcquireInstallationLease,
        InstallationActionKind::PrepareCredentials,
        InstallationActionKind::CreateEngramDirectories,
        InstallationActionKind::MaterializeBootstrap,
        InstallationActionKind::CreateStorageRoot,
        InstallationActionKind::DeliverCredentials,
        InstallationActionKind::InitializeInstance,
        InstallationActionKind::CreateAttunementJob,
        InstallationActionKind::PublishProjectLocator,
    ];
    let estimates = [
        0,
        0,
        u64::from(credential_bytes) * 2 + 4_096,
        0,
        64 * 1024,
        4_096,
        u64::from(credential_bytes) * 2 + 512,
        16_384,
        4_096,
        1_024,
    ];
    action_kinds
        .into_iter()
        .zip(estimates)
        .map(|(kind, estimated_write_bytes)| {
            Ok(InstallationPlanAction {
                kind,
                disposition: if kind == InstallationActionKind::ValidateProject {
                    InstallationActionDisposition::Unchanged
                } else {
                    InstallationActionDisposition::Create
                },
                input_sha256: json_sha256(&(kind, configuration_sha256, attunement_plan_sha256))?,
                estimated_write_bytes,
            })
        })
        .collect()
}

pub(super) fn validate_sealed_preview(
    preview: &InstallationPreview,
    profile: &DefaultInstallProfile,
    profile_sha256: &str,
    executable_sha256: &str,
) -> Result<()> {
    let installation = &preview.installation;
    let seed = installation_identity_seed(
        &installation.installation_identity,
        installation.target_kind,
        &installation.project_precondition_sha256,
        profile_sha256,
        executable_sha256,
        &installation.configuration.configuration_sha256,
    )?;
    let install_id = canonical_prefixed("install", &seed)?;
    let expected_target = InstalledEngramIdentity {
        project_id: canonical_prefixed("project", &seed)?,
        engram_id: canonical_prefixed("engram", &seed)?,
        instance_id: canonical_prefixed("instance", &seed)?,
    };
    let expected_attunement_id = canonical_prefixed("attune", &seed)?;
    let (steps, minimum, maximum) = match &preview.attunement_proposal {
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
    validate_parse_policy_limits(&steps, profile, &installation.configuration)?;
    let expected_phases = planned_attunement_steps(&steps)?;
    let absent = digest::sha256_hex(b"rrflow-install-managed-path-absent-v1");
    let expected_layout = EngramLayout::for_plan(installation)?;
    let mut expected_paths = expected_layout.managed_paths(&absent);
    for (expected, actual) in expected_paths
        .iter_mut()
        .take(2)
        .zip(&installation.managed_paths)
    {
        if actual.removal_rule == rrd_contract::InstallationRemovalRule::RetainExistingDirectory {
            expected.precondition_sha256 = actual.precondition_sha256.clone();
            expected.existing_directory_identity_sha256 =
                actual.existing_directory_identity_sha256.clone();
            expected.existing_directory_security_sha256 =
                actual.existing_directory_security_sha256.clone();
            expected.removal_rule = actual.removal_rule;
        }
    }
    let expected_actions = installation_actions(
        &installation.configuration.configuration_sha256,
        &preview.attunement.plan_sha256,
        profile.credential_bytes,
    )?;
    if installation.id != install_id
        || installation.target != expected_target
        || installation.deployment != profile.deployment
        || installation.product_version != env!("CARGO_PKG_VERSION")
        || installation.executable_sha256 != executable_sha256
        || installation.profile_id != profile.id
        || installation.profile_sha256 != profile_sha256
        || installation.storage_root_id != installation.id
        || installation.configuration.revision != 1
        || installation.initial_seat != profile.specialization.seat
        || installation.bootstrap
            != bootstrap::build(profile, &expected_layout, &installation.managed_paths)?
        || installation.initial_principal_id != profile.initial_principal_id
        || installation.initial_grants
            != rrd_contract::installation_initial_grants(
                preview.source_read_resource_scope.is_some(),
            )
        || installation.credential_bytes != profile.credential_bytes
        || installation.inactive_capabilities != profile.inactive_capabilities
        || installation.managed_paths != expected_paths
        || installation.attunement_plan_id != expected_attunement_id
        || installation.actions != expected_actions
        || preview.attunement.id != expected_attunement_id
        || preview.attunement.target != expected_target
        || preview.attunement.source_sha256 != installation.project_precondition_sha256
        || preview.attunement.phases != expected_phases
        || preview.attunement.estimated_min_duration_ms != minimum
        || preview.attunement.estimated_max_duration_ms != maximum
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

pub(super) fn canonical_project(
    project: &Path,
    target_kind: InstallationTargetKind,
) -> Result<EngramFilesystem> {
    let project = canonical_project_root(project, false)?;
    validate_installation_target(&project, target_kind)?;
    Ok(project)
}

pub(super) fn canonical_installing_project(
    project: &Path,
    target_kind: InstallationTargetKind,
) -> Result<EngramFilesystem> {
    let project = canonical_project_root(project, true)?;
    validate_installation_target(&project, target_kind)?;
    Ok(project)
}

pub(super) fn canonical_project_root(project: &Path, installing: bool) -> Result<EngramFilesystem> {
    let project = AnchoredPath::open_root(project).map_err(storage_error)?;
    if installing {
        EngramFilesystem::admit_for_install(project)
    } else {
        EngramFilesystem::admit(project, true)
    }
}

pub(super) fn validate_installation_target(
    project: &EngramFilesystem,
    target_kind: InstallationTargetKind,
) -> Result<()> {
    if target_kind == InstallationTargetKind::FreshProject {
        let admitted_lease = lease::admit_existing(project.project())?;
        for entry in project.project().entries().map_err(storage_error)? {
            let entry = entry.map_err(storage_error)?;
            if admitted_lease.is_some() && entry.relative_path() == Path::new(lease::RELATIVE_PATH)
            {
                continue;
            }
            return Err(ServiceError::ProjectBindingMismatch);
        }
    }
    Ok(())
}

pub(super) fn observe_install_stage(
    preview: &InstallationPreview,
    started: Instant,
    stage: InstallApplyStage,
    recovery_state: InstallRecoveryState,
    idempotent_replay: Option<bool>,
    observer: &mut dyn FnMut(InstallApplyStage, InstallRecoveryState) -> Result<()>,
) -> Result<()> {
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let planned_write_bytes = preview
        .installation
        .actions
        .iter()
        .map(|action| action.estimated_write_bytes)
        .sum::<u64>();
    tracing::debug!(
        target: "rrflow::installation",
        operation = "install.apply",
        plan_sha256 = %preview.installation.plan_sha256,
        instance_id = %preview.installation.target.instance_id,
        stage = stage.as_str(),
        recovery_state = recovery_state.as_str(),
        idempotent_replay_known = idempotent_replay.is_some(),
        idempotent_replay = idempotent_replay.unwrap_or(false),
        planned_managed_path_count = preview.installation.managed_paths.len(),
        planned_write_bytes,
        elapsed_ms,
        "installation durable stage reached"
    );
    observer(stage, recovery_state)
}

pub(super) fn classify_installation_store(engine: &RrdEngine) -> Result<InstallRecoveryState> {
    let claim_sequence = engine.storage.claims().sequence()?;
    let runtime_cursor = engine.storage.runtime().cursor()?;
    let control_sequence = engine.storage.control().sequence()?;
    let installed = engine.storage.control().get(INSTALLED_RECORD_KEY)?;
    if claim_sequence == 0 && runtime_cursor == 0 && control_sequence == 0 && installed.is_none() {
        return Ok(InstallRecoveryState::EmptyStore);
    }
    if runtime_cursor > 0 && control_sequence > 0 && installed.is_some() {
        return Ok(InstallRecoveryState::Committed);
    }
    Err(ServiceError::StorageConflict(
        "installation recovery found an unclassifiable engine state".into(),
    ))
}

pub(super) fn require_empty_installation(
    engine: &RrdEngine,
    transitions: &[ControlTransition],
) -> Result<()> {
    if engine.storage.claims().sequence()? != 0
        || engine.storage.runtime().cursor()? != 0
        || engine.storage.control().sequence()? != 0
    {
        return Err(ServiceError::StorageConflict(
            "installation recovery expected an empty engine".into(),
        ));
    }
    for transition in transitions {
        if engine.storage.control().get(&transition.key)?.is_some() {
            return Err(ServiceError::StorageConflict(format!(
                "installation recovery found unexpected control state at {}",
                transition.key
            )));
        }
    }
    Ok(())
}

pub(super) fn require_committed_installation(
    storage_root: &AnchoredPath,
    runtime_commit: &RuntimeCommit,
    transitions: &[ControlTransition],
    expected_runtime_cursor: u64,
    expected_control_sequence: u64,
    options: rrd_store::DatabaseOptions,
) -> Result<ReadStamp> {
    let inspector = RrflowKvInspector::open_anchored_with_options(storage_root, options)?;
    let inspection = inspector.inspection();
    if inspection.claim_sequence != 0
        || inspection.runtime_cursor != expected_runtime_cursor
        || inspection.control_journal_sequence != expected_control_sequence
    {
        return Err(ServiceError::StorageConflict(
            "installation recovery found a non-canonical committed sequence".into(),
        ));
    }
    for transition in transitions {
        let expected = transition.replacement.as_ref().ok_or_else(|| {
            ServiceError::StorageConflict(
                "installation bootstrap contains a non-create transition".into(),
            )
        })?;
        if inspector.control_record(&transition.key)?.as_ref() != Some(expected) {
            return Err(ServiceError::StorageConflict(format!(
                "installation committed control state differs at {}",
                transition.key
            )));
        }
    }
    let read = inspector.runtime_read_stamp(&runtime_commit.scope)?;
    if read.scope != runtime_commit.scope || read.commit_cursor != expected_runtime_cursor {
        return Err(ServiceError::StorageConflict(
            "installation semantic read stamp differs from the sealed bootstrap".into(),
        ));
    }
    Ok(read)
}
