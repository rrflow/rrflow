# H-01 context evidence locator correction

**Status:** active candidate correction evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-01-context-ci-correction`
**Owner:** OBJ-06/H-01 PR 31 candidate verification prerequisite

Starting revision: `9e1c901976ada93ae0b5991732369a38f863df71`.
PR: <https://github.com/rrflow/rrflow-development/pull/31>.
Candidate run 39364763536 failed the unchanged retired-identity architecture
test in verify, engine-authority and optional-engine lanes. The original
[implementation receipt](h-01-context-anchor-repair.md) named a local build
folder containing the prohibited retired composition name. This new node
explicitly records the correction of that locator to the existing PR artifact
receipt; the original text remains at the starting revision. All measurements,
commands, failures and limits in that receipt remain unchanged.

The initial 166-test local pass preceded the final journal text. It was valid
for that earlier tree, but did not prove the final candidate documentation.
The focused command `cargo test --locked -p rrd-engine --test workspace_architecture
retired_pre_release_identity_is_absent_from_the_active_repository -- --nocapture`
reproduced the failure locally: 0 passed, 1 failed, 30 filtered out. No policy
allowlist, test exemption, runtime or binary change is used to resolve it.

Read: README and AGENTS entry points, change-authoring procedure, complete
context implementation and mainline receipts, PR brief, the identity scanner
and failing architecture test, and the three failed CI job logs. The brief
was updated before correction. Scope is the locator, this correction node,
generated journal index and file inventory. No upstream research, resource
benchmark or new runtime trace is needed for a documentation-only correction.
Stop on altered runtime source, weakened checks or any remaining candidate
failure. No objective, roadmap, POA&M or release status changes.

After correction, the same focused test passed (1 passed, 30 filtered out),
then the full architecture target passed all 31 tests. Navigation generation
passed with 321 nodes and 1392 edges, inventory generation with 1235 records,
and documentation policy with 322 statuses and 320 classified coordinates.
No Rust implementation changed, so runtime tests and native artifact bytes
were not rebuilt for this locator correction. Changed files and full diff
were reviewed; ownership, baseline, scope, oracle, failures, non-runs and
stop decisions complete this correction's change checklist.

Final candidate validation and landing are recorded in PR 31. This
correction does not extend the native restart probe into complete reasoning,
cloud synchronization, PostgreSQL compatibility or release qualification.
