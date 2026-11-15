# POAM-039 — no versioned handoff or open-question record

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-039`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

The parent ledger owns priority and lifecycle status. This detail records a
verified gap against [OBJ-14](../../objectives/rrflow-1.0-alpha.md#required-outcomes);
it cannot close a roadmap gate independently.

## Observed deficiency

Verified at development `main` `04fae276`:

- Plans are versioned and digest-bound to the project root and inventory, but
  none records a source revision such as a Git commit.
- The only plan handoffs are the CLI's `install plan` output consumed by
  `install apply --plan --expect` and the Kubernetes renderer's sealed plan.
  No record carries a plan from one agent, session or step to another with its
  digest, bound sources, sender, receiver and approval.
- Contracts that would carry such work exist without engine behaviour:
  [`RouteStepRequest`](../../../crates/transport/rrd-contract/src/router.rs) has
  no engine caller, [`KnowledgePackageV1`](../../../crates/transport/rrd-contract/src/knowledge.rs)
  is used only by the contract crate and its tests, and `ActivityPlan` exists
  only in the [project command capabilities](../../reference/automation/project-command-capabilities.md)
  document.
- No type, record or rule represents an open question. The rule that a missing
  fact blocks only the mutation or claim that depends on it is prose in
  [ADR-0002](../../decisions/0002-adaptive-reasoning-governed-effects.md) and the
  [change-authoring routine](../../roadmap/rrflow-1.0-execution/change-authoring.md).
  The D-02a job-control hold in [POAM-006](poam-006.md) was itself such a
  question, tracked only in documents.


A follow-up inspection of development main `6fe8b759` found an additional
public-path restriction: `DefaultInstallProfile::load` accepts only the bundled
`default` and `clyffy` profiles, `validate` requires each profile's exact
11-entry `ATTUNEMENT_PHASES` order, and `validate_sealed_preview` requires the
plan to equal that profile. The [plan-owned progression candidate](../../evidence/change-journals/gate-d/d-02-plan-owned-attunement.md)
changes internal sequencing but does not remove these installation restrictions
or provide a public project-tailored plan. The [correction record](../../evidence/change-journals/gate-d/d-02-installation-profile-plan-gap.md)
retains the source and candidate distinction. Per-prompt reasoning handoff is a
separate missing capability, not evidence supplied by an installation template.

The [dependent public-proposal candidate](../../evidence/change-journals/gate-d/d-02-authorized-attunement-proposal.md) permits an explicitly supplied,
validated attunement sequence and seals it into an installation preview. Its
schema is a mechanical proposal-shape projection; it does not select or generate
per-prompt schemas. The installed plan retains selected kinds, positions,
estimates and descriptor digests, while proposal descriptions remain in the
preview and are not yet available through engine readback. Only Connect executes.
Since 2026-11-14 the stored plan also keeps each step's description, bound by
the plan digest, and an authorized plan read returns it
([journal](../../evidence/change-journals/gate-d/d-02-readable-plan-steps-20261114.md));
that needed attunement contract version 3.
This candidate does not close the handoff, open-question, automatic planning or
source-to-knowledge gaps, and requires candidate and merge-group evidence.

## Impact

An agent cannot prove which plan it executed, who approved it, or which
unanswered question its effect depended on. Blocking on an open question is a
convention, not something the engine enforces, so an effect can proceed while
the fact it needs is still undecided.

## Owner decision

On 2026-10-26 the repository owner decided that an open question is a typed,
versioned engine record. Each planned effect names the questions it depends on;
the engine refuses that effect while one is open and never blocks exploration or
unrelated effects; answering is an authorized, audited operation.

## Owning gates

D-02a/D-02b for attunement plans and their questions, G-02 route handoff and
I-03 generic routines.

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md#project-workflow-build-packages),
[Gate G](../../roadmap/rrflow-1.0/gate-g.md) and
[Gate I](../../roadmap/rrflow-1.0/gate-i.md).

## Closure evidence

`RrdEngine` alone accepts a versioned handoff record that binds the plan digest,
the source revision where one exists, the inventory digest, configuration,
budgets, the sending and receiving principals and the approval; a receiver that
presents a different digest is refused. Open-question records are typed and
versioned, each planned effect references the questions it depends on, and the
engine refuses that effect while one is open while unrelated reads and effects
succeed. An answer is an authorized, audited operation; an answer that changes
the plan requires an explicit re-plan. Tests cover each refusal and success
through public operations, after restart, at an exact revision.
