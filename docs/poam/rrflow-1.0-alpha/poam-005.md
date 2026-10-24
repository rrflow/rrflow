# POAM-005 — native graph, scalar, lexical, and vector access

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-005`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Accepted C-03 proves atomic multi-family publication/recovery, and C-04's accepted evidence
proves authenticated direct semantic-version selection and exact snapshots without normal
runtime-log reconstruction. Vector search now reads its artifact and quantization catalogues
at the request stamp; maintenance replay and final closure keep C-04/[POAM-028](poam-028.md)
open. rrflowQL traversal now reads native temporal adjacency ranges,
but context graph expansion and memory-estate seat planning still materialize scope
snapshots; scalar/unique state, incremental BM25 postings, and exact/approximate vector generations
still do not form one planner-selected, stamped native access system. Elias-Fano/PFOR
posting compression, TurboQuant_prod, and LSM-VEC-style disk navigation are evidence-gated
candidates, not implemented defaults.

The existing traversal also replayed its entire accumulated selected history at
every breadth-first depth. The
[incremental reduction journal](../../evidence/change-journals/gate-e/e-01-incremental-graph-reduction.md)
records the reproduced eight-hop case (45 temporal mutation visits for 17
supplied versions) and its replacement with one request-local cursor-aware
reducer. This repairs repeated temporal reduction without closing E-01:
context/seat paths, total memory budgets, eager result/path allocation and the
complete native access qualification remain open. Research-to-boundary choices
are explained by the [scale rationale](../../architecture/engine-scale-and-contracts.md#graph-vector-and-knowledge-research-mapped-to-rrflow).

## Impact

Context can remain slow, and a derived index can still become stale, lose exact semantics,
or select an unbounded path until the accepted deltas have stamped materializers, exact
fallbacks, algorithm-quality evidence, and planner evidence.

## Owning gates

E-01 through E-05

Roadmap owner: [Gate E](../../roadmap/rrflow-1.0/gate-e.md).

## Closure evidence

Retain the C-03 atomic/reopen/fault and C-04 direct-read/source-closure corpora; bounded
native readers and materializers must match exact oracles across update, retire, stale,
corrupt, interrupted-build, and reopen. BM25 codec tests compare raw/bit-packed, partitioned
Elias-Fano, and PFOR by partition; vector tests measure exact oracle, recall@k, estimator
error/bias, filtered recall, RSS, update/build/compaction amplification, and exact
rerank/fallback before retaining TurboQuant_prod or disk navigation. Physical counters prove
engine-selected access.
