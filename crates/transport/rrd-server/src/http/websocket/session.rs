//! WebSocket upgrade, the client session loop and client payload handling.

use super::*;

pub(super) const SOCKET_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(super) struct ActiveSubscription {
    pub(super) snapshot: SubscriptionSnapshot,
    pub(super) next_poll_at: Instant,
}

pub(super) enum SubscriptionInvocationError {
    Service(ServiceError),
    Worker(String),
}

pub(in super::super) async fn websocket_upgrade(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    websocket: WebSocketUpgrade,
) -> HttpResponse {
    let now = unix_time_ms();
    let mut context = generated_context(now, "websocket-connect");
    context.idempotency_key = Some(context.operation_id.clone());
    let (session_id, token) = match authenticated_session(&headers, None) {
        Ok(session) => session,
        Err(error) => return failure(&context, error),
    };
    let connection_id = context.request_id.clone();
    let limits = WebSocketLimits::default();
    let authorized = match state.service.begin_invocation(
        Invocation {
            context: context.clone(),
            resource: instance_resource(state.service.instance_id()),
            observed_at_unix_ms: now,
            attempt: HTTP_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            request_sha256: sha256_hex(b"rrd-websocket-connect-v1"),
        },
        RrdOperation::WebSocketConnect,
        InvocationCredential::Session {
            session_id: &session_id,
            token: &token,
        },
    ) {
        Ok(authorized) => authorized,
        Err(error) => return failure(&context, api_error(error)),
    };
    let response_sha256 = sha256_hex(
        &serde_json::to_vec(&(&connection_id, &limits))
            .expect("WebSocket connection coordinates serialize"),
    );
    if let Err(error) = state.service.complete_invocation(
        &authorized,
        InvocationCompletion {
            decision: AuditDecision::Allowed,
            // The audit contract records completed logical outcomes (2xx+),
            // while the wire handshake itself returns 101 Switching Protocols.
            status_code: StatusCode::OK.as_u16(),
            response_sha256,
        },
    ) {
        return failure(&context, api_error(error));
    }

    websocket
        .read_buffer_size(limits.read_buffer_bytes as usize)
        .write_buffer_size(limits.write_buffer_bytes as usize)
        .max_write_buffer_size(limits.max_write_buffer_bytes as usize)
        .max_message_size(limits.max_message_bytes as usize)
        .max_frame_size(limits.max_frame_bytes as usize)
        .on_upgrade(move |socket| {
            serve_websocket(socket, state, session_id, token, connection_id, limits)
        })
}

pub(super) async fn serve_websocket(
    mut socket: WebSocket,
    state: Arc<AppState>,
    session_id: CorrelationId,
    token: CorrelationId,
    connection_id: CorrelationId,
    limits: WebSocketLimits,
) {
    let mut sender =
        match WebSocketSendState::new(WebSocketPeer::Server, connection_id.clone(), limits.clone())
        {
            Ok(sender) => sender,
            Err(_) => return,
        };
    let mut receiver = match WebSocketReceiveState::for_connection(
        WebSocketPeer::Client,
        connection_id,
        limits.clone(),
    ) {
        Ok(receiver) => receiver,
        Err(_) => return,
    };
    if send_payload(
        &mut socket,
        &mut sender,
        WebSocketPayload::Connected(WebSocketConnected {
            contract_version: WEBSOCKET_CONTRACT_VERSION,
            session_id: session_id.clone(),
            limits: limits.clone(),
        }),
        &limits,
    )
    .await
    .is_err()
    {
        return;
    }

    let mut subscriptions = BTreeMap::<CorrelationId, ActiveSubscription>::new();
    let mut poll = tokio::time::interval(SOCKET_POLL_INTERVAL);
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    poll.tick().await;
    let mut last_inbound = Instant::now();
    let mut next_heartbeat = last_inbound + Duration::from_millis(limits.heartbeat_interval_ms);
    let mut last_client_observed_sequence = 0;

    loop {
        let receive_deadline = last_inbound + Duration::from_millis(limits.receive_timeout_ms);
        tokio::select! {
            inbound = socket.recv() => {
                match inbound {
                    Some(Ok(Message::Text(text))) => {
                        last_inbound = Instant::now();
                        let frame = match receiver.accept(text.as_bytes()) {
                            Ok(frame) => frame,
                            Err(error) => {
                                let _ = send_error(
                                    &mut socket,
                                    &mut sender,
                                    WebSocketErrorTarget::Connection,
                                    ErrorCode::InvalidArgument,
                                    error.to_string(),
                                    &limits,
                                ).await;
                                close_socket(&mut socket, close_code::PROTOCOL, "invalid RRFlow frame").await;
                                return;
                            }
                        };
                        if handle_client_payload(
                            &mut socket,
                            &mut sender,
                            &mut subscriptions,
                            &state,
                            &session_id,
                            &token,
                            frame.payload,
                            receiver.last_sequence(),
                            &mut last_client_observed_sequence,
                            &limits,
                        ).await.is_err() {
                            close_socket(&mut socket, close_code::PROTOCOL, "RRFlow protocol failure").await;
                            return;
                        }
                    }
                    Some(Ok(Message::Ping(bytes))) => {
                        last_inbound = Instant::now();
                        if send_transport(&mut socket, Message::Pong(bytes), &limits).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => last_inbound = Instant::now(),
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                    Some(Ok(Message::Binary(_))) => {
                        close_socket(&mut socket, close_code::UNSUPPORTED, "text application messages required").await;
                        return;
                    }
                }
            }
            _ = poll.tick() => {
                if poll_subscriptions(
                    &mut socket,
                    &mut sender,
                    &mut subscriptions,
                    &state,
                    &session_id,
                    &token,
                    &limits,
                ).await.is_err() {
                    return;
                }
                if Instant::now() >= next_heartbeat && receiver.last_sequence() > 0 {
                    let heartbeat_id = generated_context(unix_time_ms(), "websocket-heartbeat").request_id;
                    let coordinates = subscriptions
                        .values()
                        .map(|active| SubscriptionLeaseCoordinate::from_snapshot(&active.snapshot))
                        .collect();
                    if send_payload(
                        &mut socket,
                        &mut sender,
                        WebSocketPayload::Heartbeat(WebSocketHeartbeat {
                            heartbeat_id,
                            observed_sequence: receiver.last_sequence(),
                            subscriptions: coordinates,
                        }),
                        &limits,
                    ).await.is_err() {
                        return;
                    }
                    next_heartbeat = Instant::now() + Duration::from_millis(limits.heartbeat_interval_ms);
                }
            }
            _ = tokio::time::sleep_until(receive_deadline) => {
                let _ = send_error(
                    &mut socket,
                    &mut sender,
                    WebSocketErrorTarget::Connection,
                    ErrorCode::DeadlineExceeded,
                    "WebSocket receive deadline elapsed",
                    &limits,
                ).await;
                close_socket(&mut socket, close_code::POLICY, "receive deadline elapsed").await;
                return;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_client_payload(
    socket: &mut WebSocket,
    sender: &mut WebSocketSendState,
    subscriptions: &mut BTreeMap<CorrelationId, ActiveSubscription>,
    state: &Arc<AppState>,
    session_id: &CorrelationId,
    token: &CorrelationId,
    payload: WebSocketPayload,
    observed_sequence: u64,
    last_client_observed_sequence: &mut u64,
    limits: &WebSocketLimits,
) -> std::result::Result<(), ()> {
    match payload {
        WebSocketPayload::Request(request) => {
            send_error(
                socket,
                sender,
                WebSocketErrorTarget::Request {
                    target: request.target(),
                },
                ErrorCode::FailedPrecondition,
                "WebSocket operation execution is not enabled until H-04 binds the shared dispatch",
                limits,
            )
            .await
        }
        WebSocketPayload::Cancel(cancel) => {
            send_payload(
                socket,
                sender,
                WebSocketPayload::Cancellation(WebSocketCancellation {
                    cancellation_id: cancel.cancellation_id,
                    target: cancel.target,
                    disposition: WebSocketCancellationDisposition::NotFound,
                }),
                limits,
            )
            .await
        }
        WebSocketPayload::Subscribe(request) => {
            let expected_generation = request
                .resume
                .connection_generation
                .saturating_add(1)
                .max(1);
            if subscriptions.contains_key(&request.resume.subscription_id) {
                return send_error(
                    socket,
                    sender,
                    WebSocketErrorTarget::Subscription {
                        subscription_id: request.resume.subscription_id,
                        connection_generation: expected_generation,
                    },
                    ErrorCode::Conflict,
                    "subscription is already attached to this connection",
                    limits,
                )
                .await;
            }
            if subscriptions.len() >= usize::from(limits.max_subscriptions) {
                return send_payload(
                    socket,
                    sender,
                    WebSocketPayload::Backpressure(WebSocketBackpressure {
                        target: WebSocketBackpressureTarget::Subscription {
                            subscription_id: request.resume.subscription_id,
                            connection_generation: expected_generation,
                        },
                        in_flight: subscriptions.len() as u32,
                        limit: u32::from(limits.max_subscriptions),
                        retry_after_ms: limits.heartbeat_interval_ms,
                    }),
                    limits,
                )
                .await;
            }
            let resume = request.resume;
            let result = invoke_subscription_connect(
                Arc::clone(state),
                session_id.clone(),
                token.clone(),
                resume.clone(),
            )
            .await;
            match result {
                Ok(snapshot) => {
                    subscriptions.insert(
                        snapshot.subscription_id.clone(),
                        ActiveSubscription {
                            snapshot: snapshot.clone(),
                            next_poll_at: Instant::now(),
                        },
                    );
                    send_payload(
                        socket,
                        sender,
                        WebSocketPayload::Subscribed(WebSocketSubscribed {
                            subscription: snapshot,
                        }),
                        limits,
                    )
                    .await
                }
                Err(error) => {
                    send_subscription_invocation_error(
                        socket,
                        sender,
                        WebSocketErrorTarget::Subscription {
                            subscription_id: resume.subscription_id,
                            connection_generation: expected_generation,
                        },
                        error,
                        limits,
                    )
                    .await
                }
            }
        }
        WebSocketPayload::Ack(acknowledgement) => {
            if !active_matches(subscriptions, &acknowledgement) {
                return send_error(
                    socket,
                    sender,
                    WebSocketErrorTarget::Subscription {
                        subscription_id: acknowledgement.subscription_id,
                        connection_generation: acknowledgement.connection_generation,
                    },
                    ErrorCode::FailedPrecondition,
                    "ACK does not target an active subscription generation",
                    limits,
                )
                .await;
            }
            let result = invoke_subscription_ack(
                Arc::clone(state),
                session_id.clone(),
                token.clone(),
                acknowledgement.clone(),
            )
            .await;
            match result {
                Ok(snapshot) => {
                    if let Some(active) = subscriptions.get_mut(&snapshot.subscription_id) {
                        active.snapshot = snapshot.clone();
                    }
                    send_payload(
                        socket,
                        sender,
                        WebSocketPayload::Acknowledged(WebSocketAcknowledged {
                            acknowledgement,
                            subscription: snapshot,
                        }),
                        limits,
                    )
                    .await
                }
                Err(error) => {
                    let target = WebSocketErrorTarget::Subscription {
                        subscription_id: acknowledgement.subscription_id,
                        connection_generation: acknowledgement.connection_generation,
                    };
                    send_subscription_invocation_error(socket, sender, target, error, limits).await
                }
            }
        }
        WebSocketPayload::Heartbeat(heartbeat) => {
            if heartbeat.observed_sequence > sender.last_sequence()
                || heartbeat.observed_sequence < *last_client_observed_sequence
            {
                return send_error(
                    socket,
                    sender,
                    WebSocketErrorTarget::Connection,
                    ErrorCode::FailedPrecondition,
                    "heartbeat peer sequence is ahead of the server or moved backward",
                    limits,
                )
                .await;
            }
            for coordinate in &heartbeat.subscriptions {
                let active = subscriptions.get(&coordinate.subscription_id);
                if active.is_none_or(|active| {
                    active.snapshot.connection_generation != coordinate.connection_generation
                        || active.snapshot.acknowledged_cursor != coordinate.acknowledged_cursor
                }) {
                    return send_error(
                        socket,
                        sender,
                        WebSocketErrorTarget::Subscription {
                            subscription_id: coordinate.subscription_id.clone(),
                            connection_generation: coordinate.connection_generation,
                        },
                        ErrorCode::FailedPrecondition,
                        "heartbeat does not target active durable coordinates",
                        limits,
                    )
                    .await;
                }
            }
            *last_client_observed_sequence = heartbeat.observed_sequence;
            let mut renewed = Vec::with_capacity(heartbeat.subscriptions.len());
            for coordinate in heartbeat.subscriptions {
                match invoke_subscription_heartbeat(
                    Arc::clone(state),
                    session_id.clone(),
                    token.clone(),
                    coordinate.clone(),
                )
                .await
                {
                    Ok(snapshot) => {
                        subscriptions
                            .get_mut(&snapshot.subscription_id)
                            .expect("active subscription was checked")
                            .snapshot = snapshot.clone();
                        renewed.push(SubscriptionLeaseCoordinate::from_snapshot(&snapshot));
                    }
                    Err(error) => {
                        let target = WebSocketErrorTarget::Subscription {
                            subscription_id: coordinate.subscription_id,
                            connection_generation: coordinate.connection_generation,
                        };
                        return send_subscription_invocation_error(
                            socket, sender, target, error, limits,
                        )
                        .await;
                    }
                }
            }
            send_payload(
                socket,
                sender,
                WebSocketPayload::Heartbeat(WebSocketHeartbeat {
                    heartbeat_id: heartbeat.heartbeat_id,
                    observed_sequence,
                    subscriptions: renewed,
                }),
                limits,
            )
            .await
        }
        WebSocketPayload::Unsubscribe(request) => {
            let active = subscriptions.get(&request.subscription_id);
            if active.is_none_or(|active| {
                active.snapshot.connection_generation != request.connection_generation
            }) {
                return send_error(
                    socket,
                    sender,
                    WebSocketErrorTarget::Subscription {
                        subscription_id: request.subscription_id,
                        connection_generation: request.connection_generation,
                    },
                    ErrorCode::FailedPrecondition,
                    "unsubscribe does not target an active subscription generation",
                    limits,
                )
                .await;
            }
            let snapshot = subscriptions
                .remove(&request.subscription_id)
                .expect("active subscription was checked")
                .snapshot;
            send_payload(
                socket,
                sender,
                WebSocketPayload::Unsubscribed(WebSocketUnsubscribed {
                    subscription: snapshot,
                }),
                limits,
            )
            .await
        }
        _ => {
            send_error(
                socket,
                sender,
                WebSocketErrorTarget::Connection,
                ErrorCode::InvalidArgument,
                "server-only WebSocket payload received from client",
                limits,
            )
            .await
        }
    }
}
