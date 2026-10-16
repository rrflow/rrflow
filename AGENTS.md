# RRFlow engineering instructions

`README.md` is the bootstrap product entry point and knowledge map. It owns
product identity and current status, and it links to the single owning memory
record for each detailed subject. The RRFlow 1.0 release checklist is owned by
`docs/roadmap/rrflow-1.0.md`. Other Markdown files are supporting design notes,
contracts, evidence, or history and cannot override either owning record.

`AGENTS.md` is the provider-neutral instruction entry point for repository
work. Provider-specific instruction files may import or point to this file, but
must not copy its rules or create another product, planning, recall, or
lifecycle authority. The planning chain is:

1. `README.md` owns identity, invariants, current maturity, and warp points.
2. `docs/objectives/rrflow-1.0-alpha.md` owns the measurable alpha outcomes.
3. `docs/roadmap/rrflow-1.0.md` owns dependency order and completion evidence.
4. `docs/poam/rrflow-1.0-alpha.md` owns the open-gap and remediation ledger.
5. Supporting records explain or prove one of those owners and cannot change
   their status independently.

The product version is frozen at `1.0.0` during pre-release convergence. Do
not use version changes as progress markers. A later version change requires
the repository owner's explicit instruction after the alpha objective, release
gates, optimization evidence, and release decision are complete.

The private `rrflow/rrflow-development` repository is the only push target for
incremental pre-release source, evidence, and candidate history. The private
`rrflow/rrflow` repository is the official promotion target, not a development
remote. Changes reach `main` only through pull requests whose candidate CI
checks pass; a direct push to `main` is not a landing path. Before every push,
resolve the destination URL and record its remote, ref, and exact revision. Do
not push a source ref, tag, binary, artifact, or release to the official
repository until every Gate J prerequisite passes and
the repository owner explicitly authorizes that exact promotion. Never use a
force push or history rewrite as the promotion mechanism.

All first-party code that implements the RRFlow engine or a bundled RRFlow
adapter, plus its schemas, bootstrap templates, default configuration,
installation logic, release assembly, verification, and recovery tooling, must
live in this repository.
Do not add a sibling-checkout dependency, escaping local path, Git submodule,
undeclared generator, or install-time/runtime fetch. Locked third-party source
dependencies are permitted build inputs, but the signed default distribution
must contain every linked binary and runtime asset needed to install, start,
persist, recover, and verify RRFlow after the bundle has been acquired.
Connectome, project databases, mesh services, providers, and other external
systems remain optional public-contract integrations and cannot be required
for default RRFlow readiness.

RRFlow is an independently installable and operable reasoning and recall
engine. A project's code or schema generator, build/test/evaluation harness,
CI system, database, mesh, model provider, or development tool may be
discovered during attunement and integrated through a typed, configurable
adapter, but it is never an undeclared prerequisite or a second RRFlow
authority. Discovery does not authorize activation. Installation must expose
fresh-project and existing-project modes; preview the exact versioned
scaffolding, records, adapter bindings, commands, permissions, budgets, and
digests; require explicit configuration and authorization before invoking an
external capability; and make RRFlow-owned integration removable without
damaging project-owned state. Generator output must re-enter the authorized
project-inventory and mutation flow, and harness results are evidence rather
than lifecycle state.

RRFlow uses adaptive reasoning with governed durable effects. Exploratory
reading, search, context selection, hypothesis formation, and private model
reasoning do not require a closed workflow or a complete committed project
snapshot. A closed contract begins when work persists canonical state, mutates
project-owned state, invokes an external effect, resumes or replays durable
work, or claims reproducible evidence. At that boundary, `RrdEngine` binds the
relevant source/read identity, authorization, budgets, validation, and receipt.
Do not build a schema of thought. Use progressively typed, versioned semantic
capabilities and keep unfamiliar ideas representable without allowing unknown
effects. Follow
`docs/decisions/0002-adaptive-reasoning-governed-effects.md`.

The framework must generate mechanical projections from one owner. Executable
operation and capability catalogues drive dispatch, OpenAPI, internal client
bindings, language SDK operations/models, reference projections, and shared
conformance. Coordinated documentation and its ordinary links drive indexes,
warp discovery, and relationship projections. Git drives file/digest
inventory. Generated output is replaceable discovery or acceleration; it cannot become another semantic, planning, lifecycle, or status
authority.

Upstream and experimental Rust implementations are reference inputs, not
architecture. Before adapting code from SurrealDB, Qdrant, Lance, Fjall, or
another implementation, map the useful behavior, algorithm, failure
semantics, and provenance to one canonical RRFlow boundary and its owning
roadmap gate. Adapt it to RRFlow naming, types, transaction authority,
physical model, budgets, and tests. Do not copy an upstream crate tree,
public model, compatibility surface, or hidden runtime and rename it RRFlow.

Every code-bearing or structural package must execute the
[codebase-grounded change-authoring routine](docs/roadmap/rrflow-1.0-execution-map.md#codebase-grounded-change-authoring-routine).
Complete its auditable change checklist in the package journal; an unrecorded
private or chat checklist is not evidence. This is a repository engineering
procedure, not an RRFlow runtime routine, hook, trigger, skill, or lifecycle.
It governs repository effects and claimed evidence, not exploratory reasoning
or private analysis.
Each bounded package lands as one pull request against `main`. Its description
carries the change brief: owning outcome and roadmap item, current and target
behavior, exact scope, failure oracle, acceptance commands, research and
trace/resource/debug decisions, and stop conditions. The pull request merges
only after the candidate CI pipeline in `docs/operations/ci.md` passes.
Derive mechanical baseline and navigation facts with repository tooling; authors
own intent, risk, evidence, and stop decisions rather than a duplicate prose
inventory. This repository-owned pull-request and candidate-CI gate cannot be
replaced by a Git, editor, provider, prompt, or session hook. The planning-only
commit and `docs/roadmap/rrflow-1.0-active-change.json` record were retired on
2026-10-16; that file stays unchanged as historical evidence linked from
earlier journals.

For project work:

1. Read `README.md` and follow its relevant owner warp point, then inspect the
   current worktree, implementation, tests, and existing diff before changing
   files.
2. Preserve unrelated user changes. Keep each change coherent and reviewable.
3. Use the existing `rrd-engine` composition boundary and provider-neutral
   contracts; do not add provider-specific state or a parallel source of truth.
4. Keep every first-party build, install, and runtime dependency within the
   repository and make optional external integration explicit at the contract
   edge.
5. Apply the independent-engine, project-integration, and source-adaptation
   rules in
   `docs/reference/agent-bootstrap.md#independent-engine-and-project-integration`
   before changing installation, attunement, scaffolding, or an adapter.
6. Write the routine's change brief in the pull request description before
   implementation: bind one alpha outcome, roadmap package, current behavior,
   exact files and symbols,
   smallest test oracle, trace/debugging decision, research decision, and stop
   conditions. Let intermediate errors expose boundary mismatches, but never
   hide them or treat compilation as completion.
7. Create one linked journal record for every bounded package under
   `docs/evidence/change-journals/` using the
   [evidence-record template](docs/roadmap/rrflow-1.0-execution-map.md#evidence-record-template),
   then regenerate its nearest index with
   `python3 scripts/knowledge/render_navigation.py` before commit. The linked
   record—not another journal body appended to the execution map—satisfies this
   rule. Record the starting revision, complete files read, changed paths and
   behavior, exact commands and results, surfaced failures, checks not run,
   remaining errors, and any roadmap-status change. Add a verified new
   deficiency to the POA&M; do not use chat or a Git commit alone as the work
   record.
8. Verify with the smallest relevant test first, then the owning package suite.
9. Report what actually passed, what failed, and what was not run.
10. Treat existing types, files, compilation, and mocks as implementation
   inventory, not proof. Update objective, roadmap, or POA&M status only with
   the acceptance evidence named by the owning record.
11. Before deleting, moving, merging, or rewriting implementation, follow the
   [implementation traceability procedure](docs/roadmap/rrflow-1.0-execution-map.md#implementation-requirements-traceability).
   Put the package-specific map in the pull request description and its result
   in the linked journal: current behavior, source modules, characterization
   tests, canonical destination, owning gate, and replacement
   evidence. Do not grow a global prose implementation audit. Git ancestry,
   merge status, a path rename, and a successful compile are not consolidation
   proof.
12. RRFlow 1.0 has one current pre-release implementation. Carry reusable
   behavior and tests into their canonical boundary, then remove conflicting
   paths directly. Do not create a parallel fallback, migration, or
   historical-code lane to avoid completing that convergence.

RRFlow has no editor- or provider-owned automatic hooks. Durable recall,
routine state, external effects, and mutation authorization are explicit
capabilities composed through `rrd-engine`; clients must not create a parallel
lifecycle authority. Exploratory reasoning remains adaptive under ADR-0002.
