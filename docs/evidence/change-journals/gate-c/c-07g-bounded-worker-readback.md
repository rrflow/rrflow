# C-07g bounded worker readback

**Status:** active implementation evidence; native timing acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-bounded-worker-readback`
**Owner:** [C-07g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Starting canonical revision: 868fa45dec7dbd20372b6d10f820cce025828900.
PR72 pre-edit comment 6361981753 binds the scheduler correction. Native Windows
Job characterization03 used clean combined f7ac9044a235218b8d38811630b86d3510658003,
the same optimized engine and unchanged corpus/rate/limits as the preceding
failed runs. Raw evidence remains under
`target/c07-windows-characterization-03/evidence`.

The [explicit abort repair](c-07g-rejected-transaction-cleanup.md) removed the
observed quota cascade: measured segment one had 163 ACKs, 13 known rejected
conflicts and 24 driver concurrency rejections, with no uncertain outcomes.
Every ACK had independent readback. That is not a throughput pass.

The driver also exposed a scheduling defect: each 200-attempt, ten-second
offered segment took approximately 24.5 seconds because it then verified all
operations serially. The next segment inherited the original offered origin,
immediately rejected 196 overdue requests, and failed the start-to-first-sample
coverage check. Its failed segment SHA256 is
41866ff8bf679bd7ed0d5f042cdf0b95fc6133f10c20205c9ee587934cfce6b9.
This driver pause cannot be attributed to the engine's sustainable envelope.

`support/sustained.rs` now performs independent verification inside each
existing admitted worker. The real API response timestamp still determines
the recorded operation latency; verification consumes the same client slot
and total operation deadline. Completion collection immediately retains
ACK/rejection/readback evidence and bounded failure provenance. The duplicate
serial post-segment verification pass is removed. A working generation advances
only after its independent Present readback; each slot is used at most once
per segment. Final full-corpus/working-state readback after actual process
restart, reader cancellation/release, and backup evidence remain unchanged.

Traceability: the original PointRun segment worker, Completion and collect
remain the only scheduler/evidence owner. The old serial oracle maps to the
same Workload::verify invocation within its admitted worker; the aggregate
generation-update loop consumes that result rather than querying again.
The fixed offered origin, zero-error threshold, sample-gap checks, raw failed
segments, client count and finite segment budgets are preserved. No background
worker, second executor, schema, implicit retry or schedule reset is introduced.

Reviewed the complete segment preparation/scheduling/join/readback flow,
Completion/collect, workload execute/verify generation semantics, final point
readback, progress event handling and ledger validation. Existing code and the
native timing series establish the defect; no external research or source
adaptation is required. Stop on missing independent proof, altered API timing,
increased worker admission or weakened resource/gap/error thresholds.

Strict CLI installed-reopen Clippy passed (0.90s); formatting and changed-source
review completed. The same native controlled campaign is the next behavioral
oracle, and candidate CI remains required. No runtime suite or capacity result
is inferred from compilation. Changed paths: sustained.rs, this journal,
POAM-035 and generated navigation/inventory. C-07, alpha and version status
remain unchanged; 24-hour/native profile qualification is still unrun.
