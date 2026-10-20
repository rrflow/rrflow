# Gate I — add explicit automation scaffolding without automatic hooks

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-i`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | I-01 | Define one canonical engine-event envelope with producer, action, target, scope, stamp, idempotency key, provenance, and authorization coordinates; directly converge the current kernel `RuntimeEvent` representation into that vocabulary without a parallel event log or compatibility type. | `rrd-contract`, `rrd-core` | Golden, lowering, commit, and replay tests reject ambiguous identity, duplicate mismatches, unbounded payloads, and events outside the authenticated estate while proving public and persisted forms are one semantic event. |
| [ ] | I-02 | Implement triggers as persisted conditions over canonical committed events; a trigger may request an authorized operation but cannot commit independently. Keep transaction function bindings as the separate pre-commit validation/proposal mechanism defined by the governed-function contract, with no `Trigger` type, field, operation, or persisted marker shared between them. | `rrd-engine` | Match/non-match, denial, duplicate, ordering, recursion-depth, and restart tests prove deterministic bounded behavior; direct-convergence tests reject every old `FunctionTrigger*` shape and show transaction bindings cannot consume committed-event cursors or schedule routines. |
| [ ] | I-03 | Implement optional durable routines as versioned resumable graphs of authorized engine operations with explicit effecting inputs, checkpoints, budgets, cancellation, verification, and terminal status. The graph records public operations and observable effects, not every exploratory or private reasoning step. Model, process, network, and external MCP effects are prepared fenced activities: adapters return bounded observations, while `RrdEngine` alone accepts receipts and advances state. Express repeatable error resolution and context projection maintenance as routine definitions, not operation-specific repositories or state machines. | `rrd-contract`, `rrd-engine`, outward activity adapters | Generic kill/restart, retry, compensation, stale-input, denial, maximum-step, prepared/effect/receipt crash-gap, lost-ack, uncertain-outcome reconciliation, redaction, and resource-limit tests plus both template corpora prove no busy loop, silently repeated mutation/effect, direct storage access, private event log, adapter-authored completion, client-owned state, or requirement to serialize hidden model reasoning. |
| [ ] | I-04 | Implement host-event adapters as stateless translators that submit typed events only when explicitly installed and configured; ship no editor/provider-owned automatic hook. | outward adapters | Claude/OpenAI/reference adapter conformance produces the same envelope; uninstall removes the adapter cleanly and leaves canonical state readable. |
| [ ] | I-05 | Implement skills as versioned instruction/resource packages referenced by identity and digest, resolved through governed context rather than executed as storage or lifecycle code. | `rrd-contract`, `rrd-engine` | Install/resolve/update/retire tests prove provenance, authorization, version pinning, prompt-budget enforcement, and no implicit mutation. |
| [ ] | I-06 | Add previewable install/configure/retire/uninstall scaffolding for governed functions, transaction function bindings, triggers, routines, host-event adapters, skills, and optional capability/activity bindings after their individual contracts pass. Default function runtimes/artifacts come only from the manifest-verified distribution; attunement may propose an inactive project function but cannot compile, fetch, activate, or execute it. | `rrflow-cli`, adapters | Fresh/existing project tests show exact planned artifacts/schemas/files/records/invocation closures, explicit consent, preview without execution, idempotent apply, retirement that blocks new work without deleting receipts, clean uninstall of only unreferenced RRFlow-owned scaffolding, offline readiness, and no session-start loop. |
| [ ] | I-07 | Before a project-development routine persists project knowledge, applies a source effect, resumes durable work, or claims snapshot-complete evidence, bind the relevant complete authorized project-tree snapshot. Exploratory inspection and planning may use explicitly provisional live evidence. React to committed inventory, schema, dependency, workload, and failure signals by scheduling only the required incremental attunement phases and evaluating eligible capability, routine, and skill activation under estate policy. | `rrd-engine`, `rrd-attunement` | Missing/stale coverage returns `inventory-required` only for the dependent durable effect or claim; changed-since-plan entries and paths outside the root are denied before commit; adding one language, framework, data source, or recurring failure triggers the minimal bounded work, survives restart, records its decision evidence, and never turns an ad hoc client observation into canonical state, performs a blanket reinstall, or activates without authorization. |

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

## Exit condition

Gate I exits only when automation is explicit, bounded, replayable, removable,
and subordinate to `RrdEngine`; installation alone is never evidence that a
trigger, routine, adapter, or skill worked.

## Required end-to-end corpus

The Gate I end-to-end corpus must include the generic `error-resolution`
vertical slice defined by the
[automation flow](../../architecture/engine-data-flow.md#first-complete-automation-proof).
It must prove one committed diagnostic can activate, resolve governed skills
and context, select or reject graph/BM25/vector/TurboQuant/DataFusion work,
invoke an attuned verification capability, survive restart without repeating
effects, and expose the same persisted result through MCP and the other public
surfaces.
