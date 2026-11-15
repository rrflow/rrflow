use super::*;

pub(super) type HttpResponse = Response<Body>;

#[derive(Debug, Clone)]
pub(super) struct ResponseDigest(pub(super) String);

#[derive(Debug)]
pub(super) struct ApiError {
    status: StatusCode,
    body: ErrorBody,
}

impl ApiError {
    pub(super) fn new(code: ErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        let status = status_for(code);
        Self {
            status,
            body: ErrorBody {
                code,
                message: message.into(),
                retryable,
                details: BTreeMap::new(),
            },
        }
    }
}

pub(in crate::http) fn generated_context(now: u64, seed: &str) -> RequestContext {
    let counter = HTTP_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let digest = sha256_hex(format!("{now}\0{seed}\0{counter}").as_bytes());
    RequestContext {
        request_id: CorrelationId::new(format!("request-{digest}")).expect("generated request id"),
        operation_id: CorrelationId::new(format!("operation-{digest}"))
            .expect("generated operation id"),
        idempotency_key: None,
        deadline_unix_ms: None,
    }
}

pub(in crate::http) fn instance_resource(instance: &CanonicalId) -> rrd_contract::ResourcePath {
    rrd_contract::ResourcePath {
        segments: vec![
            ResourceId::new(ResourceKind::Instance, instance.as_str().to_owned())
                .expect("server instance identity is already canonical"),
        ],
    }
}

pub(in crate::http) fn success<T: Serialize>(
    status: StatusCode,
    context: &RequestContext,
    payload: T,
) -> HttpResponse {
    json_response(
        status,
        &ResponseEnvelope {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            request_id: context.request_id.clone(),
            operation_id: context.operation_id.clone(),
            outcome: ResponseOutcome::Ok { payload },
        },
    )
}

pub(in crate::http) fn failure(context: &RequestContext, error: ApiError) -> HttpResponse {
    json_response(
        error.status,
        &ResponseEnvelope::<serde_json::Value> {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            request_id: context.request_id.clone(),
            operation_id: context.operation_id.clone(),
            outcome: ResponseOutcome::Error { error: error.body },
        },
    )
}

pub(in crate::http) fn json_response<T: Serialize>(status: StatusCode, body: &T) -> HttpResponse {
    let bytes = serde_json::to_vec(body)
        .unwrap_or_else(|error| format!("{{\"serialization_error\":{error:?}}}").into_bytes());
    let response_sha256 = sha256_hex(&bytes);
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
        .extensions_mut()
        .insert(ResponseDigest(response_sha256));
    response
}

fn stale_transaction_error(expected: u64, actual: u64) -> ApiError {
    let mut response = ApiError::new(
        ErrorCode::Conflict,
        "The transaction snapshot is stale. Abort it and reexecute its reads and writes from a fresh snapshot.",
        false,
    );
    for (key, value) in [
        ("reason", "transaction-snapshot-stale".to_owned()),
        ("next-action", "abort-and-reexecute-transaction".to_owned()),
        ("expected-cursor", expected.to_string()),
        ("actual-cursor", actual.to_string()),
    ] {
        response
            .body
            .details
            .insert(CanonicalId::new(key).expect("constant detail key"), value);
    }
    response
}

pub(in crate::http) fn api_error(error: ServiceError) -> ApiError {
    if let ServiceError::TransactionSnapshotStale { expected, actual } = error {
        return stale_transaction_error(expected, actual);
    }
    if matches!(error, ServiceError::AttunementSourceChanged) {
        let mut response = ApiError::new(
            ErrorCode::FailedPrecondition,
            "The project source changed after the plan was observed. Preview a replan or reconcile the job before continuing; acceptance is explicit.",
            false,
        );
        response.body.details.insert(
            CanonicalId::new("reason").expect("constant detail key"),
            "attunement-source-changed".into(),
        );
        response.body.details.insert(
            CanonicalId::new("next-action").expect("constant detail key"),
            "preview-attunement-replan".into(),
        );
        return response;
    }
    let message = error.to_string();
    let retryable = error.retryable();
    let code = match error.kind() {
        ServiceErrorKind::InvalidArgument => ErrorCode::InvalidArgument,
        ServiceErrorKind::NotFound => ErrorCode::NotFound,
        ServiceErrorKind::Unauthenticated => ErrorCode::Unauthenticated,
        ServiceErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
        ServiceErrorKind::Conflict => ErrorCode::Conflict,
        ServiceErrorKind::FailedPrecondition => ErrorCode::FailedPrecondition,
        ServiceErrorKind::ResourceExhausted => ErrorCode::ResourceExhausted,
        ServiceErrorKind::DeadlineExceeded => ErrorCode::DeadlineExceeded,
        ServiceErrorKind::Internal => ErrorCode::Internal,
    };
    ApiError::new(code, message, retryable)
}

pub(in crate::http) fn websocket_error(error: ServiceError) -> ErrorBody {
    if let ServiceError::TransactionSnapshotStale { expected, actual } = error {
        return stale_transaction_error(expected, actual).body;
    }
    let message = error.to_string();
    let retryable = error.retryable();
    let code = match error.kind() {
        ServiceErrorKind::InvalidArgument => ErrorCode::InvalidArgument,
        ServiceErrorKind::NotFound => ErrorCode::NotFound,
        ServiceErrorKind::Unauthenticated => ErrorCode::Unauthenticated,
        ServiceErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
        ServiceErrorKind::Conflict => ErrorCode::Conflict,
        ServiceErrorKind::FailedPrecondition => ErrorCode::FailedPrecondition,
        ServiceErrorKind::ResourceExhausted => ErrorCode::ResourceExhausted,
        ServiceErrorKind::DeadlineExceeded => ErrorCode::DeadlineExceeded,
        ServiceErrorKind::Internal => ErrorCode::Internal,
    };
    ErrorBody {
        code,
        message,
        retryable,
        details: BTreeMap::new(),
    }
}

pub(in crate::http) fn sha256_hex(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut output = String::with_capacity(hash.len() * 2);
    for byte in hash {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

pub(in crate::http) fn status_for(code: ErrorCode) -> StatusCode {
    match code {
        ErrorCode::InvalidArgument => StatusCode::BAD_REQUEST,
        ErrorCode::Unauthenticated => StatusCode::UNAUTHORIZED,
        ErrorCode::PermissionDenied => StatusCode::FORBIDDEN,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::AlreadyExists | ErrorCode::Conflict => StatusCode::CONFLICT,
        ErrorCode::FailedPrecondition => StatusCode::PRECONDITION_FAILED,
        ErrorCode::ResourceExhausted => StatusCode::TOO_MANY_REQUESTS,
        ErrorCode::Cancelled => StatusCode::from_u16(499).expect("valid nonstandard status"),
        ErrorCode::Internal | ErrorCode::Corruption => StatusCode::INTERNAL_SERVER_ERROR,
        ErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorCode::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
        ErrorCode::UnsupportedVersion => StatusCode::HTTP_VERSION_NOT_SUPPORTED,
    }
}

#[cfg(test)]
mod stale_transaction_tests {
    use super::*;

    /// Both errors are mapped before the generic kind table, so each keeps
    /// its own status, reason and next action.
    #[test]
    fn source_change_and_stale_transaction_keep_distinct_recovery_actions() {
        let source = api_error(ServiceError::AttunementSourceChanged);
        let transaction = api_error(ServiceError::TransactionSnapshotStale {
            expected: 7,
            actual: 9,
        });
        let detail = |body: &ErrorBody, key: &str| {
            body.details.get(&CanonicalId::new(key).unwrap()).cloned()
        };
        assert_eq!(source.status, StatusCode::PRECONDITION_FAILED);
        assert_eq!(source.body.code, ErrorCode::FailedPrecondition);
        assert!(!source.body.retryable);
        assert_eq!(
            detail(&source.body, "reason").as_deref(),
            Some("attunement-source-changed")
        );
        assert_eq!(
            detail(&source.body, "next-action").as_deref(),
            Some("preview-attunement-replan")
        );
        assert_eq!(detail(&source.body, "expected-cursor"), None);
        assert_eq!(transaction.status, StatusCode::CONFLICT);
        assert_eq!(transaction.body.code, ErrorCode::Conflict);
        assert!(!transaction.body.retryable);
        assert_eq!(
            detail(&transaction.body, "reason").as_deref(),
            Some("transaction-snapshot-stale")
        );
        assert_eq!(
            detail(&transaction.body, "next-action").as_deref(),
            Some("abort-and-reexecute-transaction")
        );
        assert_eq!(
            detail(&transaction.body, "expected-cursor").as_deref(),
            Some("7")
        );
        assert_eq!(
            detail(&transaction.body, "actual-cursor").as_deref(),
            Some("9")
        );
        assert!(source.body.validate().is_ok());
        assert!(transaction.body.validate().is_ok());
    }

    #[test]
    fn http_and_websocket_preserve_the_same_nonretryable_recovery_details() {
        let http = api_error(ServiceError::TransactionSnapshotStale {
            expected: 7,
            actual: 9,
        });
        let websocket = websocket_error(ServiceError::TransactionSnapshotStale {
            expected: 7,
            actual: 9,
        });
        assert_eq!(http.status, StatusCode::CONFLICT);
        assert_eq!(http.body, websocket);
        assert_eq!(websocket.code, ErrorCode::Conflict);
        assert!(!websocket.retryable);
        assert_eq!(
            websocket
                .details
                .get(&CanonicalId::new("reason").unwrap())
                .map(String::as_str),
            Some("transaction-snapshot-stale")
        );
        assert_eq!(
            websocket
                .details
                .get(&CanonicalId::new("next-action").unwrap())
                .map(String::as_str),
            Some("abort-and-reexecute-transaction")
        );
        assert!(websocket.validate().is_ok());
    }
}
