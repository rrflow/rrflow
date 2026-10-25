# RRFlow

RRFlow is one reasoning-data engine for AI governance, reasoning, recall, and
bounded context assembly. RRD is its embedded and daemon runtime; rrflowDB,
rrflowKV, rrflowMX, rrflowQL, Arrow/DataFusion, graph, lexical, vector,
inference, and client capabilities remain parts of that one system.

RRFlow is being built as an open-source reasoning engine for people building
with AI, including vibe coders learning professional development practices.
Connectome should make project context, evidence, change previews, verification
and review understandable without hiding their limits. The existing roadmap's
[project-to-learning journey](docs/roadmap/rrflow-1.0/executable-dependency-spine.md#project-to-learning-acceptance-journey)
maps that experience to its implementation dependencies and real-world tests.

**rrflowDB is the database within RRFlow: a pure Rust, PostgreSQL-inspired
database dedicated to AI and designed for AI use, with PostgreSQL compatibility
as a product requirement.** It stores the knowledge, memory, reasoning state,
and evidence of each project estate. The [database identity and compatibility
boundary](docs/architecture/system-overview.md#persistence-and-memory-boundary)
separates that target from current implementation and compatibility evidence.
The [PostgreSQL compatibility and independent database contract](docs/reference/protocol/postgresql.md)
defines the first SQL/client profile, native-engine integration, Turso exit,
data portability and qualification. [H-08](docs/roadmap/rrflow-1.0/gate-h.md#h-08-postgresql-compatibility-packages)
owns its implementation packages; compatibility is not yet implemented.

This README is the bootstrap product entry point and knowledge map. It owns
RRFlow's identity, non-negotiable architecture invariants, and current status;
it delegates each detailed subject to exactly one linked memory record. The
RRFlow 1.0 release checklist is owned by
[`docs/roadmap/rrflow-1.0.md`](docs/roadmap/rrflow-1.0.md). Supporting records
may explain or prove an owner, but cannot override it.

The target release-train version is `1.0.0`. It is frozen while the alpha
baseline is established. Current maturity is **pre-alpha**. The version is a
contract line, not a claim that the complete system is stable, optimized, or
release-ready.

## Engine, UI, cloud, and desktop

RRFlow is the reasoning engine. **Connectome** is its operator and developer
UI, maintained in its own repository and pinned here as the optional
`connectome/` Git submodule. The [Connectome client contract](docs/reference/client/connectome.md#repository-checkout)
owns checkout instructions and the UI's public-engine boundary.

Planned **rrflow.ai** and **rroflow.ai** services offer optional remote engine
access/synchronization and governed learning respectively; their
[service roles and opt-in boundaries](docs/reference/client/devpulse-cloud.md#optional-managed-service-roles)
are targets, not deployed-service claims. **shippin.cloud** extends this
experience with a private devspace. The initial friends-and-family experience will
demonstrate current AI-assisted development: create projects, run Rust,
Python, and Go code, load the running project's UI inside the private cloud
devspace, and launch projects on the web.

**DevPulse** is the project development application, shipping first on Windows.
Standalone RRFlow with Connectome targets guided installation in a few steps
on Windows, macOS and Linux. DevPulse bundles RRFlow and guides optional
Docker setup when a project needs containers. Its RRFlow integration
must provision local project VMs/containers, run their toolchains and UIs, and
support explicitly selected **bidirectional synchronization** with either
**shippin.cloud or an operator-controlled homelab**. Local and homelab operation
must remain available without a hosted account. Database synchronization,
project-file transfer and environment provisioning have distinct receipts and
failure states; the [integration contract](docs/reference/client/devpulse-cloud.md)
owns those requirements.

These are delivery targets, not claims of available hosted execution,
deployment, or synchronization. The [delivery boundaries](docs/architecture/system-overview.md#delivery-experiences)
define how these experiences share one engine; the existing roadmap and
pre-alpha status remain authoritative.

## Enter RRFlow

For a local desktop workflow, use the [native Connectome workbench](docs/reference/client/connectome.md#native-local-workbench):
start the bundled RRFlow engine after an explicit installation preview, explore
the authenticated schema and data models, query records, and navigate their
relationships. This is a development build with real-engine evidence; full
client qualification remains open.

Start with the [runnable local rrflowDB integration](docs/reference/sdk/rrflowdb-local.md):
build the native engine, create related records, query them with rrflowQL and
reopen persistent rrflowKV state. [DevPulse and optional shippin.cloud sync](docs/reference/client/devpulse-cloud.md)
defines local operation without an account and the explicit cloud opt-in boundary.
Cloud account synchronization and PostgreSQL compatibility remain unimplemented.

The installed engine also exposes an authenticated, bounded
[Parquet query-result export](docs/reference/data/knowledge-connectors.md#bundled-query-export).
Apache's Rust writer is linked into the binary. This capability is available
through the public operation catalogue and clients; a Connectome export control,
bulk import and streaming exchange remain unqualified.

| Need | Single owning record |
|---|---|
| Understand the product, components, and boundaries | [RRFlow system overview](docs/architecture/system-overview.md) |
| Follow researched scale, bounded execution, input/schema and signal decisions into their owning implementation packages | [RRFlow scale and contract rationale](docs/architecture/engine-scale-and-contracts.md) |
| Configure CPU analysis, System One model selection and scoped inference caches | [LFG and SWAG target contract](docs/reference/inference/lfg-swag.md) |
| Prepare Spark experiments, qualify SWAG candidates and govern RROFLOW improvement | [Learning reference](docs/reference/inference/learning/README.md) |
| Discover connector requirements and build source-grounded knowledge trees | [Knowledge connectors and Parquet target contract](docs/reference/data/knowledge-connectors.md) |
| Follow planned DuckDB analytical exchange and optional execution | [DuckDB interoperability contract](docs/reference/data/duckdb.md) — unimplemented; native rrflowDB remains authoritative |
| Understand project, estate, RRD instance, environment, deployment, and physical placement | [RRFlow instance topology](docs/architecture/instance-topology.md) |
| Select and prove deployment form, storage profile, and endpoint presentation independently | [RRFlow deployment profiles](docs/reference/deployment/modes.md) |
| Install, serve, verify, repair, restore, and remove one project estate through the shipped binary | [RRFlow installed lifecycle](docs/reference/deployment/installed-lifecycle.md) |
| Deploy one installed RRD instance through Kubernetes without creating another authority | [RRFlow Kubernetes deployment adapter](docs/reference/deployment/kubernetes-operator.md) |
| Understand the unavailable distributed target and disposition of current cluster code | [RRFlow distributed cluster contract](docs/reference/distributed/cluster-contract.md) |
| Follow writes, persistence, reads, Arrow/DataFusion, and context end to end | [RRFlow engine data flow](docs/architecture/engine-data-flow.md) |
| Understand why all capabilities remain under one authority | [ADR-0001: single-engine authority](docs/decisions/0001-single-engine-authority.md) |
| Understand where adaptive AI reasoning ends and governed durable effects begin | [ADR-0002: adaptive reasoning with governed effects](docs/decisions/0002-adaptive-reasoning-governed-effects.md) |
| See the measurable alpha result | [RRFlow 1.0 alpha objective](docs/objectives/rrflow-1.0-alpha.md) |
| Execute work in dependency order and inspect accepted evidence | [RRFlow 1.0 release roadmap](docs/roadmap/rrflow-1.0.md) |
| Inspect verified gaps and remediation ownership | [RRFlow 1.0 alpha POA&M](docs/poam/rrflow-1.0-alpha.md) |
| Assemble a gate's canonical requirements, exact active paths, generated inventory, and linked evidence | [RRFlow 1.0 execution map](docs/roadmap/rrflow-1.0-execution-map.md) |
| Inspect frozen product-version and change-control rules | [RRFlow version policy](docs/reference/release/version-policy.md) |
| Understand documentation ownership and future rrflowDB migration | [RRFlow knowledge map](docs/README.md) |

## Non-negotiable architecture

This section is the invariant index. The linked owner holds each detailed
definition; this portal does not reproduce those bodies.

| Invariant | Owning section |
|---|---|
| Canonical names for RRFlow, RRD, `RrdEngine`, rrflowDB, rrflowKV, rrflowMX, rrflowQL, Arrow/DataFusion, vectors, inference, RRO, RROFLOW, LFG, SWAG, Governance, workspace, and Connectome | [Canonical component terminology](docs/architecture/system-overview.md#canonical-component-terminology) |
| One project ↔ one estate/rrflowDB ↔ one RRD instance; an account may own many estates and a workspace groups estates above the engine; deployment form, storage profile, endpoint presentation, security, and physical placement remain distinct | [Locked alpha topology](docs/architecture/instance-topology.md#locked-alpha-topology) and [deployment profiles](docs/reference/deployment/modes.md#independent-profile-coordinates) |
| One semantic, security, transaction, mutation, and context authority | [ADR-0001 decision](docs/decisions/0001-single-engine-authority.md#decision) |
| Adaptive exploration and model reasoning; closed contracts at durable state, external effect, replay, and evidence boundaries | [ADR-0002 decision](docs/decisions/0002-adaptive-reasoning-governed-effects.md#decision) |
| No parallel product line: one current pre-release implementation with requirement-to-code/test traceability before direct convergence | [Pre-release convergence boundary](docs/architecture/system-overview.md#pre-release-convergence-boundary) and [implementation-requirements traceability](docs/roadmap/rrflow-1.0-execution-map.md#implementation-requirements-traceability) |
| Persistent rrflowDB versus volatile rrflowMX | [Persistence and memory boundary](docs/architecture/system-overview.md#persistence-and-memory-boundary) |
| Temporal graph, scalar, BM25, and vector data under one transaction model | [Native multi-model boundary](docs/architecture/system-overview.md#native-multi-model-boundary) |
| rrflowQL fast and analytical paths over stamped Arrow/DataFusion work | [Query and analytical boundary](docs/architecture/system-overview.md#query-and-analytical-boundary) |
| Authentication, authorization, read stamps, compute, mutation, and evidence | [Security boundary](docs/architecture/system-overview.md#security-boundary) |
| Repository-contained source and an offline-verifiable default distribution | [Source and distribution boundary](docs/architecture/system-overview.md#source-and-distribution-boundary) |
| One primary `rrflow`/`rrflow.exe` lifecycle with no checkout-only readiness path | [RRFlow installed lifecycle](docs/reference/deployment/installed-lifecycle.md#product-boundary) |
| Exact semantic write and durable rrflowKV commit sequence | [Write and commit flow](docs/architecture/engine-data-flow.md#write-and-commit-flow) |
| Hybrid immutable storage and measured conditional zero-copy | [rrflowKV physical target](docs/architecture/engine-data-flow.md#rrflowkv-physical-target) and [conditional zero-copy](docs/architecture/engine-data-flow.md#conditional-zero-copy) |
| One bounded, deterministic, provider-neutral context operation | [Context assembly contract](docs/architecture/engine-data-flow.md#context-assembly-contract) |
| Exploratory discovery may be iterative; persisted project models, reproducible mutations, and evidence bind a deterministic project snapshot | [ADR-0002](docs/decisions/0002-adaptive-reasoning-governed-effects.md) and [project-tree inventory](docs/architecture/engine-data-flow.md#project-tree-inventory-and-incremental-attunement) |
| External databases, meshes, providers, and clients remain explicit adapters | [External integrations](docs/architecture/system-overview.md#external-integrations) |

## Product operation map

| Operation or surface | Owning record or gate |
|---|---|
| New/existing-project installation, specialization, and attunement | [Provider-neutral agent bootstrap](docs/reference/agent-bootstrap.md#installation-and-attunement) and [roadmap Gate D](docs/roadmap/rrflow-1.0.md#gate-d--install-configure-and-attune-one-real-estate) |
| Binary acquisition, install plan/apply, serve/readiness, verify, repair, backup/restore, salvage, and uninstall | [RRFlow installed lifecycle](docs/reference/deployment/installed-lifecycle.md) and [roadmap D-01/D-11](docs/roadmap/rrflow-1.0.md#gate-d--install-configure-and-attune-one-real-estate) |
| Deterministic project-tree snapshot and incremental refresh | [Project-tree inventory flow](docs/architecture/engine-data-flow.md#project-tree-inventory-and-incremental-attunement) and [roadmap D-03/D-04](docs/roadmap/rrflow-1.0.md#gate-d--install-configure-and-attune-one-real-estate) |
| Governed in-engine functions and proposed-transaction bindings | [RRFlow governed functions](docs/reference/automation/functions.md) and [roadmap A-07/C/I](docs/roadmap/rrflow-1.0.md#executable-dependency-spine) |
| Discovered project commands, installed capability bindings, and external activities | [Project command capability contract](docs/reference/automation/project-command-capabilities.md), [roadmap D-06](docs/roadmap/rrflow-1.0.md#gate-d--install-configure-and-attune-one-real-estate), and [roadmap I-03](docs/roadmap/rrflow-1.0.md#gate-i--add-explicit-automation-scaffolding-without-automatic-hooks) |
| Durable provider-neutral self and `rrflow://` record resolution | [Seat identity and memory warps](docs/reference/seat-identity.md) |
| Estate desired/observed state and fenced external effects | [RRFlow estate control](docs/reference/operations/estate-control.md) |
| Embedded, single-node, cluster, rrflowMX, rrflowKV, loopback, and network combinations | [RRFlow deployment profiles](docs/reference/deployment/modes.md) |
| Kubernetes projection, reconciliation, readiness, deletion, and qualification | [RRFlow Kubernetes deployment adapter](docs/reference/deployment/kubernetes-operator.md) |
| Distributed consistency, replication, recovery, and qualification | [RRFlow distributed cluster contract](docs/reference/distributed/cluster-contract.md) |
| Local RRD launch, authenticated readiness, and bounded shutdown | [RRFlow local process adapter](docs/reference/deployment/local-process-driver.md) |
| Rust and generated language clients over the one public operation authority | [RRFlow SDK reference](docs/reference/sdk/README.md) and [roadmap H-04](docs/roadmap/rrflow-1.0.md#gate-h--prove-context-flow-feedback-live-delivery-and-connectome) |
| Connectome connection, projections, interactions, and conformance | [RRFlow Connectome client contract](docs/reference/client/connectome.md), [client bootstrap boundary](docs/architecture/system-overview.md#client-bootstrap-boundary), and [roadmap H-06/H-07](docs/roadmap/rrflow-1.0.md#gate-h--prove-context-flow-feedback-live-delivery-and-connectome) |
| Engine events, triggers, routines, skills, and host-event adapters | [Automation, routine, and skill flow](docs/architecture/engine-data-flow.md#automation-routine-and-skill-flow), [generic routine packages](docs/reference/agent-bootstrap.md#generic-routine-package), and [roadmap Gate I](docs/roadmap/rrflow-1.0.md#gate-i--add-explicit-automation-scaffolding-without-automatic-hooks) |
| Current rrflowKV bytes and removable format readers | [rrflowKV current physical format](docs/reference/storage/rrflowkv-current-format.md) |
| Canonical source placement and generated implementation discovery | [Implementation navigation](docs/roadmap/rrflow-1.0-execution-map.md#frozen-target-source-tree) |
| Plan, author, instrument, verify, and journal one bounded code change | [Codebase-grounded change-authoring routine](docs/roadmap/rrflow-1.0-execution-map.md#codebase-grounded-change-authoring-routine) |
| Repository verification sequence | [Repository-wide run checklist](docs/roadmap/rrflow-1.0-execution-map.md#repository-wide-run-checklist) and [RRFlow CI operations](docs/operations/ci.md) |
| Repository work instructions | [`AGENTS.md`](AGENTS.md) |

## Current status

The persistent rrflowDB target is not complete or alpha-qualified. Existing
types, crates, tests, rrflowKV persistence, rrflowMX execution, rrflowQL, and
DataFusion integration are implementation inventory—not proof that the target
engine flow is finished. The exact present-versus-target boundary is maintained
in [Current implementation boundary](docs/architecture/engine-data-flow.md#current-implementation-boundary),
and all observed deficiencies are maintained in the
[POA&M](docs/poam/rrflow-1.0-alpha.md#open-deficiencies).

The canonical checklist and accepted evidence are in the
[release roadmap](docs/roadmap/rrflow-1.0.md#rrflow-10-execution-checklist).
A-06 is complete: the checkout knowledge package is classified,
content-addressed, reproducible, and ready for later authorized import. A-07 is
also complete: every current package/capability family is mapped to its
canonical destination; package, type, path, dependency, and repository-source
boundaries are frozen; and the 13-boundary causal-evidence vocabulary now has
closed operation, typed-link, type-checked attribute, propagation, and
diagnostic-export rules. Exact current trace names and attributes that await
their behavior-owning C-through-I gates are finite shrinking inventories, not
alternate APIs. This completes structural and evidence vocabulary only; it
does not qualify persistence, native indexes, DataFusion streaming, reasoning,
installation, or cross-surface observability. B-01 through B-05 are complete:
router model artifacts, tokenizer, output schema, capabilities, resource
limits, runtime ABI/device, quantization, and deterministic grammar now fail
closed before model loading. One authenticated `/v1/ws` connection now carries
the closed, bounded request, cancellation, subscription, delivery, ACK,
heartbeat, error, and backpressure vocabulary; the Rust client/server proof
covers multiplexed durable subscriptions, exact replay coordinates, and
malicious peers. Generic operation execution remains fail-closed until H-04;
GraphQL now parses and validates against a read-stamped schema catalogue and
lowers into the same bound rrflowQL `Query` representation, with no resolver,
storage call, authorization bypass, or second executor. Its outward HTTP
surface remains H-04 work. C-01 and C-02 are complete: live rrflowKV
application keys use the manifest-authenticated `RRKV0001` ordered codec, and
one snapshot-isolation transaction port now backs the claim, control,
projection, runtime, and invocation repositories for both rrflowMX and
rrflowKV. The broad profile-specific semantic store implementations are gone.
C-03 is also complete: one storage-neutral semantic plan now commits temporal
records and relations, both graph directions, schema-bound scalar/unique
changes, BM25/vector source deltas, runtime state, durable projection work,
governed-function receipts and proposals, semantic audit, outbox, cursor, and
outcome through one rrflowMX/rrflowKV transaction. The rrflowKV fault matrix
proves all-or-none recovery at prepared, WAL-appended, WAL-synced, and visible-
before-acknowledgement boundaries; public function-catalogue limits fit one
physical batch; corrupt runtime-build substitution fails closed; and a durable
receipt closes a lost acknowledgement without guest re-execution. C-04 is
reopened: current and temporal state is selected through authenticated,
budgeted semantic-version point/range reads at one `ReadStamp`, and normal
query, retrieval, context, memory, inference, and vector-candidate paths no
longer reconstruct state from the runtime change log. Vector search now reads
artifact and quantization catalogues with its candidates at one authenticated
stamp and within one request key budget. A retained-stamp regression and a
1,024-unrelated-record growth corpus pass through rrflowKV reopen; this is
bounded evidence, not massive-codebase qualification. C-04 remains open for
the remaining [POAM-028](docs/poam/rrflow-1.0-alpha/poam-028.md) closure work. Exact
rrflowMX/rrflowKV, rrflowKV reopen, and physical-counter evidence passes for
the direct reads. C-05 is complete
for the single-node alpha composition: retired physical readers are absent;
`rrd-store` is the only required production owner of `rrd-lsm`; schemas require
one explicit logical table map; and every persisted vector requires its
canonical collection plus named-vector address. The generic vector artifact
catalogue now admits only exact/compact and HNSW projections. Scalar, product,
binary, and TurboQuant artifacts enter the same serving planner only through
their explicit build/activate/retire quantization lifecycle; the duplicate
TurboQuant `ensure_vector_index` request, generic publication, replay
suppression, and successful fixtures are gone. The complete default workspace
test and strict Clippy matrices pass after that convergence. The unavailable
post-alpha OpenRaft implementation remains an explicit POA&M item and is not
part of the alpha composition. C-06 is complete: segment v6 supplies the ordered
key/version spine, six Arrow-layout page buffers, authenticated format and
writer-policy identities, measured mapped-versus-allocated ownership, and
authenticated configurable row-group targets applied by flush and compaction.
Its default writer independently retains each page raw or encodes it as an LZ4
block only when the authenticated 12.5-percent saving threshold is met; the
explicit `none` policy remains available. Stored bytes are authenticated before
bounded decoding, raw mmap pages may be borrowed, and compressed pages become
owned aligned Arrow buffers with decompression charged separately. Persisted,
authenticated row-group Bloom filters survive normal reopen without
semantic-page reads, prune definite point misses before key-page acquisition,
and remain acceleration metadata: positives still use the exact MVCC spine.
C-06g adds
a synchronous bounded projected storage stream over an owned sequence,
manifest, memtable, and segment generation. It selects key-spine pages first,
resolves one MVCC winner, suppresses winning tombstones, defers value pages by
projection, emits bounded Arrow-compatible buffers, pins the manifest closure
through cancellation/drop, and reports query work separately from segment-open
validation and rrflowKV startup reconciliation. Fixed and generated mixed-
family histories match the independent oracle through reopen and protected
compaction. C-06h now adds deterministic independent-model, stress, fault,
lifetime, and coverage-guided sanitizer qualification over that stream. The
integrated C-06i compression corpus proves exact none/adaptive semantics,
reopen and compaction behavior, raw/compressed ownership, bounded malformed
decode rejection, and 1,418,038 stored bytes for 8,988,877 logical page bytes
on one clean Linux revision. C-06j completes the physical-policy decision with
one family-neutral, exact-byte cache: scan-resistant probationary/protected LRU
is the default, exact LRU remains a selectable oracle/operator policy, and one
opaque engine-generated projected-stream scope prevents a scan from promoting
its own repeated page touches. On clean revision `5b1c31d`, the same persisted
eight-family corpus produced 48 post-scan hot-page loads under exact LRU and
zero under scan-resistant LRU, while recording 18,200 same-scope suppressions,
48 cross-operation promotions, 48 protected entries, identical manifest,
semantic, and projected-row digests, and exact 1 MiB capacity compliance.
Measured evidence rejects value separation, semantic-family cache partitions,
Moka, TinyLFU, and caller-controlled cache bypass for this accepted format.
D-01 is complete for the bounded installed-engine foundation. Its
[acceptance matrix and native receipts](docs/roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21)
bind one candidate to successful Linux, macOS and Windows isolation, native
filesystem/ownership denial, recovery and actual primary-binary lifecycle
checks. C-07 is the active executable item. The original walking prerequisite
remains: a source-built primary `rrflow` 1.0.0 executable can
deterministically preview and apply a fresh or existing local-project
installation, serve the generated public contract, complete an authenticated
readiness challenge, and perform mutation-free quick verification. The linked
[walking-product journal](docs/evidence/change-journals/gate-d/d-01-installed-ui-walking-product.md)
records the exact limits and evidence. The next bounded prerequisite also now
establishes one canonical project-local `.rrflow` estate, a portable locator,
owner-only credential and storage roots, and one sealed
`EstateConfiguration`. Installation can accept a bounded TOML input at plan
time; apply uses only the sealed plan; installed open and server discovery bind
the same project/estate/instance identity, independent deployment coordinates,
configuration revision, digest, and reasoning/recall/query ceilings. A real
process rejects over-ceiling query work as resource exhausted before executing
it. The linked
[canonical-estate journal](docs/evidence/change-journals/gate-d/d-01-canonical-estate-layout-and-configuration.md)
records the exact implementation and tests. A third bounded prerequisite now
makes that sealed installation the only product startup authority. The same
plan resumes after deterministic interruptions at each of ten durable-effect
stages, plus the pre-intent lease boundary,
without changing its first-apply time or duplicating its token, credential,
installed record, audit, checkpoint, or locator. The primary CLI, internal
server process, embedded MCP mode, six-language SDK conformance daemon,
deployment catalogue, and Kubernetes renderer now enter through installed
state; the pre-canonical manifest, private project binding, raw product CLI,
standalone security initializer, development supervisor, and server initializer
were removed directly. The linked
[authority-convergence journal](docs/evidence/change-journals/gate-d/d-01-precanonical-authority-convergence-and-install-recovery.md)
records the exact replacement and limits. A fourth bounded prerequisite now
removes caller-selected installation time. `RrdEngine` makes one fallible host
clock observation, labels it `host_system_time` / `unverified`, seals the first
observation into recovery and installed records, and applies the strict
format-2 `clock.maximum_rollback_ms = 0` default during recovery, replay, and
open. Read-only verification reports current, explicitly tolerated rollback,
exceeded rollback, or unavailable time without mutating the estate; unsafe
open fails closed. The same bounded abrupt-kill/reopen primary-binary test is
now scheduled on Linux, macOS, and Windows. The linked
[clock-substrate journal](docs/evidence/change-journals/gate-d/d-01-engine-owned-installation-clock.md)
records the exact boundary and proof. This does not authenticate or correct
UTC, establish runtime-wide temporal high-water marks, or qualify a native
platform distribution. The reasoning runner remains
truthfully unavailable. This is still not a streamed DataFusion `RecordBatch`
provider, native graph/BM25/vector access, complete attunement, turnkey release
distribution, or persistent reasoning/recall proof.

The source-built walking product is not yet a releasable installed
distribution. The primary executable now composes `version`, `install plan`,
`install apply`, `serve`, authenticated `ready`, read-only
`verify --level quick`, and offline `restore plan` and `restore apply`, which
publish a staged restore into the storage root's own name with the previous
root quarantined; its help and parser reject the removed raw-database and
development commands. Repair, uninstall, durable attunement execution, and
release service management remain absent. The retained internal server and
adapter binaries open only an already-installed estate, but they are not a
second supported operator command tree. Native primary-executable lifecycle
tests now pass on Windows, Linux and macOS; the
[native lifecycle journal](docs/evidence/change-journals/gate-d/d-01-native-installed-lifecycle.md)
records their bounded scope. No signed/offline-complete platform bundle has
been qualified. D-01 supplies the accepted single-executable installation
foundation;
C-07 owns wider installed storage recovery/lifetime, D-11 owns complete
repair/restore/uninstall, and Gate J owns
native signed platform distributions. See the
[installed lifecycle](docs/reference/deployment/installed-lifecycle.md) and
[POA&M](docs/poam/rrflow-1.0-alpha.md).

## Knowledge warp points

The checkout is RRFlow's bootstrap memory until authorized import and durable
rrflowDB readback pass. Each row provides one stable future data coordinate and
one repository fallback; the linked record owns the content.

| Memory | Durable warp | Checkout fallback |
|---|---|---|
| Knowledge structure and ownership | [`rrflow://rrflow-instance/data/documentation-index/rrflow-knowledge-map`](rrflow://rrflow-instance/data/documentation-index/rrflow-knowledge-map) | [`docs/README.md`](docs/README.md) |
| Master system overview | [`rrflow://rrflow-instance/data/architecture/system-overview`](rrflow://rrflow-instance/data/architecture/system-overview) | [`docs/architecture/system-overview.md`](docs/architecture/system-overview.md) |
| Project, estate, instance, and deployment topology | [`rrflow://rrflow-instance/data/architecture/instance-topology`](rrflow://rrflow-instance/data/architecture/instance-topology) | [`docs/architecture/instance-topology.md`](docs/architecture/instance-topology.md) |
| Deployment profiles and cross-profile conformance | [`rrflow://rrflow-instance/data/reference/deployment/modes`](rrflow://rrflow-instance/data/reference/deployment/modes) | [`docs/reference/deployment/modes.md`](docs/reference/deployment/modes.md) |
| Installed product lifecycle | [`rrflow://rrflow-instance/data/reference/deployment/installed-lifecycle`](rrflow://rrflow-instance/data/reference/deployment/installed-lifecycle) | [`docs/reference/deployment/installed-lifecycle.md`](docs/reference/deployment/installed-lifecycle.md) |
| Kubernetes deployment adapter | [`rrflow://rrflow-instance/data/reference/deployment/kubernetes-operator`](rrflow://rrflow-instance/data/reference/deployment/kubernetes-operator) | [`docs/reference/deployment/kubernetes-operator.md`](docs/reference/deployment/kubernetes-operator.md) |
| Distributed cluster contract | [`rrflow://rrflow-instance/data/reference/distributed/cluster-contract`](rrflow://rrflow-instance/data/reference/distributed/cluster-contract) | [`docs/reference/distributed/cluster-contract.md`](docs/reference/distributed/cluster-contract.md) |
| Single-engine authority decision | [`rrflow://rrflow-instance/data/decision/0001-single-engine-authority`](rrflow://rrflow-instance/data/decision/0001-single-engine-authority) | [`docs/decisions/0001-single-engine-authority.md`](docs/decisions/0001-single-engine-authority.md) |
| Adaptive reasoning and governed effects decision | [`rrflow://rrflow-instance/data/decision/0002-adaptive-reasoning-governed-effects`](rrflow://rrflow-instance/data/decision/0002-adaptive-reasoning-governed-effects) | [`docs/decisions/0002-adaptive-reasoning-governed-effects.md`](docs/decisions/0002-adaptive-reasoning-governed-effects.md) |
| Engine data flow | [`rrflow://rrflow-instance/data/architecture/engine-data-flow`](rrflow://rrflow-instance/data/architecture/engine-data-flow) | [`docs/architecture/engine-data-flow.md`](docs/architecture/engine-data-flow.md) |
| rrflowKV current physical format | [`rrflow://rrflow-instance/data/reference/storage/rrflowkv-current-format`](rrflow://rrflow-instance/data/reference/storage/rrflowkv-current-format) | [`docs/reference/storage/rrflowkv-current-format.md`](docs/reference/storage/rrflowkv-current-format.md) |
| Seat identity and memory warps | [`rrflow://rrflow-instance/data/reference/seat-identity`](rrflow://rrflow-instance/data/reference/seat-identity) | [`docs/reference/seat-identity.md`](docs/reference/seat-identity.md) |
| Provider-neutral agent bootstrap | [`rrflow://rrflow-instance/data/reference/agent-bootstrap`](rrflow://rrflow-instance/data/reference/agent-bootstrap) | [`docs/reference/agent-bootstrap.md`](docs/reference/agent-bootstrap.md) |
| Governed functions and transaction bindings | [`rrflow://rrflow-instance/data/reference/automation/functions`](rrflow://rrflow-instance/data/reference/automation/functions) | [`docs/reference/automation/functions.md`](docs/reference/automation/functions.md) |
| Project command capabilities and activities | [`rrflow://rrflow-instance/data/reference/automation/project-command-capabilities`](rrflow://rrflow-instance/data/reference/automation/project-command-capabilities) | [`docs/reference/automation/project-command-capabilities.md`](docs/reference/automation/project-command-capabilities.md) |
| SDK references | [`rrflow://rrflow-instance/data/reference-index/sdk`](rrflow://rrflow-instance/data/reference-index/sdk) | [`docs/reference/sdk/README.md`](docs/reference/sdk/README.md) |
| Connectome client contract | [`rrflow://rrflow-instance/data/reference/client/connectome`](rrflow://rrflow-instance/data/reference/client/connectome) | [`docs/reference/client/connectome.md`](docs/reference/client/connectome.md) |
| CI and runner operations | [`rrflow://rrflow-instance/data/operations/ci`](rrflow://rrflow-instance/data/operations/ci) | [`docs/operations/ci.md`](docs/operations/ci.md) |
| Product version policy | [`rrflow://rrflow-instance/data/reference/release/version-policy`](rrflow://rrflow-instance/data/reference/release/version-policy) | [`docs/reference/release/version-policy.md`](docs/reference/release/version-policy.md) |
| Alpha objectives | [`rrflow://rrflow-instance/data/objective/rrflow-1.0-alpha`](rrflow://rrflow-instance/data/objective/rrflow-1.0-alpha) | [`docs/objectives/rrflow-1.0-alpha.md`](docs/objectives/rrflow-1.0-alpha.md) |
| Release roadmap | [`rrflow://rrflow-instance/data/roadmap/rrflow-1.0`](rrflow://rrflow-instance/data/roadmap/rrflow-1.0) | [`docs/roadmap/rrflow-1.0.md`](docs/roadmap/rrflow-1.0.md) |
| Alpha POA&M | [`rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha`](rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha) | [`docs/poam/rrflow-1.0-alpha.md`](docs/poam/rrflow-1.0-alpha.md) |
| Execution navigation portal | [`rrflow://rrflow-instance/data/execution-map/rrflow-1.0`](rrflow://rrflow-instance/data/execution-map/rrflow-1.0) | [`docs/roadmap/rrflow-1.0-execution-map.md`](docs/roadmap/rrflow-1.0-execution-map.md) |
| Linked change journals | [`rrflow://rrflow-instance/data/evidence-index/change-journals`](rrflow://rrflow-instance/data/evidence-index/change-journals) | [`docs/evidence/change-journals/`](docs/evidence/change-journals/) |

The [knowledge map](docs/README.md#record-header-and-indexing-pattern) defines
how these reviewed Markdown owners become deterministic JSON/JSONL import
records and, after the persistence gates pass, rrflowDB-resolved memory without
creating a second editable source of truth.
