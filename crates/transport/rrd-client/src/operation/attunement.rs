//! Attunement job operations.

use super::*;

impl RrdClient {
    /// Discover stored installed jobs without deriving engine identities locally.
    pub async fn list_attunement_jobs(
        &self,
        session: &Session,
        request: rrd_contract::ListAttunementJobs,
        options: RequestOptions,
    ) -> Result<rrd_contract::AttunementJobPage> {
        request.validate().map_err(contract)?;
        let result: rrd_contract::AttunementJobPage = self
            .generated_attunement_call::<generated::AttunementJobList>(session, &request, options)
            .await?;
        result
            .validate_for(&request, &self.instance)
            .map_err(contract)?;
        Ok(result)
    }

    /// Request cancellation; the returned state does not imply worker termination.
    pub async fn cancel_attunement(
        &self,
        session: &Session,
        request: rrd_contract::CancelAttunement,
        options: RequestOptions,
    ) -> Result<rrd_contract::AttunementCancelResult> {
        let result: rrd_contract::AttunementCancelResult = self
            .generated_attunement_call::<generated::AttunementCancel>(session, &request, options)
            .await?;
        result.validate_for(&request).map_err(contract)?;
        Ok(result)
    }

    /// Read the caller's original committed receipt without replaying its command.
    pub async fn read_attunement_receipt(
        &self,
        session: &Session,
        request: rrd_contract::ReadAttunementReceipt,
        options: RequestOptions,
    ) -> Result<rrd_contract::AttunementReceiptReadResult> {
        request.validate().map_err(contract)?;
        let result: rrd_contract::AttunementReceiptReadResult = self
            .generated_attunement_call::<generated::AttunementReceiptRead>(
                session, &request, options,
            )
            .await?;
        result
            .validate_for(&request, session.principal_id(), &self.instance)
            .map_err(contract)?;
        Ok(result)
    }

    /// Read the engine-owned installed job and its exact stored plan.
    pub async fn read_attunement(
        &self,
        session: &Session,
        request: rrd_contract::ReadAttunement,
        options: RequestOptions,
    ) -> Result<rrd_contract::AttunementReadResult> {
        request.validate().map_err(contract)?;
        let job_id = request.job_id.clone();
        let result: rrd_contract::AttunementReadResult = self
            .generated_attunement_call::<generated::AttunementRead>(session, &request, options)
            .await?;
        result.validate().map_err(contract)?;
        if result.plan.target.instance_id != self.instance || result.status.job.id != job_id {
            return Err(Error::ResponseIdentityMismatch);
        }
        Ok(result)
    }
}
