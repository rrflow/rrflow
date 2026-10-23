use super::super::*;

impl AppState {
    pub(in crate::http) fn inspect_storage_cursor(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<rrd_contract::InspectStorageCursor, _, _>(
            headers,
            body,
            now,
            RrdOperation::DiagnosticsRead,
            None,
            |envelope, session, token| {
                self.service
                    .inspect_storage_cursor(
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

    pub(in crate::http) fn read_storage_diagnostics(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<rrd_contract::ReadStorageDiagnostics, _, _>(
            headers,
            body,
            now,
            RrdOperation::DiagnosticsRead,
            None,
            |envelope, session, token| {
                self.service
                    .read_storage_diagnostics(
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

    pub(in crate::http) fn read_diagnostic_snapshot(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ReadDiagnosticSnapshot, _, _>(
            headers,
            body,
            now,
            RrdOperation::DiagnosticsRead,
            None,
            |envelope, session, token| {
                self.service
                    .read_diagnostic_snapshot(
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
