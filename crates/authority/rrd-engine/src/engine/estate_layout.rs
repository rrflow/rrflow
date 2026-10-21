//! One canonical project-local filesystem layout for an installed RRFlow estate.

use super::*;
use rrd_contract::{InstallationManagedPath, InstallationManagedPathKind, InstallationRemovalRule};
use rrd_store::filesystem::AnchoredPath;

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
const PRIVATE_ESTATE_DIRECTORIES: [&str; 4] = [
    ESTATE_DIRECTORY,
    STORAGE_DIRECTORY,
    STORAGE_ROOTS_DIRECTORY,
    CREDENTIAL_DIRECTORY,
];

/// One admitted project plus the exact private directories selected beneath it.
/// Retained directory identities, never display paths, authorize later effects.
#[derive(Clone, Debug)]
pub(super) struct EstateFilesystem {
    project: AnchoredPath,
    private: BTreeMap<&'static str, AnchoredPath>,
}

impl EstateFilesystem {
    pub fn admit(project: AnchoredPath, allow_missing: bool) -> Result<Self> {
        let mut filesystem = Self {
            project,
            private: BTreeMap::new(),
        };
        for relative in PRIVATE_ESTATE_DIRECTORIES {
            let path = filesystem.managed(relative)?;
            match super::token_key::pin_private_directory(&path) {
                Ok(directory) => {
                    filesystem.private.insert(relative, directory);
                }
                Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(ServiceError::Storage(error.to_string())),
            }
        }
        Ok(filesystem)
    }

    pub fn project(&self) -> &AnchoredPath {
        &self.project
    }

    pub fn managed(&self, relative: &str) -> Result<AnchoredPath> {
        validate_managed_relative(relative)?;
        if let Some((prefix, directory)) = self
            .private
            .iter()
            .filter(|(prefix, _)| {
                relative == **prefix
                    || relative
                        .strip_prefix(**prefix)
                        .is_some_and(|tail| tail.starts_with('/'))
            })
            .max_by_key(|(prefix, _)| prefix.len())
        {
            let suffix = relative
                .strip_prefix(*prefix)
                .ok_or(ServiceError::ProjectBindingMismatch)?
                .trim_start_matches('/');
            return Ok(directory.join(suffix));
        }
        Ok(self.project.join(relative))
    }

    pub fn create_private_directories(&mut self) -> Result<()> {
        for relative in PRIVATE_ESTATE_DIRECTORIES {
            if let Some(directory) = self.private.get(relative) {
                super::token_key::pin_private_directory(directory)
                    .map_err(|error| ServiceError::Storage(error.to_string()))?;
            } else {
                let directory =
                    super::token_key::create_private_directory_anchored(&self.managed(relative)?)
                        .map_err(|error| ServiceError::Storage(error.to_string()))?;
                self.private.insert(relative, directory);
            }
        }
        Ok(())
    }
}

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
pub(super) fn reject_existing_estate(project: &EstateFilesystem) -> Result<()> {
    match project.project().join(ESTATE_DIRECTORY).metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(ServiceError::ProjectBindingMismatch),
        Err(error) => Err(ServiceError::Storage(error.to_string())),
    }?;
    let entries = project
        .project()
        .entries()
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    for entry in entries {
        let name = entry
            .map_err(|error| ServiceError::Storage(error.to_string()))?
            .relative_path()
            .file_name()
            .ok_or(ServiceError::ProjectBindingMismatch)?
            .to_owned()
            .into_string()
            .map_err(|_| ServiceError::ProjectBindingMismatch)?;
        if name.starts_with(INSTALL_INTENT_PREFIX) && name.ends_with(INSTALL_INTENT_SUFFIX) {
            return Err(ServiceError::ProjectBindingMismatch);
        }
    }
    Ok(())
}

pub(super) fn create_estate_directories(project: &mut EstateFilesystem) -> Result<()> {
    project.create_private_directories()
}

pub(super) fn validate_estate_directories(
    project: &EstateFilesystem,
    allow_missing: bool,
) -> Result<()> {
    for relative in PRIVATE_ESTATE_DIRECTORIES {
        match project.private.get(relative) {
            Some(directory) => {
                super::token_key::pin_private_directory(directory)
                    .map_err(|error| ServiceError::Storage(error.to_string()))?;
            }
            None if allow_missing => {}
            None => return Err(ServiceError::ProjectBindingMismatch),
        }
    }
    Ok(())
}

pub(super) fn validate_recovery_estate(
    project: &EstateFilesystem,
    layout: &EstateLayout,
) -> Result<()> {
    validate_estate_directories(project, true)?;
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

fn validate_directory_entries(
    project: &EstateFilesystem,
    relative: &str,
    allowed: &[&str],
) -> Result<()> {
    let path = project.managed(relative)?;
    let metadata = match path.metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(ServiceError::Storage(error.to_string())),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let entries = path
        .entries()
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|error| ServiceError::Storage(error.to_string()))?;
        let name = entry
            .relative_path()
            .file_name()
            .ok_or(ServiceError::ProjectBindingMismatch)?
            .to_owned()
            .into_string()
            .map_err(|_| ServiceError::ProjectBindingMismatch)?;
        if !allowed.contains(&name.as_str()) {
            return Err(ServiceError::ProjectBindingMismatch);
        }
    }
    Ok(())
}

pub(super) fn install_intent_path(
    project: &EstateFilesystem,
    layout: &EstateLayout,
) -> Result<AnchoredPath> {
    validate_intent_relative(&layout.install_intent)?;
    Ok(project.project().join(&layout.install_intent))
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

pub(super) fn reject_existing_symlink_path(
    _project: &EstateFilesystem,
    path: &AnchoredPath,
) -> Result<()> {
    match path.metadata() {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ServiceError::Storage(error.to_string())),
    }
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

pub(super) fn safe_join(project: &EstateFilesystem, relative: &str) -> Result<AnchoredPath> {
    project.managed(relative)
}

/// One installed instance's backup catalogue or restore staging root,
/// `.rrflow/rrd/<kind>/<instance>` below its project. The served engine, the
/// offline publication, and the estate controller all resolve the same path.
pub(super) fn instance_operation_root(
    project: &EstateFilesystem,
    kind: &str,
    instance: &CanonicalId,
) -> Result<AnchoredPath> {
    safe_join(project, &format!("{STORAGE_DIRECTORY}/{kind}/{instance}"))
}
