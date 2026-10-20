# RRFlow vector search

**Status:** active implementation reference; native incremental recall execution remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/vector/search`
**Owner:** read-stamped vector visibility, scoring, filtering, planning, and execution semantics

`rrd-vector` is RRFlow's rebuildable vector-search subsystem over canonical
`RuntimeVector` versions. It is not a sidecar service and cannot publish
canonical data. `RrdEngine` captures and authorizes the read stamp, resolves
collection policy, loads eligible artifacts, and returns plan and resource
evidence.

The [collection record](collections.md) owns named-vector and point
administration. The [HNSW projection](hnsw-projection.md) and
[quantization lifecycle](quantization-lifecycle.md) own their physical artifact
semantics. [Recursive retrieval](../context/retrieval.md) owns fusion,
reranking, and result shaping and is not redefined here.

## Exact semantic oracle

Every search is bound to one scope, `ReadStamp`, valid time, vector field,
shape, dimensions, metric, optional embedding-model digest, filter, and result
budget. Dense, sparse, and multi-dense `MaxSim` exact search supports cosine,
dot, Euclidean, and Manhattan scoring. Higher is always better; distance
metrics return negative distance.

The latest transaction-visible version wins per vector identity. Future
valid-time versions do not hide an applicable version, while a visible
retirement does. Exact filters support equality, inequality, membership,
ranges, existence, and bounded recursive `all`, `any`, and `not`. Final order
is score descending, reference ascending, then source cursor descending.

Invalid shape, non-finite data, duplicate identity/cursor versions, stale or
wrong-scope stamps, incompatible model bindings, and malformed or over-bounded
filters fail closed. This exact implementation is the correctness oracle for
every approximate path.

## Planner and approximate execution

`VectorPlanner` considers an exact scan plus ready artifact candidates. It
rejects stale coverage, wrong field/dimensions/metric/model, missing filter
coverage, invalid projection stamps, and paths incompatible with the caller's
exact/allow/require mode. A prepared plan binds the request digest, catalogue
revision, selected generation, source coverage, `ef_search`, and plan digest;
execution recomputes and compares it before reading artifact bytes.

The current dense HNSW implementation has deterministic construction,
filter-aware layer-zero admission, immutable generations, scalar and
runtime-dispatched AVX2 scoring, and final exact reranking. New vector or
retirement versions after an HNSW generation are searched as an exact overlay
until a new immutable generation incorporates them. Sparse and multi-dense
queries remain exact-only.

Quantized and TurboQuant candidates can also be selected through the same
planner and exact-f32 reranking path. Their format and lifecycle semantics are
owned by their dedicated records; their existence does not qualify E-04 or
establish Qdrant equivalence.

## Current persistent path

`RrdEngine::search_vectors_at` captures one authenticated `ReadStamp`, selects
canonical vector versions through the direct bounded semantic-version access
path, and materializes the selected candidates in memory. It then reconstructs
the serving catalogue from durable artifact records, verifies
content-addressed objects and descriptors, applies memory-tier admission, and
installs selected artifacts into a process-local `VectorRuntime` for the
request.

Artifact descriptors and object references are persisted through engine-owned
runtime state, and restart tests prove verified HNSW and active-quantization
reopen plus retirement-overlay behavior. Compact exact and quantized codecs
support mapped reads; the current exact JSON segment and HNSW JSON artifact do
not. The generic vector artifact catalogue accepts only exact/compact/HNSW
contract-v2 entries and computes one identity digest that includes optional
HNSW build evidence. Quantized and TurboQuant artifacts use their separate
build/activate/retire lifecycle and join only the shared process-local planner.
Pre-1.0 or generic-quantized catalogue entries fail before artifact decoding
or engine replay.

Consequently, a successful HNSW query today is a real approximate traversal
and exact rerank, but it still pays process-local canonical-candidate
materialization and runtime-catalogue reconstruction before planning. It is
not the final rrflowKV-native incremental vector path, and it does not exchange
stamped Arrow batches with DataFusion.

## Evidence and remaining gates

Focused tests prove:

- the checked-in exact/HNSW semantic fixture;
- equal exact results on rrflowMX and rrflowKV;
- deterministic immutable-generation reopen;
- immediate insert and retirement overlay;
- nested filter behavior during HNSW traversal;
- exact reranking and fail-closed stale/corrupt denial; and
- mean Recall@10 of at least 0.95 for the fixed 512-vector, 16-dimension,
  four-metric corpus at 100%, 50%, 10%, and 1% filter selectivity.

The retained 10,000-by-128 local observation remains raw evidence at
[`docs/evidence/m5-vector-local-10000x128.json`](../../evidence/m5-vector-local-10000x128.json).
It is not a cross-system or production performance claim.

Gate C-04 has replaced normal whole-log candidate reconstruction with direct
versioned reads, but every search still rebuilds the vector artifact and
quantization catalogues from cursor 0, so C-04 is reopened
([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md)). Gate C-05 has removed
the older artifact-catalogue reader and separate digest branch, requires the
canonical collection plus named-vector address on every persisted candidate,
and removes the duplicate generic quantized/TurboQuant publication, request,
and suppression paths. Gate E-04
must atomically maintain canonical vectors and index deltas, prove immutable
HNSW plus exact overlay after crash/reopen, and satisfy fixed recall and
filtering gates. Gate F-03 must make vector candidate generation and RRF native
operators over one stamped Arrow/DataFusion plan. Only then does this become
the persistent multimodal recall path required by RRFlow 1.0.
