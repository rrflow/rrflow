//! Engine references, events and transport observations.

use super::*;

/// The RROFLOW v1 receiver retains these as reported references, not source attestation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineReferences {
    pub version: u16,
    pub instance_id: String,
    pub scope: String,
    pub operation: String,
    pub response_sha256: String,
    pub task_id: Option<String>,
    pub attempt_id: Option<String>,
    pub job_id: Option<String>,
    pub plan_sha256: Option<String>,
    pub source_sha256: Option<String>,
    pub configuration_sha256: Option<String>,
}

pub(super) fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
}
pub(super) fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, b)| {
            if [8, 13, 18, 23].contains(&index) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

impl EngineReferences {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 1
            || !id(&self.instance_id)
            || !id(&self.scope)
            || !id(&self.operation)
            || !digest(&self.response_sha256)
            || [&self.task_id, &self.attempt_id, &self.job_id]
                .iter()
                .any(|value| value.as_deref().is_some_and(|value| !id(value)))
            || [
                &self.plan_sha256,
                &self.source_sha256,
                &self.configuration_sha256,
            ]
            .iter()
            .any(|value| value.as_deref().is_some_and(|value| !digest(value)))
            || (self.task_id.is_none() && self.job_id.is_none())
            || (self.attempt_id.is_some() && self.task_id.is_none())
        {
            return Err("invalid engine observation references");
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub(super) struct Event<'a> {
    schema: &'static str,
    event_id: &'a str,
    capture_id: &'a str,
    sequence: u64,
    agent: &'static str,
    session_id: &'a str,
    operation_id: &'a str,
    observed_at: String,
    kind: &'static str,
    engine: &'a EngineReferences,
}

/// Encode only the closed metadata projection; no arbitrary payload is accepted.
pub fn encode_event(
    event_id: &str,
    capture_id: &str,
    delivery_id: &str,
    observed_at_unix_ms: u64,
    references: &EngineReferences,
) -> Result<Vec<u8>, &'static str> {
    references.validate()?;
    if !uuid(event_id) || !uuid(capture_id) || !id(delivery_id) || observed_at_unix_ms == 0 {
        return Err("invalid observation envelope identity or time");
    }
    let time = i64::try_from(observed_at_unix_ms).map_err(|_| "invalid observation time")?;
    let at = chrono::DateTime::from_timestamp_millis(time).ok_or("invalid observation time")?;
    let event = Event {
        schema: "rroflow.capture/v1",
        event_id,
        capture_id,
        sequence: 0,
        agent: "rrflow",
        session_id: capture_id,
        operation_id: delivery_id,
        observed_at: at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        kind: "engine.observed",
        engine: references,
    };
    #[derive(Serialize)]
    struct Batch<'a> {
        events: [Event<'a>; 1],
    }
    let bytes = serde_json::to_vec(&Batch { events: [event] })
        .map_err(|_| "observation encoding failed")?;
    if bytes.len() > MAX_OBSERVATION_EVENT_BYTES {
        return Err("observation exceeds byte limit");
    }
    Ok(bytes)
}

/// Facts returned to the engine. The adapter does not accept a delivery receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportObservation {
    pub disposition: ObservationDeliveryDisposition,
    pub request_may_have_been_sent: bool,
    pub http_status: Option<u16>,
    pub reply_sha256: Option<String>,
}
impl TransportObservation {
    pub(super) fn failure(disposition: ObservationDeliveryDisposition, sent: bool) -> Self {
        Self {
            disposition,
            request_may_have_been_sent: sent,
            http_status: None,
            reply_sha256: None,
        }
    }
}
