# RRFlow HNSW projection

**Status:** active implementation reference; compact native projection storage remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/vector/hnsw-projection`
**Owner:** dense HNSW construction, immutable generation, traversal, and freshness semantics

HNSW is a rebuildable RRFlow projection, not a vector sidecar and not canonical
data. Canonical point versions remain authenticated `RuntimeVector` mutations.
The [vector search contract](search.md) owns exact visibility and scoring, and
the [collection contract](collections.md) owns named-vector and payload-field
configuration.

## Projection identity and format

An HNSW configuration binds projection identity, scope, field, dimensions,
metric, optional embedding-model digest, `m`, construction width, maximum
level, deterministic seed, and covered filter properties. Each immutable
generation records a source cursor, configuration digest, artifact digest,
node count, maintenance mode, and—when incrementally advanced—the immediately
previous generation and inserted-version count.

The current HNSW artifact format is canonical JSON with magic `RRDHNS02` and
format version `2`. Decode verifies canonical bytes, digest, descriptor,
candidate coverage, neighbor bounds, generation metadata, and layer-zero
reachability. Earlier graph bytes are rejected rather than interpreted as the
current format.

This format is deterministic and corruption-detecting, but it is storage-heavy
and cannot be memory-mapped as a native graph. Compact adjacency, vector
payload, and filter structures remain required E-04 work.

## Freshness and exact overlay

An active generation covers canonical vector versions through its immutable
source cursor. New puts, updates, and retirement versions after that cursor
form an exact in-process overlay. The planner may select the graph only when
that overlay reaches the request's required vector source cursor. It records
the base cursor, overlay cursor, and overlay candidate count.

Graph traversal proposes dense candidates. Non-matching records remain
available for navigation but cannot enter the eligible layer-zero result heap.
The exact oracle then evaluates graph candidates plus covered overlay versions
for transaction visibility, valid time, latest-version selection, retirement,
filtering, metric score, and deterministic order. This makes committed inserts
and retirements visible before another graph generation is published.

`AllowApproximate` may select an exact path when estimated graph work is more
expensive. `RequireApproximate` requires an identity-compatible graph and
sufficient overlay; it cannot serve a stale graph by itself.

## Immutable maintenance and publication

For unchanged configuration, `HnswIndex::advance` decodes the verified active
artifact, clones its current graph state into a new build, and inserts only
newer vector versions. It does not recompute old neighbor selection from the
canonical corpus. The result is a new immutable generation; configuration
changes intentionally use a full build.

`RrdEngine::ensure_vector_index` currently obtains canonical candidates by
reading every vector version in the scope at one read stamp
(`read_vector_versions`, a whole-family read rather than a cursor-zero replay),
rebuilds the vector artifact catalogue by replaying retained changes from
cursor zero ([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md)), reopens the
active artifact, builds or advances a generation, stages its content-addressed
object, and publishes the descriptor through the engine-owned vector artifact
transaction.
Point commits never wait for the rebuild. An unchanged source/configuration
can return the already-published generation idempotently.

That lifecycle is real, but its input discovery is still a broad log scan and
its serving graph is still loaded into process-local structures. The current
filter properties are validated definitions and embedded candidate metadata;
they are not yet an incrementally maintained persistent bitmap/payload index.

## Kernels, evidence, and remaining gates

Construction uses the scalar scorer so artifact bytes remain portable.
Traversal uses runtime-dispatched AVX2 on supported x86_64 hosts and scalar
otherwise; final exact reranking remains authoritative. Focused tests prove
deterministic bytes and reopen, corrupt/stale denial, filtered traversal,
incremental generations, immediate insert/retirement overlay, scalar/automatic
identity, and mean Recall@10 of at least 0.95 in the fixed 512-vector,
16-dimension, four-metric/selectivity corpus at `ef=128`.

Those tests do not prove Qdrant equivalence or production performance. HNSW
currently supports dense values only. Compact graph storage, payload-derived
edges or bitmaps, bounded native delta reads, compaction/reclamation under
pinned snapshots, distributed placement, and qualified GPU construction are
not complete.

Gate C-04 replaced whole-log candidate discovery with direct stamped reads and
must still serve the artifact catalogue without replay
([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md)). Gate E-04 must commit
canonical vectors with atomic index deltas and prove immutable graph plus exact
overlay through crash/reopen and corruption scenarios. Gate F-03 must expose
HNSW candidate generation as a stamped native operator exchanging bounded
Arrow batches with rrflowQL/DataFusion. Until those gates pass, this is a real
rebuildable HNSW implementation, not the finished persistent recall path.
