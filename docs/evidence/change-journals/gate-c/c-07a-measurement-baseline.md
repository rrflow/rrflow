# C-07a measurement baseline

**Status:** active characterization evidence; C-07 remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07a-measurement-baseline`
**Owner:** OBJ-02 prerequisite, [C-07a](../../../roadmap/rrflow-1.0/gate-c-07.md)

Starting revision `6d63bea79094bea13c0487090d889dde315a8fbd` was fetched from
public development main and matched the clean worktree. The branch is
`agent/c07a-storage-measurement`. The user requested execution of the accepted
C-07 plan. This package implements its first measurement/policy contract;
later native fault, enforcement and capacity packages remain separate.

`cargo test -p rrflow-eval` passed with **zero tests**. This is inventory, not
coverage. `cargo run --locked -p rrflow-eval -- verify
eval/results/2026-08-18-summary.json` passed: eight retained trials, eight
successes, zero regressions. No live provider was invoked.

The complete existing evaluation manifest and main source were read.
`Trial`, `run_trial`, `metrics`, `summarize` and `verify` remain in place;
new storage dispatch must preserve their behavior and add characterization.
Targeted reads of LSM policy/WAL types, store physical evidence, engine
diagnostic projection and public diagnostic/configuration types confirm that
soft maintenance targets are not aggregate hard memory limits. The public
configuration currently has clock/reasoning/recall/query settings, not the
future C-07d storage policies. No runtime defect is inferred from that gap.

README, AGENTS, C-07, POAM-035, change-authoring, implementation navigation
and CI owners were inspected; OBJ-02, J-04 measurement and dependency-order
sections bind the package. Schema generation will use the existing locked
schema library and public contracts rather than duplicate operation models.
The C-07 research decisions remain applicable; no upstream source adaptation,
runtime trace, benchmark or service activation is needed for this baseline.

Before this seed, the exact brief was written locally. GitHub requires an
ahead commit to open a PR, so this factual journal and generated projections
enable the PR description before code changes, as in the preceding packages.
The final journal will retain the complete change checklist, commands,
failures, actual evidence and omissions. No gate or POA&M status changes.
Exploratory path/glob misses were resolved through repository discovery;
they are not implementation failures. Full candidate CI is required before
landing; native exhaustion, power loss, sustained scale and full alpha are
not claimed.
