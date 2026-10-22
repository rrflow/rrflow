# C-07a diagnostic clock oracle baseline

**Status:** active bounded diagnostic-oracle repair; implementation and CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07a-diagnostic-clock-baseline`
**Owner:** OBJ-02 prerequisite, [C-07a](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

Start at accepted main `c8d8e2e2aa1bc3f3e7ca28a9aa2603b36a010e58`,
tree `75bcff09727bb5882338465f28e9819ab85b758e`. PR 64 passed all 23
candidate checks in run `39832685534` and merged on 2026-10-22.

PR 65's combination with the reader-history candidate reproduced a zero
uncontended commit mutex duration, failing the existing store operator oracle
at its strictly-positive assertion. The combined suite stopped after 298 passes,
1 failure and 1 existing owned-child ignore. Native/storage effects were not
reported corrupt by this assertion; the failure is retained for correction.

The bounded brief selects the existing store operator and native diagnostics
tests, required native CI/policy, supporting CI/POAM evidence and projections.
[Engine clock documentation](https://doc.rust-lang.org/std/time/struct.Instant.html)
defines nondecreasing observations and permits zero elapsed duration; it does
not identify the hardware cause of the observed sample. Do not fabricate
positive timing or add sleeps/retries. Compare real measurements with an outer
operation duration, preserve exact zero/nonzero/saturating forwarded context,
bounded collection and durable semantic readback. Existing I/O oracles retain
physical synchronization responsibility. No runtime code or public contract
change, and no full C-07/alpha completion claim.

The first failure is already observed; implementation will run focused targets,
full native/store suites and strict lint, then exact-candidate native CI.
Full-file/diff review and all results/remaining gaps belong in a successor
journal. Stop on false timing, lost diagnostics or effects, durability-check
weakening, expanded scope or failed CI.
