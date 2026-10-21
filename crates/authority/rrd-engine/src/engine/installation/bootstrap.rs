//! Pure bundled bootstrap projection and validation at installation effects.
use super::*;
use rrd_contract::{
    InstallationBootstrap, InstallationBootstrapPolicy, InstallationRecordKind,
    InstallationStorageGeneration, INSTALLATION_BOOTSTRAP_VERSION, MAX_BOOTSTRAP_RECORD_BYTES,
};

const TEMPLATE: &str = include_str!("../../../assets/install/templates/generic-project-v1.md");

pub(super) fn build(
    profile: &DefaultInstallProfile,
    layout: &EstateLayout,
) -> Result<InstallationBootstrap> {
    let formats = rrd_store::RrflowKvStore::initialization_formats();
    let paths = layout.managed_paths(&digest::sha256_hex(
        b"rrflow-install-managed-path-absent-v1",
    ));
    let mut private_directories: Vec<_> = paths[..4]
        .iter()
        .map(|path| path.relative_path.clone())
        .collect();
    private_directories.sort();
    let mut private_files = vec![
        layout.token_key.clone(),
        layout.operator_credential.clone(),
        layout.install_intent.clone(),
        rrd_contract::INSTALLATION_LEASE_PATH.to_owned(),
    ];
    private_files.sort();
    let mut bootstrap = InstallationBootstrap {
        format_version: INSTALLATION_BOOTSTRAP_VERSION,
        template_id: CanonicalId::new("generic-project")
            .map_err(|error| ServiceError::Contract(error.to_string()))?,
        template_revision: 1,
        template_text: TEMPLATE.into(),
        template_sha256: digest::sha256_hex(TEMPLATE.as_bytes()),
        specialization: profile.specialization.clone(),
        specialization_sha256: profile
            .specialization
            .digest()
            .map_err(|error| ServiceError::Contract(error.to_string()))?,
        instructions_sha256: zero_digest(),
        project_policy: InstallationBootstrapPolicy::PreserveProjectOwnedState,
        initial_records: InstallationRecordKind::INITIAL.to_vec(),
        max_record_bytes: MAX_BOOTSTRAP_RECORD_BYTES,
        storage_generation: InstallationStorageGeneration {
            application_format: formats.application_format,
            manifest_format: formats.manifest_format,
            wal_format: formats.wal_format,
            batch_format: formats.batch_format,
            max_initial_transaction_bytes: 4 * MAX_BOOTSTRAP_RECORD_BYTES,
        },
        token_key_bytes: 32,
        credential_entropy_bytes: profile.credential_bytes,
        private_directories,
        private_files,
        process_invocations: Vec::new(),
        network_endpoints: Vec::new(),
        activated_provider_bindings: Vec::new(),
    };
    bootstrap.instructions_sha256 =
        digest::sha256_hex(bootstrap.rendered_instructions().as_bytes());
    bootstrap
        .validate()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    Ok(bootstrap)
}

pub(super) fn materialize(
    project: &EstateFilesystem,
    layout: &EstateLayout,
    preview: &InstallationPreview,
    staged: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    let bytes = preview.installation.bootstrap.rendered_instructions();
    publish_new_file_with_stage(
        &project.managed(&layout.bootstrap)?,
        &project.managed(&layout.bootstrap_pending)?,
        bytes.as_bytes(),
        staged,
    )
}

pub(super) fn verify(project: &EstateFilesystem, plan: &InstallationPlan) -> Result<()> {
    let path = project.managed(
        &plan
            .managed_path(InstallationManagedPathKind::ProjectBootstrap)
            .relative_path,
    )?;
    let bytes = read_regular_bounded(&path, MAX_LOCATOR_BYTES)?;
    if digest::sha256_hex(&bytes) != plan.bootstrap.instructions_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

/// Match the actual initial transaction to the declared semantic effects,
/// before handing any of those payloads to the storage commit authority.
pub(super) fn validate_transaction(
    preview: &InstallationPreview,
    runtime: &RuntimeCommit,
    transitions: &[ControlTransition],
) -> Result<()> {
    let expected = [
        "security.initialized",
        "security.audit",
        "security.audit.head",
        "configuration.installed",
        "install.record",
        "install.checkpoint",
        "attunement.plan.created",
        "attunement.job.created",
    ];
    if transitions.len() != expected.len()
        || !matches!(
            runtime.mutations.as_slice(),
            [
                RuntimeMutation::Schema { .. },
                RuntimeMutation::Record { .. }
            ]
        )
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let limits = &preview.installation.bootstrap;
    let mut total = serde_json::to_vec(runtime).map_err(contract_json)?.len() as u64;
    for (transition, action) in transitions.iter().zip(expected) {
        let bytes = transition
            .replacement
            .as_ref()
            .ok_or(ServiceError::ProjectBindingMismatch)?;
        if transition.action != action
            || transition.expected.is_some()
            || bytes.len() as u64 > limits.max_record_bytes
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        total = total
            .checked_add(bytes.len() as u64)
            .ok_or(ServiceError::ProjectBindingMismatch)?;
    }
    for mutation in &runtime.mutations {
        if serde_json::to_vec(mutation).map_err(contract_json)?.len() as u64
            > limits.max_record_bytes
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
    }
    if total > limits.storage_generation.max_initial_transaction_bytes {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}
