# H-06 native workbench mainline merge

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-06-native-turnkey-mainline-merge`
**Owner:** PR27 mainline integration receipt for the H-06 workbench prerequisite

[PR27](https://github.com/rrflow/rrflow-development/pull/27) initially could not
start candidate CI: main advanced to `13ce3386248e87e9d3be1e94969ec8f408eb7317`
while the workbench was built from the earlier native engine input. The branch
head before integration was `e42ca3cd48c52c38972da2dbb12abce089e2e2da`.

The mainline change is merged without rewriting history or modifying its
engine/source behavior. Git reported one conflict, in the generated execution
inventory. Regenerate that projection from the merged tracked sources and
regenerate navigation to retain both journals. The optional Connectome pin and
its engine/SDK input remain the exact separately tested revisions in the
[original workbench receipt](h-06-native-turnkey-workbench.md).

Reviewed: merge status and complete path diff, execution inventory generator,
new mainline flight/query diff and journal, current objective/POAM owners,
and original workbench receipt. Scope is the merge, this receipt, generated
navigation and file inventory. No deletion or adapted source implementation;
no new trace/resource instrumentation, external research or performance claim.
Failure oracle: unresolved conflict or stale generated output must fail.

Acceptance: navigation, execution inventory, documentation and workflow checks
are rerun after merge; the owning 31 workspace architecture tests passed on the
workbench candidate before integration. Full merged-source behavior is checked
by PR27 candidate CI before landing. Stop on unresolved conflicts, new owner
changes or child/parent CI failure. The prior receipt remains immutable. No
objective, gate or POAM lifecycle status changes.

Actual local results: the first inventory regeneration correctly refused an
unmerged index entry. After staging resolution and regenerating, inventory and
its check passed (1212 records); navigation passed (14 indexes, 308 nodes,
1341 edges); documentation passed (309 statuses, 307 coordinates); workflow
policy and staged whitespace/conflict-marker checks passed. No authored
engine path differs from the new mainline. Candidate CI remains pending.
