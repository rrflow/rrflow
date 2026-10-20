//! Restore publication contracts: promoting one verified restore staging root
//! into an installed estate's storage root.
//!
//! Publication is an offline lifecycle operation in the plan/apply shape of
//! installation: a deterministic, digest-bound plan binds the candidate, the
//! active root it replaces, and the backup that produced the candidate; apply
//! executes only that plan after the operator accepts its digest. `RrdEngine`
//! is the sole authority that performs the mutation.

use crate::attunement::{
    digest_json, validate_bounded_text, validate_contract_version, validate_project_relative_path,
};
use crate::{invalid, validate_sha256, CanonicalId, InstalledEstateIdentity, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_RESTORE_PUBLICATION_ACTIONS: usize = 8;

/// The canonical publication actions, in execution order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RestorePublicationActionKind {
    /// Re-verify the candidate against its backup and the plan's evidence.
    VerifyCandidate,
    /// Copy the estate's token key into the candidate; a backup never carries it.
    CarryTokenKey,
    /// Rename the active storage root into its quarantine location.
    QuarantineActiveRoot,
    /// Rename the candidate into the storage root's own name.
    PublishCandidateRoot,
    /// Open the published root through the installed engine path and record
    /// the publication receipt in its control journal.
    OpenPublishedEstate,
}

pub const RESTORE_PUBLICATION_ACTIONS: [RestorePublicationActionKind; 5] = [
    RestorePublicationActionKind::VerifyCandidate,
    RestorePublicationActionKind::CarryTokenKey,
    RestorePublicationActionKind::QuarantineActiveRoot,
    RestorePublicationActionKind::PublishCandidateRoot,
    RestorePublicationActionKind::OpenPublishedEstate,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestorePublicationAction {
    pub kind: RestorePublicationActionKind,
    pub input_sha256: String,
    pub estimated_write_bytes: u64,
}

impl RestorePublicationAction {
    fn validate(&self) -> Result<()> {
        validate_sha256(
            &self.input_sha256,
            "restore publication action input_sha256",
        )
    }
}

/// Read-only evidence about one rrflowKV storage root at planning time. Apply
/// re-derives it and refuses a root that changed since the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreRootEvidence {
    pub physical_sha256: String,
    pub runtime_manifest_sha256: String,
    pub runtime_cursor: u64,
    pub claim_sequence: u64,
    pub control_journal_sequence: u64,
}

impl RestoreRootEvidence {
    fn validate(&self, label: &str) -> Result<()> {
        validate_sha256(&self.physical_sha256, &format!("{label} physical_sha256"))?;
        validate_sha256(
            &self.runtime_manifest_sha256,
            &format!("{label} runtime_manifest_sha256"),
        )
    }
}

/// A deterministic preview of restore publication. Applying this plan must
/// use the same digest; implementations cannot silently re-plan during
/// mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestorePublicationPlan {
    pub contract_version: u16,
    pub id: CanonicalId,
    pub target: InstalledEstateIdentity,
    pub product_version: String,
    pub executable_sha256: String,
    pub restore_id: CanonicalId,
    pub backup_sha256: String,
    pub backup_archive_sha256: String,
    pub backup_control_archive_sha256: String,
    pub storage_root_id: CanonicalId,
    pub active_root: String,
    pub candidate_root: String,
    pub quarantine_root: String,
    pub token_key_sha256: String,
    pub installed_record_sha256: String,
    pub locator_sha256: String,
    pub active: RestoreRootEvidence,
    pub candidate: RestoreRootEvidence,
    pub actions: Vec<RestorePublicationAction>,
    pub plan_sha256: String,
}

impl RestorePublicationPlan {
    pub fn validate(&self) -> Result<()> {
        validate_contract_version(self.contract_version)?;
        self.target.validate()?;
        validate_bounded_text(
            &self.product_version,
            "restore publication product_version",
            64,
        )?;
        validate_sha256(
            &self.executable_sha256,
            "restore publication executable_sha256",
        )?;
        validate_sha256(&self.backup_sha256, "restore publication backup_sha256")?;
        validate_sha256(
            &self.backup_archive_sha256,
            "restore publication backup_archive_sha256",
        )?;
        validate_sha256(
            &self.backup_control_archive_sha256,
            "restore publication backup_control_archive_sha256",
        )?;
        for (path, label) in [
            (&self.active_root, "active"),
            (&self.candidate_root, "candidate"),
            (&self.quarantine_root, "quarantine"),
        ] {
            validate_project_relative_path(path)
                .map_err(|error| crate::ContractError(format!("{label} root: {error}")))?;
        }
        if self.active_root == self.candidate_root
            || self.active_root == self.quarantine_root
            || self.candidate_root == self.quarantine_root
        {
            return invalid("restore publication roots must be three distinct paths");
        }
        if !self
            .active_root
            .ends_with(&format!("/{}", self.storage_root_id))
        {
            return invalid("restore publication active root must be the storage root it names");
        }
        validate_sha256(
            &self.token_key_sha256,
            "restore publication token_key_sha256",
        )?;
        validate_sha256(
            &self.installed_record_sha256,
            "restore publication installed_record_sha256",
        )?;
        validate_sha256(&self.locator_sha256, "restore publication locator_sha256")?;
        self.active.validate("restore publication active root")?;
        self.candidate
            .validate("restore publication candidate root")?;
        if self.actions.len() != RESTORE_PUBLICATION_ACTIONS.len()
            || self.actions.len() > MAX_RESTORE_PUBLICATION_ACTIONS
        {
            return invalid(
                "restore publication plan must contain every canonical action exactly once",
            );
        }
        for (action, expected) in self.actions.iter().zip(RESTORE_PUBLICATION_ACTIONS) {
            action.validate()?;
            if action.kind != expected {
                return invalid(
                    "restore publication actions are missing, duplicated, or out of order",
                );
            }
        }
        validate_sha256(&self.plan_sha256, "restore publication plan_sha256")?;
        if self.plan_sha256 != restore_publication_plan_sha256(self)? {
            return invalid("restore publication plan digest does not match its content");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestorePublicationActionResult {
    pub kind: RestorePublicationActionKind,
    pub output_sha256: String,
}

/// The durable receipt returned after a previewed publication plan is applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestorePublicationResult {
    pub contract_version: u16,
    pub plan_id: CanonicalId,
    pub plan_sha256: String,
    pub action_results: Vec<RestorePublicationActionResult>,
    pub published_root: String,
    pub quarantine_root: String,
    pub locator_sha256: String,
    pub runtime_manifest_sha256: String,
    pub runtime_cursor: u64,
    pub control_journal_sequence: u64,
    pub applied_at_unix_ms: u64,
    pub idempotent_replay: bool,
    pub result_sha256: String,
}

impl RestorePublicationResult {
    pub fn validate_for(&self, plan: &RestorePublicationPlan) -> Result<()> {
        plan.validate()?;
        validate_contract_version(self.contract_version)?;
        if self.plan_id != plan.id || self.plan_sha256 != plan.plan_sha256 {
            return invalid(
                "restore publication result does not bind its plan identity and digest",
            );
        }
        if self.action_results.len() != plan.actions.len() {
            return invalid("restore publication result does not cover every planned action");
        }
        for (result, action) in self.action_results.iter().zip(&plan.actions) {
            validate_sha256(
                &result.output_sha256,
                "restore publication action result output_sha256",
            )?;
            if result.kind != action.kind {
                return invalid("restore publication result actions do not match plan order");
            }
        }
        if self.published_root != plan.active_root || self.quarantine_root != plan.quarantine_root {
            return invalid("restore publication result roots do not match the plan");
        }
        validate_sha256(
            &self.locator_sha256,
            "restore publication result locator_sha256",
        )?;
        validate_sha256(
            &self.runtime_manifest_sha256,
            "restore publication result runtime_manifest_sha256",
        )?;
        if self.runtime_cursor != plan.candidate.runtime_cursor {
            return invalid("restore publication result runtime cursor differs from the candidate");
        }
        if self.applied_at_unix_ms == 0 {
            return invalid("restore publication result applied_at_unix_ms must be nonzero");
        }
        validate_sha256(&self.result_sha256, "restore publication result_sha256")?;
        if self.result_sha256 != restore_publication_result_sha256(self)? {
            return invalid("restore publication result digest does not match its content");
        }
        Ok(())
    }
}

pub fn restore_publication_plan_sha256(plan: &RestorePublicationPlan) -> Result<String> {
    digest_json(
        b"rrflow-restore-publication-plan-v1",
        &(
            (
                plan.contract_version,
                &plan.id,
                &plan.target,
                &plan.product_version,
                &plan.executable_sha256,
                &plan.restore_id,
                &plan.backup_sha256,
                &plan.backup_archive_sha256,
                &plan.backup_control_archive_sha256,
                &plan.storage_root_id,
            ),
            (
                &plan.active_root,
                &plan.candidate_root,
                &plan.quarantine_root,
                &plan.token_key_sha256,
                &plan.installed_record_sha256,
                &plan.locator_sha256,
                &plan.active,
                &plan.candidate,
                &plan.actions,
            ),
        ),
    )
}

pub fn restore_publication_result_sha256(result: &RestorePublicationResult) -> Result<String> {
    digest_json(
        b"rrflow-restore-publication-result-v1",
        &(
            result.contract_version,
            &result.plan_id,
            &result.plan_sha256,
            &result.action_results,
            &result.published_root,
            &result.quarantine_root,
            &result.locator_sha256,
            &result.runtime_manifest_sha256,
            result.runtime_cursor,
            result.control_journal_sequence,
            result.applied_at_unix_ms,
            result.idempotent_replay,
        ),
    )
}
