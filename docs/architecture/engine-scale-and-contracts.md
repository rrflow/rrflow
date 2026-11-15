# RRFlow scale and contract design rationale

**Status:** active researched target design; implementation and capacity unqualified
**Coordinate:** `rrflow://rrflow-instance/data/architecture/engine-scale-and-contracts`
**Owner:** researched tradeoffs supporting existing schema, execution, signal and qualification owners
**Reviewed:** 2026-10-25

Keep the native engine foundation and repair or complete its canonical paths.
Do not replace rrflowKV with a vendor database, copy an upstream engine tree,
or mistake existing Rust types for an implemented reasoning service.
This record explains decisions; the [dependency spine](../roadmap/rrflow-1.0/executable-dependency-spine.md)
still owns order, [Gate G](../roadmap/rrflow-1.0/gate-g.md#bounded-reasoning-implementation-packages)
owns reasoning packages, and [Gate J](../roadmap/rrflow-1.0/gate-j.md#capacity-and-optimization-qualification)
owns capacity experiments and release evidence. No gate is completed here.

## Code-grounded starting point

Original design inspection revision (2026-10-20): `554ce4ccf1cb25aceba036dafaf4c8df79c42701`.
These are bounded design inputs, not a replacement source inventory:

| Boundary inspected | Preserve | Repair or complete, with existing owner |
|---|---|---|
| `rrd-engine/src/engine/invocation.rs` and PR 32 | Per-admission ownership, authorization and audited completion | The process-local reservation is not a durable reasoning lease or resume checkpoint. G-02/G-04 and I-03 must bind durable work separately. |
| `rrd-engine/src/engine/inference.rs::generate_embeddings_at`; `rrd-inference/src/lib.rs::EmbeddingBackendRegistry::embed_batch` | Model provenance, input/output validation, explicit network policy | The inspected baseline held the registry mutex through backend work. The current single-slot admission repair below removes that coupling; configurable concurrency, cancellation and complete resource reclamation remain G-01/G-06 and POAM-009 work. |
| `rrd-contract/src/router.rs`; kernel/public `reasoning_tree.rs` | Existing descriptor, request, three-decision and cursor contracts | Admission and schema validation exist; executable router dispatch and persisted tree execution remain G work. `TaskRecord` open/close is not a reasoning worker. |
| `rrd-query/src/fusion.rs::RrdSnapshotTableProvider`; `arrow.rs` | Stamped execution, honest unsupported filters and existing budget evidence | Replace eager Arrow snapshots with the accepted storage stream; account for allocations before and after DataFusion. F-01/F-02/F-04; POAM-004. |
| `rrd-core/src/schema.rs`; public data and generated surfaces | One explicit logical table map, atomic schema/data mutation, generated projections | Add admitted semantic contracts and versioned validation at the owning boundary, not handwritten competing API schemas. H-04/H-08 and the schema catalogue. |
| `rrflow-eval/src/main.rs::Trial` and existing component evidence | Paired provider trials and retained failure evidence | The current trial record is not the complete engine workload manifest or saturation harness. J-02/J-04; POAM-012. |

The installed lifecycle now has native process evidence. Full installed
durability, inventory, indexes, streamed analytics, routing, SQL compatibility
and sync still require their named acceptance tests. Fix demonstrated failures
in place; replace an implementation only after mapping its behavior and tests
to the canonical replacement. In particular, retain accepted segment v6,
the shared transaction port and the measured cache policy.

## Next executable cut

The 2026-10-25 audit at `337d136210ff59b2359f898e717166c087dd5bfc`
found that the prior storage-first paragraph contradicted the corrected roadmap.
Follow the [current dependency spine](../roadmap/rrflow-1.0/executable-dependency-spine.md)
and [Gate D build packages](../roadmap/rrflow-1.0/gate-d.md#project-workflow-build-packages).
The installed lifecycle has accepted bounded evidence; real-project ingestion is
not complete. Public attunement read/control and replan handlers now exist, but
the complete background executor, selected parser publication and usable
project-to-retrieval flow remain unproven. The owner has directed implementation
to resume; each durable effect still needs its own admitted authorization and
evidence. See [POAM-006](../poam/rrflow-1.0-alpha/poam-006.md) and the current
Gate D package records for the exact remaining gaps.

Preserve existing native growth repairs and frozen campaign evidence. C-07
still requires its complete native/profile/repetition/24-hour qualification;
a failed stage cannot be relabeled and a source change cannot inherit an older
binary's results. Qualification no longer postpones missing project ingestion.
The existing [local database example](../reference/sdk/rrflowdb-local.md)
exercises records/edges/queries/reopen; it is not source indexing, vector quality,
model reasoning, PostgreSQL compatibility or cloud synchronization proof.

### Boundary-based reasoning after the developer build

The owner intends RRFlow to work with large real projects such as
platform_devpulse and RRFlow itself. The near-term deliverable remains the
working developer build: install and open an existing project, index permitted
source, retrieve source-grounded context, and inspect progress and results in
Connectome. Existing storage and cache work needed to make that journey work
continues. After a usable candidate exists, establish repeatable baselines and
tune it; final alpha, C-07 and release qualification retain their own exits.

A subsequent target is to derive configurable **reasoning shards** from
observed or explicitly configured project boundaries: for example, a module,
service or other meaningful responsibility. A shard is a candidate unit for
selecting and organizing reasoning context, not automatically a physical
rrflowKV shard, a distributed database, or a new authorization authority.
Boundaries can overlap or change, and relationships across them remain
retrievable under the same `RrdEngine` authorization and read identity. The
engine must be able to represent uncertainty and user corrections rather than
assuming a folder name or manifest fully describes a project.

The intended custom KV-cache strategy must be specified and evaluated at its
actual boundary. rrflowKV stores durable project state and has physical cache
policies; an ephemeral request context is assembled from authorized, stamped
state. A model's attention KV cache is a different, model/runtime-specific
object. If model-cache reuse is pursued, it must bind the model, tokenizer,
prefix, configuration and scope supported by that runtime; storage-cache hits
must not be reported as model-token savings. No cache design, compatibility or
performance result is claimed here.

For each proposed shard/cache strategy, retain source and configuration
identity, cross-boundary dependencies, exclusions and update provenance. An
ordinary edit invalidates affected derived material and permits a bounded
successor/replan under the previously authorized scope. Compare against the
working baseline on representative large-project questions, cross-boundary
changes and no-answer cases, at matched correctness and resource budgets.
Report answer/patch verification, retrieval quality and freshness, latency,
CPU/memory/I/O, cache behavior and actual model-token/cost data when available,
including construction and maintenance costs. Unknown usage stays unknown.
An improvement to one metric alone is not evidence that the reasoning engine
is competitive; advance to the next stage when the measured evidence shows
sufficient promise for the stated workload and quality criteria.

[GitHub's dependency-graph documentation](https://docs.github.com/en/code-security/concepts/supply-chain-security/dependency-graph-data)
illustrates that manifest-based dependency discovery has a defined scope, not a
complete semantic boundary map. [Tree-sitter's incremental parsing documentation](https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html)
requires applying the exact edit before reusing a syntax tree. [vLLM's prefix-cache design](https://docs.vllm.ai/projects/vllm-omni/en/v0.32.0/design/feature/prefix_caching/)
illustrates model-specific prefix reuse, not RRFlow cache compatibility. These
are research inputs, not implementation or comparative evidence.

### Evidence-based build decisions

The following primary sources were checked online on 2026-10-25. These are
bounded design inputs, not proof RRFlow already implements or outperforms them.
The chosen scope and dependency order are RRFlow engineering decisions.

| Evidence and limitation | Decision at the existing owner | Alternative and falsifying experiment |
|---|---|---|
| [Google SRE pipeline design](https://sre.google/workbook/data-processing/) describes idempotent mutations, validation before publication and checkpointing. | D-02/D-05 use durable progress and validated engine-owned publication. | Restart-from-zero work and unrelated per-store writes are rejected for durable jobs. Duplicate delivery and crash between output/ACK must not duplicate accepted effects or mark incomplete output complete. |
| [PostgreSQL WAL](https://www.postgresql.org/docs/18/wal-intro.html) explains durable log-before-data ordering and redo. It does not specify RRFlow's exact physical format. | Preserve rrflowKV's one durable transaction authority and C-07 recovery proof. | Reject a second graph/vector durability owner. Native crash/readback of one semantic mutation proves all accepted families agree; a surviving row alone is insufficient. |
| [Qdrant hybrid queries](https://qdrant.tech/documentation/search/hybrid-queries/) combine distinct ranked retrieval results. A documented algorithm does not establish code-search quality. | E/H qualify lexical, vector and graph contributions independently before combined ranking. | Compare lexical-only, exact-vector-only and combined retrieval at equal corpus, authorization, token/context limits and freshness. Keep the simpler path when extra work does not improve the declared quality/cost result. |
| [BEIR](https://arxiv.org/abs/2104.08663v4) evaluates heterogeneous retrieval and finds strong lexical baselines alongside model tradeoffs. Its tasks do not qualify RRFlow source-code understanding. | J-04/G-06 require held-out task classes and a lexical baseline, not one favorable prompt. | Evaluate symbols, cross-file dependencies, natural-language intent, changed/deleted source and no-answer cases on independently labeled project questions. Report per-class quality and cost instead of an average that hides regressions. |
| [GraphRAG](https://arxiv.org/abs/2404.16130v2) reports gains for global sensemaking over document corpora. It is not evidence that every local code query benefits from graph summaries. | D-05/H-01 treat inferred summaries/relationships as source-linked candidates and retain direct source facts. | Test graph summaries only after direct retrieval works; compare global questions separately from exact symbol lookup, include build/update cost and source-delete freshness, and do not promote unsupported relationships as facts. |

No source algorithm or dependency is adopted by this planning correction.
Existing HydraDB/SurrealDB/Pinecone references below remain design comparisons;
each actual knowledge-path adapter needs a pinned product/API, authorization,
provenance and live-service proof at D-06. A similarly named graph product and
hosted context service are not interchangeable contracts.

Telemetry references also need provenance: the old OpenTelemetry GenAI web
pages now redirect readers to the [GenAI conventions repository](https://github.com/open-telemetry/semantic-conventions-genai).
H-05 must pin the supported convention revision and reconcile it with RRFlow's
existing signal catalogue; this audit does not import changing field names.
Separate observed provider/model usage from tokenizer estimates and local
storage work, as required by the [objective](../objectives/rrflow-1.0-alpha.md#observable-engine-objective).
No new public metric or token-savings claim is established here.

### Turnkey capability comparison and proof obligations

Official product documentation was checked on 2026-10-25. "Fold in their
capabilities" means implement selected behavior at RRFlow's canonical boundary
and qualify it, or expose an explicitly admitted external knowledge path. It
does not mean copy another engine, inherit its benchmarks or claim all-feature
parity. The following comparison defines relevant obligations, not a status
ledger; current findings are in the linked package evidence.

| Reference capability | RRFlow requirement and existing owner | Proof needed before claiming the capability |
|---|---|---|
| [Pinecone integrated embedding](https://docs.pinecone.io/guides/search/integrated-embedding) and [namespace isolation](https://docs.pinecone.io/guides/index-data/implement-multitenancy) | D-01/D-05 automatically bind each project engram and admitted embedding space; D-06 optionally reads a Pinecone namespace. | Real source text produces model-bound vectors; edits/deletes invalidate them; filtered queries cannot cross project grants. A vector dimension or manually seeded vector does not prove semantic indexing. |
| [SurrealDB relations](https://surrealdb.com/docs/reference/query-language/statements/relate), [indexes](https://surrealdb.com/docs/reference/query-language/statements/define/indexes) and [transactions](https://surrealdb.com/docs/reference/query-language/language-primitives/transactions) | C/E/H preserve one transaction for canonical facts, relations and index deltas; rrflowQL combines authorized evidence. | Record/edge/vector updates are atomically visible, exact traversal and current-source retrieval survive crash/reopen, and failed mutations publish no partial semantic state. PostgreSQL compatibility has its separate H-08 proof. |
| [Qdrant hybrid queries](https://qdrant.tech/documentation/search/hybrid-queries/) and [multitenancy](https://qdrant.tech/documentation/manage-data/multitenancy/) | E-03/E-04/E-05/H-01 supply lexical/vector retrieval, eligible filtering, exact reranking and tenant-aware execution. | Compare each path and fusion at matched quality and budgets; qualify filtered recall, deletion freshness, index update/rebuild and resource cost. External Qdrant partitioning does not replace RRFlow's project engram/authorization design. |
| [Apache Parquet](https://parquet.apache.org/docs/overview/) | D-06/F typed interchange through a locked bundled Rust codec; H-06 operator controls. | Decode actual exports and compare typed authorized values after restart. Independently qualify import and bounded streaming; a linked codec alone proves neither. |

For scale comparisons, freeze product versions, source/model/corpus, filters,
hardware, offered load and quality targets. Report indexing, update/delete,
query, maintenance, recovery and total costs separately. Vendor docs establish
what to investigate; J-04 measured results alone support RRFlow performance
claims. Qdrant's shared-collection/tenant approach and Pinecone namespaces are
reference alternatives, not authorization to combine project context implicitly.

The [Connectome journey](../reference/client/connectome.md#turnkey-project-entry-and-progressive-detail)
requires minimum upfront decisions and progressive detail; the
[effort inheritance contract](../reference/inference/lfg-swag.md#inheriting-effort-from-the-calling-ai)
keeps inherited preferences explicit and bounded; the
[rroflow.ai handoff](../reference/client/devpulse-cloud.md#turnkey-rroflowai-handoff)
keeps account, data transfer, learning and activation independently provable.
All three remain subject to the existing D/G/H/I admission decisions and source
truth. No provider subscription, learned controller or cloud endpoint is assumed
available merely because its integration is planned.

## One engine across desktop and service

```mermaid
flowchart LR
    Clients["DevPulse / Connectome / SDK / PostgreSQL client"] --> Contract["Versioned public operation and input contract"]
    Contract --> Engine["RrdEngine: identity, policy, admission, stamp, budgets"]
    Engine --> Reason["Reasoning: bounded proposals and durable effects"]
    Engine --> Query["rrflowQL: native operators / DataFusion"]
    Reason --> Query
    Reason --> Adapter["Explicit model or tool adapter"]
    Query --> DB["rrflowDB: schema, records, relations, receipts"]
    Reason --> DB
    DB --> KV["rrflowKV: atomic persistent transactions"]
    DB --> MX["rrflowMX: equivalent volatile semantics"]
    Engine --> Signals["Bounded diagnostic projections"]
    Engine --> Transfer["Optional authorized sync adapter"]
    Transfer --> Peer["Another independently installed RRD instance"]
```

This is the target interaction graph, not a claim that every arrow is live.
The [topology](instance-topology.md) keeps one project/engram/instance identity.
Scale a service fleet by placing independently authorized engrams on more
capacity first. A control plane may place, authenticate and monitor engrams;
it does not become their transaction authority. Private project VMs and
containers remain outside the database process.

A large engram first gets measured vertical headroom, native indexes, bounded
streaming and admission control. Intra-engram replication/sharding follows the
[distributed contract](../reference/distributed/cluster-contract.md), with
quorum, membership, fencing, placement and recovery qualification. Client
sync is not consensus replication. No cluster or HA claim follows from
running several single-node binaries.

Qdrant documents a useful small/large-tenant placement tradeoff, but its
shared-collection model is not RRFlow's engram topology. Use noisy-neighbor
and movement tests as reference behavior; do not import its collection or
shard authority. [Qdrant multitenancy](https://qdrant.tech/documentation/manage-data/multitenancy/)
supports that comparison, not an RRFlow performance claim.

## Bounded execution before more concurrency

Admission must reserve execution slots and byte budgets before allocating
large inputs or launching a provider. Keep separate bounded classes for
interactive recall, model work, ingestion/index maintenance and transfer.
Reserve operational capacity for cancellation/readiness/recovery; a full model
queue must not prevent observing or stopping the engine.

The budget is hierarchical: host/service allocation, engram, request, stage
and backend. A child may reduce a budget, never expand it. Queue waiting counts
against the request deadline. Measure queue wait, actual execution, lock wait,
model time and durable commit independently. Schedule fairly across engrams
and requests; do not confuse DataFusion operator fairness with tenant fairness.

For model adapters, take an immutable descriptor/handle under a short registry
lock, then execute outside it. A backend that needs exclusive mutable state
gets its own bounded worker or permit. A per-backend worker still belongs to
`RrdEngine` admission; it has no independent durable job state. Avoid replacing
one global mutex with unbounded spawned work. Preserve descriptor revision and
credential revocation checks across queueing.

The [embedding admission repair](../evidence/change-journals/gate-g/g-01a-embedding-admission-repair.md)
applies that separation to the existing synchronous path. It preserves the
previous one-batch execution ceiling and rejects contention immediately;
listing and adapter installation use only the short catalogue lock. Selection
pins the admitted descriptor and registry revision, descriptor drift rejects
results, and an owned permit remains held through execution and validation.
There are no spawned workers, waiting queue or claimed cancellation. The
failure corpus uses controlled channel ordering and simultaneous admission,
not a throughput benchmark or full deterministic-simulation system. This is a
bounded POAM-009 repair; G-01a must still qualify configurable parallelism,
deadlines, revocation, cancellation and late output before raising the ceiling.
Rust's [mutex semantics](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
explain why a backend panic is confined to its own adapter lock; poisoning is
advisory and is not a substitute for descriptor/result validation.

Tokio explicitly documents that a started blocking task cannot be aborted by
dropping/aborting its handle. Therefore a timeout may stop waiting without
stopping work. Keep permits and memory reservations until termination is
confirmed; use cooperative cancellation or an explicitly supervised,
bundle-contained worker process where hard interruption is necessary.
Uncertain completion is visible and cannot authorize another effect.
[Tokio shared state](https://tokio.rs/tokio/tutorial/shared-state) and
[Tokio 1.53.1 blocking work](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html)
are the reference semantics.

Overload rejects or sheds work before expensive execution, with bounded
retry hints and a declared retry budget. Invalid input, denied permission and
permanent digest conflict are not retryable. Reconnect/retry must retain the
logical operation identity. Do not remove transaction serialization merely to
improve a benchmark: first measure contention, then prove any replacement
preserves atomic publication and acknowledged-write recovery.
[Google SRE overload guidance](https://sre.google/sre-book/handling-overload/)
supports bounded queues and explicit rejection.

## Durable reasoning without a schema of thought

The [schema catalogue](../reference/data/schema-catalogue.md#structure-and-input-contract-plan)
owns the structural design. Governed reasoning records describe exchanged
inputs, public proposals, verified observations and accepted effects.
Exploratory thoughts and arbitrary model-native search remain outside this
durable schema, as required by [ADR-0002](../decisions/0002-adaptive-reasoning-governed-effects.md).

At the durable boundary, bind the relevant read stamp, source/schema revision,
principal/seat/policy, model/recipe digest, input digest, budget and expected
cursor. Persist a bounded dispatch intent before an external effect. Validate
the response against both its structural schema and current authorization;
commit the accepted outcome, evidence references, cursor, audit and outbox
atomically. A stale worker cannot commit after its lease/fence is superseded.

Recovery distinguishes intent recorded, execution in progress, accepted result,
denied result, cancelled work and uncertain external outcome. These are
required semantic cases, not a second hand-authored enum replacing existing
job/tree/routine types. Resolve the exact type changes in the owning package.
A durable result can be consumed again without rerunning the provider.
Missing provider evidence cannot be relabeled successful or blindly replayed
when duplication would create effects or charges.

Deterministic replay means reusing the accepted public result and its receipt,
not claiming that a model generates identical text twice. Record model/runtime,
sampling configuration and seed where supported, then measure residual output
variance in the quality corpus. A retained digest is not a recoverable result:
authorized bounded result content or a verified content reference must exist
where resume requires it.

The model may select only the existing admitted decision forms; the engine
selects storage/index plans and evaluates deterministic conditions. Grammar
constraints reduce malformed output but do not establish truth, authorization,
source freshness or permission to mutate.
[llama.cpp grammar guidance](https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md)
is a reference for constrained decoding; no runtime or model is selected by
this planning package.

## Storage, analytical and multimodal optimization

Finish the installed C-07 failure/lifetime proof and remaining C-04 closure
evidence; the search-path vector-catalogue replay repair is already recorded
under [POAM-028](../poam/rrflow-1.0-alpha/poam-028.md). Then E supplies incremental graph, scalar,
BM25 and vector access. F consumes those paths at one stamp through bounded
Arrow batches. Preserve an exact unoptimized oracle for every access path;
measure ANN recall, filter selectivity and freshness rather than treating ANN
results as exact.

Planning a DataFusion scan should build a plan, not read the whole engram.
Execute through a bounded stream; declare each pushed predicate exact,
inexact or unsupported and retain residual filters as needed. Enforce input,
output, staging, pinned-page, decoding and spill reservations as well as
DataFusion pool memory. Cancellation must release generations after consumers
drop their buffers. Reuse buffers only with proven ownership.
[DataFusion providers](https://datafusion.apache.org/library-user-guide/custom-table-providers.html)
and [memory-pool behavior](https://docs.rs/datafusion/latest/datafusion/execution/memory_pool/struct.FairSpillPool.html)
inform F; pool admission alone is not a process-RSS guarantee.

Multimodal inputs carry media type, content digest, byte size, source revision,
extractor/model identity and declared limits. Admission constrains expanded
images, audio duration, decompression and decoded tensors before allocation.
Do not inline unbounded assets into route packets or duplicate asset bytes in
each graph edge. Source/object references and provenance survive derived
projection rebuilds. Every advertised modality needs a real model/extractor
fixture, quality oracle and hostile-input/resource case.

Optimization order is evidence-driven: bound work; remove whole-history and
whole-result reconstruction; add selective native access; stream; measure
contention; then tune batching/cache/compression/quantization on the same
correctness corpus. A change that improves latency by omitting authorization,
durability, failure samples or recall is rejected.

## Graph, vector and knowledge research mapped to RRFlow

The 2026-10-24 research refresh adds the following decisions to the existing
owners. These are design references; no upstream source or benchmark is imported.

| Primary reference | Existing owner and implementation consequence | Required counterexample |
|---|---|---|
| [FoundationDB SIGMOD 2021](https://www.foundationdb.org/files/fdb-paper.pdf), deterministic simulation and fault testing | C-07 keeps independent acknowledged-history oracles and native crash/disk/resource testing alongside deterministic fault coverage. Simulation does not replace the installed binary. | Lost response, interrupted publication, restart and replay must resolve the original operation once; a green isolated test is insufficient. |
| [Linux cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html), hierarchical resource enforcement | C-07d/g binds the actual host/process allocation and resource events to workload receipts. Application reservations and measured RSS remain distinct evidence. | Reclaim pressure is not itself an OOM. Denied allocation, stalled maintenance or retained owners beyond declared drain/budget must fail the corresponding operating-point oracle; record the actual resource events. |
| [Qdrant hybrid queries](https://qdrant.tech/documentation/search/hybrid-queries/), prefetch/fusion/reranking | E/F/G evaluate lexical, dense and graph-assisted retrieval at one authorized source/read identity; preserve exact and filtered quality baselines. | Selective filters, source deletion, stale embeddings and inconsistent cursor joins cannot be hidden by higher unfiltered recall. |
| [Hydra hosted-memory report](https://benchmarks.hydradb.com/hydradb), versioned relational memory, context enrichment and temporal retrieval | D/G and knowledge construction preserve source time, provenance and revision through derived relationships. This hosted report is distinct from the open-source graph design below. | Updates, contradictory facts, revoked sources and temporal questions need retained expected answers. Vendor-reported scores are not independent validation or RRFlow results. |
| [Pinecone Nexus](https://www.pinecone.io/product/nexus/), manifests and typed cited answers | Knowledge construction and G consume explicit entity/relationship requirements and bounded evidence; J-04 includes ingestion, maintenance, invalidation and inference cost. | An apparently cheap answer that omits preprocessing/update cost, cites stale evidence or crosses project authorization is not an optimization. |
| [SurrealDB transactions](https://surrealdb.com/docs/learn/querying/concepts-and-guides/transactions), grouped atomic effects | Preserve the existing RRFlow schema/record/relation commit authority and independently verify readback through rrflowQL over rrflowKV. | A failed member must not leave a partial record, relationship or receipt; reads combined across commits must not masquerade as one snapshot. |

Measure provider-reported input/output/cache usage separately from local query
work, storage I/O, embedding work and maintenance. Local database computation
is not an invented count of "reasoning tokens". Unknown usage remains unknown.
Comparisons must hold corpus, authorization, freshness and answer quality
constant and include ongoing cost; J-04 owns the measured performance claim.

Primary sources reviewed on 2026-10-20 support the following engineering
choices. Vendor descriptions and benchmarks are design inputs, not measured
RRFlow capability. The existing E/F/D/G/J gates own implementation and proof;
this mapping neither starts a second roadmap nor changes the dependency spine.

| Reference behavior | Canonical RRFlow application | Failure and qualification requirement |
|---|---|---|
| [HydraDB architecture](https://github.com/hydra-db/hydradb/blob/main/architecture.md): pinned snapshot; immutable topology plus bounded visible write overlay; shared request-local adjacency; canonical fallback | E-01 graph access and E-05 planning reuse temporal state within one authenticated `ReadStamp`. Any future compiled adjacency remains a disposable projection of rrflowKV versions. | Reordered identity histories, retirement, moved endpoints and expired versions must match chronological replay. Stale/missing projection cannot silently omit recent edges. Keep RRFlow storage/transaction authority; do not import SlateDB, object-store leases, GraphBLAS or HydraDB's cell topology. |
| [SurrealDB index documentation](https://surrealdb.com/docs/reference/query-language/statements/define/indexes): tunable HNSW, separate graph/vector cache costs, disk-backed ANN tradeoffs, explain metrics | E-04 compares native exact/HNSW and later disk/quantized candidates; E-05 reports the actually selected access path. F-04 accounts for graph, vectors, candidates, decoded pages and output together. | A bounded vector cache does not bound a resident HNSW graph. Compare filtered recall, exact rerank, update/delete, reopen, build amplification and total memory at equal quality before changing defaults. No SurrealDB source or defaults are copied. |
| [Pinecone slab architecture](https://www.pinecone.io/learn/slab-architecture/): immutable partitions plus recent writes, metadata prefiltering, storage/cache hierarchy | E-04's native generation plus exact delta and E-05's selectivity-aware planning; retain C-06's accepted segment/cache implementation. | Bind vector identity, collection/named vector, source cursor and filter eligibility at the same stamp. Measure selective-filter quality and delete freshness; post-filtering top-k alone can underfill results. No new slab backend, bitmap codec or distributed router is selected here. |
| [Pinecone Nexus GA](https://www.pinecone.io/blog/pinecone-nexus-generally-available/): incrementally compiled, domain-scoped knowledge with typed answers and citations | D-03/D-04 supply code/source identity; D-05 owns derived knowledge; G-03/G-04 consume admitted context and evidence. Existing connector schemas govern optional external acquisition. | Changed/deleted sources invalidate dependent evidence, edges and embeddings. Retain source digest, extractor/model/recipe identity, valid time, conflict evidence and authorization. Reuse verified derived artifacts without treating generated summaries or model confidence as ground truth. Nexus is a knowledge product, distinct from Pinecone Database's storage design. |

[HydraDB's integration entry point](https://hydradb.com/ai) explicitly separates
its open-source graph engine from its hosted context API. Evaluate connector
readiness and context quality independently from the graph engine's published
architecture. An integration guide or successful HTTP call is not a benchmark
or a qualified RRFlow connector.

The relationship is one logical database over one native transactional store:

```mermaid
flowchart TD
    Q["rrflowQL: stamped query and explain"] --> R["rrflowDB: canonical records, relations, evidence"]
    Q --> G["Temporal adjacency access"]
    Q --> V["Vector candidates and exact rerank"]
    R --> K["rrflowKV: atomic versions and index deltas"]
    G --> K
    V --> K
    S["Authorized source revision and content digest"] --> R
    R --> D["Rebuildable lexical, vector and knowledge projections"]
    D --> G
    D --> V
```

This describes the intended full integration; the objective/POA&M owners retain
its partial status. Semantic graph edges express relationships with evidence.
HNSW links are an ANN acceleration structure and cannot become those semantic
relationships. Vector dimensions, distance, named collection/vector, model and
source identity belong to the existing typed contracts; client configuration
cannot silently reinterpret vectors or authorize cross-project retrieval.

The [incremental graph reduction repair](../evidence/change-journals/gate-e/e-01-incremental-graph-reduction.md)
applies request-local reuse to the existing rrflowQL traversal. It replaces
per-depth replay with one cursor-aware reducer shared by expansion and row
projection. It retains an expired/retired identity's cursor to prevent an older
version arriving in a later read step from reviving it. It does not add a
persistent index, compiled graph, vector algorithm or cross-request cache.
Storage still accumulates authenticated selected history, row projection still
materializes results, and path construction remains depth-sensitive. The
mutation-visit test measures this reduction only, not total query time or RSS.

## SQL compatibility and optional synchronization

Use the existing [H-08 package DAG](../roadmap/rrflow-1.0/gate-h.md#h-08-postgresql-compatibility-packages).
Wire framing, SQL types, transaction isolation, catalogue discovery and client
behavior each need differential evidence against a pinned PostgreSQL 18
reference. Native snapshot isolation does not establish PostgreSQL READ
COMMITTED statement behavior or serializability. Unsupported cases fail
explicitly before effects. [Protocol message flow](https://www.postgresql.org/docs/18/protocol-flow.html)
and [transaction isolation](https://www.postgresql.org/docs/18/transaction-iso.html)
are the semantic references; PostgreSQL is a test input, not the database
under RRFlow.

Use the existing [H-07 sync packages](../roadmap/rrflow-1.0/gate-h.md#h-07-devpulse-managed-cloud-and-homelab-packages):
atomic import plus origin receipt, snapshot/cursor handoff, then bidirectional
reconciliation. Preserve explicit base-version conflicts, delete retention,
schema compatibility, revocation and offline operation. Relational uniqueness,
authorization and arbitrary effects cannot be made safe by declaring every
field a CRDT. [Local-first research](https://www.inkandswitch.com/essay/local-first/)
motivates user-owned local data; it is not proof of generic relational merging.
PostgreSQL itself documents [replication restrictions](https://www.postgresql.org/docs/18/logical-replication-restrictions.html)
and [conflicts](https://www.postgresql.org/docs/18/logical-replication-conflicts.html),
reinforcing the need for an explicit admitted replication profile.

## Signals, evidence and growth decisions

The [existing signal owner](engine-data-flow.md#signal-to-action-acceptance)
defines actionable checks. Emit diagnostics from the same operation identity;
derive UI status from canonical receipts/checkpoints, never from the presence
of a log line. Exporter failure cannot change transaction success. Keep
identifiers in authorized trace/evidence fields and out of metric dimensions.
[Prometheus instrumentation](https://prometheus.io/docs/practices/instrumentation/)
supports this cardinality discipline.

Generate schemas and surface bindings from the existing canonical types and
catalogues. JSON Schema validation does not replace digest, policy, CAS,
cross-field or resource checks. In particular, `format` is not universally an
assertion; pin validator vocabulary and test semantic validators independently.
[JSON Schema 2020-12](https://json-schema.org/draft/2020-12)
is the reference. OpenTelemetry's [convention stability rules](https://opentelemetry.io/docs/specs/semconv/general/semantic-convention-groups/)
and [GenAI repository](https://github.com/open-telemetry/semantic-conventions-genai)
require a pinned export mapping; upstream changing attributes cannot silently
rename RRFlow's frozen signal authority or enable prompt-content collection.

The capacity ladder and workload/evidence field specification live in
[Gate J](../roadmap/rrflow-1.0/gate-j.md#capacity-and-optimization-qualification).
Targets there are proposed engineering acceptance values, not published
capabilities. Revisit placement after measuring idle per-engram overhead and
noisy neighbors; revisit backend workers after cancellation and queue evidence;
revisit cache/index choices only at matched quality; revisit distributed
operation only after the single-node limit is measured and its consistency
contract is qualified.

Research used primary documentation on 2026-10-20. Cargo.lock records
DataFusion 55.0.0, Arrow 59.2.0 and Tokio 1.53.1. The version-specific DataFusion
documentation URL was unavailable through the research tool; current upstream
provider/pool material is architectural guidance only. Implementation must
verify APIs against the locked source and retain conformance evidence. No
dependency upgrade, source adaptation or benchmark result is implied.
