# C-07e overlapping lifetime history baseline

**Status:** active bounded characterization brief; no new acceptance
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-interleaved-baseline`
**Owner:** OBJ-02 prerequisite, [C-07e](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

This seed permits a preimplementation pull request for the joined lifetime
history. It does not implement or qualify that history. Accepted development
main is `f4e404be38a0c21a4a3665b1907bad2226f5c13a`, tree
`e10ae21641df4eeea7ac647979d4f9268bce70ec`; its reader repair passed all 23
jobs in CI 39811509553 before PR 62 merged. Independent admission PR 63 is
still in CI. Reconcile accepted main before this package lands.

The existing `tests/support/projected_read_model.rs` owns the independent
version map and generated histories. Its pinned interleaving holds one stream
through a write, flush, compaction and GC. Separate compaction fixtures cover
transaction pins and checkpoint roots, and the accepted reader tests cover
empty admission and terminal source release. The missing joined schedule is
overlapping readers at distinct stamps, checkpoint release before the last
reader, concrete reclamation afterward, copied buffer clones beyond database
close, and native mapped/raw/LZ4 paths with controlled reader/writer threads.

The intended scope extends that model with one child support module and the
existing adversarial integration target; existing native CI partitions and
workflow policy require it. A reproduced source-lifetime defect may be repaired
only at the existing reader lease/finish or GC-root boundary. Broader admission,
format, recovery, scheduling or process-memory work stops this package.
No implementation is moved or deleted and no second expected-state model is
introduced. Exact scope, failure oracle and stop decisions are in the PR brief
before implementation; the successor journal records the actual outcome.

Read: README/AGENTS, change-authoring, C-07 and POAM-035; full model, adversarial
and projected-lifetime fixtures and Cargo manifest; reader lease/batch/terminal
ownership, database compaction/checkpoint/GC paths and the corresponding
compaction fixtures. Research/source adaptation is not needed: the existing
snapshot and ownership contracts supply the oracle. Preserve runtime traces;
use fixed schedules, bounded channels, explicit thread joins and phase-local
assertions. Expected rows come from requested mutations in the existing model.

First command: `cargo test -p rrd-lsm --test projected_read_adversarial interleaved --locked -- --nocapture`.
Then full adversarial/compaction/lifetime fixtures, complete LSM suite, strict
all-target lint, formatting and repository projection/policy checks. The first
new fixture must characterize accepted code before any runtime correction.
No new runtime test has run at this seed; no performance or correctness result
is inferred. Candidate CI, exact revision and merge remain required.

The package checklist is carried by the preimplementation brief and the
successor immutable journal: baseline, bounded outcome, read/source map,
first oracle, research/resource decisions, actual failures/results, final
file/diff review, regenerated projections and remaining limits. C-07/POAM-035
and alpha stay open; installed GC, aggregate retained buffers/native memory,
real capacity exhaustion, capacity profiles and the 24-hour soak are not
qualified by this native lifetime schedule.
