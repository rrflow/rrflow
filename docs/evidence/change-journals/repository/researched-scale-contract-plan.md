# Researched scale, schema and signal planning

**Status:** active immutable planning evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/researched-scale-contract-plan`
**Owner:** OBJ-06/OBJ-10 planning prerequisites, existing G/H-05/J owners

Starting revision: `554ce4ccf1cb25aceba036dafaf4c8df79c42701`.
A separate clean worktree/branch was created from public development main.
The user's existing worktrees were preserved. A local document-only PR brief
preceded edits; GitHub requires a nonempty diff to open the PR. No retired
planning-only record or commit was created.

## Change brief and ownership

The user requested online research and a code-grounded plan for scale, growth,
optimization, signals and schemas defining structure/inputs. Existing owners
already specify the product; this package makes the missing reasoning
execution and capacity work reviewable without creating another roadmap.

The new `docs/architecture/engine-scale-and-contracts.md` records primary-source
research, code-grounded tradeoffs, a target relationship diagram and the next
existing H-05a/D-01 prerequisite. Gate G gains bounded source/oracle packages;
Gate J gains proposed workload/quality/resource thresholds, a typed manifest
field specification and qualification packages. The schema catalogue defines
type producers, input/output structure and semantic validators; engine data
flow connects signals to bounded actions and failure tests. README and the
authored architecture index link the rationale. POAM-009/-012/-026 record
specific inspected gaps; POAM-013 corrects an obsolete Windows-build claim
using merged PR 32, retaining the signed/offline release deficiency.
The journal index and file inventory are generated.

No runtime source, executable schema, API, dependency, model, format, feature
closure, product version, service account or deployed artifact changes.
Proposed schema/benchmark models are not reported as existing generated APIs.
All outcome, roadmap and parent POA&M statuses remain unchanged. Existing
H-07/H-08 DAGs are linked, not duplicated. No global implementation audit or
schema of private reasoning is introduced.

## Inspection and first oracle

Read owner records: README/AGENTS; objective outcomes; dependency spine;
change-authoring procedure; G/F/H/J gate requirements; D-01/C-07 acceptance;
schema catalogue; instance topology; PostgreSQL and DevPulse contracts;
ADR-0002; engine data-flow build/metrics/latency/current-boundary sections;
POAM-004/-009/-012/-013/-014/-026; PR 32 evidence.
Complete changed sections and new records were reviewed with their diff.
The architecture index is authored; only marked journal indexes are generated.

Exact implementation sections inspected:

- engine inference generation/descriptor/installation and the full inference
  registry `embed_batch`/`dispatch_embedding_batch`/output-validation path;
- DataFusion `RrdSnapshotTableProvider` construction and scan;
- public router descriptor/limits/`RouteStepRequest` validation, public task
  lifecycle and kernel/public tree definitions;
- CLI `VersionReport` and `version`, generated-surface source declarations,
  evaluation `Trial` and run dispatch, and Cargo.lock dependency versions.

The first oracle was source reconciliation, not a runtime benchmark.
The embedding registry lock encloses synchronous adapter work; input/output
shape checks do not prove runtime cancellation or fair admission. DataFusion
uses materialized snapshot batches. Router/tree schema and model admission
do not provide executable reasoning. The version report lacks the mandated
build identity, and the paired-provider trial is not the complete engine-load
manifest. These support explicit repair/qualification packages, not measured
throughput, deadlock or product-completion claims.

## Research and engineering decisions

The research rationale cites the primary pages actually consulted on
2026-10-20: PostgreSQL 18 message flow/isolation/replication restrictions and
conflicts; Apache DataFusion providers/pool behavior; Tokio shared state and
1.53.1 blocking work; JSON Schema 2020-12; OpenTelemetry stability and its
current GenAI repository; Prometheus instrumentation; Google SRE overload;
Qdrant multitenancy; Ink & Switch local-first research; llama.cpp grammar
guidance. Broader Temporal/Jepsen/Pinecone reference discovery did not justify
another runtime or competitor claim. No upstream source was copied.

The locked DataFusion 55.0.0 API page was unavailable through the browsing
tool. Current upstream material is used only for design behavior; the plan
requires checking locked APIs during implementation. Documentation is not
evidence of achieved performance. Desktop/service/fleet/large-data limits and
latency/quality thresholds are explicit proposals, frozen before future runs.
Existing canonical schema and signal owners must generate mechanical outputs.

No runtime trace or benchmark was needed for this documentation package.
Future packages must capture queue/execution/lock/model/commit timing,
physical memory/work, uncertainty and provenance at their actual boundaries.
Stop on runtime scope, duplicate authority, inferred gate completion, or a
superiority claim without matched evidence. Native release, reasoning,
PostgreSQL and synchronization acceptance remain open.

## Verification and surfaced errors

An initially guessed dependency-spine path was absent; repository file
discovery resolved the canonical file. A PowerShell wildcard supplied as a
literal `rg` path was rejected; no repository data changed. The tool-search
entry point was unavailable, so the native web tool performed the research.
The first documentation check rejected the new record's status because it
lacked `active`/`historical`; it was corrected to `active researched target
design` without changing product maturity.

`python scripts/knowledge/test_navigation.py` passed 6 tests and
`python scripts/knowledge/test_export.py` passed 11 tests.
`python scripts/ci/test_execution_inventory.py` passed 4 tests.
Documentation policy passed with 326 statuses/324 classified coordinates;
navigation passed with 325 nodes/1423 edges; inventory generation/check passed
with 1239 records; whitespace checks passed. Final content edits are followed
by inventory regeneration and non-mutating documentation/navigation checks.
Candidate CI and exact merge evidence are attached to this package's pull
request. Runtime, real-model, load, soak and competitor experiments were not
run locally: this change defines their acceptance work and changes no code.

This record supplies the auditable change checklist: baseline, explicit
scope/owners, actual inspection, research provenance, semantic/source oracle,
trace/resource decisions, failures, acceptance and omissions. No implementation
was moved, deleted or rewritten; runtime replacement traceability is deferred
to each named code-bearing package.
