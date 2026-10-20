//! Prompt flights: governed, durable records of one prompt's passage through
//! the runtime boundary.
//!
//! A flight records only externally observable runtime and provider events at
//! one authenticated read stamp. It never requests, infers, or persists hidden
//! chain-of-thought. The engine owns the flight lifecycle; a client renders
//! flights and compares cohorts without maintaining a second flight protocol.

use crate::{
    invalid, sha256_bytes, validate_sha256, AssembleContext, CanonicalId, ContextEvidenceKind,
    ContextReadStamp, CorrelationId, DataReference, QueryValue, ReadEvidence, Result,
    MAX_CONTEXT_ITEMS, MAX_CONTEXT_QUERY_BYTES, MAX_CONTEXT_STORAGE_KEYS, MAX_ID_BYTES,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Wire version of the prompt-flight record. Unknown versions are rejected.
pub const PROMPT_FLIGHT_FORMAT_VERSION: u16 = 1;
/// Strict engine-owned record table holding one row per flight.
pub const PROMPT_FLIGHT_KIND: &str = "rrflow-prompt-flight";
/// Strict engine-owned append-only event table whose subject is a flight.
pub const PROMPT_FLIGHT_EVENT_KIND: &str = "rrflow-prompt-flight-event";
/// Strict engine-owned relation from a flight to each canonical record its
/// injected context used. These edges make every prompt a graph neighbourhood
/// that rrflowQL traverses and the temporal graph shows.
pub const PROMPT_FLIGHT_USE_KIND: &str = "rrflow-prompt-flight-used";
pub const MAX_PROMPT_FLIGHT_PROMPT_BYTES: usize = MAX_CONTEXT_QUERY_BYTES;
pub const MAX_PROMPT_FLIGHT_ACCEPTANCE_MARKER_BYTES: usize = 1_024;
pub const MAX_PROMPT_FLIGHT_EVENT_KIND_BYTES: usize = 64;
pub const MAX_PROMPT_FLIGHT_EVENT_LABEL_BYTES: usize = 256;
pub const MAX_PROMPT_FLIGHT_EVENT_DETAIL_BYTES: usize = 16 * 1024;
pub const MAX_PROMPT_FLIGHT_EVENT_DATA_BYTES: usize = 64 * 1024;
pub const MAX_PROMPT_FLIGHT_EVENTS: usize = 4_096;
pub const MAX_PROMPT_FLIGHT_PAGE: u16 = 256;
pub const MAX_PROMPT_FLIGHT_STORAGE_KEYS: u64 = MAX_CONTEXT_STORAGE_KEYS;

/// The arm that receives the prepared prompt packet.
///
/// `observe` prepares the packet inside the engine and launches nothing. Model
/// providers remain explicit opt-in adapters; one is added to this closed
/// vocabulary only together with its governed adapter, budget, and audit
/// evidence.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PromptFlightProvider {
    Observe,
}

impl PromptFlightProvider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "observe" => Ok(Self::Observe),
            other => invalid(format!("unknown prompt flight provider {other:?}")),
        }
    }
}

/// How much canonical context the flight injects ahead of the prompt.
///
/// `fresh` injects nothing and records a zero-context baseline. `pruned`
/// injects only the engine-assembled, prompt-matched `ContextPacket` within
/// the declared budget. Both arms read history; neither mutates it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PromptFlightContextMode {
    Fresh,
    Pruned,
}

impl PromptFlightContextMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Pruned => "pruned",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "fresh" => Ok(Self::Fresh),
            "pruned" => Ok(Self::Pruned),
            other => invalid(format!("unknown prompt flight context mode {other:?}")),
        }
    }
}

/// Terminal state of the recorded flight. The `observe` provider completes a
/// flight synchronously as `prepared`; provider arms extend this vocabulary
/// together with their adapters.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PromptFlightStatus {
    Prepared,
}

impl PromptFlightStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "prepared" => Ok(Self::Prepared),
            other => invalid(format!("unknown prompt flight status {other:?}")),
        }
    }
}

/// Runtime lane of one flight event. The vocabulary is closed so a client
/// cannot invent a lane; provider arms add `model` and `tool` lanes together
/// with their adapters.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PromptFlightStage {
    Prompt,
    Context,
    Outcome,
}

impl PromptFlightStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::Context => "context",
            Self::Outcome => "outcome",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "prompt" => Ok(Self::Prompt),
            "context" => Ok(Self::Context),
            "outcome" => Ok(Self::Outcome),
            other => invalid(format!("unknown prompt flight stage {other:?}")),
        }
    }
}

/// The exact context budget a flight declares. It is the same bounded request
/// the public `context-assemble` operation accepts, minus the scope and the
/// query, which the flight supplies from its own coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptFlightContextRequest {
    pub valid_at: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seeds: Vec<DataReference>,
    pub max_graph_depth: u8,
    pub max_items: u64,
    pub max_output_bytes: u64,
    pub max_storage_keys: u64,
}

impl PromptFlightContextRequest {
    /// The context request the engine executes for this flight. The prompt is
    /// the query; validation of every bound is owned by `AssembleContext`.
    pub fn assemble_context(&self, scope: &str, prompt: &str) -> AssembleContext {
        AssembleContext {
            scope: scope.to_owned(),
            query: prompt.trim().to_owned(),
            valid_at: self.valid_at,
            seeds: self.seeds.clone(),
            max_graph_depth: self.max_graph_depth,
            max_items: self.max_items,
            max_output_bytes: self.max_output_bytes,
            max_storage_keys: self.max_storage_keys,
        }
    }
}

/// One governed request to record a prompt flight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LaunchPromptFlight {
    pub scope: String,
    pub prompt: String,
    pub provider: PromptFlightProvider,
    pub context_mode: PromptFlightContextMode,
    pub context: PromptFlightContextRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_marker: Option<String>,
}

impl LaunchPromptFlight {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        validate_prompt(&self.prompt)?;
        validate_acceptance_marker(self.acceptance_marker.as_deref())?;
        self.context
            .assemble_context(&self.scope, &self.prompt)
            .validate()
    }

    /// Cohort identity shared by every flight of the same trimmed prompt.
    pub fn cohort_sha256(&self) -> String {
        prompt_cohort_sha256(&self.prompt)
    }
}

/// Digest that groups flights of one prompt across arms, providers, and time.
pub fn prompt_cohort_sha256(prompt: &str) -> String {
    sha256_bytes(prompt.trim().as_bytes())
}

/// One externally observable, append-only flight event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptFlightEvent {
    pub ordinal: u64,
    pub at_unix_ms: u64,
    pub elapsed_ms: u64,
    pub stage: PromptFlightStage,
    pub kind: CanonicalId,
    pub label: String,
    pub detail: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<String, QueryValue>,
}

impl PromptFlightEvent {
    pub fn validate(&self) -> Result<()> {
        if self.at_unix_ms == 0 {
            return invalid("prompt flight event at_unix_ms must be greater than zero");
        }
        if self.kind.as_str().len() > MAX_PROMPT_FLIGHT_EVENT_KIND_BYTES {
            return invalid("prompt flight event kind exceeds its byte bound");
        }
        if self.label.trim().is_empty()
            || self.label.len() > MAX_PROMPT_FLIGHT_EVENT_LABEL_BYTES
            || self.label.as_bytes().contains(&0)
        {
            return invalid("prompt flight event label is empty, oversized, or contains NUL");
        }
        if self.detail.len() > MAX_PROMPT_FLIGHT_EVENT_DETAIL_BYTES
            || self.detail.as_bytes().contains(&0)
        {
            return invalid("prompt flight event detail exceeds its byte bound or contains NUL");
        }
        for key in self.data.keys() {
            if key.is_empty() || key.len() > MAX_ID_BYTES || key.as_bytes().contains(&0) {
                return invalid(
                    "prompt flight event data key is empty, oversized, or contains NUL",
                );
            }
        }
        let encoded = serde_json::to_vec(&self.data)
            .map_err(|error| crate::ContractError(error.to_string()))?;
        if encoded.len() > MAX_PROMPT_FLIGHT_EVENT_DATA_BYTES {
            return invalid("prompt flight event data exceeds its byte bound");
        }
        Ok(())
    }
}

/// Digest coordinates of the engine-assembled context packet a flight used.
/// The packet itself is reproducible from the request at the recorded stamp;
/// the flight retains its identity, not a second copy of the items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptFlightContextSummary {
    pub request_sha256: String,
    pub plan_sha256: String,
    pub packet_sha256: String,
    pub items: u64,
    pub output_bytes: u64,
    pub truncated: bool,
    pub selected_versions: u64,
}

impl PromptFlightContextSummary {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.request_sha256, "context request_sha256")?;
        validate_sha256(&self.plan_sha256, "context plan_sha256")?;
        validate_sha256(&self.packet_sha256, "context packet_sha256")
    }
}

/// Observed, non-authoritative measurements of one flight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptFlightMetrics {
    pub context_items: u64,
    pub context_output_bytes: u64,
    pub elapsed_ms: u64,
}

/// The durable flight row. Every field is a typed property of the engine-owned
/// `rrflow-prompt-flight` record; nothing is a serialized blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptFlightRecord {
    pub format_version: u16,
    pub flight_id: CanonicalId,
    pub scope: String,
    pub cohort_sha256: String,
    pub prompt: String,
    pub provider: PromptFlightProvider,
    pub context_mode: PromptFlightContextMode,
    pub context_request: PromptFlightContextRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_marker: Option<String>,
    pub status: PromptFlightStatus,
    pub created_at_unix_ms: u64,
    /// The caller's replay coordinate. The same key with the same operation
    /// digest returns this flight; a different digest is a conflict.
    pub idempotency_key: CorrelationId,
    pub operation_sha256: String,
    /// The authenticated runtime coordinate the flight was prepared at.
    pub read: ContextReadStamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<PromptFlightContextSummary>,
    pub metrics: PromptFlightMetrics,
    /// Cursor of the flight's first committed version; stable creation order.
    pub flight_cursor: u64,
    /// Cursor of the version this projection was read from.
    pub known_at_cursor: u64,
    /// Number of `rrflow-prompt-flight-used` edges written with the flight:
    /// one per context item that is a canonical record. Claims are not graph
    /// nodes and are counted only in the context summary.
    pub use_count: u64,
    pub event_count: u64,
}

impl PromptFlightRecord {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != PROMPT_FLIGHT_FORMAT_VERSION {
            return invalid(format!(
                "prompt flight format version must be {PROMPT_FLIGHT_FORMAT_VERSION}"
            ));
        }
        validate_scope(&self.scope)?;
        validate_prompt(&self.prompt)?;
        validate_sha256(&self.cohort_sha256, "cohort_sha256")?;
        if self.cohort_sha256 != prompt_cohort_sha256(&self.prompt) {
            return invalid("prompt flight cohort digest does not match its prompt");
        }
        validate_acceptance_marker(self.acceptance_marker.as_deref())?;
        self.context_request
            .assemble_context(&self.scope, &self.prompt)
            .validate()?;
        if self.created_at_unix_ms == 0 {
            return invalid("prompt flight created_at_unix_ms must be greater than zero");
        }
        validate_sha256(&self.operation_sha256, "operation_sha256")?;
        self.read.validate()?;
        match (self.context_mode, &self.context) {
            (PromptFlightContextMode::Fresh, None) => {
                if self.metrics.context_items != 0
                    || self.metrics.context_output_bytes != 0
                    || self.use_count != 0
                {
                    return invalid("fresh prompt flight cannot report injected context");
                }
            }
            (PromptFlightContextMode::Pruned, Some(context)) => {
                context.validate()?;
                if self.metrics.context_items != context.items
                    || self.metrics.context_output_bytes != context.output_bytes
                {
                    return invalid("prompt flight metrics disagree with its context summary");
                }
                if self.use_count > context.items {
                    return invalid("prompt flight uses more records than its context returned");
                }
            }
            _ => {
                return invalid(
                    "prompt flight context summary is required exactly when context was injected",
                )
            }
        }
        // The flight commits after the stamp it was prepared at, and every
        // later version of the row can only move its known-at cursor forward.
        if self.flight_cursor <= self.read.runtime_cursor
            || self.known_at_cursor < self.flight_cursor
        {
            return invalid("prompt flight cursor coordinates are invalid");
        }
        if self.event_count == 0 || self.event_count > MAX_PROMPT_FLIGHT_EVENTS as u64 {
            return invalid(format!(
                "prompt flight event count must be in 1..={MAX_PROMPT_FLIGHT_EVENTS}"
            ));
        }
        Ok(())
    }
}

/// One edge from a flight to a canonical record its context used, in the
/// order the engine ranked the context items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptFlightUse {
    pub reference: DataReference,
    /// One-based position of the item in the flight's context packet.
    pub rank: u64,
    /// The fused context score the item carried in the packet.
    pub score: f64,
    /// The context avenues that produced the item, sorted and unique.
    pub evidence: Vec<ContextEvidenceKind>,
}

impl PromptFlightUse {
    pub fn validate(&self) -> Result<()> {
        if self.rank == 0 || self.rank > MAX_CONTEXT_ITEMS {
            return invalid(format!(
                "prompt flight use rank must be in 1..={MAX_CONTEXT_ITEMS}"
            ));
        }
        if !self.score.is_finite() || self.score <= 0.0 {
            return invalid("prompt flight use score must be finite and positive");
        }
        if self.evidence.is_empty() || self.evidence.windows(2).any(|pair| pair[0] >= pair[1]) {
            return invalid("prompt flight use evidence must be non-empty, sorted, and unique");
        }
        Ok(())
    }
}

/// One complete flight: its record, the records its context used, and every
/// event in ordinal order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptFlight {
    pub record: PromptFlightRecord,
    pub uses: Vec<PromptFlightUse>,
    pub events: Vec<PromptFlightEvent>,
}

impl PromptFlight {
    pub fn validate(&self) -> Result<()> {
        self.record.validate()?;
        if self.uses.len() as u64 != self.record.use_count {
            return invalid("prompt flight uses disagree with the recorded use count");
        }
        let mut referenced = BTreeSet::new();
        let mut previous_rank = 0;
        for used in &self.uses {
            used.validate()?;
            if used.rank <= previous_rank || !referenced.insert(&used.reference) {
                return invalid(
                    "prompt flight uses must be in rank order and name each record once",
                );
            }
            previous_rank = used.rank;
        }
        if self.events.len() as u64 != self.record.event_count {
            return invalid("prompt flight events disagree with the recorded event count");
        }
        let mut previous: Option<&PromptFlightEvent> = None;
        for (ordinal, event) in self.events.iter().enumerate() {
            event.validate()?;
            if event.ordinal != ordinal as u64 {
                return invalid("prompt flight event ordinals must be contiguous from zero");
            }
            if event.at_unix_ms < self.record.created_at_unix_ms {
                return invalid("prompt flight event precedes its flight");
            }
            if let Some(previous) = previous {
                if event.elapsed_ms < previous.elapsed_ms || event.at_unix_ms < previous.at_unix_ms
                {
                    return invalid("prompt flight events must not move backwards in time");
                }
            }
            previous = Some(event);
        }
        if self
            .events
            .last()
            .is_some_and(|event| event.elapsed_ms > self.record.metrics.elapsed_ms)
        {
            return invalid("prompt flight events exceed the recorded elapsed time");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LaunchPromptFlightResult {
    pub flight: PromptFlight,
    pub idempotent_replay: bool,
}

impl LaunchPromptFlightResult {
    pub fn validate(&self) -> Result<()> {
        self.flight.validate()
    }
}

/// Bounded, cursor-ordered page of flight records at one read stamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListPromptFlights {
    pub scope: String,
    /// Return flights whose first committed version is after this cursor.
    pub after_cursor: u64,
    pub limit: u16,
    pub max_storage_keys: u64,
}

impl ListPromptFlights {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        if self.limit == 0 || self.limit > MAX_PROMPT_FLIGHT_PAGE {
            return invalid(format!(
                "prompt flight page limit must be in 1..={MAX_PROMPT_FLIGHT_PAGE}"
            ));
        }
        validate_storage_keys(self.max_storage_keys)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptFlightPage {
    pub scope: String,
    pub read: ContextReadStamp,
    pub requested_after_cursor: u64,
    pub flights: Vec<PromptFlightRecord>,
    /// True when more flights follow the last returned `flight_cursor`.
    pub truncated: bool,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
}

impl PromptFlightPage {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        self.read.validate()?;
        self.read_evidence.validate()?;
        if self.flights.len() > usize::from(MAX_PROMPT_FLIGHT_PAGE) {
            return invalid("prompt flight page exceeds its limit");
        }
        let mut previous = self.requested_after_cursor;
        for flight in &self.flights {
            flight.validate()?;
            if flight.scope != self.scope {
                return invalid("prompt flight page mixes scopes");
            }
            if flight.flight_cursor <= previous || flight.known_at_cursor > self.read.runtime_cursor
            {
                return invalid("prompt flight page is not in ascending flight-cursor order");
            }
            previous = flight.flight_cursor;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadPromptFlight {
    pub scope: String,
    pub flight_id: CanonicalId,
    pub max_storage_keys: u64,
}

impl ReadPromptFlight {
    pub fn validate(&self) -> Result<()> {
        validate_scope(&self.scope)?;
        validate_storage_keys(self.max_storage_keys)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptFlightReadResult {
    pub flight: PromptFlight,
    pub read: ContextReadStamp,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
}

impl PromptFlightReadResult {
    pub fn validate(&self) -> Result<()> {
        self.flight.validate()?;
        self.read.validate()?;
        self.read_evidence.validate()?;
        if self.flight.record.known_at_cursor > self.read.runtime_cursor {
            return invalid("prompt flight was read beyond its read stamp");
        }
        Ok(())
    }
}

fn validate_scope(scope: &str) -> Result<()> {
    if scope.is_empty() || scope.len() > MAX_ID_BYTES || scope.as_bytes().contains(&0) {
        return invalid("prompt flight scope is invalid");
    }
    Ok(())
}

fn validate_prompt(prompt: &str) -> Result<()> {
    if prompt.trim().is_empty() {
        return invalid("prompt flight prompt must not be empty");
    }
    if prompt.len() > MAX_PROMPT_FLIGHT_PROMPT_BYTES || prompt.as_bytes().contains(&0) {
        return invalid("prompt flight prompt exceeds its byte bound or contains NUL");
    }
    Ok(())
}

fn validate_acceptance_marker(marker: Option<&str>) -> Result<()> {
    if let Some(marker) = marker {
        if marker.trim().is_empty()
            || marker.len() > MAX_PROMPT_FLIGHT_ACCEPTANCE_MARKER_BYTES
            || marker.as_bytes().contains(&0)
        {
            return invalid("prompt flight acceptance marker is empty, oversized, or contains NUL");
        }
    }
    Ok(())
}

fn validate_storage_keys(max_storage_keys: u64) -> Result<()> {
    if max_storage_keys == 0 || max_storage_keys > MAX_PROMPT_FLIGHT_STORAGE_KEYS {
        return invalid(format!(
            "prompt flight max_storage_keys must be in 1..={MAX_PROMPT_FLIGHT_STORAGE_KEYS}"
        ));
    }
    Ok(())
}
