//! Subscription acknowledgements, lease coordinates and WebSocket frames.

use super::*;

impl From<SubscriptionDeliveryBatch> for WebSocketDelivery {
    fn from(value: SubscriptionDeliveryBatch) -> Self {
        Self {
            subscription_id: value.subscription_id,
            connection_generation: value.connection_generation,
            delivery_sequence: value.delivery_sequence,
            from_cursor: value.from_cursor,
            through_cursor: value.through_cursor,
            delivery: value.delivery,
        }
    }
}

/// Exact cumulative acknowledgement applied by `RrdEngine`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionAcknowledgement {
    pub subscription_id: CorrelationId,
    pub connection_generation: u64,
    pub delivery_sequence: u64,
    pub through_cursor: u64,
}

impl SubscriptionAcknowledgement {
    pub fn validate(&self) -> Result<()> {
        if self.connection_generation == 0 || self.delivery_sequence == 0 {
            return invalid(
                "subscription acknowledgement generation and sequence must be positive",
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WebSocketAcknowledged {
    pub acknowledgement: SubscriptionAcknowledgement,
    pub subscription: SubscriptionSnapshot,
}

impl WebSocketAcknowledged {
    pub(super) fn validate(&self) -> Result<()> {
        self.acknowledgement.validate()?;
        validate_subscription_snapshot(&self.subscription, true)?;
        if self.acknowledgement.subscription_id != self.subscription.subscription_id
            || self.acknowledgement.connection_generation != self.subscription.connection_generation
            || self.acknowledgement.through_cursor != self.subscription.acknowledged_cursor
        {
            return invalid("acknowledged result does not match its subscription coordinates");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionLeaseCoordinate {
    pub subscription_id: CorrelationId,
    pub connection_generation: u64,
    pub acknowledged_cursor: u64,
}

impl SubscriptionLeaseCoordinate {
    pub fn from_snapshot(snapshot: &SubscriptionSnapshot) -> Self {
        Self {
            subscription_id: snapshot.subscription_id.clone(),
            connection_generation: snapshot.connection_generation,
            acknowledged_cursor: snapshot.acknowledged_cursor,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.connection_generation == 0 {
            return invalid("subscription lease generation must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WebSocketHeartbeat {
    pub heartbeat_id: CorrelationId,
    pub observed_sequence: u64,
    pub subscriptions: Vec<SubscriptionLeaseCoordinate>,
}

impl WebSocketHeartbeat {
    pub(super) fn validate(&self, limits: &WebSocketLimits) -> Result<()> {
        if self.observed_sequence == 0
            || self.subscriptions.len() > usize::from(limits.max_subscriptions)
        {
            return invalid("WebSocket heartbeat coordinates exceed their bounds");
        }
        let mut ids = BTreeSet::new();
        for subscription in &self.subscriptions {
            subscription.validate()?;
            if !ids.insert(&subscription.subscription_id) {
                return invalid("WebSocket heartbeat subscription identities must be unique");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WebSocketUnsubscribe {
    pub subscription_id: CorrelationId,
    pub connection_generation: u64,
}

impl WebSocketUnsubscribe {
    pub(super) fn validate(&self) -> Result<()> {
        if self.connection_generation == 0 {
            return invalid("unsubscribe connection generation must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WebSocketUnsubscribed {
    pub subscription: SubscriptionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WebSocketErrorTarget {
    Connection,
    Request {
        target: WebSocketRequestTarget,
    },
    Cancellation {
        cancellation_id: CorrelationId,
        target: WebSocketRequestTarget,
    },
    Subscription {
        subscription_id: CorrelationId,
        connection_generation: u64,
    },
}

impl WebSocketErrorTarget {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Request { target } | Self::Cancellation { target, .. } => target.validate()?,
            Self::Connection | Self::Subscription { .. } => {}
        }
        if matches!(
            self,
            Self::Subscription {
                connection_generation: 0,
                ..
            }
        ) {
            return invalid("subscription error generation must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WebSocketError {
    pub target: WebSocketErrorTarget,
    pub error: ErrorBody,
}

impl WebSocketError {
    pub(super) fn validate(&self) -> Result<()> {
        self.target.validate()?;
        self.error.validate()?;
        if self.error.details.len() > MAX_WEBSOCKET_ERROR_DETAILS
            || serde_json::to_vec(&self.error)
                .map_err(|error| ContractError(error.to_string()))?
                .len()
                > MAX_WEBSOCKET_ERROR_BYTES
        {
            return invalid("WebSocket error exceeds its detail or encoded-byte bound");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WebSocketBackpressureTarget {
    Request {
        target: WebSocketRequestTarget,
    },
    Subscription {
        subscription_id: CorrelationId,
        connection_generation: u64,
    },
}

impl WebSocketBackpressureTarget {
    fn validate(&self) -> Result<()> {
        if let Self::Request { target } = self {
            target.validate()?;
        }
        if matches!(
            self,
            Self::Subscription {
                connection_generation: 0,
                ..
            }
        ) {
            return invalid("subscription backpressure generation must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WebSocketBackpressure {
    pub target: WebSocketBackpressureTarget,
    pub in_flight: u32,
    pub limit: u32,
    pub retry_after_ms: u64,
}

impl WebSocketBackpressure {
    pub(super) fn validate(&self) -> Result<()> {
        self.target.validate()?;
        if self.limit == 0
            || self.in_flight < self.limit
            || self.retry_after_ms == 0
            || self.retry_after_ms > MAX_WEBSOCKET_RETRY_AFTER_MS
        {
            return invalid("WebSocket backpressure coordinates are invalid");
        }
        Ok(())
    }
}

pub fn encode_websocket_frame(
    frame: &WebSocketFrame,
    sender: WebSocketPeer,
    limits: &WebSocketLimits,
) -> Result<Vec<u8>> {
    frame.validate(sender, limits)?;
    let bytes = serde_json::to_vec(frame).map_err(|error| ContractError(error.to_string()))?;
    if bytes.len() > limits.max_message_bytes as usize {
        return invalid("encoded WebSocket message exceeds the negotiated byte limit");
    }
    Ok(bytes)
}

pub fn decode_websocket_frame(
    bytes: &[u8],
    sender: WebSocketPeer,
    limits: &WebSocketLimits,
) -> Result<WebSocketFrame> {
    limits.validate()?;
    if bytes.len() > limits.max_message_bytes as usize {
        return invalid("WebSocket message exceeds the negotiated byte limit");
    }
    let frame = serde_json::from_slice::<WebSocketFrame>(bytes)
        .map_err(|error| ContractError(format!("invalid WebSocket frame: {error}")))?;
    frame.validate(sender, limits)?;
    Ok(frame)
}

/// Transport-local monotonically sequenced encoder. It is deliberately not
/// serializable and must never be persisted by `RrdEngine`.
#[derive(Debug, Clone)]
pub struct WebSocketSendState {
    sender: WebSocketPeer,
    connection_id: CorrelationId,
    limits: WebSocketLimits,
    last_sequence: u64,
}

impl WebSocketSendState {
    pub fn new(
        sender: WebSocketPeer,
        connection_id: CorrelationId,
        limits: WebSocketLimits,
    ) -> Result<Self> {
        limits.validate()?;
        Ok(Self {
            sender,
            connection_id,
            limits,
            last_sequence: 0,
        })
    }

    pub fn encode(&mut self, payload: WebSocketPayload) -> Result<Vec<u8>> {
        let sequence = self
            .last_sequence
            .checked_add(1)
            .ok_or_else(|| ContractError("WebSocket send sequence overflowed".into()))?;
        if self.sender == WebSocketPeer::Server
            && sequence == 1
            && !matches!(payload, WebSocketPayload::Connected(_))
        {
            return invalid("the first server application message must be connected");
        }
        let frame = WebSocketFrame {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            connection_id: self.connection_id.clone(),
            sequence,
            payload,
        };
        let bytes = encode_websocket_frame(&frame, self.sender, &self.limits)?;
        self.last_sequence = sequence;
        Ok(bytes)
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }
}

/// Transport-local decoder that rejects replay, gaps, cross-connection
/// substitution, and a server stream that does not begin with `connected`.
#[derive(Debug, Clone)]
pub struct WebSocketReceiveState {
    sender: WebSocketPeer,
    connection_id: Option<CorrelationId>,
    limits: WebSocketLimits,
    last_sequence: u64,
}

impl WebSocketReceiveState {
    pub fn awaiting_server(limits: WebSocketLimits) -> Result<Self> {
        limits.validate()?;
        Ok(Self {
            sender: WebSocketPeer::Server,
            connection_id: None,
            limits,
            last_sequence: 0,
        })
    }

    pub fn for_connection(
        sender: WebSocketPeer,
        connection_id: CorrelationId,
        limits: WebSocketLimits,
    ) -> Result<Self> {
        limits.validate()?;
        Ok(Self {
            sender,
            connection_id: Some(connection_id),
            limits,
            last_sequence: 0,
        })
    }

    pub fn accept(&mut self, bytes: &[u8]) -> Result<WebSocketFrame> {
        let frame = decode_websocket_frame(bytes, self.sender, &self.limits)?;
        let expected = self
            .last_sequence
            .checked_add(1)
            .ok_or_else(|| ContractError("WebSocket receive sequence overflowed".into()))?;
        if frame.sequence != expected {
            return invalid(format!(
                "WebSocket sequence {} is not the expected {expected}",
                frame.sequence
            ));
        }
        match &self.connection_id {
            Some(connection_id) if *connection_id != frame.connection_id => {
                return invalid("WebSocket frame carries another connection identity");
            }
            Some(_) if self.sender == WebSocketPeer::Server && expected == 1 => {
                if !matches!(frame.payload, WebSocketPayload::Connected(_)) {
                    return invalid("the first server application message must be connected");
                }
            }
            Some(_) => {}
            None => {
                let WebSocketPayload::Connected(connected) = &frame.payload else {
                    return invalid("the server stream did not begin with connected");
                };
                connected.limits.admitted_by(&self.limits)?;
                self.limits = connected.limits.clone();
                self.connection_id = Some(frame.connection_id.clone());
            }
        }
        self.last_sequence = frame.sequence;
        Ok(frame)
    }

    pub fn connection_id(&self) -> Option<&CorrelationId> {
        self.connection_id.as_ref()
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }
}

pub fn validate_subscription_snapshot(
    subscription: &SubscriptionSnapshot,
    connected: bool,
) -> Result<()> {
    validate_sha256(&subscription.stream_sha256, "subscription stream_sha256")?;
    if subscription.acknowledged_cursor > subscription.head_cursor
        || subscription.retention_floor_cursor > subscription.acknowledged_cursor
        || subscription.batch_size == 0
        || subscription.batch_size > MAX_CHANGEFEED_PAGE
        || subscription.max_in_flight == 0
        || subscription.max_in_flight > MAX_SUBSCRIPTION_IN_FLIGHT
        || subscription.heartbeat_interval_ms < MIN_SUBSCRIPTION_HEARTBEAT_MS
        || subscription.heartbeat_interval_ms > MAX_SUBSCRIPTION_HEARTBEAT_MS
        || (connected && subscription.connection_generation == 0)
        || (!connected && subscription.connection_generation != 0)
        || (subscription.status == SubscriptionStatus::Open
            && subscription.lease_expires_at_unix_ms == 0)
    {
        return invalid("subscription snapshot coordinates are inconsistent");
    }
    Ok(())
}

pub(super) fn validate_json_value(value: &serde_json::Value) -> Result<()> {
    fn walk(value: &serde_json::Value, depth: usize, items: &mut usize) -> Result<()> {
        *items = items.saturating_add(1);
        if depth > MAX_WEBSOCKET_JSON_DEPTH || *items > MAX_WEBSOCKET_JSON_ITEMS {
            return invalid("WebSocket JSON payload exceeds its depth or item bound");
        }
        match value {
            serde_json::Value::String(value) if value.len() > MAX_WEBSOCKET_MESSAGE_BYTES => {
                invalid("WebSocket JSON string exceeds the message byte bound")
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    walk(value, depth + 1, items)?;
                }
                Ok(())
            }
            serde_json::Value::Object(values) => {
                for (name, value) in values {
                    if name.len() > MAX_ID_BYTES {
                        return invalid("WebSocket JSON object key exceeds the identifier bound");
                    }
                    walk(value, depth + 1, items)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    walk(value, 0, &mut 0)
}
