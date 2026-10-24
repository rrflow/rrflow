# RRFlow vector quantization lifecycle

**Status:** active implementation reference; native delta input and outward conformance remain incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/vector/quantization-lifecycle`
**Owner:** immutable quantized artifact build, activation, retirement, and exact-rerank semantics

RRFlow treats quantization as a rebuildable projection of canonical dense
vectors. Scalar, product, binary, and TurboQuant artifacts use one
engine-authorized lifecycle and the [shared vector planner](search.md). Their
scores can select candidate references; canonical full-precision vectors and
the exact scorer remain authoritative for returned hits.

## Canonical lifecycle

Provider-neutral contract types and `RrdEngine` methods implement four
operations:

- build the next immutable generation in `ready` state;
- list bounded artifact and lifecycle evidence;
- activate one verified generation for planning; and
- retire a generation without changing canonical vector history.

An immutable artifact entry binds collection, named vector, codec kind,
projection descriptor, generation, source cursor, configuration and artifact
digests, content-addressed object receipt, and build time. Append-only
build/activate/retire events use contiguous revisions and a predecessor-digest
chain. Reconstruction rejects missing or duplicate builds, revision gaps,
digest-chain changes, non-contiguous generations, backward activation, and
non-atomic record/object publication.

`ready` artifacts are not served. Activation verifies object length and digest,
binary framing, descriptor identity, codec, and internal checksum. Activating
a later generation retires the prior active generation for the same identity.
Restart reconstructs active generations through the same planner; missing or
corrupt bytes fail closed.

The generic HNSW/exact artifact catalogue and the quantization lifecycle have
separate revision coordinates because they govern different artifact families.
They meet only in the engine-owned planner and cannot publish canonical point
state.

## Codecs and query semantics

The implemented codec families are:

| Method | Packed-vector ratio | Representation |
|---|---:|---|
| Scalar | 4x | artifact-wide symmetric signed int8 scale and dense codes |
| Product | 4x, 8x, 16x, 32x, or 64x | deterministic 256-entry subspace codebooks and centroid codes |
| Binary | 32x | sign-bit vectors with full-precision query scoring |
| TurboQuant | 8x, 16x, 21x, or 32x | seeded Hadamard rotation, fixed low-bit codes, and norm correction |

These ratios compare full-precision vector bytes to packed vector bytes, not
complete artifact size or resident memory. Metadata and product codebooks are
accounted separately, and small product-quantized corpora may produce a larger
complete artifact than their f32 payload.

All four families have bounded checksummed binary formats and read-only mmap
reopen. Scalar, product, and TurboQuant scoring use runtime-dispatched AVX2 on
supported x86_64 systems with a scalar oracle; binary uses packed-bit
operations. The planner rechecks scope, field, dimensions, metric, model,
filter coverage, source cursor, valid time, read cursor, generation, and
artifact identity. Approximate scores select references, then exact-f32
reranking produces final results.

## Current persistence boundary

Artifact records, object references, and lifecycle events persist through the
same `RrdEngine`/rrflowKV authority, and the engine integration test proves
ready isolation, activation, search, close/reopen, retirement, update, rebuild,
and reactivation. However, each build currently reads every vector version in
the scope at one read stamp (`read_vector_versions`) and reconstructs all
collection candidates before encoding a new full artifact, and the quantization
catalogue it publishes into is rebuilt by replaying retained changes from
cursor zero ([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md)). There is no
native changed-vector input or incremental quantized-segment maintenance yet.

The generic vector-index request now configures HNSW only. Scalar, product,
binary, and TurboQuant artifacts can be constructed only through the explicit
quantization build operation and can serve only after explicit activation.
Generic catalogue construction and runtime publication reject quantized
descriptors, and restart installs only the active lifecycle generation through
an explicitly lifecycle-named, revision-neutral planner restoration. There is
no TurboQuant `ensure_vector_index` adapter or reconstruction-time suppression
path.

## Required TurboQuant scope

By the repository owner's direction of 2026-10-24, TurboQuant is a complete
capability, not one codec. [OBJ-13](../../objectives/rrflow-1.0-alpha.md#required-outcomes)
owns the measurable result; this record owns the contract. In every part,
canonical full-precision vectors and the exact scorer remain authoritative.

| Part | Current state | Owning gate and required behavior |
|---|---|---|
| Persistent vector search | The MSE codec and lifecycle above serve after explicit activation; builds read every vector version and the catalogue replays from cursor zero. | E-04 maintains TurboQuant generations incrementally and atomically with point commits. F-03 runs candidate generation and exact reranking in the stamped Arrow pipeline. Closing [POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md) removes the per-search catalogue rebuild. |
| Inner-product variant | Absent: `rrd-vector`'s `turboquant` module implements the MSE path only. | E-04 adds the [paper's](https://arxiv.org/abs/2504.19874) product estimator, an MSE stage plus a one-bit Quantized Johnson-Lindenstrauss residual, as an explicit lifecycle codec choice. Recall and score error are measured against the MSE variant and exact search on the same corpus. |
| Model KV cache | Absent: RRFlow runs embedding models only, and attention caches belong to admitted model runtimes. | In Gate G, SWAG selects a TurboQuant KV-cache format only when an admitted runtime advertises it and qualification passes. The format joins the [cache identity](../inference/lfg-swag.md#cache-identity-and-sharing). This is inference acceleration, not rrflowKV compression. |
| Sync transfer | Absent. | H-07 may offer a selected-data profile in which a device receives verified quantized artifacts instead of a family's full-precision vectors, which reduces transfer bytes. That device only proposes candidates: its returned hits are exact-reranked by a peer that holds the canonical vectors, or they are labelled approximate. Full synchronization still transfers canonical vectors. |

J-04 qualifies quality, memory, latency, and transfer-size claims for every
part. A published paper result is not RRFlow evidence.

## Evidence and remaining gates

The fixed 512-vector by 64-dimension matrix covers eleven codec/ratio rows,
eight queries, exact reranking over 96 candidates, mmap descriptor identity,
corruption denial, scalar/SIMD parity, byte accounting, mean absolute score
error no greater than 0.15, and mean Recall@10 of at least 0.80. The retained
debug-host observation is
[`docs/evidence/g04-w04-quantization-local-512x64.json`](../../evidence/g04-w04-quantization-local-512x64.json);
it is not a release SLO or Qdrant-equivalence claim.

Gate C-04 provides bounded direct canonical vector reads, and C-05 leaves one
quantization authority; the lifecycle catalogue itself is still rebuilt from
cursor 0 by every search and lifecycle operation
([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md)). Gate E-04
must make index deltas atomic with point commits and prove rebuild/reopen under
failure. Gate F-03 must execute quantized candidate generation and exact
reranking inside the stamped Arrow operator pipeline. H-04 must prove every
public adapter lowers to the same engine operations. Until then, the lifecycle
and codecs are real, but the optimized persistent recall path is incomplete.
