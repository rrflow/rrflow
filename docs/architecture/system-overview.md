# RRFlow system overview

**Status:** active accepted master platform architecture; implementation gaps remain open
**Coordinate:** `rrflow://rrflow-instance/data/architecture/system-overview`
**Owner:** canonical platform components, authority boundaries, and high-level relationships
**Decision:** [`../decisions/0001-single-engine-authority.md`](../decisions/0001-single-engine-authority.md)
**Reasoning decision:** [`../decisions/0002-adaptive-reasoning-governed-effects.md`](../decisions/0002-adaptive-reasoning-governed-effects.md)

The repository root [README](../../README.md) is the bootstrap product portal.
This record is the first architecture destination behind that portal. It tells
an operator or AI system what every RRFlow component means and where detailed
architecture lives. The [instance-topology record](instance-topology.md) owns
project, estate, RRD instance, environment, deployment, and physical-placement
relationships. The [engine data-flow record](engine-data-flow.md) owns the
complete read, write, Arrow/DataFusion, and context flows; the
[roadmap](../roadmap/rrflow-1.0.md) owns implementation order and completion.

## Platform map

```text
RRFlow — AI governance, reasoning, recall, and context platform
│
├── RRD — embedded and daemon runtime
│   └── RrdEngine — sole semantic, security, and mutation authority
│
├── rrflowDB — persistent per-project AI estate
│   └── rrflowKV — WAL/MVCC/LSM physical persistence
│       ├── hot mutable memtable
│       └── immutable key/version and Arrow-compatible segment target
│
├── rrflowMX — volatile process-local execution profile
│
├── rrflowQL — query language, binding, planning, and execution
│   ├── native KV, temporal graph, lexical, and vector access
│   └── stamped Arrow batches → bounded DataFusion execution
│
├── RRFlow vector subsystem
│   └── canonical vectors → filtered candidates → HNSW/quantized indexes
│       → exact reranking
│
├── RRFlow inference — replaceable embedding adapters and the LFG and SWAG
│   plug-in configurations, every model reached through a configured endpoint
├── RRFlow seats — durable provider-neutral identities; Clyffy is this
│   repository's primary specialization
├── RRFlow attunement — bounded project evidence → committed derived knowledge
├── RRFlow automation — committed events, triggers, routines, and skills
├── RRFlow Security — identity, sessions, policy, authorization, and audit
└── HTTP / WebSocket / SDK / MCP / CLI / Connectome — outward clients
```

These are cooperating layers of one system. They are not separately
authoritative databases or engines.

## Canonical component terminology

| Term | Canonical meaning | Implementation boundary |
|---|---|---|
| **RRFlow** | Reason Ready Flow, the complete product and platform. | This repository supplies the engine and bundled adapters; Connectome is a separate public-contract UI client pinned as an optional submodule. |
| **RRD** | Reason Ready Daemon, the embedded/server runtime hosting RRFlow. | `rrd-server` plus embedded `RrdEngine` composition. |
| **`RrdEngine`** | The only semantic coordinator and mutation authority. | `crates/authority/rrd-engine` |
| **RRFlow kernel** | Canonical temporal values, identities, read stamps, mutations, and invariants; it defines meaning without owning transport or physical storage. | `rrd-core` |
| **canonical runtime log** | The ordered source of truth for committed RRFlow changes from which temporal snapshots are resolved. | Semantic types in `rrd-core`, persistence mapping in `rrd-store`, and durable bytes in rrflowKV. |
| **rrflowDB** | The pure Rust, PostgreSQL-inspired database dedicated to AI and designed for AI use, with PostgreSQL compatibility as a product requirement. Each instance stores a persistent project AI estate containing governed knowledge, reasoning state, evidence, and index definitions. Its configurable prompt graph families, memory contract, and opt-in project links are defined by the [rrflowDB capability record](../reference/data/rrflowdb-capability.md). | Semantic state composed by `RrdEngine`; physical durability supplied by rrflowKV. PostgreSQL compatibility remains unqualified. |
| **rrflowKV** | The durable physical engine beneath rrflowDB: WAL, MVCC, LSM, manifests, snapshots, compaction, and recovery. | `rrd-lsm` through `rrd-store` |
| **rrflowMX** | RRFlow Memory Execution, the non-durable process-local implementation of the same semantic storage port. | Volatile implementation in `rrd-store`, behind the semantic storage port. |
| **RRFlow temporal graph** | Typed records and relations resolved from the canonical log at one runtime read stamp and valid-time coordinate; never a separate graph database. | Kernel graph values, persistence adjacency, native query operators, and `RrdEngine` authorization. |
| **RRFlow memory** | Durable temporal claims, records, relations, schemas, vectors, and evidence owned by the same engine; never a separate memory store. The "memory database" is rrflowDB itself. | rrflowDB semantic data composed through `RrdEngine`. |
| **RRO** | Reason Ready Object: one persisted, versioned unit of knowledge the engine reasons over, such as a record, relation, claim, vector, or reasoning node, read at one stamp with its evidence. The abbreviation names no component, role, or provisioning path. | Kernel values in `rrd-core`; persistence through `rrd-store`; no type carries this name. |
| **RROFLOW** | The recursive self-improvement configuration of RRFlow: committed reasoning outcomes, verdicts, and corrections promote into better rules, exemplars, and claims, each promotion an authorized, versioned, reversible engine change, never an automatic hook. | No roadmap item owns it yet; nearest owners are Gate G reasoning trees and Gate I triggers, routines, and skills. A record precedes any work. |
| **rrflowQL** | RRFlow Query Language: syntax, AST, binding, logical and physical planning, and execution. | `rrd-query` |
| **Arrow substrate** | The columnar buffer model for eligible immutable rrflowKV pages and stamped analytical batches. | Apache Arrow types composed by storage and query boundaries. |
| **DataFusion execution** | Bounded vectorized computation over stamped Arrow batches. It cannot authorize or commit state. | DataFusion integration inside `rrd-query` |
| **fast path** | Low-latency point, state, pointer, and bounded graph work that does not invoke DataFusion. | `RrdEngine` over the selected semantic storage profile and native operators. |
| **analytical path** | Broad retrieval, ingest transformation, joins, and analytics planned by rrflowQL over native access paths and stamped Arrow batches. | `rrd-query`, `rrd-vector`, `rrd-store`, and bounded DataFusion execution. |
| **index projection** | Derived acceleration state bound to its source cursor and relevant schema or catalogue revision; it is rebuildable and cannot outrank canonical data. | Scalar, graph, lexical, and approximate-vector structures in persistence and compute boundaries. |
| **context assembly** | Bounded discovery and deterministic fusion owned by `RrdEngine`, returning a read-stamped `ContextPacket` with evidence. | `rrd-contract` operation and `rrd-engine` implementation. |
| **prompt flight** | One durable, comparable record of a prompt crossing the runtime boundary: its context arm, engine-assembled context coordinates, an edge to every canonical record its context used, and append-only observable stage events at one read stamp; never hidden chain-of-thought. Each flight is a graph neighbourhood rrflowQL traverses; flights of one trimmed prompt share a cohort digest. | `flight-launch`/`flight-list`/`flight-read` in `rrd-contract`; lifecycle record, `rrflow-prompt-flight-used` edges, and events through `RrdEngine`. |
| **task** | One named, durable unit of intended work (title, goal, and status) that prompt flights join through `rrflow-task-includes` edges written in each flight's own commit. It groups work and orchestrates nothing, unlike a routine. | `task-open`/`task-close`/`task-list`/`task-read` in `rrd-contract`; lifecycle record and edges through `RrdEngine`. |
| **project-tree snapshot** | One deterministic, bounded, committed inventory of observed root-relative project entries, errors, containment, policy, and change evidence; it is required when RRFlow persists a project model, applies a reproducible project mutation, or claims snapshot-complete evidence, but not merely to inspect or reason about available files. | Pure proposal construction in planned `rrd-attunement`; authorized durable reads and commit coordination in `RrdEngine`. |
| **RRFlow vector subsystem** | Native vector-database capability inside rrflowDB: exact values, payload filters, candidate indexes, and exact reranking. | `rrd-vector`, `rrd-query`, and `RrdEngine` |
| **RRFlow inference** | Provider-neutral, manifest-gated execution of embedding and routing models, each reached through a configured endpoint so that a local or remote binding changes no behavior. | `rrd-inference`; LFG and SWAG are the plug-in configurations below. |
| **LFG** | A CPU-based, optional plug-in configuration that classifies work and feeds SWAG data points such as complexity. Whether a model runs inside it is undecided. | No code. Gate G's current text predates the LFG/SWAG split and is revised before Gate G work starts. |
| **SWAG** | The System One controller and model gateway: the plug-in configuration that makes the fast decision, selects a model through its configured endpoint, and dispatches the call. It replaces the earlier working name Dalal; the concept is unchanged. | No code or record yet; a record precedes any work. It never gains planning, authorization, or persistence authority, which stay in `RrdEngine`. |
| **Governance** | Deterministic-first verification and permission: checks named as data on terminal reasoning nodes, the permission scope checked at the effect boundary before any verification, and the proof pattern (name the seam, freeze it with a golden file, prove it with a kept artifact). | `verification_requirement` on reasoning nodes (`rrd-core`), authorization in `RrdEngine`; the permission-scope type does not exist yet. |
| **RRFlow seat** | A durable provider-neutral identity represented by one or more authenticated provider identities at a read coordinate; representation establishes attribution, not authorization. Clyffy is this repository's primary seat specialization, not another kernel, repository, runtime, or provider. | Canonical seat/provider/relation values in rrflowDB; resolution, authorization, routing, and mutation through `RrdEngine`. |
| **engine event** | One immutable, identity- and provenance-bound occurrence committed through `RrdEngine`; it is data, not a callback or lifecycle owner. | Canonical event value in the RRFlow kernel, public submission envelope in `rrd-contract`, and semantic persistence through `rrd-store`. |
| **trigger** | A persisted deterministic condition over committed engine events that may propose an authorized operation or routine start but cannot mutate independently. | `RrdEngine` evaluation over rrflowDB state. |
| **routine** | A versioned, durable graph of public `RrdEngine` operations with explicit state, budgets, leases, checkpoints, cancellation, compensation, verification, and terminal outcome. | Definition and run state in rrflowDB; orchestration in `RrdEngine`. |
| **skill** | An immutable, digest-addressed instruction and resource package resolved as governed context; it grants no capability and executes no lifecycle code. | Contract and manifest in `rrd-contract`; content and bindings in rrflowDB; resolution in `RrdEngine`. |
| **host-event adapter** | An optional, explicitly installed stateless translator from a host callback or external signal into the canonical engine-event submission operation. | Outward `rrflow-*` adapter; never an automatic hook or scheduler. |
| **Connectome** | The RRFlow client: a separate operator and developer workbench using public RRD capabilities. The name no longer means a storage substrate; that role is RRFlow's. | Separate repository pinned at `connectome/`; no embedded engine authority. |
| **shippin.cloud** | Planned hosted RRFlow offering with private development spaces, project execution, UI previews, and web deployment. | Cloud delivery and account services consume explicit public engine and project-execution contracts. |
| **DevPulse** | Planned desktop application offering a local RRFlow engine and optional shippin.cloud signup and synchronization. | Local client and distribution integration; engine state remains under `RrdEngine`. |
| **workspace** | A set of estates grouped by an account or client above the engine. It is never an engine scope, resource, or authorization boundary; the [instance-topology record](instance-topology.md#canonical-identities-and-cardinality) owns the relation. | Client and account concern; no engine type. |

Product documentation uses the exact spellings **RRFlow**, **RRD**,
**`RrdEngine`**, **rrflowDB**, **rrflowKV**, **rrflowMX**, **rrflowQL**,
**RRO**, **RROFLOW**, **LFG**, and **SWAG**.
Rust package names remain `rrd-*` for internal daemon/runtime boundaries and
`rrflow-*` for outward product adapters.

"Trigger" is reserved for the post-commit engine-event condition above.
Synchronous functions bound to a proposed transaction are transaction function
bindings, not routines or event triggers. "Runtime event" must not survive as
a second event vocabulary when I-01 freezes the canonical engine event.

## Pre-release convergence boundary

RRFlow 1.0 has one current target implementation and no legacy product,
compatibility, migration, or deprecation line. Existing pre-release formats,
backends, names, APIs, missing-field defaults, and alternate success paths are
either part of the accepted target or superseded implementation residue. The
accepted result contains no alternate database, query, graph, vector,
security, context, or lifecycle path and no forwarding alias, old-shape
decoder, read-old/write-new branch, or migration executor that keeps
superseded RRFlow state executable.

Before an existing implementation is replaced or removed, its useful
semantics, invariants, tests, fixtures, and measured behavior must be mapped to
the canonical boundary and owning roadmap gate. The
[implementation-requirements traceability matrix](../roadmap/rrflow-1.0-execution-map.md#implementation-requirements-traceability)
is the execution control for that accounting. A branch relationship, merge,
rename, compilation result, or file deletion does not prove that behavior was
integrated.

Direct convergence follows one absorption rule:

1. Read the complete affected source, test, fixture, and owning record.
2. Classify each behavior as required canonical semantics, a useful invariant
   to generalize, or superseded residue to remove.
3. Bind every retained requirement to one canonical module, operation, data
   family, and acceptance test under `RrdEngine`.
4. Implement and prove the equal-or-stronger canonical behavior.
5. Remove the competing reader, default, alias, format, path, success fixture,
   and terminology in the same owning gate. Retain earlier bytes only as a
   negative rejection fixture when they materially prove fail-closed behavior.

Interoperability terms such as type-compatible Arrow buffers, compatible
embedding spaces, S3-compatible APIs, and negotiated public protocol versions
describe current technical contracts; they do not authorize an older RRFlow
execution path. An upstream dependency namespace that happens to contain the
word `legacy` is likewise not an RRFlow surface, but it must remain isolated
behind its owning adapter.

## Persistence and memory boundary

rrflowDB is the database; a persistent project estate is the governed data it
holds. Its product target is a pure Rust database engine inspired by PostgreSQL,
dedicated to AI workloads and designed for AI systems to use. Knowledge,
temporal memory, reasoning state, graph and vector retrieval, context, and
evidence are native database concerns. PostgreSQL compatibility is part of
this target.

The [PostgreSQL compatibility contract](../reference/protocol/postgresql.md)
owns the selected core profile, SQL/type/transaction/client matrix, semantic
mapping, data portability and qualification. [H-08](../roadmap/rrflow-1.0/gate-h.md#h-08-postgresql-compatibility-packages)
owns implementation order. The current outward PostgreSQL/pgvector adapter
connects to an external database; it does not implement an inbound compatible
server. [POAM-011](../poam/rrflow-1.0-alpha/poam-011.md) remains open until the
new profile has real-client and installed-engine evidence.

Compatibility must enter through RRFlow's public transport/query boundaries
and preserve `RrdEngine` authorization, semantics, transactions, and receipts.
rrflowKV remains rrflowDB's native physical storage engine; PostgreSQL is not a
required server or persistence backend. The rrflowKV memtable is the hot
mutable tier inside a persistent rrflowDB instance. The pure Rust identity is
a database-engine requirement, not an audited claim that every optional adapter
or transitive platform dependency is implemented in Rust.

rrflowMX is different: it is an intentionally non-durable composition for
volatile and conformance execution. It does not silently flush or promote into
rrflowDB. Moving state from rrflowMX to rrflowDB requires an explicit,
authorized `RrdEngine` transaction whose result is subject to the ordinary
schema, policy, audit, and durability rules.

The [local database integration](../reference/sdk/rrflowdb-local.md) contains
component and relational diagrams plus runnable Rust client operations.
[DevPulse and optional cloud sync](../reference/client/devpulse-cloud.md) keeps
local installation independent of shippin.cloud account creation and transfer.

## Native multi-model boundary

Temporal records, claims, relations, reasoning trees, project-tree snapshots,
graph adjacency, lexical postings, vectors, engine events, routine state, skill
bindings, and evidence are rrflowDB data families. They share one transaction
coordinator, read-stamp model, canonical runtime log, and persistence authority.

The temporal graph is not another graph database. RRFlow vector search is not
a separately authoritative vector database. BM25 is not another text store.
Exact vector values may be canonical rrflowDB values; HNSW and quantized
structures are derived candidate indexes and cannot outrank their source data.

## Query and analytical boundary

rrflowQL selects a native fast path or the analytical path. Fast point, range,
CAS, and bounded graph navigation does not require DataFusion. Analytical work
reads one authorized stamp, obtains native KV/graph/lexical/vector candidates,
streams eligible rrflowKV state as Arrow batches, and evaluates bounded
DataFusion operators before deterministic exact reranking or reciprocal-rank
fusion.

DataFusion computes. It never owns the database, WAL, authorization decision,
read stamp, or commit. Any transformed result returns to `RrdEngine` as a
proposal and passes semantic validation and authorization before persistence.

The accepted detailed flow and current implementation differences are owned
by [engine-data-flow.md](engine-data-flow.md). In particular, the present
checkout still materializes rows into newly allocated Arrow arrays and has not
implemented the target Arrow-compatible rrflowKV segment pages.

## Security boundary

RRFlow Security is cross-cutting and enforced by `RrdEngine`:

| Boundary | Required authority | Prohibited bypass |
|---|---|---|
| Ingress | Typed public operation and bounded request | Transport-specific semantic execution |
| Identity and session | Authenticated principal, lease, and instance scope | Provider identity becoming an RRFlow authority |
| Read | Policy, valid time, and one `ReadStamp` | Index, model, or client selecting unstamped state |
| Compute | Authorized plan and resource budget | DataFusion or LFG granting permissions |
| Mutation | Schema validation, effect authorization, and transaction coordination | Direct model, adapter, hook, or DataFusion storage writes |
| Evidence | Correlated audit, plan, source, and commit coordinates | Trace events substituting for canonical state |

## Source and distribution boundary

RRFlow is self-contained at two different boundaries that must not be
conflated:

| Boundary | Must contain | Must not require |
|---|---|---|
| First-party source | Every RRFlow engine, persistence, query, vector, inference-adapter, transport, install, attunement, release, verification, backup, and recovery implementation; all schemas, templates, fixtures, and generated-source inputs | A sibling checkout, local path outside the repository, undeclared or unpinned Git submodule, untracked generator, or host-specific absolute path |
| Signed default distribution | RRD and operator executables; linked rrflowKV, rrflowQL, Arrow/DataFusion, graph, BM25, vector, security, and engine code; public schemas/goldens; bootstrap and attunement templates; default configuration; runtime/model assets required by the declared default profile; verifier; licenses/SBOM; recovery material and runbook | A compiler, Cargo/npm cache, package registry, source checkout, external database/query/vector service, Connectome, mesh, or provider connection |

The source build may resolve content-locked third-party packages through the
normal toolchain. That does not authorize a deployed RRFlow instance to fetch
code or required runtime assets. Release assembly records dependency digests
and licenses, and installation must work with outbound network access denied
after the signed bundle is acquired. An optional model or adapter bundle may be
side-loaded only when the operator selects it explicitly; anything required by
the default profile belongs in the default distribution.

Connectome remains a separate client release, pinned in this repository as the
optional `connectome/` UI submodule under the declared, pinned-source policy.
Engine and bundled-adapter builds cannot consume source or assets from it.
An engine-only checkout and default distribution need no submodule acquisition.
The [client contract](../reference/client/connectome.md#repository-checkout)
owns its source identity and explicit acquisition procedure.

PostgreSQL, Turso, SQLite,
Dragonfly, object stores, Zuul Zero/shippin.ai meshes, and model providers
remain optional integrations. None is needed to make a local rrflowKV estate
ready, commit data, close, reopen, recover, or verify itself.

## Client bootstrap boundary

Connectome is an optional operator and developer workbench, not part of the
engine source or default readiness authority. Its first connection to an RRD
instance uses the public HTTP operations in this order:

```text
GET /v1/health/live
GET /v1/health/ready
GET /v1/capabilities
```

The client accepts the runtime only after validating the RRD protocol version
and returned instance resource. Plain HTTP is restricted to loopback;
off-device or mesh-resolved endpoints require HTTPS. A Zuul Zero or
shippin.ai adapter may resolve a devspace endpoint and carry opaque transport
attestation, but reachability does not authenticate the RRD instance and the
mesh never owns database state.

Connectome invokes installation, attunement, context, query, trace, and live
delivery through public RRD capabilities. It does not reproduce phase state,
retrieval, authorization, or persistence in the client. The
[Connectome client contract](../reference/client/connectome.md) owns connection,
projection, interaction, local-state, and client-conformance requirements. The
[agent-bootstrap reference](../reference/agent-bootstrap.md#installation-and-attunement)
owns the project-installation and attunement contract; roadmap
[H-06](../roadmap/rrflow-1.0.md#gate-h--prove-context-flow-feedback-live-delivery-and-connectome)
owns the real-client acceptance evidence.

## Delivery experiences

The root [product portal](../../README.md#engine-ui-cloud-and-desktop) owns the
offering: one RRFlow reasoning engine, Connectome UI, hosted private devspaces
through shippin.cloud, and local operation through DevPulse. These are target
delivery experiences; neither this record nor submodule registration proves
that they are available.

| Experience | Intended behavior | Authority boundary |
|---|---|---|
| Connectome UI | Present engine state and invoke available public operations from its independently built UI. | RRFlow's pinned submodule identifies UI source; the public client contract governs connections and effects. |
| shippin.cloud | Offer RRFlow as a cloud service with an included private devspace, initially for friends-and-family demonstrations of AI-assisted development. Supply project creation, Rust/Python/Go execution, private running-UI previews, and web launch capabilities. | Hosted provisioning, toolchains, execution, preview routing, and deployment are explicit services/adapters; they do not become another reasoning or persistence engine. |
| DevPulse | Offer the local reasoning engine and invite the user to create a shippin.cloud account and synchronize selected work. | Account creation, project selection, transfer permissions, and synchronization are explicit choices; signup alone grants no upload or mutation permission. |

A devspace is the hosted project development environment, distinct from an
RRD instance or its estate. Running project code uses a configured execution
capability with explicit filesystem/network permissions, resource and time
budgets, cancellation, and observable results. A private UI preview is scoped
to its authorized devspace; publishing a web application is a separate,
explicit deployment effect. A successful build or preview cannot claim that
the engine's reasoning or persistence gates passed.

DevPulse's local instance and a hosted instance retain their exact project,
estate, and instance identities. Synchronization needs a versioned public
contract covering selected data, authorization, conflicts, retry uncertainty,
and receipts. Copying storage files or equating an account/workspace with an
estate cannot substitute for that contract. The local engine remains usable
without a cloud account or synchronization.

The [Connectome client contract](../reference/client/connectome.md),
[project command capabilities](../reference/automation/project-command-capabilities.md),
and [instance topology](instance-topology.md) own the existing integration
boundaries. H-06/H-07 and [POAM-011](../poam/rrflow-1.0-alpha/poam-011.md)
track client/transport qualification. Hosted provisioning, language execution,
preview/deployment, account, and synchronization implementations require their
own bounded packages and acceptance evidence before being advertised as ready.

## External integrations

PostgreSQL, Turso, SQLite, Dragonfly, application databases, object stores,
mesh networks, and model providers are operator-configured external
integrations. Attunement may discover them but cannot activate them. They never
become rrflowDB's implicit persistence, transaction, query, security, or
context authority.

## Documentation and future memory

During bootstrap, Markdown records remain the reviewed authoring authority.
The root and boundary READMEs are portals now, and each warp retains one local
checkout fallback. The deterministic JSON/JSONL knowledge package is a
generated, content-addressed import artifact rather than a second editable
truth.

Ordinary Markdown links and record metadata also produce generated parent
indexes and a coordinate relationship graph. Those projections make records
self-maintaining and retrievable without duplicating their bodies. They can be
rebuilt and never decide truth, authorization, roadmap completion, or POA&M
status. The same single-source rule applies to the executable public catalogue:
dispatch, OpenAPI, internal bindings, language SDKs, reference views, and
conformance are projections, not parallel APIs.

rrflowDB becomes the normal durable warp-resolution path only after authorized
import, readback, restart, idempotency, drift-denial, and recovery evidence
passes. Architecture explanations live here, decisions live under
`docs/decisions/`, exact contracts live under `docs/reference/`, delivery order
lives in the roadmap, and observed gaps live in the POA&M.
