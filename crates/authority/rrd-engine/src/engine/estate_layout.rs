//! One canonical project-local filesystem layout for an installed RRFlow estate.

use super::*;
use rrd_contract::{InstallationManagedPath, InstallationManagedPathKind, InstallationRemovalRule};

pub(super) const ESTATE_DIRECTORY: &str = ".rrflow";
pub(super) const STORAGE_DIRECTORY: &str = ".rrflow/rrd";
pub(super) const STORAGE_ROOTS_DIRECTORY: &str = ".rrflow/rrd/roots";
pub(super) const CREDENTIAL_DIRECTORY: &str = ".rrflow/credentials";
/// Backups of an installed estate live under `.rrflow/rrd/backups/<instance>`,
/// beside the storage roots and never below `roots/`, which holds only roots
/// the locator can select. Created on first use; retained by uninstall.
pub(super) const BACKUPS_DIRECTORY_NAME: &str = "backups";
/// Restore staging roots live under `.rrflow/rrd/restores/<instance>/<id>`
/// until an explicit publication plan promotes one into `roots/`.
pub(super) const RESTORES_DIRECTORY_NAME: &str = "restores";
pub(super) const LOCATOR_RELATIVE_PATH: &str = ".rrflow/config.toml";
const INSTALL_INTENT_PREFIX: &str = ".rrflow-install-";
const INSTALL_INTENT_SUFFIX: &str = ".json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EstateLayout {
    pub storage_root: String,
    pub token_key: String,
    pub operator_credential: String,
    pub install_intent: String,
    pub locator_pending: String,
}

impl EstateLayout {
    pub fn new(storage_root_id: &CanonicalId, principal_id: &CanonicalId) -> Result<Self> {
        let layout = Self {
            storage_root: format!("{STORAGE_ROOTS_DIRECTORY}/{storage_root_id}"),
            token_key: format!("{STORAGE_ROOTS_DIRECTORY}/{storage_root_id}/RRD.TOKEN"),
            operator_credential: format!("{CREDENTIAL_DIRECTORY}/{principal_id}.json"),
            install_intent: format!(
                "{INSTALL_INTENT_PREFIX}{storage_root_id}{INSTALL_INTENT_SUFFIX}"
            ),
            locator_pending: format!("{ESTATE_DIRECTORY}/.config.{storage_root_id}.pending"),
        };
        for relative in [
            layout.storage_root.as_str(),
            layout.token_key.as_str(),
            layout.operator_credential.as_str(),
            layout.locator_pending.as_str(),
            LOCATOR_RELATIVE_PATH,
        ] {
            validate_managed_relative(relative)?;
        }
        validate_intent_relative(&layout.install_intent)?;
        Ok(layout)
    }

    pub fn managed_paths(&self, absent_sha256: &str) -> Vec<InstallationManagedPath> {
        [
            (
                InstallationManagedPathKind::EstateDirectory,
                ESTATE_DIRECTORY.to_owned(),
                InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            ),
            (
                InstallationManagedPathKind::StorageDirectory,
                STORAGE_DIRECTORY.to_owned(),
                InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            ),
            (
                InstallationManagedPathKind::StorageRootsDirectory,
                STORAGE_ROOTS_DIRECTORY.to_owned(),
                InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            ),
            (
                InstallationManagedPathKind::CredentialDirectory,
                CREDENTIAL_DIRECTORY.to_owned(),
                InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            ),
            (
                InstallationManagedPathKind::StorageRoot,
                self.storage_root.clone(),
                InstallationRemovalRule::RemoveIfOwnedTreeDigestMatches,
            ),
            (
                InstallationManagedPathKind::TokenKey,
                self.token_key.clone(),
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::OperatorCredential,
                self.operator_credential.clone(),
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::ProjectLocator,
                LOCATOR_RELATIVE_PATH.to_owned(),
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
        ]
        .into_iter()
        .map(
            |(kind, relative_path, removal_rule)| InstallationManagedPath {
                kind,
                relative_path,
                precondition_sha256: absent_sha256.to_owned(),
                removal_rule,
            },
        )
        .collect()
    }
}

/// A new install may not coexist with either a pre-canonical
/// `.rrflow/instance.toml` estate or an unrecognized `.rrflow` tree. Replay is
/// resolved from the canonical locator before this check is called.
pub(super) fn reject_existing_estate(project: &Path) -> Result<()> {
    match std::fs::symlink_metadata(project.join(ESTATE_DIRECTORY)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(ServiceError::ProjectBindingMismatch),
        Err(error) => Err(ServiceError::Storage(error.to_string())),
    }?;
    let entries =
        std::fs::read_dir(project).map_err(|error| ServiceError::Storage(error.to_string()))?;
    for entry in entries {
        let name = entry
            .map_err(|error| ServiceError::Storage(error.to_string()))?
            .file_name()
            .into_string()
            .map_err(|_| ServiceError::ProjectBindingMismatch)?;
        if name.starts_with(INSTALL_INTENT_PREFIX) && name.ends_with(INSTALL_INTENT_SUFFIX) {
            return Err(ServiceError::ProjectBindingMismatch);
        }
    }
    Ok(())
}

pub(super) fn create_estate_directories(project: &Path) -> Result<()> {
    for relative in [
        ESTATE_DIRECTORY,
        STORAGE_DIRECTORY,
        STORAGE_ROOTS_DIRECTORY,
        CREDENTIAL_DIRECTORY,
    ] {
        let path = project.join(relative);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(ServiceError::Storage(error.to_string())),
        }
        let parent = path.parent().ok_or(ServiceError::ProjectBindingMismatch)?;
        owner_only_directory_builder()
            .create(&path)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        rrd_store::sync_directory_metadata(parent)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
    }
    Ok(())
}

/// Estate directories are owner-only (0700) on Unix; other platforms use the
/// platform's default directory permissions.
fn owner_only_directory_builder() -> std::fs::DirBuilder {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        builder
    }
    #[cfg(not(unix))]
    {
        std::fs::DirBuilder::new()
    }
}

pub(super) fn validate_recovery_estate(project: &Path, layout: &EstateLayout) -> Result<()> {
    let storage_id = Path::new(&layout.storage_root)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    let credential_name = Path::new(&layout.operator_credential)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    let locator_pending = Path::new(&layout.locator_pending)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    validate_directory_entries(
        project,
        ESTATE_DIRECTORY,
        &["rrd", "credentials", "config.toml", locator_pending],
    )?;
    validate_directory_entries(
        project,
        STORAGE_DIRECTORY,
        &["roots", BACKUPS_DIRECTORY_NAME, RESTORES_DIRECTORY_NAME],
    )?;
    validate_directory_entries(project, STORAGE_ROOTS_DIRECTORY, &[storage_id])?;
    validate_directory_entries(project, CREDENTIAL_DIRECTORY, &[credential_name])
}

fn validate_directory_entries(project: &Path, relative: &str, allowed: &[&str]) -> Result<()> {
    let path = project.join(relative);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(ServiceError::Storage(error.to_string())),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let entries =
        std::fs::read_dir(&path).map_err(|error| ServiceError::Storage(error.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|error| ServiceError::Storage(error.to_string()))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| ServiceError::ProjectBindingMismatch)?;
        if !allowed.contains(&name.as_str()) {
            return Err(ServiceError::ProjectBindingMismatch);
        }
    }
    Ok(())
}

pub(super) fn install_intent_path(project: &Path, layout: &EstateLayout) -> Result<PathBuf> {
    validate_intent_relative(&layout.install_intent)?;
    Ok(project.join(&layout.install_intent))
}

fn validate_intent_relative(relative: &str) -> Result<()> {
    if !relative.starts_with(INSTALL_INTENT_PREFIX)
        || !relative.ends_with(INSTALL_INTENT_SUFFIX)
        || relative.contains('/')
        || relative.contains('\\')
        || relative.len() > 256
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

pub(super) fn reject_existing_symlink_path(project: &Path, path: &Path) -> Result<()> {
    let relative = path
        .strip_prefix(project)
        .map_err(|_| ServiceError::ProjectBindingMismatch)?;
    let mut current = project.to_owned();
    for component in relative.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(ServiceError::Storage(error.to_string())),
        }
    }
    Ok(())
}

pub(super) fn validate_managed_relative(relative: &str) -> Result<()> {
    if relative.is_empty()
        || relative.len() > 4_096
        || relative.starts_with('/')
        || relative.contains('\\')
        || relative
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
        || !(relative == ESTATE_DIRECTORY || relative.starts_with(".rrflow/"))
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

pub(super) fn safe_join(project: &Path, relative: &str) -> Result<PathBuf> {
    validate_managed_relative(relative)?;
    let path = project.join(relative);
    reject_existing_symlink_path(project, &path)?;
    Ok(path)
}
