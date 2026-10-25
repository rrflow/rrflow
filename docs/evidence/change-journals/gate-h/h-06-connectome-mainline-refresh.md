# H-06 Connectome mainline refresh

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-06-connectome-mainline-refresh`
**Owner:** H-06 source-association candidate integration receipt

Starting candidate: `f0cbbc869d6a6c43964a31e89c63935cf0f4414a` in
[PR #20](https://github.com/rrflow/rrflow-development/pull/20), OBJ-09 and
H-06/H-07 prerequisite. Main advanced to merge `9bcc6154` while publication
checks ran. The PR brief was updated before integrating it.

Scope: preserve main's prompt-flight implementation without modification and
regenerate the file inventory and journal navigation. The only merge conflict
was `docs/roadmap/rrflow-1.0-file-plan.jsonl`; authored system-overview,
Connectome client-contract and journal-index changes merged automatically.
No runtime behavior is adapted, moved, deleted, or replaced by this follow-up.

Read/reviewed: main's commit list and complete changed-path inventory; exact
overlap diffs in system overview, client contract and journal navigation;
source-association publication receipt and current PR brief. The repository
generator owns the conflicted inventory, so no competing prose inventory is
authored. No additional external research, tracing or profiling is needed.
Stop on a source/runtime conflict or changed UI pin.

Acceptance: `python scripts/knowledge/render_navigation.py` and `--check`
passed (14 indexes, 295 nodes, 1275 edges);
`python scripts/ci/build_execution_inventory.py` and `--check` passed
(1196 records); `python scripts/ci/check_documentation.py` passed
(296 statuses, 294 coordinates); `cargo test -p rrd-engine --test
workspace_architecture --locked` passed all 31 tests; `git diff --check`
passed. The initial inventory generation correctly rejected the unmerged
index entry. After resolving that index entry, regeneration replaced the
conflicted generated contents and both inventory checks passed.
Full runtime/UI suites are delegated to candidate CI; the upstream engine
implementation is preserved, and local source checks cannot replace CI.

The [publication receipt](h-06-connectome-public-association.md) remains
historical evidence. No roadmap gate, POA&M lifecycle status, product version,
UI pin or official promotion changes. PR #20 must pass its candidate pipeline
before merge; this receipt claims no merge or runtime qualification.
