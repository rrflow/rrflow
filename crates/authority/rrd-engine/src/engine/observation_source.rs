//! Source configuration is an engine-owned control transition, not collector activation.
use super::*;
use rrd_contract::{
    ApplyObservationSourceConfiguration, ObservationSourceConfiguration,
    ObservationSourceConfigurationPlan, PreviewObservationSourceConfiguration,
    OBSERVATION_SOURCE_CONFIGURATION_VERSION, OBSERVATION_SOURCE_PREVIEW_TTL_MS,
};

impl RrdEngine {
    fn observation_source_identity(&self) -> Result<InstalledEngramIdentity> {
        self.installed_engram_identity().cloned().ok_or_else(|| {
            ServiceError::Contract(
                "observation sources require an installed project identity".into(),
            )
        })
    }
    fn observation_source_key(&self) -> String {
        format!("server/state/{}/observation-sources/active", self.instance)
    }
    fn load_observation_source_configuration(
        &self,
    ) -> Result<(Option<Vec<u8>>, ObservationSourceConfiguration)> {
        let identity = self.observation_source_identity()?;
        let bytes = self.storage.control().get(&self.observation_source_key())?;
        let state = match &bytes {
            Some(bytes) => serde_json::from_slice::<ObservationSourceConfiguration>(bytes)
                .map_err(contract_json)?,
            None => ObservationSourceConfiguration {
                format_version: OBSERVATION_SOURCE_CONFIGURATION_VERSION,
                identity: identity.clone(),
                revision: 0,
                sources: vec![],
                last_applied_plan_sha256: None,
            },
        };
        state
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        if state.identity != identity {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        Ok((bytes, state))
    }
    pub fn read_observation_source_configuration(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request_id: &str,
        operation_id: &str,
    ) -> Result<ObservationSourceConfiguration> {
        let now = super::clock::observe_required(
            &super::clock::HostSystemClock,
            "observation-source-read",
        )?
        .observed_at_unix_ms;
        self.read_observation_source_configuration_at(session, token, now, request_id, operation_id)
    }
    pub(in crate::engine) fn read_observation_source_configuration_at(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<ObservationSourceConfiguration> {
        self.authorize(
            session,
            token,
            SecurityAction::EngramRead,
            now,
            request_id,
            operation_id,
        )?;
        Ok(self.load_observation_source_configuration()?.1)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn preview_observation_source_configuration(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &PreviewObservationSourceConfiguration,
        request_id: &str,
        operation_id: &str,
    ) -> Result<ObservationSourceConfigurationPlan> {
        let now = super::clock::observe_required(
            &super::clock::HostSystemClock,
            "observation-source-preview",
        )?
        .observed_at_unix_ms;
        self.preview_observation_source_configuration_at(
            session,
            token,
            request,
            now,
            request_id,
            operation_id,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine) fn preview_observation_source_configuration_at(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &PreviewObservationSourceConfiguration,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<ObservationSourceConfigurationPlan> {
        self.authorize(
            session,
            token,
            SecurityAction::EngramAdmin,
            now,
            request_id,
            operation_id,
        )?;
        request
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let (_, current) = self.load_observation_source_configuration()?;
        if current.revision != request.expected_revision {
            return Err(ServiceError::StorageConflict(
                "observation sources changed; preview the current revision".into(),
            ));
        }
        let mut plan = ObservationSourceConfigurationPlan {
            format_version: OBSERVATION_SOURCE_CONFIGURATION_VERSION,
            identity: current.identity.clone(),
            session_id: session.clone(),
            expected_revision: current.revision,
            expected_configuration_sha256: current
                .sha256()
                .map_err(|e| ServiceError::Contract(e.to_string()))?,
            sources: request.sources.clone(),
            issued_at_unix_ms: now,
            expires_at_unix_ms: now
                .checked_add(OBSERVATION_SOURCE_PREVIEW_TTL_MS)
                .ok_or_else(|| ServiceError::Contract("preview expiry overflow".into()))?,
            plan_sha256: String::new(),
            engine_authenticator: vec![],
        };
        plan.plan_sha256 = plan
            .sha256()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(&self.token_key)
            .expect("HMAC accepts a 32-byte key");
        mac.update(b"rrflow-observation-source-plan-v1\0");
        mac.update(plan.plan_sha256.as_bytes());
        plan.engine_authenticator = mac.finalize().into_bytes().to_vec();
        plan.validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        Ok(plan)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply_observation_source_configuration(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ApplyObservationSourceConfiguration,
        request_id: &str,
        operation_id: &str,
    ) -> Result<ObservationSourceConfiguration> {
        let now = super::clock::observe_required(
            &super::clock::HostSystemClock,
            "observation-source-apply",
        )?
        .observed_at_unix_ms;
        self.apply_observation_source_configuration_at(
            session,
            token,
            request,
            now,
            request_id,
            operation_id,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine) fn apply_observation_source_configuration_at(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ApplyObservationSourceConfiguration,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<ObservationSourceConfiguration> {
        self.authorize(
            session,
            token,
            SecurityAction::EngramAdmin,
            now,
            request_id,
            operation_id,
        )?;
        request
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let _permit = self.admit_transaction(request, request_id, operation_id)?;
        let (expected, current) = self.load_observation_source_configuration()?;
        let plan = &request.plan;
        if plan.identity != current.identity {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(&self.token_key)
            .expect("HMAC accepts a 32-byte key");
        mac.update(b"rrflow-observation-source-plan-v1\0");
        mac.update(plan.plan_sha256.as_bytes());
        mac.verify_slice(&plan.engine_authenticator).map_err(|_| {
            ServiceError::Contract("observation plan was not issued by this engine".into())
        })?;
        if &plan.session_id != session {
            return Err(ServiceError::Contract(
                "observation plan belongs to another session".into(),
            ));
        }
        // A lost response is recoverable only while this publication is still the head.
        if current.last_applied_plan_sha256.as_ref() == Some(&plan.plan_sha256) {
            return Ok(current);
        }
        if now < plan.issued_at_unix_ms || now >= plan.expires_at_unix_ms {
            return Err(ServiceError::DeadlineExceeded);
        }
        if current.revision != plan.expected_revision
            || current
                .sha256()
                .map_err(|e| ServiceError::Contract(e.to_string()))?
                != plan.expected_configuration_sha256
        {
            return Err(ServiceError::StorageConflict(
                "observation sources changed; preview the current revision".into(),
            ));
        }
        let next = ObservationSourceConfiguration {
            format_version: OBSERVATION_SOURCE_CONFIGURATION_VERSION,
            identity: current.identity,
            revision: current.revision + 1,
            sources: plan.sources.clone(),
            last_applied_plan_sha256: Some(plan.plan_sha256.clone()),
        };
        next.validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        commit_engine_control(
            &self.storage,
            self.observation_source_key(),
            expected,
            &next,
            EngineControlContext {
                now,
                session_id: session,
                action: if next.sources.is_empty() {
                    "observation.sources.revoked"
                } else {
                    "observation.sources.applied"
                },
                request_id,
                operation_id,
            },
        )?;
        Ok(next)
    }
}
