# RRFlow 1.0 release roadmap

**Status:** active canonical release plan; 1.0.0 is frozen and the alpha baseline is not established
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0`
**Owner:** release planning; linked from the repository root `README.md`

## Product delivery path

Build one configurable engine that opens a real project, constructs and
maintains source-grounded knowledge, answers a prompt, and makes its actions
inspectable. The milestones below name engineering outcomes and evidence; they
are not a fixed number of runtime reasoning steps.

| Requested outcome | Owning work | Required demonstration |
|---|---|---|
| Select an existing project with minimal required input | D-01 through D-04, H-06 | Preview actual effects and preserve inherited project access; create/discover the project map, index permitted data in bounded background work and report exclusions, progress and restart. |
| One AI-oriented relational and document database | C, E, F, H-08, H-12 | Tables, document records identified by table/kind and ID, relationships and indexes share one transaction/read authority. rrflowQL provides relational SQL-like operations; PostgreSQL and SQLite compatibility are separately defined and tested. |
| Persistent hot state and cold analytics | C-07, F-01/F-02/F-04/F-06 | A durable native WAL and bounded memtable recover acknowledged writes; configurable background Parquet publication and hot/cold reads survive interruption without mixing snapshots or losing data. Prompt-local state has an explicit shorter lifetime. |
| Dynamic reasoning, relationships and handoff | D-02/D-05, G, H-01/H-09, I-03 | Each request selects available configured capabilities and effort; the engine exposes selected/skipped reasons, source identity and effect-bound plan/receipts. Relevant edits cause a successor plan, while unrelated work continues. Generated schemas describe mediated effects, not private reasoning. |
| Knowledge paths and AI integrations | D-06, G, H-04/H-06, I-04 | Explicitly admitted local folders and supported services provide provenance and freshness; LFG/SWAG and provider adapters are configured and authorized. UI and SDKs expose the same engine state; no integration is silently activated. |
| Tune and compete on measured results | C-07, H-05, J-04 | Retain current-revision correctness, latency, throughput, resource and failure data on named hardware. Compare like-for-like with pinned Fjall and relevant database/retrieval/agent baselines; do not treat old wins, labels or export as current proof. |

The [RRFlow kernel contract](kernel-contract.md) orders the engineering work
that delivers these outcomes, starting with a working skeleton test that every
later package keeps passing. The immediate developer-build proof remains the
joined real-project cycle in the dependency spine. Parquet, SQL compatibility, native qualification and
full comparative evidence stay open until their own named cases pass; their
research and safe implementation can progress without inventing runtime phases.
The current bundled attunement defaults still list the same eleven phases;
automatic project- or prompt-tailored selection has not been established.
Replace that fixed default and its linear assumptions through the D-02/D-05
owners while preserving existing committed effects and safe replan behavior.

### One project, then resource-managed instances

First prove one existing project end to end: open its installed daemon in
Connectome, index authorized project sources, retrieve source-grounded context,
use the configured LFG and SWAG contracts where available, and show the actual
results and stage/resource evidence. A connection or advisory result alone does
not prove the prompt cycle; the existing D/G/H gates own its acceptance.

After that usable baseline, extend H-06/H-07 to manage multiple independent
project instances under configurable host and per-instance budgets. The user
may register more projects than can run at once. Admission, waiting, idle
shutdown, restart, failures and resource pressure must be visible; capacity is
bounded by configured policy and measured resources, not a promised unlimited
number or a hardcoded product-wide count. Preserve each project's durable
engram and identity; never silently stop a process owned by another operator or
share context across projects. See the
[instance-management acceptance](rrflow-1.0/gate-h.md#resource-managed-project-instances).

An optional enterprise control plane is intended as separately maintained code
for managed cloud and self-hosted operation, consuming the same authenticated
engine contracts. It must not be required by the local project journey or
become a second engine or transaction authority. The owner requested a six-year
source-availability direction for that enterprise code. The precise licence,
change licence, distribution and self-hosting grant are open decisions; no
existing licence or service-source commitment is changed by this plan.

## Current alpha objective and remediation

[Open the current alpha objective and remediation chapter](rrflow-1.0/current-alpha-objective-and-remediation.md)

## RRFlow 1.0 execution checklist

[Open the execution-checklist chapter](rrflow-1.0/execution-checklist.md)

### Executable dependency spine

[Open the executable dependency spine](rrflow-1.0/executable-dependency-spine.md)

The active priority is the installed real-project workflow: D-02 through D-05
with their specific canonical E/F prerequisites and incremental H-06 evidence.
Accepted D-01 enables this work now. C-07 correctness failures that affect the
workflow must be repaired, but final capacity/profile/24-hour qualification
does not block implementing missing ingestion and retrieval. Full release
requirements and every unchecked gate remain open. Preserve the existing API
and converge the one pre-release implementation directly.

First deliver a usable developer build and its real-project journey. Then use
repeatable benchmarks to tune that build while the existing qualification work
continues. Boundary-derived reasoning shards and associated custom KV-cache
strategies are a later, evidence-gated direction for large projects; they do not
block the first usable build or change the alpha exit. The [scale rationale](../architecture/engine-scale-and-contracts.md#boundary-based-reasoning-after-the-developer-build)
records their intended scope and evaluation.

### Build-plan admission

The [package readiness rules](rrflow-1.0/executable-dependency-spine.md#package-readiness-and-evidence)
apply before implementation. The [D-02 through D-05 build packages](rrflow-1.0/gate-d.md#project-workflow-build-packages)
identify the next concrete boundaries. A requirement, type or planned file is
not an available capability. Known missing admission/configuration decisions
are recorded in [POAM-006](../poam/rrflow-1.0-alpha/poam-006.md#build-plan-admission-gaps).
The owner has since directed runtime implementation to resume. Resolve each
actual effect-boundary decision against the existing authorization and current
owner instructions; an unrelated open question does not block safe work.

### Gate A — freeze authority, names, and boundaries

[Open Gate A requirements](rrflow-1.0/gate-a.md)

#### A-06 knowledge-bootstrap sequence

[Open the A-06 knowledge-bootstrap sequence](rrflow-1.0/gate-a.md#a-06-knowledge-bootstrap-sequence)

#### Later knowledge-persistence sequence

[Open the later knowledge-persistence sequence](rrflow-1.0/gate-a.md#later-knowledge-persistence-sequence)

#### Gate A accepted evidence

[Open Gate A accepted evidence](rrflow-1.0/gate-a-evidence.md)

### Gate B — freeze public and model-neutral contracts

[Open Gate B requirements](rrflow-1.0/gate-b.md)

#### Gate B accepted evidence

[Open Gate B accepted evidence](rrflow-1.0/gate-b-evidence.md)

### Gate C — make rrflowKV the only local persistent substrate

[Open Gate C requirements](rrflow-1.0/gate-c.md)

#### Gate C accepted evidence

[Open Gate C accepted evidence](rrflow-1.0/gate-c-evidence.md)

### Gate D — install, configure, and attune one real engram

[Open Gate D](rrflow-1.0/gate-d.md)

#### Gate D accepted evidence

[Open Gate D accepted evidence](rrflow-1.0/gate-d-evidence.md)

### Gate E — make graph and indexes native incremental access paths

[Open Gate E](rrflow-1.0/gate-e.md)

### Gate F — connect rrflowKV to Arrow/DataFusion correctly

[Open Gate F](rrflow-1.0/gate-f.md)

### Gate G — connect LFG without creating another engine

[Open Gate G](rrflow-1.0/gate-g.md)

### Gate H — prove context flow, feedback, live delivery, and Connectome

[Open Gate H](rrflow-1.0/gate-h.md)

1. **H-05a —** [build and signal identity](rrflow-1.0/gate-h.md#h-05a--build-and-signal-identity)
2. **H-05b —** [causal context](rrflow-1.0/gate-h.md#h-05b--causal-context)
3. **H-05c —** [correlated diagnostics](rrflow-1.0/gate-h.md#h-05c--correlated-diagnostics)
4. **H-05d —** [physical path coverage](rrflow-1.0/gate-h.md#h-05d--physical-path-coverage)
5. **H-05e —** [capture and proof](rrflow-1.0/gate-h.md#h-05e--capture-and-proof)

[H-08 PostgreSQL compatibility packages](rrflow-1.0/gate-h.md#h-08-postgresql-compatibility-packages)
define the native rrflowDB SQL/client and data-portability work.

### Gate I — add explicit automation scaffolding with host hooks as alias stubs

[Open Gate I](rrflow-1.0/gate-i.md)

### Gate J — RRFlow 1.0 release proof

[Open Gate J](rrflow-1.0/gate-j.md)

## Checklist item warps

| Done | ID | Chapter |
|---|---|---|
| [x] | A-01 | [Gate A](rrflow-1.0/gate-a.md) |
| [x] | A-02 | [Gate A](rrflow-1.0/gate-a.md) |
| [x] | A-03 | [Gate A](rrflow-1.0/gate-a.md) |
| [x] | A-04 | [Gate A](rrflow-1.0/gate-a.md) |
| [x] | A-05 | [Gate A](rrflow-1.0/gate-a.md) |
| [x] | A-06 | [Gate A](rrflow-1.0/gate-a.md) |
| [x] | A-07 | [Gate A](rrflow-1.0/gate-a.md) |
| [x] | KB-01 | [Gate A knowledge sequences](rrflow-1.0/gate-a.md) |
| [x] | KB-02 | [Gate A knowledge sequences](rrflow-1.0/gate-a.md) |
| [x] | KB-03 | [Gate A knowledge sequences](rrflow-1.0/gate-a.md) |
| [x] | KB-04 | [Gate A knowledge sequences](rrflow-1.0/gate-a.md) |
| [x] | KB-05 | [Gate A knowledge sequences](rrflow-1.0/gate-a.md) |
| [ ] | KB-06 | [Gate A knowledge sequences](rrflow-1.0/gate-a.md) |
| [ ] | KB-07 | [Gate A knowledge sequences](rrflow-1.0/gate-a.md) |
| [ ] | KB-08 | [Gate A knowledge sequences](rrflow-1.0/gate-a.md) |
| [x] | B-01 | [Gate B](rrflow-1.0/gate-b.md) |
| [x] | B-02 | [Gate B](rrflow-1.0/gate-b.md) |
| [x] | B-03 | [Gate B](rrflow-1.0/gate-b.md) |
| [x] | B-04 | [Gate B](rrflow-1.0/gate-b.md) |
| [x] | B-05 | [Gate B](rrflow-1.0/gate-b.md) |
| [x] | C-01 | [Gate C](rrflow-1.0/gate-c.md) |
| [x] | C-02 | [Gate C](rrflow-1.0/gate-c.md) |
| [x] | C-03 | [Gate C](rrflow-1.0/gate-c.md) |
| [ ] | C-04 | [Gate C](rrflow-1.0/gate-c.md) |
| [x] | C-05 | [Gate C](rrflow-1.0/gate-c.md) |
| [x] | C-06 | [Gate C](rrflow-1.0/gate-c.md) |
| [ ] | C-07 | [Gate C](rrflow-1.0/gate-c.md) |
| [x] | D-01 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-02 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-03 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-04 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-05 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-06 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-07 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-08 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-09 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-10 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | D-11 | [Gate D](rrflow-1.0/gate-d.md) |
| [ ] | E-01 | [Gate E](rrflow-1.0/gate-e.md) |
| [ ] | E-02 | [Gate E](rrflow-1.0/gate-e.md) |
| [ ] | E-03 | [Gate E](rrflow-1.0/gate-e.md) |
| [ ] | E-04 | [Gate E](rrflow-1.0/gate-e.md) |
| [ ] | E-05 | [Gate E](rrflow-1.0/gate-e.md) |
| [ ] | F-01 | [Gate F](rrflow-1.0/gate-f.md) |
| [ ] | F-02 | [Gate F](rrflow-1.0/gate-f.md) |
| [ ] | F-03 | [Gate F](rrflow-1.0/gate-f.md) |
| [ ] | F-04 | [Gate F](rrflow-1.0/gate-f.md) |
| [ ] | F-05 | [Gate F](rrflow-1.0/gate-f.md) |
| [ ] | F-06 | [Gate F](rrflow-1.0/gate-f.md) |
| [ ] | G-01 | [Gate G](rrflow-1.0/gate-g.md) |
| [ ] | G-02 | [Gate G](rrflow-1.0/gate-g.md) |
| [ ] | G-03 | [Gate G](rrflow-1.0/gate-g.md) |
| [ ] | G-04 | [Gate G](rrflow-1.0/gate-g.md) |
| [ ] | G-05 | [Gate G](rrflow-1.0/gate-g.md) |
| [ ] | G-06 | [Gate G](rrflow-1.0/gate-g.md) |
| [ ] | H-01 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-02 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-03 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-04 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-05 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-06 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-07 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-08 | [Gate H](rrflow-1.0/gate-h.md#h-08-postgresql-compatibility-packages) |
| [ ] | H-09 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-10 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-11 | [Gate H](rrflow-1.0/gate-h.md) |
| [ ] | H-12 | [Gate H](rrflow-1.0/gate-h.md#h-12-sqlite-compatibility-target) |
| [ ] | I-01 | [Gate I](rrflow-1.0/gate-i.md) |
| [ ] | I-02 | [Gate I](rrflow-1.0/gate-i.md) |
| [ ] | I-03 | [Gate I](rrflow-1.0/gate-i.md) |
| [ ] | I-04 | [Gate I](rrflow-1.0/gate-i.md) |
| [ ] | I-05 | [Gate I](rrflow-1.0/gate-i.md) |
| [ ] | I-06 | [Gate I](rrflow-1.0/gate-i.md) |
| [ ] | I-07 | [Gate I](rrflow-1.0/gate-i.md) |
| [ ] | J-01 | [Gate J](rrflow-1.0/gate-j.md) |
| [ ] | J-02 | [Gate J](rrflow-1.0/gate-j.md) |
| [ ] | J-03 | [Gate J](rrflow-1.0/gate-j.md) |
| [ ] | J-04 | [Gate J](rrflow-1.0/gate-j.md) |
| [ ] | J-05 | [Gate J](rrflow-1.0/gate-j.md) |

## Supporting execution map

[Open the RRFlow 1.0 code execution map](rrflow-1.0-execution-map.md)
