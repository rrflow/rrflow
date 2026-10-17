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
runtime-log reconstruction. C-04 is reopened because every vector search still rebuilds the
vector artifact and quantization catalogues from cursor 0 ([POAM-028](poam-028.md)). Temporal adjacency is not yet a native bounded traversal;
scalar/unique state, incremental BM25 postings, and exact/approximate vector generations
still do not form one planner-selected, stamped native access system. Elias-Fano/PFOR
posting compression, TurboQuant_prod, and LSM-VEC-style disk navigation are evidence-gated
candidates, not implemented defaults.

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
