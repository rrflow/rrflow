//! Configuration publication uses the canonical control CAS and never invokes a provider.
use super::*;
use rrd_contract::{
    ApplyEndpointConfiguration, EndpointConfiguration, EndpointConfigurationPlan, EndpointProfile,
    PreviewEndpointConfiguration, ENDPOINT_CONFIGURATION_PREVIEW_TTL_MS,
    ENDPOINT_CONFIGURATION_VERSION,
};

impl RrdEngine {
    fn endpoint_configuration_identity(&self) -> Result<InstalledEstateIdentity> {
        self.installed_estate_identity().cloned().ok_or_else(|| {
            ServiceError::Contract(
                "endpoint configuration requires an installed project identity".into(),
            )
        })
    }
    fn endpoint_configuration_key(&self) -> String {
        format!(
            "server/state/{}/endpoint-configuration/active",
            self.instance
        )
    }
    fn load_endpoint_configuration(&self) -> Result<(Option<Vec<u8>>, EndpointConfiguration)> {
        let identity = self.endpoint_configuration_identity()?;
        let bytes = self
            .storage
            .control()
            .get(&self.endpoint_configuration_key())?;
        let state = match &bytes {
            Some(bytes) => {
                serde_json::from_slice::<EndpointConfiguration>(bytes).map_err(contract_json)?
            }
            None => EndpointConfiguration {
                format_version: ENDPOINT_CONFIGURATION_VERSION,
                identity: identity.clone(),
                revision: 0,
                profile: None,
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
    pub fn read_endpoint_configuration(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<EndpointConfiguration> {
        self.authorize(
            session,
            token,
            SecurityAction::EstateRead,
            now,
            request_id,
            operation_id,
        )?;
        Ok(self.load_endpoint_configuration()?.1)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn preview_endpoint_configuration(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &PreviewEndpointConfiguration,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<EndpointConfigurationPlan> {
        self.authorize(
            session,
            token,
            SecurityAction::EstateAdmin,
            now,
            request_id,
            operation_id,
        )?;
        request
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let (_, current) = self.load_endpoint_configuration()?;
        if current.revision != request.expected_revision {
            return Err(ServiceError::StorageConflict(
                "endpoint configuration changed; preview the current revision".into(),
            ));
        }
        let mut plan = EndpointConfigurationPlan {
            format_version: ENDPOINT_CONFIGURATION_VERSION,
            identity: current.identity.clone(),
            expected_revision: current.revision,
            expected_configuration_sha256: current
                .sha256()
                .map_err(|e| ServiceError::Contract(e.to_string()))?,
            profile: request
                .profile
                .clone()
                .map(|p| EndpointProfile::from_input(current.revision + 1, p))
                .transpose()
                .map_err(|e| ServiceError::Contract(e.to_string()))?,
            issued_at_unix_ms: now,
            expires_at_unix_ms: now
                .checked_add(ENDPOINT_CONFIGURATION_PREVIEW_TTL_MS)
                .ok_or_else(|| ServiceError::Contract("preview expiry overflow".into()))?,
            plan_sha256: String::new(),
        };
        plan.plan_sha256 = plan
            .sha256()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        plan.validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        Ok(plan)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply_endpoint_configuration(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ApplyEndpointConfiguration,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<EndpointConfiguration> {
        self.authorize(
            session,
            token,
            SecurityAction::EstateAdmin,
            now,
            request_id,
            operation_id,
        )?;
        request
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let _permit = self.admit_transaction(request, request_id, operation_id)?;
        let (expected, current) = self.load_endpoint_configuration()?;
        let plan = &request.plan;
        if plan.identity != current.identity {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        // Recover a lost response only while this exact publication is still current.
        // A replay after a later edit/revocation cannot resurrect the old profile.
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
                "endpoint configuration changed; preview the current revision".into(),
            ));
        }
        let next = EndpointConfiguration {
            format_version: ENDPOINT_CONFIGURATION_VERSION,
            identity: current.identity,
            revision: current.revision + 1,
            profile: plan.profile.clone(),
            last_applied_plan_sha256: Some(plan.plan_sha256.clone()),
        };
        next.validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        commit_engine_control(
            &self.storage,
            self.endpoint_configuration_key(),
            expected,
            &next,
            EngineControlContext {
                now,
                session_id: session,
                action: if next.profile.is_some() {
                    "endpoint.configuration.applied"
                } else {
                    "endpoint.configuration.revoked"
                },
                request_id,
                operation_id,
            },
        )?;
        Ok(next)
    }
}
