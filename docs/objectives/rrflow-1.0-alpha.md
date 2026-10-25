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

Deliver one self-contained, installable per-project RRFlow estate that can
drive development, maintenance, governance, reasoning, and recall through one
persistent engine. After its signed release bundle is acquired, the default
estate installs, starts, persists, recovers, and verifies without a source
checkout, sibling repository, package registry, external database/query/vector
service, or outbound network access. The estate must remain provider-neutral:
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

rrflowDB owns AI knowledge, temporal graph state, reasoning state,
indexes, evidence, automation state, and analytical storage. Application and
operator databases such as PostgreSQL, Turso, SQLite, Dragonfly, and hosted
services remain governed external sources unless an operator explicitly adds
an adapter. They never become an implicit RRFlow persistence backend.

rrflowDB also targets the [PostgreSQL core profile](../reference/protocol/postgresql.md)
over its own native storage. An external PostgreSQL connector does not satisfy
that compatibility outcome. H-08 supplies the ordered implementation and
real-client proof, including independent logical export, source-exit rehearsal,
and recovery without a vendor service.

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
| OBJ-05 | Real Arrow/DataFusion analytical execution | Partial: DataFusion executes stamped Arrow batches allocated from materialized `QueryRow` values. rrflowKV segment v6 can borrow eligible raw mmap pages or own bounded aligned decoded pages, persists row-group filters for point reads, and C-06g exposes a bounded synchronous projected storage stream with selective page-family and per-operation read/borrow/copy/decode/decompression/allocation evidence. Neither the filter nor compression connects storage to DataFusion by itself. No stamped DataFusion `TableProvider`/`ExecutionPlan` consumes that stream or emits eligible borrowed `RecordBatch` buffers yet. | A stamped provider streams bounded batches from rrflowKV, pushes eligible work down, proves conditional zero-copy with read/borrow/copy/decode/decompression/allocation counters, and enforces memory, spill, scan, time, candidate, and output limits. | F |
| OBJ-06 | Dynamic governed reasoning and recall | Partial: bounded lexical, exact-vector, graph, fusion, context evidence, reasoning-tree, and router contracts exist; native indexes, persisted tree execution, adaptive routing, and LFG dispatch do not. | One authorized request chooses fast or analytical work, retrieves through eligible graph/BM25/vector paths, fuses deterministically, advances a persisted tree through CAS, and reproduces its evidence at the same stamp after reopen. | E, G, H |
| OBJ-07 | Install, operate, repair, and attune a new or existing project | Partial: the source-built primary `rrflow` command now performs deterministic no-write plan, exact-digest apply, installed serve, authenticated readiness, and read-only quick verification for fresh or existing local projects. Installation owns one sealed estate/configuration, persists a pending attunement plan/job, recovers the same plan at ten typed durable-effect stages plus the pre-intent lease boundary without duplicating authority or secrets, and is the only product startup path used by CLI, internal server, embedded MCP, six-language SDK conformance, deployment catalogue, and the Kubernetes renderer. The pre-canonical manifest/private binding, raw product commands, standalone security initializer, development supervisor, and server initializer are absent. D-01 now has [accepted native installation evidence](../roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21) for source/outbound isolation, owner/ACL/mount denial and actual fresh/existing lifecycle. Attunement phase execution and committed project inventory, configuration apply, full verification, complete repair/restore/uninstall, signed offline bundles, wider storage recovery/lifetime, and qualified process/Kubernetes adapters remain open. | From one verified platform bundle and its single primary executable, plan/apply, authenticated readiness, deterministic tree snapshot, interrupt/resume, commit/query, close/reopen, read-only quick/full verify, torn-tail repair plan/apply, backup verification, restore to an absent target, and ownership-safe uninstall pass with outbound network denied and sibling repositories absent for empty and existing projects. The committed snapshot proves exact exclusions, errors, incremental changes, and bounded work; no credentials, caches, generated output, application databases, or provider lifecycle files become canonical engine state. | B, C, D, J |
| OBJ-08 | Adaptive routines, triggers, skills, and adapters | Partial foundation: synchronous governed functions and transaction bindings exist. An unused standalone context-maintenance state-machine draft also exists but is not an accepted routine implementation because it directly owns storage, scope, events, and replay. Canonical engine events, persisted event triggers, generic resumable routines, skill packages, explicit host-event adapters, and capability activation do not. | Generic packages are versioned, digest-bound, authorized, budgeted, observable, restartable, removable, and activated only from verified estate signals and policy; error resolution and context projection maintenance execute through the same routine authority with no operation-specific repository. | I |
| OBJ-09 | Provider-neutral client operation and PostgreSQL compatibility | Partial: HTTP, current WebSocket behavior, SDKs, CLI/MCP, and a separate Connectome bootstrap exist at different levels of conformance; the PostgreSQL core profile is specified but unimplemented and unqualified. | Every supported client and model adapter produces equivalent operation, authorization, stamp, digest, denial, trace, and restart results without embedding engine logic. H-08's required SQL/type/transaction/wire/client matrix passes against a pinned PostgreSQL reference and installed native servers, with independent source-exit/export/recovery proof and explicit unsupported-feature denials. | B, H |
| OBJ-10 | Reproducible competitive evidence | Not established: local component evidence exists, but no complete fixed-hardware engine or clean-rollout corpus proves the alpha. | Published raw results cover correctness, p50/p95/p99 latency, throughput, memory, copied/decoded/allocated bytes, disk/WAL/compaction, recall@k, filtered quality, restart, failure behavior, and clean-machine deployment steps/time/bytes/services against pinned declared baselines, including SurrealDB and Qdrant where the compared capability exists. | J |
| OBJ-11 | Visible, extensible reasoning and learning evidence | Not established: Connectome lenses, H-05 signals, and the feedback and learning contracts define parts of this evidence, but no public operation returns a request's retrieval hits with their delivered and used disposition, usage and savings against a declared baseline, or learning results. No versioned extension model for operator-defined views or metrics exists, and rrflow.ai and rroflow.ai are planned destinations, not deployed services. | For one installed estate, engine-owned typed records and public operations expose four views: (1) retrieval hits for each request, meaning the returned candidates, the context actually delivered, what the caller used or cited, and each avenue's contribution; (2) usage and savings, meaning requests, latency, cache hit/miss/unsupported, and delivered context size against a declared baseline method, with unknown savings shown as unknown; (3) learning results, meaning candidate, evaluation, promotion, and rollback history; and (4) the project's stored records, graph, and knowledge sources. Local Connectome, an operator-controlled self-hosted deployment, and, when explicitly linked, rrflow.ai for the reasoning engine and rroflow.ai for learning show the same identities and values at the same read stamp. Operators configure and extend views and metrics through versioned, authorized, removable definitions over the public operation catalogue; an extension cannot read outside its grant or change engine records. | G, H, I |
| OBJ-12 | Governed supervised, unsupervised, and recursive learning (RROFLOW) | Not established: the [staged learning contract](../reference/inference/learning/controller-training.md#staged-learning-and-feedback) and the G-06, I-03, and I-07 packages are specified; no learning implementation exists. | In one installed estate, supervised candidates from independently checked labels or preferences, unsupervised candidates from authorized clustering or representation analysis, and bounded recursive I-03 improvement remain distinct proposals. Held-out comparison against simple baselines, denied-data exclusion, bounded iterations and spend, restart, competing promotions, and rollback pass, and a failed candidate never replaces the active revision. Operators configure learning methods and can add one as a versioned, authorized candidate generator or evaluator that produces proposals only. Learning runs locally without an account; a linked rroflow.ai receives only separately authorized datasets and artifacts. | G, H, I |
| OBJ-13 | Complete TurboQuant | Partial: the deterministic MSE TurboQuant codec and its explicit build/activate/retire lifecycle exist with bounded 512-vector evidence, but its catalogue is rebuilt from cursor zero for every search ([POAM-028](../poam/rrflow-1.0-alpha/poam-028.md)). There is no inner-product variant, model KV-cache format, or quantized transfer profile. | The four parts in the [TurboQuant scope](../reference/vector/quantization-lifecycle.md#required-turboquant-scope) pass with canonical full-precision vectors and exact reranking authoritative: incremental persistent serving through E-04 and the stamped Arrow pipeline through F-03; the inner-product variant measured against the MSE variant and exact search on one corpus; KV-cache format selection only for an admitted model runtime that qualifies it; and a selected-data transfer profile whose quantized-only replicas never present approximate hits as exact. J-04 qualifies recall, memory, and latency claims. | E, F, G, H, J |

OBJ-11 through OBJ-13 record the repository owner's direction of 2026-10-24.
They bind work to existing gates and leave the
[executable dependency spine](../roadmap/rrflow-1.0/executable-dependency-spine.md)
order unchanged.

## Alpha exit

The baseline exists only when all thirteen objectives are complete and their linked
roadmap gates carry reproducible evidence. Optimization continues within the
same frozen version until the repository owner makes an explicit release and
version decision.
