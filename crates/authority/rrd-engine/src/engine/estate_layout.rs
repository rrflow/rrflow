//! One canonical project-local filesystem layout for an installed RRFlow estate.

use super::*;
use rrd_contract::{
    InstallationManagedPath, InstallationManagedPathKind, InstallationRemovalRule,
    ADOPTED_ESTATE_DIRECTORY, MAX_ADOPTED_ESTATE_ENTRIES,
};
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
        filesystem.admit_missing_private_directories(allow_missing)?;
        Ok(filesystem)
    }

    /// Discover newly created descendants without replacing any directory
    /// capability already admitted by this operation.
    ///
    /// Before a project has a locator, a leftover `.rrflow` folder that is not
    /// private is left unadmitted: installation plans its adoption instead.
    /// An installed estate (one with a locator) must stay private as it is.
    pub fn admit_missing_private_directories(&mut self, allow_missing: bool) -> Result<()> {
        let adoptable = allow_missing && !self.has_locator()?;
        for relative in PRIVATE_ESTATE_DIRECTORIES {
            if let Some(directory) = self.private.get(relative) {
                super::token_key::pin_private_directory(directory)
                    .map_err(|error| estate_directory_error(relative, error))?;
                continue;
            }
            let path = self.managed(relative)?;
            match super::token_key::pin_private_directory(&path) {
                Ok(directory) => {
                    self.private.insert(relative, directory);
                }
                Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {}
                // Children of an unadmitted leftover folder move with it.
                Err(_) if adoptable && relative == ESTATE_DIRECTORY => break,
                Err(_) if adoptable => {}
                Err(error) => return Err(estate_directory_error(relative, error)),
            }
        }
        Ok(())
    }

    fn has_locator(&self) -> Result<bool> {
        match self.project.join(LOCATOR_RELATIVE_PATH).metadata() {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(ServiceError::Storage(error.to_string())),
        }
    }

    /// Make a leftover `.rrflow` folder private in place and move exactly the
    /// planned top-level entries into `.rrflow/previous/<plan id>/`. Nothing is
    /// deleted. Entries already moved by an interrupted attempt are skipped.
    pub fn adopt_estate_directory(&mut self, plan_id: &str, entries: &[String]) -> Result<()> {
        rrd_contract::validate_adopted_estate_entries(entries)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        validate_managed_relative(&format!("{ESTATE_DIRECTORY}/{plan_id}"))?;
        let estate = super::token_key::adopt_private_directory_anchored(
            &self.project.join(ESTATE_DIRECTORY),
        )
        .map_err(storage_error)?;
        let previous = open_or_create_private(&estate.join(ADOPTED_ESTATE_DIRECTORY))?;
        let target = open_or_create_private(&previous.join(plan_id))?;
        for name in entries {
            let source = estate.join(name);
            match source.metadata() {
                Ok(_) => source
                    .rename_new_to(&target.join(name))
                    .map_err(storage_error)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(storage_error(error)),
            }
        }
        // Every admitted descendant moved with its entry; only the folder stays.
        self.private.clear();
        self.private.insert(ESTATE_DIRECTORY, estate);
        Ok(())
    }

    pub fn project(&self) -> &AnchoredPath {
        &self.project
    }

    pub fn managed(&self, relative: &str) -> Result<AnchoredPath> {
        if relative.starts_with(INSTALL_INTENT_PREFIX) {
            validate_intent_relative(relative)?;
        } else {
            validate_managed_relative(relative)?;
        }
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
                    .map_err(|error| estate_directory_error(relative, error))?;
            } else {
                let directory =
                    super::token_key::create_private_directory_anchored(&self.managed(relative)?)
                        .map_err(|error| estate_directory_error(relative, error))?;
                self.private.insert(relative, directory);
            }
        }
        Ok(())
    }
}

fn estate_directory_error(relative: &str, error: std::io::Error) -> ServiceError {
    ServiceError::Storage(format!("estate directory {relative}: {error}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EstateLayout {
    pub storage_root: String,
    pub token_key: String,
    pub operator_credential: String,
    pub install_intent: String,
    pub locator_pending: String,
    pub bootstrap: String,
    pub bootstrap_pending: String,
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
            bootstrap: format!("{ESTATE_DIRECTORY}/bootstrap.md"),
            bootstrap_pending: format!("{ESTATE_DIRECTORY}/.bootstrap.{storage_root_id}.pending"),
        };
        for relative in [
            layout.storage_root.as_str(),
            layout.token_key.as_str(),
            layout.operator_credential.as_str(),
            layout.locator_pending.as_str(),
            layout.bootstrap.as_str(),
            layout.bootstrap_pending.as_str(),
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
            (
                InstallationManagedPathKind::InstallationLease,
                rrd_contract::INSTALLATION_LEASE_PATH.to_owned(),
                InstallationRemovalRule::RetainForExclusion,
            ),
            (
                InstallationManagedPathKind::InstallationIntent,
                self.install_intent.clone(),
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::LocatorPending,
                self.locator_pending.clone(),
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::ProjectBootstrap,
                self.bootstrap.clone(),
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::BootstrapPending,
                self.bootstrap_pending.clone(),
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
        ]
        .into_iter()
        .map(
            |(kind, relative_path, removal_rule)| InstallationManagedPath {
                kind,
                relative_path,
                precondition_sha256: if kind == InstallationManagedPathKind::InstallationLease {
                    rrd_core::digest::sha256_hex(
                        rrd_contract::INSTALLATION_LEASE_PRECONDITION.as_bytes(),
                    )
                } else {
                    absent_sha256.to_owned()
                },
                removal_rule,
            },
        )
        .collect()
    }
}

fn open_or_create_private(path: &AnchoredPath) -> Result<AnchoredPath> {
    match super::token_key::create_private_directory_anchored(path) {
        Ok(directory) => Ok(directory),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            super::token_key::adopt_private_directory_anchored(path).map_err(storage_error)
        }
        Err(error) => Err(storage_error(error)),
    }
}

fn storage_error(error: std::io::Error) -> ServiceError {
    ServiceError::Storage(error.to_string())
}

/// The sorted top-level entries of a leftover `.rrflow` folder, excluding the
/// `previous` folder that retains earlier adoptions. `None` when the project
/// has no `.rrflow`. An installed estate (one with a locator) is never adopted.
pub(super) fn existing_estate_entries(project: &EstateFilesystem) -> Result<Option<Vec<String>>> {
    let estate = project.project().join(ESTATE_DIRECTORY);
    match estate.metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(storage_error(error)),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(ServiceError::Contract(
                "the project's .rrflow entry is not a folder; move it before installing".into(),
            ))
        }
        Ok(_) => {}
    }
    if project.has_locator()? {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let mut names = Vec::new();
    for entry in estate.entries().map_err(storage_error)? {
        let name = entry
            .map_err(storage_error)?
            .relative_path()
            .file_name()
            .ok_or(ServiceError::ProjectBindingMismatch)?
            .to_owned()
            .into_string()
            .map_err(|_| {
                ServiceError::Contract("a .rrflow entry name is not valid Unicode".into())
            })?;
        if name == ADOPTED_ESTATE_DIRECTORY {
            continue;
        }
        if names.len() >= MAX_ADOPTED_ESTATE_ENTRIES {
            return Err(ServiceError::Contract(
                "the existing .rrflow folder has too many entries to adopt".into(),
            ));
        }
        names.push(name);
    }
    names.sort();
    Ok(Some(names))
}

/// A new install adopts exactly the `.rrflow` entries its plan listed and may
/// not coexist with another installation intent. Replay is resolved from the
/// canonical locator before this check is called.
pub(super) fn reject_existing_estate(
    project: &EstateFilesystem,
    adopted: Option<&[String]>,
) -> Result<()> {
    if existing_estate_entries(project)?.as_deref() != adopted {
        return Err(ServiceError::ProjectBindingMismatch);
    }
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
                    .map_err(|error| estate_directory_error(relative, error))?;
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
    let bootstrap_pending = Path::new(&layout.bootstrap_pending)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    // The project map is a replaceable projection that install and serve
    // write below `.rrflow`; its owner names the two files.
    let [project_map, project_map_pending] =
        [PROJECT_MAP_RELATIVE_PATH, project_map::PROJECT_MAP_PENDING].map(|relative| {
            Path::new(relative)
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(ServiceError::ProjectBindingMismatch)
        });
    let (project_map, project_map_pending) = (project_map?, project_map_pending?);
    validate_directory_entries(
        project,
        ESTATE_DIRECTORY,
        &[
            "rrd",
            "credentials",
            "config.toml",
            "bootstrap.md",
            project_map,
            project_map_pending,
            ADOPTED_ESTATE_DIRECTORY,
            locator_pending,
            bootstrap_pending,
        ],
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
        || !(relative == ESTATE_DIRECTORY
            || relative.starts_with(".rrflow/")
            || relative == rrd_contract::INSTALLATION_LEASE_PATH)
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
