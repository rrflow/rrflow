//! One canonical project-local filesystem layout for an installed RRFlow engram.

use super::*;
use rrd_contract::{InstallationManagedPath, InstallationManagedPathKind, InstallationRemovalRule};
use rrd_store::filesystem::AnchoredPath;

pub(super) const ENGRAM_DIRECTORY: &str = ".rrflow";
pub(super) const STORAGE_DIRECTORY: &str = ".rrflow/rrd";
pub(super) const STORAGE_ROOTS_DIRECTORY: &str = ".rrflow/rrd/roots";
pub(super) const CREDENTIAL_DIRECTORY: &str = ".rrflow/credentials";
/// Backups of an installed engram live under `.rrflow/rrd/backups/<instance>`,
/// beside the storage roots and never below `roots/`, which holds only roots
/// the locator can select. Created on first use; retained by uninstall.
pub(super) const BACKUPS_DIRECTORY_NAME: &str = "backups";
/// Restore staging roots live under `.rrflow/rrd/restores/<instance>/<id>`
/// until an explicit publication plan promotes one into `roots/`.
pub(super) const RESTORES_DIRECTORY_NAME: &str = "restores";
pub(super) const LOCATOR_RELATIVE_PATH: &str = ".rrflow/config.toml";
const INSTALL_INTENT_PREFIX: &str = ".rrflow-install-";
const INSTALL_INTENT_SUFFIX: &str = ".json";
const PRIVATE_ENGRAM_DIRECTORIES: [&str; 4] = [
    ENGRAM_DIRECTORY,
    STORAGE_DIRECTORY,
    STORAGE_ROOTS_DIRECTORY,
    CREDENTIAL_DIRECTORY,
];

/// `.rrflow` and `.rrflow/rrd` may be project-owned directories that RRFlow did
/// not create. They are retained as they are, never made private or moved.
fn is_project_ancestor(relative: &str) -> bool {
    matches!(relative, ENGRAM_DIRECTORY | STORAGE_DIRECTORY)
}

/// One admitted project plus the exact private directories selected beneath it.
/// Retained directory identities, never display paths, authorize later effects.
#[derive(Clone, Debug)]
pub(super) struct EngramFilesystem {
    project: AnchoredPath,
    private: BTreeMap<&'static str, AnchoredPath>,
    /// True once `.rrflow` or `.rrflow/rrd` was admitted without being private.
    /// That is only allowed for a planned or proven retained ancestor; every
    /// installed-engram operation then validates the identity and security the
    /// plan bound, and a fresh engram's directories always stay strictly private.
    retained: bool,
    /// Planning or applying an install: a non-private `.rrflow` is the
    /// project's own and is bound by the plan rather than refused.
    installing: bool,
}

impl EngramFilesystem {
    pub fn admit(project: AnchoredPath, allow_missing: bool) -> Result<Self> {
        Self::admit_with(project, allow_missing, false)
    }

    /// Admission for planning and applying an install, the only operations
    /// that may meet a project-owned `.rrflow` with no proof of retention yet.
    pub fn admit_for_install(project: AnchoredPath) -> Result<Self> {
        Self::admit_with(project, true, true)
    }

    fn admit_with(project: AnchoredPath, allow_missing: bool, installing: bool) -> Result<Self> {
        let mut filesystem = Self {
            project,
            private: BTreeMap::new(),
            retained: false,
            installing,
        };
        filesystem.admit_missing_private_directories(allow_missing)?;
        Ok(filesystem)
    }

    fn pin(&self, relative: &str, path: &AnchoredPath) -> std::io::Result<AnchoredPath> {
        if self.retained && is_project_ancestor(relative) {
            path.pin_directory()
        } else {
            super::token_key::pin_private_directory(path)
        }
    }

    /// A retained install leaves an ownership proof for its locator in the
    /// private credential directory, which this check reads under strict
    /// privacy. Without one, a non-private `.rrflow` is not an engram's.
    fn has_retention_proof(&self) -> bool {
        let Ok(path) = self.managed(CREDENTIAL_DIRECTORY) else {
            return false;
        };
        if super::token_key::pin_private_directory(&path).is_err() {
            return false;
        }
        let Ok(entries) = path.entries() else {
            return false;
        };
        entries.flatten().any(|entry| {
            entry
                .relative_path()
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".config.") && name.ends_with(".proof"))
        })
    }

    /// Discover newly created descendants without replacing any directory
    /// capability already admitted by this operation. `.rrflow` and
    /// `.rrflow/rrd` may be the project's own, but only while an install is
    /// being planned or recovered, or when a retained install left its proof;
    /// everything else must be private.
    pub fn admit_missing_private_directories(&mut self, allow_missing: bool) -> Result<()> {
        for relative in PRIVATE_ENGRAM_DIRECTORIES {
            if let Some(directory) = self.private.get(relative) {
                self.pin(relative, directory)
                    .map_err(|error| engram_directory_error(relative, error))?;
                continue;
            }
            let path = self.managed(relative)?;
            let mut pinned = super::token_key::pin_private_directory(&path);
            if let Err(error) = &pinned {
                if is_project_ancestor(relative)
                    && error.kind() == std::io::ErrorKind::PermissionDenied
                    && (self.installing || self.has_retention_proof())
                {
                    self.retained = true;
                    pinned = path.pin_directory();
                }
            }
            match pinned {
                Ok(directory) => {
                    self.private.insert(relative, directory);
                }
                Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(engram_directory_error(relative, error)),
            }
        }
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

    pub fn create_private_directories(
        &mut self,
        plan: &rrd_contract::InstallationPlan,
    ) -> Result<()> {
        for relative in PRIVATE_ENGRAM_DIRECTORIES {
            if let Some(directory) = self.private.get(relative) {
                let retained = plan.managed_paths.iter().any(|path| {
                    path.relative_path == relative
                        && path.removal_rule == InstallationRemovalRule::RetainExistingDirectory
                });
                if retained {
                    directory
                        .pin_directory()
                        .map_err(|error| engram_directory_error(relative, error))?;
                } else {
                    super::token_key::pin_private_directory(directory)
                        .map_err(|error| engram_directory_error(relative, error))?;
                }
            } else {
                let directory =
                    super::token_key::create_private_directory_anchored(&self.managed(relative)?)
                        .map_err(|error| engram_directory_error(relative, error))?;
                self.private.insert(relative, directory);
            }
        }
        Ok(())
    }
}

/// Bind preexisting ancestors without claiming ownership. Newly-created
/// directories remain absent in the preview and private when applied.
pub(super) fn bind_existing_ancestors(
    project: &EngramFilesystem,
    paths: &mut [InstallationManagedPath],
) -> Result<()> {
    for path in paths.iter_mut().take(2) {
        match project
            .project()
            .join(&path.relative_path)
            .source_observation()
        {
            Ok(observed) => {
                if observed.kind() != rrd_store::filesystem::SourceKind::Directory {
                    return Err(ServiceError::ProjectBindingMismatch);
                }
                let retained = project
                    .private
                    .get(path.relative_path.as_str())
                    .ok_or(ServiceError::ProjectBindingMismatch)?;
                let held = retained
                    .source_observation()
                    .map_err(|e| engram_directory_error(&path.relative_path, e))?;
                let security = super::token_key::directory_security_sha256(retained)
                    .map_err(|e| engram_directory_error(&path.relative_path, e))?;
                if held
                    .object_identity_sha256()
                    .map_err(|e| ServiceError::Storage(e.to_string()))?
                    != observed
                        .object_identity_sha256()
                        .map_err(|e| ServiceError::Storage(e.to_string()))?
                    || project
                        .project()
                        .join(&path.relative_path)
                        .source_observation()
                        .map_err(|e| engram_directory_error(&path.relative_path, e))?
                        != observed
                {
                    return Err(ServiceError::ProjectBindingMismatch);
                }
                path.precondition_sha256 = observed
                    .revision_sha256()
                    .map_err(|e| ServiceError::Storage(e.to_string()))?;
                path.existing_directory_identity_sha256 = Some(
                    observed
                        .object_identity_sha256()
                        .map_err(|e| ServiceError::Storage(e.to_string()))?,
                );
                path.existing_directory_security_sha256 = Some(security);
                path.removal_rule = InstallationRemovalRule::RetainExistingDirectory;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(engram_directory_error(&path.relative_path, error)),
        }
    }
    Ok(())
}

/// Revalidate retained object identity after recovery; before the first write,
/// also require the exact metadata revision observed by the approved preview.
pub(super) fn validate_plan_ancestors(
    project: &EngramFilesystem,
    plan: &rrd_contract::InstallationPlan,
    require_revision: bool,
    allow_missing_owned: bool,
) -> Result<()> {
    for path in &plan.managed_paths[..4] {
        let anchored = project.project().join(&path.relative_path);
        if path.removal_rule == InstallationRemovalRule::RetainExistingDirectory {
            let observed = anchored
                .source_observation()
                .map_err(|e| engram_directory_error(&path.relative_path, e))?;
            if observed.kind() != rrd_store::filesystem::SourceKind::Directory
                || Some(
                    observed
                        .object_identity_sha256()
                        .map_err(|e| ServiceError::Storage(e.to_string()))?,
                ) != path.existing_directory_identity_sha256
                || (require_revision
                    && observed
                        .revision_sha256()
                        .map_err(|e| ServiceError::Storage(e.to_string()))?
                        != path.precondition_sha256)
            {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            let retained = project
                .private
                .get(path.relative_path.as_str())
                .ok_or(ServiceError::ProjectBindingMismatch)?;
            let held = retained
                .source_observation()
                .map_err(|e| engram_directory_error(&path.relative_path, e))?;
            if Some(
                super::token_key::directory_security_sha256(retained)
                    .map_err(|e| engram_directory_error(&path.relative_path, e))?,
            ) != path.existing_directory_security_sha256
                || held
                    .object_identity_sha256()
                    .map_err(|e| ServiceError::Storage(e.to_string()))?
                    != observed
                        .object_identity_sha256()
                        .map_err(|e| ServiceError::Storage(e.to_string()))?
            {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            let after = anchored
                .source_observation()
                .map_err(|e| engram_directory_error(&path.relative_path, e))?;
            if after
                .object_identity_sha256()
                .map_err(|e| ServiceError::Storage(e.to_string()))?
                != observed
                    .object_identity_sha256()
                    .map_err(|e| ServiceError::Storage(e.to_string()))?
                || (require_revision && after != observed)
            {
                return Err(ServiceError::ProjectBindingMismatch);
            }
        } else {
            match super::token_key::pin_private_directory(&anchored) {
                Ok(_) => {}
                Err(error)
                    if allow_missing_owned && error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(engram_directory_error(&path.relative_path, error)),
            }
        }
    }
    Ok(())
}

fn engram_directory_error(relative: &str, error: std::io::Error) -> ServiceError {
    ServiceError::Storage(format!("engram directory {relative}: {error}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EngramLayout {
    pub storage_root: String,
    pub token_key: String,
    pub operator_credential: String,
    pub install_intent: String,
    pub locator_pending: String,
    pub bootstrap: String,
    pub bootstrap_pending: String,
    pub bootstrap_proof: Option<String>,
    pub locator_proof: Option<String>,
}

impl EngramLayout {
    #[cfg(test)]
    pub fn new(storage_root_id: &CanonicalId, principal_id: &CanonicalId) -> Result<Self> {
        Self::with_retained_engram(storage_root_id, principal_id, false)
    }

    /// `retained` is true when `.rrflow` already existed: the locator and
    /// bootstrap are then staged in the private credential directory, each with
    /// an ownership proof, because a project-owned parent cannot vouch for them.
    pub fn with_retained_engram(
        storage_root_id: &CanonicalId,
        principal_id: &CanonicalId,
        retained: bool,
    ) -> Result<Self> {
        let staging_directory = if retained {
            CREDENTIAL_DIRECTORY
        } else {
            ENGRAM_DIRECTORY
        };
        let layout = Self {
            storage_root: format!("{STORAGE_ROOTS_DIRECTORY}/{storage_root_id}"),
            token_key: format!("{STORAGE_ROOTS_DIRECTORY}/{storage_root_id}/RRD.TOKEN"),
            operator_credential: format!("{CREDENTIAL_DIRECTORY}/{principal_id}.json"),
            install_intent: format!(
                "{INSTALL_INTENT_PREFIX}{storage_root_id}{INSTALL_INTENT_SUFFIX}"
            ),
            locator_pending: format!("{staging_directory}/.config.{storage_root_id}.pending"),
            bootstrap: format!("{ENGRAM_DIRECTORY}/bootstrap.md"),
            bootstrap_pending: format!("{staging_directory}/.bootstrap.{storage_root_id}.pending"),
            bootstrap_proof: retained
                .then(|| format!("{CREDENTIAL_DIRECTORY}/.bootstrap.{storage_root_id}.proof")),
            locator_proof: retained
                .then(|| format!("{CREDENTIAL_DIRECTORY}/.config.{storage_root_id}.proof")),
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
        for proof in [
            layout.bootstrap_proof.as_deref(),
            layout.locator_proof.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_managed_relative(proof)?;
        }
        Ok(layout)
    }

    pub fn for_plan(plan: &rrd_contract::InstallationPlan) -> Result<Self> {
        let retained = plan
            .managed_path(InstallationManagedPathKind::EngramDirectory)
            .removal_rule
            == InstallationRemovalRule::RetainExistingDirectory;
        Self::with_retained_engram(&plan.storage_root_id, &plan.initial_principal_id, retained)
    }

    pub fn managed_paths(&self, absent_sha256: &str) -> Vec<InstallationManagedPath> {
        let mut paths: Vec<InstallationManagedPath> = [
            (
                InstallationManagedPathKind::EngramDirectory,
                ENGRAM_DIRECTORY.to_owned(),
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
                existing_directory_identity_sha256: None,
                existing_directory_security_sha256: None,
                publication_proof: None,
            },
        )
        .collect();
        for (kind, proof) in [
            (
                InstallationManagedPathKind::BootstrapPending,
                &self.bootstrap_proof,
            ),
            (
                InstallationManagedPathKind::LocatorPending,
                &self.locator_proof,
            ),
        ] {
            if let Some(relative_path) = proof {
                let path = paths
                    .iter_mut()
                    .find(|path| path.kind == kind)
                    .expect("layout pending path");
                path.publication_proof = Some(rrd_contract::InstallationPublicationProof {
                    relative_path: relative_path.clone(),
                    precondition_sha256: absent_sha256.to_owned(),
                    removal_rule: InstallationRemovalRule::RemoveIfOwnedDigestMatches,
                });
            }
        }
        paths
    }
}

/// A new install cannot adopt an already identified engram. Other existing
/// project-owned entries are preserved and individual output collisions are
/// checked against the approved plan before any write. Replay is resolved from
/// the canonical locator before this check is called.
pub(super) fn reject_existing_engram(project: &EngramFilesystem) -> Result<()> {
    match project.project().join(LOCATOR_RELATIVE_PATH).metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => return Err(ServiceError::ProjectBindingMismatch),
        Err(error) => return Err(ServiceError::Storage(error.to_string())),
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

pub(super) fn create_engram_directories(
    project: &mut EngramFilesystem,
    plan: &rrd_contract::InstallationPlan,
) -> Result<()> {
    project.create_private_directories(plan)
}

pub(super) fn validate_engram_directories(
    project: &EngramFilesystem,
    allow_missing: bool,
) -> Result<()> {
    for relative in PRIVATE_ENGRAM_DIRECTORIES {
        match project.private.get(relative) {
            Some(directory) => {
                project
                    .pin(relative, directory)
                    .map_err(|error| engram_directory_error(relative, error))?;
            }
            None if allow_missing => {}
            None => return Err(ServiceError::ProjectBindingMismatch),
        }
    }
    Ok(())
}

pub(super) fn validate_recovery_engram(
    project: &EngramFilesystem,
    layout: &EngramLayout,
    plan: &rrd_contract::InstallationPlan,
) -> Result<()> {
    validate_engram_directories(project, true)?;
    validate_plan_ancestors(project, plan, false, true)?;
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
    // A retained `.rrflow` or `.rrflow/rrd` holds the project's own entries as
    // well; only directories RRFlow created are held to its exact entry list.
    if plan
        .managed_path(InstallationManagedPathKind::EngramDirectory)
        .removal_rule
        != InstallationRemovalRule::RetainExistingDirectory
    {
        validate_directory_entries(
            project,
            ENGRAM_DIRECTORY,
            &[
                "rrd",
                "credentials",
                "config.toml",
                "bootstrap.md",
                project_map,
                project_map_pending,
                locator_pending,
                bootstrap_pending,
            ],
        )?;
    }
    if plan
        .managed_path(InstallationManagedPathKind::StorageDirectory)
        .removal_rule
        != InstallationRemovalRule::RetainExistingDirectory
    {
        validate_directory_entries(
            project,
            STORAGE_DIRECTORY,
            &["roots", BACKUPS_DIRECTORY_NAME, RESTORES_DIRECTORY_NAME],
        )?;
    }
    validate_directory_entries(project, STORAGE_ROOTS_DIRECTORY, &[storage_id])?;
    let mut credential_entries = vec![credential_name];
    if let (Some(bootstrap_proof), Some(locator_proof)) =
        (&layout.bootstrap_proof, &layout.locator_proof)
    {
        credential_entries.push(bootstrap_pending);
        credential_entries.push(locator_pending);
        credential_entries.push(
            Path::new(bootstrap_proof)
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(ServiceError::ProjectBindingMismatch)?,
        );
        credential_entries.push(
            Path::new(locator_proof)
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(ServiceError::ProjectBindingMismatch)?,
        );
    }
    validate_directory_entries(project, CREDENTIAL_DIRECTORY, &credential_entries)
}

fn validate_directory_entries(
    project: &EngramFilesystem,
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
    project: &EngramFilesystem,
    layout: &EngramLayout,
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
    _project: &EngramFilesystem,
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
        || !(relative == ENGRAM_DIRECTORY
            || relative.starts_with(".rrflow/")
            || relative == rrd_contract::INSTALLATION_LEASE_PATH)
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

pub(super) fn safe_join(project: &EngramFilesystem, relative: &str) -> Result<AnchoredPath> {
    project.managed(relative)
}

/// One installed instance's backup catalogue or restore staging root,
/// `.rrflow/rrd/<kind>/<instance>` below its project. The served engine, the
/// offline publication, and the engram controller all resolve the same path.
pub(super) fn instance_operation_root(
    project: &EngramFilesystem,
    kind: &str,
    instance: &CanonicalId,
) -> Result<AnchoredPath> {
    safe_join(project, &format!("{STORAGE_DIRECTORY}/{kind}/{instance}"))
}
