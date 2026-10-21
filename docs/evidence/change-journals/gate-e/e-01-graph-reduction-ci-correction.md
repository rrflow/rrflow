# E-01 graph reduction: candidate-CI oracle correction

**Status:** active correction evidence; Gate E remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-e/e-01-graph-reduction-ci-correction`
**Owner:** [Gate E](../../../roadmap/rrflow-1.0/gate-e.md), OBJ-04 prerequisite and [POAM-005](../../../poam/rrflow-1.0-alpha/poam-005.md)
**Pull request:** [42](https://github.com/rrflow/rrflow-development/pull/42)

## Baseline, failure and change brief

Correction starts at `88ad21181685ec1d1b0dcdae8a70b6bd4c8e99f3`, tree
`0f0f645ee3ab5c2586eb0593a7e9924ee1e0f3fc`. The
[original journal](e-01-incremental-graph-reduction.md) remains unchanged.
Its local 60-test query suite and lint/policy results are valid, but its first
candidate did not pass CI and cannot be merged.

[Candidate run 39518578908](https://github.com/rrflow/rrflow-development/actions/runs/39518578908)
failed both engine-authority lanes in
`causal_log_and_snapshot_reducer_uses_match_the_explicit_allowlist`.
The actual source inventory had three `CausalLogPage` calls in query
`tests/graph_traversal.rs`; the allowance expected two. The only added call
builds the independent full-history oracle for the new close/reopen test.
It does not execute on a production query path. The failed candidate was
cancelled after retrieving both failed job logs; it is not acceptance evidence.

The PR brief was extended before this edit to name the exact qualification
repair. Update the existing test-file allowance from two to three, describe
the reopen oracle, preserve every production restriction, then rerun the
cross-boundary guard and candidate CI. Stop if any production call needs a new
allowance or another discrepancy appears. This is part of the same bounded
E-01 repair, not a new roadmap package or a planning-order change.

## Traceability, scope and review

Read in full `crates/authority/rrd-engine/tests/direct_read_source_closure.rs`,
the already-reviewed new reopen test, and both CI failure details. The guard
scans all crate Rust sources for six sensitive read/reducer forms and compares
exact per-file counts against named allowances and tracked open deficiencies.
It continues to reject unlisted normal-path replay. The replacement is one
count and one test-oracle description in the existing `ALLOWED_USES` entry.
No production source, public symbol, format, schema, dependency or assertion
logic changes; no file is removed or moved.

Created this correction record; regenerated the Gate E journal index and
execution inventory. The original journal supplies the complete source,
research and implementation traceability. This correction adds no algorithm
or vendor research and no runtime trace/resource changes: CI's exact mismatch
is sufficient diagnostic evidence. Test-only full-log replay is deliberate
independent evidence, unlike the forbidden normal query replay.

## Commands and results

- Before the correction, `cargo test -p rrd-engine --test direct_read_source_closure --locked` reproduced CI: 0 passed, 1 failed, with the same single count mismatch.
- After the correction, the same command passed its 1 test.
- `cargo test -p rrd-query --test graph_traversal --locked`: 4 passed, zero failed; the independent oracle and reopen case remain active.
- `cargo clippy -p rrd-engine --test direct_read_source_closure --locked -- -D warnings`: passed.
- Navigation/inventory regeneration, `python scripts/ci/check_documentation.py`, `python scripts/ci/check_workflow.py`, `cargo fmt --all -- --check` and diff whitespace checks passed. Generated navigation has 17 indexes, 349 nodes and 1,536 edges; inventory has 1,286 paths.
- The original 60-test query suite is unchanged; no production code changed after that result. Full corrected engine/platform/SDK qualification remains the exact candidate CI gate.

The standard `gh run view --log-failed` command withheld logs while the full
run was active. The completed-job log API returned them; terminal escape
sequences were stripped from retained diagnostic output. This did not alter
CI outcomes. The unrelated read-only PowerShell tree-revision inspection
needed a quoted `HEAD^{tree}` argument; the corrected command returned the
baseline tree above.

## Checklist and remaining work

- PR scope/owner/outcome, actual versus expected behavior and stop conditions updated before edit.
- Exact failure retained; test-only source call traced to the independent reopen oracle.
- One named allowance corrected without weakening production read restrictions.
- New immutable correction journal linked; navigation/inventory regenerated.
- Changed source reread in full and complete correction diff reviewed before commit.
- Candidate CI must pass on the corrected exact revision before merge.

No roadmap, alpha, POA&M lifecycle or version status changes. No new product
deficiency was established; this was missing qualification metadata for an
already-authorized test. Broad runtime/resource/competitive qualification
remains as recorded in the original journal. No local full-workspace run or
new platform/benchmark claim is made by this correction. PR 42 carries the
corrected candidate workflow and merge receipt once available.
