# POAM-027 — change-package and repository enforcement

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-027`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Changes reach `main` only through pull requests whose candidate CI passes
(decision of 2026-10-16). The planning-only commit, the
`docs/roadmap/rrflow-1.0-active-change.json` record, and the
`scripts/ci/check_change_plan.py` validator were retired at the same time:
between 2026-08-20 and 2026-09-15, 442 commits reached `main` without a pull
request, and because candidate CI triggers only on `pull_request`,
`merge_group`, and `workflow_dispatch`, none of them ran it automatically. At
`cfa0e80` the verify job's generated-surface check and edge binary ceiling and
the service-protocol suite's backup kill-matrix test failed while the
per-change validator passed.

One gap remains: the pull request rule is written policy, not enforced. No
recorded repository ruleset yet requires pull requests and the stable
candidate check, or denies branch deletion and force pushes, on `main`. On
2026-09-11 GitHub returned HTTP 403 for both rulesets and `main`
branch-protection APIs while the repository was private; the repository owner
confirmed on 2026-10-16 that `rrflow/rrflow-development` is public, where those
controls are available.

## Impact

A direct push can still bypass candidate CI and land a failing revision on
`main`. Installing a client Git, editor, or provider hook would remain
unportable and would create a forbidden parallel lifecycle mechanism.

## Owning gates

J-01, J-02

Roadmap owner: [Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Enable a ruleset on `main` that requires pull requests and the exact stable
candidate check and denies branch deletion and force pushes; record the
repository, ref, rule identity, API evidence, and an attempted bypass.
Continue to require one linked journal per bounded package and regenerate its
nearest index. This is a repository effect and evidence boundary, not a schema
for exploratory or private AI reasoning. No client hook or workflow event
becomes RRFlow runtime authority.

## Remediation evidence

- [POAM-027a change-plan enforcement implementation](../../evidence/change-journals/repository/poam-027a-change-plan-enforcement-implementation-journal.md)
- [POAM-027b canonical package-identity correction](../../evidence/change-journals/repository/poam-027b-canonical-package-identity-correction-journal.md)
- [POAM-027c linked execution records and generated navigation](../../evidence/change-journals/repository/poam-027c-linked-execution-records-and-generated-navigation.md)
- [POAM-027d pull-request landing and CI gate](../../evidence/change-journals/repository/poam-027d-pull-request-landing-and-ci-gate.md)
- [Current CI and repository-enforcement boundary](../../operations/ci.md)
