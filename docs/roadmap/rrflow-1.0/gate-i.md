# Gate I — add explicit automation scaffolding with host hooks as alias stubs

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-i`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | I-01 | Define one canonical engine-event envelope with producer, action, target, scope, stamp, idempotency key, provenance, and authorization coordinates; directly converge the current kernel `RuntimeEvent` representation into that vocabulary without a parallel event log or compatibility type. | `rrd-contract`, `rrd-core` | Golden, lowering, commit, and replay tests reject ambiguous identity, duplicate mismatches, unbounded payloads, and events outside the authenticated engram while proving public and persisted forms are one semantic event. |
| [ ] | I-02 | Implement triggers as persisted conditions over canonical committed events; a trigger may request an authorized operation but cannot commit independently. Keep transaction function bindings as the separate pre-commit validation/proposal mechanism defined by the governed-function contract, with no `Trigger` type, field, operation, or persisted marker shared between them. | `rrd-engine` | Match/non-match, denial, duplicate, ordering, recursion-depth, and restart tests prove deterministic bounded behavior; direct-convergence tests reject every old `FunctionTrigger*` shape and show transaction bindings cannot consume committed-event cursors or schedule routines. |
| [ ] | I-03 | Implement optional durable routines as versioned resumable graphs of authorized engine operations with explicit effecting inputs, checkpoints, budgets, cancellation, verification, and terminal status. The graph records public operations and observable effects, not every exploratory or private reasoning step. Model, process, network, and external MCP effects are prepared fenced activities: adapters return bounded observations, while `RrdEngine` alone accepts receipts and advances state. Express repeatable error resolution and context projection maintenance as routine definitions, not operation-specific repositories or state machines. | `rrd-contract`, `rrd-engine`, outward activity adapters | Generic kill/restart, retry, compensation, stale-input, denial, maximum-step, prepared/effect/receipt crash-gap, lost-ack, uncertain-outcome reconciliation, redaction, and resource-limit tests plus both template corpora prove no busy loop, silently repeated mutation/effect, direct storage access, private event log, adapter-authored completion, client-owned state, or requirement to serialize hidden model reasoning. |
| [ ] | I-04 | Implement host-event adapters as stateless translators that submit typed events only when explicitly installed and configured. Bind an editor or provider hook to an adapter only as an [alias stub](../../reference/agent-bootstrap.md#host-hooks) generated from an RRFlow-owned host binding. | outward adapters | Claude/OpenAI/reference adapter conformance produces the same envelope; each generated hook stub holds only its invocation and matches its binding digest, a changed binding regenerates it, and a drifted stub is reported; uninstall removes the adapter and its stubs cleanly, leaves project-owned host configuration as it was, and leaves canonical state readable. |
| [ ] | I-05 | Implement skills as versioned instruction/resource packages referenced by identity and digest, resolved through governed context rather than executed as storage or lifecycle code. | `rrd-contract`, `rrd-engine` | Install/resolve/update/retire tests prove provenance, authorization, version pinning, prompt-budget enforcement, and no implicit mutation. |
| [ ] | I-06 | Add previewable install/configure/retire/uninstall scaffolding for governed functions, transaction function bindings, triggers, routines, host-event adapters and their hook stubs, skills, and optional capability/activity bindings after their individual contracts pass. Default function runtimes/artifacts come only from the manifest-verified distribution; attunement may propose an inactive project function but cannot compile, fetch, activate, or execute it. | `rrflow-cli`, adapters | Fresh/existing project tests show exact planned artifacts/schemas/files/records/invocation closures, explicit consent, preview without execution, idempotent apply, retirement that blocks new work without deleting receipts, clean uninstall of only unreferenced RRFlow-owned scaffolding, offline readiness, and no session-start loop. |
| [ ] | I-07 | Before a project-development routine persists project knowledge, applies a source effect, resumes durable work, or claims snapshot-complete evidence, bind the relevant complete authorized project-tree snapshot. Exploratory inspection and planning may use explicitly provisional live evidence. React to committed inventory, schema, dependency, workload, and failure signals by scheduling only the required incremental attunement phases and evaluating eligible capability, routine, and skill activation under engram policy. | `rrd-engine`, `rrd-attunement` | Missing/stale coverage returns `inventory-required` only for the dependent durable effect or claim; changed-since-plan entries and paths outside the root are denied before commit; adding one language, framework, data source, or recurring failure triggers the minimal bounded work, survives restart, records its decision evidence, and never turns an ad hoc client observation into canonical state, performs a blanket reinstall, or activates without authorization. |

## RROFLOW improvement packages

The [learning contract](../../reference/inference/learning/controller-training.md)
defines RROFLOW as governed improvement using the existing engine, not a
Python lifecycle. These open packages refine I-03/I-07 and do not reorder the
dependency spine or complete either item.

| Package and prerequisite | Canonical boundary | First failure oracle and required exit |
|---|---|---|
| I-07 learning candidates, after committed inventory and I-01/I-07 signal foundations | Existing outcome, correction, configuration and evidence records under `RrdEngine` | A repeated failure can propose a bounded rule/example/model experiment with source and consent identities. Denied training consent, revoked sharing, missing labels and stale inventory block dependent export/effects. Clustering never becomes a correctness verdict. Profile preferences and weight updates remain separate proposals. |
| I-03 RROFLOW promotion, after generic I-03 activity/recovery proof, D-06 configuration and G-06 candidate qualification for models | Generic routines and existing configuration CAS, artifact admission and receipts | Race two promotions against one revision; accept at most one. Kill before/after external training, after result receipt and after activation before ACK. Reconcile uncertain work, reject stale fences and revoke before effects. Duplicate completion cannot deploy twice. Rollback creates a new authorized compatible revision and preserves prior evidence. |

Only a qualified explicit policy can authorize bounded automatic promotion.
Installation does not enable a learning loop. I-06 scaffolding and H-06 client
views consume these existing capabilities after their acceptance; they cannot
declare a checkpoint active or mark a routine complete themselves.

## Provider observation for RROFLOW

The intended experience is one investigation view for available activity from
Codex, Claude and Grok: sessions, model/API activity, tool decisions and
results, errors, timing, usage, and permitted conversation or tool content.
Operators can organize retained observations into signals and tags, follow
related activity and inspect evidence. RROFLOW builds on the one RRFlow engine
and its governed learning boundary; a provider, collector or hosted service
does not become a second event, audit, project-state or completion authority.
This is a target, not a claim that any feed or rroflow.ai service is active.

| Source | Documented intake to qualify | Coverage that must not be inferred |
|---|---|---|
| Codex | [Official OpenAI configuration](https://learn.chatgpt.com/docs/config-file/config-advanced#observability-and-telemetry) describes opt-in OTLP logs for conversations, API requests, tool decisions and results, with prompt-content controls. | An enabled exporter does not establish historical backfill, all client surfaces, complete delivery or hidden reasoning. Verify the actual installed version, event families and configured destination. |
| Claude Code | [Official monitoring documentation](https://code.claude.com/docs/en/monitoring-usage) describes configurable OTLP metrics, logs and optional traces, including tool results and separate content settings. | CLI, Desktop Code and Cowork configuration have different reach; one configured source does not establish coverage of every Claude product or conversation. |
| Grok Build | [Official headless documentation](https://docs.x.ai/build/cli/headless-scripting) describes newline-delimited streaming JSON and ACP for sessions started through those interfaces. | A controlled session stream does not establish a global OTLP feed or access to independent/historical Grok chats. Qualify the actual installed version and supported events. |

Execute bounded packages in dependency order:

1. **I-01 engine admission and identity.** Converge the existing canonical
   event representation around authenticated engram/principal, configured
   source binding and revision, provider/session/event identity or an
   explicitly qualified stable stream checkpoint, event kind/time, provenance,
   payload digest, authorization and retention policy. The engine owns the
   event and its receipt. Reconnect and a new transport session must resolve
   the same source event to the same accepted result; a reused identity with
   different content must conflict. A claimed provider name or a client-side
   cache is not authenticated source identity. The current transaction
   replay is tied to its transaction/session and does not by itself establish
   this cross-session deduplication.
2. **I-04 source adapters.** Implement stateless, versioned translators for
   each qualified documented source. Preview exact supported events, scopes,
   endpoint, credentials, content classes, limits and configuration; install
   and activate only the selected binding. Stream or import through the same
   engine admission, with bounded queues, backpressure, retry, checkpoints,
   explicit loss/unsupported states and idempotent replay. A provider or
   editor hook is only an alias stub onto the adapter's binding; do not read
   undocumented account stores.
   Codex/Claude OTLP and Grok controlled streaming are distinct transports;
   unimplemented transports cannot be labelled connected.
3. **H-05 signals and H-06 views.** Project retained, authorized observations
   and the existing signal catalogue into correlated timelines, error groups,
   usage and latency views, and operator-defined versioned tags. A tag or
   clustering suggestion is a derived annotation with provenance, not evidence
   that the source event, task, external effect or underlying claim succeeded.
   Show source coverage, dropped events, delay, duplicates, truncation and
   unknown cost/quality explicitly. Bound cardinality and preserve raw-event
   identity separately from derived groups.
4. **H-07 transfer and I-03/I-07 learning.** Keep local capture usable without
   a cloud account. A selected remote destination requires an authenticated
   binding, exact dataset/purpose preview, authorization, resumable transfer,
   durable receipts and revocation behavior. Collection, remote transfer,
   training and promotion are separate decisions; telemetry does not grant
   training or effect authority. Never represent hidden model reasoning as
   captured provider content.

Acceptance runs real configured provider versions and an installed engine:
event emission, receipt and readback; three-source normalization without
semantic flattening; restart/reconnect and cross-session deduplication; changed
duplicate rejection; untrusted producer denial; partial, out-of-order and
missing events; queue exhaustion and recovery; source and destination revocation;
content-scope and secret handling; audit readback; and local operation while
remote delivery is unavailable. Retain exact versions, configuration and
coverage. Use real provider/collector behavior for claims; fixtures prove only
their declared input cases. None of these steps closes C-07 or the learning,
observability or service gates without their own acceptance evidence.

## Exit condition

Gate I exits only when automation is explicit, bounded, replayable, removable,
and subordinate to `RrdEngine`; installation alone is never evidence that a
trigger, routine, adapter, hook stub, or skill worked.

## Required end-to-end corpus

The Gate I end-to-end corpus must include the generic `error-resolution`
vertical slice defined by the
[automation flow](../../architecture/engine-data-flow.md#first-complete-automation-proof).
It must prove one committed diagnostic can activate, resolve governed skills
and context, select or reject graph/BM25/vector/TurboQuant/DataFusion work,
invoke an attuned verification capability, survive restart without repeating
effects, and expose the same persisted result through MCP and the other public
surfaces.
