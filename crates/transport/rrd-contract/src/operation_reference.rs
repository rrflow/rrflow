//! Reference text for the public operation catalogue.
//!
//! The endpoint catalogue owns each operation's family, one-line summary, and
//! description; OpenAPI, the SDK projections, and client references project
//! them. The text describes what the engine implements today and nothing it
//! does not. Families group operations for readers and carry no
//! authorization meaning.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The reference group an operation belongs to.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum OperationFamily {
    Inspection,
    IdentityAndControl,
    Transactions,
    QueryAndContext,
    Knowledge,
    PromptFlights,
    Tasks,
    Recall,
    Delivery,
    Recovery,
}

impl OperationFamily {
    /// Every family, in reference order.
    pub const ALL: [Self; 10] = [
        Self::Inspection,
        Self::IdentityAndControl,
        Self::Transactions,
        Self::QueryAndContext,
        Self::Knowledge,
        Self::PromptFlights,
        Self::Tasks,
        Self::Recall,
        Self::Delivery,
        Self::Recovery,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inspection => "inspection",
            Self::IdentityAndControl => "identity-and-control",
            Self::Transactions => "transactions",
            Self::QueryAndContext => "query-and-context",
            Self::Knowledge => "knowledge",
            Self::PromptFlights => "prompt-flights",
            Self::Tasks => "tasks",
            Self::Recall => "recall",
            Self::Delivery => "delivery",
            Self::Recovery => "recovery",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Inspection => "Inspection",
            Self::IdentityAndControl => "Identity and control",
            Self::Transactions => "Transactions",
            Self::QueryAndContext => "Query and context",
            Self::Knowledge => "Knowledge",
            Self::PromptFlights => "Prompt flights",
            Self::Tasks => "Tasks",
            Self::Recall => "Recall",
            Self::Delivery => "Delivery",
            Self::Recovery => "Recovery",
        }
    }

    pub const fn description(self) -> &'static str {
        FAMILY_DESCRIPTIONS[self as usize]
    }
}

/// One family's reader-facing title and description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationFamilyDescriptor {
    pub family: OperationFamily,
    pub title: String,
    pub description: String,
}

impl OperationFamilyDescriptor {
    pub fn of(family: OperationFamily) -> Self {
        Self {
            family,
            title: family.title().into(),
            description: family.description().into(),
        }
    }
}

/// Every family descriptor, in reference order.
pub fn operation_families() -> Vec<OperationFamilyDescriptor> {
    OperationFamily::ALL
        .into_iter()
        .map(OperationFamilyDescriptor::of)
        .collect()
}

/// The reference text of one catalogue operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OperationReference {
    pub family: OperationFamily,
    pub summary: &'static str,
    pub description: &'static str,
}

pub(crate) const SUMMARY_MAX_BYTES: usize = 80;
pub(crate) const DESCRIPTION_MAX_BYTES: usize = 600;

/// A summary is one plain line: ASCII, trimmed, at most 80 bytes, and without
/// a closing period.
pub(crate) fn valid_summary(summary: &str) -> bool {
    valid_line(summary, SUMMARY_MAX_BYTES) && !summary.ends_with('.')
}

/// A description is plain ASCII prose of whole sentences on one line, at most
/// 600 bytes.
pub(crate) fn valid_description(description: &str) -> bool {
    valid_line(description, DESCRIPTION_MAX_BYTES) && description.ends_with('.')
}

fn valid_line(text: &str, max_bytes: usize) -> bool {
    !text.is_empty()
        && text.len() <= max_bytes
        && text.trim() == text
        && text
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        && !text.contains("  ")
}

/// Looks up one operation's reference text.
pub(crate) fn operation_reference(operation: &str) -> Option<OperationReference> {
    OPERATION_REFERENCES
        .binary_search_by(|(id, _)| (*id).cmp(operation))
        .ok()
        .map(|index| OPERATION_REFERENCES[index].1)
}

/// The operation identifiers that have reference text, in sorted order.
pub fn referenced_operations() -> impl Iterator<Item = &'static str> {
    OPERATION_REFERENCES.iter().map(|(id, _)| *id)
}

const FAMILY_DESCRIPTIONS: [&str; 10] = [
    // Inspection
    "Liveness, readiness, capabilities, the operation catalogue and OpenAPI document, diagnostics and storage inspection, and the data schema. The first five need no credentials.",
    // Identity and control
    "Sessions and their bearer tokens, the engram snapshot, the hash-chained audit journal, and the previewed and applied endpoint configuration.",
    // Transactions
    "Typed mutation work in a session: begin at a read cursor, preview without publishing, commit once with a receipt, or abort.",
    // Query and context
    "Bounded rrflowQL reads, Parquet export, live-query polling, query indexes, and ranked context assembly from one snapshot.",
    // Knowledge
    "The project's knowledge-tree catalogue: trees, paths and their native or external bindings, replaced by compare-and-swap.",
    // Prompt flights
    "Recorded prompt flights at one read stamp. Only the observe provider exists, so a flight is prepared and no provider runs.",
    // Tasks
    "Named tasks that prompt flights join, their durable reasoning budgets, and budgeted embedding attempts.",
    // Recall
    "Vector collections and their points, nearest-vector search, and the built-in feature-hash embedding backend.",
    // Delivery
    "Committed changes by cursor: changefeed pages and waits, durable subscriptions, and their delivery over the WebSocket.",
    // Recovery
    "Logical backups of the instance, the backup catalogue, and restores into a new storage root that does not replace the active one.",
];

const fn reference(
    family: OperationFamily,
    summary: &'static str,
    description: &'static str,
) -> OperationReference {
    OperationReference {
        family,
        summary,
        description,
    }
}

use OperationFamily as F;

/// Sorted by operation identifier, the order of the endpoint catalogue.
const OPERATION_REFERENCES: &[(&str, OperationReference)] = &[
    ("attunement-cancel", reference(F::IdentityAndControl,
        "Request cancellation of an installed attunement job",
        "Requires EngramAdmin, the exact job revision and plan digest, and an idempotency key. Returns the durable cancel_requested transition or its original receipt on replay. Cancellation is not proof that a worker or external resource has terminated.")),
    ("attunement-job-list", reference(F::IdentityAndControl,
        "Discover installed attunement jobs",
        "Requires EngramRead. Follows the installed initial job and committed successor links within the requested page bound. Returns stored identities and state at an unchanged control sequence; each later page is a new read. Does not inspect current source, start work, or accept a plan.")),
    ("attunement-pause", reference(F::IdentityAndControl,
        "Pause an attunement", "Requires EngramAdmin, the current revision and worker lease generation, and idempotency. Records a paused job without asserting external resource termination.")),
    ("attunement-read", reference(F::IdentityAndControl,
        "Read an installed attunement job and its plan",
        "Returns the engine's stored attunement plan and status for a known job, authorized by EngramRead. This read does not inspect the current project source and reports source verification as not checked. It does not start, resume, or replan the job.")),
    ("attunement-receipt-read", reference(F::IdentityAndControl,
        "Read an installed attunement control receipt",
        "Reads the original committed outcome for the authenticated principal, job, and idempotency key under EngramRead. Returns the stored plan and status or an absent receipt without replaying a mutation. This read does not verify the current project source.")),
    ("attunement-recover-expired", reference(F::IdentityAndControl,
        "Recover an expired attunement lease", "Requires EngramAdmin, the exact revision, an expired lease and idempotency. Records a paused job; the caller must explicitly resume.")),
    ("attunement-replan-accept", reference(F::IdentityAndControl,
        "Accept an observed source replan", "Requires EngramAdmin, exact proposal identity and idempotency. Fences the predecessor and records a successor and receipt atomically. Source observation is not an atomic filesystem snapshot.")),
    ("attunement-replan-preview", reference(F::IdentityAndControl,
        "Preview an attunement replan", "Requires EngramAdmin and the exact predecessor revision. Observes source without accepting a plan; it is not an atomic filesystem snapshot.")),
    ("attunement-replan-receipt-read", reference(F::IdentityAndControl,
        "Read an attunement replan receipt", "Requires EngramRead and reads the authenticated principal's receipt without replaying acceptance or verifying current source.")),
    ("attunement-resume", reference(F::IdentityAndControl,
        "Resume an attunement", "Requires EngramAdmin, the exact plan, checkpoint, revision and next phase, plus idempotency. The engine checks observed source before claiming a new lease.")),
    ("attunement-start", reference(F::IdentityAndControl,
        "Start an attunement", "Requires EngramAdmin, an exact plan and revision, and idempotency. The engine checks observed source before the transition.")),
    ("attunement-step", reference(F::IdentityAndControl,
        "Advance an attunement", "Requires EngramAdmin, exact revision and lease generation, and idempotency. The engine checks observed source before advancing.")),
    ("audit-export", reference(F::IdentityAndControl,
        "Export a range of the audit journal as JSON Lines",
        "Exports up to limit (1..=1024) audit records after after_sequence as canonical JSON Lines (application/x-ndjson; profile=rrd-audit-v1) with record_count, content_sha256, through_sequence, and chain anchor and head digests. Range semantics match audit-read, and the chain and content digest are validated before return.")),
    ("audit-read", reference(F::IdentityAndControl,
        "Read one page of the audit journal",
        "Returns up to limit (1..=1024) hash-chained audit records after after_sequence, with through_sequence to resume from and the chain anchor and head digests. The engine verifies chain continuity and the durable audit head before returning and fails the read if either check fails.")),
    ("backup-create", reference(F::Recovery,
        "Create a logical backup of this instance",
        "Captures a content-addressed backup into the instance backup catalogue and returns its digest, per-family coverage, and catalogue revision. label is 1 to 96 ASCII letters, digits, dots, hyphens, or underscores (catalogued with a digest suffix). An idempotency key is required and scoped to the session; a replay returns the stored result with idempotent_replay true, and rrflowMX is refused.")),
    ("backup-list", reference(F::Recovery,
        "List the backups in the instance catalogue",
        "Returns the backup catalogue: revision, catalogue digest, and every backup with archive inventory and coverage. With verify_archives true, every archive inventory, manifest, control archive, and object payload digest is checked and archives_verified is true; otherwise only the catalogue digest is checked. Refused on non-durable rrflowMX.")),
    ("capabilities-read", reference(F::Inspection,
        "Read the service capabilities of this instance",
        "Returns the protocol and implementation versions, deployment profile, installed engram identity when present, active engram configuration, instance id, capability descriptors with status, limits, and limitations, and the product capability catalogue. Descriptors are computed when the server binds and reflect whether security, mTLS, and JWT are configured. No credentials are required.")),
    ("changefeed-follow", reference(F::Delivery,
        "Wait for committed changes after a cursor",
        "Reads the changefeed as changefeed-read does; when the page holds no change it polls for up to wait_timeout_ms (1 to 5000), then returns the page with timed_out and waited_ms. A wait that would end after the request deadline_unix_ms is refused.")),
    ("changefeed-read", reference(F::Delivery,
        "Read a page of committed changes after a cursor",
        "Returns committed runtime changes after after_cursor on scope instance:{id}, scanning at most limit (1 to 4096) cursors and verifying the change hash chain. The page reports through_cursor, head_cursor and has_more; through_cursor is the after_cursor for the next page. It returns at once and does not wait for new changes.")),
    ("cold-history-read", reference(F::Delivery,
        "Read cold-history status or a published generation",
        "Reports the configured cold-history status or reads one exact, digest-verified Parquet generation under the changefeed permission and query budgets. The result is derived history; scoped completeness, audit archival and source reclamation are not established by this operation.")),
    ("context-assemble", reference(F::QueryAndContext,
        "Assemble a ranked context packet from one snapshot",
        "Reads one data snapshot of scope instance:{id} at valid_at and ranks records and claims by reciprocal-rank fusion of seed anchors, BM25 text matches, exact vector scores (with an installed local text embedder) and bidirectional graph expansion. It needs a query or a seed; max_items is at most 512, max_output_bytes 768 KiB and max_graph_depth 32. Sessions under a data policy are refused.")),
    ("data-schema-read", reference(F::Inspection,
        "Read the data schema at one read stamp",
        "Returns the schema registry at the current read stamp, or null when no schema is committed, with known_at_cursor, the read manifest digest, and read evidence. Reads only schema versions within max_storage_keys (1..=1000000) and never scans application data. scope must equal instance:{id}; principals whose grant carries a data policy are refused.")),
    ("diagnostics-read", reference(F::Inspection,
        "Read a consistent cross-model diagnostic snapshot",
        "Assembles schema, models, graph and graph difference, retention, vector artifacts, indexes, collections, engram, changefeed, audit, and readiness under one verified read stamp, retrying up to 3 times before returning conflict. scope must equal instance:{id}; change_limit is 1..=4096 and audit_limit 1..=1024. Refused when the runtime head exceeds runtime_max_scanned_changes (max 1000000).")),
    ("diagnostics-storage-cursor", reference(F::Inspection,
        "Drive a bounded physical storage inspection cursor",
        "Applies one action (open, observe, advance, cancel_source, release) to a session-owned cursor over an rrflowKV snapshot; responses report page ordinal, rows, allocated bytes, and page SHA-256, never row content. open caps batch_rows at 4096, batch_bytes at 4 MiB, and lifetime_ms at 60000 and session expiry. Reopening with identical parameters or advancing from the previous ordinal is a replay.")),
    ("diagnostics-storage-read", reference(F::Inspection,
        "Read physical storage counters for this process",
        "Returns process-local physical storage counters for the instance storage profile: rrflowKV reports memtable, WAL, maintenance, segment, page-cache, segment I/O, and admission counters; rrflowMX reports logical_only with none. Counters are observations, not bound to a read stamp, and reset when the process reopens its store. scope must equal instance:{id}.")),
    ("embedding-builtin-activate", reference(F::Recall,
        "Activate the built-in feature-hash embedding backend",
        "Installs the deterministic feature-hash backend rrflow:feature-hash:cpu:v1 (CPU, local and offline, text or JSON input) in the process-local registry with dimensions 8 to 1024 and a seed, then returns the catalogue of installed backends. Repeating the same activation changes nothing; other dimensions or seed while it is installed conflict. It is not a semantic model.")),
    ("endpoint-catalogue", reference(F::Inspection,
        "Read the HTTP and WebSocket operation catalogue",
        "Returns the static operation catalogue: one descriptor per HTTP operation (id, method, path template, authentication, mutation flag, security action, request and response type names, family, summary and description), sorted by operation id, plus the WebSocket endpoint and the operation families. The server resolves every request against this same catalogue. No credentials are required.")),
    ("endpoint-configuration-apply", reference(F::IdentityAndControl,
        "Apply a previewed endpoint configuration plan",
        "Publishes the plan as the next revision. approved_plan_sha256 must equal the plan digest, the plan must be unexpired (else deadline_exceeded), and the current revision and configuration digest must still match it (else conflict). An idempotency key is required; resubmitting the plan that is still the last applied returns the current configuration.")),
    ("endpoint-configuration-preview", reference(F::IdentityAndControl,
        "Preview an endpoint configuration change as a plan",
        "Returns a plan for the next revision without changing state: the profile with engine-assigned revision and digest, or null to preview revocation, bound to the current configuration digest and valid for 300000 ms. expected_revision must equal the current revision, otherwise the call returns conflict. No idempotency key is needed and no provider is contacted.")),
    ("endpoint-configuration-read", reference(F::IdentityAndControl,
        "Read the active endpoint configuration",
        "Returns the project endpoint configuration: installed engram identity, revision, current endpoint profile, and the digest of the last applied plan. Revision 0 with no profile means never configured; revocation keeps a nonzero revision with no profile. Requires an installed project identity.")),
    ("engram-read", reference(F::IdentityAndControl,
        "Read the snapshot of one engram",
        "Returns the engram snapshot named by the {engram} path segment: revision, activity policy, authority resources and receipts, instances with desired, observed, and activity state, and operations. The request resource must carry the same engram id. Returns not_found when no engram authority exists.")),
    ("flight-launch", reference(F::PromptFlights,
        "Record a prompt flight without running a provider",
        "Commits a prompt flight at one read stamp: record, stage events, a use edge per canonical record in pruned context, and a task edge when task_id names an open task. Only the observe provider exists, so the flight ends prepared and no provider runs. The same session and idempotency key replay the stored flight; a changed payload conflicts. Context bounds above engram recall limits are refused.")),
    ("flight-list", reference(F::PromptFlights,
        "List prompt flight records in creation order",
        "Returns up to limit (1 to 256) flight records whose first commit is after after_cursor, ascending by flight_cursor, at one read stamp. Records carry event and use counts but not the events or edges. truncated reports more flights; pass the last flight_cursor as after_cursor. max_storage_keys bounds the read, and an exceeded budget is refused.")),
    ("flight-read", reference(F::PromptFlights,
        "Read one prompt flight with its events and uses",
        "Returns one flight at one read stamp: its record, the use edges to the canonical records its context used in rank order, and every event in ordinal order. Edges and events are read by identity, so the cost is the flight's own size within max_storage_keys. An unknown flight_id returns not found.")),
    ("health-live", reference(F::Inspection,
        "Check that the server process is serving requests",
        "Returns only the server observation time and does not inspect storage state. Each call is still appended to the audit journal as service_inspect, and the call fails if that append fails. No credentials are required.")),
    ("health-ready", reference(F::Inspection,
        "Check that storage is readable and report its position",
        "Reads the current claim sequence, runtime cursor, and storage backend name and returns them with the observation time. Returns an error when storage cannot be read. No credentials are required.")),
    ("knowledge-tree-list", reference(F::Knowledge,
        "Read a project's knowledge-tree catalogue",
        "Returns the knowledge-tree catalogue for project_id and scope, with its revision, trees, paths and bindings; a catalogue never written is returned empty at revision 0. project_id must match the installed engram's project and scope must be instance:{id}. No vector-store provider is contacted.")),
    ("knowledge-tree-replace", reference(F::Knowledge,
        "Replace a knowledge-tree catalogue by compare-and-swap",
        "Replaces the whole catalogue for project_id and scope when expected_revision equals the current revision, assigning the next revision; omitted entries are removed and a path id cannot move between trees. Native bindings must name an existing vector collection and vector; external bindings are stored as configuration only. The same session, idempotency key and request replays the stored result.")),
    ("observation-deliver", reference(F::Delivery,
        "Deliver one prepared engine observation",
        "Checks current owner, source-read permission, configuration and payload digests before one fenced HTTP attempt. Only the configured devspace and exact event acknowledgement count as delivered. Lost acknowledgements remain uncertain; authorized retries reuse the same payload and event ID. An acknowledged record never dispatches again. Delivery grants neither task success nor training approval.")),
    ("observation-prepare", reference(F::Delivery,
        "Prepare an authorized engine observation",
        "Reads a terminal task attempt or installed job at the requested identity. Freezes its public response locally and prepares metadata-only export under an approved endpoint profile, source scope and finite limits. Requires initialized authority and current source-read permission. A repeated delivery ID replays its retained record; conflicting reuse fails. Preparation does not execute or complete its source task.")),
    ("observation-read", reference(F::Delivery,
        "Read an engine observation delivery receipt",
        "Returns only the current principal's retained delivery and exact locally captured source response, with current source-read authorization. Source content is never exported to the sink. An expired in-flight fence is projected as uncertain without repeating the source task or contacting the sink. Absent or explicitly retired records return null. A transport receipt is historical evidence, not current project-source verification.")),
    ("openapi-read", reference(F::Inspection,
        "Read the OpenAPI 3.1 document for the protocol",
        "Returns a deterministic OpenAPI 3.1.0 document generated from the endpoint catalogue and the Rust wire types: each operation's summary, description, family tag, and request and response envelope schemas, the family tags, the rrdApiKey and rrdBearer security schemes, and x-rrd extensions including the signal catalogue and its SHA-256. The servers entry is fixed at http://127.0.0.1:9477. No credentials are required.")),
    ("query-execute", reference(F::QueryAndContext,
        "Run a bounded rrflowQL query at one read stamp",
        "Parses and runs one rrflowQL query (at most 64 KiB and 128 null, boolean, integer, unsigned or string parameters) on scope instance:{id} at one captured read stamp, returning rows with the canonical query, plan and read evidence. The budget caps storage keys, rows, output, batch rows, memory, spill and elapsed time within engram limits. A principal data policy adds row filters and limits fields.")),
    ("query-export", reference(F::QueryAndContext,
        "Export a query result as a Parquet artifact",
        "Runs the embedded query as query-execute does and encodes the result as one rrflow-parquet-1 artifact, returned base64-encoded with its SHA-256 and the source query evidence. The result must be untruncated with at most 1024 rows and 64 fields; max_output_bytes must be 1 to 524288 and within the query budget, and max_elapsed_ms bounds the whole export.")),
    ("query-index-ensure", reference(F::QueryAndContext,
        "Create or rebuild a query index from a definition query",
        "Installs index_id from an rrflowQL definition that reads KNOWN HEAD at a literal valid time without join, limit or EXPLAIN, and builds it to ready within the budget; kind is scalar, count, geo, materialized_view, aggregate_count or bm25. A changed definition for an existing id, or a quarantined or retiring index, is refused. The same idempotency key and request replays the recorded index.")),
    ("query-index-list", reference(F::QueryAndContext,
        "List the query indexes in a scope",
        "Returns the query index catalogue for scope instance:{id}: its revision and, per index, the canonical definition, kind, generation, source cursor, configuration and artifact digests, and state (building, ready, quarantined or retiring). It reads catalogue state and builds nothing.")),
    ("query-live-poll", reference(F::QueryAndContext,
        "Poll a live query for row changes after a cursor",
        "Evaluates a KNOWN HEAD rrflowQL query at after_cursor and at the current head and returns the added, updated and removed rows; a delta over max_delta_rows (1 to 100000) is refused. wait_timeout_ms (0 to 5000) waits for the head to pass after_cursor and sets timed_out if it does not. A wait past the request deadline and a session under a data policy are refused.")),
    ("restore-create", reference(F::Recovery,
        "Restore a catalogued backup into a new root",
        "Restores the backup named by backup_sha256 into a new storage root named restore_id and returns its archive inventory; the active instance is not switched to it. If that root already holds a restore with matching watermarks, it is verified and reported with idempotent_replay true. An idempotency key is required and scoped to the session; refused on non-durable rrflowMX.")),
    ("session-close", reference(F::IdentityAndControl,
        "Close a session and abort its open transactions",
        "Marks the session closed, aborts its open transactions (counted in affected_open_transactions), and drops its storage cursors; refused while a transaction commit is in progress. The DELETE carries a JSON envelope and its {session} path must match X-RRD-Session. An idempotency key is required; replaying it with the same token returns the original termination with idempotent_replay true.")),
    ("session-create", reference(F::IdentityAndControl,
        "Create a session and issue its bearer token",
        "Authenticates with X-RRD-Principal and Authorization: ApiKey, or a Bearer JWT when the server has a JWT key; anonymous creation works only before security is initialized. limits sets idle and absolute timeouts (1000..=3600000 ms, idle not above absolute) and max_open_transactions (1..=256). An idempotency key is required; replaying it with the same payload returns the original lease.")),
    ("session-renew", reference(F::IdentityAndControl,
        "Renew a session and rotate its token",
        "Issues a new token, invalidating the presented one, and moves idle expiry to now plus idle_timeout_ms, capped at the unchanged absolute expiry. The {session} path must match X-RRD-Session and an idempotency key is required; replaying it with the previous token returns the same lease. A session accepts at most 64 renewals.")),
    ("subscription-close", reference(F::Delivery,
        "Close a durable subscription",
        "Marks a subscription owned by the caller's session closed, discards its unacknowledged deliveries and returns the final snapshot. Other sessions are refused and an unknown subscription_id is not found. Replaying the same idempotency key returns the closed snapshot with idempotent_replay set; a different key is refused.")),
    ("subscription-open", reference(F::Delivery,
        "Open a durable changefeed or live-query subscription",
        "Records a subscription owned by the caller's session for a changefeed or live-query stream on scope instance:{id} from after_cursor, which must lie within retention_cursor_window of the head. batch_size is 1 to 4096 and max_in_flight 1 to 64, with bounded lease and heartbeat intervals. Deliveries arrive only over /v1/ws; replaying the same subscription_id, key and request returns it.")),
    ("task-assessment-execute", reference(F::Tasks,
        "Request one source-bound controller assessment",
        "Requires EndpointPropose, TaskRead, ReasoningWrite, ProjectSourceRead, MemoryContextRead and network/source/context export consent. Binds task/config and accepted source to a binary/choice/score descriptor and bounded practice text. Reserves a task-budget step and shared controller capacity; calls configured selector once. Retains typed answer or abstention only after source and authority revalidation. Source/practice references use byte ranges. Advisory, not correctness, completion, permission, training or model evidence. Unknown calls retain capacity; read task-assessment-read, never replay.")),
    ("task-assessment-read", reference(F::Tasks,
        "Read a retained typed task assessment",
        "Requires ReasoningRead, TaskRead and the current ProjectSourceRead and MemoryContextRead grants, plus current assessment source/context export policy and selected-path eligibility. Returns only a validated accepted assessment from task-attempt storage; legacy and non-assessment attempts return an absent result. Current revocation withholds the historical answer while task-attempt-read keeps its unchanged v1 projection. A read does not dispatch or retry a controller call.")),
    ("task-attempt-read", reference(F::Tasks,
        "Read one durable attempt of a task",
        "Returns the recorded attempt for scope, task_id, and attempt_id: ordinal, reserved/accepted/stopped/invalidated status, and the accepted embedding result or endpoint proposal and digest. Invalidation identifies changed inputs or authorization. attempt is null for an unknown id. Reading never re-executes an effect.")),
    ("task-budget-configure", reference(F::Tasks,
        "Configure the reasoning budget of an open task",
        "Creates the durable reasoning budget of an open task from max_run_elapsed_ms, max_steps, and max_step_elapsed_ms, with an optional cap on equivalent attempts. Limits above the engram caps are refused. The run deadline starts when the budget is created. Repeating the same limits returns the existing budget; different limits, or a changed engram configuration, conflict.")),
    ("task-budget-read", reference(F::Tasks,
        "Read the reasoning budget of a task",
        "Returns the task's durable reasoning budget with its budget_sha256: limits, deadline, reserved steps, charged elapsed time, any active attempt, and the stop reason once stopped. Both fields are null when no budget is configured. task-budget-stop takes this digest as expected_budget_sha256.")),
    ("task-budget-resume", reference(F::Tasks,
        "Resume safely stopped work under its original task budget",
        "Requires the exact current expected_budget_sha256 and ReasoningWrite authorization. Resumes cancelled, attempt_failed or repeated_work budgets only with no active or uncertain execution, an open task, unchanged engram configuration and remaining original time and steps. Preserves spent effort, deadline, repetition counts and attempt history; dispatches nothing. An equivalent request stops again before reservation. Old attempts remain non-replayable. Stale requests conflict; after a lost reply read the budget before any further action.")),
    ("task-budget-stop", reference(F::Tasks,
        "Stop a task budget at its current digest",
        "Marks the task budget stopped as cancelled and signals any attempt running in this process. expected_budget_sha256 must equal the current budget digest or the stop conflicts; repeating a stop with its original digest replays it. A stopped budget refuses new attempts. A task without a budget returns not found.")),
    ("task-close", reference(F::Tasks,
        "Close an open task as completed or abandoned",
        "Writes the task's second and final version with status completed or abandoned and an optional outcome of at most 4 KiB. Repeating the same close replays it; any other close of a closed task fails its precondition, and an unknown task returns not found. A closed task accepts no new flights, budgets, or embedding attempts.")),
    ("task-embedding-execute", reference(F::Tasks,
        "Run one budgeted embedding attempt for a task",
        "Reserves one budget step for attempt_id, embeds up to 16 inputs (64 KiB total, at most 4,096 output values) with an activated backend, and keeps the result in the attempt record, not as vector points. Requires an open task, a configured budget, and the EmbeddingGenerate action. A reused attempt_id never re-executes. Exhausted steps or time, a failed attempt, or repeated work stop the budget.")),
    ("task-endpoint-propose", reference(F::Tasks,
        "Request an experimental endpoint proposal for a task",
        "Requires EndpointPropose, TaskRead and ReasoningWrite under initialized authority. Binds expected task and endpoint configuration digests, reserves task effort and configured concurrency, and calls the configured controller once. Changed authority or inputs withhold acceptance. Uncertain remote work retains capacity across reopen. Read task-attempt-read for recovery; a reused attempt_id never dispatches again. A proposal grants no worker effect or model qualification.")),
    ("task-list", reference(F::Tasks,
        "List task records in creation order",
        "Returns up to limit (1 to 256) task records whose first commit is after after_cursor, ascending by task_cursor, at one read stamp. truncated reports more tasks; pass the last task_cursor as after_cursor. max_storage_keys bounds the read, and an exceeded budget is refused.")),
    ("task-open", reference(F::Tasks,
        "Open a named task that prompt flights can join",
        "Commits a new rrflow-task record with status open, a one-line title of at most 256 bytes, and a goal of at most 4 KiB. The task id derives from the session and idempotency key: a replay returns the recorded task with idempotent_replay true, and the same key with a different payload conflicts. A task groups flights; it orchestrates nothing.")),
    ("task-read", reference(F::Tasks,
        "Read one task at a read stamp",
        "Returns the current version of one task record with its read stamp, selected versions, and read evidence, within max_storage_keys. The result carries no flight list; traverse rrflow-task-includes from the task in rrflowQL to reach its flights. An unknown task_id returns not found.")),
    ("task-worker-execute", reference(F::Tasks,
        "Execute one admitted selected-worker model call",
        "Requires WorkerExecute, TaskRead, ReasoningRead and ReasoningWrite, plus MemoryContextRead for requested context. Resolves the exact accepted selection and current task/configuration, reserves the existing task budget and shared binding capacity, and dispatches once. Changed authority or task/configuration withholds acceptance. Context is captured historical evidence, not current project-file permission. Output is a draft, not task success. Read task-attempt-read after uncertainty; never replay an effect.")),
    ("transaction-abort", reference(F::Transactions,
        "Abort an open transaction",
        "Marks an open transaction of the caller's session aborted and returns its lease; the body is an empty object and an idempotency key is required. Replaying the same key returns the aborted lease. Abort is refused while a commit is in progress, after commit, and once the lease has expired.")),
    ("transaction-begin", reference(F::Transactions,
        "Begin a transaction at the current read cursor",
        "Opens a transaction in the caller's session for scope claims or data and records the runtime read cursor its commit must still match. timeout_ms must be 1000 to 3600000, expiry is capped at the session's absolute expiry, and max_open_transactions bounds open leases. The id derives from the session and idempotency key: the same request replays the same lease and a different one is refused.")),
    ("transaction-commit", reference(F::Transactions,
        "Commit mutations in an open transaction",
        "Publishes 1 to 4096 mutations as one runtime commit at the transaction's read cursor, refused if any commit has advanced the runtime head since begin or the lease has expired. It requires deadline_unix_ms and an operation_sha256 equal to the SHA-256 of the mutation list, matching any preview. Replaying the same idempotency key and digest returns the stored receipt with idempotent_replay set.")),
    ("transaction-preview", reference(F::Transactions,
        "Preview a transaction's mutations without committing",
        "Evaluates 1 to 4096 mutations over the transaction's read cursor and returns the prospective read-your-writes data snapshot at valid_at (default: request time) within max_storage_keys (default 10000); nothing is published. The first preview binds the transaction to its operation_sha256: later previews must reuse that idempotency key and mutations, and commit must carry the same digest.")),
    ("vector-collection-ensure", reference(F::Recall,
        "Create or update a vector collection definition",
        "Stores a collection of 1 to 64 named vectors (unique names and fields; kind, dimensions, metric, optional embedding model, memory tier) in the scope's catalogue (at most 4,096 collections). An identical definition is kept; a different one replaces it, keeps its payload indexes, and increments generation. The same idempotency key and payload replay the receipt; a changed payload conflicts.")),
    ("vector-collection-list", reference(F::Recall,
        "List the vector collections in a scope",
        "Returns the scope's vector collection catalogue: its revision and, for each collection, the named vector definitions, payload indexes, generation, creation and update times, and configuration digest. It reads the catalogue, not points.")),
    ("vector-index-ensure", reference(F::Recall,
        "Build or advance the graph index of a named dense vector",
        "Builds the HNSW index of one collection and dense named vector, or advances it to the current source, within max_storage_keys. An index already covering the source is returned without a new generation. filter_properties name active payload indexes a filtered search can use. The build runs inside this request. An idempotency key is required: a key that published a generation returns it even after the source moves on, and another request under the key is a conflict. The response carries the build evidence. Requires a session bearer with vector_collection_ensure.")),
    ("vector-payload-index-delete", reference(F::Recall,
        "Delete a payload index",
        "Removes one payload index from a collection, refused while an active index generation filters on it. An idempotency key is required: the same key and request replay the recorded deletion, and another request under the key is a conflict. Requires a session bearer with vector_payload_index_delete.")),
    ("vector-payload-index-ensure", reference(F::Recall,
        "Declare a typed payload index on a collection",
        "Declares one typed payload index on a collection, so filters and index generations can use that property, and returns the index and the catalogue revision. An idempotency key is required: the same key and request replay the recorded index, and another request under the key is a conflict. Requires a session bearer with vector_payload_index_ensure.")),
    ("vector-point-retrieve", reference(F::Recall,
        "Retrieve points of a named vector by reference",
        "Returns the points of one collection and named vector valid at valid_at for 1 to 4,096 unique references, each with subject, value, provenance, and payload, at one read stamp. References without a visible point are listed in missing. Only the named identities are read, within max_storage_keys. An unknown collection or vector name is refused.")),
    ("vector-point-scroll", reference(F::Recall,
        "Page through the points of a named vector",
        "Returns up to limit (1 to 4,096) points of one collection and named vector valid at valid_at, ordered by reference, with an optional payload filter, at one read stamp. Pass next_after as after_reference to continue; truncated reports more points. The scope's vector versions are read within max_storage_keys, and an exceeded budget is refused.")),
    ("vector-quantization-activate", reference(F::Recall,
        "Activate a built quantization generation for search",
        "Activates one ready generation of a quantization artifact after verifying its stored bytes, and retires the generation it replaces. Approximate search may then select it; its candidates are reranked exactly against canonical vectors, and vectors written after it are reranked from their versions. An idempotency key is required: the same key and request return the recorded artifact, as it is now, with idempotent_replay true, and another request under the key conflicts.")),
    ("vector-quantization-build", reference(F::Recall,
        "Build a quantized generation of a named dense vector",
        "Encodes the visible points of one collection and dense named vector into the next immutable generation, in ready state, with scalar, product, binary, or TurboQuant quantization. Every filter property needs a payload index. The build reads the scope's vector versions within max_storage_keys, and search is unchanged until activation. An idempotency key is required: the same key and request return the recorded generation with idempotent_replay true, and another request under the key conflicts.")),
    ("vector-quantization-list", reference(F::Recall,
        "List quantization artifacts and their lifecycle states",
        "Returns up to max_artifacts quantization artifacts of a scope, optionally of one collection and named vector, each with its method, state (ready, active, or retired), generation, source cursor, sizes, and digests, with the lifecycle revision. It reads the lifecycle catalogue, not points.")),
    ("vector-quantization-retire", reference(F::Recall,
        "Retire a quantization generation from search",
        "Retires one built generation, so approximate search no longer selects it; canonical vectors and other generations are unchanged. An idempotency key is required: the same key and request return the recorded artifact, as it is now, with idempotent_replay true, and another request under the key conflicts.")),
    ("vector-search", reference(F::Recall,
        "Search for the nearest vectors to a query",
        "Scores points valid at valid_at against a dense, sparse, or multi-dense query and returns up to top_k (1 to 100,000) hits at one read stamp, with the access path used. Address exactly one of collection_id with vector_name, or field with metric. exact is the default mode; approximate modes need top_k <= exact_rerank <= ef_search, and require_approximate is refused without a ready approximate path.")),
    ("websocket-connect", reference(F::Delivery,
        "Open a WebSocket for durable subscription delivery",
        "Upgrades GET /v1/ws for a session bearer, sends a connected frame with the connection limits, then exchanges sequenced JSON text WebSocketFrames. Subscribe resumes an open subscription at its exact acknowledged cursor under a new connection generation; deliveries pause at max_in_flight, acks advance the cursor and heartbeats renew the lease. Request frames are refused with failed_precondition.")),
];
