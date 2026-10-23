# C-07e output and mutable admission composition

**Status:** active local dependency composition; accepted-base reconciliation and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-output-mutable-composition`
**Owner:** OBJ-02 prerequisite, [C-07e](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

[PR 66](https://github.com/rrflow/rrflow-development/pull/66) continues from
local `530bbe1f1798743adaab9f755ec929ad491bb026`, tree
`1e065c2f1e62d2bc9297e2c2382695d0e9301df5`, under its existing brief and
[implementation record](c-07e-output-budget.md). Merge the pending mutable
policy at `e4f546d1b37861e7bc8297db4a60b8ee206acffb`; it incorporates
accepted reader-history main `c8d8e2e2aa1bc3f3e7ca28a9aa2603b36a010e58`.
The mutable policy is still pending acceptance and not claimed qualified here.

Six conflicts occurred: native `DatabaseOptions::validate`, workflow, workflow
policy, CI notes, generated journal index and file inventory. Keep the one
crate-visible validation method required by read-only open AND its output-cap
validation; retain mutable and output fields/defaults/validation at their one
owner. Keep every focused mutable/output/lifetime command exactly once per
native job. CI notes retain both scopes. Regenerate projections after staging
conflict resolution. No alternative policy, reader or restore path was added.

The existing zero-output-limit fixture now additionally rejects writer and
read-only reopen with a zero cap and then reopens the prior acknowledged value
under valid options. This characterizes the shared validator after composition.
Other native runtime changes are the pending mutable policy already documented
by its owner. Reviewed the combined native options and admission/replay changes,
updated fixture, workflow/policy, supporting records and complete merge diff.
No new research, source adaptation or runtime diagnostic change is needed.
The earlier Arrow allocation ownership research and limits remain applicable.

Windows used isolated target `target/c07e-output-build`.
`cargo test -p rrd-lsm --test projected_output_limits --test mutable_limits --test projected_read_adversarial --locked`
passed 19 tests with no failures/ignores before and after the additional
reopen-validation assertions. Format and workflow-policy checks passed.

The full combined suite is intentionally pending the independently recorded
[PR 69 diagnostic oracle repair](https://github.com/rrflow/rrflow-development/pull/69).
PR 65's prior combined suite exposed a zero-duration assertion; do not silently
rerun that old oracle and cite a lucky pass. Compose the repaired candidate,
repeat the affected complete suites and strict lint, then reconcile accepted
predecessors and require exact-candidate native CI before final landing.
No new compile/runtime failure occurred in this focused composition.

The preimplementation brief still binds owner/scope/oracles, source-adaptation,
trace/resource decisions and stop conditions. This successor records baseline,
conflicts, canonical destinations, tests, failures and remaining checks.
Regenerate linked navigation/inventory and verify frozen version, docs and
staged diff before commit. No objective, POAM closure, C-07 package or alpha
completion status changes. Prior Linux/macOS/Windows receipts accept the
reader package only, not this pending combined resource implementation.

Not run for this composition: full native/store/engine suites and strict lint,
final candidate CI, installed selected-budget flow, aggregate heap/RSS,
true capacity exhaustion, declared capacity profiles or 24-hour soak. Pool
scope remains one native open instance; user copies, source pages, allocator
metadata, retained mutable generations and complete query memory remain outside
this output reservation. Stop on premature release, lost ACK, policy bypass,
uncharged returned output, expanded scope or failed candidate CI.
