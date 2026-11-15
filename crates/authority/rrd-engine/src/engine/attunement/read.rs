//! Reading attunement jobs, details and receipts.

use super::*;

impl RrdEngine {
    /// Read the installed job through current engram authorization.
    #[allow(clippy::too_many_arguments)]
    pub fn read_attunement(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        job: &CanonicalId,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<AttunementStatus> {
        self.read_attunement_detail(
            session,
            token,
            &ReadAttunement {
                contract_version: ATTUNEMENT_CONTRACT_VERSION,
                job_id: job.clone(),
            },
            now,
            request_id,
            operation_id,
        )
        .map(|result| result.status)
    }

    /// Return the stored plan with its job. Source freshness is not checked.
    #[allow(clippy::too_many_arguments)]
    pub fn read_attunement_detail(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ReadAttunement,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<AttunementReadResult> {
        request.validate().map_err(contract)?;
        self.authorize(
            session,
            token,
            SecurityAction::EngramRead,
            now,
            request_id,
            operation_id,
        )?;
        let (_, job, plan) = self.load_attunement(&request.job_id)?;
        let result = AttunementReadResult {
            status: status(job, &plan, now)?,
            plan,
            source_verification: AttunementSourceVerification::NotChecked,
        };
        result.validate().map_err(contract)?;
        Ok(result)
    }

    /// Read the caller's previously committed receipt by its original key.
    /// This does not execute a command or verify the project's current sources.
    #[allow(clippy::too_many_arguments)]
    pub fn read_attunement_receipt(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        job_id: &CanonicalId,
        idempotency_key: &CorrelationId,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<Option<AttunementReceiptView>> {
        let (_, authorization) = self.authorize(
            session,
            token,
            SecurityAction::EngramRead,
            now,
            request_id,
            operation_id,
        )?;
        let principal = authorization
            .principal_id
            .ok_or(ServiceError::Unauthenticated)?;
        let identity = self
            .installed_engram_identity()
            .ok_or(ServiceError::ProjectBindingMismatch)?;
        let key = attunement_receipt_key(&self.instance, job_id, &principal, idempotency_key)?;
        let Some(bytes) = self.storage.control().get(&key)? else {
            return Ok(None);
        };
        let receipt: Receipt = decode(&bytes)?;
        if receipt.format_version != 1
            || receipt.principal_id != principal
            || receipt.status.job.id != *job_id
            || receipt.operation_sha256.len() != 64
            || !receipt
                .operation_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ServiceError::Storage(
                "invalid attunement receipt identity".into(),
            ));
        }
        // Resolve the receipt's plan, not the current job's plan: a later
        // successor must not make a valid historical receipt disappear.
        let plan: AttunementPlan = decode(
            &self
                .storage
                .control()
                .get(&attunement_plan_key(
                    &self.instance,
                    &receipt.status.job.plan_id,
                ))?
                .ok_or_else(|| ServiceError::Storage("attunement receipt plan missing".into()))?,
        )?;
        if plan.target != *identity {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        receipt.status.validate(&plan).map_err(contract)?;
        Ok(Some(AttunementReceiptView {
            plan,
            principal_id: receipt.principal_id,
            operation_sha256: receipt.operation_sha256,
            status: receipt.status,
        }))
    }

    pub(super) fn load_attunement(
        &self,
        id: &CanonicalId,
    ) -> Result<(Vec<u8>, AttunementJob, AttunementPlan)> {
        let identity = self
            .installed_engram_identity()
            .ok_or(ServiceError::ProjectBindingMismatch)?;
        let bytes = self
            .storage
            .control()
            .get(&attunement_job_key(&self.instance, id))?
            .ok_or_else(|| ServiceError::Contract("installed attunement job not found".into()))?;
        let job: AttunementJob = decode(&bytes)?;
        if job.id != *id {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let plan: AttunementPlan = decode(
            &self
                .storage
                .control()
                .get(&attunement_plan_key(&self.instance, &job.plan_id))?
                .ok_or_else(|| ServiceError::Storage("installed attunement plan missing".into()))?,
        )?;
        if plan.target != *identity {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        job.validate(&plan).map_err(contract)?;
        Ok((bytes, job, plan))
    }
}
