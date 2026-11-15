# Gate F — connect rrflowKV to Arrow/DataFusion correctly

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-f`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | F-01 | Replace pre-materialized `Vec<QueryRow>` snapshots with a stamped `RrflowKvTableProvider` streaming bounded Arrow `RecordBatch` values from rrflowKV memtables and immutable segment pages, including relationship adjacency selected for a query. Relationship expansion must use that governed storage stream, not raw-pointer chasing or an unmetered path around the configured physical page cache. DataFusion receives borrowed buffers only when the physical encoding is eligible and receives pool-owned decoded buffers otherwise. | `rrd-query`, `rrd-store`, `rrd-lsm` | Provider tests prove batch streaming, buffer lifetime safety, fixed memory bounds on data larger than query memory, exact results across borrowed and decoded paths, and cache/accounting parity for relationship work. |
| [ ] | F-02 | Push supported projection, predicate, limit, and ordering requirements into rrflowKV key/page scans; report unsupported predicates honestly and account for physical I/O, decoded, copied, and allocated bytes. | `rrd-query`, `rrd-store` | Explain and physical-counter tests show less I/O and decoding for selective queries while results equal the unoptimized oracle; no test equates memory mapping with universal zero-copy. |
| [ ] | F-03 | Implement internal relationship expansion, BM25 candidate generation, HNSW candidate generation, and `math::rrf()` as native physical operators that exchange stamped Arrow batches with DataFusion. Relationship expansion consumes the E-01/F-01 stream and respects the same authorization, cache, stamp and budgets. | `rrd-query`, `rrd-vector` | Mixed query tests prove one stamp, deterministic ordering, exact reranking, configured cache use and no external database round trip or bypass of the storage stream. |
| [ ] | F-04 | Enforce query memory, spill, elapsed-time, scanned-key, graph-step, candidate, and result-byte budgets across native and DataFusion operators. | `rrd-query`, `rrd-engine` | Each limit has a deterministic truncation or denial fixture with measured resource evidence. |
| [ ] | F-05 | Add read-stamp/query/projection caches with byte accounting and cursor/schema invalidation. | `rrd-engine`, `rrd-query` | Repeated-query benchmark shows bounded reuse; mutation and schema tests prove stale batches are never returned. |
| [ ] | F-06 | Implement configurable engine-owned background compaction of selected cold history, audit and analytical data into compressed Parquet row groups, with the native WAL and transaction authority unchanged. Decide and document the canonical versus derived status of each cold artifact before reclaiming its source; preserve table/record identity, versions, tombstones, provenance, authorization and read stamps. | `rrd-lsm`, `rrd-store`, `rrd-query`, `rrd-engine` | Independent hot/cold and retained-stamp results agree across update/delete, interruption, full disk, corruption, concurrent reads, restart and reclamation. Durable publication never loses an acknowledged write or exposes an incomplete generation. Show bounded background work and actual rows/pages/bytes read, decoded and compressed; measure selective projection/predicate pruning against an exact unpruned oracle. Bounded export alone does not pass. |

F-06 is a target, not an assertion that Parquet currently backs rrflowKV.
The current v6 segment path and its accepted tests remain authoritative until
an exact replacement or archival design proves its identity, snapshot,
recovery and reclamation semantics. Evaluate native Parquet-compatible writing
against the existing locked Rust implementation and the format specification;
do not equate columnar files with WAL durability or automatic zero-copy.

## Consumption by the real-project build

D-05 uses F-01/F-02/F-04 when a phase actually consumes analytical streaming;
F-03 owns mixed graph/lexical/vector analytical operators and F-05 cache reuse.
Native source commits and exact native queries retain their existing engine
path. Do not postpone all source ingestion behind unrelated analytical cache
optimization, or call eager whole-engram materialization the finished streaming
implementation. Each consuming package names its missing provider/operator
prerequisite and measured allocation/batch-lifetime oracle.

## Exit condition

Gate F exits only when DataFusion consumes streamed authoritative access paths
instead of hiding an eager whole-engram materialization, and F-06's Parquet
cold-storage publication, query and recovery cases pass.
