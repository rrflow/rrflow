# C-07g drain probe receipt

**Status:** active implementation evidence; native campaign acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-drain-probe-receipt`
**Owner:** [C-07g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Starting revision: 78e83d9c (full identity retained by Git ancestry), PR72.
Pre-edit comment 6362294709 binds the correction. Controlled Windows campaign04
confirmed the bounded-worker change shortened ten-second offered segments to
approximately 10.1-11.9 seconds. It still failed at 20 requests/second with real
rejections and one unresolved resource-exhaustion response. That operating
point is not qualified; no original failure or threshold was discarded.

A separately frozen one-request/second campaign05 at clean combined
c0a0a85c50561b692117219a71fa6a0e2bd440c2 completed nine load segments and reached
drain. The drain case then claimed a present probe with no recorded attempt
and an effect-derived receipt. The existing independent ledger correctly
rejects missing-attempt/effect-without-admitted-attempt. Retained raw artifacts
are at `target/c07-windows-characterization-05/evidence`; failed-segment digest
651fc255512b3fdab7cb8cb545ae6c81bb4947d2afb68877e08f55dcce0b2ade.

PointRun::drain now records the probe intent, executes the actual public point
read, retains its real response digest and monotonic attempt interval, then
independently verifies its result under the same finite probe deadline. The
existing case ledger contains this probe; the drain offered-load list remains
empty, so the probe cannot inflate throughput. No validation rule, native
resource threshold, schema or runtime behavior changes.

Reviewed the complete drain builder, Workload execute/verify, independent
ledger reconciliation, progress event handling, segment drain/load distinction
and native campaign05 raw probe/resource evidence. The existing missing-attempt
oracle stays canonical. Strict installed-reopen CLI Clippy passed (0.98s),
formatting/diff checks passed. The same native campaign is the behavioral
acceptance still required; no compile-only completion is claimed.

The original frozen zero mutable-growth policy also observes 17,706,205 bytes
before load versus 36,716,001 during idle drain, while transactions, read views,
pins, projected output, queue owners and compaction debt are all zero. This
gauge includes the current mutable table. That observation does not establish
a retained-reader leak, nor is the zero-growth run relabeled as passing. A
later characterization may explicitly reserve one configured active mutable
buffer, as the existing DrainPolicy permits; it must use a new frozen manifest
and retain these original results. C-07 acceptance requirements stay unchanged.

Changed paths: sustained.rs, this journal, POAM-035 and generated navigation/
inventory. No implementation is moved/deleted, no source adaptation or external
research is needed, and no private message trace is introduced. Stop on fake
receipts, counted drain throughput, relaxed independent validation or implicit
effect retry. Candidate CI, actual capacity profiles and 24-hour evidence remain
required. C-07, version and alpha status remain unchanged.
