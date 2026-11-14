//! Polling active subscriptions and invoking their operations.

use super::*;

pub(super) fn active_matches(
    subscriptions: &BTreeMap<CorrelationId, ActiveSubscription>,
    acknowledgement: &SubscriptionAcknowledgement,
) -> bool {
    subscriptions
        .get(&acknowledgement.subscription_id)
        .is_some_and(|active| {
            active.snapshot.connection_generation == acknowledgement.connection_generation
        })
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn poll_subscriptions(
    socket: &mut WebSocket,
    sender: &mut WebSocketSendState,
    subscriptions: &mut BTreeMap<CorrelationId, ActiveSubscription>,
    state: &Arc<AppState>,
    session_id: &CorrelationId,
    token: &CorrelationId,
    limits: &WebSocketLimits,
) -> std::result::Result<(), ()> {
    let now_instant = Instant::now();
    let due = subscriptions
        .iter()
        .filter(|(_, active)| active.next_poll_at <= now_instant)
        .map(|(id, active)| (id.clone(), active.snapshot.clone()))
        .collect::<Vec<_>>();
    for (subscription_id, snapshot) in due {
        if let Some(active) = subscriptions.get_mut(&subscription_id) {
            active.next_poll_at =
                now_instant + Duration::from_millis(active.snapshot.heartbeat_interval_ms.max(1));
        }
        let now = unix_time_ms();
        let context = generated_context(now, "subscription-delivery");
        let state_for_call = Arc::clone(state);
        let session_for_call = session_id.clone();
        let token_for_call = token.clone();
        let subscription_for_call = subscription_id.clone();
        let result = tokio::task::spawn_blocking(move || {
            state_for_call.service.next_subscription_delivery(
                &session_for_call,
                &token_for_call,
                &subscription_for_call,
                snapshot.connection_generation,
                now,
                context.request_id.as_str(),
                context.operation_id.as_str(),
            )
        })
        .await;
        match result {
            Ok(Ok(SubscriptionPoll::Idle { subscription })) => {
                if let Some(active) = subscriptions.get_mut(&subscription_id) {
                    active.snapshot = subscription;
                }
            }
            Ok(Ok(SubscriptionPoll::Delivery { batch })) => {
                if send_payload(
                    socket,
                    sender,
                    WebSocketPayload::Delivery(WebSocketDelivery::from(batch)),
                    limits,
                )
                .await
                .is_err()
                {
                    return Err(());
                }
            }
            Ok(Err(ServiceError::SubscriptionBackpressure)) => {
                if send_payload(
                    socket,
                    sender,
                    WebSocketPayload::Backpressure(WebSocketBackpressure {
                        target: WebSocketBackpressureTarget::Subscription {
                            subscription_id: subscription_id.clone(),
                            connection_generation: snapshot.connection_generation,
                        },
                        in_flight: u32::from(snapshot.max_in_flight),
                        limit: u32::from(snapshot.max_in_flight),
                        retry_after_ms: snapshot.heartbeat_interval_ms,
                    }),
                    limits,
                )
                .await
                .is_err()
                {
                    return Err(());
                }
            }
            Ok(Err(error)) => {
                let target = WebSocketErrorTarget::Subscription {
                    subscription_id: subscription_id.clone(),
                    connection_generation: snapshot.connection_generation,
                };
                send_service_error(socket, sender, target, error, limits).await?;
                subscriptions.remove(&subscription_id);
            }
            Err(error) => {
                send_error(
                    socket,
                    sender,
                    WebSocketErrorTarget::Subscription {
                        subscription_id: subscription_id.clone(),
                        connection_generation: snapshot.connection_generation,
                    },
                    ErrorCode::Internal,
                    error.to_string(),
                    limits,
                )
                .await?;
                subscriptions.remove(&subscription_id);
            }
        }
    }
    Ok(())
}

pub(super) async fn invoke_subscription_connect(
    state: Arc<AppState>,
    session_id: CorrelationId,
    token: CorrelationId,
    resume: SubscriptionResume,
) -> std::result::Result<SubscriptionSnapshot, SubscriptionInvocationError> {
    tokio::task::spawn_blocking(move || {
        invoke_subscription_operation(
            &state,
            &session_id,
            &token,
            RrdOperation::SubscriptionConnect,
            &resume,
            |context| {
                state.service.connect_subscription(
                    &session_id,
                    &token,
                    &resume,
                    context.0,
                    context.1.request_id.as_str(),
                    context.1.operation_id.as_str(),
                )
            },
        )
    })
    .await
    .map_err(|error| SubscriptionInvocationError::Worker(error.to_string()))?
    .map_err(SubscriptionInvocationError::Service)
}

pub(super) async fn invoke_subscription_ack(
    state: Arc<AppState>,
    session_id: CorrelationId,
    token: CorrelationId,
    acknowledgement: SubscriptionAcknowledgement,
) -> std::result::Result<SubscriptionSnapshot, SubscriptionInvocationError> {
    tokio::task::spawn_blocking(move || {
        invoke_subscription_operation(
            &state,
            &session_id,
            &token,
            RrdOperation::SubscriptionAck,
            &acknowledgement,
            |context| {
                state.service.acknowledge_subscription(
                    &session_id,
                    &token,
                    &acknowledgement,
                    context.0,
                    context.1.request_id.as_str(),
                    context.1.operation_id.as_str(),
                )
            },
        )
    })
    .await
    .map_err(|error| SubscriptionInvocationError::Worker(error.to_string()))?
    .map_err(SubscriptionInvocationError::Service)
}

pub(super) async fn invoke_subscription_heartbeat(
    state: Arc<AppState>,
    session_id: CorrelationId,
    token: CorrelationId,
    coordinate: SubscriptionLeaseCoordinate,
) -> std::result::Result<SubscriptionSnapshot, SubscriptionInvocationError> {
    tokio::task::spawn_blocking(move || {
        invoke_subscription_operation(
            &state,
            &session_id,
            &token,
            RrdOperation::SubscriptionAck,
            &coordinate,
            |context| {
                state.service.renew_subscription_lease(
                    &session_id,
                    &token,
                    &coordinate,
                    context.0,
                    context.1.request_id.as_str(),
                    context.1.operation_id.as_str(),
                )
            },
        )
    })
    .await
    .map_err(|error| SubscriptionInvocationError::Worker(error.to_string()))?
    .map_err(SubscriptionInvocationError::Service)
}

pub(super) fn invoke_subscription_operation<T: serde::Serialize>(
    state: &AppState,
    session_id: &CorrelationId,
    token: &CorrelationId,
    operation: RrdOperation,
    request: &T,
    call: impl FnOnce((u64, RequestContext)) -> std::result::Result<SubscriptionSnapshot, ServiceError>,
) -> std::result::Result<SubscriptionSnapshot, ServiceError> {
    let now = unix_time_ms();
    let mut context = generated_context(now, "websocket-subscription-operation");
    context.idempotency_key = Some(context.operation_id.clone());
    let authorized = state.service.begin_invocation(
        Invocation {
            context: context.clone(),
            resource: instance_resource(state.service.instance_id()),
            observed_at_unix_ms: now,
            attempt: HTTP_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            request_sha256: sha256_hex(
                &serde_json::to_vec(request).expect("subscription operation serializes"),
            ),
        },
        operation,
        InvocationCredential::Session { session_id, token },
    )?;
    let result = call((now, context));
    let completion = match &result {
        Ok(snapshot) => InvocationCompletion {
            decision: AuditDecision::Allowed,
            status_code: StatusCode::OK.as_u16(),
            response_sha256: sha256_hex(
                &serde_json::to_vec(snapshot).expect("subscription snapshot serializes"),
            ),
        },
        Err(error) => InvocationCompletion {
            decision: if matches!(
                error.kind(),
                ServiceErrorKind::Unauthenticated | ServiceErrorKind::PermissionDenied
            ) {
                AuditDecision::Denied
            } else {
                AuditDecision::Failed
            },
            status_code: service_status(error.kind()).as_u16(),
            response_sha256: sha256_hex(error.to_string().as_bytes()),
        },
    };
    state.service.complete_invocation(&authorized, completion)?;
    result
}

pub(super) async fn send_service_error(
    socket: &mut WebSocket,
    sender: &mut WebSocketSendState,
    target: WebSocketErrorTarget,
    error: ServiceError,
    limits: &WebSocketLimits,
) -> std::result::Result<(), ()> {
    let error = websocket_error(error);
    send_error(socket, sender, target, error.code, error.message, limits).await
}

pub(super) async fn send_subscription_invocation_error(
    socket: &mut WebSocket,
    sender: &mut WebSocketSendState,
    target: WebSocketErrorTarget,
    error: SubscriptionInvocationError,
    limits: &WebSocketLimits,
) -> std::result::Result<(), ()> {
    match error {
        SubscriptionInvocationError::Service(error) => {
            send_service_error(socket, sender, target, error, limits).await
        }
        SubscriptionInvocationError::Worker(message) => {
            send_error(socket, sender, target, ErrorCode::Internal, message, limits).await
        }
    }
}

pub(super) async fn send_error(
    socket: &mut WebSocket,
    sender: &mut WebSocketSendState,
    target: WebSocketErrorTarget,
    code: ErrorCode,
    message: impl Into<String>,
    limits: &WebSocketLimits,
) -> std::result::Result<(), ()> {
    let message = bounded_message(message.into());
    send_payload(
        socket,
        sender,
        WebSocketPayload::Error(WebSocketError {
            target,
            error: ErrorBody {
                code,
                message,
                retryable: false,
                details: BTreeMap::new(),
            },
        }),
        limits,
    )
    .await
}

pub(super) fn bounded_message(mut message: String) -> String {
    if message.is_empty() {
        return "RRFlow WebSocket error".into();
    }
    while message.len() > rrd_contract::MAX_MESSAGE_BYTES {
        message.pop();
    }
    message
}

pub(super) async fn send_payload(
    socket: &mut WebSocket,
    sender: &mut WebSocketSendState,
    payload: WebSocketPayload,
    limits: &WebSocketLimits,
) -> std::result::Result<(), ()> {
    let bytes = sender.encode(payload).map_err(|_| ())?;
    let text = String::from_utf8(bytes).map_err(|_| ())?;
    send_transport(socket, Message::Text(text.into()), limits).await
}

pub(super) async fn send_transport(
    socket: &mut WebSocket,
    message: Message,
    limits: &WebSocketLimits,
) -> std::result::Result<(), ()> {
    tokio::time::timeout(
        Duration::from_millis(limits.send_timeout_ms),
        socket.send(message),
    )
    .await
    .map_err(|_| ())?
    .map_err(|_| ())
}

pub(super) async fn close_socket(socket: &mut WebSocket, code: u16, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await;
}

pub(super) fn service_status(kind: ServiceErrorKind) -> StatusCode {
    match kind {
        ServiceErrorKind::InvalidArgument => StatusCode::BAD_REQUEST,
        ServiceErrorKind::NotFound => StatusCode::NOT_FOUND,
        ServiceErrorKind::Unauthenticated => StatusCode::UNAUTHORIZED,
        ServiceErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
        ServiceErrorKind::Conflict => StatusCode::CONFLICT,
        ServiceErrorKind::FailedPrecondition => StatusCode::PRECONDITION_FAILED,
        ServiceErrorKind::ResourceExhausted => StatusCode::TOO_MANY_REQUESTS,
        ServiceErrorKind::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
        ServiceErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
