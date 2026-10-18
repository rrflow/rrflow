//! Restore publication: promoting a verified restore staging root into the
//! installed estate's storage root.
//!
//! The candidate keeps the storage root's own name, so the installed record,
//! the locator, and the single-root install-recovery rule stay as they are;
//! the previous root is quarantined beside the restore staging roots and is
//! never overwritten. Publication is offline: both roots are read through the
//! read-only inspector, which refuses a root that a running service holds.
//! Apply records a durable intent beside the candidate and advances it stage
//! by stage, so an interrupted publication resumes without repeating or
//! skipping an action; the published root's control journal carries the
//! publication receipt.

use super::estate_layout::{
    reject_existing_symlink_path, safe_join, BACKUPS_DIRECTORY_NAME, RESTORES_DIRECTORY_NAME,
    STORAGE_DIRECTORY,
};
use super::installation::{
    canonical_prefixed, decode_control, estate_configuration_key, json_sha256, read_locator,
    read_regular_bounded, require_locator_record_match, runtime_manifest_from_inspection,
    sha256_file, DefaultInstallProfile, InstallationVerificationStatus, InstalledEstateRecord,
    ProjectLocator, INSTALLED_RECORD_KEY,
};
use super::token_key::{private_regular_file, read_token_key, write_private_new, TOKEN_KEY_BYTES};
use super::*;
use rrd_contract::{
    restore_publication_plan_sha256, restore_publication_result_sha256, RestorePublicationAction,
    RestorePublicationActionKind, RestorePublicationActionResult, RestorePublicationPlan,
    RestorePublicationResult, RestoreRootEvidence, INSTALL_ATTUNEMENT_CONTRACT_VERSION,
    RESTORE_PUBLICATION_ACTIONS,
};
use rrd_store::RrflowKvInspector;
use std::fs;

const PUBLICATION_INTENT_FORMAT: u16 = 1;
const PUBLICATION_RECEIPT_FORMAT: u16 = 1;
const MAX_PUBLICATION_INTENT_BYTES: u64 = 16 * 1024;
const TOKEN_KEY_FILE: &str = "RRD.TOKEN";
const PUBLICATION_LEASE_FILE: &str = ".publication.lock";

/// The exclusive offline publication lease of one instance: an advisory lock
/// on a file beside the restore staging roots, held for the whole apply so two
/// publications cannot interleave their renames.
struct PublicationLease {
    _file: fs::File,
}

impl PublicationLease {
    fn acquire(paths: &PublicationPaths) -> Result<Self> {
        let path = paths.restores.join(PUBLICATION_LEASE_FILE);
        reject_existing_symlink_path(&paths.project, &path)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(storage_error)?;
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(std::fs::TryLockError::WouldBlock) => Err(ServiceError::StorageConflict(
                "another restore publication holds this instance's publication lease".into(),
            )),
            Err(std::fs::TryLockError::Error(error)) => Err(storage_error(error)),
        }
    }
}

/// The durable stages of one publication. Each stage is idempotent against
/// the observable filesystem state it produces, so a rerun after a crash
/// re-checks rather than repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::engine) enum PublicationStage {
    IntentPublished,
    CandidateVerified,
    TokenCarried,
    ActiveQuarantined,
    CandidatePublished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicationIntent {
    format_version: u16,
    stage: PublicationStage,
    plan_sha256: String,
    started_at_unix_ms: u64,
    intent_sha256: String,
}

impl PublicationIntent {
    fn new(plan_sha256: &str, started_at_unix_ms: u64) -> Result<Self> {
        let mut intent = Self {
            format_version: PUBLICATION_INTENT_FORMAT,
            stage: PublicationStage::IntentPublished,
            plan_sha256: plan_sha256.into(),
            started_at_unix_ms,
            intent_sha256: String::new(),
        };
        intent.intent_sha256 = intent.calculated_sha256()?;
        Ok(intent)
    }

    fn calculated_sha256(&self) -> Result<String> {
        json_sha256(&(
            self.format_version,
            self.stage,
            &self.plan_sha256,
            self.started_at_unix_ms,
        ))
    }

    fn validate(&self, plan: &RestorePublicationPlan) -> Result<()> {
        if self.format_version != PUBLICATION_INTENT_FORMAT
            || self.plan_sha256 != plan.plan_sha256
            || self.started_at_unix_ms == 0
            || self.intent_sha256 != self.calculated_sha256()?
        {
            return Err(ServiceError::Contract(
                "a restore publication is in progress under another plan or a corrupt intent"
                    .into(),
            ));
        }
        Ok(())
    }
}

/// The receipt an applied publication leaves in the published root's control
/// journal. It is instance-local operation provenance and is not carried by
/// backups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicationReceipt {
    format_version: u16,
    plan_id: CanonicalId,
    plan_sha256: String,
    restore_id: CanonicalId,
    backup_sha256: String,
    published_root: String,
    quarantine_root: String,
    locator_sha256: String,
    action_outputs: Vec<String>,
    applied_at_unix_ms: u64,
}

struct PublicationPaths {
    project: PathBuf,
    active: PathBuf,
    candidate: PathBuf,
    quarantine: PathBuf,
    intent: PathBuf,
    restores: PathBuf,
    backups: PathBuf,
}

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
        if fs::symlink_metadata(&paths.intent).is_ok() {
            return Err(ServiceError::Contract(
                "a restore publication is already in progress for this restore; apply its plan"
                    .into(),
            ));
        }
        if fs::symlink_metadata(&paths.quarantine).is_ok() {
            return Err(ServiceError::Contract(
                "the quarantine location for this restore already exists".into(),
            ));
        }
        require_directory(&paths.candidate, "restore candidate root")?;
        require_directory(&paths.active, "active storage root")?;

        let backup = catalogued_backup(&paths.backups, backup_sha256)?;
        rrd_store::verify_restored_backup_objects(&paths.backups, backup_sha256, &paths.candidate)?;
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

    fn apply_restore_publication_with_observer(
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

        let mut intent = match fs::symlink_metadata(&paths.intent) {
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
            rrd_store::verify_restored_backup_objects(
                &paths.backups,
                &plan.backup_sha256,
                &paths.candidate,
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
                fs::symlink_metadata(&paths.active).is_ok(),
                fs::symlink_metadata(&paths.quarantine).is_ok(),
            ) {
                (true, false) => {
                    reject_existing_symlink_path(&paths.project, &paths.quarantine)?;
                    fs::rename(&paths.active, &paths.quarantine).map_err(storage_error)?;
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
                fs::symlink_metadata(&paths.candidate).is_ok(),
                fs::symlink_metadata(&paths.active).is_ok(),
            ) {
                (true, false) => {
                    fs::rename(&paths.candidate, &paths.active).map_err(storage_error)?;
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
        let engine = RrdEngine::open_installed(&paths.project, executable)?;
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
        let report = RrdEngine::inspect_installed(&paths.project, executable)?;
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

/// Read-only evidence and installed control state of one storage root.
struct RootReading {
    evidence: RestoreRootEvidence,
    installed: InstalledEstateRecord,
    configuration: EstateConfiguration,
}

impl RootReading {
    fn open(root: &Path, locator: &ProjectLocator) -> Result<Self> {
        super::core::validate_existing_object_layout(root)?;
        let inspector = RrflowKvInspector::open(root)?;
        let installed: InstalledEstateRecord =
            decode_control(&inspector, INSTALLED_RECORD_KEY, "installed estate record")?;
        installed.validate()?;
        let configuration: EstateConfiguration = decode_control(
            &inspector,
            &estate_configuration_key(&locator.identity.instance_id),
            "estate configuration",
        )?;
        configuration.validate().map_err(contract_error)?;
        let inspection = inspector.inspection();
        let evidence = RestoreRootEvidence {
            physical_sha256: json_sha256(inspection)?,
            runtime_manifest_sha256: runtime_manifest_from_inspection(&inspector, locator)?,
            runtime_cursor: inspection.runtime_cursor,
            claim_sequence: inspection.claim_sequence,
            control_journal_sequence: inspection.control_journal_sequence,
        };
        Ok(Self {
            evidence,
            installed,
            configuration,
        })
    }
}

enum TokenKeyState {
    Present,
    Absent,
}

fn publication_paths(
    project: &Path,
    locator: &ProjectLocator,
    restore_id: &CanonicalId,
) -> Result<PublicationPaths> {
    let instance = locator.identity.instance_id.as_str();
    let restores = format!("{STORAGE_DIRECTORY}/{RESTORES_DIRECTORY_NAME}/{instance}");
    let storage_root_id = storage_root_id(locator)?;
    let candidate = format!("{restores}/{restore_id}");
    let quarantine = format!("{restores}/{restore_id}.quarantined-{storage_root_id}");
    let intent = format!("{restores}/.publish-{restore_id}.json");
    let backups = format!("{STORAGE_DIRECTORY}/{BACKUPS_DIRECTORY_NAME}/{instance}");
    Ok(PublicationPaths {
        project: project.to_owned(),
        active: safe_join(project, &locator.storage_root)?,
        candidate: safe_join(project, &candidate)?,
        quarantine: safe_join(project, &quarantine)?,
        intent: safe_join(project, &intent)?,
        restores: safe_join(project, &restores)?,
        backups: safe_join(project, &backups)?,
    })
}

fn storage_root_id(locator: &ProjectLocator) -> Result<CanonicalId> {
    let name = Path::new(&locator.storage_root)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    CanonicalId::new(name).map_err(|_| ServiceError::ProjectBindingMismatch)
}

fn relative(project: &Path, path: &Path) -> Result<String> {
    path.strip_prefix(project)
        .map_err(|_| ServiceError::ProjectBindingMismatch)?
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .ok_or(ServiceError::ProjectBindingMismatch)
        })
        .collect::<Result<Vec<_>>>()
        .map(|segments| segments.join("/"))
}

fn require_distribution_binding(locator: &ProjectLocator, executable: &Path) -> Result<String> {
    let (_, profile_sha256) = DefaultInstallProfile::load(locator.profile_id.as_str())?;
    let executable_sha256 = sha256_file(executable)?;
    if executable_sha256 != locator.executable_sha256 || profile_sha256 != locator.profile_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(executable_sha256)
}

fn require_directory(path: &Path, label: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(ServiceError::Contract(format!(
            "{label} is not a directory"
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(ServiceError::Contract(format!("{label} is absent")))
        }
        Err(error) => Err(storage_error(error)),
    }
}

fn catalogued_backup(backups: &Path, backup_sha256: &str) -> Result<rrd_store::BackupEntry> {
    let catalogue = rrd_store::verify_backup_catalogue(backups)?;
    let entry = catalogue
        .backups
        .into_iter()
        .find(|entry| entry.backup_id == backup_sha256)
        .ok_or_else(|| ServiceError::Backup("backup is not catalogued".into()))?;
    if !entry.application_complete {
        return Err(ServiceError::Backup(
            "only an application-complete backup can be published".into(),
        ));
    }
    Ok(entry)
}

fn active_token_key_sha256(active: &Path, installed: &InstalledEstateRecord) -> Result<String> {
    let token_key = read_token_key(&active.join(TOKEN_KEY_FILE)).map_err(storage_error)?;
    let token_key_sha256 = digest::sha256_hex(&token_key);
    if token_key_sha256 != installed.token_key_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(token_key_sha256)
}

fn candidate_token_key_state(candidate: &Path, expected_sha256: &str) -> Result<TokenKeyState> {
    let path = candidate.join(TOKEN_KEY_FILE);
    match fs::symlink_metadata(&path) {
        Ok(_) => {
            private_regular_file(&path).map_err(storage_error)?;
            let token_key = read_token_key(&path).map_err(storage_error)?;
            if digest::sha256_hex(&token_key) != expected_sha256 {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            Ok(TokenKeyState::Present)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(TokenKeyState::Absent),
        Err(error) => Err(storage_error(error)),
    }
}

fn publication_receipt_key(instance: &CanonicalId, plan_id: &CanonicalId) -> String {
    format!("server/state/{instance}/restore/publication/{plan_id}")
}

/// A completed publication whose intent was acknowledged leaves the candidate
/// absent, the quarantine present, and the receipt in the published root.
fn published_receipt(
    paths: &PublicationPaths,
    locator: &ProjectLocator,
    plan: &RestorePublicationPlan,
) -> Result<Option<PublicationReceipt>> {
    if fs::symlink_metadata(&paths.candidate).is_ok()
        || fs::symlink_metadata(&paths.quarantine).is_err()
    {
        return Ok(None);
    }
    let inspector = RrflowKvInspector::open(&paths.active)?;
    let key = publication_receipt_key(&locator.identity.instance_id, &plan.id);
    let Some(bytes) = inspector.control_record(&key)? else {
        return Ok(None);
    };
    let receipt: PublicationReceipt = serde_json::from_slice(&bytes).map_err(contract_json)?;
    if receipt.format_version != PUBLICATION_RECEIPT_FORMAT
        || receipt.plan_sha256 != plan.plan_sha256
        || receipt.plan_id != plan.id
    {
        return Err(ServiceError::IdempotencyConflict);
    }
    Ok(Some(receipt))
}

/// A replay re-verifies the published estate the way the first application
/// did and reports its current coordinates under the recorded receipt.
fn replay_result(
    paths: &PublicationPaths,
    plan: &RestorePublicationPlan,
    receipt: &PublicationReceipt,
    executable: &Path,
) -> Result<RestorePublicationResult> {
    let report = RrdEngine::inspect_installed(&paths.project, executable)?;
    if report.status != InstallationVerificationStatus::Passed {
        return Err(ServiceError::Storage(
            "published estate failed quick verification".into(),
        ));
    }
    build_result(
        plan,
        receipt,
        &report.runtime_manifest_sha256,
        report.runtime_cursor,
        report.control_journal_sequence,
        true,
    )
}

fn build_result(
    plan: &RestorePublicationPlan,
    receipt: &PublicationReceipt,
    runtime_manifest_sha256: &str,
    runtime_cursor: u64,
    control_journal_sequence: u64,
    idempotent_replay: bool,
) -> Result<RestorePublicationResult> {
    if receipt.action_outputs.len() != plan.actions.len() {
        return Err(ServiceError::Contract(
            "publication receipt does not cover every planned action".into(),
        ));
    }
    let mut result = RestorePublicationResult {
        contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
        plan_id: plan.id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        action_results: plan
            .actions
            .iter()
            .zip(&receipt.action_outputs)
            .map(|(action, output_sha256)| RestorePublicationActionResult {
                kind: action.kind,
                output_sha256: output_sha256.clone(),
            })
            .collect(),
        published_root: receipt.published_root.clone(),
        quarantine_root: receipt.quarantine_root.clone(),
        locator_sha256: receipt.locator_sha256.clone(),
        runtime_manifest_sha256: runtime_manifest_sha256.into(),
        runtime_cursor,
        control_journal_sequence,
        applied_at_unix_ms: receipt.applied_at_unix_ms,
        idempotent_replay,
        result_sha256: String::new(),
    };
    result.result_sha256 = restore_publication_result_sha256(&result).map_err(contract_error)?;
    result.validate_for(plan).map_err(contract_error)?;
    Ok(result)
}

fn advance(path: &Path, intent: &mut PublicationIntent, stage: PublicationStage) -> Result<()> {
    intent.stage = stage;
    intent.intent_sha256 = intent.calculated_sha256()?;
    let bytes = encode(intent)?;
    let temporary = path.with_extension("json.pending");
    let _ = fs::remove_file(&temporary);
    write_private_new(&temporary, &bytes).map_err(storage_error)?;
    let parent = path.parent().ok_or(ServiceError::ProjectBindingMismatch)?;
    rrd_store::publish_durable_rename(parent, &temporary, path).map_err(storage_error)?;
    if read_intent(path)? != *intent {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

fn acknowledge_intent(path: &Path, expected: &PublicationIntent) -> Result<()> {
    if read_intent(path)? != *expected {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    fs::remove_file(path).map_err(storage_error)?;
    sync_parent(path)
}

fn read_intent(path: &Path) -> Result<PublicationIntent> {
    private_regular_file(path).map_err(storage_error)?;
    let bytes = read_regular_bounded(path, MAX_PUBLICATION_INTENT_BYTES)?;
    let intent: PublicationIntent = serde_json::from_slice(&bytes).map_err(contract_json)?;
    if encode(&intent)? != bytes || intent.intent_sha256 != intent.calculated_sha256()? {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(intent)
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(contract_json)
}

fn sync_parent(path: &Path) -> Result<()> {
    sync_directory(path.parent().ok_or(ServiceError::ProjectBindingMismatch)?)
}

fn sync_directory(path: &Path) -> Result<()> {
    rrd_store::sync_directory_metadata(path).map_err(storage_error)
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn contract_error(error: rrd_contract::ContractError) -> ServiceError {
    ServiceError::Contract(error.to_string())
}

fn storage_error(error: std::io::Error) -> ServiceError {
    ServiceError::Storage(error.to_string())
}
