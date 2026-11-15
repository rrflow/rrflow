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

They are public operations (H-04q2): `vector-quantization-build`,
`vector-quantization-list`, `vector-quantization-activate` and
`vector-quantization-retire`. Each one is in OpenAPI, the Rust client, the five
generated SDKs and the shared SDK conformance corpus, and each lowers to its
`RrdEngine` method. The three mutations are authorized as
`vector_collection_ensure` and list as `vector_collection_list`, the engine's
existing actions, so no grant changes.

An immutable artifact entry binds collection, named vector, codec kind,
projection descriptor, generation, source cursor, configuration and artifact
digests, content-addressed object receipt, and build time. Its record identity
is `{projection}.{generation}` and its object's is
`{projection}.{generation}.quantized-bytes`. Both are public canonical ids, so
the public changefeed projects them, and the build refuses any other identity.
Append-only
build/activate/retire events use contiguous revisions and a predecessor-digest
chain. Reconstruction rejects missing or duplicate builds, revision gaps,
digest-chain changes, non-contiguous generations, backward activation, and
non-atomic record/object publication.

Build, activate and retire are replay-safe (H-04q1). Each one requires the
request's idempotency key, and its event records that key with the request's
SHA-256. The event digest covers the receipt only when there is one, so events
recorded before receipts keep their digests.

- **The same key and request** return the recorded artifact, as it is now, with
  `idempotent_replay` set. Nothing is read, encoded or appended, and this works
  after the session's lease has expired.
- **Another request under that key** conflicts.
- **A commit that loses its cursor** to another commit in the scope retries
  from a fresh catalogue read, up to 16 times. Each attempt re-checks the
  receipt and the transition, so concurrent requests with one key apply once.

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
| TurboQuant | 8x, 16x, 21x, or 32x | seeded Hadamard rotation, fixed low-bit codes, and norm correction; or, with `estimator: inner_product`, codes one bit narrower plus the residual's QJL signs |

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

TurboQuant has two estimators (E-04d), chosen at build as part of the
generation's configuration:

- **`mse`, the default:** TurboQuant_mse. Each coordinate keeps its nearest
  codebook value, rescaled to the vector's length. Its inner products shrink
  at low widths: on unit vectors across inner products from 0 to 0.9, the
  slope of estimate on truth is 0.996 at 4 bits, 0.942 at 2 and 0.803 at 1,
  about √(2/π).
- **`inner_product`:** TurboQuant_prod ([arXiv:2504.19874](https://arxiv.org/abs/2504.19874),
  Algorithm 2). It stores codes one bit narrower (3-, 1- or 0-bit Lloyd-Max
  stages at 4, 2 and 1 total bits), the signs of the residual's projection by
  a seeded finite-precision approximation of a Gaussian matrix S, and the
  residual's length. On the tested vectors, the estimate's slope is 1.00 within
  its standard error at each width and mean squared error stays under the paper's
  bound. Those observations do not prove the paper's unbiasedness assumptions
  for this implementation or its cross-platform bitwise reproducibility.
  - The residual is projected in the rotated space, following the paper's
    construction under its independent Gaussian assumptions.
  - S is dense, so the estimator accepts at most 4096 padded dimensions, and
    a loaded generation keeps its S (at most 64 MiB).
  - It refuses 1.5 bits, which has no narrower stage, and Manhattan
    collections.

An estimator is part of the configuration, so an incremental build never
extends a generation of the other one.

## Current persistence boundary

Artifact records, object references, and lifecycle events persist through the
same `RrdEngine`/rrflowKV authority, and the engine integration test proves
ready isolation, activation, search, close/reopen, retirement, update, rebuild,
and reactivation. A full build reads every vector version in the scope at one
read stamp (`read_vector_versions`) and encodes the named vector's view of
each identity's history (`rrd_vector::source_history`). That view drops what
a move out of the collection supersedes, and ends the version held before
the move where the move takes effect, so a generation never proposes an
identity that has left
([POAM-048](../../poam/rrflow-1.0-alpha/poam-048.md)). Quantization
catalogue reads use authenticated, budgeted typed ranges at a read stamp
rather than replaying the change log from cursor zero; the selected lifecycle
records still require reconstruction
([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md)). The commit
path writes each collection, named vector and field's vector deltas
atomically, and storage reads one source's deltas at a request's stamp under
its key budget, checking every pointer against the authenticated version it
names (E-04a).

An active TurboQuant generation keeps serving after writes (E-04b). It answers
at its own source cursor, and every identity whose version changed since then
is reranked exactly, from its full history, as an overlay, as HNSW already
does. When a request requires an approximate path on a named dense vector and
that generation is its only eligible one, search reads only:

- the catalogue;
- that source's deltas since the generation;
- the histories of the generation's unchanged candidates.

Its work then stays the same as unrelated vectors grow. A request that only
allows approximation still reads every version in the scope, because its plan
compares the full exact-scan cost.

A TurboQuant build extends a base when it has one (E-04c). The base is the
newest published generation of the same artifact whose configuration digest
equals the request's and whose object verifies through the binding serving
uses. The build then:

- reads only that source's deltas since the base's source cursor;
- keeps every untouched identity's stored versions and their codes;
- encodes the same view of each changed identity's complete history.

A version outside the collection that follows another outside it changes
nothing the collection records, so the build leaves that identity as the
base holds it, and a full build holds the same.

The codec encodes each vector on its own with the configuration's seed and
fixed codebook, so the generation is byte for byte the one a full build makes
from the same versions. Source-version reads and newly encoded versions follow
the change; the stamp's accumulator proof grows with the logarithm of the
engram's leaves. The build still copies retained codes and serializes a complete
generation, so its total work can grow with the base. Total memory and latency
are not qualified here. The result reports the base generation and the versions
the build encoded.

- **Without a base,** or when the base's object does not verify, the build
  encodes every version.
- **An interrupted build** staged no published entry, so it is never a base.
- **Racing builds:** a build that loses the race for its generation number is
  refused.
- **Other codecs:** scalar, product and binary builds always encode every
  version, because scalar and product quantization take artifact-wide
  parameters from the data.

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
| Persistent vector search | The MSE codec and lifecycle above serve after explicit activation, through the generation plus the stamped delta (E-04b). A TurboQuant build extends its base with the change (E-04c); its catalogues use authenticated stamped direct reads. | E-04 maintains TurboQuant generations incrementally and atomically with point commits. F-03 runs candidate generation and exact reranking in the stamped Arrow pipeline. Closing [POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md) proves that one search's catalogue work stays constant as unrelated history grows. |
| Inner-product variant | `estimator: inner_product` (E-04d) builds, serves, extends and replays through the lifecycle above. Its recall, score error and empirical bias are measured beside the MSE variant and exact search on the same corpus. | J-04 qualifies its quality, memory and latency on representative corpora. Larger dimensions need a projection the paper's analysis covers. |
| Model KV cache | Absent: RRFlow runs embedding models only, and attention caches belong to admitted model runtimes. | In Gate G, SWAG selects a TurboQuant KV-cache format only when an admitted runtime advertises it and qualification passes. The format joins the [cache identity](../inference/lfg-swag.md#cache-identity-and-sharing). This is inference acceleration, not rrflowKV compression. |
| Sync transfer | Absent. | H-07 may offer a selected-data profile in which a device receives verified quantized artifacts instead of a family's full-precision vectors, which reduces transfer bytes. That device only proposes candidates: its returned hits are exact-reranked by a peer that holds the canonical vectors, or they are labelled approximate. Full synchronization still transfers canonical vectors. |

J-04 qualifies quality, memory, latency, and transfer-size claims for every
part. A published paper result is not RRFlow evidence.

## Evidence and remaining gates

The fixed 512-vector by 64-dimension matrix covers fourteen codec/ratio rows,
including TurboQuant's inner-product estimator at 4, 2 and 1 bits. It checks:

- eight queries, with exact reranking over 96 candidates;
- mmap descriptor identity and corruption denial;
- scalar/SIMD parity and byte accounting;
- mean absolute score error no greater than 0.15 and mean Recall@10 of at
  least 0.80.

Each row also reports its mean signed score error. On that cosine corpus, the
inner-product rows have the following recall at equal width: Recall@10 is
1.00, 0.94 and 0.85 at 4, 2 and 1 bits, against MSE's 1.00, 1.00 and 0.975.
The retained debug-host observation is
[`docs/evidence/g04-w04-quantization-local-512x64.json`](../../evidence/g04-w04-quantization-local-512x64.json);
it is not a release SLO or Qdrant-equivalence claim.

Gate C-04 provides bounded direct canonical vector reads, and C-05 leaves one
quantization authority; the lifecycle catalogue itself comes from direct
stamped reads, and C-04's closure evidence is still open
([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md)). Gate E-04
must make index deltas atomic with point commits and prove rebuild/reopen under
failure. Gate F-03 must execute quantized candidate generation and exact
reranking inside the stamped Arrow operator pipeline. H-04 must prove every
public adapter lowers to the same engine operations. Until then, the lifecycle
and codecs are real, but the optimized persistent recall path is incomplete.

The [E-04 TurboQuant persistent serving plan](../../roadmap/e04-turboquant-persistent-serving-engineering-plan.md)
orders the packages for that path and for the public lifecycle operations a
client needs; it owns no contract or status.
