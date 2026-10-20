# Qdrant v1.19.1 capability reference

**Status:** active, source-pinned external-system research; not RRFlow architecture, implementation status, or delivery authority
**Coordinate:** `rrflow://rrflow-instance/data/research/qdrant-capability-inventory`
**Owner:** Qdrant capability taxonomy, pinned implementation anchors, and bounded adaptation inputs for RRFlow
**Reviewed:** 2026-09-08
**Upstream baseline:** Qdrant `v1.19.1`, signed annotated tag `de333e3c04660fe475d6275e9efc9fb9f54138fe`, dereferenced commit `6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de`

This record answers one question: which Qdrant behaviors are useful reference
inputs while RRFlow builds its native vector, hybrid-retrieval, persistence,
resource, and deployment capabilities? It does not make Qdrant a dependency,
sidecar, compatibility target, storage authority, query authority, or public
product model. It cannot mark an RRFlow roadmap gate complete.

RRFlow terms and current maturity come from the repository
[`README.md`](../../README.md). Required outcomes, dependency order, and open
deficiencies remain owned by the
[alpha objective](../objectives/rrflow-1.0-alpha.md),
[roadmap](../roadmap/rrflow-1.0.md), and
[POA&M](../poam/rrflow-1.0-alpha.md). The accepted engine flow is defined in
[`engine-data-flow.md`](../architecture/engine-data-flow.md).

## Research boundary

The technical baseline is the exact Qdrant source commit above, its signed
`v1.19.1` release, and first-party documentation available on the review date.
Documentation pages describe the current product and can move after this
review; source links below are commit-pinned. A later RRFlow implementation
package must repin any upstream behavior that it actually adapts.

This is a technical provenance record, not a legal-rights determination. Even
when source use is authorized, RRFlow records the exact upstream file and
revision, extracts a bounded invariant or failure case, gives it an RRFlow
destination, and proves the resulting RRFlow behavior independently. No
package may copy an upstream directory tree, preserve a Qdrant wire/storage
compatibility shape, or install Qdrant as hidden first-party implementation.

Qdrant and RRFlow solve overlapping but different problems:

- Qdrant is a vector-search system organized around collections, points,
  vectors, payloads, segments, shards, and replicas.
- RRFlow is one per-project AI governance, reasoning, and recall engine. Its
  vector capability is one transactional projection family beside canonical
  records, temporal graph relations, scalar/unique indexes, BM25, reasoning
  state, evidence, and feedback.
- A Qdrant HNSW graph is an approximate-nearest-neighbor index. It is not the
  RRFlow temporal property graph and cannot satisfy native graph traversal.
- Qdrant's BM25 facility is expressed through sparse-vector inference/search.
  RRFlow's native BM25 postings remain a distinct lexical access path that may
  be fused with vector and graph results by rrflowQL.
- Qdrant's distributed consistency model is not RRFlow's transaction model.
  Qdrant documents that Raft governs cluster metadata rather than point data
  and that distributed point updates are not atomically committed by default.
  RRFlow therefore retains one stronger `RrdEngine` transaction boundary for
  records, graph edges, index deltas, audit, and outbox state.

## Pinned source topology

The source tree is useful because it separates collection coordination from
segment-local data structures rather than presenting vector search as one
monolith. These anchors establish observed Qdrant responsibilities; their
module names are not proposed RRFlow names.

| Qdrant source anchor at `6ab21cac` | Observed responsibility | RRFlow use |
|---|---|---|
| [`lib/segment/src/data_types/manifest.rs`](https://github.com/qdrant/qdrant/blob/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/segment/src/data_types/manifest.rs) | Segment and per-file sequence-version metadata. | Reference for generation/coverage validation, never an RRFlow manifest format. |
| [`lib/collection/src/update_workers/update_worker.rs`](https://github.com/qdrant/qdrant/blob/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/collection/src/update_workers/update_worker.rs) | Ordered update consumption, WAL flush coordination, feedback, and optimizer signaling. | Failure-ordering input for the sole engine transaction, delta, receipt, and maintenance flow. |
| [`lib/collection/src/operations/universal_query/collection_query.rs`](https://github.com/qdrant/qdrant/blob/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/collection/src/operations/universal_query/collection_query.rs) | Recursive prefetch, vector/recommend/discover/context queries, fusion, grouping, and validation. | Behavioral oracle for bounded rrflowQL logical and physical plans. |
| [`lib/segment/src/index/struct_payload_index/read_view/optimizer.rs`](https://github.com/qdrant/qdrant/blob/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/segment/src/index/struct_payload_index/read_view/optimizer.rs) | Payload-index cardinality and filter planning. | Selectivity/cost-model input for typed RRFlow scalar and vector-filter access paths. |
| [`lib/segment/src/index/hnsw_index/graph_layers_builder.rs`](https://github.com/qdrant/qdrant/blob/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/segment/src/index/hnsw_index/graph_layers_builder.rs) | HNSW graph construction, connectivity checks, atomic artifact files, and memory-mapped loading. | Algorithm and failure-test reference for native RRFlow HNSW projections. |
| [`lib/segment/src/index/sparse_index/sparse_vector_index/read_view/idf.rs`](https://github.com/qdrant/qdrant/blob/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/segment/src/index/sparse_index/sparse_vector_index/read_view/idf.rs) | Sparse-vector IDF adjustment during reads. | Corpus-scope and tenant/filter test input; not a replacement for RRFlow BM25 postings. |
| [`lib/segment/src/vector_storage/quantized/quantized_vectors`](https://github.com/qdrant/qdrant/tree/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/segment/src/vector_storage/quantized/quantized_vectors) | Scalar, product, binary, and TurboQuant artifact creation/loading. | Codec, alignment, mmap, recall, and exact-rerank differential inputs. |
| [`lib/collection/src/shards/transfer/mod.rs`](https://github.com/qdrant/qdrant/blob/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/collection/src/shards/transfer/mod.rs) | Stream-record, snapshot, WAL-delta, and reshard transfer modes and state. | Future distributed fault/recovery oracle only; no first-alpha distributed claim. |
| [`lib/edge`](https://github.com/qdrant/qdrant/tree/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de/lib/edge) | In-process edge shard bindings, configuration, reads, writes, and snapshots. | Packaging and offline-test reference; never an edge-owned RRFlow state authority. |

The v1.19 release line materially affects this review. Qdrant `v1.19.0`
introduced per-component `cold`/`cached`/`pinned` placement, TurboQuant as a
four-bit primary vector datatype, per-query IDF corpus selection, global
quotas, read-affinity routing, and multiple storage/query optimizations.
`v1.19.1` then added batched HNSW and quantized scoring improvements plus
crash-safety and data-consistency fixes. RRFlow must test the failure cases and
tradeoffs, not merely reproduce the feature names.

## Capability and adaptation matrix

Each row distinguishes an upstream behavior from the RRFlow rule that may be
derived from it. “Required gate” means the behavior remains unaccepted until
that gate's named RRFlow evidence passes.

| Capability family | Qdrant v1.19.1 behavior | RRFlow adaptation rule | RRFlow owner / required gate |
|---|---|---|---|
| Logical data model | A collection contains points; a point has an ID, optional JSON payload, and zero or more named dense, sparse, or multivectors. Each named vector fixes kind, dimensions, and distance. | Keep typed vector collections and named-vector contracts as schemas inside one estate. A collection is neither an estate nor an rrflowDB. Canonical subjects and vector versions use RRFlow identity, bitemporal validity, model provenance, and transaction stamps. | [`collections.md`](../reference/vector/collections.md); C-03, E-04 |
| Updates and visibility | Point operations enter a WAL, receive sequence numbers, and are applied to segments; `wait=false` acknowledges receipt before search visibility, while `wait=true` waits for application. | One `RrdEngine` commit must state semantic certainty explicitly. Canonical data, graph/index deltas, audit, and outbox publish atomically; asynchronous projection work uses prepared work and accepted receipts without weakening commit truth. | C-02 through C-04, E-01 through E-04, H-03, H-05 |
| Segment lifecycle | Collections contain independent appendable and non-appendable segments with vector storage, payload storage, indexes, and ID maps; background optimizers consolidate them. | rrflowKV owns one key spine, WAL/MVCC/manifest sequence, Arrow-native immutable pages, deltas, and compaction. Qdrant segment boundaries inform lifecycle and fault tests but do not dictate RRFlow's file layout. | C-01, C-02, C-04, C-06, C-07 |
| Exact and ANN search | Qdrant can use exact scans or HNSW, with configurable construction/search breadth and full-scan crossover. | Preserve an exact RRFlow oracle. HNSW is a source-cursor-bound derived projection with an exact delta overlay and deterministic exact rerank; the cost planner chooses exact or ANN under one read stamp. | [`search.md`](../reference/vector/search.md), [`hnsw-projection.md`](../reference/vector/hnsw-projection.md); E-04, E-05 |
| Filtered ANN | Payload indexes estimate cardinality; Qdrant can add payload-aware HNSW edges, use ACORN traversal, or select a full scan according to filter selectivity. | Typed scalar/payload indexes must be physical and transactionally maintained. rrflowQL uses their measured selectivity to push filters into vector candidate generation or choose another access path. Copying Qdrant's graph-edge layout is not implied. | E-02, E-04, E-05, F-02, F-03 |
| Lexical and sparse retrieval | Qdrant supplies text payload indexes, sparse-vector indexes, tokenization options, BM25 sparse inference, and query-scoped IDF. | RRFlow keeps incremental native BM25 postings and model-produced sparse vectors as separate typed leaves. Both can enter the same plan and RRF, but one cannot silently substitute for the other. | E-03, E-04, E-05, F-03 |
| Hybrid and recursive query | The Query API supports nested prefetch, dense/sparse/multivector search, recommend, discover, context, RRF/DBSF, formula rescoring, grouping, facets, and matrices. | Preserve this useful retrieval algebra where it fits RRFlow contracts, but lower it into one bounded rrflowQL plan. Native graph/BM25/HNSW/RRF operators and DataFusion transformations consume the same read-stamped Arrow streams; no second in-memory query executor becomes authoritative. | [`retrieval.md`](../reference/context/retrieval.md), [`engine-data-flow.md`](../architecture/engine-data-flow.md); E-05, F-01 through F-04, H-01 |
| Quantization | Qdrant supports scalar, product, binary, and TurboQuant compression, optional rescoring, and compressed primary vector datatypes. | Quantization remains a typed, versioned RRFlow projection unless a later evidence-backed schema explicitly chooses a compressed canonical datatype. Record codec/model/configuration identity, retain exact-oracle comparisons, and reject silent quality loss. | [`quantization-lifecycle.md`](../reference/vector/quantization-lifecycle.md); E-04, F-03, J-04 |
| Memory placement | Dense vectors, HNSW, quantized vectors, sparse indexes, payloads, and payload indexes can independently use pinned, cached, or cold placement where supported. | Keep vector-artifact residency distinct from rrflowMX and from the bounded DataFusion memory pool. Moka or any other cache library is an implementation candidate, not architecture; F-05 retains one only after hit-rate, RSS, eviction, compaction-interference, and duplicate-residency evidence. | [`memory-tiers.md`](../reference/vector/memory-tiers.md); C-06, F-04, F-05, J-04 |
| Multitenancy | Qdrant recommends payload partitioning, user-defined shard keys, or tiered tenant placement rather than unbounded collections. | The first RRFlow alpha is one project ↔ one estate ↔ one instance. Tenant or shard vocabulary may later describe authorized logical partition or physical placement, but cannot create a second project/estate topology now. | [`instance-topology.md`](../architecture/instance-topology.md); D-01, D-03; later distributed amendment |
| Distribution and recovery | Qdrant shards and replicates collections, uses Raft for cluster metadata, offers configurable read/write consistency, and transfers by records, snapshot, or WAL delta. Point updates are not one atomic distributed transaction by default. | Retain transfer integrity, fencing, restart, and partial-failure scenarios as future tests. Do not import Qdrant consistency as RRFlow ACID semantics or advertise a clustered first alpha. Any future replication applies engine-compiled commits and all semantic families together. | [`cluster-contract.md`](../reference/distributed/cluster-contract.md); J-01 through J-05 plus a future explicit distributed gate |
| Resource protection | Strict mode can reject unindexed filters, excessive limits, exact searches, oversampling, large batches, and other expensive requests; global quotas can reject resource-consuming writes. | Authorization and budgets are mandatory engine inputs, not an optional adapter setting. Plan admission, scan/output/time/memory/spill limits, storage pressure, and typed denial evidence must apply across graph, lexical, vector, DataFusion, and delivery paths. | E-05, F-02, F-04, F-05, H-05, J-02 |
| API and SDKs | Qdrant exposes REST and gRPC and publishes several official clients. | RRFlow's HTTP, WebSocket, GraphQL-lowering, MCP, CLI, and SDKs remain outward adapters over the same public catalogue and `RrdEngine`; no Qdrant-compatible endpoint or client model is retained. | B-04, B-05, H-04, H-07, J-01 through J-05 |
| Inference | Self-hosted Qdrant expects client-side inference except for in-cluster BM25; managed cloud can call hosted or external embedding providers. | RRFlow owns provider-neutral, manifest-bound embedding and router contracts. Local or remote model providers are installed adapters whose proposals and embeddings are validated before canonical mutation. | B-03, D-05, G-01 through G-04, H-01 |
| Edge | Qdrant Edge provides an embedded shard API and snapshot/synchronization surfaces. | RRFlow may ship embedded or offline forms, but they open the same engine semantics or carry explicitly derived artifacts. Edge code cannot own another catalogue, transaction log, or reasoning lifecycle. | H-07, J-03, J-05 |
| Security and operations | Self-hosted Qdrant requires explicit authentication, TLS, network binding, and hardening; it exposes metrics, telemetry, health endpoints, snapshots, and operational controls. | D-01 installation must default closed, bind identity and secrets without plaintext state, prove authenticated readiness and persistent readback, and emit correlated low-cardinality resource/failure evidence. External mesh, observability, and orchestration systems remain optional adapters. | D-01 through D-06, H-05, J-02 through J-05 |

## Canonical RRFlow flow informed by this research

Qdrant contributes algorithms, workload shapes, and failure cases to this
flow; it does not add another box to it.

```text
HTTP | WebSocket | SDK | CLI | MCP
                 |
                 v
     RrdEngine ingress + authentication
                 |
       authorize one requested effect set
                 |
       parse/bind rrflowQL or typed operation
                 |
        capture one immutable ReadStamp
                 |
      +----------+-------------------------+
      |                                    |
      v                                    v
native rrflowKV access paths       Arrow RecordBatch streams
record / graph / scalar /          through rrflowQL / DataFusion
BM25 / vector / projection         filter / aggregate / fusion
      |                                    |
      +----------------+-------------------+
                       v
          bounded context + evidence
                       |
            model or routine proposal
                       |
             RrdEngine revalidation
                       |
     one atomic rrflowMX or rrflowKV commit
 record + both graph directions + index deltas
       + reasoning + audit + outbox + receipt
```

The fast path may use direct rrflowKV point/range/adjacency/index access. The
analytical path streams those same stamped physical families as Arrow
`RecordBatch` values into bounded DataFusion execution. DataFusion computes;
it never commits canonical state. A computed batch or model output returns as
a proposal to `RrdEngine`, which authorizes and commits it. rrflowMX and
rrflowKV implement the same semantic operations; only rrflowKV claims durable
reopen after the required evidence exists.

## Current RRFlow implementation facts

These observations constrain later implementation work. They are not a second
status ledger and do not change a roadmap checkbox.

| Current evidence | Consequence still owned by the roadmap |
|---|---|
| Vector collections, typed named vectors, model provenance, payload-index definitions, exact scoring, deterministic HNSW, exact rerank, scalar/product/binary/TurboQuant codecs, residency policies, and recursive retrieval contracts exist. | Existing files are implementation inventory. Their canonical behavior must survive direct convergence and pass the E/F/J evidence. |
| `RrdEngine` vector point, search, collection deletion, HNSW and quantization build, and recursive retrieval paths select the scope's whole vector-version family through C-04's direct reader (`read_vector_versions`) and reduce it in memory. Search, HNSW build, and collection or payload-index deletion also rebuild the artifact and quantization catalogues from cursor 0, and quantization operations rebuild the latter ([POAM-028](../poam/rrflow-1.0-alpha/poam-028.md)). | C-04, reopened, must serve both catalogues from direct reads at the request `ReadStamp`; E-04 requires collection-addressed point/range/index reads and incremental deltas. A larger key budget is not the solution. |
| The current HNSW format is deterministic canonical JSON and incremental advance clones the active graph into a new generation. | E-04 must establish a compact persistent projection, bounded delta maintenance, exact overlay/rerank, and crash/reopen behavior before Qdrant-class performance can be discussed. |
| Payload-index entries currently validate schemas and artifact prerequisites but do not constitute the complete native scalar/filter indexes needed by the planner. | E-02/E-05/F-02 require transactional physical indexes, cardinality evidence, and pushdown. |
| Hybrid and recursive retrieval execute multiple branches and perform fusion, recommendation, discovery, MMR, facets, groups, and matrix work in Rust collections. | F-03 must lower the accepted algebra into native and DataFusion physical operators over one stamp and one shared budget. |
| Quantization has one ready/active/retired lifecycle and TurboQuant no longer has a generic `ensure_vector_index` adapter, but full builds still materialize the selected canonical vector set before encoding a new generation, and the lifecycle catalogue is rebuilt from cursor 0 ([POAM-028](../poam/rrflow-1.0-alpha/poam-028.md)). | E-04 must consume committed source deltas incrementally and retain codecs only with exact-oracle, recall, resource, update, crash, and reopen evidence. |
| Current Arrow/DataFusion execution exists, but it materializes broad row collections before `MemorySource` execution rather than exposing streamed rrflowKV partitions. | F-01 through F-05 build the stamped provider, projection/filter/limit pushdown, native graph/BM25/HNSW/RRF operators, shared memory/spill limits, and cache accounting. |

The detailed current contracts remain in the
[vector reference index](../reference/vector/README.md),
[query index catalogue](../reference/query/index-catalogue.md), and
[retrieval reference](../reference/context/retrieval.md). If those owners and
this research record disagree about RRFlow, those owners win.

## Adaptation protocol

Before using any Qdrant implementation detail, the assigned RRFlow package
must record all of the following in its package journal:

1. the exact Qdrant tag, commit, file, symbol, and relevant test or issue;
2. the bounded behavior, invariant, algorithm, or failure mode being studied;
3. the RRFlow semantic family, canonical destination, transaction boundary,
   key/projection identity, Arrow schema, budget, and owning roadmap gate;
4. which Qdrant assumptions are rejected, including collection authority,
   eventual visibility, metadata-only consensus, JSON payload semantics,
   wire compatibility, storage format, or background-worker ownership;
5. an independent exact oracle and, where relevant, a differential corpus for
   recall, ranking, filtering, mutation, restart, corruption, cancellation,
   pressure, and resource accounting;
6. provenance retained in code comments or evidence without adopting a
   Qdrant namespace or forwarding API; and
7. deletion of any temporary experimental path before the package commits.

A copied algorithm that compiles is not accepted. A translated algorithm is
accepted only when the named RRFlow semantic, fault, resource, and reopen
evidence passes through the one engine boundary.

## Comparative-evidence contract

RRFlow currently makes no ease, performance, durability, recall, or
superiority claim against Qdrant. Gate J-04 may support such a claim only with
a like-for-like, source-pinned comparison that records:

- exact Qdrant tag, image or binary digest, configuration, feature flags, and
  client/harness revisions;
- exact RRFlow revision, release manifest, storage profile, and configuration;
- hardware, kernel, filesystem, mount options, storage medium, memory limits,
  CPU affinity, accelerator, toolchains, and dependency closure;
- corpus generation and digests, vector kinds/dimensions/distributions,
  payload/filter selectivity, mutation mix, concurrency, warmup, and duration;
- exact and ANN recall, ranking agreement, tail latency, throughput, failures,
  CPU, page faults, RSS, cache residency, I/O, write amplification, and
  logical/apparent/allocated bytes;
- update visibility, crash points, WAL recovery, corruption handling,
  snapshot/reopen behavior, and long-duration resource drift; and
- clean installation bytes, commands, elapsed time, processes/services,
  ports, configuration, secrets, authenticated readiness, and persistent
  readback.

Results that omit failed runs, use unlike semantics, compare warm with cold,
or rely on an unpinned `latest`/`master` input cannot support a product claim.

## Claim-to-source ledger

All sources are first-party and were accessed on 2026-09-08.

| Claim family | Primary source |
|---|---|
| Exact release identity and v1.19.1 changes | Qdrant, [`v1.19.1` release](https://github.com/qdrant/qdrant/releases/tag/v1.19.1), signed tag dereferenced independently with `git ls-remote` to `6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de` |
| v1.19 feature additions and failure fixes inherited by the baseline | Qdrant, [`v1.19.0` release](https://github.com/qdrant/qdrant/releases/tag/v1.19.0) |
| Collections, named vectors, sparse vectors, datatypes, configuration | Qdrant, [Collections](https://qdrant.tech/documentation/manage-data/collections/) and [Points](https://qdrant.tech/documentation/manage-data/points/) |
| Segments, mmap/vector placement, payload storage, WAL sequence/version recovery | Qdrant, [Storage](https://qdrant.tech/documentation/manage-data/storage/) and pinned source anchors above |
| Payload/full-text/sparse indexes, cardinality planning, filter-aware HNSW, ACORN | Qdrant, [Indexing](https://qdrant.tech/documentation/manage-data/indexing/) and [Filtering](https://qdrant.tech/documentation/search/filtering/) |
| Nested prefetch, RRF/DBSF, rescoring, formulas, grouping | Qdrant, [Hybrid and multi-stage queries](https://qdrant.tech/documentation/search/hybrid-queries/) and pinned collection-query source above |
| Quantization and per-component memory placement | Qdrant, [Quantization](https://qdrant.tech/documentation/manage-data/quantization/) and [Memory tiers](https://qdrant.tech/documentation/ops-configuration/memory-tiers/) |
| Tenant partition/shard strategies and filtered IDF | Qdrant, [Multitenancy](https://qdrant.tech/documentation/manage-data/multitenancy/) |
| Shards, replicas, transfer methods, recovery, and consistency limitations | Qdrant, [Distributed deployment](https://qdrant.tech/documentation/scaling/distributed_deployment/), [Consistency guarantees](https://qdrant.tech/documentation/scaling/consistency-guarantees/), and [Snapshots](https://qdrant.tech/documentation/operations/snapshots/) |
| Strict-mode and resource controls | Qdrant, [Administration](https://qdrant.tech/documentation/operations/administration/) and [Configuration](https://qdrant.tech/documentation/operations/configuration/) |
| REST/gRPC and official client surface | Qdrant, [API and SDKs](https://qdrant.tech/documentation/interfaces/) |
| Self-hosted security defaults and hardening | Qdrant, [Security and access control](https://qdrant.tech/documentation/security/) |
| Metrics, telemetry, and health surfaces | Qdrant, [Monitoring and telemetry](https://qdrant.tech/documentation/ops-monitoring/monitoring/) |
| Inference and edge product boundaries | Qdrant, [Inference](https://qdrant.tech/documentation/inference/) and [Edge API](https://qdrant.tech/documentation/edge/edge-api/) |

The research stopped after the consequential capability families had exact
source or first-party documentation support, Qdrant's transaction/consistency
limits were explicitly reconciled, and every retained pattern had one RRFlow
owner and evidence gate. Further Qdrant feature enumeration would not change
the immediate RRFlow dependency order: finish canonical storage and
transactions, then native access paths, then streamed Arrow/DataFusion
execution, and only then comparative qualification.
