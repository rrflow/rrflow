# Gate E — make graph and indexes native incremental access paths

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-e`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | E-01 | Maintain temporal outgoing/incoming adjacency as a derived rrflowKV access path for canonical relationships. Feed selected neighborhoods through the same stamped, bounded projected storage stream and its configured physical page cache; rrflowQL exposes relational data, not a separate graph engine or required graph language. | `rrd-store`, `rrd-lsm`, `rrd-query` | Directed/typed/depth-bounded relationship results match an independent temporal oracle across mutations, authorization, reopen, compaction and retained stamps. Controlled growth of unrelated data measures seeks, overlapping runs, relevant versions, keys/pages/bytes and cache work; traversal must not scan the unrelated engram. Streamed and exact results agree, with bounded buffer lifetimes and resource accounting. |
| [ ] | E-02 | Persist scalar and unique indexes transactionally with record mutations. | `rrd-store`, `rrd-query` | Insert/update/retire/conflict/reopen differential proves index and authoritative record cannot drift. |
| [ ] | E-03 | Persist incremental BM25 dictionary, document statistics, postings, positions, and tombstones at a declared source cursor. Select posting-partition codecs from measured raw/bit-packed, partitioned Elias-Fano, and PFOR candidates; codec choice is authenticated metadata and cannot change lexical semantics. | `rrd-query`, `rrd-store` | Incremental results equal a full exact rebuild across update/delete/reopen/corruption fixtures; adversarial sparse/dense/clustered postings prove exact seek/iteration while bytes, decode work, latency, and update/compaction amplification justify each retained codec. |
| [ ] | E-04 | Commit canonical vectors with an atomic index delta; search an immutable HNSW generation plus exact delta overlay and exact-rerank final candidates. Evaluate TurboQuant_prod (MSE quantizer plus one-bit QJL residual) as an authenticated candidate representation and evaluate an LSM-VEC-style disk graph only after the exact/HNSW baseline exists; neither is an assumed default. | `rrd-vector`, `rrd-store`, `rrd-engine` | Exact oracle, recall@k, estimator error/bias, filtered search, update/delete, stale generation, reopen, interrupted-build, build/compaction amplification, RSS, and latency tests pass on declared RRFlow corpora. Approximate candidates never become authoritative results and exact reranking/fallback remains available. |
| [ ] | E-05 | Add cost/selectivity estimates choosing point, range, scalar, BM25, exact-vector, or HNSW access without caller-selected internals. | `rrd-query` | Stable explain plans and adversarial fixtures prove correctness fallback when statistics or projections are absent/stale. |

## Consumption by the real-project build

[Gate D's phase packages](gate-d.md#project-workflow-build-packages) consume
these exact access paths. E-01/E-02 support canonical source relationships;
E-03 completes persistent lexical access; E-04 first qualifies exact filtered
vectors and atomic deltas before approximate generations; E-05 selects only
available qualified paths. The full gate stays open when a bounded path passes.
HNSW/quantization or disk graph changes must improve a matched quality/cost
experiment against the exact baseline; they are not prerequisites to prove a
correct source-grounded exact-vector query.

## Exit condition

Gate E exits only when graph, lexical, scalar, and vector routes are real
bounded storage access paths with exact fallbacks.
