# H-06 Connectome submodule policy integration

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-06-connectome-submodule-policy-integration`
**Owner:** follow-up receipt for H-06 source integration against current main

## Baseline and change brief

Gate/package and alpha outcome or prerequisite advanced: the same OBJ-09,
H-06/H-07 source-integration prerequisite in
[PR #20](https://github.com/rrflow/rrflow-development/pull/20).

Starting revision/tree and pull request: initial candidate
`6570f200403a7ea48d983e9bd32579b9546c0c4a` conflicted with development main
`b298fa968c3fcd69616b456abe394c2dbdc88094`. The upstream
[policy receipt](../repository/pinned-git-submodule-source-input-policy-journal.md)
records the owner's acceptance of declared, pinned HTTPS submodules and the
requirement to fetch them recursively in CI. The PR brief was updated before
resolving the overlapping source-policy changes.

Change brief (current -> target behavior, owner, exact scope, unchanged
behavior, stop conditions): preserve main's general declaration validator and
negative tests; retain explicit Connectome identity and engine-source isolation;
set recursive acquisition on all nine existing workflow checkout steps; align
the instruction, architecture, client, CI, and POA&M owners with that policy.
Keep the UI pin, product positioning, inventory implementation, runtime, and
gate statuses unchanged. Stop short of changing private source visibility,
credentials, or claiming CI/real-client qualification.

The [initial journal](h-06-connectome-submodule-and-delivery-positioning.md)
remains an immutable receipt for its initial candidate. Its exclusive-submodule
policy and engine-only CI checkout description are superseded by this
integration; its local test results remain historical evidence.

## Scope and traceability

Files and symbols read in full at the affected boundary: upstream AGENTS source
policy; `workspace_source_inputs_are_repository_closed`,
`SubmoduleDeclaration`, `declared_submodules`, `submodule_violations`,
`submodule_policy_accepts_only_declared_pinned_https_submodules`, and the new
Connectome identity test; `verify_workflow_security` and `checkout_steps`;
all nine workflow checkout blocks; the upstream policy journal; the affected
source-distribution, CI, client-checkout, and POAM-011 sections.

Implementation-requirements traceability: the initial exclusive registration
helper and synthetic cases converge into main's general declaration validator
and its negative cases. Exact Connectome URL/path registration remains one
additional test; engine member/target/dependency exclusion remains in source
closure. The result accepts other properly declared pinned inputs under main's
policy without letting the Connectome UI supply engine source. No engine
runtime was removed or replaced. H-06 remains the owning gate.

Implementation/evidence paths created, changed, moved, or deleted: resolve
AGENTS and the architecture test against main; update the existing architecture,
client, CI, and POAM-011 owners; add `submodules: recursive` to the seven
checkouts in `ci-reusable.yml` and two in `rrd-lsm-benchmark.yml`; create this
receipt and regenerate its index and the file inventory. Main's policy script
and historical journal are preserved unchanged.

Research decision and primary-source/adaptation record: inspect the exact
upstream commit and GitHub repository metadata. No outside implementation was
adapted. Trace/resource/debug decision: no runtime instrumentation or resource
claim; source-policy messages and test output suffice.

## Evidence and remaining blocker

First-failure or characterization oracle and result: GitHub reported the initial
PR as conflicting and scheduled no candidate run. Merging current main exposed
overlaps in AGENTS, the architecture test, and generated inventory. Resolve
authored policy against the accepted upstream owner and regenerate the inventory.

Acceptance commands and exact results:

- `cargo test -p rrd-engine --test workspace_architecture --locked`: 31 passed
  after integration, including the retained generic negative corpus and new
  exact Connectome registration test.
- `python scripts/ci/check_workflow.py`: passed with all nine checkout steps
  recursively acquiring submodules and credentials still not persisted.
- `cargo fmt --all -- --check` and `cargo clippy -p rrd-engine --test
  workspace_architecture --locked -- -D warnings`: passed. The first formatting
  check found two assertions formatted with the wrong edition by a direct
  rustfmt invocation; `cargo fmt --all` applied the workspace settings.
- `python scripts/ci/test_execution_inventory.py`: 4 passed;
  `python scripts/knowledge/test_navigation.py`: 6 passed;
  `python scripts/knowledge/test_export.py`: 11 passed after regenerating the
  new receipt's index. Its first run correctly rejected the missing index link.
- Navigation generation and `--check`: 14 indexes, 291 nodes, 1257 edges;
  inventory generation: 1188 records; documentation policy: 292 statuses and
  290 classified coordinates; `git diff --check`: passed.
- Post-commit proof on initial candidate `6570f200`: after successful
  `git submodule deinit -- connectome`, all 30 then-current architecture tests,
  inventory `--check`, and documentation policy passed; the exact pin was
  restored with `git submodule update --init -- connectome`. This proves local
  engine-check independence, not compliance with recursive candidate checkout.

Surfaced failures and corrections: the initial candidate and new main edited
the same source policy. The generic policy was preserved rather than reverted;
the prior blanket/sole-submodule validator was removed in the same integration.
The private-source acquisition prerequisite remains unresolved.

Remaining known errors and owner: `gh repo view rrflow/connectome-development
--json isPrivate,url` still reports `isPrivate: true`. Main's source policy
requires anonymous clean acquisition, so candidate recursive checkout cannot
qualify yet. The upstream policy journal already records source review pending
with the repository owner. No private content is published and no token is
added. POAM-011 records this concrete prerequisite; the PR stays draft.
An unauthenticated `curl.exe -q --silent --show-error --output NUL --write-out
'%{http_code}' https://api.github.com/repos/rrflow/connectome-development`
returned HTTP 404, corroborating the anonymous-access limitation.

Checks not run and reason: hosted/UI/runtime/release acceptance was not rerun;
no such behavior changed. Candidate CI is recorded on PR #20 and cannot be
substituted by local tests. Do not merge unless its required checks pass.

Full-file reread and diff review: affected implementation symbols, all checkout
blocks, changed authored text, and the complete merge diff were reviewed;
generated navigation/inventory were rebuilt from owners.

Change checklist: owner policy preserved; exact source pin retained; upstream
tests carried forward; private access blocker recorded; no credential,
visibility, runtime, or version change; original evidence preserved; navigation
and inventory regenerated; PR/candidate-CI landing retained.

Roadmap/POA&M status change, or explicit none: no gate or lifecycle completion;
POAM-011 now identifies anonymous source acquisition as a candidate-CI blocker.
Pull request, CI run, and merge evidence: PR #20 carries the integrated
candidate and checks. No merge to main is claimed by this receipt.
