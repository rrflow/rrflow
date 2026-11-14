//! Prompt flights: governed, durable records of one prompt's passage through
//! the runtime boundary.
//!
//! A flight is one `rrflow-prompt-flight` record, one
//! `rrflow-prompt-flight-used` edge to each canonical record its context
//! used, append-only `rrflow-prompt-flight-event` events in the canonical
//! runtime log, and, when the flight joins a named task, one
//! `rrflow-task-includes` edge from that task, so every prompt is a graph
//! neighbourhood rrflowQL traverses.
//! The engine prepares the flight at one authenticated read stamp, assembles
//! any injected context through the same `context-assemble` path every client
//! uses, and commits record, edges, and events in one cursor-CAS runtime
//! commit. A flight read names its record, edges, and events by identity, so
//! it costs the flight's own size; no client-side flight lifecycle exists.
//!
//! A flight records only externally observable runtime events. It never
//! requests, infers, or persists hidden chain-of-thought.

use super::task::{register_task_includes_schema, task_includes_reference};
use super::*;
use rrd_contract::{
    context_request_sha256, prompt_cohort_sha256, ContextEvidenceKind, ContextPacket,
    ContextReadStamp, LaunchPromptFlight, LaunchPromptFlightResult, ListPromptFlights,
    PromptFlight, PromptFlightContextMode, PromptFlightContextRequest, PromptFlightContextSummary,
    PromptFlightEvent, PromptFlightMetrics, PromptFlightPage, PromptFlightProvider,
    PromptFlightReadResult, PromptFlightRecord, PromptFlightStage, PromptFlightStatus,
    PromptFlightUse, ReadAccessPath, ReadEvidence, ReadPathEvidence, ReadPromptFlight,
    ReadStampValidationEvidence, ReadStampValidationMethod, PROMPT_FLIGHT_EVENT_KIND,
    PROMPT_FLIGHT_FORMAT_VERSION, PROMPT_FLIGHT_KIND, PROMPT_FLIGHT_USE_KIND,
};
use rrd_store::{RuntimeReadBudget, RuntimeVersionedSource};
use std::collections::BTreeSet;

mod launch;
mod records;
mod schema;
mod storage;

pub(super) use records::*;
use schema::*;
use storage::*;

#[cfg(test)]
mod context_reference_tests {
    use super::context_record_reference;

    #[test]
    fn flight_context_uses_the_shared_canonical_reference_decoder() {
        let (target, public) = context_record_reference("note:alpha").unwrap();
        assert_eq!(target.kind.as_str(), "note");
        assert_eq!(target.id.as_str(), "alpha");
        assert_eq!(public.kind.as_str(), "note");
        assert_eq!(public.id.as_str(), "alpha");

        // These decode to what no reference can hold, so the decoder refuses them too.
        for invalid in [
            "note:alpha:other",
            "note:%3a",
            "note:%41",
            "note:%",
            "a%3Ab:c",
            "a:b%3Ac",
            "note:percent%25",
        ] {
            assert!(context_record_reference(invalid).is_err(), "{invalid}");
        }
        // An event's identity decodes, and the public flight contract cannot carry it.
        let error = context_record_reference("note:cursor%3A7").unwrap_err();
        assert!(error.to_string().contains("not representable"), "{error}");
    }
}
