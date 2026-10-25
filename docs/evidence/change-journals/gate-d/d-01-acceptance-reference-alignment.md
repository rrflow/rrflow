# D-01.6 supporting-reference alignment

**Status:** active correction evidence; fresh documentation candidate CI required
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-acceptance-reference-alignment`
**Owner:** OBJ-07 prerequisite, D-01.6, POAM-006/007 reference accuracy

Starting revision `533441484b1911ca3aeb3914fc0d38c3e1205dad`, tree
`3c31bcea84a1ad989f1d531d131dfac82b6193df`.
[PR 53](https://github.com/rrflow/rrflow-development/pull/53)'s brief was amended
before these edits. The prior [reconciliation journal](d-01-acceptance-reconciliation.md)
and measured receipts remain immutable.

## Observed mismatch and bounded correction

A final `rg` scan for active/open D-01 claims found stale status in the roadmap
index and installed-lifecycle reference. Full-file review also found obsolete
claims that no installed specialization existed, that the generic profile
hardcoded Clyffy, and that privileged ownership remained unqualified. The
roadmap's overview table still said D=0/11 and C=6/7 although the canonical
checkboxes now say D=1/11 and C=5/7 (C-04 remains reopened).

The roadmap index and overview now link the canonical checklist/dependency spine
instead of duplicating their active pointer/counts. Lifecycle references link
the accepted bounded D-01 matrix while preserving broader release, clock,
filesystem and operational limits. Bootstrap/seat references and POAM-007 now
distinguish the accepted versioned generic/Clyffy installation subset from
unfinished full attunement and provider conformance. POAM-006 records the
observed documentation drift and this correction. Neither POA&M closes.

Complete files read: `docs/roadmap/README.md`,
`docs/roadmap/rrflow-1.0/current-alpha-objective-and-remediation.md`,
`docs/reference/deployment/installed-lifecycle.md`,
`docs/reference/agent-bootstrap.md`, `docs/reference/seat-identity.md`, and
`docs/poam/rrflow-1.0-alpha/poam-007.md`; POAM-006 and the owning README,
objective, roadmap and evidence were already fully read in this package.
The execution-checklist chapter and relevant documentation index links were
also checked. Review uses the accepted D-01 bootstrap/ownership evidence;
it adds no new semantic or runtime behavior.

Traceability: replace only subordinate handwritten status/count summaries with
links to their existing canonical owners. No implementation, contract requirement,
historical journal or measured artifact is moved, deleted or rewritten. Exact
scope is the six supporting records, POAM-006's correction link, this journal
and generated navigation/inventory. No extra gate closes; C-04 remains open.

Research/source-adaptation and trace/resource/debug decisions: no new research,
upstream code, telemetry or resource policy is needed for reference alignment.
The smallest oracle is a stale-status scan plus mechanical checkbox comparison,
followed by documentation/link/navigation/inventory/diff checks. Runtime tests
are not repeated locally because runtime/test code remains unchanged. The full
fresh PR candidate pipeline still gates merge.

## Verification

The stale-status scan returned no remaining `D-01 remains open`, `D-01 is now
active`, `D-01 active`, `D-01.5/.6 still`, `installed specialization does not`, or
unqualified privileged-owner claims in the current README/roadmap/objective/
operation/reference scope. Historical journals and dated POA&M observations
are intentionally preserved. Complete corrected bodies and the change diff
are reviewed; generated indexes and inventory are regenerated before commit.

The preceding candidate's run `39680718281` had fifteen successful jobs and
passed all three native isolated receipts at the time this correction was
prepared. Those receipts were downloaded and independently validated against
that exact candidate, with five passing cases and both durable modes each.
It is superseded because supporting documentation changed; no pending final
gate or future merge is claimed as passed. D-01.5's accepted implementation run
`39674145683` and immutable retained measurements are unchanged.

Checklist: evidence mismatch reproduced; exact scope added to PR before edits;
complete supporting records read; one-owner links selected; no runtime change;
errors/limits and unrun checks explicit; immutable correction journal linked;
generated projection and final candidate CI required. Stop on any changed gate
acceptance, runtime scope expansion, artifact mutation or unsupported status.

Final local checks: `python -X utf8 scripts/knowledge/render_navigation.py --check`
passed with 17 indexes, 376 nodes and 1,636 edges;
`python -X utf8 scripts/ci/check_documentation.py` passed with 377 document
statuses and 375 classified coordinates. `git diff --check` passed. The
complete seven-record correction diff was reviewed. The prior candidate's
run subsequently completed successfully; it remains prior-candidate evidence,
not acceptance of this changed revision. Inventory is regenerated and checked
after this journal update. No local runtime suite was rerun.
