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

mod apply;
mod plan;
mod support;

pub(in crate::engine) use support::*;

use super::estate_layout::{
    instance_operation_root, reject_existing_symlink_path, safe_join, EstateFilesystem,
    BACKUPS_DIRECTORY_NAME, RESTORES_DIRECTORY_NAME, STORAGE_DIRECTORY,
};

use super::installation::{
    canonical_prefixed, decode_control, engram_configuration_key, json_sha256, read_locator,
    require_locator_record_match, runtime_manifest_from_inspection, sha256_file,
    DefaultInstallProfile, InstallationVerificationStatus, InstalledEstateRecord, ProjectLocator,
    INSTALLED_RECORD_KEY,
};

use super::token_key::{
    read_private_anchored as read_private_bounded, read_token_key_anchored as read_token_key,
    write_private_anchored as write_private_new, TOKEN_KEY_BYTES,
};

use super::*;

use rrd_contract::{
    restore_publication_plan_sha256, restore_publication_result_sha256, RestorePublicationAction,
    RestorePublicationActionKind, RestorePublicationActionResult, RestorePublicationPlan,
    RestorePublicationResult, RestoreRootEvidence, INSTALL_ATTUNEMENT_CONTRACT_VERSION,
    RESTORE_PUBLICATION_ACTIONS,
};

use rrd_store::filesystem::{AnchoredPath, OpenOptions};

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
pub(super) struct PublicationLease {
    _file: fs::File,
}

impl Drop for PublicationLease {
    fn drop(&mut self) {
        // File close alone leaves the lock held by duplicated/inherited file
        // descriptions (including a concurrent fork before exec). Ownership
        // ends here, after publication unwinds, not when the last duplicate closes.
        let _ = self._file.unlock();
    }
}

#[cfg(test)]
mod publication_lease_tests {
    use super::*;

    #[test]
    fn publication_owner_drop_releases_even_with_a_duplicated_handle() {
        let root = tempfile::tempdir().unwrap();
        let anchor = AnchoredPath::open_root(root.path()).unwrap();
        let owner = PublicationLease::acquire_at(&anchor).unwrap();
        let duplicate = owner._file.try_clone().unwrap();
        assert!(matches!(
            PublicationLease::acquire_at(&anchor),
            Err(ServiceError::StorageConflict(_))
        ));
        drop(owner);
        let replacement = PublicationLease::acquire_at(&anchor).unwrap();
        drop(duplicate);
        // Disposing the former descriptor must not release the new owner's lock.
        assert!(matches!(
            PublicationLease::acquire_at(&anchor),
            Err(ServiceError::StorageConflict(_))
        ));
        drop(replacement);
        let _next = PublicationLease::acquire_at(&anchor).unwrap();
    }
}

impl PublicationLease {
    fn acquire(paths: &PublicationPaths) -> Result<Self> {
        let path = paths.restores.join(PUBLICATION_LEASE_FILE);
        reject_existing_symlink_path(&paths.project, &path)?;
        Self::acquire_at(&paths.restores)
    }

    pub(super) fn acquire_at(restores: &AnchoredPath) -> Result<Self> {
        let path = restores.join(PUBLICATION_LEASE_FILE);
        let file = OpenOptions::new()
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
    project: EstateFilesystem,
    active: AnchoredPath,
    candidate: AnchoredPath,
    quarantine: AnchoredPath,
    intent: AnchoredPath,
    restores: AnchoredPath,
    backups: AnchoredPath,
}

/// Read-only evidence and installed control state of one storage root.
struct RootReading {
    evidence: RestoreRootEvidence,
    installed: InstalledEstateRecord,
    configuration: EstateConfiguration,
}

impl RootReading {
    fn open(root: &AnchoredPath, locator: &ProjectLocator) -> Result<Self> {
        super::core::validate_existing_object_layout(root)?;
        let inspector = RrflowKvInspector::open_anchored_with_options(
            root,
            super::core::installed_storage_options(&locator.storage)?,
        )?;
        let installed: InstalledEstateRecord =
            decode_control(&inspector, INSTALLED_RECORD_KEY, "installed estate record")?;
        installed.validate()?;
        let configuration: EstateConfiguration = decode_control(
            &inspector,
            &engram_configuration_key(&locator.identity.instance_id),
            "estate configuration",
        )?;
        configuration.validate().map_err(contract_error)?;
        if configuration.storage != locator.storage {
            return Err(ServiceError::ProjectBindingMismatch);
        }
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
