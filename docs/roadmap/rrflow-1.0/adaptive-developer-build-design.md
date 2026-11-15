# Adaptive project execution and the developer build

**Status:** historical; superseded on 2026-11-13 by the [kernel contract](../kernel-contract.md), which carries every package listed here; kept as the record of the work and decisions it describes
**Superseded by:** [RRFlow kernel contract](../kernel-contract.md)
**Coordinate:** `rrflow://rrflow-instance/data/design/adaptive-developer-build`
**Owner:** [Gate D project-workflow build packages](gate-d.md#project-workflow-build-packages)
**Baseline:** `2cadd3a9045e3c1a2d9f873fe4215343680ab18b`

The [alpha objective](../../objectives/rrflow-1.0-alpha.md),
[roadmap](../rrflow-1.0.md), and [POA&M](../../poam/rrflow-1.0-alpha.md)
own outcomes, order, and gaps. This design makes a proposed replacement
reviewable; it cannot change their status or authorize an implementation PR.
[ADR-0002](../../decisions/0002-adaptive-reasoning-governed-effects.md) owns
the boundary between adaptive exploration and governed effects.

The [installed project-indexing implementation plan](project-indexing-execution-plan.md)
maps the first source-to-query proof to current code, package oracles, native
worker requirements, configurable retrieval and independent readback.

## Problem at the inspected baseline

Both bundled installation profiles enumerate Connect, Inventory, Parse,
Normalize, EntityLink, LexicalIndex, Embed, VectorIndex, Graph, Ground, and
Verify in the same order. If no explicit proposal is supplied,
`RrdEngine` copies the profile sequence into the installation plan. A caller
can supply a different ordered proposal, so the exact eleven entries are not
mandatory for every submitted plan. However, `AttunementPhase` is a closed
enum, plans require a linear sequence ending in Verify, and checkpoint
validation chains each input to the immediately preceding output. Neither
form selects work from discovered project needs and installed capabilities.

Connect and Inventory have engine execution paths. The installed dispatcher
returns `phase-unavailable` for the other phases. The embedded replan preserves
the existing phase list and changes its source identity; it only accepts
specific pre-effect states and requires reconciliation for other cases. It
does not yet offer task-dependent replanning after arbitrary authorized work.
The existing inventory tests demonstrate useful commit, interruption, and
reopen behavior; they do not demonstrate an end-to-end prompt or developer
handoff. The unmerged Parse draft in PR 241 is not main behavior.

At the original baseline, Connectome pin
`38142b77110a8f1db77e306e7d1ad375fd918267` defined eleven static phase
views and a fixed 30–45 minute estimate in `src/rrflow/attunement.ts`.
Main subsequently advanced its pin to
`c1cbff203ecee89013878609d11aed04e89d0e71` in PR 239; that file is unchanged
and retains both declarations. A Git search of both revisions found the two
constants only at their declarations and found no imports of that module;
this does not prove that a visible screen uses them. The current pin's native
`src-tauri/src/rrd/client.rs` does contain an authenticated SDK session and
query path, while `engram.rs` implements preview and exact-digest installation.
Those source paths are implementation inventory, not a real desktop acceptance
result. The wider pinned UI integration remains unaudited.

This is a product gap, not a finding that every fixed constant or explicit
verification requirement is bad. Protocol versions, resource ceilings and
fail-closed validation can be fixed for a documented reason. A global fixed
sequence is not evidence of adaptive work selection.

## Proposed runtime boundary

```text
project and task -> exploratory discovery / prompt-specific context
                 -> engine proposal -> reviewed, versioned effect plan
                 -> authorized capability execution -> receipts + readback
                 -> current knowledge and next prompt / successor plan
Connectome and other clients <---- public engine operations ----> RrdEngine
                                     |              |
                              rrflowQL / records   rrflowKV transaction
```

Exploratory reading and model reasoning stay adaptive and do not require a
complete plan. A durable effect is admitted through `RrdEngine`, with an
explicit operation, effective configuration, authorization, relevant inputs,
budgets, and a verifiable result. A capability is an executable, versioned
engine-owned contract, not a free-form instruction or permission to execute
unknown code. Discovery of a tool or model never activates it. Existing
endpoint and product-capability projections must derive from their executable
owner; the implementation must not create a second capability registry.

The planner selects only available capabilities needed for a task and its
declared dependencies. It records why an avenue is selected, unavailable,
or skipped. Multiple valid plans, including no embedding provider and no
external tools, must be possible. Model-generated suggestions remain
proposals until validated; do not encode hidden chain-of-thought. A fixed
execution order is justified only where an actual effect dependency requires
it. Concurrency is optional and must never be claimed until its lease,
publication and cancellation behavior is proven.

### Handoff contract and admission

The following are required semantic fields for a versioned handoff; their
wire names and canonical encoding must be chosen in the implementation PR and
generated from the owning contract, not copied from this prose:

| Group | Required binding |
|---|---|
| Identity | Engram, project, plan ID/version/digest, actor and successor lineage; explicit effective configuration and capability revisions. |
| Evidence | Relevant source identities and content/revision digests, selected retrieval evidence and read stamp, plus exclusions and uncertainty relevant to the effect. |
| Effect | Capability ID/version, typed parameters, intended resource and effect, required grants, hard budgets, idempotency key, dependencies, and questions that block that effect. |
| Verification | Observable acceptance predicates, independent checker and permitted evidence source; named inputs and outputs, not a claim that the model complied. |
| Progress | Attempt identity, transitions, receipts, failure/uncertain outcome, and memory reads or writes, all retrievable after restart. |

Schema validation checks structure. Semantic validation resolves capability
identity, authorization, source observations, budgets, questions and plan
digest immediately before a governed effect. An unsupported capability,
changed relevant input, changed required answer or exhausted limit refuses
that effect without publishing it. Unrelated exploration and independent
effects remain possible. A schema cannot prevent edits by an external actor
with direct filesystem access; RRFlow must detect and reconcile such edits.

For an external effect, a lost acknowledgement is **uncertain**, not an
automatic safe retry. Retry only when the provider's idempotency or an
independent reconciliation proves it safe. Local canonical publication must
atomically bind its mutation, job transition and receipt where the owning
transaction supports that guarantee. A trace is diagnostic and may be sampled;
it is not the durable audit record.

### Changed source and replan

Bind each effect to the exact inputs on which it depends, including absence,
directory membership or configuration when those facts matter. A single
file hash is insufficient for a claim that depends on a directory or query
range. Re-observe the necessary boundary immediately before publication;
preserve containment and race protections. If the read set is incomplete,
claim only the weaker guarantee or conservatively refresh the relevant
boundary. Do not silently treat an unknown dependency as unaffected.

An ordinary edit creates a successor proposal linked to the old plan and its
receipts. Preserve already committed effects and reuse derived work only when
their inputs, capability version and authorization are still valid. A
relevant changed input invalidates dependent work; an unrelated edit must not
block an independent effect. Changed authorization, configuration or an
answered question is handled the same way when it changes that effect. The
operator can inspect why a replan is required; no old receipt is rewritten.

## Existing code to preserve or replace

| Existing source and characterization | Destination and required evidence |
|---|---|
| `rrd-engine/assets/install/{default,clyffy}-profile-v3.json`; `installation.rs` proposal validation, profile fallback, sealed preview | Replace fixed default work selection with capability- and project-derived proposals at the existing engine boundary. Preserve no-write preview, exact apply and installed identity. Test multiple different projects/configurations, inactive providers, and unknown capability denial. |
| `rrd-contract/src/attunement.rs`: `AttunementPhase`, `AttunementPlan`, checkpoint and job validators; `tests/attunement_contract.rs` | Replace the phase-specific planning assumption with versioned capability/effect records and dependency-bound checkpoints. Preserve public operation identities where feasible and document exact contract changes; do not silently reinterpret old digests. The implementation PR must decide and prove what happens to existing pre-release engrams and persisted jobs without adding an alternate runtime authority. |
| `rrd-engine/src/engine/attunement.rs`: `control_attunement`, `make_attunement_replan`, lease/receipt/replay | Generalize selected execution and successor lineage while retaining authorization rechecks, fencing, idempotency, cancellation and original-principal receipt readback. Test stale workers and post-effect replan, including uncertain external outcomes. |
| `attunement/inventory.rs`, `inventory/{records,publication,reuse}.rs`, installation inventory; engine `tests/attunement.rs` and `tests/attunement/inventory_delta.rs` | Preserve actual bounded anchored reads, source recheck, incremental reuse, publication groups and committed head. Prove no premature head/receipt on failure and exact state after replay; do not replace with an in-memory index. |
| Draft PR 241 Parse implementation and `rrd-attunement` | Reconcile its source binding and parser tests into the chosen capability contract. Before activation prove process isolation, memory/time/output enforcement, cancellation/reaping and stale-source publication on each claimed native platform. Its whole-inventory read bound is not 100,000-record evidence. |
| `rrd-contract::endpoint_catalogue`, `rrd-engine::product_capability_catalogue`, server dispatch, generated SDKs | Use one executable owner for new operations and schema projections. Verify actual HTTP/SDK dispatch and permissions; a catalogue row alone is not execution. |
| `rrd-query` graph traversal and store transaction paths | Retrieve the same persisted project/source identity through rrflowQL and public readback after reopen. PR 242 merged as `b8e615baab4f7033a3dead4506d7d5d9dc5ce24c` after the original design baseline. Its quoted-identity repair does not by itself prove the complete source-to-query project journey. |
| Connectome pinned `src/rrflow/attunement.ts`, native `src-tauri/src/rrd/{client,engram}.rs`, `docs/reference/client/connectome.md`, H-06 | Reconcile the static phase/estimate declarations with the engine projection; inspect remaining callers at the implementation revision. Use the existing authenticated SDK session, generated/public operations and readback; UI cannot own plans, source state or claim verified work from health responses. Test the exact pinned desktop. |

Before deleting or replacing a path, its implementation PR must complete the
repository's traceability map, characterize existing persisted formats and
callers, and prove equal-or-stronger replacement. This design does not approve
dropping existing installed data or introducing a parallel fallback engine.

## Implementation handoff

Each row is a reviewable change at its owning boundary, not a mandatory
runtime stage. The first implementation brief must state the exact files,
symbols, selected compatibility decision, failure oracle and dependency PRs
after refreshing main. Existing roadmap sequencing and candidate CI still
apply; reconcile an overlap before starting concurrent work.

| Change | Concrete implementation and first-failure oracle |
|---|---|
| Plan and effect admission | Characterize the current profile fallback and persisted v2 plan/job/checkpoint fixtures. Extend the existing executable owner with typed capability identities, parameter validation, selected dependencies and plan digest; generate projections. Replace the fixed default in both profiles. Reject unknown effects, duplicate IDs, unresolved dependencies, cycles and altered digests. Show two distinct valid selected plans and an unavailable provider without manufacturing an executor. Exact handling of existing v2 engrams must be decided and tested before any removal. |
| Durable execution and replan | Map plan nodes to the existing engine control, lease and transaction authority. Bind a receipt to plan, actor, operation, input observations and outcome. Preserve stale-worker fencing and historical receipts; reconcile committed effects before successor acceptance. Demonstrate that relevant changes refuse mutation, unrelated changes do not block independent work, and lost acknowledgements do not duplicate effects. |
| Source to query | Reconcile PR 241 against the resulting plan contract. Use the committed inventory identity and authorized source reader, enforce the parser worker's native resource limits, and publish source spans/relations through the existing transaction. Query the persisted source through rrflowQL after reopen; test changed/deleted files, cancellation, stale input and worker failure. |
| Developer handoff and UI | Implement handoff, question and receipt operations at the engine catalogue, generate SDK surfaces, and wire the exact pinned Connectome client through those operations. Drive a real permitted change and independent readback/checker through the desktop; show pending and unavailable states honestly. |

Existing characterization commands for the relevant baseline, to rerun on
the selected implementation revision before changing it, are:

```text
cargo test --locked -p rrd-contract --test attunement_contract
cargo test --locked -p rrd-engine --lib inventory_checkpoint_and_source_records_commit_reopen_and_replan_together
cargo test --locked -p rrd-engine --lib git_selected_plan_ignores_irrelevant_edits_replans_tracked_edits_and_reopens
cargo test --locked -p rrd-engine --lib replan_rechecks_inputs_and_fences_the_old_worker
cargo test --locked -p rrd-engine --lib replan_lost_response_survives_process_kill_and_replays
cargo test --locked -p rrflow-cli --test installed_lifecycle -- --nocapture
```

These existing tests characterize portions of the old model; passing them is
not acceptance of the replacement. Add targeted failing tests for each new
oracle before implementing it, then run the owning package suites, generated
surface checks and the repository's complete candidate and merge-group CI.
The desktop acceptance test must launch a real UI and engine; its exact command
depends on the pinned client audit and must be specified in that UI PR. Keep
the C-07 qualification campaign tied to its own exact integrated revision,
binary, configuration and retained native evidence.

## Developer-build acceptance oracle

The first full loop runs real source-built binaries and a real Connectome
desktop build against a project fixture and then an independently chosen
project. The receipt records source revisions, exact binary and UI digests,
configuration, platform, commands, outcomes, and unavailable features.

1. Preview and explicitly apply the project boundary; verify denied source
   and inactive provider behavior. Restart and authenticate the installed
   engram. The UI displays identities and actual readiness.
2. Commit a bounded inventory and parse an authorized source. Query the same
   source ID and span through rrflowQL and public engine readback; the UI shows
   the same result and provenance. No fixture-loaded substitute is accepted.
3. Submit two materially different tasks/configurations and show different
   selected capabilities and handoffs. Inspect why optional work was skipped.
4. Hand the digest-bound plan to another actor, apply one authorized project
   change, and have an independent checker compare actual files, tests and
   engine receipts to the approved predicates. Record memory writes and reads.
5. Change a relevant source, configuration and answer before their dependent
   effects; each is denied without mutation and can be replanned. Change an
   unrelated file and demonstrate an independent effect remains possible.
6. Kill and restart at admission, publication, acknowledgement and replay
   boundaries; test duplicate request, cancellation, expired lease, worker
   death and lost external acknowledgement. Compare exact durable readback.
7. Exercise two project engrams for isolation, selected Windows and Linux
   native builds, and the declared ARM target when a native runner is available.
   Record a missing platform as unqualified. macOS remains required where the
   owning release matrix requires it; this checkpoint does not waive it.

This is an intermediate developer-build proof, not alpha completion. Native
C-07 capacity, storage-full, recovery and 24-hour soak evidence, real provider
adapters, dynamic retrieval quality, PostgreSQL profile, Parquet and the rest
of the owning objective remain separate required work.

## Research and decisions to resolve

- SurrealKV [commit ring](https://github.com/surrealdb/surrealkv/blob/3e5e5147543db50ff4aa09e63fb09b93b07a5a3f/src/ring/commit_ring.rs)
  and [WAL crash tests](https://github.com/surrealdb/surrealkv/blob/3e5e5147543db50ff4aa09e63fb09b93b07a5a3f/src/test/wal_group_crash_tests.rs)
  motivate explicit publication and recovery invariants. Compare algorithms
  and failure semantics with rrflowKV before adaptation; do not substitute a
  second storage authority. SurrealKV and SurrealDB core have different
  licenses; confirm the exact file and revision before copying code.
- Turso's [PostgreSQL compatibility record](https://github.com/tursodatabase/turso/blob/8ba16a6cb96d30f549f9b3731cb0727867857908/postgres/COMPAT.md)
  separates grammar, translation, execution and wire semantics; its frontend
  is experimental and documents unsupported or incomplete behavior. Its
  [differential simulator](https://github.com/tursodatabase/turso/blob/8ba16a6cb96d30f549f9b3731cb0727867857908/testing/simulator/runner/differential.rs)
  motivates pinned reference comparisons. Do not assume its parser satisfies
  RRFlow's pure-Rust requirement or that syntax acceptance proves semantics.
- Pinecone describes [curation, task-specific knowledge and evaluation](https://www.pinecone.io/blog/introducing-nexus-knowledge-engine/).
  Treat this as a design reference, not an RRFlow performance result. Evaluate
  persisted artifacts and per-prompt retrieval against held-out project tasks.
- [OpenTelemetry GenAI conventions](https://github.com/open-telemetry/semantic-conventions-genai/blob/main/docs/gen-ai/gen-ai-spans.md)
  inform interoperable diagnostic spans, with content capture explicitly
  controlled. Durable receipts remain engine records even when traces are sampled.

Implementation admission still needs an exact contract/compatibility decision
for existing installed v2 plans and jobs, a capability-catalogue extension that
does not duplicate the existing owner, a safe native parser worker design,
and a real Connectome pinned-revision audit. Define those in the relevant PR
brief before editing code. A documented limit must state its owner, rationale,
configuration surface and failure behavior; no unmeasured value is a capacity
claim. Revisit planner concurrency and index strategy only with measured workload
and failure evidence.
