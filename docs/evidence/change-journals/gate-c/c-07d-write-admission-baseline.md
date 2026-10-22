# C-07d encoded write staging baseline

**Status:** active bounded implementation brief; no new runtime acceptance
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-write-admission-baseline`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

Baseline: development main `952ec21b03a9e701967f2f15c0a37e3d34051fed`, tree
`12955e6e4d097f59401f5974eadad80e6bec8292`, after PR 61's 23-job candidate
acceptance. The new isolated worktree is clean. PR 62's independent reader
repair is still in CI and is not silently treated as accepted main.

Current inspection: `WriteBatch::new` enforces the existing authenticated
16 MiB encoded payload ceiling at final batch construction. Native
`Transaction`, shared `TransactionWriteSet` and `SemanticCommitPlan` can retain
larger staged maps before that check. Store error conversion erases the typed
cause into a generic substrate error; a failed semantic commit can therefore
retain uncertain intent even when admission proves no domain publication.

The bounded target is one format-owned incremental encoded-size/count
accountant, reused before staged-map mutation and by final batch validation.
Replacement/delete accounting must release the prior charge without losing
prior data on rejection. Native and profile-neutral transactions and semantic
staging must use it. Preserve typed pre-publication admission through store
and engine classification; explicit abort/retry must remain possible after a
known rejected domain commit. Unknown I/O outcomes remain uncertain.

Exact proposed scope: `rrd-lsm` batch/error/export/native-transaction owners;
`rrd-store` transaction/error/semantic-plan owners; `rrd-engine` error and
transaction-rejection owners; focused accounting, native/profile-neutral and
engine rejection/reopen tests; native CI commands/policy; C-07/POAM-035,
linked evidence and generated navigation/inventory. No format ceiling,
serialized layout, product version, installed configuration default or
semantic authority change. Configurable aggregate budgets, queue/debt,
recovery, allocator/RSS, real disk exhaustion and soak remain required.
This package bounds logical encoded staging, not all temporary allocations.

First oracle: two individually valid values exceed the complete encoded
batch limit; the second staging call must fail while the first remains exact,
replacement can reclaim capacity, and rollback/commit/reopen preserve the
independent expected map. Boundary/overflow and repeated replacement cases
must agree with real batch encoding. An oversized semantic commit must leave
no domain/index/audit/outcome fragment and permit explicit abort plus valid
subsequent work. Reproduce before repair; do not weaken the oracle to pass.

Research/adaptation: existing authenticated encoding is the owner; no external
algorithm or upstream source is needed. Trace/debug: preserve runtime signals,
use typed content-free limit evidence and focused local histories. Resource
proof is encoded bytes/counts and ownership transfer, not measured heap/RSS.
Stop on absent reproduction, changed accepted final encoding, unsafe retry,
partial domain effects, broader policy scope or failing candidate CI.

The preimplementation PR carries this brief, reviewed paths, traceability and
acceptance commands. This minimal baseline/index seed exists only because
GitHub cannot open a PR from an unchanged main tree. It contains no runtime
implementation and its seed CI is not acceptance. The implementation journal
will retain red/green commands, full changed-file review, surfaced failures,
not-run checks and exact-candidate evidence. No roadmap or alpha closure.
