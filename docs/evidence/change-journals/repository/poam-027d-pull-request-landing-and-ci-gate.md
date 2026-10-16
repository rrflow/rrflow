# POAM-027d pull-request landing and CI gate

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/poam-027d-pull-request-landing-and-ci-gate`
**Owner:** evidence for the repository change-landing decision recorded by POAM-027

This record reports one repository-process package. It does not close
POAM-027, advance a roadmap checkbox, or change runtime behavior.

## Evidence record

```text
gate/package and alpha outcome or prerequisite advanced: POAM-027d / J-01 and J-02 prerequisite — every change reaches main through a pull request whose candidate CI passes; one linked journal per package
starting revision/tree and pull request: clean baseline cfa0e8073c1a5479427beb29c7c88ee0a71f2f01 on main; branch process/pr-ci-one-journal; pull request not yet opened
change brief (current -> target behavior, owner, exact scope, unchanged behavior, stop conditions): current — AGENTS.md requires a planning-only commit of docs/roadmap/rrflow-1.0-active-change.json and CI runs scripts/ci/check_change_plan.py, while candidate CI triggers only on pull_request, merge_group, and workflow_dispatch and 218 commits since 2026-08-19 reached main by direct push. Target — changes land only through pull requests whose candidate CI passes; the pull request description carries the change brief; one linked journal records evidence; the planning-only commit, the active-change record, and the validator are retired. Scope: AGENTS.md, change-authoring procedure, execution map, implementation navigation, CI operations record, two automation references, POAM-014, POAM-027, the CI workflow, check_workflow.py, check_documentation.py, build_execution_inventory.py, removal of check_change_plan.py and test_change_plan.py, this journal, and regenerated indexes and file plan. Unchanged: runtime code, wire schemas, SDK behavior, dependencies, release version, roadmap order and checkboxes, and the active-change file itself, which stays as historical evidence linked from earlier journals. Stop on a broken link, a removed required owner, or any runtime change.
files and symbols read in full: AGENTS.md; docs/roadmap/rrflow-1.0-execution/change-authoring.md; docs/roadmap/rrflow-1.0-execution-map.md lines 1-40; docs/poam/rrflow-1.0-alpha/poam-014.md; docs/poam/rrflow-1.0-alpha/poam-027.md; docs/operations/ci.md lines 70-135; implementation-navigation.md lines 15-80; .github/workflows/ci.yml; .github/workflows/ci-reusable.yml lines 72-180; the change-plan-related sections of check_workflow.py (325-430), check_documentation.py (25-45, 355-405, 575-615, 820-905), and build_execution_inventory.py (1-30, 1715-1782).
implementation/evidence paths created, changed, moved, or deleted: see the pull request diff; deleted scripts/ci/check_change_plan.py and scripts/ci/test_change_plan.py; created this journal; regenerated docs/roadmap/rrflow-1.0-file-plan.jsonl and the repository journal index.
research decision and primary-source/adaptation record: not required; repository-local Git history and workflow triggers are the primary evidence. No external code or dependency adapted.
trace/resource/debug decision (add/preserve/not applicable, with reason): not applicable — no runtime operation, persisted state, or resource path changed.
first-failure or characterization oracle and result: at baseline every policy check passed except scripts/ci/check_generated_surfaces.py, which failed with "python endpoints is stale: missing OpenAPI digest 0ef644d8…"; the Python, Go, Java, and .NET endpoint maps still carried digest 8f9efc7be194 from 9d6b03f while TypeScript carried 0ef644d8b650 from a6648de. The service-protocol suite failed backup_controller_kill_matrix_converges_after_the_archive_effect_gap because its fixture built a bare rrflowKV store where open_existing now requires a complete engine root.
acceptance commands and exact results: recorded in the pull request after the final local run.
surfaced failures and corrections: recorded in the pull request after the final local run.
checks not run and reason: full workspace build and the SDK conformance job run in candidate CI on the pull request.
remaining known errors and owner: POAM-027 stays open until a GitHub ruleset on main requires pull requests and the stable candidate check and denies deletion and force pushes. Canonical installed backup resolution and restore remain D-11.
full-file reread and diff review: complete for every changed authored file before commit.
change checklist: complete — owner chain, adaptive-reasoning boundary, required documentation owners, link closure, generated indexes, and unchanged runtime reconciled.
roadmap/POA&M status change, or explicit none: none — POAM-027 remains Open; its observed deficiency and closure evidence now describe the pull-request rule and the missing ruleset.
pull request, CI run, and merge evidence: pending — recorded on the pull request.
```

## Links

- [POAM-027](../../../poam/rrflow-1.0-alpha/poam-027.md)
- [Change-authoring procedure](../../../roadmap/rrflow-1.0-execution/change-authoring.md)
- [CI operations](../../../operations/ci.md)
