# SurrealDB v3.2.4 capability reference

**Status:** active, source-pinned external-system research; not RRFlow architecture, implementation status, or delivery authority
**Coordinate:** `rrflow://rrflow-instance/data/research/surrealdb-capability-inventory`
**Owner:** SurrealDB capability taxonomy, pinned implementation anchors, and bounded adaptation inputs for RRFlow
**Reviewed:** 2026-09-10
**Upstream baseline:** SurrealDB `v3.2.4`, lightweight tag at verified commit `93ab219d69f09d8f999851b0359c80ebe6726102`
**Preview watchpoint:** SurrealDB `v3.3.0-beta.3`, commit `6dce5c84e29ff6c12b73c401b2251566e1aeca60`; not a stable baseline

This record answers one question: which SurrealDB behaviors and failure cases
are useful reference inputs while RRFlow builds one persistent multi-model AI
governance, reasoning, and recall engine? It does not make SurrealDB a
dependency, sidecar, compatibility target, storage format, query language,
topology, or public product model. It cannot mark an RRFlow roadmap gate
complete.

RRFlow terms and current maturity come from the repository
[`README.md`](../../README.md). Required outcomes, dependency order, and open
deficiencies remain owned by the
[alpha objective](../objectives/rrflow-1.0-alpha.md),
[roadmap](../roadmap/rrflow-1.0.md), and
[POA&M](../poam/rrflow-1.0-alpha.md). The accepted engine composition and
context path are defined in
[`system-overview.md`](../architecture/system-overview.md) and
[`engine-data-flow.md`](../architecture/engine-data-flow.md).

## Research boundary

The technical baseline is the exact stable source commit above, SurrealDB's
3.2 release line, and first-party documentation available on the review date.
Documentation pages describe a moving product; source links below are pinned
to immutable commit coordinates. A later RRFlow implementation package must
repin any upstream behavior it actually adapts and identify its exact symbol,
test, issue, or failure report.

This is a technical provenance record, not a legal-rights determination. Even
when source use is authorized, RRFlow extracts only a bounded invariant,
algorithm, layout idea, test shape, or failure case; gives it an RRFlow-owned
destination; rejects incompatible assumptions; and proves the resulting
behavior independently. No package may copy an upstream directory tree,
retain SurrealDB names or encodings as first-party RRFlow vocabulary, or add a
successful compatibility path.

SurrealDB and RRFlow overlap, but they are not the same product:

- SurrealDB is a general multi-model database organized around namespaces,
  databases, tables, records, relations, indexes, functions, users, and
  multiple storage/deployment choices.
- RRFlow is one per-project AI estate observed through `RrdEngine`. Canonical
  project knowledge, temporal graph state, reasoning state, evidence,
  feedback, and native scalar/BM25/vector projections share one transaction
  model; `rrflowMX` and `rrflowKV` are volatile and durable profiles of those
  same semantics.
- `rrflowQL` is RRFlow's bounded query and analytical planning surface.
  DataFusion is embedded compute over read-stamped Arrow streams, never a
  second database or a writer of canonical state.
- PostgreSQL, Turso, Dragonfly, object stores, meshes, model providers, and
  application databases are optional installed adapters or governed sources.
  They do not become an RRFlow storage backend merely because a project uses
  them.

## Pinned upstream source topology

The source tree was resolved at the stable commit; the listed anchors were
inspected selectively for architecture and failure semantics. Their presence
does not imply that RRFlow adopts their types or module layout.

| Stable source anchor | Behavior studied | RRFlow use |
|---|---|---|
| [`surrealdb/core/src/dbs`](https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/dbs) | Execution context, iterator orchestration, capabilities, and query-level coordination. | Reference for keeping modular executors under one semantic authority, not for importing a second executor. |
| [`surrealdb/core/src/doc`](https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/doc) | One mutation pipeline coordinates field/schema work, records, edges, indexes, events, changefeeds, and live notifications. | Reference for an effect-complete `RrdEngine` commit plan. |
| [`surrealdb/core/src/key/graph/mod.rs`](https://github.com/surrealdb/surrealdb/blob/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/key/graph/mod.rs) | Ordered direction-specific graph keys keep the opposite endpoint in the adjacency key and support prefix traversal. | Input to an RRFlow-owned binary key grammar and atomic outgoing/incoming adjacency families. |
| [`surrealdb/core/src/kvs/tx.rs`](https://github.com/surrealdb/surrealdb/blob/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/kvs/tx.rs) | Transaction-local caches plus pending changefeed, live-query, and index-build work around one underlying transaction. | Reference for coordinating derived effects with commit outcome behind `RrdEngine`. |
| [`surrealdb/core/src/idx`](https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/idx) | Full-text/vector/scalar index implementations, planner inputs, and build state. | Workload, lifecycle, exactness, and failure-oracle inputs for E/F; Qdrant remains the richer vector reference. |
| [`surrealdb/core/src/kvs/index`](https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/kvs/index) | Concurrent index build coordination, queues, progress, and compaction. | Reference for leases, checkpoints, bounded work, fairness, quarantine, and restart tests. |
| [`surrealdb/core/src/rpc`](https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/rpc) | Protocol/session/transaction and response-format boundaries over the same datastore. | Adapter-conformance and resource-lifecycle input; no wire compatibility. |
| [`surrealdb/mcp/src`](https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/mcp/src) | MCP service, authentication, session, and tool adapters. | Reference for a thin authenticated outward adapter that cannot own context or lifecycle state. |

The stable release history is itself an important input. SurrealDB 3.2.4
bounded index-compaction draining, added an optional physical write-key cap,
and fixed a concurrent index build that could lose progress under active
writes. The 3.2.2 and 3.2.1 notes document durable RPC sessions, bounded
abandoned WebSocket transactions, live-query cache and conflict-error fixes,
resumable index builds, interruption cleanup, MVCC reclamation, exact planner
conditions, and a permission bypass inside ANN candidate evaluation. RRFlow
must retain these as adversarial cases rather than copying only happy-path
feature names.

## Capability and adaptation matrix

“Required gate” means the RRFlow behavior is unaccepted until that gate's
named evidence passes. Current types, files, and unit tests are inventory only.

| Capability family | SurrealDB v3.2.4 behavior | RRFlow adaptation rule | RRFlow owner / required gate |
|---|---|---|---|
| One modular engine | Query, document, index, key, transaction, RPC, and storage modules are separated inside one database product. | Keep focused crates/modules, but all authoritative operations compose through `RrdEngine`; physical modularity cannot create a second kernel, database, lifecycle, or context authority. | [`system-overview.md`](../architecture/system-overview.md); A-07, C-01, C-02 |
| Storage abstraction | The compute layer targets transactional point/range storage and supports several backend/deployment profiles. | `rrflowMX` and `rrflowKV` are the only alpha storage profiles. External databases are data/activity adapters. No backend selector, SurrealKV/RocksDB/TiKV compatibility, or storage-specific semantic branch survives. | C-01 through C-04, D-03, J-01 |
| Ordered key grammar | Documents, relations, indexes, catalogues, and metadata lower to ordered byte-key families. | Freeze an RRFlow binary tuple/key grammar with explicit family, estate/scope, type, identity, temporal/version, direction, index, and generation components. Lexicographic order, prefix bounds, and malformed-key denial require golden bytes and property tests. | [`rrflowkv-current-format.md`](../reference/storage/rrflowkv-current-format.md); C-01, C-05 |
| Native model families | Documents, relational records, graph relations, time-series values, geospatial values, vectors, files, and scalar/object values share one multi-model surface. | Keep each accepted RRFlow model as a typed semantic family under one identity, schema, temporal, transaction, and authorization model. Do not collapse native meaning into opaque JSON or split a family into a sidecar engine. | [`multi-model-object-contract.md`](../reference/data/multi-model-object-contract.md); C-01 through C-04, E-01 through E-04 |
| Multi-model commit | Record, graph, index, event, changefeed, and live effects are coordinated in one mutation pipeline and transaction. | `RrdEngine` compiles one authorized effect set. Canonical values, both graph directions, scalar/BM25/vector deltas, reasoning/evidence, audit, outbox, and receipt cross one rrflowMX/rrflowKV commit boundary or none do. DataFusion and adapters only return proposals. | C-02, C-03, E-01 through E-04, G-04, H-03, H-05 |
| Transactions and isolation | SurrealDB documents snapshot isolation and detects write conflicts at commit; it is not a blanket serializable-isolation claim. | State RRFlow isolation and conflict ranges explicitly. Preserve immutable read stamps and optimistic conflict denial, add predicate/range protection only where required, and never infer serializability from a successful test. | C-02, C-03, E-02, J-02 |
| Physical write fan-out | One logical row can cause record, edge, index, full-text, and cleanup writes; 3.2.4 can fail atomically at a configured write-key cap. | Admission estimates and execution counts the full physical effect set, including adjacency and index deltas. Exceeding keys/bytes/time/memory fails before or inside the transaction with no partial publication. Internal maintenance is also bounded by a separately declared policy. | C-02, C-07, E-01 through E-04, F-04, J-02 |
| Native graph traversal | Direction-specific ordered adjacency keys make traversal a bounded prefix/range operation instead of a table-wide relation scan. | Persist RRFlow-owned outgoing and incoming adjacency entries atomically with the canonical relation record. Temporal visibility, delete/retire, dangling-reference denial, self-loops, duplicate edges, and both-direction traversal require exact oracles and reopen proof. | E-01, C-03, C-04 |
| Scalar and unique indexes | Secondary, compound, count, and uniqueness indexes participate in mutation and planning. | Maintain scalar/compound/unique entries in the semantic transaction. A uniqueness constraint is authoritative even while a derived access artifact rebuilds; planner selection requires exact stamp/configuration/cardinality evidence. | E-02, C-02, E-05 |
| Schema and stored behavior | Schema definitions, functions, events, and stored behavior are catalogued database capabilities. | Schema revisions, functions, triggers, routines, and skills remain separately typed, digest-bound, policy-gated engine records. Stored behavior may propose or schedule work but cannot create a parallel lifecycle, persistence, authorization, or query authority. | [`schema-catalogue.md`](../reference/data/schema-catalogue.md), [`functions.md`](../reference/automation/functions.md); D-02, H-05, I-01 through I-07 |
| Full-text and vector indexes | Full-text and vector structures are native index families selected by the planner. | Keep incremental RRFlow BM25 postings and Qdrant-informed vector/HNSW projections as distinct typed paths under the same stamp. Neither replaces canonical data; both retain an exact oracle and deterministic rerank/fusion evidence. | [`qdrant-capability-inventory.md`](qdrant-capability-inventory.md); E-03 through E-05, F-03 |
| Concurrent index lifecycle | Builds and rebuilds have generations, progress, takeover/restart, cleanup, and publication concerns; stable releases fixed lost progress, starvation, and unbounded queues. | Use engine-owned jobs, leases, checkpoints, source cursors, bounded batches, fair scheduling, quarantine, and atomic ready-generation publication. A build cannot resume from zero silently or claim readiness from an emitted event. | C-07, E-02 through E-04, H-05, I-03 |
| Query planning | Source, predicates, permissions, available indexes, and exactness determine physical access. Planner bugs can return wrong counts or leak restricted ANN candidates. | Bind authorization and one `ReadStamp` before planning. Cost candidates include point/range, adjacency, scalar/BM25/vector, exact scan, and Arrow/DataFusion operators; security predicates must be enforced at the earliest candidate-producing operator and revalidated at delivery. | E-05, F-02 through F-04, H-04, H-05 |
| Query languages | SurrealQL, GraphQL, and experimental GQL lower onto SurrealDB execution. | RRFlow accepts typed operations and rrflowQL. GraphQL may lower through the public contract, but no SurrealQL/GQL syntax, behavior, RPC, or compatibility promise enters the engine. Parsing never grants mutation authority. | B-05, F-01 through F-03, H-04 |
| Analytical execution | SurrealDB's streaming query engine is not Apache DataFusion and does not define RRFlow's Arrow-page contract. | rrflowKV exposes pinned, read-stamped Arrow-compatible page streams; rrflowQL composes native access with bounded DataFusion plans. Projection/filter/limit pushdown, buffer ownership, cancellation, spill, scan, decode, copy, allocation, and cache residency are measurable. | F-01 through F-05, C-06 |
| Live results | Live queries publish committed matching changes; durable changefeeds are a separate history/replay concern and strict total ordering must be specified by the consumer contract. | Keep durable changefeed cursor pages distinct from live semantic deltas. Derive subscription impact from the accepted commit and index changes, persist ACK/lease/backpressure state, and avoid re-running complete before/after snapshots. | H-03, B-04, H-05 |
| Security and capabilities | Authentication, permissions, row/field policy, capability gates, and candidate-time checks constrain execution; release fixes show the danger of adapter-only checks. | One engine authorization decision is bound to resource, policy revision, read stamp, plan, proposal, commit, delivery, and trace. Adapters cannot weaken it; secrets remain external references and sensitive outputs are structurally redacted. | D-01, H-04, H-05, J-02 |
| Extensions and model execution | Functions, JavaScript/Wasm execution, object/file integrations, and model/provider facilities extend the database surface. | RRFlow exposes closed, capability-scoped extension contracts only where a gate requires them. Provider-neutral embedding/model execution, immutable-object activity, and project-generator adapters enter through typed proposals and receipts; no embedded language, object store, model, or provider gains engine authority or SurrealDB compatibility. | [`functions.md`](../reference/automation/functions.md), [`embedding-and-model-bound-search.md`](../reference/inference/embedding-and-model-bound-search.md); D-03, E-04, G-02, H-04, I-01 through I-07 |
| RPC, SDK, and MCP | HTTP/WebSocket RPC, SDKs, and MCP expose the same database engine while managing session and transaction resources. | All RRFlow transports resolve one generic public operation catalogue and `RrdEngine`. MCP, Connectome, LFG, and provider adapters own carriage/presentation only; no client creates reasoning, persistence, attunement, or subscription truth. | B-04, H-04, H-07, J-01 through J-05 |
| Installation and readiness | SurrealDB publishes install forms and distinguishes liveness, backend reachability, and ready-to-serve state. Its agent material covers MCP/skill setup and repository memory ingestion. | D-01 installs one bundle-contained RRFlow instance closed by default; attunement inventories a real project incrementally and persists its jobs. Readiness proves authentication, installed binding, rrflowKV reopen, and a semantic read—not merely a bound port or uploaded folder. | D-01 through D-06, I-06, J-01, J-03, J-05 |
| Operations, observability, and recovery | Backup/restore, import/export, compaction, diagnostics, logs, metrics, and operational endpoints surround the database runtime. | RRFlow coordinates backup, archive, restore, compaction, tiering, and hibernation through typed engine jobs with leases, budgets, receipts, and correlated traces. A CLI response, open port, log line, dashboard, or copied directory is not durability or readiness proof. | [`operations/README.md`](../reference/operations/README.md), [`logical-archive.md`](../reference/storage/logical-archive.md), [`tiered-persistence.md`](../reference/storage/tiered-persistence.md); C-07, H-05, J-01 through J-05 |
| Interactive UI | Surrealist and administration surfaces expose database inspection and operations. | Connectome remains a separately deployable public client of the generic protocol. It visualizes stamped plans, graph/context evidence, jobs, subscriptions, and receipts; it cannot inspect storage internals as authority or own canonical state. | B-04, H-05, H-07, J-03 through J-05 |
| Topology and multitenancy | SurrealDB uses namespace → database → table/record plus separate cloud organization/project/instance vocabulary. | Reject that hierarchy as RRFlow authority. The first alpha remains one project ↔ one estate ↔ one instance; environment and physical placement are typed relations, not copied product tenancy levels. | [`instance-topology.md`](../architecture/instance-topology.md); D-01, D-03 |
| Distributed operation | SurrealDB has distributed products/backends with consistency, transfer, recovery, and upgrade concerns. | Use only failure, fencing, transfer, and recovery shapes for future evidence. The first alpha is single-node; no distributed claim exists until a later explicit gate proves the complete RRFlow semantic corpus on independent hosts. | J; later distributed amendment |
| Preview features | The 3.3 beta line advertises PostgreSQL wire support, gRPC streaming, object backends, bitmap fusion, and prefiltered vector work. | Treat preview behavior as a watchpoint only. It cannot change RRFlow's stable input baseline, roadmap order, product vocabulary, or acceptance claim; repin it only after a relevant implementation gate begins. | Research only until explicitly scheduled |

## Flat inventory reconciliation

The superseded flat scorecard had thirteen capability sections. This map
proves their disposition before the old file is removed; no former
“Verified” label is carried forward as RRFlow acceptance evidence.

| Superseded section | Canonical disposition |
|---|---|
| Product form, tenancy, and deployment | Split across one-engine composition, storage profiles, installation/readiness, topology, and distributed-operation rows. RRFlow keeps the one-project/one-estate alpha topology and rejects SurrealDB product and tenancy vocabulary. |
| Persistence, transactions, and lifecycle | Preserved in storage abstraction, ordered key grammar, multi-model commit, isolation, physical fan-out, concurrent index lifecycle, and operations/recovery rows; acceptance remains C and J evidence. |
| Native data models | Preserved as the native-model-families row and the RRFlow multi-model object contract. Each accepted model must become typed engine state rather than a JSON compatibility envelope. |
| Schema | Preserved in schema-and-stored-behavior, scalar/unique-index, and authorization rows. Functions and automation remain subordinate to the schema and engine transaction authorities. |
| Query | Preserved across query languages, planning, native access paths, and analytical execution. rrflowQL and the typed public operation catalogue remain the only RRFlow owners. |
| Indexing | Split into graph adjacency, scalar/unique, BM25/vector, index lifecycle, planner, and Qdrant-reference rows so no umbrella index claim can hide missing transaction or exactness proof. |
| Realtime | Preserved as committed live impact plus separately durable change history, subscription ACK/lease/backpressure state, and policy-gated automation. |
| Security | Preserved in the security/capabilities row and security authority documentation; enforcement spans candidate production through delivery rather than residing in an adapter. |
| APIs | Preserved in the RPC/SDK/MCP row and generic public contract; outward surfaces cannot create another engine or context authority. |
| Extensions | Split into schema/stored behavior and extensions/model execution. Functions, inference providers, object activity, generators, triggers, routines, and skills require closed typed contracts and explicit gates. |
| Operations, observability, UI, and estates | Split into operations/recovery, interactive UI, installation/readiness, and topology. Connectome stays a separate client repository; operational evidence stays engine-correlated. |
| Distributed operation | Preserved only as a future failure/evidence input. RRFlow 1.0 alpha remains single-node and makes no distributed claim. |
| SurrealDB 3.3 preview | Preserved only as a pinned watchpoint; it cannot change the stable baseline or authorize implementation work. |

## Canonical RRFlow flow informed by this research

SurrealDB contributes implementation patterns and failure cases to this flow;
it does not add another runtime component.

```text
HTTP | WebSocket | SDK | CLI | MCP | Connectome | LFG
                         |
                         v
             RrdEngine ingress + authentication
                         |
           authorize one bounded requested effect/query
                         |
          parse/bind typed operation or rrflowQL AST
                         |
             capture one immutable ReadStamp
                         |
          +--------------+--------------------+
          |                                   |
          v                                   v
 native rrflowKV access paths       stamped Arrow page streams
 point/range/adjacency/              through rrflowQL/DataFusion
 scalar/BM25/vector                  filter/aggregate/fuse/shape
          |                                   |
          +------------------+----------------+
                             v
                 bounded context + evidence
                             |
              model/routine/operator proposal
                             |
                  RrdEngine revalidation
                             |
          one atomic rrflowMX or rrflowKV commit
 record + both graph directions + index/reasoning deltas
          + audit + outbox + receipt + live impact
```

The fast and analytical paths share identity, schema, authorization, stamp,
budget, and evidence. `rrflowMX` may keep equivalent typed structures in
process; `rrflowKV` additionally owns WAL durability, MVCC, manifests,
immutable pages, compaction, crash recovery, and reopen. DataFusion computes
over borrowed or explicitly accounted Arrow buffers. It never commits state.

## Current RRFlow implementation facts

These facts came from complete reads of the current storage transaction,
rrflowKV, keyspace, query execution, DataFusion, index, live-query,
subscription, and MCP authority files. They constrain later work but do not
change a roadmap checkbox.

| Current evidence | Consequence still owned by the roadmap |
|---|---|
| `RrflowKvStore` opens an authenticated rrflowKV application format over the custom `rrd-lsm` database, persists WAL batches, validates read stamps, tracks an authenticated runtime log accumulator, and has crash/storage-full tests around WAL publication. C-01 through C-06 establish the typed key, shared transaction, atomic semantic fan-out, direct reads, one accepted reader set, and segment v6; C-04 is reopened because every vector search rebuilds the vector and quantization catalogues from cursor 0 ([POAM-028](../poam/rrflow-1.0-alpha/poam-028.md)). | This is a substantive durable substrate. C-07 must still qualify WAL/manifest recovery, bounded maintenance, write backpressure, pinned compaction, mapped-buffer lifetime, and storage-full behavior through the D-01 installed composition; C-04 must remove the vector-catalogue replay. |
| One semantic commit plan writes current and temporal records and relations, both adjacency directions, scalar/unique entries, scalar/BM25/vector source deltas, the hash-chained change log, accumulator, projection work, outbox, audit, function receipts, the catalogue revision it observed, and its outcome in one rrflowKV batch. | Adjacency, scalar entries, and source deltas have no production reader; BM25 term families have no writer; HNSW/quantized artifacts and reasoning state stay outside the commit. E-01 through E-04 and G must consume or add them without splitting authority. |
| Many semantic values, catalogues, changes, index artifacts, and control records remain encoded payload bytes, but accepted C-06 persists them in segment v6's ordered key/version spine and six Arrow-layout page buffers per row group, with authenticated row-group filters, none/adaptive-LZ4 page policy, raw-mmap borrowing versus owned decode, and read/borrow/copy/allocation/decompression/filter evidence. | A columnar physical envelope and membership filter do not make JSON-like semantic values native graph/BM25/vector columns or prove DataFusion streaming. C-07 owns mapped-buffer lifetime and maintenance qualification; E/F own typed access paths, exact fallbacks, and stamped `RecordBatch` execution. |
| `StorageEngine` exposes only transaction creation, the claim, control, function-catalogue, projection, runtime, and invocation repositories, and physical evidence. Index catalogue transitions commit in their own control transaction with their commit bindings and entry backfill, and every semantic commit writes the catalogue revision it observed, so the two conflict at physical commit rather than race. | The port split (A-07/C-02) is done. Index artifact bytes and ready-state transitions still publish in separate transactions; E-02/E-03 must bind generation and publication to one receipt with no second catalogue authority. |
| Query execution validates a stamp and uses DataFusion 55 with memory, spill, elapsed-time, batch, row, scan, and output bounds. | DataFusion is real and embedded, but F is not accepted until the provider streams stamped rrflowKV pages and accounts for decode/copy/allocation/backpressure across the full plan. |
| `execute` reduces directly selected semantic versions, or a whole decoded index artifact, into a complete `Vec<QueryRow>`, converts it to Arrow batches, feeds a `MemorySourceConfig`, declares filter pushdown unsupported, and collects output rows again. | F-01/F-02 replace eager materialization with a stamped rrflowKV provider over pinned page streams and measured projection/filter/limit pushdown. |
| rrflowQL traversal reduces the directly read relation-kind and record version families, then runs a breadth-first search that rescans every relation for each node; context assembly builds adjacency in memory from a full `RuntimeDataSnapshot`. The committed adjacency families have no reader. | E-01 must traverse the committed outgoing/incoming adjacency with prefix scans, temporal overlay rules, exact structural oracles, bounded steps, and reopen evidence. |
| Scalar/count/aggregate-count/geo/materialized/BM25 index catalogues publish immutable JSON artifacts; every build executes the full source query, ready reads decode the whole artifact, and “incremental reconciliation” compares complete old/new row maps. Commits already maintain scalar/unique entries and scalar/BM25 source deltas. | E-02 must serve scalar/unique access from the committed entries and E-03 must materialize BM25 from the source deltas; C-07 still owns bounded resumable generation work. Existing artifacts remain an oracle until replacement evidence passes. |
| Commit-bound unique indexes check a bounded value-prefix scan of committed unique entries, and first installation validates full source-query rows under the engine transaction gate. Schema unique properties/pairs and degree limits still scan every current scoped record or relation on each commit. Catalogue publication remains a separate control transaction. | rrflowKV detects write-write conflicts only, so uniqueness relies on every commit writing the global runtime cursor and the catalogue revision it observed. E-02 must keep that serialization or add range-conflict semantics, bound schema-constraint checks, and bind ready-generation publication with no second catalogue authority. |
| Live query polling executes a query at the prior cursor and again at the captured head, then diffs full row maps. Durable subscription state adds ownership, leases, ACKs, replay windows, and backpressure through control records. | Preserve the useful subscription safety semantics, but H-03 must consume commit impacts/durable cursor pages and persist state through the final typed engine model instead of polling two complete snapshots. |
| The MCP adapter can use an exclusive embedded engine or authenticated daemon client and calls the engine context operation. | Keep the thin-adapter shape; D/H/J still must prove installed configuration, provider-neutral endpoint resolution, complete authorization, bounded streaming, restart, and the same engine behavior across all outward surfaces. |

## Gate-by-gate convergence and proof

This table prevents later packages from converting upstream feature names or
passing tests into false completion claims.

| Gate package | Required authored result | Proof that can close it |
|---|---|---|
| C-01 | One frozen RRFlow key grammar and segment/page identity covering records, temporal relations, both adjacency directions, scalar/BM25/vector entries, reasoning state, audit, outbox, and manifests. | Golden byte ordering/prefix/malformed-key corpus plus direct removal of every successful prior-format writer/reader. |
| C-02 | A narrow `RrdEngine` transaction port compiles the complete authorized logical effect set into bounded physical mutations and returns one certainty-bearing receipt. | Conflict/range/fan-out/idempotency/lost-ack/fault injection at every write boundary proves all-or-none semantic effects. |
| C-03 | The exact same semantic operations and error classes run against rrflowMX and rrflowKV. | Shared randomized/model-based differential corpus; rrflowKV additionally survives process kill, torn tail, storage full, reopen, and compaction. |
| C-04 | Direct stamped point/range/family readers replace history reconstruction on normal paths. | Scan/decode/copy counters and adversarial corpus prove result equivalence while work follows selected keys/pages rather than total history. |
| C-06/C-07 | Immutable Arrow-compatible pages, delta/compaction policy, leases, cache accounting, backpressure, and bounded maintenance. | Zero-copy eligibility is demonstrated from buffer addresses/lifetimes; every fallback allocation is counted; pressure, cancellation, corruption, compaction, and long-run RSS tests retain failures. |
| E-01/E-02 | Atomic temporal adjacency and scalar/compound/unique entries with exact direct readers. | Structural and transaction differential against simple exact oracles, including deletes, retirements, self-loops, duplicates, temporal windows, conflicts, rebuild, and reopen. |
| E-03/E-04 | Incremental BM25 and vector/HNSW projection paths with exact lexical/vector oracles and deterministic rerank. | Mutation/restart/corruption/filter/recall/ranking/resource corpus at one stamp; see the pinned Qdrant reference for vector-specific inputs. |
| E-05/F | One costed physical planner composes native readers with streaming rrflowQL/DataFusion operators under one authorization and budget. | Plan-selection, security-pushdown, result differential, cancellation, spill, scan, memory, allocation, cache, concurrency, and historical-stamp evidence. |
| G/H | Persisted reasoning trees select bounded engine work; context/evidence/feedback and live impacts survive reopen. | Real task corpus proves selected/skipped routes, same-stamp evidence, CAS advancement, denied invalid proposals, feedback adaptation, token/resource effects, restart, and no hidden client lifecycle. |
| D/J | A closed-by-default bundle installs, configures, attunes, authenticates, opens, closes, and reopens one per-project RRFlow estate. | Clean-machine and existing-project installation matrices prove preview/apply/uninstall, secret handling, resumability, readiness, persistent semantic readback, outward conformance, and measured footprint. |

## Adaptation protocol

Before using any SurrealDB implementation detail, the assigned RRFlow package
must journal all of the following:

1. the exact SurrealDB tag, commit, file, symbol, and relevant test, issue, or
   release failure;
2. the bounded behavior, invariant, layout property, algorithm, or adversarial
   condition being studied;
3. the RRFlow semantic family, canonical destination, transaction boundary,
   binary key/page or projection identity, Arrow schema, budget, and owning
   roadmap gate;
4. rejected assumptions, including namespace/database/table authority,
   SurrealQL/GQL/GraphQL or RPC compatibility, general JSON physical form,
   backend selection, snapshot-isolation overclaim, migration reader,
   background-worker authority, edition/Cloud topology, and adapter-owned
   state;
5. an independent exact oracle and the required semantic, fault, corruption,
   pressure, cancellation, security, resource, compaction, crash, and reopen
   corpus;
6. provenance retained in code comments/evidence without adopting a
   SurrealDB namespace, exported contract, or on-disk shape; and
7. deletion of temporary experimental or comparison paths before commit.

A copied module that compiles is not accepted. A translated behavior is
accepted only when the RRFlow-owned semantic, transaction, resource, security,
and reopen evidence named by its gate passes through the one engine boundary.

## Comparative-evidence contract

RRFlow currently makes no ease, performance, durability, correctness, or
feature-superiority claim against SurrealDB. Gate J-04 may support a bounded
claim only with a like-for-like comparison that records:

- exact SurrealDB tag, commit/image/binary digest, edition, configuration,
  capabilities, storage backend, query surface, and client/harness revisions;
- exact RRFlow revision, release manifest, storage profile, configuration,
  key/page format, and enabled native/DataFusion operators;
- hardware, kernel, filesystem, mount options, storage medium, memory limits,
  CPU affinity, accelerators, toolchains, and dependency closure;
- corpus and digest, data model, schema/index definitions, temporal/history
  distribution, query/mutation mix, concurrency, warmup, and duration;
- result/transaction agreement, graph/index/vector recall, tail latency,
  throughput, failures, CPU, page faults, RSS, cache residency, I/O, write
  amplification, logical/apparent/allocated bytes, and context/token output;
- conflict/fan-out behavior, update visibility, crash points, WAL recovery,
  corruption, snapshot/reopen, compaction, rebuild, subscription replay, and
  long-duration resource drift; and
- clean installation bytes, commands, elapsed time, processes/services,
  ports, configuration, secrets, authenticated readiness, attunement, and
  persistent semantic readback.

Unlike semantics, omitted failures, warm-vs-cold comparisons, unpinned inputs,
or a benchmark that bypasses either product's normal authority cannot support
a claim.

### Rejected local claim diagnostic and retained corrections

KB-05 removed the 2026-08-23 RRFlow/SurrealDB claim diagnostic, its Python
driver, its stored ratio artifact, and the Rust test that asserted those
ratios. They were superseded pre-release residue, not J-04 evidence. The
review retained the useful failure lessons here instead of moving an
unqualified result into an evidence archive.

| Defect in the removed diagnostic | Direct evidence | Required J-04 correction |
|---|---|---|
| Baseline identity was incomplete and stale. | SurrealDB `3.0.5` and its local binary digest were recorded, but the canonical reference is `3.2.4`; the RRFlow revision, binary digest, build profile, dependency closure, and release manifest were absent. | Pin both source revisions and all executed artifact/image digests. Build or acquire them through the manifest-recorded commands and retain their complete closures. |
| The purported reproducer no longer executed. | It defaulted to a host-specific Cargo cache and invoked the current benchmark as `--child native --path …`; the current child contract is `--child --path …`. A fresh current build exited before its first trial. | Resolve only manifest-declared repository or release-bundle artifacts, validate the harness/binary handshake before timing, and fail the evidence run when any declared cell executes zero trials. |
| The semantic corpus and verification were not equal. | SurrealDB and RRFlow used different session values. SurrealDB verification checked every projected field, while the RRFlow verifier checked sequence, cardinality, and `object` only. The run did not inject conflicts or batch failures. | Generate one content-addressed typed corpus, lower it explicitly for each system, verify every field, identity, order, result, transaction outcome, and post-reopen value against an independent oracle, and retain failure cells. |
| Measurement layers were conflated. | Direct embedded RRFlow calls were compared with SurrealDB HTTP/SQL; SurrealDB server-reported statement time had no equivalent RRFlow phase. “Readiness” compared an authenticated server/namespace/database exchange with opening a storage object. | Report separate like-for-like kernel/storage, public-protocol, and clean-deployment cells. Every timer declares its boundaries, included processes, serialization, authentication, planning, and durability work; unlike cells are never divided into a competitive ratio. |
| Resource and lifecycle accounting was asymmetric. | Process inclusion differed, fixed CPU/memory/filesystem controls were absent, and the primary disk ratio compared different unmaintained states without an equivalent maintenance boundary. | Run both systems in equivalent constrained process/container envelopes and report logical, apparent, allocated, resident, cache, WAL, segment, snapshot, I/O, and maintenance phases independently before and after clean reopen. |
| Statistical and workload coverage was diagnostic only. | Three small alternating trials had no recorded warm-up, frequency/governor state, background-load control, concurrency matrix, confidence treatment, long-duration drift, or graph/BM25/vector/DataFusion/context workload. | Predeclare warm-up, order/randomization, repetitions, concurrency, duration, outlier and interval treatment, workload digests, mixed-family interference, recall/quality thresholds, and failure retention before execution. |
| The test proved stored direction, not reproducibility. | The Rust test parsed the checked-in JSON and asserted that selected ratios were above or below one; it never executed the harness, recomputed results from raw samples, or verified provenance. | Evidence verification validates schema/provenance, recomputes aggregates from immutable raw trials, rejects missing/failed cells, and never requires RRFlow to win. Reproduction is a separate commanded run against the pinned manifest. |

Alternating isolated trials, clean reopen, bounded paged verification, exact
artifact digests, and separate logical/apparent/allocated-byte accounting
remain useful inputs. They must be implemented once in the planned
`scripts/release/compare_deployment.py` and
`fixtures/release/deployment-baselines-v1.toml` J-04 package after its C
through J-03 prerequisites pass. No active benchmark implementation or RRFlow
performance claim survives this review.

## Claim-to-source ledger

All sources are first-party and were accessed on 2026-09-08.

| Claim family | Primary source |
|---|---|
| Stable release identity and bounded write/index failure changes | SurrealDB, [`3.2` release line](https://surrealdb.com/releases/3.2); `v3.2.4` tag independently resolved to commit `93ab219d69f09d8f999851b0359c80ebe6726102` |
| Preview watchpoint | SurrealDB, [`3.3` preview release line](https://surrealdb.com/releases/3.3); `v3.3.0-beta.3` independently resolved to commit `6dce5c84e29ff6c12b73c401b2251566e1aeca60` |
| Unified compute/storage transaction architecture and backend abstraction | SurrealDB, [Architecture](https://surrealdb.com/docs/learn/data-models/architecture) and pinned `dbs`/`doc`/`kvs` source anchors above |
| Native multi-model families | SurrealDB, [Data models](https://surrealdb.com/docs/learn/data-models), [Geospatial model](https://surrealdb.com/docs/learn/data-models/geospatial/overview), and [Files](https://surrealdb.com/docs/learn/schema-management/files/working-with-files) |
| Snapshot isolation and conflict behavior | SurrealDB, [Transactions](https://surrealdb.com/docs/learn/querying/concepts-and-guides/transactions) |
| Ordered directional graph keys | SurrealDB, pinned [`key/graph/mod.rs`](https://github.com/surrealdb/surrealdb/blob/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/key/graph/mod.rs) |
| Transaction-local index/changefeed/live coordination | SurrealDB, pinned [`kvs/tx.rs`](https://github.com/surrealdb/surrealdb/blob/93ab219d69f09d8f999851b0359c80ebe6726102/surrealdb/core/src/kvs/tx.rs) and `doc` source tree above |
| Live query behavior and limits | SurrealDB, [Live queries](https://surrealdb.com/docs/learn/querying/real-time/live-queries) and [LIVE SELECT](https://surrealdb.com/docs/reference/query-language/statements/live-select) |
| Durable change history | SurrealDB, [Changefeeds](https://surrealdb.com/docs/learn/querying/real-time/changefeeds) |
| Security, permissions, and capability gates | SurrealDB, [Security](https://surrealdb.com/docs/learn/security), [Permissions](https://surrealdb.com/docs/learn/security/authorization/permissions-and-row-level-security), and [Capabilities](https://surrealdb.com/docs/learn/security/authorization/capabilities) |
| Extensions and stored execution | SurrealDB, [Extensions](https://surrealdb.com/docs/learn/extensions) and [Surrealism plugins](https://surrealdb.com/docs/learn/extensions/plugins/overview) |
| Protocol, SDK, and MCP boundary | SurrealDB, [RPC protocol](https://surrealdb.com/docs/reference/rest-api/rpc-protocol), [SDK execution](https://surrealdb.com/docs/learn/querying/surrealql/executing-queries/via-sdks), and pinned MCP source above |
| Agent setup and repository-memory examples | SurrealDB, [Agent setup](https://surrealdb.com/docs/agents) and [coding-agent project memory](https://surrealdb.com/docs/agent-memory/cookbooks/build/coding-agent-with-project-memory) |
| Operations, observability, and UI | SurrealDB, [Observability](https://surrealdb.com/docs/manage/observability), [Self-hosted monitoring and observability](https://surrealdb.com/docs/manage/self-hosted/monitoring-and-observability), and [Surrealist concepts](https://surrealdb.com/docs/explore/surrealist/concepts/overview) |

The research stopped after every consequential stable capability family and
failure case had a first-party or commit-pinned source, every retained pattern
had one RRFlow owner and proof gate, and incompatible authority assumptions
were explicit. Further SurrealDB feature enumeration would not change the
immediate dependency order: finish canonical storage and transactions, add
native access paths, stream stamped Arrow pages through rrflowQL/DataFusion,
then persist reasoning/context feedback and qualify installation and delivery.
