# RRFlow context projection maintenance

**Status:** active target contract; the current standalone maintenance repository is not accepted
**Coordinate:** `rrflow://rrflow-instance/data/reference/context/context-maintenance`
**Owner:** safe derivation, evaluation, activation, and rollback of context projections

Context maintenance reduces what RRFlow injects into a model request without
deleting or rewriting canonical rrflowDB knowledge, reasoning state, evidence,
source history, or project files. It is an application of the generic routine
and context contracts, not another lifecycle engine.

The [engine data flow](../../architecture/engine-data-flow.md) owns the common
transaction, context, and evidence paths. Roadmap
[Gates H and I](../../roadmap/rrflow-1.0.md#gate-h--prove-context-flow-feedback-live-delivery-and-connectome)
own implementation and acceptance. The
[POA&M](../../poam/rrflow-1.0-alpha.md) records the current standalone-code
conflict.

## Authority boundary

`RrdEngine` is the only authority that may capture a source stamp, authorize a
maintenance operation, persist a routine checkpoint, activate a projection,
or commit a rollback. A maintenance definition cannot:

- open or replay a storage engine directly;
- create a private scope, event log, repository, scheduler, or state machine;
- select rrflowKV keys, indexes, DataFusion nodes, providers, or model runtimes;
- infer success from traces, client state, or an absent error; or
- delete canonical data to satisfy a size or percentage target.

The routine uses versioned public RRFlow operations. Its mutable run state,
leases, checkpoints, cancellation, compensation, and terminal outcome are the
same generic routine records required by I-03. Context maintenance does not
receive operation-specific lifecycle machinery.

## Source and projection semantics

One proposal is bound to a single authorized `ReadStamp`, context-policy
revision, project-tree snapshot when applicable, model/tokenizer manifests
used for measurement, and evaluation-corpus digest. The inventory accounts for
every eligible source and explicitly records exclusions, errors, and
truncation.

The result classifies context by delivery behavior rather than storage
temperature:

- **injected** — included in the bounded default request context;
- **recall eligible** — retained and addressable through governed retrieval,
  but not injected by default; and
- **retained evidence** — preserved for audit, replay, recovery, or later
  reprocessing and omitted from ordinary model context.

These labels do not move rrflowKV pages between physical hot, warm, or cold
storage tiers. Source classifications and protected categories come from a
versioned engram policy or attuned specialization. The engine does not hardcode
a universal document taxonomy, review cadence, token-reduction percentage, or
deletion quota.

Age, last verified access, repeated successful contribution, contradictory
evidence, provenance, and a policy-selected retention model may be inputs to a
priority or retirement proposal. An Ebbinghaus-style score is therefore an
optional policy feature, never a compaction verdict. Legal/retention holds,
immutable audit evidence, current project truth, and recovery dependencies are
evaluated before a proposal can be accepted. `RrdEngine` commits the resulting
versioned disposition explicitly; only then may ordinary compaction reclaim a
now-unreachable physical version.

A context projection is derived acceleration state. Its identity binds the
source stamp, parent generation, policy and builder revisions, selected source
identities and digests, ordering, estimated and measured bytes/tokens, and
projection digest. It cannot outrank canonical data or remain eligible after
its bound source, policy, model, or schema becomes incompatible.

## Generic routine template

An installed context-maintenance template expresses the following semantic
steps through the generic routine graph:

1. **Inventory** — capture one stable source stamp and bounded source/class
   accounting without changing active context.
2. **Protect** — verify a recovery point that covers the source cut and every
   non-rebuildable referenced object required by policy.
3. **Propose** — construct a deterministic candidate projection and report the
   exact included, recall-eligible, retained, and excluded identities.
4. **Review** — persist configured operator or policy decisions; an override
   records its actor, reason, and affected identities.
5. **Validate** — execute the declared fixed-corpus replay and safety gates,
   using the [model-context effect plan](../../evidence/test-plans/model-context-effect.md)
   when a model-assisted outcome is part of the claim. Pending or failed
   evidence keeps activation closed.
6. **Activate** — compare-and-swap one new projection generation through
   `RrdEngine`; source, projection, activation, outbox, and audit coordinates
   commit atomically.
7. **Observe** — compare actual bounded outcomes with the accepted baseline and
   either accept the generation or commit a compensating activation of the
   prior verified projection.

Those steps are a reusable routine definition, not Rust enum variants that
create another workflow authority. A definition revision never mutates an
existing run, and restart resumes from the last committed generic checkpoint.

## Mandatory validation

Activation requires retained evidence for:

- recovery-point integrity and source-stamp coverage;
- recall of every policy-protected goal, constraint, decision, security rule,
  and reusable failure lesson;
- deterministic reproduction of the candidate projection;
- identical evaluation inputs, project snapshot, model manifests, resource
  limits, and verification rubric for baseline and candidate;
- task success, answer/source correctness, recall quality, context bytes and
  tokens, latency, and total physical work;
- authorization, classification, legal-retention, and redaction policy; and
- rollback availability plus readback after close and reopen.

No universal reduction range is an acceptance gate. A candidate that safely
reduces nothing is a valid measured result. Approximate or model-scored checks
identify their model and corpus digests and cannot replace deterministic
structural checks.

## Context, graph, index, and DataFusion flow

Inventory and evaluation use the same context operation and physical planner
as interactive requests. `RrdEngine` captures the stamp; rrflowQL may select
native graph, BM25, scalar, exact-vector, HNSW, or streamed Arrow/DataFusion
work; the result retains selected/skipped reasons, source and projection
coordinates, contribution, and resource evidence. The maintenance routine does
not build a private retrieval path.

An activation changes only which verified context projection is eligible for
later requests. It does not mutate query-time RRF weights. A learned policy
change is a separately versioned later-stamp mutation under H-02, with its own
regression and rollback evidence.

## Connectome and adapter boundary

Connectome may preview the exact proposal, display validation evidence, request
authorized review/activation/rollback operations, and compare generations via
public RRD capabilities. Draft UI edits are not persisted decisions, and the
client cannot advance a routine from a dropdown, timeout, disconnected stream,
or local state.

Provider and host adapters may submit observable evidence or invoke the same
public operations when explicitly configured. They cannot install a hidden
hook, maintain a second run state, or translate provider context-window events
into automatic deletion.

## Current implementation disposition

`crates/operations/rrd-maintenance/src/lib.rs` contains useful draft validation
ideas: digest-chained revisions, complete evidence-gate sets, projection
generations, explicit review, rollback, and fail-closed recovery checks. It is
not the accepted implementation. The crate currently:

- depends directly on `rrd-store::StorageEngine` and commits outside
  `RrdEngine` authorization;
- owns a fixed seven-stage state machine, private `maintenance:*` scope, and
  maintenance-specific event family;
- reconstructs its state by scanning retained runtime changes from cursor
  zero;
- embeds authoritative run and projection structures as JSON strings inside
  records;
- hardcodes eight history classes and a 50–75% reduction range; and
- has no focused test target and no caller outside its own package.

### Existing capability disposition

The implementation is not deleted merely because its boundary is wrong. The
following matrix is the required A-07 preservation ledger. A later source
change must cite the destination and acceptance proof for every **preserve** or
**generalize** row before removing the current construct.

| Current construct or behavior | Disposition | Canonical destination and required proof |
|---|---|---|
| `MaintenanceInventory` and `HistoryClassInventory`: stable source cut, per-class counts, checked byte/token totals, derivation labels | Preserve | A typed context-projection inventory produced at one `ReadStamp`; tests reject overflow, duplicate/missing policy classes, total mismatch, missing derivation, and stale or incomplete sources. |
| `HistoryClass`, `HistoryClass::ALL`, and built-in default dispositions | Replace | Versioned engram policy or attuned specialization defines classifications and protected categories. No Rust enum freezes a universal project history taxonomy. |
| `ContextDisposition::{Hot,Warm,Cold}` | Generalize | The delivery labels `injected`, `recall eligible`, and `retained evidence`; tests prove they alter projection eligibility without claiming physical storage movement or deleting canonical data. |
| `MaintenanceDecision`: proposed versus selected disposition, rationale, and mandatory operator note on override | Preserve | Typed proposal/review records in generic routine checkpoints; tests reject an unaccounted source and an unexplained override and retain actor plus affected identities. |
| `ProjectionPlan`: deterministic accounting, checked totals, source binding, and content digest | Preserve | A content-addressed projection proposal bound to exact source/policy/builder/model revisions; deterministic rebuild and tamper tests must pass. |
| Fixed `DEFAULT/MIN/MAX_TARGET_REDUCTION_BPS` and the 128–32,000 injected-token clamp | Reject | Engram policy and request budgets declare goals and ceilings. A safe zero-reduction result remains valid, and tests cannot require a universal percentage or token range. |
| `ProtectionEvidence`: verified archive/catalogue identity and coverage of both source watermarks | Generalize | A verified recovery point covering the exact source cut and every non-rebuildable referenced object; short-coverage, corrupt, wrong-engram, and reopen tests fail closed. |
| `ValidationGate`, exact gate-set checks, pending/pass/fail status, summaries, and evidence digests | Generalize | The routine revision declares its complete required evidence set. Tests reject missing, duplicate, unknown, digestless terminal, stale-input, and failed evidence and keep activation closed. |
| `MaintenanceObservation`: measured task success, context tokens, latency, evidence, and accept/rollback decision | Preserve and extend | Typed observation evidence following the [model-context effect plan](../../evidence/test-plans/model-context-effect.md), including quality, recall, work, bytes/tokens, latency, failures, and declared evaluator provenance. |
| `MaintenanceRun` revision, predecessor/state digests, actor/time attribution, terminal guards, and required-stage shape | Generalize | Generic `RoutineRun` plus immutable checkpoints; CAS/restart/tamper tests prove monotonic revisions, digest lineage, required inputs, terminal immutability, and no inferred completion. |
| `MaintenanceStage` and `MaintenanceStatus` as a fixed operation-specific state machine | Replace | Inventory/protect/propose/review/validate/activate/observe are nodes in one versioned routine definition using the generic I-03 status vocabulary; other routines use the same executor. |
| Expected run revision and captured runtime cursor conflict checks | Preserve | `RrdEngine` compare-and-swap over routine and active-projection records at one authorized write stamp; stale revision/cursor tests commit nothing. |
| Only one active maintenance run and one instance-global active projection | Generalize | Engram policy defines concurrency and a typed projection address defines the active generation. Idempotency and conflict tests prevent competing activation without imposing a global singleton on unrelated context profiles. |
| `ActiveMaintenanceProjection`: monotonic generation, predecessor digest, source run, active/rollback lineage, and prior plan | Preserve | Immutable projection generations plus one CAS-controlled active pointer; close/reopen, competing activation, lineage corruption, and compensating rollback tests pass. |
| Verification that rollback still owns the active generation it applied | Preserve | An observation can compensate only the exact active generation produced by its run; intervening activation causes a conflict and no mutation. |
| Atomic publication of run revision, matching event, and optional projection | Preserve | One authorized rrflowKV batch commits checkpoint, canonical engine event, active-pointer change, outbox, and audit evidence all-or-nothing under C-03/I-01/I-03. |
| `MaintenanceEvent` and matching state/event replay validation | Generalize | One canonical engine-event envelope references the routine/checkpoint identity and digest; replay tests detect missing, duplicate, reordered, or disagreeing evidence without making events the run authority. |
| `runs`, `active_run`, `run`, and `active_projection` reads | Preserve | Bounded public `RrdEngine` operations using direct typed rrflowKV keys at one stamp; HTTP/WebSocket/SDK/CLI/MCP/Connectome conformance resolves identical identities. |
| `start`, `protect`, `propose`, `review`, `validate_evidence`, `apply`, `observe`, and `cancel` repository methods | Generalize | Generic routine start/step/review/cancel operations and the context-projection operation family; kill/retry/lease/cancellation/compensation tests exercise the definition through the shared executor. |
| Direct `StorageEngine`, `commit_runtime`, private `maintenance:*` scope, private schema/event types, cursor-zero replay, and JSON-wrapper records | Reject | Authorized engine operations, canonical engram scope, typed records, canonical events, and bounded direct reads. Repository and dependency searches must show the old path absent. |
| `MaintenanceError`, `MAINTENANCE_FORMAT_VERSION`, `default_decisions`, and `pending_gates` as package-local API | Replace | Public contract errors, versioned routine/template manifests, and digest-bound installed definition data; no forwarding alias or compatibility reader remains. |

A-07 must identify the reusable validation logic behind the preserved rows and
assign it to its final contract or engine module before structural edits. I-03
then re-expresses behavior as a generic routine definition plus public engine
operations; C-03/C-04 provide atomic writes and bounded stamped reads; H-05
provides evidence. J-01 may remove `rrd-maintenance`, its dependency edge,
private vocabulary, and JSON storage only after the new focused corpus covers
every preserved/generalized row. No forwarding wrapper or compatibility type
survives.

## Acceptance

This contract is implemented only when the same generic routine executor used
by other project work can preview, authorize, interrupt, resume, activate,
observe, and roll back a context projection; every step survives rrflowKV
reopen; baseline and candidate evidence are reproducible; and HTTP, WebSocket,
SDK, CLI, MCP, and Connectome resolve the same run and projection identities.
The existing standalone types, a successful compile, or deletion of the old
crate do not satisfy that proof.
