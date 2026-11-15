use super::*;

impl RrdEngine {
    pub fn read_engram(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        engram_id: CanonicalId,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<Option<rrd_contract::EngramSnapshot>> {
        self.authorize(
            session_id,
            token,
            SecurityAction::EngramRead,
            now,
            request_id,
            operation_id,
        )?;
        let repository = rrd_engram::EngramRepository::new(&self.storage, engram_id);
        repository
            .load()
            .map(|document| document.as_ref().map(rrd_engram::public_snapshot))
            .map_err(Into::into)
    }
}
