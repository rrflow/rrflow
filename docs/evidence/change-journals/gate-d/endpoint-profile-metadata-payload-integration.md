# PR113: bounded metadata payload repair integration

**Status:** active integration evidence; final candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/endpoint-profile-metadata-payload-integration`
**Owner:** PR113's existing alpha outcome and roadmap package; shared C-07f / POAM-035 prerequisite

## Change brief and traceability

[PR113](https://github.com/rrflow/rrflow-development/pull/113) received its
bounded integration brief before effects. Starting revision `7fdf8abd01f8f7e3da6b9a229e3f9574b0baf40f`;
ordinary-merge predecessor `71a8d60b0140bd3cded4318c7a5e57dedbadc838`. The ordered chain is PR129, PR105,
PR113, PR16, PR118, PR30, PR23, PR76, PR46, PR114. PR105 also integrates
accepted main `90373e90e6d04e4093e8e813a74d396bacb96fcf`.
No feature is reimplemented and no branch history is rewritten.

Current -> target: preserve existing package implementation while carrying
PR129's bounded metadata-payload correction `817a1b8b9c99fbc3dcc1907075333398c1f5f4b8` and predecessor source.
The capacity.rs/capacity_apfs.rs implementation and acceptance validator/tests
are byte-identical to the owning repair.
The [owning correction](../gate-c/c-07f-apfs-metadata-payload.md) retains
its deterministic first-failure and native failure evidence. The 25-second fill clock,8192-entry cap and shared payload allowance remain
unchanged;24additional APFS cases cover all four native failure boundaries.

Reviewed: the complete incoming diff, capacity helpers and acceptance tests,
shared containment helper/controller tests, owning metadata-payload journal, package's prior integration record and PR brief,
README, codebase-grounded authoring procedure and exact baseline/incoming diff.
No new external research or resource profiling is needed for verbatim source
integration. Existing bounded diagnostics and source-owner tests are preserved.

Conflicts: none. Only replaceable
inventory/navigation projections were resolved by regeneration. Every changed
non-documentation path was checked against the incoming Git blob or gitlink;
every other baseline implementation blob was checked unchanged. The mechanical
Git diff is the complete path inventory; this journal does not duplicate it.
Source/test ownership remains with each original package and its acceptance
records. Stop for a non-generated conflict, lost source/test, unexplained
change, widened resource bound or unsupported qualification claim.

Created: this immutable journal. Changed: inherited predecessor paths,
shared capacity helpers/tests, POAM035 support and generated navigation/inventory. No
existing journal is rewritten. No new engine or UI behavior is authored here.
PR114's optional Connectome source pin is not current-engine client qualification.

## Validation and limits

Results below are recorded before commit. Native CI must qualify this final
candidate tree before merge; an earlier shared-repair result does not qualify
another head. No local native APFS case, full resource profile or 24-hour soak
is claimed. C-07, POAM035 and alpha status remain open and unchanged.

Passed commands:

- `python scripts/ci/test_native_capacity.py`: passed.
- `python scripts/knowledge/render_navigation.py`: passed.
- `python scripts/ci/build_execution_inventory.py`: passed.
- `python scripts/ci/check_documentation.py`: passed.
- `python scripts/ci/check_workflow.py`: passed.
- `python scripts/knowledge/render_navigation.py --check`: passed.
- `python scripts/ci/build_execution_inventory.py --check`: passed.
- `git diff --check`: passed.

The focused controller suite passed all 14 tests. Source blob preservation
checks passed. Full engine/SDK suites were not rerun locally for a verbatim
merge; prior results retain their original revisions. Final candidate and
merge-group suites remain required. Checklist: brief before effects, owner
and failure trace, source-preserving ordinary merge, focused oracle, complete
diff review, immutable linked journal, explicit pending evidence and no status
upgrade.
