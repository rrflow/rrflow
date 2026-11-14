//! Endpoint configuration operations.

use super::*;

impl RrdClient {
    pub async fn read_endpoint_configuration(
        &self,
        session: &Session,
        request: rrd_contract::ReadEndpointConfiguration,
        options: RequestOptions,
    ) -> Result<rrd_contract::EndpointConfiguration> {
        let catalogue = rrd_contract::endpoint_catalogue();
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|e| e.operation.as_str() == "endpoint-configuration-read")
            .ok_or_else(|| {
                Error::Contract("endpoint configuration operation is unavailable".into())
            })?;
        let result: rrd_contract::EndpointConfiguration = self
            .session_call(
                Method::POST,
                &endpoint.path,
                session,
                request,
                options,
                endpoint.mutation,
            )
            .await?;
        result.validate().map_err(contract)?;
        if result.identity.instance_id != self.instance {
            return Err(Error::ResponseIdentityMismatch);
        }
        Ok(result)
    }

    pub async fn preview_endpoint_configuration(
        &self,
        session: &Session,
        request: rrd_contract::PreviewEndpointConfiguration,
        options: RequestOptions,
    ) -> Result<rrd_contract::EndpointConfigurationPlan> {
        request.validate().map_err(contract)?;
        let expected_revision = request.expected_revision;
        let expected_profile = request
            .profile
            .clone()
            .map(|p| rrd_contract::EndpointProfile::from_input(expected_revision + 1, p))
            .transpose()
            .map_err(contract)?;
        let catalogue = rrd_contract::endpoint_catalogue();
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|e| e.operation.as_str() == "endpoint-configuration-preview")
            .ok_or_else(|| {
                Error::Contract("endpoint configuration operation is unavailable".into())
            })?;
        let result: rrd_contract::EndpointConfigurationPlan = self
            .session_call(
                Method::POST,
                &endpoint.path,
                session,
                request,
                options,
                endpoint.mutation,
            )
            .await?;
        result.validate().map_err(contract)?;
        if result.identity.instance_id != self.instance {
            return Err(Error::ResponseIdentityMismatch);
        }
        if result.expected_revision != expected_revision || result.profile != expected_profile {
            return Err(Error::ResponseIdentityMismatch);
        }
        Ok(result)
    }

    pub async fn apply_endpoint_configuration(
        &self,
        session: &Session,
        request: rrd_contract::ApplyEndpointConfiguration,
        options: RequestOptions,
    ) -> Result<rrd_contract::EndpointConfiguration> {
        request.validate().map_err(contract)?;
        let expected_plan = request.plan.clone();
        let catalogue = rrd_contract::endpoint_catalogue();
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|e| e.operation.as_str() == "endpoint-configuration-apply")
            .ok_or_else(|| {
                Error::Contract("endpoint configuration operation is unavailable".into())
            })?;
        let result: rrd_contract::EndpointConfiguration = self
            .session_call(
                Method::POST,
                &endpoint.path,
                session,
                request,
                options,
                endpoint.mutation,
            )
            .await?;
        result.validate().map_err(contract)?;
        if result.identity.instance_id != self.instance {
            return Err(Error::ResponseIdentityMismatch);
        }
        if result.identity != expected_plan.identity
            || result.revision != expected_plan.expected_revision + 1
            || result.profile != expected_plan.profile
            || result.last_applied_plan_sha256.as_ref() != Some(&expected_plan.plan_sha256)
        {
            return Err(Error::ResponseIdentityMismatch);
        }
        Ok(result)
    }
}
