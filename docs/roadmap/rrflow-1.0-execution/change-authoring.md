# Codebase-grounded change authoring

**Status:** active repository engineering procedure
**Coordinate:** `rrflow://rrflow-instance/data/execution-procedure/change-authoring`
**Owner:** the effect-boundary procedure for one reviewable repository change

This procedure governs repository mutations and the evidence claimed for them.
It does not prescribe an AI's private reasoning, force exploratory reads into a
workflow, or become an RRFlow runtime routine. The
[adaptive-reasoning decision](../../decisions/0002-adaptive-reasoning-governed-effects.md)
owns that separation.

## Operating principle

Explore as widely and iteratively as the problem requires. Before changing the
repository, convert the best current understanding into one bounded change
brief. Deterministic records begin at that effect boundary so reviewers can
answer what was allowed to change, why, from which baseline, and what actually
proved the result.

Mechanical facts—revision, tree, file digests, line counts, generated links,
and changed-path reconciliation—belong to repository tooling. Human or AI
judgment supplies intent, owners, risks, failure oracles, acceptance evidence,
and stop conditions. Do not turn the mechanical envelope into a schema of
thought.

## Change sequence

1. Follow `README.md` to the owning objective, roadmap item, POA&M item,
   architecture or reference record, implementation, tests, and current diff.
2. Explore the relevant code and evidence completely enough to explain current
   behavior and uncertainty. An exploratory file read does not require a
   committed project inventory.
3. Select one coherent outcome or prerequisite. Identify the smallest source
   and test boundary that can expose whether the design is wrong.
4. Decide whether external research, a runtime trace, resource measurement, or
   diagnostic capture is necessary. Record why each is used or not applicable.
5. Before the first repository edit, branch from current `main` and write the
   change brief (checklist below) as the pull request description. That brief
   binds the baseline and intended scope. It is not a transcript of reasoning
   and may not contain hidden chain-of-thought.
6. Author at the one canonical boundary. Allow intermediate compiler or test
   failures to expose wrong assumptions; do not hide them with compatibility
   layers, duplicated authorities, or false completion claims.
7. Run the smallest meaningful oracle first, then the owning package and only
   the cross-boundary checks justified by risk. Record failures as well as
   passes and identify checks not run.
8. Reread every changed file and the complete diff. Create one linked journal
   record under [`docs/evidence/change-journals/`](../../evidence/change-journals/),
   regenerate its index, and push the branch. The pull request merges into
   `main` only after the candidate CI pipeline passes.

## Required change checklist

Before implementation, the pull request description must make these decisions
reviewable without attempting to encode the reasoning process:

- the objective outcome or prerequisite and owning roadmap/POA&M records;
- current behavior, target behavior, exact effect scope, unchanged behavior,
  and stop conditions;
- complete owner and implementation files actually reviewed for the proposed
  change;
- the first failure or characterization oracle and acceptance commands;
- research, source-adaptation, trace, resource, and debugging decisions; and
- created, changed, moved, deleted, generated, and evidence paths.

A missing fact blocks only the mutation or claim that depends on it. It does
not make prior exploration invalid. If implementation discovers a materially
different scope, stop and open a new pull request from current `main`.

## Pull request and CI binding

Every change reaches `main` through a pull request. The
[candidate CI pipeline](../../operations/ci.md) runs on the pull request and
must pass before merge; a direct push to `main` is not a landing path. The pull
request, its CI run, and its merge commit are the effect record; the linked
journal is the evidence record. Neither is a product API, a runtime lifecycle,
or proof that an implementation is correct.

The planning-only commit and the machine-bound
`rrflow-1.0-active-change.json` record were retired on 2026-10-16. That file
stays unchanged as historical evidence linked from earlier journals; it is not
read by CI and is not updated.

## Linked journal record

One journal record is one immutable evidence node. It lives outside the roadmap
and is linked from the generated journal indexes. A later correction creates a
new record and link; it does not silently rewrite the original receipt.

The record must state, in clear prose or a compact table:

```text
gate/package and alpha outcome or prerequisite advanced:
starting revision/tree and pull request:
change brief (current -> target behavior, owner, exact scope, unchanged behavior, stop conditions):
files and symbols read in full:
implementation/evidence paths created, changed, moved, or deleted:
research decision and primary-source/adaptation record:
trace/resource/debug decision (add/preserve/not applicable, with reason):
first-failure or characterization oracle and result:
acceptance commands and exact results:
surfaced failures and corrections:
checks not run and reason:
remaining known errors and owner:
full-file reread and diff review:
change checklist:
roadmap/POA&M status change, or explicit none:
pull request, CI run, and merge evidence:
```

A journal records what happened. It cannot mark a release gate complete; only
the owning roadmap requirement and its named acceptance evidence can do that.

## Repository-wide run checklist

Start narrow and widen according to the changed boundary:

1. first failure or characterization oracle;
2. affected unit or focused integration target;
3. owning package tests and strict lint/format checks;
4. generated-surface, documentation, knowledge, and inventory checks when
   their sources or projections changed;
5. cross-package/workspace, fault, performance, SDK, deployment, or release
   suites only when the owning risk requires them.

The [CI operations record](../../operations/ci.md) owns exact candidate lanes.
A command that was not run is reported, never implied by a broader green label.

## Global stop conditions

Stop the package when the proposed edit would:

- cross an undeclared owner or materially expand the intended outcome;
- create a second engine, status ledger, lifecycle, catalogue, or persistence
  authority;
- require an undeclared external dependency, fetch, credential, or effect;
- delete or rewrite behavior without mapped replacement evidence;
- make a generated projection authoritative or hand-maintained; or
- claim completion without the acceptance evidence named by the owner.

When a stop condition is reached, preserve the evidence and return to the
owning record. Do not stretch the contract to make the current attempt pass.
