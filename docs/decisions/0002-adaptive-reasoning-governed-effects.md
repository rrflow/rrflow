# ADR-0002: Adaptive reasoning with governed durable effects

**Status:** active accepted architecture decision
**Coordinate:** `rrflow://rrflow-instance/data/decision/0002-adaptive-reasoning-governed-effects`
**Owner:** the boundary between open-ended AI reasoning and deterministic RRFlow effects
**Decision date:** 2026-09-14

## Context

RRFlow must help an AI discover, connect, reason over, and improve a real
project without reducing that work to a brittle workflow or a schema of
thought. At the same time, durable knowledge, source mutations, external
effects, authorization decisions, replay, and claimed evidence must be
inspectable and safe.

The repository blurred those needs. Several records required a complete
committed project inventory before exploratory reading or planning, described
routines as the shape of all reasoning, and repeated executable API and SDK
facts in manually maintained documentation. That approach can make the system
deterministic at the cost of making it unable to adapt.

## Decision

RRFlow adopts **adaptive reasoning with governed durable effects**.

An AI may explore available project material, form hypotheses, choose context,
revise a plan, and use model-native reasoning without first encoding those
private or provisional steps as RRFlow records. Discovery may be incomplete,
iterative, and opportunistic. RRFlow does not define, persist, request, or
reconstruct hidden chain-of-thought.

A closed contract begins when work crosses a durable or externally observable
boundary. `RrdEngine` authenticates and authorizes the actor, binds the inputs
needed for the claimed guarantee, enforces budgets and policy, and commits or
accepts the resulting receipt. The amount of structure is proportional to the
effect and the claim, not to the complexity of the reasoning that preceded it.

| Activity | Contract posture |
|---|---|
| Read, search, inspect, compare, hypothesize, and select provisional context | Adaptive and incrementally grounded. Record uncertainty and source identity when useful; do not require a complete engram snapshot merely to think or look. |
| Produce a recommendation or draft | Flexible semantic output with provenance appropriate to its use. It remains a proposal and has no mutation authority. |
| Persist canonical knowledge, activate a projection, or mutate project state | Authenticate, authorize, bind the relevant read/source revision, validate the proposed effect, and commit through `RrdEngine`. |
| Invoke a process, provider, network service, or other external effect | Use an explicit installed capability, bounded activity plan, and accepted or uncertain-outcome receipt. |
| Resume, replay, audit, qualify, or claim evidence | Bind exact durable identities, inputs, versions, commands, limits, outcomes, and omissions needed to reproduce the claim. |

A `SourceTreeSnapshot` is therefore an authoritative input to a persisted
project model, reproducible mutation, or evidence claim. It is not permission
to read a file and not a prerequisite for exploratory analysis. When an AI
turns exploration into a proposed durable change, the system captures or
refreshes the affected source boundary and rejects stale or conflicting
effects before commit.

Routines are optional durable orchestration for repeatable, long-running, or
side-effecting work. Their graphs contain public operations, checkpoints, and
effect receipts. They do not enumerate every reasoning step, dictate a
model's internal search strategy, or make unrecorded thought invalid.

Contracts use progressive formalization:

- stable identity, authorization, resource, persistence, error, and receipt
  fields are closed and versioned at their boundary;
- semantic intent remains extensible through versioned capabilities and typed
  extensions instead of one universal project or reasoning taxonomy;
- unknown intent may be inspected or declined without being silently coerced;
- frequently repeated and proven behavior may graduate into a stronger typed
  operation; and
- a schema validates an exchanged effect or durable record, never the AI's
  private reasoning process.

## Generated relationships and product surfaces

Human and AI authors maintain the smallest owning source. The framework derives
mechanical relationships and projections from it:

- Markdown bodies remain narrative memory; their stable coordinates, owner
  links, and ordinary links form a generated navigation and retrieval graph;
- the executable operation and capability catalogues generate transport
  dispatch metadata, OpenAPI, internal client bindings, language SDK
  operations and models, reference tables, and the shared conformance matrix;
- Git plus the active change boundary generates file/digest inventory rather
  than requiring authors to synchronize another prose catalogue; and
- immutable journal and test artifacts are linked evidence nodes, not sections
  appended forever to a planning authority.

Generated output is replaceable acceleration and discovery. It cannot override
its executable or narrative owner, authorize an effect, advance a routine,
close a roadmap gate, or change POA&M status.

## Consequences

- RRFlow can exploit model-native exploration and synthesis while keeping
  persistent and external behavior auditable.
- A caller can express semantic goals without selecting physical keys,
  indexes, or providers; the intent vocabulary can grow without accepting
  arbitrary unvalidated effects.
- Reproducibility is strongest where it matters: committed state, external
  effects, replay, and evidence. Exploratory paths need not be identical.
- Documentation and SDK maintenance move toward compiler-like projection from
  owners rather than hand-copied tables.
- Incomplete discovery is represented honestly. It blocks only a guarantee
  that depends on completeness, not unrelated reasoning.

## Rejected alternatives

- A closed schema for all context selection and reasoning was rejected because
  it would encode today's assumptions as the limit of future intelligence.
- Requiring a complete committed project snapshot before any inspection was
  rejected because it confuses reproducible effects with exploratory reads.
- Allowing models or clients to commit directly was rejected because adaptive
  reasoning does not remove authorization, consistency, or audit needs.
- Hand-maintaining equivalent route, SDK, documentation, and conformance lists
  was rejected because synchronization drift is a tooling defect.
- Treating generated indexes, traces, or plans as a second authority was
  rejected because projections must always be reconstructible from owners.

## Links

- [Single-engine authority](0001-single-engine-authority.md)
- [System overview](../architecture/system-overview.md)
- [Engine data flow](../architecture/engine-data-flow.md)
- [Execution navigation](../roadmap/rrflow-1.0-execution-map.md)
- [Public contract](../reference/protocol/public-contract.md)
