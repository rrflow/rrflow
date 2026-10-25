# PR16: joint APFS pressure repair integration

**Status:** active integration evidence; final candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/knowledge-trees-joint-pressure-integration`
**Owner:** PR16's existing alpha outcome and roadmap package; C-07f / POAM-035 prerequisite

[PR16](https://github.com/rrflow/rrflow-development/pull/16) received the
bounded integration brief before effects. Driver: current Codex session.
Starting revision `0422579c5a18faccc0dd399edcddf590b0be278b`; ordinary-merge predecessor `177c4d1eee36ac0556dc6450be5c74057a374431`.
This preserves the existing ordered PR129/105/113/16/118/30/23/76/46/114 stack.
No history rewrite, new feature path or protected-main mutation is performed.

Current -> target: preserve the existing package while integrating owning
repair `f1b6bd851fffac703171e1b3e2e3aa5f2a8a740d`. The [repair journal](../gate-c/c-07f-apfs-joint-pressure.md)
retains PR113's native WAL failure, exact raw log and red/green regression.
APFS data and metadata pressure repeat until a full round adds neither
bytes nor files, under the original shared clock and byte allowance.
The APFS WAL phase remains outside the authoritative WAL directory. Runtime APIs, workloads, caps and native recovery oracles remain.

Reviewed the incoming diff, complete capacity.rs, capacity_apfs.rs,
capacity_observation.rs, owning repair journal and this package's preceding
integration record/PR brief. All three native capacity jobs passed before this local integration;
the complete shared candidate gate remains pending. Dependent PRs remain drafts
until that shared full gate passes. No new research or instrumentation is needed for identical source
integration; existing primary sources and bounded diagnostics remain owners.

Conflicts: none. Only generated
navigation/inventory may be resolved by regeneration. All changed non-doc
Git blobs/gitlinks are checked against incoming; every other implementation
blob is checked unchanged. The capacity source and acceptance validators must
match the shared repair byte-for-byte. This is the source-preservation oracle;
no feature implementation or tests are deleted or duplicated.

Created: this immutable journal. Changed: inherited repair/evidence and
regenerated discovery projections. Historical journals remain untouched.
Stop for a source conflict, lost behavior/test, unexplained blob change,
widened limit, external effect or unsupported completion claim.

The local integration checks below precede commit. Full candidate and
merge-group CI must still qualify this PR's exact final tree. No unrelated
engine suites are repeated locally for identical fixture source. C-07,
POAM035, sustained profiles and 24-hour soak remain open; this does not qualify
Connectome against the current engine or claim alpha completion.

Passed commands:

- `python scripts/knowledge/render_navigation.py`: passed.
- `python scripts/ci/build_execution_inventory.py`: passed.
- `python scripts/ci/check_documentation.py`: passed.
- `python scripts/ci/check_workflow.py`: passed.
- `python scripts/knowledge/render_navigation.py --check`: passed.
- `python scripts/ci/build_execution_inventory.py --check`: passed.
- `git diff --check`: passed.

Source blob preservation
checks passed. Full engine/SDK suites were not rerun locally for a verbatim
merge; prior results retain their original revisions. Final candidate and
merge-group suites remain required. Checklist: brief before effects, owner
and failure trace, source-preserving ordinary merge, focused oracle, complete
diff review, immutable linked journal, explicit pending evidence and no status
upgrade.
