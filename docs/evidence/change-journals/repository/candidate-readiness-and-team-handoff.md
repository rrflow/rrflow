# Candidate readiness and team handoff

**Status:** active implementation evidence; candidate CI acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/candidate-readiness-and-team-handoff`
**Owner:** OBJ-01 / J-02 repository-delivery prerequisite / POAM-027

## Baseline and change brief

[PR125](https://github.com/rrflow/rrflow-development/pull/125) carried the brief
before implementation. Public development main was
`94d0567fb865fb49a90aaeeef4b5861707c9f8a7`; the empty package-opening commit was
`a630b7ad6bae36f3a111d4e9b85db47ec8ef6011`. Work uses an isolated worktree;
the original checkout and existing contributors' branches remain untouched.

The old caller automatically schedules the complete candidate matrix for
draft creation and updates, but not a transition to ready. The new caller
explicitly handles both readiness transitions and gates its one reusable
pipeline on ready PRs or merge/manual events. Returning to draft uses the
existing PR/ref concurrency to cancel stale work. Ready candidates retain
all checks, native platforms, timeouts and the same `pipeline / ci-gate`.

The new PR template exposes the existing brief, driver, dependency and
overlap ownership, concrete handoff, validation and qualification identity.
The owning change-authoring routine explains separate worktrees, explicit
integration order, failure retention, and an exact-source qualification
freeze. It creates neither a second roadmap nor an engine lifecycle.

## Scope and traceability

- `.github/workflows/ci.yml`: explicit activity list and `pipeline.if`.
- `scripts/ci/check_workflow.py`: `verify_candidate_admission` guards that
  small YAML envelope; the existing policy still covers all candidate jobs,
  source security, runner routing, toolchains and final reduction.
- `scripts/ci/test_candidate_admission.py`: eight tests against the actual
  caller, including mutations that remove transitions, filters, conditions,
  merge/manual coverage or safe concurrency.
- `.github/workflows/ci-reusable.yml`: run the new corpus in the existing
  repository-policy step; no native or engine command changes.
- `.github/PULL_REQUEST_TEMPLATE.md`: new forwarding brief/handoff prompts.
- `docs/operations/ci.md`, the owning change-authoring routine, and POAM-027:
  explain the scheduling contract and observed gap; no gate closure.
- This journal, its generated repository index and the generated execution
  inventory are the evidence/projection changes. No files move or disappear.

The old and new scheduling behavior share `jobs.pipeline` and the same
reusable `ci-gate`; no alternative acceptance path replaces them. PR23 owns
external ruleset evidence and PR30 owns toolchain changes; neither package
is superseded here. Live rules showed merge-queue enforcement during the
audit, but this package does not modify those external controls or duplicate
PR23's pending enforcement record.

## Research and instrumentation decisions

GitHub's [official workflow-event reference](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request)
documents that ready/draft transitions require explicit activity subscription.
No upstream implementation is adapted. Workflow admission is checked as a
deliberately restricted source shape, not a local emulation of GitHub's
expression evaluator. Hosted transition observation and full candidate CI
remain distinct evidence.

Existing workflow concurrency is the resource control. Runtime telemetry,
database traces, benchmarks, Rust recompilation and changing C-07 deadlines
are not relevant to this scheduling change. No throughput saving is claimed
from local policy tests. Other contributors' runs are not cancelled.

## Validation

The first characterization ran
`python scripts/ci/test_candidate_admission.py CandidateAdmissionTests.test_checked_in_caller_is_admitted`
against the unchanged caller. It failed as intended with the candidate-event
envelope violation, exposing the missing explicit activity subscription.
After the workflow change, `python scripts/ci/test_candidate_admission.py`
passed all eight tests (including mutation subcases), and
`python scripts/ci/check_workflow.py` passed: three workflows, nine substantive
job definitions, five engine suites, twenty workspace packages, six optional
feature packages and nineteen development-container pins.

`python scripts/knowledge/render_navigation.py` passed and updated one of
seventeen indexes (502 nodes, 2,091 edges). `python scripts/ci/check_documentation.py`
passed with 503 document statuses and 501 classified coordinates.
`python scripts/check_version.py` and `git diff --check` passed. Candidate CI
and live draft/ready observation are pending.
`python scripts/ci/build_execution_inventory.py` regenerated 1,595 records;
its `--check` passed. `python scripts/knowledge/render_navigation.py --check`
passed with no index drift. Inventory is regenerated after final journal edits.
Rust/native/storage suites were not rerun locally because runtime and native
test behavior do not change; the full candidate gate remains required.

## Review and handoff

Owner review followed README and AGENTS into the alpha objective/OBJ-01,
J-02, POAM-027 and change-authoring procedure. Implementation review read
both workflow files, the policy checker and new corpus, CI operations,
POAM-027, change-authoring, PR template and this journal in full, then the
complete authored diff. The current PR/rules/queue state was read through
GitHub. Generated navigation and inventory are checked mechanically; they
remain replaceable projections. The linked owners retain their own status.

| Required change checklist | Result |
|---|---|
| Owning outcome, baseline and brief before implementation | OBJ-01/J-02/POAM-027; baseline and PR125 above |
| Current/target behavior, exact scope and traceability | Same caller/reducer; only admission scheduling and repository handoff changed |
| Research, adaptation and trace/resource decisions | Primary GitHub documentation; no upstream adaptation or runtime tracing |
| First oracle and focused/package validation | Expected baseline rejection, eight passing regressions and full workflow policy |
| Failure and not-run reporting | Baseline failure retained above; hosted and runtime limits explicit |
| Full authored-file/diff review and linked evidence | Reviewed; journal index regenerated; inventory checked before commit |
| Completion and remaining errors | No objective/gate closure; C-07 native failures remain open |

Stop conditions remain a required-check bypass, undeclared scope expansion,
parallel status authority, cancellation of unrelated work, destructive
worktree cleanup, changed qualification limits or private promotion.

The C-07 integrated candidate remains PR123; PR124 adds timeout attribution
but does not fix the intermittent native failure. Their source must be
integrated before binding a new qualification campaign. The running Desktop
preflight retains its original identity and is characterization only; no
24-hour soak is claimed. This package does not close C-07, J-02, OBJ-01 or
POAM-027. The remaining native errors stay with POAM-035/C-07.
