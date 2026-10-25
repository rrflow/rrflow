# C-07d propagation composed with native lifetime and output limits

**Status:** active local qualification; accepted prerequisites and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-propagation-output-composition`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

[PR 67](https://github.com/rrflow/rrflow-development/pull/67) continues from
`3bd777239b0eec1c4d94f78a9fd416ac6aea2e35`, tree
`b55f46e15f5ea8fecde1e8e9cc74d79038804b9a`, under its existing brief and
[implementation record](c-07d-limit-propagation.md). Merge pending output
candidate `6a08b8ae900f37ea7d55d8aaf6d49d3deb959fc5`, which composes accepted
PR 64 with pending PR 65/66/69. PR 69 CI `39843394936` is still running.
This record does not accept those prerequisites or this final candidate.

## Canonical reconciliation and review

Four conflicts affected CI notes, POAM-035, workflow policy and generated
inventory. Preserve both independent prose sections and all native commands;
regenerate the inventory after staging conflict resolution. The full merged
diff and affected workflow/policy/records were reviewed. Native allocation,
reader lifetime and validation sources are byte-identical to the reviewed
output candidate (`git diff --cached agent/c07e-output-budget -- crates/persistence/rrd-lsm`
is empty). Propagation source, engine composition/transaction/error handling,
mutable inspection and semantic atomicity fixtures are unchanged from this
package's previously reviewed head (the corresponding staged source diff is
empty). No implementation was removed, moved or duplicated.

Reread the propagation journal and typed inspection history and reviewed the
combined options and diagnostic test changes. Existing output ownership and
clock research records apply; this is dependency composition with no new
algorithm, source adaptation, runtime tracing or RSS claim. Retain the
absent-durable-outcome condition for clearing known rejected intent and leave
unknown I/O uncertainty unchanged.

## Commands, failures and results

Windows used isolated `target/c07d-propagation-build`.

- The first focused command incorrectly named nonexistent `semantic_runtime`;
  Cargo exited 101 before executing tests. Tracked target discovery identified
  `semantic_commit_atomicity` and the separate `write_admission` targets.
- `cargo test -p rrd-store --test mutable_admission --test write_admission --locked`:
  4 passed, 0 failed, 0 ignored.
- `cargo test -p rrd-engine write_admission --locked`: 3 passed, 0 failed,
  0 ignored across the filtered package targets.
- `cargo test -p rrd-store -p rrd-engine --locked`: 417 passed, 0 failed,
  2 existing ignored cases across 49 result groups. This includes the full
  semantic atomicity target, so no focused semantic result is substituted.
- `cargo clippy -p rrd-store -p rrd-engine --all-targets --locked -- -D warnings`:
  passed.
- `cargo fmt --all -- --check`, workflow policy and staged whitespace checks:
  passed. Version/docs/generated navigation/inventory checks precede commit.

A log read used a root-relative path from the child worktree and failed;
reading the explicit root log resolved it. Neither command mistake changed
runtime state or test assertions. No compiler, runtime-test or lint failure
occurred after correcting the target. Original adapter and clock failures
remain in their immutable records.

The preimplementation checklist remains bound: outcome, exact owner/scope,
source/test traceability, smallest oracle, research/trace decisions and stop
conditions. This linked successor records composition and actual evidence.
No objective, roadmap or POAM completion changes. Not run: final native
candidate CI, installed custom-budget startup/restore, aggregate memory,
native exhaustion, capacity profiles or 24-hour soak. Native output/lifetime
suites passed on the input candidate, not independently rerun here; the
affected store/engine suites above qualify this composition. Reconcile accepted
predecessors before final candidate CI and stop on any failed gate or unknown
outcome reclassified as rejection. C-07 and alpha remain open.
