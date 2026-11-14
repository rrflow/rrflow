//! Effect-hold test drivers and the engine's local backup driver.

use super::*;

pub(super) struct EffectHoldDriver {
    pub(super) inner: rrd_estate::LocalProcessDriver,
    pub(super) marker: Option<PathBuf>,
}

impl EstateDriver for EffectHoldDriver {
    fn apply(
        &mut self,
        request: &rrd_estate::DriverRequest,
    ) -> std::result::Result<rrd_estate::DriverEffect, rrd_estate::DriverError> {
        let effect = self.inner.apply(request)?;
        maybe_test_hold(self.marker.as_deref(), &effect.evidence_sha256).map_err(|error| {
            rrd_estate::DriverError::retryable(error.to_string(), effect.evidence_sha256.clone())
        })?;
        Ok(effect)
    }

    fn observe(
        &mut self,
        request: &rrd_estate::DriverRequest,
    ) -> std::result::Result<rrd_estate::DriverObservation, rrd_estate::DriverError> {
        self.inner.observe(request)
    }
}

pub(super) struct BackupEffectHoldDriver {
    pub(super) inner: EngineLocalBackupDriver,
    pub(super) marker: Option<PathBuf>,
}

impl EstateBackupDriver for BackupEffectHoldDriver {
    fn create(
        &mut self,
        request: &rrd_estate::BackupDriverRequest,
    ) -> std::result::Result<rrd_estate::BackupResult, rrd_estate::DriverError> {
        let result = self.inner.create(request)?;
        maybe_test_hold(self.marker.as_deref(), &result.backup_id).map_err(|error| {
            rrd_estate::DriverError::retryable(error.to_string(), result.evidence_sha256.clone())
        })?;
        Ok(result)
    }
}

/// Engine-owned backup driver over the estate's managed instances.
///
/// A managed instance is the installed project `instances/<instance>` below
/// the estate's state root. The driver resolves it through its own locator,
/// binds the estate's distribution executable, opens it through
/// `RrdEngine::open_installed` (the same path `serve` and `verify` take, so
/// the real token key and the installed record are checked), and writes the
/// backup into the instance's own catalogue, `.rrflow/rrd/backups/<id>`,
/// which the served engine lists and the estate prunes. There is no second
/// catalogue lane.
pub(in crate::engine) struct EngineLocalBackupDriver {
    state_root: PathBuf,
    distribution_executable: PathBuf,
}

/// The resolved filesystem coordinates of one managed instance.
pub(super) struct ManagedInstance {
    pub(super) catalogue_root: rrd_store::filesystem::AnchoredPath,
    pub(super) restore_parent: rrd_store::filesystem::AnchoredPath,
}

/// Resolves `instances/<instance>` below the state root as an installed
/// project: reads its locator, requires the supplied executable to be the
/// installed distribution, and returns the instance's backup catalogue and
/// restore staging roots. Nothing is opened and nothing is created.
pub(super) fn resolve_managed_instance(
    state_root: &Path,
    instance_id: &CanonicalId,
    distribution_executable: &Path,
) -> Result<ManagedInstance> {
    let instances = state_root.join("instances");
    require_direct_directory(&instances, "managed instances root")?;
    let project = instances.join(instance_id.as_str());
    require_direct_directory(&project, "managed instance project")?;
    let (project, locator, _) = read_locator(&project)?;
    require_distribution_binding(&locator, distribution_executable)?;
    let installed = &locator.identity.instance_id;
    Ok(ManagedInstance {
        catalogue_root: instance_operation_root(&project, BACKUPS_DIRECTORY_NAME, installed)?,
        restore_parent: instance_operation_root(&project, RESTORES_DIRECTORY_NAME, installed)?,
    })
}

impl EngineLocalBackupDriver {
    pub(in crate::engine) fn new(
        state_root: impl AsRef<Path>,
        distribution_executable: impl AsRef<Path>,
    ) -> std::result::Result<Self, String> {
        let supplied = state_root.as_ref();
        if !supplied.is_absolute() {
            return Err("local backup state root must be absolute".into());
        }
        let state_root = fs::canonicalize(supplied)
            .map_err(|error| format!("cannot resolve local backup state root: {error}"))?;
        if !state_root.is_dir() {
            return Err("local backup state root must be a directory".into());
        }
        ensure_direct_directory(&state_root, "instances")?;
        ensure_direct_directory(&state_root, "processes")?;
        let executable = distribution_executable.as_ref();
        if !executable.is_absolute() {
            return Err("distribution executable path must be absolute".into());
        }
        let distribution_executable = fs::canonicalize(executable)
            .map_err(|error| format!("cannot resolve distribution executable: {error}"))?;
        if !distribution_executable.is_file() {
            return Err("distribution executable must be a regular file".into());
        }
        Ok(Self {
            state_root,
            distribution_executable,
        })
    }

    fn project_path(&self, request: &rrd_estate::BackupDriverRequest) -> PathBuf {
        self.state_root
            .join("instances")
            .join(request.instance_id.as_str())
    }

    fn create_inner(
        &self,
        request: &rrd_estate::BackupDriverRequest,
    ) -> std::result::Result<rrd_estate::BackupResult, rrd_estate::DriverError> {
        let process_record = self
            .state_root
            .join("processes")
            .join(format!("{}.json", request.instance_id));
        match fs::symlink_metadata(&process_record) {
            Ok(_) => {
                return Err(permanent(
                    request,
                    "backup denied because the instance has a retained process record",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(retryable(
                    request,
                    format!("cannot inspect the instance process record: {error}"),
                ));
            }
        }

        let project = self.project_path(request);
        require_direct_directory(&project, "managed instance project")
            .map_err(|error| permanent(request, error.to_string()))?;
        // The instance is opened exactly as `serve` opens it: through its
        // locator with the estate's distribution binding and its own token
        // key. A project that is not an installed estate, or one installed by
        // a different distribution, fails here without creating anything.
        let engine = RrdEngine::open_installed(&project, &self.distribution_executable).map_err(
            |error| permanent(request, format!("installed instance open failed: {error}")),
        )?;
        let catalogue_root = engine.backup_root().map_err(|error| {
            permanent(
                request,
                format!("backup catalogue root unresolved: {error}"),
            )
        })?;
        RrdEngine::verify_backup_catalogue(&catalogue_root).map_err(|error| {
            permanent(
                request,
                format!("backup catalogue verification failed: {error}"),
            )
        })?;
        let entry = rrd_store::create_application_backup(
            &engine.storage,
            &engine.objects,
            &catalogue_root,
            &request.label,
            request.created_at,
            &super::super::backup::installed_control_families(&engine.instance),
        )
        .map_err(|error| {
            retryable(
                request,
                format!("application-complete backup creation failed: {error}"),
            )
        })?;
        let verified = RrdEngine::verify_backup_catalogue(&catalogue_root).map_err(|error| {
            permanent(
                request,
                format!("created backup verification failed: {error}"),
            )
        })?;
        let retained = verified
            .backups
            .iter()
            .find(|candidate| candidate.backup_id == entry.backup_id)
            .ok_or_else(|| permanent(request, "created backup is absent from its catalogue"))?;
        if retained != &entry {
            return Err(permanent(
                request,
                "created backup entry diverges from its verified catalogue",
            ));
        }
        let evidence_sha256 = digest::sha256_hex(
            &serde_json::to_vec(&(
                "rrd-local-estate-backup-v1",
                request,
                &entry.backup_id,
                &entry.archive.archive_sha256,
                &verified.catalogue_sha256,
            ))
            .expect("validated local backup evidence serializes"),
        );
        Ok(rrd_estate::BackupResult {
            backup_id: entry.backup_id.clone(),
            archive_sha256: entry.archive.archive_sha256.clone(),
            catalogue_sha256: verified.catalogue_sha256,
            evidence_sha256,
        })
    }
}

impl EstateBackupDriver for EngineLocalBackupDriver {
    fn create(
        &mut self,
        request: &rrd_estate::BackupDriverRequest,
    ) -> std::result::Result<rrd_estate::BackupResult, rrd_estate::DriverError> {
        self.create_inner(request)
    }
}
