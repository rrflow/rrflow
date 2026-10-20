# RRFlow engine data flow

**Status:** active accepted target architecture; implementation gaps remain open
**Coordinate:** `rrflow://rrflow-instance/data/architecture/engine-data-flow`
**Owner:** detailed transactional, storage, query, context, and automation flow

The repository root [README](../../README.md) owns RRFlow's identity and
non-negotiable invariants. The [system overview](system-overview.md) owns the
master component and security-boundary map, the
[alpha objective](../objectives/rrflow-1.0-alpha.md) defines the result, the
[roadmap](../roadmap/rrflow-1.0.md) orders delivery, and the
[POA&M](../poam/rrflow-1.0-alpha.md) tracks observed gaps. This record owns the
detailed end-to-end flow only.

The [adaptive-reasoning decision](../decisions/0002-adaptive-reasoning-governed-effects.md)
applies throughout this flow. Reading, searching, comparing, choosing
provisional context, and forming hypotheses may be iterative and incompletely
grounded. The closed paths below begin when RRFlow promises a durable mutation,
accepted external effect, resumable state, deterministic replay, or evidence
claim. They govern observable effects, not a model's private reasoning.

## System boundary

RRFlow is one hybrid transactional and analytical reasoning-data engine. The
names describe cooperating layers, not independent products or stores:

| Layer | Responsibility |
|---|---|
| rrflowDB | One persistent AI estate containing governed knowledge, temporal graph state, reasoning state, evidence, and indexes. |
| `RrdEngine` | Sole semantic transaction coordinator: authenticates, authorizes, binds read stamps, validates mutations and model proposals, and invokes the selected storage profile. |
| rrflowKV | Durable physical profile: WAL, sequence allocation, MVCC snapshots, mutable memtable, immutable segments, manifests, flush, compaction, and recovery. |
| rrflowMX | Non-durable physical profile implementing the same semantic storage port for volatile and conformance execution. |
| rrflowQL | Query language and planning boundary selecting native fast operators or the analytical path. |
| Arrow substrate | Shared columnar buffer model used by eligible immutable segment pages and streamed analytical batches. |
| DataFusion | Bounded vectorized execution over stamped Arrow batches; it does not authorize or commit RRFlow state. |
| Native indexes | Scalar/unique, graph adjacency, BM25, exact-vector, HNSW, and related access paths bound to canonical source coordinates. |
| LFG or provider model | Replaceable constrained adapter that proposes recipes, branches, or bounded query intent. |
| HTTP, WebSocket, SDK, MCP, CLI, Connectome | Transport and client surfaces invoking public `RrdEngine` capabilities without recreating engine logic. |

## Write and commit flow

```text
HTTP / WebSocket / SDK / MCP / embedded caller
                         |
                  typed public request
                         |
                         v
                     RrdEngine
       parse -> authenticate -> authorize -> bind schema/policy
                         |
              optional analytical transform
                         |
            rrflowQL/DataFusion returns batches
              or model returns a proposal
                         |
                         v
                     RrdEngine
       validate -> lower semantic mutation -> authorize effects
                         |
                         v
                      rrflowKV
       WAL frame -> sequence/MVCC memtable -> commit receipt
                         |
              flush and compaction builders
                         |
                         v
       immutable segment + index roots + manifest publication
                         |
                         v
             changefeed / live delta / trace
```

`RrdEngine` owns the semantic transaction and authorization decision. rrflowKV
owns WAL concurrency, sequence allocation, physical snapshot isolation, flush,
compaction, and durable publication. `RrdEngine` must not hold a global WAL
lock or expose physical keys to callers.

rrflowQL parses and binds expressions. DataFusion may scan, filter, join,
aggregate, rank, or produce transformed Arrow batches. A DataFusion `DataSink`
or another compute adapter returns a proposed result to `RrdEngine`; it does
not write rrflowKV files, publish manifests, or bypass semantic validation.

One accepted semantic transaction must publish its canonical record changes,
temporal versions, both graph adjacency directions, synchronous index changes,
runtime-log entry, and durable asynchronous index deltas as one rrflowKV write
batch. Index builders may run later, but their source cursor and freshness
state are part of the committed transaction.

### Current runtime-log and graph oracle

The current semantic storage port exposes a bounded, authenticated runtime-log
page containing `requested_after`, `through_cursor`, `head_cursor`, validation
evidence, and matching changes. A consumer always resumes at
`through_cursor`, even when its scope filter matched no changes, because the
cursor records the examined global prefix rather than the number of returned
rows. This behavior is tested on rrflowMX and rrflowKV, including rrflowKV
flush, close, reopen, and continued hash chaining.

`RuntimeGraphSnapshot::from_changes` is the current temporal graph oracle. It
folds transaction-visible record, relation, event, and retirement mutations at
one scope, valid-time instant, and known cursor. Each event has a stable
cursor-derived node identity; a subject-bearing event also produces a
deterministic `emitted` relation from its subject to that event. The snapshot
supports outgoing and incoming relation views, bounded relation-filtered
breadth-first traversal, and an exact structural differential between two
known cursors. rrflowQL additionally exposes deterministic directed traversal
with cycle suppression and the first shortest path to each reached node.

These are semantic reference results, not the target physical graph engine.
Retained runtime changes can still construct the explicit test, diagnostic, and
structural-differential oracle. Accepted C-03 atomically maintains direct
versions and both adjacency directions, while accepted C-04 serves normal
current and temporal selection from authenticated semantic-version keys rather
than rebuilding the log. rrflowQL traversal now selects the native adjacency
reader: it reads each start identity, then per breadth-first level the
frontier's temporal adjacency ranges and the identity history of every relation
they name, and matches an independent full-replay oracle. Context graph
expansion and memory-estate seat planning still materialize a scope snapshot and
remain Gate E-01 work. A structural differential is neither a committed mutation
nor a live query result until its owning `RrdEngine` operation validates and
commits the corresponding proposal.

## rrflowKV physical target

rrflowKV combines write-optimized state with scan-optimized immutable storage
inside one durability authority:

```text
recent writes                         published immutable state
-------------                         -------------------------
checksummed WAL                       manifest generation
mutable MVCC memtable       flush     ordered key/version spine
point/range/CAS access       ----->    Arrow-compatible column pages
                                      adjacency/posting/index roots
                                      encoding and checksum metadata
```

The ordered key/version spine preserves exact point, prefix, range, temporal,
and conflict semantics. Arrow-compatible pages allow selected immutable
columns to enter `RecordBatch` streams without reconstructing generic Rust row
objects. The same manifest pins both forms so they cannot represent different
commits.

Column pages may contain canonical record values. Scalar, lexical, graph, and
approximate-vector indexes remain derived acceleration state and cannot
outrank their source keys, schema, catalogue, or read cursor. Exact vector
values remain canonical; HNSW and quantized structures remain candidate
indexes followed by exact reranking when the query contract requires it.

The mutable memtable does not need to mimic a columnar file. It is optimized
for high-frequency writes, current state, CAS, and bounded range reads. A
stamped analytical scan merges the visible memtable delta with immutable
segment pages before returning a batch.

The completed C-06a through C-06f slices reached the common immutable-page
foundation. Its
[current physical-format reference](../reference/storage/rrflowkv-current-format.md)
records segment v6's ordered spine, six Arrow-layout buffers, authenticated
persisted row-group filters, authenticated none/adaptive-LZ4 writer policy,
format identities, page ownership/copy/decompression counters, frozen bytes,
and explicit rejection of earlier segments. Validated row/byte
targets now govern new flush
and compaction outputs and remain authenticated per segment so older
generations are self-describing when the writer configuration changes. A
generated mixed-family MVCC corpus now proves exact point/range behavior through
reopen and protected compaction and rejects bounded malformed physical bytes.
C-06g adds the first storage-facing projected stream:

```text
validated ranges + projection + snapshot + budgets
    -> capture sequence + manifest + Arc memtable + eligible Arc segments
    -> acquire bounded active-manifest lease; release Database borrow
    -> key/sequence spine cursors + minimum-key heap
    -> greatest visible version; equal sequence across runs fails closed
    -> winning validity page; suppress winning tombstone
    -> optional winning value offsets/data
    -> bounded Arrow-compatible offset/data output batch
    -> completed | cancelled | failed; release lease exactly once
```

Future writes use copy-on-write when the captured memtable is shared. Flush and
compaction may publish a later manifest while the stream continues over its
owned generation. Garbage collection treats every active manifest as a root,
so it cannot unlink that generation until completion, cancellation, failure,
or drop releases the lease. Creation bounds active views, selected runs, and
pinned bytes. Iteration bounds versions, page requests/logical bytes, rows,
output buffers, batch rows, and batch allocation. Query evidence records page
families, cache work, actual I/O backend, physical bytes, ownership, decode,
allocation, copy, output, and terminal outcome. Immutable rrflowKV-open
evidence separately records whole-file segment validation and startup
checkpoint reconciliation; none of these diagnostics becomes database truth.

This is a synchronous physical storage stream, not the F-01 DataFusion
provider. It emits owned general-MVCC merge batches; it does not claim an Arrow
`RecordBatch`, asynchronous backpressure, predicate/limit pushdown, or end-to-
end zero-copy. C-06h adds finite adversarial/property/fuzz qualification.
Segment v6 authenticates each row group's persisted membership filter and its
writer/page codec policy. Normal reopen parses metadata with zero semantic-page
reads, definite point misses can skip key pages, raw mmap pages remain
borrow-eligible, and selected LZ4 pages decode into exact-length aligned owners
after stored-byte authentication. Positive filter answers still execute exact
MVCC lookup. None/adaptive histories match through reopen and protected
compaction. C-06j adds the sole process-local physical page cache below that
stream. Its default family-neutral probationary/protected LRU promotes reuse
only across engine operations; one opaque scope per projected stream suppresses
promotion caused by repeated page touches inside the scan. Exact LRU remains a
selectable oracle/operator policy. The clean eight-family production-reader
differential preserves snapshot, manifest, semantic, and projected-row identity
while reducing post-scan hot-page loads from 48 to zero under the same exact
byte capacity. C-06 is accepted; C-07 retains concurrency, recovery,
maintenance, and installed-path lifetime qualification.

## Conditional zero-copy

Memory mapping alone is not a universal zero-copy guarantee. A segment page
may be borrowed directly by Arrow only when its physical type, alignment,
endianness, compression state, validity representation, deletion state, and
requested projection permit it. The snapshot must retain the segment mapping
for the complete lifetime of every emitted batch.

Dictionary materialization, decompression, deletion filtering, type coercion,
memtable merging, computed expressions, and incompatible alignment may require
pool-owned buffers. Every physical plan therefore reports:

- mapped and physically read bytes;
- borrowed Arrow buffer bytes;
- decompressed and decoded bytes;
- copied and newly allocated bytes;
- rows/pages eliminated by key, projection, predicate, and limit pushdown; and
- peak memory, spill bytes, elapsed time, and output bytes.

The release claim is measured conditional zero-copy for eligible pages, never
"zero parsing, zero allocation" for every query. Apache Arrow likewise notes
that IPC reads can allocate when compression or other conditions require it.

## Evidence-gated physical and recall mechanisms

RRFlow adopts mechanisms by observed behavior at one canonical boundary, not
by importing an upstream product topology or treating a paper result as a
default. The following mechanisms retain their stated integration status and
remain subordinate to the exact transaction, stamp, authorization, and
fallback contracts above.

| Mechanism | Canonical RRFlow use | Required decision evidence |
|---|---|---|
| WiscKey-style key/value separation | Rejected for the accepted segment-v6 format; canonical values remain stationary in authenticated segment pages | The current byte-only model cannot prove atomic pointer publication, snapshot reachability, recovery, corruption denial, range reads, orphan reclamation, or value-log garbage collection. A future reconsideration requires a new roadmap package and complete comparative proof; it is not latent architecture. |
| Adaptive page compression | Integrated in segment v6 as authenticated none/adaptive-LZ4 writer policy; each page independently stays raw unless the checked LZ4 block meets the configured saving threshold | The retained slice proves exact semantics, malformed decode rejection, raw mmap borrowing, owned aligned decode, reopen/compaction behavior, and one-host byte reduction. F separately measures point/range/DataFusion latency. Compression makes a page ineligible for direct mapped borrowing until decoded. |
| Scan-resistant immutable-page cache | Integrated below rrflowKV projected reads as a configurable exact-byte cache; default probationary/protected LRU is scope-aware and exact LRU remains a selectable oracle/operator policy | The production-reader differential mixes all eight current physical families, proves identical durable/semantic results and exact capacity, and reduces post-scan hot-page loads from 48 to zero. C-07 still owns lock contention, duplicate-load coalescing, sustained concurrency, and reader-lifetime qualification; F-05 owns stamp-safe query/result caches above this physical cache. |
| Partitioned Elias-Fano and PFOR | Candidate authenticated codecs for immutable BM25 DocID, frequency, and position partitions | Exact seek/iteration must match the raw posting oracle across sparse, dense, clustered, update, tombstone, compaction, corruption, and reopen corpora. Select per partition from measured bytes and decode work; neither codec is globally mandated. |
| TurboQuant_prod | Candidate vector projection using an MSE quantizer plus a one-bit QJL residual to estimate inner products; exact canonical vectors remain authoritative | Measure estimator error and bias, recall@k, filtered recall, exact-rerank work, RSS, build/update/compaction cost, hardware paths, stale/corrupt behavior, and reopen on declared RRFlow embedding corpora. No universal quality-neutral or scale claim is allowed. |
| LSM-VEC-style disk graph organization | Later candidate for immutable disk-resident ANN navigation after the exact/HNSW generation and delta overlay are correct | Compare recall, tail latency, random I/O, memory residency, construction/merge amplification, update/delete behavior, and recovery against the accepted exact/HNSW baseline. It is not required merely because rrflowKV is an LSM. |

These mechanisms do not create separate databases. Graph adjacency, BM25
postings, canonical vectors, approximate generations, reasoning state, and
governance evidence remain typed families inside one rrflowDB estate and one
`RrdEngine` transaction/read-stamp authority. rrflowQL chooses eligible access
paths; DataFusion consumes streams and computes but never publishes canonical
state.

Working, semantic, and episodic memory are policy classifications over those
records, not independent keyspaces or storage authorities. Access frequency,
age, outcome evidence, provenance, legal/retention holds, and project policy
may inform a versioned priority or retirement proposal. Only an authorized
`RrdEngine` operation can accept that proposal. Compaction may reclaim a
version only after accepted retention state makes it unreachable; a
compaction filter cannot decide that a memory is semantically false or
deletable from an Ebbinghaus score alone.

A prompt or reasoning-round boundary is likewise not a durability command.
Authoritative WAL acknowledgment follows the declared durability policy;
ordinary bounded maintenance decides flush/compaction. Committed projection
deltas schedule incremental BM25/vector/graph work. No request forces a full
flush, global term-statistics rebuild, vector retraining, or graph rewrite.
DataFusion planning declares projection/filter/limit support without storage
I/O; actual page reads begin when its execution stream is polled, and batch
size comes from the request memory/work budget rather than a hardcoded row
count.

## Read and query flow

```text
authorized intent + anchors + explicit budgets
                         |
                  capture one ReadStamp
 runtime cursor + manifest + valid time + schema/catalogue/policy
                         |
                    rrflowQL planner
              +----------+-----------+
              |                      |
              v                      v
       native fast path       analytical path
   point/range/CAS/tree       native candidate operators
   adjacency prefix scans     graph + BM25 + HNSW/exact
              |                      |
              |          rrflowKV stamped page/memtable scan
              |                      |
              |             Arrow RecordBatch stream
              |                      |
              |           DataFusion bounded execution
              +-----------+----------+
                          |
                deterministic exact rerank/RRF
                          |
             ContextPacket + evidence + same stamp
```

Fast state-machine navigation and bounded graph pointer work do not invoke
DataFusion. Analytical scans use a custom provider whose planning methods
declare supported pushdown without performing storage I/O; its execution stream
performs bounded reads when polled. Pipeline-breaking operators must reserve
memory or spill within the request budget.

Graph traversal uses versioned outgoing/incoming adjacency prefix ranges.
BM25 uses incrementally maintained dictionary, document statistics, postings,
positions, and tombstones. Vector search uses payload/scalar filters to narrow
eligible candidates, HNSW or exact candidate generation according to the cost
plan, and exact reranking. Reciprocal-rank fusion is a pure query-time operator;
verified outcomes may update a versioned policy only for later read stamps.

The local routing model does not read raw Arrow or storage buffers as model
input. `RrdEngine` constructs a bounded, typed route packet or an explicit
Arrow-to-tensor feature bridge. The model returns a grammar-constrained
proposal; deterministic predicates, authorization, CAS, and persistence remain
engine operations.

Before a router model can be loaded, `rrd-inference` applies this admission
chain:

```text
closed RouterModelManifest + RouterBackendDescriptor
                         |
 independent runtime-observed RouterModelHandshake
                         |
 exact model + tokenizer + runtime + grammar bytes
                         v
 declaration, length, digest, schema, ABI, device, limit,
 capability, quantization, and grammar validation
                         |
              opaque load admission token
                         v
              future G-01 RouterBackend loader
```

The manifest is location-free: installation/adapters may resolve immutable
bytes, but cannot put a provider, filesystem path, URL, endpoint, credential,
or secret into this engine contract. An embedded tokenizer must still be
exposed as exact logical tokenizer bytes for independent verification. The
handshake repeats the runtime observation deliberately; copying a manifest is
not evidence that the currently selected process/runtime still matches it.
This B-03 boundary validates load admission only. G-01 must make executable
router loaders consume the opaque admission and enforce the admitted resource
limits; it must not turn the admission into authorization or persistence.

## Seat attribution and routing flow

```text
provider credential or local principal
                 |
        authenticate exact identity
                 |
 capture one policy/data ReadStamp
                 |
 resolve visible rrflow-represents edge
                 |
 durable seat attribution (for example Clyffy)
                 |
 authorize requested semantic operation
                 |
 RrdEngine builds bounded route packet
                 |
 RouterBackend returns one proposal
 select_recipe | advance_branch | request_context
                 |
 RrdEngine validates cursor, policy, fields, and budget
          +------+------+
          |             |
   fast-path CAS   semantic context intent
          |             |
          +------+------+
                 |
 atomic state + attribution + audit + trace commit
```

The authenticated identity, visible representation edge, resolved seat,
authorization digest, reasoning cursor, route input digest, proposal digest,
and commit receipt belong to one causal operation. Representation answers
"which durable seat is this principal acting for?" It does not grant the
operation. Ordinary policy can still deny the represented seat, and an actor
label supplied by a caller is never identity evidence.

Clyffy is the primary seat specialization for this repository, not a routing
model or a parallel orchestration runtime. A provider-neutral `RouterBackend`
can propose only the three decisions above. It cannot evaluate protected edge
conditions, select physical keys or indexes, invoke DataFusion directly,
authorize itself, or write state. A `request_context` proposal becomes the same
stamped semantic rrflowQL/context request available to other public clients;
`RrdEngine` and the planner select native graph, BM25, vector, or analytical
access.

The checkout currently persists and reopens seat/provider/representation
records and resolves record warps through context assembly. It freezes the
three router proposal shapes plus the exact manifest/runtime/byte admission
chain above, and proves rejected admission cannot call a model loader. It does
not yet install/select that manifest, authenticate a provider identity into
the representation graph, bind seat attribution into the route packet,
dispatch a `RouterBackend`, enforce runtime resources during execution, or
atomically persist routed tree state with attribution and audit evidence.
Those are C-03, D-01, D-05, G-01 through G-05, H-04, and H-05 work, not
implemented flow.

## Context assembly contract

The provider-neutral operation is `context-assemble`, served at
`POST /v1/context/assemble`. `rrd-contract` owns `AssembleContext`, the plan
and evidence types, and `ContextPacket`; `RrdEngine::assemble_context` owns the
implementation. The Rust client, RRD HTTP handler, CLI, MCP adapter, and any
embedded caller must invoke that operation or the same engine method. A host
adapter may translate its provider's request lifecycle at the edge, but it
cannot add a provider-specific context endpoint, retrieval pipeline, state
store, or lifecycle authority.

For one request, `RrdEngine` authenticates and authorizes the operation and
captures one runtime manifest, commit cursor, optional schema revision,
catalogue revision, and valid-time coordinate. The caller supplies intent, not
physical topology:

- one scope and query;
- one explicit nonzero valid-time coordinate;
- optional canonical record anchors; and
- optional versioned semantic constraints, preferences, and capability
  extensions understood by the installed contract; and
- explicit graph-depth, returned-item, output-byte, and scanned-change
  budgets.

The caller may name semantic subjects, fields, relations, collections, or
representations when the public capability supports that intent. It cannot
command rrflowKV keys, a physical index, a DataFusion node, an embedding
provider, or another execution shortcut. `RrdEngine` validates the semantic
request against the captured catalogue and selects eligible text, claims,
vector sources, graph roots, models, and physical paths under policy and
budgets. An unknown extension is rejected or reported unsupported at the
durable boundary; it is never coerced into a different effect.

The closed request contract enforces these hard ceilings:

| Resource | Maximum |
|---|---:|
| Query bytes | 64 KiB |
| Seed records | 256 |
| Returned items | 512 |
| Encoded item bytes | 768 KiB |
| Scanned runtime changes | 1,000,000 |
| Graph depth | 32 |

Within those ceilings, lexical hits are capped to `max_items`; compatible
vector sources and the exact-scored hits from each source are each capped to
`max_items`; graph traversal is capped to `max_items * 16` edge steps; and the
final fused result is capped by both `max_items` and `max_output_bytes`. Any
retrieval or output cap that hides remaining work sets `truncated=true`; a
truncated packet is never represented as complete.

Every returned item carries one or more evidence entries with the source kind,
source identifier, rank, source score, reciprocal-rank contribution, physical
plan digest, and evidence digest. The packet carries the read stamp, query
digest, exact encoded-item byte count, truncation state, and packet digest. Its
plan records all five canonical stages—seed, lexical, semantic, graph, and
fusion—in order, including whether each was selected or skipped, the selected
access path, exactness, reason, decision digest, compiled security-policy
revision, and authorization digest. The plan and packet must validate against
the same read stamp.

With the same validated request, authorization state, persisted read
coordinate, catalogue, and installed deterministic model set, assembly order,
fusion, evidence, and packet content are deterministic. Current physical paths
are snapshot BM25, exact snapshot-vector scoring, bidirectional snapshot graph
BFS, and reciprocal-rank fusion. Roadmap Gates E, F, and H replace their
whole-snapshot costs with native incremental access paths without changing the
public operation or its single-stamp semantics.

## Prompt-flight flow

A prompt flight is the durable, comparable record of one prompt crossing the
runtime boundary. `rrd-contract` owns `LaunchPromptFlight`, `PromptFlight`,
and the list/read types; `RrdEngine::launch_prompt_flight`,
`list_prompt_flights`, and `read_prompt_flight` own the implementation behind
`flight-launch` (`POST /v1/flights`, `flight_launch` grant, mutating) and
`flight-list`/`flight-read` (`POST /v1/flights/list` and `/v1/flights/read`,
`flight_read` grant). Clients render flights; none keeps a flight ledger,
runner, or diagnostics protocol of its own.

One launch runs inside one authenticated read stamp:

1. Authorize `flight_launch` and enforce the estate recall ceilings on the
   flight's declared context budget.
2. Derive the flight identity from instance, session, and idempotency key.
   A stamped identity read returns an existing flight as an exact replay; the
   same key with a different operation digest conflicts.
3. `fresh` injects no context. `pruned` calls the same engine
   `context-assemble` path every client uses; a packet whose stamp differs
   from the flight stamp restarts the attempt rather than mixing two stamps.
4. Append ordered stage events (`prompt`, `context`, `outcome`). The context
   event carries the five plan-stage decisions and the plan/packet digests,
   not a second copy of the packet items.
5. Commit one cursor-CAS runtime commit: the strict table schema on first
   use (the edge table's target kinds grow with each record kind a flight
   uses), one `rrflow-prompt-flight` lifecycle record, one
   `rrflow-prompt-flight-used` edge per context item that is a canonical
   record (rank, fused score, and contributing avenues; claims are not graph
   nodes), and its `rrflow-prompt-flight-event` lifecycle events. A lost
   cursor race retries at most three times, then reports a retryable
   conflict.

The `observe` provider prepares the packet inside the engine and launches
nothing, so the flight completes as `prepared`. Model-provider arms are
explicit opt-in adapters; each enters the closed provider, stage, and status
vocabularies only together with its governed adapter, budget, and audit
evidence.

A read names the flight's record by identity, then, at the same stamp, each
edge by its deterministic identity (`<flight-id>.<rank>`) and each event by
its log cursor, so it costs the flight's own size and never the estate's.
The list is a stamped read of the flight record kind and still grows with the
number of flights until a cursor-ordered access path exists (Gate E). Neither
replays the runtime log. Flight rows, edges, and events are ordinary temporal
data, visible to data snapshots, changefeeds, and the diagnostic graph lens,
and rrflowQL walks them in both directions:
`FROM traverse:rrflow-prompt-flight-used START rrflow-prompt-flight:<id>
DIRECTION OUTGOING DEPTH 1 …` returns the records a prompt used, and
`… START <kind>:<id> DIRECTION INCOMING …` returns every prompt that used a
record. A flight records only externally observable
runtime events and never hidden chain-of-thought.

## Task flow

A task is a named, durable record of one unit of intended work: a title, a
goal, and a status that prompt flights join. `rrd-contract` owns `OpenTask`,
`CloseTask`, `TaskRecord`, and the list/read types; `RrdEngine::open_task`,
`close_task`, `list_tasks`, and `read_task` own the implementation behind
`task-open` and `task-close` (`POST /v1/tasks` and `/v1/tasks/close`,
`task_write` grant, mutating) and `task-list`/`task-read`
(`POST /v1/tasks/list` and `/v1/tasks/read`, `task_read` grant). A task
orchestrates nothing; it is not a routine.

1. `task-open` derives the task identity from instance, session, and
   idempotency key. A stamped identity read returns an existing task as an
   exact replay; the same key with a different operation digest conflicts.
   One cursor-CAS commit installs the strict `rrflow-task` table on first use
   and writes the task's single open version.
2. `task-close` writes the second and final version, `completed` or
   `abandoned`, with its close time and optional outcome. Closing again with
   the same status and outcome replays; any other close of a closed task fails
   as a failed precondition.
3. A flight launched with `task_id` checks, at its own stamp, that the task
   exists and is open, then writes one `rrflow-task-includes` edge from the
   task to the flight in the flight's own commit, after its events. The edge
   identity is the flight identity, so a flight joins at most one task; the
   edge table declares no pair or degree constraint
   ([POAM-034](../poam/rrflow-1.0-alpha/poam-034.md)).

A task read names the row by identity and costs its one or two versions. Its
flights are graph neighbours rather than a second list:
`FROM traverse:rrflow-task-includes START rrflow-task:<id> DIRECTION OUTGOING
DEPTH 1 …` returns them, and each flight's `rrflow-prompt-flight-used` edges
return the records it used. The task list is a stamped read of the task
record kind and grows with the number of tasks until a cursor-ordered access
path exists (Gate E); traversal reconstructs the graph from relations until
E-01.

## Trace and observability flow

RRFlow has one causal operation graph with two representations. Durable trace
events are immutable governed evidence in rrflowDB. Process-local Rust
`tracing` spans and optional OpenTelemetry export are diagnostic projections
of that same operation. A sampled, dropped, or unavailable diagnostic span
cannot establish that a transaction, attunement phase, projection, trigger, or
routine completed; only its authoritative state and commit receipt can.

| Signal | Canonical use | Cannot establish |
|---|---|---|
| Engine event | Immutable committed occurrence eligible for trigger evaluation. | Completion before its enclosing commit receipt exists. |
| Durable trace event | Replayable causal evidence for a bounded operation or stage. | Job/routine state or permission to mutate. |
| Audit record | Authenticated authorization, denial, and mutation accountability. | Query performance or workflow progress by itself. |
| Physical counter/metric | Numeric work, resource, latency, quality, and failure measurement derived from named operations. | Canonical record/index contents or a completed effect. |
| Diagnostic span/log | Process-local or exported troubleshooting view correlated to durable coordinates. | Any durable truth when sampled, dropped, or unavailable. |

At HTTP, WebSocket, SDK, MCP, CLI, embedded, and adapter ingress, RRD validates
and continues W3C `traceparent`/`tracestate` context or creates a new trace.
`RrdEngine` binds it to the request correlation and idempotency coordinates,
authenticated actor, estate/scope, authorization decision, `ReadStamp`, plan,
projection, reasoning cursor, source evidence, and any resulting commit.
Synchronous child work retains parentage. Work caused later by a committed
event, projection delta, retry, or routine activity uses a typed causal link
rather than false synchronous parentage.

### Canonical operation catalogue

Machine operation names are closed, low-cardinality values of the form
`rrflow.<boundary>.<operation>`. `rrd-core::TraceOperation` is the source-level
catalogue and `TraceBoundary` is the persisted boundary. A canonical event is
invalid when those two disagree. Dynamic record, scope, project, route, query,
provider, model, file, index generation, error, or result values never enter
the name. Adding an operation requires an explicit contract and owning-gate
change; a caller cannot manufacture one from request data.

| Boundary | Exact operations reserved by `TraceOperation` | Owning implementation gates |
|---|---|---|
| `ingress` | `rrflow.ingress.request`, `rrflow.ingress.frame` | B-04, H-04, H-05 |
| `engine` | `rrflow.engine.operation`, `rrflow.engine.authenticate`, `rrflow.engine.authorize`, `rrflow.engine.validate`, `rrflow.engine.commit`, `rrflow.engine.context_assemble`, `rrflow.engine.function_execute` | C-02/C-03, G-04/G-05, H-01/H-05, I-02 |
| `kv` | `rrflow.kv.point_read`, `rrflow.kv.range_scan`, `rrflow.kv.write_batch`, `rrflow.kv.wal_append`, `rrflow.kv.snapshot`, `rrflow.kv.page_scan`, `rrflow.kv.flush`, `rrflow.kv.compact`, `rrflow.kv.recover` | C-01 through C-07, F-01/F-04 |
| `ql` | `rrflow.ql.parse`, `rrflow.ql.bind`, `rrflow.ql.plan`, `rrflow.ql.execute` | B-05, E-05, F-01 through F-04 |
| `graph` | `rrflow.graph.maintain`, `rrflow.graph.traverse` | C-03, E-01, F-03, H-01 |
| `lexical` | `rrflow.lexical.maintain`, `rrflow.lexical.search` | C-03, E-03, F-03, H-01 |
| `vector` | `rrflow.vector.maintain`, `rrflow.vector.search`, `rrflow.vector.rerank`, `rrflow.vector.projection` | C-03, E-04/E-05, F-03, H-01 |
| `datafusion` | `rrflow.datafusion.plan`, `rrflow.datafusion.scan`, `rrflow.datafusion.execute`, `rrflow.datafusion.spill` | F-01 through F-04 |
| `inference` | `rrflow.inference.embed`, `rrflow.inference.route` | D-05, G-01 through G-05 |
| `attunement` | `rrflow.attunement.job`, `rrflow.attunement.phase` | D-01 through D-06 |
| `routine` | `rrflow.routine.trigger`, `rrflow.routine.activation`, `rrflow.routine.step`, `rrflow.routine.activity`, `rrflow.routine.compensation` | I-01 through I-05 |
| `adapter` | `rrflow.adapter.resolve`, `rrflow.adapter.invoke`, `rrflow.adapter.synchronize`, `rrflow.adapter.transfer` | D-06, H-04/H-07, I-05/I-06 |
| `delivery` | `rrflow.delivery.connect`, `rrflow.delivery.publish`, `rrflow.delivery.acknowledge`, `rrflow.delivery.heartbeat`, `rrflow.delivery.resume`, `rrflow.delivery.close` | B-04, H-03 through H-05 |

Functions are engine-governed deterministic compute and therefore use
`rrflow.engine.function_execute`; they do not create a `function` runtime or
trace boundary. The exact function identity/revision is a resource or source
link. Attunement phase names, routine step kinds, adapter names, and delivery
subscription identities are likewise values, not operation-name suffixes.

### Canonical causal links and attributes

Typed links carry identity, revision, and causation. They are not unstructured
labels and they cannot grant authority:

| Evidence | Canonical `TraceLink` | Required coordinates |
|---|---|---|
| Request correlation | `Request` | request and public operation IDs, optional parent request ID, and only the digest of an idempotency key |
| Actor and estate scope | `ActorScope` | authenticated/attributed actor identity and exact `ScopeId`; a caller label alone is insufficient |
| Authorization | `Authorization` | allow/deny decision, policy revision, and authorization-evidence digest |
| Read snapshot | `Read` | complete validated `ReadStamp`, including schema/catalogue revision, commit cursor, head digest, and manifest identity |
| Runtime/snapshot position | `RuntimeCursor`, `Snapshot` | examined runtime cursor or immutable snapshot identity plus cursor |
| Logical/physical plan | `Plan` | canonical plan digest; selected/rejected paths remain bounded attributes |
| Derived index generation | `Projection` | complete validated `ProjectionStamp` and source cursor |
| Reasoning position | `ReasoningCursor` | tree/cursor identity, tree revision, node, step, and bound read stamp |
| Canonical or external input | `Source` | source kind, stable source identity, and exact revision; adapter/provider names stay separate from source authority |
| Durable result | `Commit` | commit digest and ordered nonzero first/last cursors |
| Target capability/data | `Resource` | resource kind and stable identity, never executable bytes or a secret-bearing locator |
| Asynchronous causation | `CausalSpan` | originating trace/span identity plus `follows_from`, `retry_of`, or `resumes`; it never fakes synchronous parentage |
| Returned evidence | `Output` | canonical output digest, item count, and encoded byte count |

`Workflow`, `Provider`, and `OperatorKnowledge` trace-link variants do not
survive this vocabulary freeze. Generic routine causation uses `CausalSpan` and
routine/resource coordinates; model or tool execution uses `Resource` plus
`Source`; project databases use `Source` through an adapter. There is no
provider, workflow, or external-database trace authority.

The `attributes` map is for bounded measurements and decisions, not identities
already represented by links. `rrd-core::TraceAttribute` is the closed
cross-boundary catalogue. Names are lowercase `snake_case`, fixed at the
instrumenting call site, and may be extended only with the behavior's owning
gate and a corresponding contract test. An arbitrary caller-supplied key is
invalid. The canonical vocabulary is:

| Value class | Canonical attribute names | Required `RuntimeValue` |
|---|---|---|
| Low-cardinality stage/decision/status token | `stage`, `phase`, `action`, `skip_reason`, `cache_status`, `failed_stage`, `error_class`, `verification_status`, `propagation_status` | `String` using a bounded lowercase token, never raw content or an error message |
| Decision flag | `selected`, `exact`, `fallback`, `truncated`, `retryable` | `Bool` |
| Count, byte, duration, token, rank, and cursor measurement | `attempt`, `input_items`, `input_bytes`, `output_items`, `output_bytes`, `keys_examined`, `pages_examined`, `rows_examined`, `graph_steps`, `candidates_examined`, `mapped_bytes`, `read_bytes`, `decoded_bytes`, `decompressed_bytes`, `borrowed_bytes`, `copied_bytes`, `allocated_bytes`, `elapsed_micros`, `memory_peak_bytes`, `spill_bytes`, `input_tokens`, `output_tokens`, `rank`, `selected_source_count`, `skipped_source_count`, `compacted_bytes`, `compacted_tokens`, `start_cursor` | `Unsigned` |
| Score/contribution | `score`, `rrf_contribution` | finite numeric `Decimal` string |
| Protected correlation | `error_digest`, `propagation_digest` | validated SHA-256 `Digest` |

The kernel validates this name-to-value-type map before persistence. Counts use
unsigned integers, durations use microseconds, byte and token units are
explicit, booleans are not encoded as strings, and digests are SHA-256 hex.
Free-form query text, file content/path, prompts, model output, vector values,
credentials, tokens, headers, stack traces, and raw errors are never durable
attributes. Stable public IDs may appear only in their typed link; otherwise a
bounded digest is recorded. `TraceDataClass` controls retention and diagnostic
projection but never relaxes secret exclusion.

Reviewed current producers also have one private, exact
`DIRECT_CONVERGENCE_ATTRIBUTE_NAMES` inventory in `rrd-core`. It admits their
already-emitted physical and subsystem-specific fields while their C-through-I
behavior is replaced. The list can only shrink: new work uses
`TraceAttribute`, and an owning package must move stable identities/revisions
to `TraceLink`, map reusable measurements to the canonical catalogue, or
remove the field. This is neither an extension namespace nor evidence that the
current producer has reached its target gate.

### W3C propagation contract

Ingress behavior is fixed before H-05 implements it:

1. Read `traceparent` case-insensitively and combine `tracestate` fields in
   received order within transport limits. A missing `traceparent` creates a
   fresh RRFlow root and discards an orphan `tracestate`.
2. Validate the complete W3C context before using any part of it. Duplicate,
   malformed, all-zero, unsupported, or oversized context creates a fresh root,
   discards `tracestate`, and records only `propagation_status=invalid` plus an
   input digest. It does not echo raw headers or change application
   authorization; a general header-size violation can still fail at ingress.
3. A valid incoming parent supplies the trace ID and parent span ID. RRFlow
   generates a new nonzero span ID for `rrflow.ingress.request` or
   `rrflow.ingress.frame`; it never reuses the remote parent ID as its own.
4. Synchronous work retains the trace ID and exact parentage. Work caused after
   a commit, queue handoff, retry schedule, projection delta, or routine lease
   starts with no false synchronous parent and records `CausalSpan`.
5. Outgoing adapters replace the W3C parent ID with the current span ID and
   propagate only validated, bounded `tracestate`. Sampling flags affect only
   diagnostic recording/export; they cannot suppress durable evidence or
   advance state.

RRFlow does not persist raw `tracestate`. When correlation requires it, durable
control evidence may retain its digest and validation status. H-05 must prove
valid continuation, missing/invalid context behavior, asynchronous links,
retry parentage, header bounds, and no cross-request context leakage.

### Durable, Rust, and OpenTelemetry mapping

`RuntimeTraceEvent` is the governed evidence form. Rust `tracing` and optional
OpenTelemetry are one-way diagnostic projections:

| Durable field/phase | Rust `tracing` projection | OpenTelemetry projection |
|---|---|---|
| `trace_id`, `span_id`, `parent_span_id` | correlated span fields/context | W3C-conformant `SpanContext` and parent |
| `name`, `boundary` | one static catalogue call-site name plus `rrflow.boundary` | same low-cardinality span name; operation boundary attribute |
| `Start` | create/enter the diagnostic span after the durable start is accepted | start span with available links present at creation |
| `Annotation` | `tracing` event on the correlated span | timestamped span event; never a completion signal |
| `Finish` and `duration_micros` | record terminal fields and close the span | end span; measured duration remains evidence |
| `Ok` | `rrflow.outcome=ok` | status `Ok` |
| `Error` | error level plus digest/class, never raw protected content | status `Error` plus bounded `error.type`/RRFlow digest fields |
| `Denied`, `Cancelled` | explicit outcome at non-error level unless a transport failure also occurred | status `Unset` with RRFlow outcome; expected denial/cancellation is not fabricated as a server fault |
| `CausalSpan` | causal fields/event | OpenTelemetry `Link` created with the span when known |
| Other typed links | structured `rrflow.link.*` fields/events | namespaced attributes/events; they are not converted to OpenTelemetry links because they are not `SpanContext`s |
| Bounded attributes | `rrflow.*` fields under the same redaction policy | RRFlow namespaced attributes plus applicable stable HTTP/database semantic conventions |

Durable start evidence is written before the effect it observes. A required
start that cannot be committed denies the effect. A domain commit remains the
truth if a later finish write fails; the gap is reconciled as incomplete
evidence and cannot roll back, duplicate, or relabel the committed effect.
Durable events are never sampled. Diagnostic spans may be filtered, sampled,
dropped, or unavailable and therefore cannot establish a commit, job/phase,
projection, trigger, routine step, delivery acknowledgement, or verification.

### Runtime modes, build profiles, and build identity

RRFlow has one engine and one semantic feature closure. Diagnostic capability
does not justify a second engine, storage format, executor, lifecycle, or
successful behavior. The build/runtime matrix is:

| Mode | Purpose | Semantic standing |
|---|---|---|
| Cargo `dev` and `test` | Fast iteration, assertions, unit/property/integration tests. | Correctness evidence only; never latency or release evidence. |
| `release` | Optimized candidate and shipped default. Diagnostics default to the bounded normal policy. | Required for release behavior and performance qualification. |
| `diagnostic` | Inherits release optimization and the exact release feature closure while retaining symbols/line tables. Runtime configuration may raise span detail, enable scoped physical counters, and emit a sanitized capture bundle. | May reproduce release behavior only after an automated parity test proves the operation catalogue, build inputs, feature closure, formats, results, receipts, and resource limits equal release. |
| `runtime-analysis` | Diagnostic profile plus Tokio task/resource instrumentation or an attached profiler. | Explicitly non-conformance because extra scheduler/profiler instrumentation can alter timing. |
| `loom`, `miri`, and sanitizer builds | Bounded concurrency-state exploration and memory/undefined-behavior detection. | Verification-only; each tool's supported target and limitations are recorded. |
| `benchmark` | Release-derived fixed-workload binary and harness with diagnostics at the declared level. | Comparison evidence only with the J-04 provenance and latency protocol below. |

The diagnostic profile may change debug information, symbol stripping, and
diagnostic configuration. It cannot enable an alternate query, storage, index,
reasoning, authorization, model, or adapter implementation. Expensive
per-operation counters use runtime-scoped levels (`off`, `normal`, `detailed`,
`profile`) and bounded sampling; the default hot path performs only the
measurements admitted by `normal`. Tokio Console and profiler attachment are
`runtime-analysis`, never a silent property of the release binary.

Every executable and capture reports a machine-readable build identity:

- RRFlow version, source commit and tree digest, and clean/dirty status;
- Rust toolchain, target triple, Cargo profile, panic strategy, and complete
  first-party feature closure;
- executable digest and, for a distribution, bundle/manifest identity;
- public operation-catalogue, schema, ordered-key, WAL, manifest, segment/page,
  and vector/index format identities; and
- sanitized effective-configuration and policy digests.

The version remains `1.0.0` throughout pre-release convergence; the source,
format, and configuration coordinates distinguish builds without inventing
version progress. A dirty or incompletely identified binary may aid local
debugging but cannot produce release or comparison evidence.

### Metric instruments and cardinality

Durable trace attributes describe one governed operation. Metrics aggregate
many operations and therefore use a smaller closed vocabulary. Canonical
instrument names use dotted OpenTelemetry form; a Prometheus exporter may
translate separators but cannot create another catalogue. The initial
catalogue is:

| Instrument | Kind and unit | Meaning |
|---|---|---|
| `rrflow.operation.duration` | Histogram, `s` | Wall duration of one exact operation boundary, recorded once with terminal outcome. |
| `rrflow.operation.count` | Monotonic counter, `{operation}` | Accepted terminal operations; denials, cancellations, and errors remain distinct outcomes. |
| `rrflow.operation.inflight` | Up/down counter, `{operation}` | Currently admitted operations, including queued time until terminal release. |
| `rrflow.queue.depth` | Observable gauge, `{item}` | Bounded executor, maintenance, delivery, model, spill, and telemetry queue occupancy. |
| `rrflow.queue.wait` | Histogram, `s` | Admission-to-execution delay separate from compute time. |
| `rrflow.kv.keys.examined` / `rrflow.kv.pages.examined` | Monotonic counters, `{item}` | Physical KV work attributable to point/range/page operations. |
| `rrflow.kv.bytes` | Monotonic counter, `By` | Read, mapped, decoded, decompressed, borrowed, copied, allocated, WAL, flush, compaction, and spill byte work, distinguished only by bounded `rrflow.work.kind`. |
| `rrflow.kv.cache.requests` | Monotonic counter, `{request}` | Hit, miss, admission rejection, and eviction outcomes for a named bounded cache class. |
| `rrflow.query.rows` | Monotonic counter, `{row}` | Examined and emitted rows for rrflowQL/native/DataFusion execution. |
| `rrflow.query.graph.steps` | Monotonic counter, `{step}` | Traversed eligible graph edges/vertices. |
| `rrflow.query.candidates` | Monotonic counter, `{candidate}` | Lexical, vector, fusion, and exact-rerank candidate work by bounded access path. |
| `rrflow.datafusion.memory.usage` | Observable gauge, `By` | Current DataFusion pool reservation by bounded pool class. |
| `rrflow.datafusion.memory.peak` | Histogram, `By` | Peak pool reservation observed for one terminal query execution. |
| `rrflow.datafusion.spill` | Monotonic counter, `By` | Spill bytes read/written by bounded work kind. |
| `rrflow.context.bytes` | Histogram, `By` | Authorized context input/output, selected, skipped, truncated, and compacted byte volume. |
| `rrflow.context.tokens` | Histogram, `{token}` | Authorized context input/output, selected, skipped, truncated, and compacted token volume where the tokenizer identity is bound to trace/build evidence. |
| `rrflow.delivery.backlog` | Observable gauge, `{item}` | Unacknowledged bounded delivery items by stream class. |
| `rrflow.process.memory` | Observable gauge, `By` | Process RSS and allocator-owned bytes where the platform can identify them. |
| `rrflow.process.cpu.time` | Monotonic counter, `s` | User/system CPU time for saturation and benchmark accounting. |
| `rrflow.telemetry.dropped` / `rrflow.telemetry.export.errors` | Monotonic counters, `{item}` | Diagnostic queue overflow, sampling, encode/export failure, and exporter rejection. |

Every metric point uses only the applicable subset of these attributes:
`rrflow.boundary`, `rrflow.operation`, `rrflow.outcome`,
`rrflow.storage.profile`, `rrflow.access.path`, `rrflow.work.kind`,
`rrflow.queue.kind`, `rrflow.cache.kind`, and bounded `error.type`. Values come
from closed catalogues at the instrumenting site. Estate, project, actor,
session, request, trace, span, query, record, file/path, provider, model,
collection, index generation, routine/job, error message, and user-supplied
values are prohibited metric attributes. Those identities belong in protected
trace links/log fields or build/capture manifests.

The metrics SDK enforces a configurable finite cardinality limit with a default
of 2,000 points per instrument per collection cycle and an explicit overflow
point. Exporters have bounded queues and timeouts. Export backpressure drops
diagnostic data and increments self-telemetry; it never blocks, rolls back, or
relabels authoritative engine work. Telemetry-internal failures are rate-
limited and cannot recursively generate unbounded telemetry.

Latency uses an aggregatable histogram. Exponential histograms are preferred
where the selected OpenTelemetry/Prometheus path preserves them; an explicit-
bucket fallback must cover the declared microsecond-through-minute operating
range and is versioned as diagnostic configuration. Trace exemplars associate
selected samples with trace/span IDs without putting those IDs on every metric
series.

DataFusion's per-operator `elapsed_compute`, output-row, output-batch, and
output-byte values are collected at query completion or cancellation and
attached to the corresponding `rrflow.datafusion.*` diagnostic stage. They do
not replace server wall time, queue wait, storage I/O, or the `RrdEngine` result.
rrflowKV's detailed per-operation counters similarly reset at an admitted
operation boundary and roll into aggregate metrics only after terminal capture;
background flush/compaction work uses its own causal span and operation name.

### Latency measurement contract

No latency statement is valid without naming its boundary. RRFlow uses a
monotonic clock for durations and wall-clock timestamps only for correlation:

| Boundary | Start | Stop |
|---|---|---|
| client end to end | immediately before transport send | after the complete response is received and validated, or terminal transport failure |
| server request | first bounded application-frame/envelope processing | final response byte handed to transport, durable acknowledgement emitted, or terminal denial/error/cancellation |
| engine operation | admitted authenticated operation invocation | typed result/receipt/denial returned to the transport adapter |
| durable commit | commit validation begins | durability policy is satisfied and the commit receipt is constructed, or commit fails |
| query execution | admitted stamped physical plan begins | last batch is consumed, cancellation completes, budget denies, or execution fails |
| stage | named stage begins after its queue wait | stage output/error is handed to its parent |

Server and engine measurements cannot include an unbounded request-body read;
ingress frame admission has its own bounded span. Client latency is the only
claim that includes network and client decoding. Retries expose each attempt
and one logical-call duration; attempt time is not summed and reported as a
single successful server operation.

Operational dashboards separate successful, denied, cancelled, and failed
latency and show traffic, error rate, inflight/queue saturation, resource use,
and p50/p95/p99/p99.9 distributions. An average alone is non-evidence. A
benchmark additionally separates cold/warm cache, rrflowMX/rrflowKV, durability
policy, request class, input-size band, concurrency, and access path. It records
the build identity, configuration, corpus digest, seed, hardware, filesystem,
device, clock source, warm-up, offered-load model, sample count, failures, and
raw histogram. Open-loop scheduling or coordinated-omission correction is
required when a closed-loop driver could hide stalls. Comparative claims use
identical correctness/quality requirements and retain failed samples.

### Diagnostic capture and failure workflow

RRFlow distinguishes an authenticated diagnostic snapshot from observability.
The existing `ReadDiagnosticSnapshot`/`DiagnosticSnapshot` contract is a
bounded, stamped view of canonical and projected engine state for an operator
or Connectome. Observability is the non-authoritative signal and capture system
described here. It may bind a diagnostic-snapshot receipt or invoke that
existing operation through `RrdEngine`; it cannot duplicate the snapshot
schema, replay the runtime log independently, or open repositories/storage to
assemble a second view.

The snapshot also carries one `storage` section with `observation` authority:
the storage port's process-local physical counters for the instance's
profile, measured once after the logical coordinates verify and never gating an
assembly retry. rrflowKV reports its memtable, write-ahead log, flush and
compaction, segment-set, page-cache, and segment I/O counters; rrflowMX reports
`logical_only`. Cumulative counters reset when the process reopens its store,
so they are operator evidence, not state, and are not bound to the read stamp.
The H-05 metric instruments, such as `rrflow.kv.cache.requests`, remain the
observability projection of the same physical behavior; neither is derived
from the other.

Normal logs are structured, bounded, and correlated with trace/span/request
coordinates. Raw prompts, source bodies, query parameters, vector values,
credentials, headers, paths, model output, hidden reasoning, and error messages
are excluded by default. A future protected content-capture capability must be
separately authorized, encrypted, time/size bounded, audited, and absent from
the signed default; `profile` does not imply content capture.

An H-05 diagnostic capture is a manifest-verified bundle containing:

1. the exact build and distribution identity above;
2. sanitized effective configuration/policy digests and declared omissions;
3. the workload, seed, fault point, timing boundaries, and resource budgets;
4. correlated redacted trace/log export and a metric snapshot;
5. rrflowQL logical and physical plan digests, DataFusion plan/operator metrics,
   and rrflowKV logical/I/O/cache/WAL/flush/compaction counters;
6. process CPU, RSS/allocator, file-descriptor, disk-byte, and queue evidence
   available on the target platform; and
7. typed terminal results, receipts, crash/reopen verification, and artifact
   digests without canonical data bodies.

Capture writes through a bounded outward diagnostic sink. It cannot open
storage, query canonical state outside an authorized diagnostic operation,
decide readiness, repair data, advance a job/routine, or prove an effect by its
own presence. OTLP push, Prometheus scrape, JSON log, and local capture-file
support are adapters over the same instruments and redaction policy, disabled
unless explicitly configured. A failed exporter leaves engine state unchanged
and makes the diagnostic omission visible through self-telemetry.

The verification matrix combines complementary lanes:

- deterministic unit/property/differential tests with exact oracles;
- recorded-seed randomized state machines and bounded Loom schedules;
- WAL/manifest/segment/page corruption, torn write, write rejection, ENOSPC,
  fsync uncertainty, process kill, close/reopen, and post-reopen verification;
- Miri and supported Address/Leak/Thread/UndefinedBehavior sanitizer targets;
- sustained mixed OLTP/OLAP/index/maintenance load with memory, queue, and tail-
  latency evidence; and
- optimized diagnostic reproduction plus profiler/Tokio Console analysis when
  ordinary evidence cannot locate the delay.

Every failure artifact binds its input or seed, build identity, target,
configuration, fault schedule, last authoritative stamp/receipt, and reopen
result. A nondeterministic symptom is retained and narrowed; it is never
converted into a passing test by increasing timeouts or deleting assertions.

### Direct-convergence trace inventory

The current runtime still uses the following exact operation names. Kernel
validation admits only this finite inventory in addition to `TraceOperation`;
the list can shrink but cannot grow. Each owning behavior gate must replace
the operation and its evidence together instead of renaming a weak path and
claiming the target exists.

| Current exact names | Canonical destination | Owning gate |
|---|---|---|
| `rrflow.query.run` | `rrflow.engine.operation` | F-01, H-05 |
| `rrflow.query.parse_bind` | separate `rrflow.ql.parse` and `rrflow.ql.bind` evidence | F-01 |
| `rrflow.query.plan` | `rrflow.ql.plan` | E-05, F-01 |
| `rrflow.query.execute` | `rrflow.ql.execute` | F-01 through F-04 |
| `rrflow.storage.runtime_read` | `rrflow.kv.page_scan` or the exact selected KV operation | C-04, F-01 |
| `vector.search`, `vector.execute` | `rrflow.vector.search` | E-04/E-05 |
| `vector.plan` | `rrflow.ql.plan` with vector selection attributes | E-05 |
| `vector.projection.publish`, `vector.quantization.build`, `vector.quantization.activate`, `vector.quantization.retire` | `rrflow.vector.projection` with fixed `action` | E-04/E-05 |
| `embedding.run`, `embedding.infer` | `rrflow.inference.embed` with stage evidence | D-05, G-01 |
| `embedding.commit` | `rrflow.engine.commit` linked to the embedding source/output | C-03, D-05 |
| `operator.knowledge.search`, `operator.knowledge.execute` | `rrflow.adapter.invoke` plus the selected native/vector child work | D-06, H-04 |
| `operator.knowledge.sync`, `operator.knowledge.apply` | `rrflow.adapter.synchronize` | D-06 |
| `cluster.artifact_transfer`, `cluster.artifact_chunk`, `object.replicate` | `rrflow.adapter.transfer` under an engine-owned job/receipt | D-06, H-07 and the future distributed gate |

The process-local `rrd.http.request` span is also noncanonical inventory; H-04
and H-05 replace it with `rrflow.ingress.request`, route-template rather than
raw-path diagnostics, W3C continuation, and the same authorized engine
operation identity. No current direct-store trace helper, mixed operation
name, or passing trace component test claims H-05.

### Instrumentation ownership

Every implementation package adds its evidence with the behavior rather than
waiting for a later observability rewrite:

| Gate wave | Evidence that must arrive in the same package |
|---|---|
| C | engine authorization/validation/commit plus KV point/range, WAL, batch, snapshot, flush, compaction, recovery, conflict, and crash coordinates |
| D | install and attunement job/phase inputs, checkpoint/lease, source-tree work, skip/retry/cancel decision, output digest, and reopen coordinates |
| E | atomic graph/lexical/vector maintenance; plan selection/rejection; traversal/posting/candidate/rerank work; projection freshness and exact fallback |
| F | rrflowQL parse/bind/plan/execute, stamped KV page scan, Arrow batch/copy accounting, DataFusion memory/spill/time/output limits, and cancellation |
| G | model-manifest/resource identity, route input/proposal digests, reasoning cursor, constrained-decode denial, selected path, CAS result, and commit |
| H | context source selection/skips/RRF contributions, W3C ingress/egress, all public surfaces, live delivery/ACK/resume, diagnostic projection, and redaction |
| I | committed-event causation, trigger decision, routine activation/step/activity/compensation, skill/function resource identity, retries, and terminal evidence |

H-05 proves the final chain is complete and export-safe. It does not turn
traces into state or retroactively excuse a C-through-I package that shipped
without its bounded physical and causal evidence.

### Context-path evidence and optimization

Context-path observability is part of the engine evidence model, not a
standalone profiler, provider lifecycle, or Connectome state machine. One
request must correlate its ingress and parent request, actor and scope,
`ReadStamp`, reasoning cursor, plan and projection digests, selected and
skipped context avenues, source identities, ranks and fusion contributions,
cache decisions, model-context compaction, truncation, model/tool attempts,
verification, feedback, and final outcome. Each physical stage reports its
bounded work: keys, pages, rows, graph steps, candidates,
mapped/read/decoded/copied/allocated bytes,
context bytes or tokens when known, latency, spill, and output.

Accepted diagnostics must derive from observable events and persisted engine
coordinates. They identify repeated scans or searches, missed canonical
records, conflicting or duplicate inputs, stale routes and projections,
expensive hops, cache misses/evictions, context lost to model-context
compaction, unused contributions, and outcome regressions. A weak/strong or
before/after comparison uses the same project snapshot, request class,
resource budget, verification rubric, and retained raw evidence. Any inference
is labeled; no view claims access to hidden model chain-of-thought.

Connectome may render the resulting immutable evidence as a timeline, path
graph, resource meter, comparison, or drill-down, but it cannot create missing
events or decide engine state. A future context consolidation or pruning
operation is an explicit, reversible `RrdEngine` proposal against versioned
state. Representative replay must deny the proposal on quality, correctness,
latency, resource, security, legal-retention, or policy regression; configured
operator approval and rollback evidence remain required. A review cadence or
reduction percentage is policy input, never a hardcoded deletion target.

## Project-tree inventory and incremental attunement

Durable attunement begins with a deterministic project-tree snapshot. The
snapshot is the input boundary for claims that RRFlow persists about a project
and for reproducible parse, embedding, index, graph, routine, or evidence work.
Those durable phases cannot guess what a project contained or combine bytes
from a changing filesystem walk.

This requirement does not gate exploratory inspection. An AI or authorized
tool may read available project material, search live files, and form a
provisional plan before a complete snapshot exists. Before RRFlow persists the
result as project knowledge, applies a source mutation, resumes it, or claims
snapshot-complete evidence, `RrdEngine` captures or refreshes the affected
inventory boundary and rejects stale, escaped, or conflicting inputs.

```text
rrflow install preview
        -> resolve project root + inventory policy + budgets
        -> produce exact plan and plan digest; no writes

explicit apply(plan digest)
        -> RrdEngine creates estate + durable attunement job
        -> authorize one bounded inventory read plan
        -> enumerate metadata under the project root
        -> stream eligible file bytes for digest/classification only
        -> pure rrd-attunement inventory proposal
        -> RrdEngine validates and atomically commits:
             source-tree snapshot + containment edges + change set
             + inventory checkpoint + runtime-log entry + audit
        -> commit receipt permits the parse phase to begin
```

The engine-side attunement executor coordinates filesystem effects,
authorization, leases, cancellation, budgets, and commits. The planned
`rrd-attunement` crate is pure compute: it accepts bounded enumerated entries
and content chunks, sorts and normalizes them, and returns a deterministic
proposal. It cannot open paths, contact a provider, read secrets, access a
store, emit lifecycle callbacks, or commit state. No client, watcher, hook,
parser, or model owns project discovery.

### Canonical inventory records

| Record | Required semantics |
|---|---|
| `SourceTreeSnapshot` | Estate and project identity; logical root coordinate; snapshot identity and root digest; prior snapshot/cursor when incremental; inventory-policy, walker, and hasher revisions; entry/error totals; bounded-work accounting; creation commit. Host-absolute paths are evidence-local metadata, not portable record identities. |
| `SourceTreeEntry` | Stable identity from the estate plus normalized root-relative path; parent identity; entry kind (`directory`, `file`, `symlink`, `mount`, or `error`); size, executable/mode facts allowed by policy; content digest only for eligible files; classification and detector evidence; observed metadata revision. |
| `SourceTreeChangeSet` | Deterministically ordered additions, modifications, removals, and rename candidates between two committed snapshots, with old/new entry identities and the evidence used for each classification. |
| `InventoryPolicy` | Versioned root, traversal, ignore, content, secret, generated/vendor/cache, symlink, mount, size, file-count, byte, concurrency, timeout, and cancellation rules plus one canonical digest. |
| `InventoryError` | Root-relative coordinate, operation, stable error class, retryability, and redacted evidence for unreadable, vanished, replaced, or changed-during-read entries. Errors are data; they are never silently omitted. |

Entries are ordered by normalized path bytes before hashing or proposal
encoding. Directory digests are Merkle-style digests over the directory's own
normalized facts plus ordered child identities and digests; the project root
digest therefore changes when an included child, classification, or applicable
policy changes. Concurrency may improve enumeration and hashing, but it cannot
alter the committed order or digest.

The inventory commit creates only observed filesystem records and typed
`contains` relations. Package, module, import, export, symbol, call,
dependency, schema, and semantic relations require later parse, normalize, and
entity-link evidence. Filename conventions are useful classifier evidence,
not permission to invent those graph edges.

### Traversal, ignore, and safety rules

Inventory uses two bounded passes: metadata enumeration first, then streaming
content fingerprint/classification for eligible files only. It follows these
rules in order:

1. Enforce RRFlow safety exclusions for VCS internals, the project's rrflowDB
   state, build outputs, dependency trees, caches, temporary files, generated
   bulk data, and secret candidates before opening file content.
2. When the root is a Git worktree, retain tracked files and apply Git's exact
   ignore precedence to eligible untracked files. A matching ignore pattern
   cannot hide a tracked source. A non-Git project remains fully supported.
3. Do not blanket-ignore hidden paths: `.github`, `.cargo`, and similar
   project-owned configuration may be material. Classification comes from the
   versioned detector registry and persisted evidence, not scattered language
   checks in the walker.
4. Do not follow symbolic links or cross the authorized root/mount boundary by
   default. Record the leaf and any escape, loop, or mount decision. A more
   permissive policy requires an explicit operator grant and a new policy
   digest.
5. Exclude candidate secret paths before content reads. For ordinary eligible
   content, persist detector rule identities and redacted findings, never
   secret bytes. Environment credentials and external data-source credentials
   are not inventory inputs.
6. Bound entries, depth, individual file bytes, total bytes, open file
   descriptors, concurrency, elapsed time, and cancellation. A truncated or
   failed snapshot is explicit and cannot masquerade as complete input to a
   later phase.
7. Revalidate identity/metadata around each streamed read. Deletion,
   replacement, or mutation during the read becomes an explicit error or a
   bounded retry; mixed bytes never receive a clean content digest.

The initial safety profile excludes at least `.git`, RRFlow estate files,
`target`, `node_modules`, common generated/build/vendor/cache trees, sockets,
devices, and files over the configured ceiling. Exact defaults belong to the
versioned policy template and its tests, not prose-only special cases.

### Incremental refresh and downstream phases

An incremental run compares the previous committed entries and policy to the
new metadata pass. It reads content only when metadata, policy, detector, or
hash revision requires it. Unchanged entries preserve their identity and
digest. Rename is a deterministic change classification supported by matching
content and entry evidence; correctness never depends on an operating-system
rename notification.

Filesystem notifications are untrusted scheduling hints. Gate D supports the
explicit inventory operation without a watcher. After Gate I defines canonical
engine events, an explicitly installed watcher may submit a bounded external
event, but still cannot update a snapshot, call a parser, or mutate an index.
Overflow, dropped-event, or rescan signals force a fresh authoritative
inventory diff. An unchanged run still commits an explicit no-work checkpoint
with zero content reads, so resume and verification never infer completion
from silence.

Each durable downstream phase consumes an exact committed snapshot/change-set
digest. Exploratory reasoning may use provisional material but cannot present
it as this evidence:

- parse incrementally reuses prior syntax trees only when source digest,
  grammar revision, and parser revision match;
- normalize and entity-link preserve unaffected canonical identities while
  recording unresolved or ambiguous evidence;
- lexical, embedding, vector, and graph phases propose atomic canonical facts
  and derived-index deltas through `RrdEngine`; and
- ground and verify prove source coordinates, projection cursors, package
  digest, restart readback, and every skipped/failed phase.

Every durable project-development operation has the same precondition. Before
`RrdEngine` accepts a source-mutation proposal, persists derived project state,
or claims a reproducible plan, it resolves the relevant complete authorized
project-tree snapshot at the operation's `ReadStamp`. A missing, failed,
truncated, policy-stale, or explicitly out-of-date snapshot returns a typed
`inventory-required` precondition for that effect or claim, or proposes the
inventory operation. Live `find` output, an editor file list, a model
observation, or a provider cache may inform provisional exploration but cannot
satisfy the durable precondition. The governed context planner queries bounded
tree records and `contains` relations plus later grounded semantic edges; it
never passes the entire project tree merely because the tree exists.

A source-mutation proposal binds its input snapshot and affected entry
digests. The authorized filesystem activity verifies those preconditions,
applies only the previewed changes, and then requests a new authoritative
inventory pass. Downstream durable knowledge and effects bind the resulting
committed change set; explicitly provisional exploration may continue without
claiming it. This makes “rebind the affected source before an effect” an
engine-enforced development rule while still detecting a file changed outside
RRFlow between planning and apply.

The first acceptance corpus runs against this repository and covers stable
digests under different traversal schedules; add/edit/remove/rename; Git ignore
precedence including tracked ignored files; non-Git and hidden source;
symlink escape/loop and mount boundaries; unreadable, vanished, and racing
files; secret candidates; generated/vendor/cache exclusions; cancellation and
restart; resource ceilings; close/reopen; and an unchanged rerun with zero
content reads. It must prove parse cannot start before the snapshot and
inventory checkpoint share a successful commit receipt. Gate I's first
durable project-development routine must additionally prove that no current
complete snapshot returns `inventory-required` for the dependent effect or
claim, a changed-since-plan file fails its precondition, and no model or adapter
can authorize a path outside the committed tree/root policy.

## Automation, routine, and skill flow

RRFlow automation is durable engine state, not a host callback framework and
not another runtime. The following responsibilities are disjoint even though
they share one rrflowDB transaction, graph, query, and evidence model:

| Element | Owns | Cannot own |
|---|---|---|
| Engine event | One immutable, uniquely identified occurrence with producer, action, target, scope, payload schema, provenance, causation, correlation, idempotency, and commit coordinates. | Execution, authorization, scheduling, or UI styling. |
| Trigger | A versioned deterministic predicate over committed engine events and a bounded operation or routine-start proposal. | Arbitrary code, direct mutation, transport calls, or retry loops. |
| Routine definition | An immutable, digest-bound directed graph of semantic operation steps, conditions, budgets, retry rules, compensation, verification, and terminal states. | Mutable run state, physical access-path selection, or provider-specific commands. |
| Routine run | Durable execution state pinned to one routine revision, activation event, specialization, policy, and input digest. | Redefining its routine during replay or inferring success from a trace/client. |
| Skill package | Immutable instructions, schemas, examples, evaluation cases, and resource descriptors resolved at a read stamp. | Permissions, mutation, process execution, scheduling, or database/index selection. |
| Function | A bounded deterministic transform or proposed-transaction validator executed by an engine-owned sandbox. | Durable orchestration, post-commit event matching, network access, or lifecycle state. |
| External activity | One prepared, fenced attempt to invoke an installed model, process, network, or MCP capability and return a bounded observation for engine validation. | Canonical mutation, routine advancement, retries, authorization, verification, or direct storage/query/index access. |
| Host-event adapter | Explicit translation of one authenticated external occurrence into the public engine-event submission contract. | Automatic installation, canonical state, routine scheduling, or storage access. |
| MCP adapter | Projection of public RRD operations/resources/tasks and, when configured, transport for one external capability invocation. | A second context assembler, workflow engine, database trigger system, or presentation authority. |

The [governed function reference](../reference/automation/functions.md) owns
function artifacts, definitions, catalogue publication, runtime profiles,
proposed-transaction bindings, invocation receipts, installation, and
DataFusion separation. The
[project command capability reference](../reference/automation/project-command-capabilities.md)
owns nondeterministic external process activities. Neither can redefine the
event, routine, transaction, or query authority in this record.

### One canonical event path

Internal and external occurrences become the same canonical engine event:

```text
accepted semantic mutation                 authenticated external occurrence
          |                                               |
          | same transaction                              | submit operation
          v                                               v
       RrdEngine validates identity, schema, scope, policy, and idempotency
                                  |
                                  v
       semantic commit: data/index deltas + engine event + outbox + audit
                                  |
                         rrflowKV commit receipt
                                  |
                                  v
                    committed-event consumption only
```

An internally caused event is included in the same semantic write batch as the
change it describes. An external host, process, telemetry source, or MCP client
submits a bounded event through an authenticated public operation; triggers do
not see it until that submission commits. Producer identity plus event identity
is unique, and a repeated identity with different canonical bytes is an
idempotency conflict. Payloads are schema-bound and size-bound, with large
content stored by verified object reference.

The kernel event value and public event envelope are two representations of
this one semantic object, not separate event systems. I-01 must directly
converge the current `RuntimeEvent` value into the canonical engine-event
vocabulary and define its exact lowering into `RuntimeMutation::Event`; no
forwarding event type or parallel event log survives.

The synchronous behavior formerly named `FunctionTrigger` occurs before a
proposed transaction commits. A-07.1 directly renamed it to a transaction
function binding with no alias or former-field decoder. It may validate the
proposed transaction or derive an event inside that same commit, but it is not
a post-commit event trigger. Its allowed completion receipt, audit, derived
event, outbox, index changes, and domain mutation must share one
effect-complete semantic commit; the current separate pre-commit audit writes
are characterization, not accepted atomicity.

### Durable trigger and routine execution

```text
committed event at cursor N
        -> trigger worker leases a bounded cursor range
        -> pure predicate evaluation against pinned trigger/specialization
        -> activation decision committed with reason and evidence
        -> unique activation key(event identity, trigger digest, routine digest)
        -> RrdEngine commits Pending RoutineRun or a denial
        -> worker leases one ready step
        -> RrdEngine authorizes the step's semantic operation
        -> operation returns a typed result or mutation proposal
        -> RrdEngine commits result + effects + checkpoint + audit + events
        -> next ready step, waiting state, compensation, or terminal outcome
```

Trigger cursors, activation decisions, routine definitions, routine runs, step
attempts, leases, cancellations, checkpoints, and terminal outcomes are
ordinary governed rrflowDB records. Their causal relationships are typed
temporal graph edges. Only the commit receipt advances them. Workers are
replaceable pollers over leased state and can crash after any external effect;
therefore every step carries an idempotency key and persists the accepted
result digest before dependent steps become ready.

Routine graph traversal is deterministic. Wall-clock reads, model calls,
process execution, network access, and external MCP calls are explicit
activities outside replay. Their inputs, selected capability identity,
timeouts, attempts, outputs, and evidence are committed before replay uses the
result. A definition revision never changes an existing run. Cancellation is a
persisted request evaluated at step boundaries; timeout and compensation are
explicit transitions, not background guesses.

A routine step names a versioned public RRFlow operation or an attuned
capability reference. It cannot name a Rust function, executable path, shell
fragment, provider, MCP endpoint, storage profile, KV prefix, table provider,
graph implementation, BM25 implementation, HNSW generation, TurboQuant
generation, or DataFusion node. The installed specialization resolves generic
capabilities to exact authorized bindings. This is how a generic `verification`
step can become a direct-process Cargo or pytest binding, or a package-script
pnpm/npm binding, only after project inventory and operator policy prove the
complete invocation closure. A literal package-manager argv is not shell-free
when the selected manifest script, lifecycle companions, or wrappers invoke a
shell. The
[project command capability contract](../reference/automation/project-command-capabilities.md)
owns discovery, binding, activity, observation, receipt, sandbox, and
re-inventory semantics.

### Skill resolution

A skill package is content, not an agent or plugin runtime. Its immutable
manifest binds:

- canonical identity, revision, media type, byte size, and content digest;
- compatible engine/operation schemas and required estate capabilities;
- instruction, input/output-schema, example, evaluation, and resource
  descriptors, each pinned by digest;
- context-token/byte limits, data classification, provenance, dependency
  digests, and retirement state; and
- requested capabilities and secret references, which remain inert until
  separately allowed by estate policy.

Attunement may install generic skill packages and record eligibility, but
activation is a separate persisted policy decision. A routine step resolves an
exact skill revision through `RrdEngine` at its captured read stamp. The
resolver returns a bounded context projection plus evidence; instructions
cannot grant themselves tools, increase budgets, select indexes, or commit
state. An update creates another immutable revision, while existing routine
runs retain the digest they started with.

### Context, graph, indexes, TurboQuant, and DataFusion

Knowledge graph edges, reasoning-tree edges, and routine-control edges are
different typed relation families in the same temporal graph and transaction
model. Routine traversal follows only routine-control relations. Context
assembly may traverse authorized knowledge/reasoning relations and returns the
contributing edge identities as evidence.

Every routine or interactive request expresses semantic intent, anchors,
freshness, quality, and resource budgets. `RrdEngine` captures one `ReadStamp`
and delegates physical planning to rrflowQL:

1. Exact point, range, state-machine, and bounded adjacency work uses the
   native fast path.
2. BM25, payload/scalar filtering, graph adjacency, exact vectors, HNSW, and
   TurboQuant are eligible native access operators selected from the stamped
   catalogue.
3. TurboQuant is a derived candidate projection bound to collection, vector,
   model, metric, dimensions, configuration digest, generation, and source
   cursor. It may reduce candidate work only when active, fresh, and admitted
   by measured quality policy; exact canonical vectors perform required final
   scoring.
4. Broad scans, joins, aggregations, telemetry analysis, and columnar
   transformations stream rrflowKV pages and memtable overlays as bounded Arrow
   batches through DataFusion.
5. Deterministic fusion produces one `ContextPacket`; no access path can return
   data outside the captured stamp or omit its work and contribution evidence.

Indexes are not refreshed by a provider hook. Schema/catalogue rules lower an
accepted semantic mutation into atomic current, temporal, outgoing/incoming
adjacency, scalar/unique, BM25, exact-vector, runtime-log, and projection-delta
changes. Rebuildable HNSW/TurboQuant workers consume committed deltas, build an
immutable generation at a source cursor, verify it against the exact oracle,
and propose activation through `RrdEngine`. A missing, stale, corrupt, or
quality-ineligible projection causes an exact fallback or an explicit budget
denial, never silent stale recall.

The eventual rrflowKV binary key codec uses typed, length-delimited ordered
tuples rather than ambiguous human-delimited strings. C-01 freezes the exact
bytes. Its semantic families must preserve bounded prefix/range access for
current and temporal records, both adjacency directions, scalar/unique values,
BM25 terms/postings, canonical vectors, projection generations/deltas, engine
events, trigger cursors, routine state/checkpoints, skill manifests/bindings,
outbox delivery, and audit evidence. Physical key families accelerate one
logical rrflowDB estate; they do not become public APIs or independent stores.

### MCP and presentation boundary

RRFlow can participate in MCP in two directions without moving authority:

- inbound, the RRFlow MCP server exposes the same context, event, routine,
  skill, status, cancellation, and subscription operations as HTTP/WebSocket
  and SDK clients; and
- outbound, an explicitly configured capability adapter may invoke an external
  MCP tool for one leased routine step after pinning server identity, tool and
  input-schema digests, permissions, secrets references, timeout, idempotency,
  and result limits.

An MCP task identifier may project a durable routine-run identifier, but MCP
task state is never the source of truth. Disconnect, reconnect, cancellation,
or another provider must resolve the same persisted routine state.

Connectome may render low-latency tooltips, panels, graphs, or modals from a
typed, schema-validated presentation proposal referencing persisted data and
evidence. RRFlow never hides control tags in model prose, parses UI behavior
with a streaming regex, or represents a deterministic rule/database result as
unexplained model reasoning. The client owns accessible styling; `RrdEngine`
owns authorization and semantic payload validation; every displayed claim can
identify whether it came from canonical data, deterministic computation, a
model proposal, or an operator action.

### First complete automation proof

The first vertical slice is one generic `error-resolution` routine:

```text
typed diagnostic event
 -> policy/capability trigger
 -> durable error-resolution run
 -> pinned language/project skill resolution
 -> context intent anchored to diagnostic + file + symbol
 -> stamped graph/BM25/vector/TurboQuant/DataFusion eligibility decisions
 -> bounded model or deterministic proposal
 -> separately authorized mutation proposal
 -> attuned owning verification capability
 -> committed verification/result/checkpoint/evidence
 -> restart and cross-surface readback without duplicated effects
```

The acceptance corpus must exercise match, non-match, denial, stale index,
exact fallback, analytical escalation, malformed model/tool output, crash after
external effect, retry, cancellation, compensation, restart, and uninstall.
HTTP, WebSocket, SDK, MCP, and Connectome must observe the same run identity,
state, stamp, result digest, and evidence. Component tests or a successful
compile cannot substitute for this proof.

## Current implementation boundary

| Concern | Present checkout | Required target |
|---|---|---|
| rrflowKV writes and hot reads | Checksummed WAL frames, copy-on-write-capable mutable MVCC generations, snapshots, one consumed point/range/write transaction, authenticated direct current/temporal reads, persisted row-group filters, authenticated adaptive page compression, and accepted C-06j's family-neutral exact-byte page cache. The default scope-aware scan-resistant policy reports exact region/admission/promotion/suppression/demotion/eviction/load counters; exact LRU remains selectable, and both policies preserve durable identity. | Preserve accepted C-02/C-04/C-06 behavior. C-07 qualifies installed-path recovery, maintenance, concurrency, duplicate-load handling, and mapped-buffer lifetime; F-05 separately owns stamp-safe query/result caches and cannot take physical page-admission authority. |
| rrflowKV immutable storage | One batch-v2, manifest-v3, and segment-v6 read path. Segment v6 stores a strict ordered key/version spine in six aligned Arrow-layout buffers plus one authenticated membership filter per row group; it authenticates none/adaptive-LZ4 writer policy and raw/LZ4 codec identity per page. Validated configurable row/byte targets apply to flush and compaction. Descriptors authenticate types, encoding/codec metadata, bounds, stored/logical lengths, statistics, stored-page digests, unique-key counts, and canonical filter words. Manifest descriptors pin segment/schema/key-codec/page-format identities. mmap can lend an eligible raw page buffer through an owned mapping lease; compressed pages decode into bounded aligned owners; bounded/io_uring raw reads allocate; snapshot validation copies or decodes. Segment v1/v2/v3/v4/v5 fail unsupported before alternate decoding. C-06g provides a bounded projected stream over one owned sequence/manifest/memtable/segment generation, selective key/validity/value page acquisition, exact global MVCC merge, tombstone suppression, Arrow-compatible output buffers, active-manifest GC retention, and separate segment-open/startup-reconciliation/query evidence. C-06h adds finite adversarial/property/fuzz/fault coverage; C-06i proves filter canonicality/miss pruning plus exact none/adaptive compression across reopen and protected compaction, corruption denial, structure-aware v6 fuzzing, and one-host integration evidence. C-06j qualifies mixed-family cache interference and rejects value separation/family partitioning for this format. | C-07 must qualify crash, maintenance, storage-full, concurrent-load, and live mapped-buffer lifetime through the installed composition. |
| Transactions, identity, and audit | C-02 supplies one snapshot-isolation point/range/write transaction and one set of repositories for rrflowMX and rrflowKV. Accepted C-03 publishes record/relation temporal state, both adjacency directions, schema-bound scalar/unique changes, BM25/vector source deltas, generic projection work, runtime entry, prepared governed-function receipts and proposals, semantic audit, outbox, cursor, and outcome through one plan. Exact semantic-key comparison passes at prepared, WAL-appended, WAL-synced, and visible-before-acknowledgement failures; catalogue maxima fit one physical batch; corrupt runtime substitution fails closed; and lost-acknowledgement recovery consumes the durable receipt without guest re-execution. | H-04/H-05 must complete same-stamp effect authorization plus identity/trace parity across embedded and transport paths. |
| Arrow conversion | C-06g emits bounded Arrow-compatible key and optional value offset/data buffers from a projected physical stream, but rrflowQL still converts materialized `QueryRow` values into newly allocated typed Arrow arrays. | Adapt eligible segment buffers and bounded decoded/memtable overlays into `RecordBatch` streams through one stamped provider; measure every borrow and copy. |
| DataFusion | Real bounded execution over the materialized Arrow snapshot; it does not yet consume the C-06g storage stream. | Push projection/predicate/limit through the provider into rrflowKV and compose native graph/BM25/vector operators at one stamp. |
| Project discovery and attunement | B-01 freezes the provider-neutral eleven-phase plan/job/checkpoint contract, but no engine-persisted executor, project-tree snapshot, inventory phase implementation, or parse phase exists. | D-01 through D-05 must prove preview/apply, a deterministic committed tree snapshot and change set, pure phase proposals, restartable checkpoints, and downstream work pinned to exact source/policy/tool revisions. |
| Embedding and vectors | Deterministic local embedding, model/provenance binding, exact search, filtered planning, compact dense artifacts, HNSW, quantization, and accelerator differential checks exist. Canonical vector heads retain their source commit coordinate and commit temporal versions plus collection/name/field pointer deltas atomically; that address is required across the public mutation, kernel, inference, commit digest, source keys, and every derived artifact, and no accelerator is built in the write path. Generic publication is exact/compact/HNSW-only; scalar/product/binary/TurboQuant artifacts use one explicit lifecycle and join the same planner only after activation. | D-05/E-04/E-05/F-03 must consume committed deltas incrementally, bind every derived artifact to its source cursor, and preserve exact fallback/reranking inside the stamped native/Arrow plan. |
| Graph, scalar, and BM25 | Current and temporal bidirectional adjacency keys commit with relation changes. Schema-bound scalar/unique entries and exact BM25 old/new source fields commit with record changes and survive rrflowKV reopen. Normal state selection now uses C-04's authenticated direct versions. rrflowQL traversal selects the native temporal adjacency reader, but context graph expansion still materializes selected state and no native scalar reader or incremental term dictionary/posting materializer is planner-selected. | E-01 through E-03 must expose bounded stamped access, prove graph/scalar/BM25 results against exact oracles, and make stale/corrupt projections fall back or fail exactly as declared. |
| Reasoning and context | Generic reasoning-tree, router, context-plan, evidence, and bounded context contracts exist; current assembly uses snapshot BM25, exact vectors, graph BFS, and RRF. | G/H must persist CAS tree execution and dynamically select native or analytical paths without adding another model, planner, or context authority. |
| Automation, skills, and MCP | The canonical `engine/function/` boundary now has a closed content-addressed artifact/schema/definition/binding/receipt contract; typed rrflowMX/rrflowKV catalogue records with immutable membership and one CAS head; frozen JavaScript and Wasm runtime-build identities; transaction-prepared receipts; atomic accepted receipt/derived-event publication; historical lineage enforcement; physically satisfiable catalogue maxima; full semantic WAL-boundary evidence; and lost-acknowledgement recovery without guest re-execution. The former monolithic control record, inline definition bytes, direct function storage path, and recovery re-execution are removed without an alias. MCP still exposes only one context tool. No canonical engine event, post-commit trigger, durable routine, skill package, project-command candidate/binding, external activity operation, install path, cross-language fixture, or outward function operation exists. | H must complete stamped identity/authorization/evidence and outward conformance; D/I must add offline installation, retirement, engine events, triggers, routines, skills, and prepared external activities without creating another authority. |
| Edge and outward delivery | The separate `rrflow-edge` adapter has deterministic offline/provenance evidence; HTTP, SDK, MCP, CLI, and subscriptions have partial real-process coverage. | H/J must route every surface through the same public operations and prove correlated identity, authorization, stamp, result, restart, and distribution behavior. |

These differences are tracked by POAM-002 through POAM-010 and their owning
roadmap gates. POAM-014 tracks implementation traceability across storage,
security, attunement, inference, reasoning, automation, and outward boundaries.
No present type, file format, merge relationship, or passing compile closes
them.

## External systems

PostgreSQL, Turso, SQLite, Dragonfly, object stores, mesh networks, model
providers, and application databases are optional governed integrations. An
operator explicitly configures an adapter; attunement may discover that an
integration is possible but cannot activate it. External systems do not become
RRFlow's canonical persistence, transaction coordinator, query planner, or
context authority.

## Source constraints

The architecture uses these upstream behaviors without inheriting another
system's authority:

- [Apache Arrow columnar format](https://arrow.apache.org/docs/format/Columnar.html)
  defines relocatable columnar buffers and the mutation tradeoff.
- [Apache Arrow IPC](https://arrow.apache.org/docs/cpp/api/ipc.html) documents
  both zero-copy-capable reads and cases requiring allocation.
- [DataFusion custom providers](https://datafusion.apache.org/library-user-guide/custom-table-providers.html)
  separate planning-time pushdown declarations from execution-time streams.
- [WiscKey](https://www.usenix.org/conference/fast16/technical-sessions/presentation/lu)
  demonstrates key/value separation's write-amplification opportunity and its
  value-log garbage-collection tradeoff; RRFlow retains it only after its own
  manifest/snapshot/workload evidence.
- [Techniques for Inverted Index Compression](https://pages.di.unipi.it/pibiri/papers/inverted_index_compression.pdf)
  surveys Elias-Fano and frame-of-reference families and informs per-partition
  posting-codec evaluation; RRFlow freezes its own representation and oracle.
- [TurboQuant](https://arxiv.org/abs/2504.19874) defines the production
  estimator as an MSE quantizer plus a one-bit QJL residual; RRFlow treats it
  as a derived candidate followed by exact scoring, not canonical vector
  truth.
- [LSM-VEC](https://arxiv.org/abs/2505.17152) motivates an LSM-managed
  disk-resident proximity graph; RRFlow evaluates that organization only after
  its exact/HNSW baseline and update/recovery corpus exist.
- [OpenTelemetry tracing](https://opentelemetry.io/docs/specs/otel/trace/api/)
  and [database semantic conventions](https://opentelemetry.io/docs/specs/semconv/db/database-spans/)
  inform low-cardinality spans, attributes, events, links, and status;
  [W3C Trace Context](https://www.w3.org/TR/trace-context/) informs transport
  propagation. RRFlow durable trace evidence remains governed engine data.
- [Git ignore semantics](https://git-scm.com/docs/gitignore) and
  [Git's content-addressed object model](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects.html)
  inform tracked-file reconciliation, deterministic path precedence, and the
  Merkle-style snapshot; RRFlow retains its own policy and digest envelope.
- [Tree-sitter incremental parsing](https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html)
  informs reuse after a committed change set; it does not replace inventory.
- [Lance read and write behavior](https://lancedb.github.io/lance/introduction/read_and_write.html)
  demonstrates immutable columnar fragments, deletion metadata, and compaction
  tradeoffs; rrflowKV is not a Lance clone.
- [Qdrant storage](https://qdrant.tech/documentation/manage-data/storage/) and
  [indexing](https://qdrant.tech/documentation/manage-data/indexing/) inform
  WAL/segment recovery, filter-aware candidates, and persisted index behavior.
- [CloudEvents](https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md)
  informs transport-neutral occurrence identity and producer-scoped duplicate
  handling; RRFlow retains its own closed, estate-bound event contract.
- [Temporal workflow determinism](https://github.com/temporalio/documentation/blob/main/docs/encyclopedia/workflow/workflow-definition.mdx)
  informs deterministic replay and the separation of external activities from
  durable control flow; RRFlow does not add Temporal as another runtime.
- [Model Context Protocol architecture](https://modelcontextprotocol.io/specification/2025-06-18/architecture)
  and [task projection](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks)
  inform adapter isolation, capability negotiation, and long-running operation
  projection; MCP never becomes RRFlow's routine authority.
- [OCI content descriptors](https://github.com/opencontainers/image-spec/blob/main/descriptor.md)
  inform media-type, size, and digest binding for portable skill resources;
  RRFlow's signed offline distribution remains the deployment authority.

## Qualification

The architecture is implemented only when the owning roadmap gates prove:

1. exact transaction, snapshot, key ordering, and rrflowMX/rrflowKV
   conformance;
2. crash recovery and compaction while read stamps and mapped buffers remain
   live;
3. atomic record, graph, scalar, BM25, vector, runtime-log, and index-delta
   maintenance;
4. streaming and pushdown against data larger than the allowed query memory;
5. exact-oracle and recall-quality results after update, delete, rebuild, and
   reopen; and
6. fixed-hardware latency, memory, copy/decode, disk, and failure evidence.
