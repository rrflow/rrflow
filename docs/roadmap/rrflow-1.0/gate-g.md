# Gate G — connect LFG without creating another engine

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-g`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | G-01 | Add a provider-neutral `RouterBackend` capability separate from `EmbeddingBackend`; implement optional LFG CPU analysis and configured SWAG selection/engine-admitted dispatch under the [LFG/SWAG contract](../../reference/inference/lfg-swag.md). | `rrd-inference`, `rrd-engine` | Reference and real router adapters pass descriptor, bounds, timeout, invalid-output and digest checks; LFG observations are separately validated, optional and bounded, with no model required for deterministic analysis. |
| [ ] | G-02 | Build a bounded route packet from one stamped tree, eligible recipes, verified observations, and allowed query fields. Bind the authenticated principal, current provider-representation edge, resolved durable seat, policy revision, and authorization digest at that same read coordinate; a caller-supplied actor label is never identity evidence. | `rrd-engine` | Golden packet and denial corpus prove representation establishes attribution rather than permission, stale/revoked/foreign representations fail closed, and the model sees no raw KV keys, secrets, hidden reasoning, unauthorized fields, or unbounded workspace content. |
| [ ] | G-03 | Constrain model-backed SWAG routing to the three existing routing decisions and validate again after decoding; validate LFG observations separately. | SWAG router adapter and engine | Corpus includes valid, malformed, unknown-recipe, unauthorized-query, stale-cursor, and prompt-injection cases; invalid decisions produce no mutation. |
| [ ] | G-04 | Execute recipe selection and branch navigation on the fast path without rrflowQL/DataFusion; keep deterministic predicates and CAS mutation in `RrdEngine`, and commit the resolved seat attribution with the resulting tree state, audit, and causal trace. | `rrd-engine`, `rrd-store` | Trace and physical-plan evidence show bounded rrflowKV operations, no DataFusion plan, conflict denial, no actor-string impersonation, and correct attributed state after reopen. |
| [ ] | G-05 | Lower `request_context` into semantic rrflowQL/context intent while leaving physical access selection to the engine. | `rrd-engine`, `rrd-query` | Neither LFG nor SWAG can select a physical index/storage backend; resulting plan is authorized, stamped, budgeted, and equivalent to a typed SDK request. |
| [ ] | G-06 | Publish model and storage latency separately with task-success, routing-accuracy, invalid-decision, and escalation metrics. | evaluation harness | Reproducible hardware/model manifest and raw samples support every reported latency or quality claim. |

## Bounded reasoning implementation packages

The [researched rationale](../../architecture/engine-scale-and-contracts.md)
and [structure/input contract](../../reference/data/schema-catalogue.md#structure-and-input-contract-plan)
support this owner. The [LFG/SWAG reference](../../reference/inference/lfg-swag.md)
corrects the earlier combined LFG/router framing without changing the existing
three-decision wire union. All packages below are open. The
[dependency spine](executable-dependency-spine.md) remains in force: D-01 is
active; installed durability, inventory, native access and analytics cannot be
skipped by adding a model loop. Specification/golden preparation can proceed
without claiming runtime completion. Each implementation package is one
bounded PR, split further when its exact diff requires it.

| Package and prerequisite | Canonical change boundary | First failure oracle and required exit |
|---|---|---|
| G-01a: bounded adapter execution, after Gate G prerequisites | `rrd-inference/src/lib.rs` backend ownership/dispatch and `rrd-engine/src/engine/inference.rs`; extend existing admission/configuration rather than add a second scheduler | Two controlled backends: block A, prove listing and admitted B progress without holding the registry mutex; configured per-backend and estate limits reject excess work before allocation. Test queue deadline, revocation, cancellation, late output and permit release. Preserve descriptor/network/provenance and embedding shape tests. |
| G-01b: LFG/SWAG configuration and router port, after G-01a and D-06 governed configuration | Existing public router/model manifest contracts, Answer/Address classification boundary and inference admission; engine-owned configuration/adapter installation | LFG disabled/rule-based/inconclusive cases and SWAG hard constraints pass independently. Reference adapter and the selected real router adapter run one corpus: exact model/tokenizer/grammar/runtime identity, byte limits, unavailable model/device, malformed output, timeout and network denial. An unavailable required real artifact blocks that model-backed profile; a fake proves only the port. Ordered fallback never bypasses residency/permission/cost limits; unsupported cache isolation is unavailable, not assumed. |
| G-02a: durable tree and dispatch identity, after G-01b and accepted transaction/storage prerequisites | Kernel `reasoning_tree.rs`, store semantic plan, engine runtime/job composition; preserve existing task/flight contracts | Competing workers target one expected cursor; only one accepted transition advances it. Kill before dispatch, after external result and after durable commit/before ACK; reopen resolves accepted receipt without re-execution and reports unresolved work honestly. Persisted fences reject stale workers. Public observations and receipts are stored; private thought is not. |
| G-02b: authorized route packet, after G-02a | `rrd-contract/src/router.rs::RouteStepRequest`, engine security/seat/context projection | Bind principal, representation, durable seat, policy and relevant source/read identity. Golden packet plus expired/revoked/foreign/stale/oversized cases prove unauthorized fields and secret/raw-KV content never reach the adapter. Schema validity alone does not authorize. |
| G-03: constrained response and semantic admission, after G-02b | Existing three-decision union, selected SWAG router decoding adapter and engine validators | Invalid grammar/unknown recipe/edge, unsupported parameters, prompt injection, changed descriptor, stale cursor and deadline all produce no accepted mutation. Valid JSON with forbidden meaning still fails. Prove exact raw-output/input digests and bounded diagnostics without logging content by default. |
| G-04: governed fast-path transition, after G-03 | Existing engine predicates/CAS and store atomic semantic transaction | Recipe/branch path executes no DataFusion plan; accepted tree state, attribution, receipt, audit and outbox are atomic across crash/reopen. Duplicate completion and lost ACK return the same accepted result; payload conflict and stale fence cannot mutate. Measure lock/queue/commit separately. |
| G-05: context escalation, after G-04 and accepted E/F access paths | Existing typed rrflowQL/context request, planner and one budget owner; full H-01 dynamic-routing qualification follows Gate G | Route-requested context equals a typed SDK request at the same stamp and permissions. Model names semantic intent, not index/backend. Repeated context requests consume one remaining budget, terminate on explicit limits and cannot form an unbounded retry loop. |
| G-06: installed reasoning qualification, after G-01 through G-05 | Installed `rrflow` process, model manifest, evaluation harness and H-05 causal evidence | One real prompt exercises an admitted model, a fast step and context escalation, persists an attributed result, and survives restart/cancellation/lost ACK. Publish routing accuracy, invalid-decision rate, task success/abstention, citation support, model versus storage/queue time, tokens and resource peaks on declared fixtures. |

For G-02a through G-04, extend the existing tree/job/receipt authority only
after a package-specific mapping of source types, storage keys and current
characterization tests. The recent process-local invocation token is retained
but cannot substitute for a persisted fence. Generic routine/tool execution
still belongs to Gate I; route execution may not create an alternate external
activity implementation.

Run the existing characterization commands before the first relevant edit:

```text
cargo test --locked -p rrd-contract --test reasoning_tree_contract
cargo test --locked -p rrd-core --test reasoning_tree_contract
cargo test --locked -p rrd-engine --lib overlapping_invocation
cargo test --locked -p rrd-inference
```

Then add each row's failing oracle at its canonical boundary, run that test,
the affected packages and strict linting, followed by candidate CI. The
commands above do not prove the planned cases already exist or pass. Fix test
names from the actual checkout before running; keep real-model and installed
acceptance separate from reference-adapter results.

### Aggregate reasoning-budget acceptance

The [user-selected effort contract](../../reference/inference/lfg-swag.md#user-selected-effort-and-run-budgets)
is mandatory across G-01a adapter execution, G-02a durable dispatch and G-05
context escalation. Implement one engine-owned aggregate run budget; retries,
fallbacks, reads and planning consume its remaining time/attempts. Extend the
existing installed limits and run/receipt authority, never a provider loop or
client timer. D-06 owns governed configuration changes; H-06 exposes finite
presets/custom limits, remaining budget, stop reasons and explicit extensions.

The falsifying corpus includes endless context requests, repeated invalid
output, fallback cycles, no progress, blocked/late providers, cancellation,
lost ACK/restart, clock rollback and concurrent budget reservations. No case
may reset consumption or silently extend the deadline. Accepted effects remain
durable and uncertain work remains visible. Unit/mock timers alone cannot
close this requirement; G-06 needs installed evidence with the chosen real
adapter. Existing configuration fields and examples do not implement it.
This preparation changes no gate status or dependency order.

### Budget-control delivery sequence

The user has prioritized usable budget controls. The existing embedding
deadline repair advances the current executable boundary without introducing
a reasoning loop before its storage/authorization prerequisites. Gate G stays
open. Each following row is a bounded implementation package within the owners
above, not an additional engine or independent roadmap.

| Order / owner | Code and required behavior | Falsifying acceptance history |
|---|---|---|
| 1 / G-01a | `rrd-inference::EmbeddingExecution` and `rrd-engine::generate_embeddings_at`: consume the sealed step ceiling, preserve worker admission after caller timeout, reject late output; common validation remains shared. | Block a real engine's admitted adapter, configure 250 ms, receive non-retryable deadline before release; deny another dispatch, then release and recover capacity. Expired requests invoke no backend; actual FeatureHash output/provenance equals synchronous execution. This proves a bounded wait, not a stopped provider. |
| 2 / G-01a, G-01b | Extend the existing adapter descriptor and execution owner with qualified cancellation/termination behavior and router dispatch. No detached replacement workers or implicit retry. | Cancel before dispatch and during work; a late result cannot advance the tree. A non-cooperative adapter cannot satisfy a hard stop profile. Real adapter cancellation evidence is required alongside controlled failure cases. |
| 3 / D-06, G-02a | Extend `EstateConfiguration`, kernel reasoning-tree/task identities and store semantic transactions with one durable resolved budget, original deadline, attempt reservations, usage and terminal reason. No client/provider-owned ledger. | Two workers contend for the final allowance; exactly one reserves it. Crash before dispatch, after response and after commit; replay never resets consumption or repeats an uncertain effect. Clock rollback denies further dispatch. |
| 4 / G-03, G-05 | All route/context/fallback attempts consume that same owner. Detect repeated semantic request/source/configuration identity with no accepted progress. Bound tracking state by the configured step/attempt cap. | An endless context requester, malformed-output controller and fallback cycle terminate at the configured limit. Changed correlation IDs cannot evade repetition; new authorized source evidence permits useful work within the remaining budget. |
| 5 / D-06, H-06 | Resolve editable finite Quick/Balanced/Thorough/Custom profiles through the same configuration and public catalogue; Connectome previews exact limits and displays remaining allowance, Stop, partial output and authorized extension. | A stale override or extension fails CAS; a client cannot raise estate caps. Reloading the UI or resuming work preserves original consumption. Unsupported cost/cancellation guarantees are visibly unavailable. |
| 6 / G-06 | Installed histories with one qualified real adapter, exact source/configuration/model identity and retained public decisions/receipts. | Repeat blocked/late/invalid/retry/concurrent/restart/no-progress cases through the shipped process. Measure actual last dispatch, output acceptance, usage and uncertain work. Candidate CI and installed evidence must pass before claiming loop prevention. |

The [research rationale](../../reference/inference/lfg-swag.md#research-decisions-for-budget-enforcement)
explains why timers, attempt limits, cancellation and durable accounting need
separate proof. No row is complete merely because its contract or unit fixture
exists. A per-batch deadline cannot close aggregate run accounting.

[PR 101](https://github.com/rrflow/rrflow-development/pull/101) advances row 2
with process-local cooperative cancellation through the existing embedding
owner and the actual bundled FeatureHash implementation. It does not qualify
router dispatch, remote provider cancellation, hard termination, a durable run
Stop operation or UI controls. The descriptor and wire catalogue are unchanged;
no adapter is advertised as forcibly stoppable because it implements a trait.

## Controller learning packages

The [learning contract](../../reference/inference/learning/controller-training.md)
owns dataset, experiment and artifact requirements under G-06. These packages
are open; offline preparation may run independently, but deployed execution
still follows G-01 through G-05 and the current dependency spine. Python tools
remain optional repository tooling or explicitly configured external
activities. They cannot become a default engine dependency.

| Package and prerequisite | Canonical boundary | First failure oracle and required exit |
|---|---|---|
| G-06a: offline dataset and baseline, before selecting a trained candidate | Public router/configuration families and first-party inference tooling under `scripts`; existing model manifests remain runtime authority | Reject unauthorized exports, duplicate leakage across project/time splits, absent labels and mismatched input/configuration digests. Reproduce simple policy/classifier and candidate results from one frozen corpus with counts, failures, total latency/cost and uncertainty. No Spark hardware is required for schema/golden preparation; actual training claims require actual hardware evidence. |
| G-06b: trained artifact qualification, after G-06a and the capability contract appropriate to the training target | Existing `rrd-contract` model manifest and `rrd-inference` pre-load admission; model/template packaging and real runtime adapter | Fresh-process reload, interrupted/resumed training, final quantization/template identity and held-out comparison pass. Existing route manifests reject endpoint-selector output; G-01b must first define that separate capability. Tampered model/tokenizer/runtime/grammar bytes never reach the loader. A LoRA sidecar is not implicitly admitted. |
| G-06c: installed candidate comparison, after G-06b and G-01 through G-05 | Existing installed engine, endpoint/configuration revisions and H-05 evidence | Repeat the candidate/baseline corpus through actual engine admission, with stale binding, denied sharing, unavailable model, timeout, cancellation and restart. Include controller overhead and downstream success. Shadow calls consume an authorized budget; comparison never mutates project state implicitly. I-03 owns later promotion. |

Preserve the current `router_contract`, `model_manifest_contract` and
`rrd-inference` `pipeline` corpora when their implementation changes. In
particular, the three-proposal golden, candidate/budget binding and rejection
before the model loader are required behavior, not scaffolding to discard.
G-01a already owns the existing registry-lock refactor; training packages must
not introduce a parallel executor to avoid it.

## Exit condition

Gate G exits only when configured LFG analysis (including disabled and
inconclusive states) and SWAG selection/dispatch pass their separate corpora.
Every selected model-backed profile needs its real pinned artifact/provider
evidence; deterministic LFG does not require a trained model. Routing must
steer a persisted tree without storage or planner authority, with qualified
budgets, explicit fallbacks and scoped cache behavior.
