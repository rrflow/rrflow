//! Replan preview, acceptance, results and receipts.

use super::*;

/// Preview is a bounded source observation, not an atomic snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementReplanPreview {
    pub predecessor_job_id: CanonicalId,
    pub predecessor_revision: u64,
    pub generation: u64,
    pub plan: AttunementPlan,
    pub proposal_sha256: String,
    /// Present when the engine derived the successor plan from committed
    /// inventory evidence; the plan then equals `derive_attunement_plan`
    /// over this evidence, which any client can recompute. Absent when the
    /// successor only rebinds the predecessor's plan to fresh source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<AttunementEvidence>,
}

/// Hashes are shared with the engine so clients validate exactly the same
/// source, predecessor and proposal identity without owning lifecycle state.
pub fn attunement_replan_seed(
    job_id: &CanonicalId,
    revision: u64,
    old_plan_sha256: &str,
    generation: u64,
    source_sha256: &str,
) -> Result<String> {
    plain_json_sha256(&(
        "attunement-replan-v1",
        job_id,
        revision,
        old_plan_sha256,
        generation,
        source_sha256,
    ))
}

pub fn attunement_replan_preview_sha256(
    job_id: &CanonicalId,
    revision: u64,
    generation: u64,
    plan: &AttunementPlan,
) -> Result<String> {
    plain_json_sha256(&(
        "attunement-replan-preview-v1",
        job_id,
        revision,
        generation,
        plan,
    ))
}

pub fn attunement_replan_accept_sha256(
    preview: &AttunementReplanPreview,
    digest: &str,
) -> Result<String> {
    plain_json_sha256(&("attunement-accept-replan-v1", preview, digest))
}

pub(super) fn plain_json_sha256(value: &impl Serialize) -> Result<String> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        crate::ContractError(format!("contract digest encoding failed: {error}"))
    })?;
    Ok(sha256_bytes(&encoded))
}

impl AttunementReplanPreview {
    pub fn validate_for(
        &self,
        job_id: &CanonicalId,
        revision: u64,
        old_plan: &AttunementPlan,
    ) -> Result<()> {
        old_plan.validate()?;
        self.plan.validate()?;
        if self.predecessor_job_id != *job_id
            || self.predecessor_revision != revision
            || self.generation == 0
        {
            return invalid("attunement replan predecessor or generation does not match request");
        }
        let seed = attunement_replan_seed(
            job_id,
            revision,
            &old_plan.plan_sha256,
            self.generation,
            &self.plan.source_sha256,
        )?;
        let id = CanonicalId::new(format!("attune-{}", &seed[..24]))?;
        let expected = match &self.evidence {
            Some(evidence) => {
                derive_attunement_plan(old_plan, evidence, id, self.plan.source_sha256.clone())?
            }
            None => {
                let mut expected = old_plan.clone();
                expected.id = id;
                expected.source_sha256 = self.plan.source_sha256.clone();
                expected.plan_sha256 = self.plan.plan_sha256.clone();
                expected
            }
        };
        if self.plan != expected
            || self.proposal_sha256
                != attunement_replan_preview_sha256(job_id, revision, self.generation, &self.plan)?
        {
            return invalid("attunement replan plan or proposal digest differs");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementReplanResult {
    pub predecessor: AttunementStatus,
    pub successor: AttunementStatus,
    pub plan: AttunementPlan,
    pub idempotent_replay: bool,
}

impl AttunementReplanResult {
    pub fn validate_for(
        &self,
        preview: &AttunementReplanPreview,
        old_plan: &AttunementPlan,
    ) -> Result<()> {
        preview.validate_for(
            &preview.predecessor_job_id,
            preview.predecessor_revision,
            old_plan,
        )?;
        self.predecessor.validate(old_plan)?;
        self.successor.validate(&self.plan)?;
        let expected_successor =
            CanonicalId::new(format!("job-{}", &preview.plan.plan_sha256[..24]))?;
        let predecessor_lineage = match self.predecessor.job.state {
            AttunementJobState::CancelRequested => {
                preview.predecessor_revision.checked_add(1) == Some(self.predecessor.job.revision)
            }
            AttunementJobState::Succeeded => {
                self.predecessor.job.revision == preview.predecessor_revision
                    && self.predecessor.job.checkpoints.iter().all(|checkpoint| {
                        matches!(
                            checkpoint.phase,
                            AttunementPhase::Connect | AttunementPhase::Inventory
                        )
                    })
            }
            _ => false,
        };
        if self.plan != preview.plan
            || self.predecessor.job.id != preview.predecessor_job_id
            || !predecessor_lineage
            || self.successor.job.id != expected_successor
            || self.successor.job.state != AttunementJobState::Pending
        {
            return invalid("attunement replan result lineage differs from proposal");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementReplanReceiptView {
    pub principal_id: CanonicalId,
    pub operation_sha256: String,
    pub preview: AttunementReplanPreview,
    pub predecessor_plan: AttunementPlan,
    pub result: AttunementReplanResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewAttunementReplan {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub expected_revision: u64,
}

impl PreviewAttunementReplan {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptAttunementReplan {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub preview: AttunementReplanPreview,
    pub proposal_sha256: String,
}

impl AcceptAttunementReplan {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)?;
        validate_sha256(&self.proposal_sha256, "attunement proposal digest")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementReplanReceiptReadResult {
    pub receipt: Option<AttunementReplanReceiptView>,
}

impl AttunementReplanReceiptReadResult {
    pub fn validate_for(
        &self,
        request: &ReadAttunementReceipt,
        principal: &CanonicalId,
        instance: &CanonicalId,
    ) -> Result<()> {
        request.validate()?;
        if let Some(receipt) = &self.receipt {
            receipt.preview.validate_for(
                &request.job_id,
                receipt.preview.predecessor_revision,
                &receipt.predecessor_plan,
            )?;
            receipt
                .result
                .validate_for(&receipt.preview, &receipt.predecessor_plan)?;
            if receipt.principal_id != *principal
                || receipt.predecessor_plan.target.instance_id != *instance
                || receipt.operation_sha256
                    != attunement_replan_accept_sha256(
                        &receipt.preview,
                        &receipt.preview.proposal_sha256,
                    )?
                || receipt.result.idempotent_replay
            {
                return invalid("attunement replan receipt identity or digest differs");
            }
        }
        Ok(())
    }
}

impl AttunementReceiptReadResult {
    pub fn validate_for(
        &self,
        request: &ReadAttunementReceipt,
        principal: &CanonicalId,
        instance: &CanonicalId,
    ) -> Result<()> {
        request.validate()?;
        if let Some(receipt) = &self.receipt {
            validate_sha256(
                &receipt.operation_sha256,
                "attunement receipt operation digest",
            )?;
            receipt.plan.validate()?;
            receipt.status.validate(&receipt.plan)?;
            if receipt.status.job.id != request.job_id
                || receipt.principal_id != *principal
                || receipt.plan.target.instance_id != *instance
            {
                return invalid("attunement receipt identity does not match request");
            }
        }
        Ok(())
    }
}
