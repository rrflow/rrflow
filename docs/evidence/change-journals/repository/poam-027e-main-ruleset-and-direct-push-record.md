# POAM-027e main ruleset and direct-push record

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/poam-027e-main-ruleset-and-direct-push-record`
**Owner:** evidence for the `main` ruleset and the 2026-10-19 direct push recorded under POAM-027

This record reports one external repository control and one landing-policy
breach. It does not change RRFlow code, CI workflows, or any roadmap item.

## Evidence record

```text
gate/package and alpha outcome or prerequisite advanced: POAM-027 / J-01 and J-02 repository-enforcement prerequisite — make the pull-request landing rule enforced rather than written, and record the one direct push that preceded it
starting revision/tree and pull request: development/main 9bcc6154 (Merge pull request #17) on branch evidence/main-ruleset-and-direct-push; pull request recorded below
change brief (current -> target behavior, owner, exact scope, unchanged behavior, stop conditions): no ruleset protected main, so a direct push could land without a pull request or candidate CI -> ruleset 25439032 requires a pull request and the pipeline / ci-gate check, denies force pushes and deletion, and lets administrators bypass only while merging a pull request; docs/operations/ci.md and the POAM-027 detail record the control and the breach. Owner: docs/operations/ci.md and POAM-027. Unchanged: workflows, triggers, check names, code, and the official promotion repository. Stop if the control would need a client hook, a force push, a history rewrite, or a change to the official repository.
files and symbols read in full: docs/operations/ci.md repository enforcement section; docs/poam/rrflow-1.0-alpha/poam-027.md; docs/roadmap/rrflow-1.0-execution/change-authoring.md journal template; GitHub API responses for the repository, its rulesets, the effective rules on main, Actions permissions, pull request #17 checks, and the workflow runs on main.
implementation/evidence paths created, changed, moved, or deleted: changed docs/operations/ci.md, docs/poam/rrflow-1.0-alpha/poam-027.md, and scripts/ci/check_documentation.py (the POAM-027 current-boundary phrase moves from "written policy, not enforced" to "The pull request rule is now enforced." and the POAM-027e journal becomes a required remediation link); created this journal; regenerated docs/evidence/change-journals/repository/README.md and docs/roadmap/rrflow-1.0-file-plan.jsonl. The ruleset itself is external GitHub state, not a repository path.
research decision and primary-source/adaptation record: GitHub repository rulesets REST API and rule types (pull_request, required_status_checks, non_fast_forward, deletion) with a RepositoryRole administrator bypass in pull_request mode; the required check is bound to the GitHub Actions app (integration 15368) so another status source cannot satisfy it.
trace/resource/debug decision (add/preserve/not applicable, with reason): not applicable to RRFlow runtime; the API responses and the rejected probe below are the debugging evidence.
first-failure or characterization oracle and result: before the ruleset, `git push -u development policy/pinned-git-submodules` with push.default=upstream and a branch tracking development/main fast-forwarded main 0bdc400a..b298fa96 with no pull request and no candidate CI run (ci.yml triggers only on pull_request, merge_group, and workflow_dispatch). After the ruleset, a direct push of empty probe commit ffafbddc6cf204e5d661a87be21879127a1bff92 (parent 9bcc6154, same tree) to refs/heads/main with administrator credentials was rejected — "Changes must be made through a pull request." and "Required status check \"pipeline / ci-gate\" is expected." — and main remained 9bcc6154.
acceptance commands and exact results: `gh api -X POST repos/rrflow/rrflow-development/rulesets --input main-ruleset.json` — created ruleset 25439032, enforcement active, target ~DEFAULT_BRANCH, created 2026-10-19T18:36:20-04:00; `gh api repos/rrflow/rrflow-development/rules/branches/main` — deletion, non_fast_forward, pull_request, and required_status_checks all from ruleset 25439032; `gh api repos/rrflow/rrflow-development/rulesets/25439032` — bypass RepositoryRole 5 in pull_request mode, current_user_can_bypass pull_requests_only; `gh api repos/rrflow/rrflow-development/actions/permissions` — sha_pinning_required false. Candidate CI for b298fa96: pull request #17 head 5be3705d contains b298fa96 and its pipeline / ci-gate succeeded in run 39309982937; workflow_dispatch run 39335982331 on main 9bcc6154 (which contains b298fa96) concluded success at 2026-10-19T22:47:34Z with all 17 jobs passing, including verify, all five engine suites, six optional-feature jobs, installed RRFlow lifecycle, Windows and macOS estate process, supported SDK conformance, and pipeline / ci-gate.
surfaced failures and corrections: the direct push to main is the surfaced failure; the first documentation-policy run failed with "POAM-027: missing current boundary 'written policy, not enforced'" because the checker hard-coded the superseded gap, so the checker now requires the enforced boundary; the authoring branch's upstream was removed and later pushes use an explicit `HEAD:refs/heads/<branch>` refspec. No force push or history rewrite was used; b298fa96 remains on main by the repository owner's decision, with the CI evidence above.
checks not run and reason: the hosted full-SHA Actions setting was not enabled because nested third-party actions could reference unpinned actions and break candidate CI; it needs its own verified change. No merge queue was configured.
remaining known errors and owner: POAM-027 closure is the repository owner's decision; this record supplies the closure evidence named by POAM-027 (ruleset requiring pull requests and the stable check, deletion and force-push denial, repository, ref, rule identity, API evidence, and an attempted bypass). The administrator bypass and the disabled hosted SHA-pinning setting remain recorded limits.
full-file reread and diff review: complete — both changed records and this journal were reread and the full diff reviewed.
change checklist: complete — ruleset created and verified through the API, bypass attempt rejected, breach recorded with cause and CI evidence, documentation updated, journal linked and indexed
roadmap/POA&M status change, or explicit none: none; POAM-027 status stays with the ledger owner
pull request, CI run, and merge evidence: recorded on the pull request for branch evidence/main-ruleset-and-direct-push
```

## Ruleset definition

```json
{
  "name": "main: pull requests with passing candidate CI",
  "target": "branch",
  "enforcement": "active",
  "conditions": { "ref_name": { "include": ["~DEFAULT_BRANCH"], "exclude": [] } },
  "bypass_actors": [
    { "actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "pull_request" }
  ],
  "rules": [
    { "type": "deletion" },
    { "type": "non_fast_forward" },
    {
      "type": "pull_request",
      "parameters": {
        "required_approving_review_count": 0,
        "dismiss_stale_reviews_on_push": true,
        "require_code_owner_review": false,
        "require_last_push_approval": false,
        "required_review_thread_resolution": true,
        "allowed_merge_methods": ["merge", "squash", "rebase"]
      }
    },
    {
      "type": "required_status_checks",
      "parameters": {
        "strict_required_status_checks_policy": true,
        "do_not_enforce_on_create": false,
        "required_status_checks": [
          { "context": "pipeline / ci-gate", "integration_id": 15368 }
        ]
      }
    }
  ]
}
```

## Links

- [POAM-027](../../../poam/rrflow-1.0-alpha/poam-027.md)
- [CI repository enforcement](../../../operations/ci.md#repository-enforcement)
- [Pinned Git submodule policy journal](pinned-git-submodule-source-input-policy-journal.md)
