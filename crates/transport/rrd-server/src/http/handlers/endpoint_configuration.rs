use super::super::*;

impl AppState {
    pub(in crate::http) fn read_endpoint_configuration(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<rrd_contract::ReadEndpointConfiguration, _, _>(
            headers,
            body,
            now,
            RrdOperation::EstateRead,
            None,
            |envelope, session, token| {
                self.service
                    .read_endpoint_configuration(
                        session,
                        token,
                        now,
                        envelope.context.request_id.as_str(),
                        envelope.context.operation_id.as_str(),
                    )
                    .map_err(api_error)
            },
        )
    }
    pub(in crate::http) fn preview_endpoint_configuration(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<rrd_contract::PreviewEndpointConfiguration, _, _>(
            headers,
            body,
            now,
            RrdOperation::EndpointConfigurationPreview,
            None,
            |envelope, session, token| {
                self.service
                    .preview_endpoint_configuration(
                        session,
                        token,
                        &envelope.payload,
                        now,
                        envelope.context.request_id.as_str(),
                        envelope.context.operation_id.as_str(),
                    )
                    .map_err(api_error)
            },
        )
    }
    pub(in crate::http) fn apply_endpoint_configuration(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<rrd_contract::ApplyEndpointConfiguration, _, _>(
            headers,
            body,
            now,
            RrdOperation::EstateAdmin,
            None,
            |envelope, session, token| {
                self.service
                    .apply_endpoint_configuration(
                        session,
                        token,
                        &envelope.payload,
                        now,
                        envelope.context.request_id.as_str(),
                        envelope.context.operation_id.as_str(),
                    )
                    .map_err(api_error)
            },
        )
    }
}
