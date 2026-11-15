# RRFlow 1.0 alpha objectives

**Status:** active measurable outcome set; no objective is complete
**Coordinate:** `rrflow://rrflow-instance/data/objective/rrflow-1.0-alpha`
**Owner:** first usable RRFlow alpha outcome

The RRFlow 1.0 version is frozen while this baseline is established. The
repository root [README](../../README.md) owns product identity and current
maturity. The [engine data-flow architecture](../architecture/engine-data-flow.md)
owns the detailed transactional and analytical flow, the
[roadmap](../roadmap/rrflow-1.0.md) owns delivery order, and the
[POA&M](../poam/rrflow-1.0-alpha.md) owns observed deficiencies. This record
owns only the measurable result.

## Objective

Deliver one self-contained, installable per-project RRFlow engram that can
drive development, maintenance, governance, reasoning, and recall through one
persistent engine. After its signed release bundle is acquired, the default
engram installs, starts, persists, recovers, and verifies without a source
checkout, sibling repository, package registry, external database/query/vector
service, or outbound network access. The engram must remain provider-neutral:
Codex, Claude, Gemini, Grok, LFG, and future callers consume the same public
operations, authorization, read stamps, context evidence, routines, and
skills.

The default operator entry point is the natively built `rrflow` executable on
Linux/macOS and `rrflow.exe` on Windows. That one command tree must plan and
apply project installation, serve the engine, prove authenticated readiness,
verify without mutation, plan and apply bounded offline repair, verify backup
and restore to an absent target, and remove only RRFlow-owned project
integration. A checkout supervisor, companion-binary build, source-tree
diagnostic, PID, port, marker, or compile result cannot satisfy this outcome.

rrflowDB owns relational tables, documents, AI knowledge, temporal
relationships and graph state, reasoning state, indexes, evidence, automation
state, and analytical storage. Its record identity is table/kind plus ID within
the engram. rrflowQL is the SQL-like relational query/planning layer; dynamic
graphs and traversal use that same data and authority. Application and
operator databases such as PostgreSQL, Turso, SQLite, Dragonfly, and hosted
services remain governed external sources unless an operator explicitly adds
an adapter. They never become an implicit RRFlow persistence backend.

rrflowDB also targets the [PostgreSQL core profile](../reference/protocol/postgresql.md)
over its own native storage. An external PostgreSQL connector does not satisfy
that compatibility outcome. H-08 supplies the ordered implementation and
real-client proof, including independent logical export, source-exit rehearsal,
and recovery without a vendor service. The owner also requires SQLite
compatibility. H-12 must first define and test the intended SQL semantics,
application API/ABI and file-format surfaces independently; migration alone is
not evidence of compatibility and none is assumed implemented.

The target combines durable rrflowKV/WAL and hot state with bounded
prompt-local working state, and background Parquet cold storage for selected
history, audit and analytics. Gate F owns the compaction, publication, query
and recovery work. An export or a volatile in-memory buffer does not establish
that lifecycle or durable acknowledgement.

## Observable engine objective

A user selects a real project, reviews the exact installation and permissions,
and receives its isolated RRFlow engram. The engine turns authorized source
revisions into retrievable records, source spans, semantic relationships and
lexical/vector access paths. A prompt retrieves current permitted evidence,
uses configured local or external inference only when admitted, and returns
inspectable results with bounded work and durable effect receipts. Editing or
deleting source invalidates dependent evidence; restart preserves accepted
state. Connectome displays that engine-owned state and its limitations.

This journey binds the existing objectives, rather than replacing them:

| User-visible result | Required objective owners | Observable proof |
|---|---|---|
| Install and choose a project | OBJ-01/02/03/07/09/14 | Actual installed binary; explicit plan/apply; project identity; permission denial; exact acknowledged readback after reopen. |
| Turn project source into knowledge | OBJ-04/05/07/14 | Committed inventory and parser/model identities; source spans; graph and vector records generated from that source; incremental edit/delete and exclusions. |
| Ask a bounded question using that knowledge | OBJ-06/09/11 | One authorized stamped retrieval, citations checked against source, selected/skipped avenue reasons, declared inference and actual usage; explicit abstention or exhaustion when evidence/budget is insufficient. |
| Inspect, tune and repeat | OBJ-07/10/11/15 | Effective settings with origins and hard limits; a retained baseline; repeatable read-only inspection; separately authorized new execution; local work and provider usage reported separately. |
| Add optional capabilities safely | OBJ-08/09/12/13 | Typed knowledge-path/learning/model bindings, explicit activation, held-out qualification and revocation; unavailable integrations shown as unavailable. |
| Operate and distribute reliably | OBJ-02/03/07/10 | Native faults, growth, resource bounds, fixed-profile sustained qualification and signed clean-machine install/recovery evidence. |

The first working local-project checkpoint is an intermediate engineering
milestone, **not the fifteen-objective alpha exit**. The existing alpha exit
below remains unchanged, including SQL compatibility, learning and TurboQuant
requirements. Naming a checkpoint must never silently narrow that scope.

The owner-directed sequence is to make that developer build usable first, then
benchmark and tune the actual project-to-context and reasoning flow. A later
research and implementation target is to derive configurable reasoning shards
from boundaries in large projects and use a custom KV cache where experiments
show it helps. The benchmark work may begin as soon as a usable candidate exists;
it does not declare the full alpha exit or waive C-07. Boundary-derived shards
and their cache are not prerequisites for the first usable developer build, and
are not established capabilities. The [scale rationale](../architecture/engine-scale-and-contracts.md#boundary-based-reasoning-after-the-developer-build)
defines what must be measured before stronger claims or the next stage.

Claims must be falsifiable. Successful retrieval is not proof that an answer is
correct; deterministic parser relations differ from model-inferred relations;
a model confidence score is not a fact. Every inferred claim retains source,
method and verification disposition. Hidden chain-of-thought is neither a
required artifact nor a correctness oracle. Observable decisions, tool calls,
returned evidence and validated effects are the inspectable record.

Provider input/output/cached tokens, measured local-model tokens and local
retrieval work are separate quantities. No tokenizer or usage receipt means
unknown usage, not zero. Cache entries and graph visits are not "reasoning
tokens". Token savings compare a declared baseline at matched answer quality
and include ingestion, embedding, index maintenance and readback costs; reduced
context alone does not prove reduced total cost or improved quality.

## Required outcomes

An outcome is `complete` only after its end-to-end proof passes at an exact
revision. `Partial` means useful implementation exists but the named proof has
not passed. A type, fixture, mock, compile, or document alone is not proof.

| ID | Outcome | Current state | Completion evidence | Roadmap gates |
|---|---|---|---|---|
| OBJ-01 | One authority and vocabulary | Partial: `RrdEngine`, grouped crates, and canonical terms exist; documentation classification, implementation-requirements traceability, and the final boundary audit remain open. | Repository policy proves one identity/status owner, one objective owner, one roadmap, one POA&M, inward dependencies, complete current-source/test/fixture traceability into the accepted boundaries, and no competing graph, memory, query, lifecycle, or provider authority. | A |
| OBJ-02 | One hybrid rrflowKV persistence engine | Partial: accepted C-06 now supplies one exclusive segment-v6 reader with an ordered key/version spine, six Arrow-layout buffers, authenticated row-group filters, authenticated none/adaptive-LZ4 policy, owned projected generations, bounded selective output, exact mixed-family MVCC/reopen/compaction behavior, finite adversarial qualification, and a process-local exact-byte scan-resistant cache. The clean C-06j production-reader differential preserves durable and semantic identity while reducing post-scan hot-page loads from 48 to zero; value separation, family partitioning, Moka, TinyLFU, and caller cache authority are rejected for this format. C-07 installed-path lifetime, recovery, maintenance, and storage-full qualification and Gate J distribution proof remain open. | Atomic crash/reopen corpus proves WAL → MVCC memtable → immutable key/version spine plus Arrow-compatible column pages → manifest publication with one backend and one accepted physical reader. | C, J |
| OBJ-03 | Equivalent volatile and persistent semantics | Partial: rrflowMX and rrflowKV share one transaction port and C-03 semantic plan; rrflowKV recovers the exact multi-family write closure all-or-none; C-04 proves equivalent authenticated, budgeted direct semantic-version reads without normal runtime-log reconstruction, including vector artifact and quantization catalogues at the search request stamp and key budget; the remaining maintenance replay and final closure evidence keep POAM-028/C-04 open; C-05 removes the accepted pre-1.0/alternate execution shapes; and accepted C-06 preserves exact rrflowKV snapshot semantics through the bounded physical stream, filters, compression, and both cache policies. Final installed-path C-07 recovery/lifetime qualification and the complete native graph/index/query corpus remain open. | The same transaction, snapshot, graph, index, and query corpus passes for rrflowMX and rrflowKV; rrflowKV additionally survives every recovery boundary. | C |
| OBJ-04 | Atomic multi-model records, graph, and indexes | Partial: C-03 atomically commits records, temporal graph state in both directions, scalar/unique entries, BM25/vector source deltas, runtime state, projection work, receipts, audit, and outbox. Incremental BM25 postings, native vector generations, bounded reads, and exact-oracle access evidence remain E work. | One transaction atomically changes records, both graph directions, scalar/unique state, BM25 postings, vectors, index deltas, and the runtime log; exact oracles remain equal after reopen. | C, E |
| OBJ-05 | Real Arrow/DataFusion and Parquet analytical execution | Partial: DataFusion executes stamped Arrow batches allocated from materialized `QueryRow` values. Segment v6 can borrow eligible raw mmap pages or own bounded decoded pages, and C-06g exposes a bounded synchronous projected storage stream with selective page-family and read/borrow/copy/decode/decompression/allocation evidence. No stamped DataFusion provider consumes it yet. A bounded Parquet query export exists; no background cold-storage lifecycle is qualified. | A stamped provider streams bounded batches, pushes eligible work down, proves conditional zero-copy with read/borrow/copy/decode/decompression/allocation counters, and enforces memory, spill, scan, time, candidate and output limits. Background cold compaction publishes and recovers selected Parquet history under the same transaction/read authority, with exact hot/cold results, bounded resources and no acknowledged-write loss. | C, F |
| OBJ-06 | Dynamic governed reasoning and recall | Partial: bounded lexical, exact-vector, graph, fusion, context evidence, reasoning-tree, and router contracts exist; native indexes, persisted tree execution, adaptive routing, and LFG dispatch do not. | One authorized request chooses fast or analytical work, retrieves through eligible graph/BM25/vector paths, fuses deterministically, advances a persisted tree through CAS, and reproduces its evidence at the same stamp after reopen. | E, G, H |
| OBJ-07 | Install, operate, repair, and attune a new or existing project | Partial: the source-built primary `rrflow` command now performs deterministic no-write plan, exact-digest apply, installed serve, authenticated readiness, and read-only quick verification for fresh or existing local projects. Installation owns one sealed engram/configuration, persists a pending attunement plan/job, recovers the same plan at ten typed durable-effect stages plus the pre-intent lease boundary without duplicating authority or secrets, and is the only product startup path used by CLI, internal server, embedded MCP, six-language SDK conformance, deployment catalogue, and the Kubernetes renderer. The pre-canonical manifest/private binding, raw product commands, standalone security initializer, development supervisor, and server initializer are absent. D-01 now has [accepted native installation evidence](../roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21) for source/outbound isolation, owner/ACL/mount denial and actual fresh/existing lifecycle. The active missing workflow is durable attunement execution, committed project inventory, incremental parsing and source-grounded lexical/graph/vector retrieval, with global defaults and explicit per-project effective configuration. Installation and data lenses do not establish that workflow. Configuration apply, full verification, complete repair/restore/uninstall, signed offline bundles, wider storage recovery/lifetime, and qualified process/Kubernetes adapters remain open. | From one verified platform bundle and its single primary executable, plan/apply, authenticated readiness, deterministic tree snapshot, interrupt/resume, commit/query, close/reopen, read-only quick/full verify, torn-tail repair plan/apply, backup verification, restore to an absent target, and ownership-safe uninstall pass with outbound network denied and sibling repositories absent for empty and existing projects. A real project also proves effective-setting origins, source spans, actual embedding identity, query results, edit/delete/reindex, restart and two-project isolation. The committed snapshot proves exact exclusions, errors, incremental changes, and bounded work; no credentials, caches, generated output, application databases, or provider lifecycle files become canonical engine state. | B, C, D, J |
| OBJ-08 | Adaptive routines, triggers, skills, and adapters | Partial foundation: synchronous governed functions and transaction bindings exist. An unused standalone context-maintenance state-machine draft also exists but is not an accepted routine implementation because it directly owns storage, scope, events, and replay. Canonical engine events, persisted event triggers, generic resumable routines, skill packages, explicit host-event adapters, and capability activation do not. | Generic packages are versioned, digest-bound, authorized, budgeted, observable, restartable, removable, and activated only from verified engram signals and policy; error resolution and context projection maintenance execute through the same routine authority with no operation-specific repository. | I |
| OBJ-09 | Provider-neutral clients and relational compatibility | Partial: HTTP, current WebSocket behavior, SDKs, CLI/MCP, and a separate Connectome bootstrap exist at different conformance levels; the PostgreSQL core profile is specified but unimplemented and unqualified. A SQLite compatibility profile is not yet defined or qualified. | Supported clients use the same authority. H-08's SQL/type/transaction/wire/client matrix passes against a pinned PostgreSQL reference; H-12 defines and proves the admitted SQLite surfaces against a pinned reference, separately identifying SQL, application API/ABI and file-format coverage. Unsupported behavior is explicit. | B, H |
| OBJ-10 | Reproducible competitive evidence | Not established: local component evidence exists, but no complete fixed-hardware engine or clean-rollout corpus proves the alpha. | Published raw results cover correctness, p50/p95/p99 latency, throughput, memory, copied/decoded/allocated bytes, disk/WAL/compaction, recall@k, filtered quality, restart, failure behavior, and clean-machine deployment steps/time/bytes/services against pinned declared baselines, including SurrealDB and Qdrant where the compared capability exists. | J |
| OBJ-11 | Visible, extensible reasoning and learning evidence | Not established: Connectome lenses, H-05 signals, and the feedback and learning contracts define parts of this evidence, but no public operation returns a request's retrieval hits with their delivered and used disposition, usage and savings against a declared baseline, or learning results. No versioned extension model for operator-defined views or metrics exists, and rrflow.ai and rroflow.ai are planned destinations, not deployed services. | For one installed engram, engine-owned typed records and public operations expose four views: (1) retrieval hits for each request, meaning the returned candidates, the context actually delivered, what the caller used or cited, and each avenue's contribution; (2) usage and savings, meaning requests, latency, cache hit/miss/unsupported, and delivered context size against a declared baseline method, with unknown savings shown as unknown; (3) learning results, meaning candidate, evaluation, promotion, and rollback history; and (4) the project's stored records, graph, and knowledge sources. Local Connectome, an operator-controlled self-hosted deployment, and, when explicitly linked, rrflow.ai for the reasoning engine and rroflow.ai for learning show the same identities and values at the same read stamp. Operators configure and extend views and metrics through versioned, authorized, removable definitions over the public operation catalogue; an extension cannot read outside its grant or change engine records. | G, H, I |
| OBJ-12 | Governed supervised, unsupervised, and recursive learning (RROFLOW) | Not established: the [staged learning contract](../reference/inference/learning/controller-training.md#staged-learning-and-feedback) and the G-06, I-03, and I-07 packages are specified; no learning implementation exists. | In one installed engram, supervised candidates from independently checked labels or preferences, unsupervised candidates from authorized clustering or representation analysis, and bounded recursive I-03 improvement remain distinct proposals. Held-out comparison against simple baselines, denied-data exclusion, bounded iterations and spend, restart, competing promotions, and rollback pass, and a failed candidate never replaces the active revision. Operators configure learning methods and can add one as a versioned, authorized candidate generator or evaluator that produces proposals only. Learning runs locally without an account; a linked rroflow.ai receives only separately authorized datasets and artifacts. | G, H, I |
| OBJ-13 | Complete TurboQuant | Partial: the deterministic MSE TurboQuant codec and its explicit build/activate/retire lifecycle exist with bounded 512-vector evidence; its catalogue uses authenticated stamped direct reads, while incremental serving and the remaining C-04 source-closure work are still open ([POAM-028](../poam/rrflow-1.0-alpha/poam-028.md)). The inner-product estimator builds and serves through the same lifecycle (E-04d), with its qualification owned by J-04. There is no model KV-cache format or quantized transfer profile. | The four parts in the [TurboQuant scope](../reference/vector/quantization-lifecycle.md#required-turboquant-scope) pass with canonical full-precision vectors and exact reranking authoritative: incremental persistent serving through E-04 and the stamped Arrow pipeline through F-03; the inner-product variant measured against the MSE variant and exact search on one corpus; KV-cache format selection only for an admitted model runtime that qualifies it; and a selected-data transfer profile whose quantized-only replicas never present approximate hits as exact. J-04 qualifies recall, memory, and latency claims. | E, F, G, H, J |
| OBJ-14 | Provable plan-to-effect flow | Partial: installation, endpoint-configuration, restore and tail-repair plans are versioned and digest-bound, refuse changed inputs before any write, and survive real process kills and reopen with exactly-once replay. Attunement does not re-check its source binding before acting ([POAM-038](../poam/rrflow-1.0-alpha/poam-038.md)); no handoff or open-question record exists ([POAM-039](../poam/rrflow-1.0-alpha/poam-039.md)); no public operation lists or reads jobs, job transitions or receipts, and a memory read leaves no durable receipt ([POAM-040](../poam/rrflow-1.0-alpha/poam-040.md)); applied-result checks derive from the plan they verify, and the change, interrupt and restart tests stop at installation and storage ([POAM-041](../poam/rrflow-1.0-alpha/poam-041.md)). | For one installed engram and one real project, effect-bearing work (attunement phases, a project mutation, and a memory write and read) runs from a versioned plan bound to the project's source revision where it has one, its inventory digest, configuration, authorization and budgets, and is handed between two actors as an exact, digest-checked record. Each planned effect names the open questions it depends on: the engine refuses that effect while one is open, unrelated exploration and effects continue, and an answer is an authorized, audited operation that requires an explicit re-plan when it changes the plan. Before every mutation the engine re-reads the bound sources and refuses a stale plan with no write. Catalogue operations list, read and steer background jobs, and every job transition and memory write or read leaves a durable receipt readable after restart. Tests deliberately change a source, a configuration and an answer between plan and apply, kill the shipped `rrflow` at every job transition, restart and resume. An independent checker derives the expected effects from the approved plan alone and compares them with the observed artifacts and the read-back audit trail; the engine does not grade its own run. | D, H, I, J |
| OBJ-15 | Configurable, measurable engram organisation | Partial: the schema registry holds versioned domain and category definitions, the membership of every kind and vector collection, and kind tags, validated at the public boundary and in the kernel, persisted on rrflowMX and rrflowKV, readable after reopen, and encoded so default-organisation digests are unchanged (H-11a). Record tags, scoped reads, isolation, domain grants and per-domain evidence do not exist yet, so every read is still engram-wide ([POAM-042](../poam/rrflow-1.0-alpha/poam-042.md)). | For one installed engram, the schema registry's organisation revision assigns every kind and vector collection to exactly one domain. The bundled default works with no setup, and an attunement proposal from the project tree takes effect only as an authorized revision. Categories and tags label kinds and records under engine-validated vocabularies, with record tags kept as bi-temporal relations that carry provenance. rrflowQL, context assembly, vector search, catalogue reads, diagnostics and changefeed reads accept a domain and tag scope that the engine applies before generating candidates, and every result names its scope and organisation revision. An isolated domain refuses every read, traversal, retrieval, changefeed and boundary-crossing relation without a grant naming it, while shared domains keep atomic commits and reasoning across domains. Per-domain evidence reports size, work, latency by stage, returned, delivered and used context, denials and truncations, with unmeasured values shown as unknown. Moving a kind changes no record identity, and old read stamps read the old organisation. Every result is identical on rrflowMX and rrflowKV after close and reopen, and Connectome and the PostgreSQL schema projection show the same organisation at the same read stamp. An organisation change is promoted as better only when a measured experiment shows it for a named domain and question type. | D, H |

OBJ-11 through OBJ-13 record the repository owner's direction of 2026-10-24.
They bind work to existing gates and leave the
[executable dependency spine](../roadmap/rrflow-1.0/executable-dependency-spine.md)
order unchanged.

OBJ-14 records the repository owner's direction of 2026-10-26. Proving the
intended flow needs a versioned, source-bound plan and handoff; explicit
decisions about which unanswered questions block an effect; observable
background-job and memory receipts; stale-plan detection before mutations; and
tests that change inputs, interrupt work, restart, and compare the
implementation and audit trail with the approved plan. The owner decided that
open questions are typed engine records that block only the effects that name
them, and that the missing planned job list, read, status, resume, cancel,
receipt-read and job-transition journal operations may be added to the
operation catalogue under existing grants. That answers the
[D-02a](../roadmap/rrflow-1.0/gate-d.md#project-workflow-build-packages)
admission question for job control; global configuration operations remain
undecided. Research may inform these designs, but a schema alone proves neither
compliance nor research quality. OBJ-14 also binds existing gates and leaves
the spine order unchanged.

OBJ-15 records the repository owner's direction of 2026-10-27. Domains inside
a project engram are configurable: RRFlow ships a default setup that each
project tunes, groups kinds into domains, lets projects create categories and
tags, can isolate data, and drills down to find bottlenecks and what is
working well, following current research on reasoning engines.
[ADR-0003](../decisions/0003-configurable-engram-organisation.md) records the
decision and its research basis, and the
[engram organisation contract](../reference/data/engram-organisation.md) owns
the terms. OBJ-15 adds roadmap item H-11 without moving any existing item: its
contract package H-11a precedes D-05, which assigns ingested kinds to domains,
and H-11b to H-11e sit beside H-06, which projects them.

## Alpha exit

The baseline exists only when all fifteen objectives are complete and their linked
roadmap gates carry reproducible evidence. Optimization continues within the
same frozen version until the repository owner makes an explicit release and
version decision.
