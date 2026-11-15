use super::super::*;
use rrd_contract::{
    AcceptAttunementReplan, AttunementCancelResult, AttunementControlResponse, AttunementReceipt,
    AttunementReceiptReadResult, AttunementReplanReceiptReadResult, AttunementSourceVerification,
    CancelAttunement, PauseAttunement, PreviewAttunementReplan, ReadAttunement,
    ReadAttunementReceipt, RecoverExpiredAttunement, ResumeAttunementWithLease, StartAttunement,
    StepAttunement,
};
use rrd_engine::AttunementCommand;

impl AppState {
    pub(in crate::http) fn pause_attunement(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<PauseAttunement, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramAdmin,
            None,
            |envelope, session, token| {
                envelope.payload.validate().map_err(|error| {
                    ApiError::new(ErrorCode::InvalidArgument, error.to_string(), false)
                })?;
                let request = &envelope.payload;
                self.service
                    .control_attunement(
                        session,
                        token,
                        &AttunementCommand::Pause {
                            job_id: request.job_id.clone(),
                            expected_revision: request.expected_revision,
                            lease_generation: request.lease_generation,
                        },
                        &envelope.context,
                        now,
                    )
                    .map(|result| AttunementControlResponse {
                        status: result.status,
                        idempotent_replay: result.idempotent_replay,
                    })
                    .map_err(api_error)
            },
        )
    }

    pub(in crate::http) fn recover_expired_attunement(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<RecoverExpiredAttunement, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramAdmin,
            None,
            |envelope, session, token| {
                envelope.payload.validate().map_err(|error| {
                    ApiError::new(ErrorCode::InvalidArgument, error.to_string(), false)
                })?;
                let request = &envelope.payload;
                self.service
                    .control_attunement(
                        session,
                        token,
                        &AttunementCommand::RecoverExpired {
                            job_id: request.job_id.clone(),
                            expected_revision: request.expected_revision,
                        },
                        &envelope.context,
                        now,
                    )
                    .map(|result| AttunementControlResponse {
                        status: result.status,
                        idempotent_replay: result.idempotent_replay,
                    })
                    .map_err(api_error)
            },
        )
    }

    pub(in crate::http) fn resume_attunement(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ResumeAttunementWithLease, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramAdmin,
            None,
            |envelope, session, token| {
                self.service
                    .control_attunement(
                        session,
                        token,
                        &AttunementCommand::Resume {
                            request: envelope.payload.request.clone(),
                            lease_ms: envelope.payload.lease_ms,
                        },
                        &envelope.context,
                        now,
                    )
                    .map(|result| AttunementControlResponse {
                        status: result.status,
                        idempotent_replay: result.idempotent_replay,
                    })
                    .map_err(api_error)
            },
        )
    }

    pub(in crate::http) fn start_attunement(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<StartAttunement, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramAdmin,
            None,
            |envelope, session, token| {
                envelope.payload.validate().map_err(|error| {
                    ApiError::new(ErrorCode::InvalidArgument, error.to_string(), false)
                })?;
                let request = &envelope.payload;
                self.service
                    .control_attunement(
                        session,
                        token,
                        &AttunementCommand::Start {
                            job_id: request.job_id.clone(),
                            plan_sha256: request.plan_sha256.clone(),
                            expected_revision: request.expected_revision,
                            lease_ms: request.lease_ms,
                        },
                        &envelope.context,
                        now,
                    )
                    .map(|result| AttunementControlResponse {
                        status: result.status,
                        idempotent_replay: result.idempotent_replay,
                    })
                    .map_err(api_error)
            },
        )
    }

    pub(in crate::http) fn step_attunement(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<StepAttunement, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramAdmin,
            None,
            |envelope, session, token| {
                envelope.payload.validate().map_err(|error| {
                    ApiError::new(ErrorCode::InvalidArgument, error.to_string(), false)
                })?;
                let request = &envelope.payload;
                self.service
                    .control_attunement(
                        session,
                        token,
                        &AttunementCommand::Step {
                            job_id: request.job_id.clone(),
                            expected_revision: request.expected_revision,
                            lease_generation: request.lease_generation,
                        },
                        &envelope.context,
                        now,
                    )
                    .map(|result| AttunementControlResponse {
                        status: result.status,
                        idempotent_replay: result.idempotent_replay,
                    })
                    .map_err(api_error)
            },
        )
    }

    pub(in crate::http) fn preview_attunement_replan(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<PreviewAttunementReplan, _, _>(
            headers,
            body,
            now,
            RrdOperation::AttunementReplanPreview,
            None,
            |envelope, session, token| {
                envelope.payload.validate().map_err(|error| {
                    ApiError::new(ErrorCode::InvalidArgument, error.to_string(), false)
                })?;
                self.service
                    .preview_attunement_replan(
                        session,
                        token,
                        &envelope.payload.job_id,
                        envelope.payload.expected_revision,
                        now,
                        envelope.context.request_id.as_str(),
                        envelope.context.operation_id.as_str(),
                    )
                    .map_err(api_error)
            },
        )
    }

    pub(in crate::http) fn accept_attunement_replan(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<AcceptAttunementReplan, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramAdmin,
            None,
            |envelope, session, token| {
                envelope.payload.validate().map_err(|error| {
                    ApiError::new(ErrorCode::InvalidArgument, error.to_string(), false)
                })?;
                self.service
                    .accept_attunement_replan(
                        session,
                        token,
                        &envelope.payload.preview,
                        &envelope.payload.proposal_sha256,
                        &envelope.context,
                        now,
                    )
                    .map_err(api_error)
            },
        )
    }

    pub(in crate::http) fn read_attunement_replan_receipt(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ReadAttunementReceipt, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramRead,
            None,
            |envelope, session, token| {
                envelope.payload.validate().map_err(|error| {
                    ApiError::new(ErrorCode::InvalidArgument, error.to_string(), false)
                })?;
                self.service
                    .read_attunement_replan_receipt(
                        session,
                        token,
                        &envelope.payload.job_id,
                        &envelope.payload.idempotency_key,
                        now,
                        envelope.context.request_id.as_str(),
                        envelope.context.operation_id.as_str(),
                    )
                    .map(|receipt| AttunementReplanReceiptReadResult { receipt })
                    .map_err(api_error)
            },
        )
    }
    pub(in crate::http) fn cancel_attunement(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<CancelAttunement, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramAdmin,
            None,
            |envelope, session, token| {
                self.service
                    .control_attunement(
                        session,
                        token,
                        &AttunementCommand::Cancel {
                            request: envelope.payload.clone(),
                        },
                        &envelope.context,
                        now,
                    )
                    .map(|result| AttunementCancelResult {
                        status: result.status,
                        idempotent_replay: result.idempotent_replay,
                    })
                    .map_err(api_error)
            },
        )
    }
    pub(in crate::http) fn read_attunement_receipt(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ReadAttunementReceipt, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramRead,
            None,
            |envelope, session, token| {
                envelope.payload.validate().map_err(|error| {
                    ApiError::new(ErrorCode::InvalidArgument, error.to_string(), false)
                })?;
                self.service
                    .read_attunement_receipt(
                        session,
                        token,
                        &envelope.payload.job_id,
                        &envelope.payload.idempotency_key,
                        now,
                        envelope.context.request_id.as_str(),
                        envelope.context.operation_id.as_str(),
                    )
                    .map(|receipt| AttunementReceiptReadResult {
                        receipt: receipt.map(|view| AttunementReceipt {
                            principal_id: view.principal_id,
                            operation_sha256: view.operation_sha256,
                            plan: view.plan,
                            status: view.status,
                            source_verification: AttunementSourceVerification::NotChecked,
                        }),
                    })
                    .map_err(api_error)
            },
        )
    }
    pub(in crate::http) fn read_attunement(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<ReadAttunement, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramRead,
            None,
            |envelope, session, token| {
                self.service
                    .read_attunement_detail(
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

    pub(in crate::http) fn list_attunement_jobs(
        &self,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
    ) -> HttpResponse {
        self.with_authenticated_envelope::<rrd_contract::ListAttunementJobs, _, _>(
            headers,
            body,
            now,
            RrdOperation::EngramRead,
            None,
            |envelope, session, token| {
                self.service
                    .list_attunement_jobs(
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
