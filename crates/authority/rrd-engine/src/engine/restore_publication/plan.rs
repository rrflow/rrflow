//! Planning a restore publication.

use super::*;

impl RrdEngine {
    /// Previews the publication of one staged restore. The plan binds the
    /// candidate, the active root it replaces, the backup that produced the
    /// candidate, and read-only evidence of both roots; `apply` refuses any
    /// drift from it. Planning writes nothing and fails while the estate is
    /// being served.
    pub fn plan_restore_publication(
        project: &Path,
        restore_id: &CanonicalId,
        backup_sha256: &str,
        executable: &Path,
    ) -> Result<RestorePublicationPlan> {
        if !is_lower_hex_sha256(backup_sha256) {
            return Err(ServiceError::Contract(
                "restore publication backup_sha256 must be lowercase SHA-256 hex".into(),
            ));
        }
        let (project, locator, locator_bytes) = read_locator(project)?;
        let executable_sha256 = require_distribution_binding(&locator, executable)?;
        let paths = publication_paths(&project, &locator, restore_id)?;
        let storage_root_id = storage_root_id(&locator)?;
        if paths.intent.metadata().is_ok() {
            return Err(ServiceError::Contract(
                "a restore publication is already in progress for this restore; apply its plan"
                    .into(),
            ));
        }
        if paths.quarantine.metadata().is_ok() {
            return Err(ServiceError::Contract(
                "the quarantine location for this restore already exists".into(),
            ));
        }
        require_directory(&paths.candidate, "restore candidate root")?;
        require_directory(&paths.active, "active storage root")?;

        let backup = catalogued_backup(&paths.backups, backup_sha256)?;
        rrd_store::verify_restored_backup_objects_with_options(
            &paths.backups,
            backup_sha256,
            &paths.candidate,
            super::super::core::installed_storage_options(&locator.storage)?,
        )?;
        let active = RootReading::open(&paths.active, &locator)?;
        let candidate = RootReading::open(&paths.candidate, &locator)?;
        if candidate.evidence.claim_sequence != backup.archive.claim_sequence
            || candidate.evidence.runtime_cursor != backup.archive.runtime_cursor
        {
            return Err(ServiceError::Backup(
                "restore candidate watermarks differ from the backup".into(),
            ));
        }
        require_locator_record_match(&locator, &active.installed)?;
        if candidate.installed != active.installed {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        if candidate.configuration != active.configuration
            || candidate.configuration.revision != locator.configuration_revision
            || candidate.configuration.configuration_sha256 != locator.configuration_sha256
        {
            return Err(ServiceError::Contract(
                "restore candidate configuration differs from the locator; locator replacement is not supported".into(),
            ));
        }
        let token_key_sha256 = active_token_key_sha256(&paths.active, &active.installed)?;
        candidate_token_key_state(&paths.candidate, &token_key_sha256)?;

        let seed = json_sha256(&(
            "rrflow-restore-publication-v1",
            &locator.identity,
            restore_id,
            backup_sha256,
            &active.evidence.physical_sha256,
            &candidate.evidence.physical_sha256,
        ))?;
        let actions = RESTORE_PUBLICATION_ACTIONS
            .iter()
            .map(|kind| {
                let (input, estimated_write_bytes) = match kind {
                    RestorePublicationActionKind::VerifyCandidate => (
                        json_sha256(&(backup_sha256, &candidate.evidence.physical_sha256))?,
                        0,
                    ),
                    RestorePublicationActionKind::CarryTokenKey => {
                        (token_key_sha256.clone(), TOKEN_KEY_BYTES as u64)
                    }
                    RestorePublicationActionKind::QuarantineActiveRoot => {
                        (active.evidence.physical_sha256.clone(), 0)
                    }
                    RestorePublicationActionKind::PublishCandidateRoot => {
                        (candidate.evidence.physical_sha256.clone(), 0)
                    }
                    RestorePublicationActionKind::OpenPublishedEstate => {
                        (active.installed.record_sha256.clone(), 1_024)
                    }
                };
                Ok(RestorePublicationAction {
                    kind: *kind,
                    input_sha256: input,
                    estimated_write_bytes,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut plan = RestorePublicationPlan {
            contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
            id: canonical_prefixed("restore-publication", &seed)?,
            target: locator.identity.clone(),
            product_version: locator.product_version.clone(),
            executable_sha256,
            restore_id: restore_id.clone(),
            backup_sha256: backup_sha256.into(),
            backup_archive_sha256: backup.archive.archive_sha256.clone(),
            backup_control_archive_sha256: backup
                .control_archive
                .as_ref()
                .map(|archive| archive.archive_sha256.clone())
                .ok_or_else(|| {
                    ServiceError::Backup("backup carries no control-journal archive".into())
                })?,
            storage_root_id,
            active_root: locator.storage_root.clone(),
            candidate_root: relative(&paths.project, &paths.candidate)?,
            quarantine_root: relative(&paths.project, &paths.quarantine)?,
            token_key_sha256,
            installed_record_sha256: active.installed.record_sha256.clone(),
            locator_sha256: digest::sha256_hex(&locator_bytes),
            active: active.evidence,
            candidate: candidate.evidence,
            actions,
            plan_sha256: String::new(),
        };
        plan.plan_sha256 = restore_publication_plan_sha256(&plan).map_err(contract_error)?;
        plan.validate().map_err(contract_error)?;
        Ok(plan)
    }
}
