//! Explicit installed recovery over the native validated prefix. Neither serve
//! nor verify can invoke this effect. The project locator remains the authority.
use super::estate_layout::safe_join;
use super::installation::{
    estate_configuration_key, json_sha256, read_locator, require_locator_record_match, sha256_file,
    InstalledEstateRecord, INSTALLED_RECORD_KEY,
};
use super::token_key::{read_api_key_document_anchored, read_token_key_anchored};
use super::*;
use rrd_contract::EstateConfiguration;
use rrd_store::{RrflowKvTailRepair, TailRepairPlan, TailRepairReceipt};
use serde::de::DeserializeOwned;
pub(in crate::engine) mod publication;
pub use publication::RepairPublication;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledTailRepairPlan {
    pub format_version: u16,
    pub target: InstalledEstateIdentity,
    pub locator_sha256: String,
    pub executable_sha256: String,
    pub configuration_sha256: String,
    pub installed_record_sha256: String,
    pub storage: TailRepairPlan,
    pub publication: RepairPublication,
    pub plan_sha256: String,
}

impl InstalledTailRepairPlan {
    fn calculated_sha256(&self) -> Result<String> {
        json_sha256(&(
            "rrflow-installed-tail-repair-v1",
            self.format_version,
            &self.target,
            &self.locator_sha256,
            &self.executable_sha256,
            &self.configuration_sha256,
            &self.installed_record_sha256,
            &self.storage,
            &self.publication,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledTailRepairReceipt {
    pub plan_sha256: String,
    pub target: InstalledEstateIdentity,
    pub runtime_cursor: u64,
    pub claim_sequence: u64,
    pub storage: TailRepairReceipt,
    pub quarantine_root: String,
    pub before_sha256: String,
    pub candidate_sha256: String,
    pub idempotent_replay: bool,
}

struct BoundRepair {
    repair: RrflowKvTailRepair,
    target: InstalledEstateIdentity,
    locator_sha256: String,
    executable_sha256: String,
    configuration_sha256: String,
    installed_record_sha256: String,
}

impl RrdEngine {
    /// Mutation-free preview, requiring a stopped estate and a positively
    /// classified incomplete final frame. Complete corruption is never repairable.
    pub fn plan_installed_tail_repair(
        project: &Path,
        executable: &Path,
    ) -> Result<InstalledTailRepairPlan> {
        let bound = BoundRepair::open(project, executable)?;
        let publication = publication::preview(project, &bound)?;
        let mut plan = InstalledTailRepairPlan {
            format_version: 1,
            target: bound.target,
            locator_sha256: bound.locator_sha256,
            executable_sha256: bound.executable_sha256,
            configuration_sha256: bound.configuration_sha256,
            installed_record_sha256: bound.installed_record_sha256,
            storage: bound.repair.plan()?,
            publication,
            plan_sha256: String::new(),
        };
        plan.plan_sha256 = plan.calculated_sha256()?;
        if serde_json::to_vec(&plan)
            .map_err(|e| ServiceError::Contract(e.to_string()))?
            .len()
            > 8 * 1024 * 1024
        {
            return Err(ServiceError::Contract(
                "repair plan exceeds bounded artifact size".into(),
            ));
        }
        Ok(plan)
    }

    /// Applies only the exact explicitly approved plan. A repeated application
    /// publishes a repaired candidate while preserving the original root.
    pub fn apply_installed_tail_repair(
        project: &Path,
        executable: &Path,
        plan: &InstalledTailRepairPlan,
        expected_sha256: &str,
    ) -> Result<InstalledTailRepairReceipt> {
        if plan.format_version != 1
            || plan.plan_sha256 != expected_sha256
            || plan.plan_sha256 != plan.calculated_sha256()?
        {
            return Err(ServiceError::Contract(
                "repair requires the exact plan digest".into(),
            ));
        }
        publication::apply(project, executable, plan)
    }
}

impl BoundRepair {
    fn open(project: &Path, executable: &Path) -> Result<Self> {
        let (project, locator, locator_bytes) = read_locator(project)?;
        let executable_sha256 = sha256_file(executable)?;
        if executable_sha256 != locator.executable_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let root = safe_join(&project, &locator.storage_root)?;
        super::core::validate_existing_object_layout(&root)?;
        let repair = RrflowKvTailRepair::open_anchored(
            &root,
            super::core::installed_storage_options(&locator.storage)?,
        )?;
        let installed: InstalledEstateRecord = decode(&repair, INSTALLED_RECORD_KEY)?;
        installed.validate()?;
        require_locator_record_match(&locator, &installed)?;
        let configuration: EstateConfiguration = decode(
            &repair,
            &estate_configuration_key(&locator.identity.instance_id),
        )?;
        configuration
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let token = read_token_key_anchored(&safe_join(&project, &locator.token_key)?)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        let credential =
            read_api_key_document_anchored(&safe_join(&project, &locator.operator_credential)?)
                .map_err(|e| ServiceError::Storage(e.to_string()))?;
        if configuration.storage != locator.storage
            || configuration.revision != locator.configuration_revision
            || configuration.configuration_sha256 != locator.configuration_sha256
            || digest::sha256_hex(&token) != installed.token_key_sha256
            || credential.plan_sha256 != installed.installation.plan_sha256
            || credential.principal_id != installed.installation.initial_principal_id
            || digest::sha256_hex(credential.credential().as_bytes()) != installed.credential_sha256
            || repair.inspection().runtime_cursor < installed.expected_runtime_cursor
            || repair.inspection().control_journal_sequence < installed.bootstrap_control_sequence
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        Ok(Self {
            repair,
            target: locator.identity,
            locator_sha256: digest::sha256_hex(&locator_bytes),
            executable_sha256,
            configuration_sha256: configuration.configuration_sha256,
            installed_record_sha256: installed.record_sha256,
        })
    }
}

fn decode<T: DeserializeOwned>(repair: &RrflowKvTailRepair, key: &str) -> Result<T> {
    let bytes = repair
        .control_record(key)?
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    serde_json::from_slice(&bytes).map_err(|e| ServiceError::Contract(e.to_string()))
}
