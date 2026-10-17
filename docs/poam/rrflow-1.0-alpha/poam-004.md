# POAM-004 — stamped streamed DataFusion provider

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-004`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

C-04's accepted evidence removes normal runtime-log reconstruction from query execution and
proves authenticated, budgeted direct semantic-version reads with rrflowMX/rrflowKV equality
and rrflowKV reopen/physical counters; C-04 is reopened because the vector catalogues still
replay history ([POAM-028](poam-028.md)). C-06g exposes a bounded synchronous projected storage stream with owned pinned
generations, selective page families, cancellation/drop release, compaction/GC survival, and
operation-scoped counters. C-06h now drives that stream through deterministic
independent-model histories, every injected storage boundary, stress replay,
cancellation/resource cases, and bounded sanitizer fuzz targets. Query execution still
materializes `QueryRow` values and allocates new Arrow arrays before DataFusion; there is no
stamped asynchronous `TableProvider`/`ExecutionPlan` proving provider backpressure, pushdown
semantics, shared request-budget enforcement, or borrowed `RecordBatch` lifetime through the
full analytical path.

## Impact

The physical prerequisite no longer requires a result-sized storage scan, but current
rrflowQL scan cost, memory, and latency can still scale with eager materialization;
installed real-process crash/lifetime qualification and the full provider
cancellation/resource path remain absent.

## Owning gates

C-07, F-01, F-02, F-04, F-05

Roadmap owners: [Gate C](../../roadmap/rrflow-1.0/gate-c.md) and
[Gate F](../../roadmap/rrflow-1.0/gate-f.md).

## Closure evidence

Retain C-04 source closure plus C-06g projected-stream and C-06h adversarial qualification
evidence; then prove bounded asynchronous batches, projection/filter/limit pushdown,
provider cancellation/backpressure, compaction safety, and measured
retained/read/decoded/borrowed/copied/allocated/decompressed/cached bytes through one
stamped DataFusion operation.
