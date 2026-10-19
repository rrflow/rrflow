use super::super::*;
use rrd_contract::{CloseTask, ListTasks, OpenTask, ReadTask};

impl AppState {
    pub(in crate::http) fn open_task(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<OpenTask, _, _>(
            headers,
            body,
            now,
            RrdOperation::TaskWrite,
            None,
            |envelope, session, token| {
                let idempotency_key = envelope
                    .context
                    .idempotency_key
                    .as_ref()
                    .expect("mutating envelopes require idempotency");
                self.service
                    .open_task(
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

    pub(in crate::http) fn close_task(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<CloseTask, _, _>(
            headers,
            body,
            now,
            RrdOperation::TaskWrite,
            None,
            |envelope, session, token| {
                self.service
                    .close_task(
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

    pub(in crate::http) fn list_tasks(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ListTasks, _, _>(
            headers,
            body,
            now,
            RrdOperation::TaskRead,
            None,
            |envelope, session, token| {
                self.service
                    .list_tasks(
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

    pub(in crate::http) fn read_task(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ReadTask, _, _>(
            headers,
            body,
            now,
            RrdOperation::TaskRead,
            None,
            |envelope, session, token| {
                self.service
                    .read_task(
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
