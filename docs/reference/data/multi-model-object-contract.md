# RRFlow multi-model and immutable-object contract

**Status:** active implementation reference; native Arrow-compatible persistence and access paths remain incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/data/multi-model-object-contract`
**Owner:** canonical multi-model commit semantics and immutable-object visibility

RRFlow has one logical mutation boundary, not a collection of databases hidden
behind a coordinator. `RrdEngine` authorizes the operation and the
`StorageEngine` transaction primitive publishes its canonical state. The
[`RuntimeSchemaRegistry`](schema-catalogue.md) governs the values carried by
that transaction. Object stores, DataFusion, vector artifacts, and operator
application databases do not acquire independent authority over RRFlow state.

This record describes what the current implementation actually proves and the
remaining work required before calling rrflowDB an Arrow-compatible persistent
multi-model reasoning and recall engine.

## One logical commit

One `RuntimeCommit` can contain any ordered combination of:

- bitemporal claims and schema revisions;
- records and directed typed relations;
- immutable events;
- dense, sparse, and multivector values;
- typed time-series samples and WGS84 geospatial values;
- verified immutable-object references; and
- explicit retirement facts.

Every accepted mutation receives a position in the same global cursor and
digest chain. A `DataTransaction` binds the commit's expected cursor to an
authenticated `ReadStamp`, so a stale or divergent authoring state fails
before publication. Schema continuity, property contracts, relation
endpoints, subject references, vector provenance, temporal windows, and typed
values are validated before publication. rrflowKV then lowers the accepted
plan into one persistent write batch.

The accepted commit also publishes its deterministic `ProjectionWork`, sealed
audit envelope, accumulator state, and content-addressed commit outcome. A
byte-identical retry returns the recorded outcome and does not duplicate
mutations or projection work.

## Current storage profiles

| Boundary | Current role | Persistence claim |
|---|---|---|
| `rrflowMX` / `RrflowMxStore` | Process-local implementation of the same logical commit, read-stamp, outbox, audit, and outcome semantics | Deliberately volatile; restart recovery is out of scope |
| `rrflowKV` / `RrflowKvStore` | Plans the complete logical commit as one mutation vector and publishes it through one authoritative custom-LSM write | WAL-backed state, outbox, audit, outcome, and read stamp survive close/reopen |
| `ImmutableObjectStore` | Holds immutable content bytes addressed by SHA-256; only their verified references enter the logical commit | Durability depends on the selected object adapter and remains outside the rrflowKV transaction |
| `rrflowQL` / DataFusion | Reads and computes over engine-authorized state | It is not a transaction coordinator and never publishes canonical mutations directly |

The rrflowKV batch stores canonical model values as JSON in separate keyspaces:
a current head keyed by scope and logical identity, a version keyed by scope,
identity, valid time, and cursor, and, for relations, both adjacency directions,
alongside the authenticated mutation log. That is a real durable implementation
with versioned native keys, bidirectional adjacency, and Arrow-layout segment
pages (segment v6). It is not yet the complete Gate C and Gate E target: values
are JSON rather than native encodings, nothing in production reads the
adjacency keys (E-01), and scalar, BM25, and vector index deltas are not
consumed by native readers (E-02 to E-04).

## Immutable-object visibility

Large source artifacts and multimodal payload bytes use a deliberately
asymmetric protocol because a filesystem or remote object service cannot
participate in the rrflowKV transaction:

1. Compute the SHA-256 content identity and stage the bytes.
2. Durably publish the immutable object at its canonical content-addressed key.
3. Verify its digest, length, and backend receipt.
4. Verify every referenced object again immediately before the data commit.
5. Commit only the verified `ObjectReference` through the normal
   `DataTransaction`.
6. Treat bytes left by a rejected transaction as inventory orphans, never as
   reachable canonical state.

This guarantees atomic *visibility of the verified reference*. It does not
claim that an object upload rolls back with rrflowKV. Lost acknowledgement
after the reference commit is handled by the normal content-addressed retry.
Retention-aware automated reclamation remains open; the current API deletes
only a caller-proven unreachable digest set.

### Implemented adapters

`MemoryObjectStore` provides the same content identity and verification rules
for rrflowMX compositions without persistence.

`LocalObjectStore` uses create-new staging files, file and directory sync,
durable rename, post-publication verification, explicit staging/orphan
inventory, corruption quarantine, and caller-directed reclamation. Streaming
ingest hashes and bounds the payload without first materializing the complete
object.

`S3CompatibleObjectStore<C>` is a provider-neutral adapter over the
`S3ObjectClient` port. It validates declared authentication, signed-payload,
conditional-write, checksum, pagination, ranged-read, and resumable-multipart
capabilities; bounds retries and transfer buffers; verifies parts and the
complete content address; and treats ETags only as opaque receipt evidence.
The repository contains an in-memory conformance client, not a production S3
HTTP transport or certified provider endpoint. S3 therefore remains an
optional immutable-object adapter, never rrflowDB's storage authority.

## Reasoning, recall, and analytical consequences

Reasoning records, reasoning events, graph relations, evidence objects, and
vector values already share the logical transaction and read-stamp model.
That prevents a query from silently combining facts accepted at different
logical heads. It does not yet prove the intended low-latency reasoning and
recall execution:

- `runtime_data_snapshot` reads every version family directly at the read
  stamp (`RuntimeVersionedSource::all()`) and reconstructs all logical families
  for the requested valid time; it is a whole-scope read, not an indexed one;
- the vector artifact and quantization catalogues are still rebuilt by
  replaying retained changes from cursor zero on every vector operation
  ([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md));
- both adjacency directions are written as native keys, but no production
  reader uses them; traversal still loads relations by identity and scans them
  (E-01);
- vector values are canonical JSON records; HNSW, quantization, BM25, and
  scalar structures remain derived artifacts or in-memory execution paths;
- time-series and geospatial values have typed semantics but no specialized
  native access path; and
- DataFusion currently receives materialized rows/Arrow allocations rather
  than stamped streams over Arrow-compatible rrflowKV pages.

Consequently, this contract is the semantic oracle for the physical rewrite,
not evidence that the rewrite is finished. Gate C must establish the versioned
key/transaction/segment spine, Gate E must publish native graph and recall
indexes in that spine, and Gate F must expose those stamped access paths as
bounded Arrow/DataFusion streams. The accepted target flow is owned by the
[engine data-flow architecture](../../architecture/engine-data-flow.md), and
the executable sequence is owned by the
[RRFlow 1.0 roadmap](../../roadmap/rrflow-1.0.md).

## Executable evidence

`crates/persistence/rrd-store/tests/unified_data.rs` proves:

- the same mixed eight-mutation transaction on rrflowMX and rrflowKV;
- seven deterministic projection-work records, a sealed audit envelope, and
  an idempotent stored outcome;
- rejection of a dangling late-family vector without advancing the cursor or
  publishing earlier record, outbox, or audit state;
- explicit orphan state when failure occurs before the reference commit and
  safe retry when acknowledgement is lost after it; and
- rrflowKV flush, close, reopen, and idempotent retry of the persistent state.

Object unit tests cover every local publication boundary, bounded verified
streaming, missing and corrupt bytes, quarantine, and explicit orphan
reclamation. S3 adapter tests cover capability refusal, local/remote content
semantic parity, bounded ranged reads, resumable multipart transfer, bounded
transient retry, and fail-closed corruption using the in-memory conformance
client. These tests establish the current boundary; they do not substitute for
the C/E/F acceptance evidence above.
