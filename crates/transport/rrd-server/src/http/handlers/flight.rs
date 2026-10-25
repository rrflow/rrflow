use super::super::*;
use rrd_contract::{LaunchPromptFlight, ListPromptFlights, ReadPromptFlight};

impl AppState {
    pub(in crate::http) fn launch_prompt_flight(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<LaunchPromptFlight, _, _>(
            headers,
            body,
            now,
            RrdOperation::FlightLaunch,
            None,
            |envelope, session, token| {
                let idempotency_key = envelope
                    .context
                    .idempotency_key
                    .as_ref()
                    .expect("mutating envelopes require idempotency");
                self.service
                    .launch_prompt_flight(
                        session,
                        token,
                        idempotency_key,
                        &envelope.payload,
                        now,
                        envelope.context.request_id.as_str(),
                        envelope.context.operation_id.as_str(),
                    )
                    .map_err(api_error)
            },
        )
    }

    pub(in crate::http) fn list_prompt_flights(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ListPromptFlights, _, _>(
            headers,
            body,
            now,
            RrdOperation::FlightRead,
            None,
            |envelope, session, token| {
                self.service
                    .list_prompt_flights(
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

    pub(in crate::http) fn read_prompt_flight(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ReadPromptFlight, _, _>(
            headers,
            body,
            now,
            RrdOperation::FlightRead,
            None,
            |envelope, session, token| {
                self.service
                    .read_prompt_flight(
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
