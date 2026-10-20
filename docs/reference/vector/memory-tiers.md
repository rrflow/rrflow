# RRFlow vector artifact residency

**Status:** active implemented serving policy; native persistent access remains open
**Coordinate:** `rrflow://rrflow-instance/data/reference/vector/memory-tiers`
**Owner:** process-local residency of immutable vector artifacts during engine-owned search

Vector memory tiers are physical serving policy inside `RrdEngine`. They do not
define another database, another catalogue, or another form of canonical
state. Durable collection definitions, projection descriptors, generations,
canonical vectors, and immutable-object receipts remain authoritative. A tier
may change how compatible artifact bytes enter a search, but never which
logical result is correct.

This boundary is separate from both principal execution profiles:

- **rrflowMX** is the complete non-durable engine profile. It is not a cache
  tier for rrflowDB or rrflowKV.
- **rrflowKV** is the persistent storage substrate. Its committed state and
  immutable artifact receipts survive restart; resident decoded objects do
  not.
- **rrflowQL/DataFusion** is the planned columnar query path in Gate F. Its
  Arrow memory pools, streaming batches, spill policy, and execution budgets
  are not owned by `VectorResidencyManager`.

## Current engine flow

`RrdEngine::search_vectors_at` captures one `ReadStamp`, resolves the named
vector definition, reconstructs canonical candidates from runtime changes,
and reopens only request-compatible active projection metadata. It then
reconciles the process manifest and asks its one `VectorResidencyManager` to
acquire each compatible immutable artifact. The policy lock is released before
planning and scoring.

```text
committed runtime + projection metadata + object receipt
                         |
                         v
              RrdEngine read-stamp validation
                         |
                         v
       VectorResidencyManager acquire/reconcile policy
          |                 |                 |
       pinned             cached             cold
          \_________________|_________________/
                         |
                         v
          immutable VectorArtifact used by search
```

The currently implemented flow is coherent but not yet the Gate C/E target.
Search reads its canonical candidates through C-04's direct stamped reader, but
it selects the scope's whole vector-version family and reduces it in memory,
and it rebuilds the vector artifact and quantization catalogues by replaying
the scope's change log from cursor 0
([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md)). C-04 must serve both
catalogues from direct stamped reads; E-04 must provide native persistent
vector and filter access paths. F-01 then exposes stamped storage
batches to DataFusion. Residency cannot be used to disguise any of those open
requirements.

## Physical policies

| Tier | Implemented behavior | Capacity rule |
|---|---|---|
| `pinned` | Retains an owned decoded artifact across requests. A cached entry can move to pinned without another decode. | Hard admission bound. Existing entries are not silently evicted; insufficient capacity is explicit pressure. |
| `cached` | Retains owned decoded artifacts in a process-local, byte-accounted LRU. A pinned entry can move to cached without another decode. | Evicts least-recently-used entries until the artifact fits. An artifact larger than the entire cache is served transiently. |
| `cold` | Retains no decoded entry. Supported immutable local binary codecs open through verified read-only mmap; unsupported codecs or backends use a verified transient owned decode. | Resident bytes return to zero after acquisition. RRFlow does not claim control of the operating-system page cache. |

The default limits are 256 MiB pinned and 64 MiB cached. Embedded callers can
provide validated limits through `RrdEngine::open_with_vector_residency` or
`RrdEngine::rrflow_mx_with_vector_residency`. Declared immutable-object length
is the conservative accounting unit.

Compact dense, scalar-quantized, product-quantized, binary-quantized, and
TurboQuant artifacts implement native read-only mmap codecs. Exact JSON
segments and the current HNSW v2 JSON artifact use transient owned cold loads.
A compact HNSW format may join the mmap path only after E-04 evidence; it does
not require a new logical collection or search contract.

## Pressure, integrity, and fallback

Resource pressure and integrity failure have different semantics:

- missing objects, digest or length mismatch, codec corruption, and descriptor
  substitution fail the request closed;
- a pressure-denied artifact is suppressed only in the current process-local
  planner view;
- `allow_approximate` may fall back to the canonical exact scan;
- `require_approximate` reports `ResourceExhausted` if pressure removes the
  required approximate path; and
- local pressure never rewrites durable catalogue generations or object
  reachability.

The manager serializes admission, LRU order, tier transitions, and stale-entry
reconciliation. Loaded immutable artifacts are shared by `Arc`, so search does
not hold the policy lock while executing vector kernels.

## Restart and evidence

Residency starts empty after every process restart. A later search replays
authenticated metadata, reacquires compatible active bytes under the current
collection policy, and must preserve the access path and results. No cache
entry is recovery state.

`RrdEngine::vector_residency_snapshot` reports configured limits, entries,
resident bytes, hits, misses, evictions, stale reclaims, tier transitions,
cold mmap and owned loads, and oversized-cache bypasses. These counters are
diagnostic evidence, not a durable correctness authority.

Current executable evidence is:

- `runtime::vector_residency::tests` for hard pinned admission, cached LRU,
  promotion and demotion, cold mmap, and stale reclamation; and
- `engine::tests::vector_index::engine_vector_memory_tiers_are_physical_bounded_and_restart_safe`
  for public cached, pinned, cold, reopen, exact-fallback, and
  required-approximate pressure behavior.

## Deliberately open scope

The tier is currently declared per named vector and applies uniformly to its
compatible active artifacts. Independent original-vector, HNSW,
quantization, sparse-index, payload, and payload-index policies; admission
queues; workload-driven warming; distributed placement; and live memory-SLO
tuning remain unimplemented. They must extend this one serving-policy boundary
rather than create a cache database or bypass `RrdEngine`.
