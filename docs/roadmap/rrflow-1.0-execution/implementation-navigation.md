# RRFlow 1.0 implementation navigation

**Status:** active implementation-navigation record
**Coordinate:** `rrflow://rrflow-instance/data/execution-navigation/rrflow-1.0-implementation`
**Owner:** assembly of canonical planning, implementation, and evidence links for one package

This record explains how to assemble an implementation view without creating a
second roadmap, architecture, POA&M, or status authority.

## Resolve one package

| Question | Owning source |
|---|---|
| What product is this and what is true now? | [`README.md`](../../../README.md) |
| What measurable alpha result is required? | [Alpha objective](../../objectives/rrflow-1.0-alpha.md) |
| What executes next and what evidence can complete it? | [Roadmap dependency spine](../rrflow-1.0/executable-dependency-spine.md) and owning [gate chapter](../rrflow-1.0.md) |
| What verified gap is being remediated? | [Canonical POA&M](../../poam/rrflow-1.0-alpha.md) and its linked item record |
| What boundary and semantics constrain the design? | [Architecture](../../architecture/) and [reference](../../reference/) owners |
| Which exact final paths may change now? | The open pull request's description and diff ([pull request and CI binding](change-authoring.md#pull-request-and-ci-binding)) |
| Which current and planned files map to gates? | [Generated file plan](../rrflow-1.0-file-plan.jsonl) |
| What happened in completed packages? | [Linked change journals](../../evidence/change-journals/) and gate evidence records |

The roadmap dependency spine is the only current-package pointer. This record
does not repeat it. The open pull request is the only exact path envelope for
the in-progress repository mutation. The generated file plan is discovery
and impact inventory; it is not permission to edit or evidence of completion.

## Former monolith disposition

The former execution map mixed several independently owned subjects. Its
refraction is explicit so removal from that file cannot make a requirement or
receipt disappear:

| Former map subject | Current destination |
|---|---|
| Change-authoring routine, checklist, evidence template, run widening, and stop conditions | [Change-authoring procedure](change-authoring.md) and [CI operations](../../operations/ci.md) |
| Product vocabulary, dependency direction, target source placement, and runtime flows | [System overview](../../architecture/system-overview.md), [engine data flow](../../architecture/engine-data-flow.md), [agent bootstrap](../../reference/agent-bootstrap.md), and [installed lifecycle](../../reference/deployment/installed-lifecycle.md) |
| Current implementation inventory and structural traceability | Generated [file plan](../rrflow-1.0-file-plan.jsonl), the package's pull request, and its final linked journal |
| Gate work packages, dependency order, and acceptance | Canonical [roadmap](../rrflow-1.0.md), [gate chapters](../rrflow-1.0/), and [dependency spine](../rrflow-1.0/executable-dependency-spine.md) |
| Verified gaps and remediation lifecycle | Canonical [POA&M](../../poam/rrflow-1.0-alpha.md) and its child records |
| Ninety-six completed package receipts | Generated [change-journal index](../../evidence/change-journals/); each payload retains its baseline locator and digest |
| Knowledge, API, SDK, and conformance projection rules | [Generated-surface design](generated-surfaces.md) and [public contract](../../reference/protocol/public-contract.md) |

The link-only [execution portal](../rrflow-1.0-execution-map.md) retains the
former top-level anchors as warp-compatible routes to these destinations.

## Architecture and source placement

The [system overview](../../architecture/system-overview.md) owns component
placement and dependency direction. The
[engine data flow](../../architecture/engine-data-flow.md) owns semantic write,
read, context, reasoning, automation, and evidence flows. This execution layer
links those owners and records only package-specific implementation choices.
It does not copy a target source tree or runtime flow into another authority.

A package should name exact files and symbols only in its active change record
and final journal. Long-lived references name stable public boundaries and
tests; the generated inventory supplies repository-wide path discovery. This
keeps navigation current as code moves without turning prose into a manually
maintained filesystem mirror.

## Traceability before structural change

Before deleting, moving, merging, or replacing implementation, map:

- current behavior and public symbols;
- source modules, callers, persistence formats, fixtures, and tests;
- the accepted canonical destination and owning roadmap item;
- behavior to preserve, generalize, replace, or reject; and
- equal-or-stronger replacement and removal evidence.

Put the package-specific map in the pull request description and its outcome in
the linked journal. Put a newly verified open deficiency in the
POA&M. Do not grow this navigation record into a global implementation audit.

## Current implementation discovery

Use these generated and executable views together:

1. Git supplies the exact current tree and changed-path history.
2. `docs/roadmap/rrflow-1.0-file-plan.jsonl` projects tracked and declared
   future paths to owning gates.
3. Cargo metadata, contract catalogues, schemas, and focused tests expose the
   implemented dependency and behavior surface.
4. The active change package narrows the permitted edit set.
5. Linked journals retain completed evidence without changing live status.

If those sources disagree, stop and correct the owner or generator. Never
resolve drift by copying the same state into another prose table.
