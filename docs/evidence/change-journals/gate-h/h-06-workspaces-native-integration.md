# PR 114: shared native candidate integration

**Status:** active integration evidence; candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/h-06-workspaces-native-integration`
**Owner:** H-06 / POAM-011

Starting head `a90194591f3aee75ae865448cd5cf12dab287c4e` integrates PR129
`d0e7f54c1496bf55c1a0932c6f4d574cae21545f` by ordinary merge.
The existing PR description bound the change before effects; the
[prior integration](h-06-workspaces-accepted-main.md) retains its original exact-source evidence.
No merge conflict occurred. Regenerate projections from the combined
tracked owners, preserving each branch's authored source and evidence.

The shared source remains in its existing canonical owners: segment-v6
incremental encoding and scratch reuse; single-cursor public database views;
bounded native WAL observation; repeated APFS qualification and its mandatory
receipt-validation step. Preserve their tests and original journals. This
integration does not replace or move this PR's behavior or tests. Reviewed the
incoming source/test/workflow diff, original integration record and package
specific automatic merges. No new upstream adaptation, external runtime trace,
resource limit or version change is necessary. Unexpected semantic conflicts
stop the package; the complete final integration diff is reviewed before commit.

Local acceptance commands are `python scripts/ci/check_documentation.py`,
`python scripts/ci/check_workflow.py`,
`python scripts/knowledge/render_navigation.py --check`,
`python scripts/ci/build_execution_inventory.py --check` and `git diff --check`.
Navigation and inventory are regenerated after this final journal text; all
listed checks passed. No Rust, external-provider, native
capacity, container rebuild or UI acceptance is claimed from these checks.
The final exact candidate and merge-group pipelines remain mandatory.

The separately retained file-handle exhaustion is unresolved; no 24-hour soak
has started. PR129's native crash/capacity jobs passed but its full candidate
remains pending at this review. This branch stays draft until that dependency
qualifies. Other candidates' results cannot qualify this tree. C-07, alpha,
roadmap and POA&M completion status do not advance.

Checklist: pre-effect scope, exact source identities, preserved canonical
behavior/tests, no hand-maintained generated resolution, linked immutable
journal, package-specific checks, named omissions and final diff review.

Connectome stays pinned to `38142b77110a8f1db77e306e7d1ad375fd918267`;
no child source or gitlink changed. Its old knowledge editor is reserved for
the owner-approved Slice D rebuild, not revived by this integration.
