//! Read-only installation preflight shared by initial admission and revalidation
//! under the native lease. It does not create or advance durable lifecycle state.
use super::recovery::InstallIntent;
use super::*;

pub(super) enum Admission {
    New(ClockObservation),
    Recover(Box<InstallIntent>),
    Published {
        intent: Option<Box<InstallIntent>>,
        result: Box<InstallationResult>,
    },
}

pub(super) fn inspect(
    project: &EstateFilesystem,
    layout: &EstateLayout,
    preview: &InstallationPreview,
    profile: &DefaultInstallProfile,
    executable: &Path,
    clock: &dyn ClockSource,
) -> Result<Admission> {
    let executable_sha256 = &preview.installation.executable_sha256;
    match safe_join(project, LOCATOR_RELATIVE_PATH)?.metadata() {
        Ok(_) => {
            let intent = read_existing_intent(project, layout, preview, executable_sha256)?;
            let result = replay_installation_result(project, preview, executable, clock)?;
            if intent.is_some() {
                validate_recovery_estate(project, layout)?;
            }
            return Ok(Admission::Published {
                intent: intent.map(Box::new),
                result: Box::new(result),
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(storage_error(error)),
    }
    if initial_project_root_sha256(project)? != preview.installation.project_root_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    if let Some(intent) = read_existing_intent(project, layout, preview, executable_sha256)? {
        require_accepted(&assess(
            clock.observe(),
            intent.clock_anchor(),
            &preview.installation.configuration.clock,
            "install.apply.recover",
        )?)?;
        validate_recovery_estate(project, layout)?;
        if project_inventory_sha256_excluding(
            project.project(),
            profile,
            Some(Path::new(&layout.install_intent)),
        )? != preview.installation.project_precondition_sha256
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        return Ok(Admission::Recover(Box::new(intent)));
    }
    reject_existing_estate(project)?;
    validate_installation_target(project, preview.installation.target_kind)?;
    if project_inventory_sha256(project.project(), profile)?
        != preview.installation.project_precondition_sha256
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    for managed in &preview.installation.managed_paths {
        if managed.kind != InstallationManagedPathKind::InstallationLease
            && project
                .managed(&managed.relative_path)?
                .try_exists()
                .map_err(storage_error)?
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
    }
    Ok(Admission::New(observe_required(
        clock,
        "install.apply.new",
    )?))
}
