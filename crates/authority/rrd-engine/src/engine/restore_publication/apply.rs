//! Applying a restore publication stage by stage.

use super::*;

impl RrdEngine {
    /// Applies one previewed publication plan after its digest is accepted.
    /// Every action is durable and idempotent: a rerun after an interruption
    /// resumes from the recorded stage, and a rerun after completion returns
    /// the recorded receipt as a replay.
    pub fn apply_restore_publication(
        project: &Path,
        plan: &RestorePublicationPlan,
        expected_plan_sha256: &str,
        at_unix_ms: u64,
        executable: &Path,
    ) -> Result<RestorePublicationResult> {
        Self::apply_restore_publication_with_observer(
            project,
            plan,
            expected_plan_sha256,
            at_unix_ms,
            executable,
            &mut |_| Ok(()),
        )
    }

    /// Applies the plan and fails once right after `fail_after` durably
    /// advanced, the way a crash at that boundary would.
    #[cfg(test)]
    pub(in crate::engine) fn apply_restore_publication_with_failure(
        project: &Path,
        plan: &RestorePublicationPlan,
        expected_plan_sha256: &str,
        at_unix_ms: u64,
        executable: &Path,
        fail_after: PublicationStage,
    ) -> Result<RestorePublicationResult> {
        Self::apply_restore_publication_with_observer(
            project,
            plan,
            expected_plan_sha256,
            at_unix_ms,
            executable,
            &mut |stage| {
                if stage == fail_after {
                    return Err(ServiceError::Storage(format!(
                        "test publication failpoint after {stage:?}"
                    )));
                }
                Ok(())
            },
        )
    }

    pub(super) fn apply_restore_publication_with_observer(
        project: &Path,
        plan: &RestorePublicationPlan,
        expected_plan_sha256: &str,
        at_unix_ms: u64,
        executable: &Path,
        observer: &mut dyn FnMut(PublicationStage) -> Result<()>,
    ) -> Result<RestorePublicationResult> {
        plan.validate().map_err(contract_error)?;
        if at_unix_ms == 0 || plan.plan_sha256 != expected_plan_sha256 {
            return Err(ServiceError::Contract(
                "restore publication apply time or expected digest does not match the plan".into(),
            ));
        }
        let (project, locator, locator_bytes) = read_locator(project)?;
        let executable_sha256 = require_distribution_binding(&locator, executable)?;
        if executable_sha256 != plan.executable_sha256
            || locator.identity != plan.target
            || locator.product_version != plan.product_version
            || locator.storage_root != plan.active_root
            || locator.installed_record_sha256 != plan.installed_record_sha256
            || digest::sha256_hex(&locator_bytes) != plan.locator_sha256
            || storage_root_id(&locator)? != plan.storage_root_id
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let paths = publication_paths(&project, &locator, &plan.restore_id)?;
        if relative(&paths.project, &paths.candidate)? != plan.candidate_root
            || relative(&paths.project, &paths.quarantine)? != plan.quarantine_root
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let _lease = PublicationLease::acquire(&paths)?;

        let mut intent = match paths.intent.metadata() {
            Ok(_) => {
                let intent = read_intent(&paths.intent)?;
                intent.validate(plan)?;
                intent
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if let Some(receipt) = published_receipt(&paths, &locator, plan)? {
                    return replay_result(&paths, plan, &receipt, executable);
                }
                let intent = PublicationIntent::new(&plan.plan_sha256, at_unix_ms)?;
                write_private_new(&paths.intent, &encode(&intent)?).map_err(storage_error)?;
                sync_parent(&paths.intent)?;
                intent
            }
            Err(error) => return Err(storage_error(error)),
        };
        let mut outputs = Vec::with_capacity(plan.actions.len());

        // VerifyCandidate: both roots must still be exactly what the plan saw,
        // and the candidate must still be the authenticated restore of the
        // backup. Reading them also proves no service holds the active root.
        if intent.stage < PublicationStage::CandidateVerified {
            require_directory(&paths.candidate, "restore candidate root")?;
            require_directory(&paths.active, "active storage root")?;
            let active = RootReading::open(&paths.active, &locator)?;
            let candidate = RootReading::open(&paths.candidate, &locator)?;
            if active.evidence != plan.active || candidate.evidence != plan.candidate {
                return Err(ServiceError::Contract(
                    "a storage root changed since the publication plan; plan again".into(),
                ));
            }
            require_locator_record_match(&locator, &active.installed)?;
            if candidate.installed != active.installed
                || active.installed.record_sha256 != plan.installed_record_sha256
                || candidate.configuration != active.configuration
            {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            catalogued_backup(&paths.backups, &plan.backup_sha256)?;
            rrd_store::verify_restored_backup_objects_with_options(
                &paths.backups,
                &plan.backup_sha256,
                &paths.candidate,
                super::super::core::installed_storage_options(&locator.storage)?,
            )?;
            if active_token_key_sha256(&paths.active, &active.installed)? != plan.token_key_sha256 {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            advance(
                &paths.intent,
                &mut intent,
                PublicationStage::CandidateVerified,
            )?;
            observer(PublicationStage::CandidateVerified)?;
        }
        outputs.push(json_sha256(&(
            &plan.backup_sha256,
            &plan.candidate.physical_sha256,
        ))?);

        // CarryTokenKey: the backup never carries the token key, so the
        // candidate receives the active root's key before it can be opened.
        if intent.stage < PublicationStage::TokenCarried {
            let token_key =
                read_token_key(&paths.active.join(TOKEN_KEY_FILE)).map_err(storage_error)?;
            if digest::sha256_hex(&token_key) != plan.token_key_sha256 {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            match candidate_token_key_state(&paths.candidate, &plan.token_key_sha256)? {
                TokenKeyState::Present => {}
                TokenKeyState::Absent => {
                    write_private_new(&paths.candidate.join(TOKEN_KEY_FILE), &token_key)
                        .map_err(storage_error)?;
                    sync_directory(&paths.candidate)?;
                }
            }
            advance(&paths.intent, &mut intent, PublicationStage::TokenCarried)?;
            observer(PublicationStage::TokenCarried)?;
        }
        outputs.push(plan.token_key_sha256.clone());

        // QuarantineActiveRoot: the active root moves aside under a name that
        // records which restore replaced it. It is never overwritten.
        if intent.stage < PublicationStage::ActiveQuarantined {
            match (
                paths.active.metadata().is_ok(),
                paths.quarantine.metadata().is_ok(),
            ) {
                (true, false) => {
                    reject_existing_symlink_path(&paths.project, &paths.quarantine)?;
                    paths
                        .active
                        .rename_new_to(&paths.quarantine)
                        .map_err(storage_error)?;
                    sync_parent(&paths.active)?;
                    sync_parent(&paths.quarantine)?;
                }
                (false, true) => {}
                _ => return Err(ServiceError::ProjectBindingMismatch),
            }
            advance(
                &paths.intent,
                &mut intent,
                PublicationStage::ActiveQuarantined,
            )?;
            observer(PublicationStage::ActiveQuarantined)?;
        }
        outputs.push(plan.active.physical_sha256.clone());

        // PublishCandidateRoot: the candidate takes the storage root's name,
        // so the locator selects it without being rewritten.
        if intent.stage < PublicationStage::CandidatePublished {
            match (
                paths.candidate.metadata().is_ok(),
                paths.active.metadata().is_ok(),
            ) {
                (true, false) => {
                    paths
                        .candidate
                        .rename_new_to(&paths.active)
                        .map_err(storage_error)?;
                    sync_parent(&paths.candidate)?;
                    sync_parent(&paths.active)?;
                }
                (false, true) => {}
                _ => return Err(ServiceError::ProjectBindingMismatch),
            }
            advance(
                &paths.intent,
                &mut intent,
                PublicationStage::CandidatePublished,
            )?;
            observer(PublicationStage::CandidatePublished)?;
        }
        outputs.push(plan.candidate.physical_sha256.clone());

        // OpenPublishedEstate: the published root opens through the installed
        // engine path, records the receipt, and passes quick verification.
        // The receipt commit below changes the root's bytes, so a resumed run
        // checks the published root by its semantic watermarks and identity.
        let published = RootReading::open(&paths.active, &locator)?;
        if published.evidence.runtime_cursor != plan.candidate.runtime_cursor
            || published.evidence.claim_sequence != plan.candidate.claim_sequence
            || published.installed.record_sha256 != plan.installed_record_sha256
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        drop(published);
        let receipt = PublicationReceipt {
            format_version: PUBLICATION_RECEIPT_FORMAT,
            plan_id: plan.id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            restore_id: plan.restore_id.clone(),
            backup_sha256: plan.backup_sha256.clone(),
            published_root: plan.active_root.clone(),
            quarantine_root: plan.quarantine_root.clone(),
            locator_sha256: plan.locator_sha256.clone(),
            action_outputs: {
                let mut all = outputs.clone();
                all.push(plan.installed_record_sha256.clone());
                all
            },
            applied_at_unix_ms: intent.started_at_unix_ms,
        };
        let receipt_key = publication_receipt_key(&locator.identity.instance_id, &plan.id);
        let receipt_bytes = encode(&receipt)?;
        let engine = RrdEngine::open_installed_anchored(
            &paths.project,
            executable,
            &super::super::clock::HostSystemClock,
        )?;
        match engine.storage.control().get(&receipt_key)? {
            Some(existing) if existing == receipt_bytes => {}
            Some(_) => return Err(ServiceError::IdempotencyConflict),
            None => {
                engine.storage.control().commit(&ControlTransition {
                    key: receipt_key,
                    expected: None,
                    replacement: Some(receipt_bytes),
                    at: at_unix_ms,
                    actor: "rrflow:restore-publication".into(),
                    action: "restore.published".into(),
                    request_id: format!("restore-publication-{}", &plan.plan_sha256[..16]),
                    operation_id: format!("restore-publication-{}", &plan.plan_sha256[..16]),
                })?;
            }
        }
        drop(engine);
        let report = RrdEngine::inspect_installed_anchored(
            &paths.project,
            executable,
            &super::super::clock::HostSystemClock,
            "restore.publication.verify",
        )?;
        if report.status != InstallationVerificationStatus::Passed {
            return Err(ServiceError::Storage(
                "published estate failed quick verification".into(),
            ));
        }
        let result = build_result(
            plan,
            &receipt,
            &report.runtime_manifest_sha256,
            report.runtime_cursor,
            report.control_journal_sequence,
            false,
        )?;
        acknowledge_intent(&paths.intent, &intent)?;
        Ok(result)
    }
}
