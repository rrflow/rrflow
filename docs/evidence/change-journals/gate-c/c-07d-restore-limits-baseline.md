# C-07d configured restore policy baseline

**Status:** active bounded dependency baseline; implementation and CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-restore-limits-baseline`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

Accepted main is `dde6ef86d278b8c59cf1f61ebc2d3923186396d5`, tree
`bc76b62d5a500395fb8e1193dcd4d94eeac79ada`. Local implementation depends
on PR 67 at `3bd777239b0eec1c4d94f78a9fd416ac6aea2e35` and its native
policy dependency. Reconcile accepted PR 64/65/66/67 before final candidate
qualification; pending dependency code does not establish acceptance.

Archive restore creates/reopens staging three times with defaults. Application
backup restore adds catalogue/control replay and verification opens that also
use defaults. This would bypass a future installed policy. The bounded package
will pass one native options value through the existing authenticated
restore/resume/publication implementation and all nested backup helpers.
Default entry points may delegate but cannot become a second restore authority.
No installed setting, archive format, repair, dependency or version changes.

The preimplementation brief binds archive restore/export, backup restore and
verification helpers, focused store tests, native CI and evidence/projections.
Read owner chain, native options/typed admission, existing archive action/receipt
prefix and backup catalogue/control replay boundaries plus their resilience
fixtures. Full changed-file reread/diff review precedes final commit.

First oracle must reproduce ignored configured byte/version caps, then prove
prior durable staging prefix, denied public publication, exact admitted resume
and subsequent reopen. Application-complete coverage includes catalogue,
objects and control history; retained source/archive bytes and acknowledged
state cannot change. Low-budget replay/verification preserves existing bytes.
Unknown failures retain staging; never label partial restore as complete.

No runtime test has run for this seed. Existing uncertainty and restoration
contracts supply the oracle; no new external research or copied algorithm.
Preserve traces; fixed histories and filesystem evidence are component proof,
not RSS, installed policy, native capacity exhaustion or soak. Record actual
commands, surfaced failures, review, not-run checks and generated projections
in a successor journal. Stop on lost/duplicated effects, changed source,
published partial target, ignored options, expanded scope or failed CI.
C-07 and alpha remain open.
