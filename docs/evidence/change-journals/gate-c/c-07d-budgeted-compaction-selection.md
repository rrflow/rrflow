# C-07d budget-admitted compaction selection

**Status:** active production repair evidence; installed capacity qualification open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07d-budgeted-compaction-selection`
**Owner:** [C-07d/g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Starting revision: ef1f2575702cd220c02cc72522f4a19187cd5cdb.
PR72 pre-edit comment 6363050798 records the behavior, exact scope, failure
oracle, traceability, research decision and stop conditions before implementation.

## Demonstrated defect and replacement

The installed campaign09 on clean combined 4c8d4e62b10033da473397d50b903ac1c0cd7d2c
failed a burst mixed write with a closed maintenance-admission cause. Diagnostics
changed from seven L0 segments/no failed compaction to eight L0 segments/one
failed compaction. The count-only picker chose the largest group before checking
its configured physical/logical-byte and version budgets. A rejected L0 group
also prevented considering lower-level work that could clear its overlap.

A native regression reproduced the defect without a large corpus: four L0
segments exceeded a 3000-byte logical budget, although two fit. Before the
repair it failed with `InputLogicalBytes`, limit 3000, observed 3210. No budget
was raised. `Database::select_compaction_candidate` now evaluates deterministic
source prefixes with their complete target-level overlap closure against one
canonical input-admission helper. It considers smaller admitted groups and
lower-level relief before returning the original typed admission failure.
A sole blocking target can move down within the existing finite level bound.
It cannot hide a rejected multi-source merge by calling a trivial one-source
move success. Planning checks the same cooperative elapsed bound.

The existing merge, snapshot retention, tombstone, immutable publication and GC
boundaries are unchanged. Automatic maintenance still retains every version;
explicit protected-snapshot compaction retains its established semantics.
No scheduler, backend, format, default policy, error class or parallel planner
was introduced. The old count-only selection was replaced at its owning method;
all hard-input-denial and repeated-write L0 rejection tests remain intact.

Files reviewed: `rrd-lsm/src/database.rs` selection/merge/write-maintenance and
options; `maintenance_limits.rs`; `error.rs`; `tests/compaction.rs` and
`tests/maintenance.rs`; store/engine error mapping and transaction rejection;
installed workload diagnostic projection and campaign09 raw failure/resources.
No external research or source adaptation is required for this local correctness
repair. Stop conditions remain unchanged: no lost ACK, partial publication,
weakened hard cap, changed history authority or unqualified capacity claim.

## Validation and retained evidence

- Focused subset regression: failed before the repair as described, then passed
  (one test, 0.05s).
- `cargo test --locked -p rrd-lsm --test compaction --test maintenance`: 12
  compaction and seven maintenance tests passed. A second native regression
  proves blocked L0 work can clear lower-level overlap, resume and preserve all
  acknowledged historical reads through reopen.
- `cargo test --locked -p rrd-lsm`: 183 passed, zero failed, one intentionally
  ignored owned-child entry that the native crash parents invoke explicitly.
  Includes native 100-kill/reopen, compound I/O/publication and lifetime oracles.
- `cargo clippy --locked -p rrd-lsm --all-targets -- -D warnings`: passed, 3.68s.
- Formatting and diff whitespace checks passed.

The earlier explicit damage-policy repair also completed its existing-project
matrix: one test, 371.06s, all eight missing/corrupt roles, exact byte restoration
and replay, backup restore, stale-session denial and restarted new work. The
ordinary debug executable was fa40fce00f4006743fdb082ce60f275d9278fec5ed1334144a9975f63a6ec75f.
This result complements the fresh 363.29s pass in the preceding journal.

Campaign07 was intentionally stopped after this defect was reproduced, rather
than presenting an old candidate's runtime as qualification. Its 212 completed
segments remain in `target/c07-windows-characterization-07/evidence`; the exact
owned Windows Job was checked against its nonce, memory ceiling and native
process membership before termination. Outer launch receipt records failed,
exit 130, cleanup_confirmed=true, elapsed 2140859ms. It is incomplete, not a
24-hour pass. A repaired installed campaign, updated candidate CI and all
required profiles/soak remain outstanding.

Changed paths: database picker/admission, compaction regressions, this journal,
POAM-035 and generated navigation/inventory. No C-07, objective or alpha closure.
