# RRFlow instance topology

**Status:** active accepted logical and physical topology; canonical installed startup implemented, resource and controller convergence remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/architecture/instance-topology`
**Owner:** project, engram, RRD instance, environment, deployment, and physical-placement relationships
**Decision:** [`../decisions/0001-single-engine-authority.md`](../decisions/0001-single-engine-authority.md)

The repository root [README](../../README.md) owns product identity and current
status. The [system overview](system-overview.md) owns component meaning, the
[engine data-flow record](engine-data-flow.md) owns execution sequence, the
[engram-control reference](../reference/operations/engram-control.md) owns
desired/observed operational semantics, the
[deployment-profile reference](../reference/deployment/modes.md) owns valid
deployment-form, storage-profile, and endpoint-presentation combinations plus
their conformance boundaries, the
[local-process adapter](../reference/deployment/local-process-driver.md) owns
host-local launch/readiness/shutdown effects, the
[Kubernetes adapter](../reference/deployment/kubernetes-operator.md) owns the
Kubernetes projection and effect boundary for one installed instance, and the
[roadmap](../roadmap/rrflow-1.0.md) owns implementation order and evidence. This
record owns only the topology that those components inhabit.

## Locked alpha topology

RRFlow 1.0 installs one independent AI engram for one project. One logical RRD
instance serves exactly that engram through one `RrdEngine` authority. Embedded,
single-node server, and clustered execution are deployment forms of that same
instance; rrflowMX and rrflowKV are storage profiles beneath the same semantics.

```text
project ──governed-by──> engram (rrflowDB)
                              ^
                              │ serves exactly one
                         RRD instance
                              │
                    one logical RrdEngine authority
                              │
              ┌───────────────┬───────────────┐
              │               │               │
       deployment form   storage profile  endpoint presentation
    embedded/server/cluster  MX or KV     in-process/HTTP/WebSocket
```

The relationships are deliberately not a single generic parent chain. A
project is the governed external work, an engram is its RRFlow semantic state,
an instance is the installed runtime identity, an environment is observed
project context, and a cluster is physical execution. Conflating them would
make paths, policy, storage, and multi-project aggregation ambiguous.

## Canonical identities and cardinality

| Identity or term | Meaning | Required relationship | Not allowed to mean |
|---|---|---|---|
| **project** | An explicitly user-bounded unit of software or product work whose selected source may contain one repository or several related repositories. | Exactly one project identity is bound by a first-alpha installation; the selected repository boundaries are part of that binding. | A filesystem path, Cargo workspace, tenant, database, or operator fleet. |
| **engram** | The complete project-scoped RRFlow semantic boundary: governed knowledge, temporal graph, memories, reasoning state, indexes, evidence, automation, and operations. It is the logical rrflowDB. | Exactly one engram governs exactly one project in the first alpha. | A fleet of projects, a deployment controller, cluster, storage directory, or JSON aggregate. |
| **RRD instance** | The stable identity of one installed embedded or daemon runtime serving one engram. | Exactly one instance serves one engram; all public faces resolve to its one logical `RrdEngine`. | A process ID, host, endpoint, project path, node, database namespace, or client session. |
| **environment** | An explicit named project context such as development, test, staging, production, device, or region, captured with source and observation evidence. | Zero or more environment descriptors belong to the project engram; every environment-scoped fact names one explicitly. | A required ancestor of the engram/instance, an inferred directory name, or a hidden selector for another database. |
| **workspace** | A set of engrams that an account or client groups above the engine, for example every project one developer works across. The grouping is a client or account record. | Each member engram keeps its own instance, authentication, and authority; a workspace never spans an engram's scope. Cross-engram context is an explicit client-side or federated read across an opt-in [project link](../reference/data/rrflowdb-capability.md#project-links). | A project, engram, deployment, authorization scope, storage profile, tenancy boundary, or an engine resource of any kind. The source topology inside one project (packages, modules, generated boundaries, harnesses) is owned by the committed project-tree snapshot, not by this term. |
| **organization/account/entitlement** | Optional external operator, billing, or policy-provider references. An account may own many engrams. | They may be linked through an explicitly configured adapter and policy; the account → engrams relation lives at the identity or control provider, never inside an engram. | A mandatory local RRFlow parent hierarchy or an authority that can bypass `RrdEngine`. |
| **tenant** | An optional security or data-partition subject whose semantics must be defined by a later schema/policy contract. | If used, policy and every affected operation name the exact tenant scope. | An implicit container between the instance and engram or a synonym for project. |
| **domain** | A configurable named part of one engram's catalogue, such as a service, package or subject area. It groups kinds, scopes reads and grants, may be isolated, and is measured on its own. The [engram organisation contract](../reference/data/engram-organisation.md) owns it ([ADR-0003](../decisions/0003-configurable-engram-organisation.md)). | Every kind and vector collection belongs to exactly one domain of its engram; domains nest at most 8 deep; the bundled default has one project domain. An isolated domain stays inside its engram's transactions, cursor and backup. | An engram, instance, database, storage root, tenant, deployment or principal; a part of a kind's identity; or a container that spans engrams. |
| **namespace/database** | Superseded by domains. The engram is the one database, and a domain is its catalogue namespace and the SQL schema of the PostgreSQL projection. H-11a removed the inert registry label from the [schema catalogue](../reference/data/schema-catalogue.md). | No topology path depends on either value; rrflowDB is the project engram. | Mandatory SurrealDB-style containers, deployment identities, or additional persistence authorities. |

Stable IDs are canonical values independent of display names, paths, endpoints,
processes, and provider identifiers. Human labels can change without changing
identity. A physical locator can be evidence in an installed binding, but it
cannot become the resource ID.

Multi-project views do not weaken these cardinalities. Connectome or another
operator can aggregate several independently authenticated RRD instances as a
client. No local engram becomes a fleet database, and no instance silently
opens another project's state.

### User-selected repository boundaries

A repository is a source boundary, not automatically an engram. When setup finds
a nested repository, it offers the user the choice of excluding it, treating it
as a separate project with its own engram and RRD instance, or including both
repositories in one explicitly bounded project and engram. Recommend separate
projects by default; the user decides. Source membership and any cross-project
link are distinct choices. A discovery or recommendation has no effect until the
user accepts an exact installation plan.

In the shared choice, each selected repository retains its own source identity,
Git inputs and revision evidence inside one project, while the engram keeps one
engine, authorization boundary, transaction cursor and backup. In the separate
choice, each project has an independent identity, engine, store, credentials,
read stamp and lifecycle; cross-project reads require the existing explicitly
authorized project-link contract and do not become cross-engram transactions.
No selection silently claims another already-installed engram or merges its data.

The [D-03 and H-06 acceptance](../roadmap/rrflow-1.0/gate-d.md#nested-repository-setup)
requires this choice on the real RRFlow and Connectome checkouts. It is a target:
the current installer cannot yet admit the submodule Git metadata or implement
this choice. The [verified gap](../poam/rrflow-1.0-alpha/poam-006.md#nested-repository-setup-gap)
records the observed failures.

## Logical engram model

The logical relationship graph is:

```text
project ──governed-by──> engram
engram  ──served-by────> RRD instance
engram  ──contains─────> canonical records, relations, schemas, memories,
                          vectors, reasoning trees, events, routines, skills,
                          checkpoints, evidence, audit, outbox, and indexes
project ──observed-as──> project-tree snapshots and environment descriptors
```

Tables, collections, records, relations, aliases, vector points, lexical
postings, and graph adjacency are semantic or derived data families inside the
engram. They do not introduce another database engine. Schema and policy decide
which families exist and how they are authorized; topology never chooses an
alternate graph, vector, search, or analytical authority.

Every canonical fact and every derived projection is engram-scoped. The engine
spells that scope as `instance:{instance_id}`, the identity of the one instance
serving the engram, so instance scope and engram scope are the same boundary. A
derived index additionally binds its source cursor and relevant schema, model,
and configuration revisions. Cross-engram reads are explicit client-side or
authorized federation operations; they are never a local prefix accident or a
claim of cross-instance ACID.

## Deployment and physical placement

Deployment form, storage profile, and endpoint presentation are orthogonal.
The [deployment-profile reference](../reference/deployment/modes.md) owns their
valid combinations, default alpha posture, runtime discovery, and layered
conformance:

| Dimension | Values | Invariant |
|---|---|---|
| Deployment form | embedded, single-node server, clustered server | The same instance, engram semantics, operation catalogue, security decisions, and result contracts remain visible. |
| Storage profile | rrflowMX, rrflowDB backed by rrflowKV | Non-durability-specific behavior is equivalent. Only rrflowKV can claim crash/reopen, backup, recovery, or replicated durability. |
| Endpoint presentation | in-process, loopback HTTP/WebSocket, or configured network HTTP/WebSocket | Reachability does not establish identity or authorization. An optional mesh adapter may resolve or carry a configured network endpoint but cannot create another presentation or authority. |

rrflowMX is not the hot tier of rrflowDB and does not automatically flush into
rrflowKV. rrflowKV's memtable and cache are the hot tiers of a persistent
rrflowDB. Moving accepted data from rrflowMX to rrflowKV is an explicit,
authorized semantic transaction, not a startup side effect.

The physical clustered relationship graph is:

```text
RRD instance ──deployed-as──> cluster
cluster      ──contains────> node
instance     ──partitions──> shard
shard        ──has-copy────> replica ──placed-on──> node
replica      ──materializes> rrflowKV segment/object artifacts
```

A cluster is one instance's consensus and placement domain. It does not own
several project engrams. A node is a physical process/host member, a shard is a
routing and consensus partition of that instance's state, a replica is one
copy placed on a node, and a segment is an internal rrflowKV or index artifact.
Changing placement, leadership, replica health, or segment generation cannot
change project, engram, or instance identity.

The current cluster contracts contain useful placement epochs, quorum,
snapshot-vector, routing, artifact-transfer, and reshard safety checks. They are
implementation inventory until the cluster package proves that those checks
operate on this topology through the canonical engine and storage contracts.
The [distributed cluster contract](../reference/distributed/cluster-contract.md)
owns their exact preserve/replace disposition and qualification boundary;
`clustered_server` remains unavailable and is not a first-alpha exit
requirement.

## Installation and binding resolution

Only the previewed and explicitly applied `rrflow install` operation may create
or change an installed topology. Installation has two distinct layers:

1. `.rrflow/config.toml` is a minimal project-local locator generated from a
   bundle-resident versioned template. It provides only enough information to
   find the installed instance/profile and a credential reference. It contains
   no canonical mutable state, plaintext secret, inferred provider hook, or
   second project model.
2. A canonical installed-engram binding is committed through `RrdEngine`. It
   binds the stable instance, engram, and project identities to the selected
   deployment form, storage profile, endpoint presentations, project-inventory
   precondition, installed configuration/template/attunement revisions, and
   security authority. The project-local locator uses relative paths; absolute
   host paths and endpoints are never identities or authorization.

The exact serialized fields remain owned by D-01. The semantic requirements do
not: a valid binding must make the following checks possible before ordinary
state is read or changed.

| Required check | Failure behavior |
|---|---|
| Locator format/profile is supported and its referenced installation exists. | Fail closed; startup must not create or repair it. |
| Instance, engram, and project identities match the canonical binding. | Fail closed without opening another scope. |
| The requested directory contains the locator and every referenced path remains inside it; nested, neighboring, symlink-escaped, mount-escaped, and foreign stores are rejected. | Fail before project content or rrflowDB state is mutated. |
| Configuration, template, attunement profile, and security revisions match their committed digests. | Require an explicit preview/apply reconfiguration operation. |
| Credential reference resolves and the authenticated principal is authorized for the requested operation/resource. | Deny without falling back to anonymous or provider identity. |
| Selected storage profile satisfies the requested durability capability. | Return an explicit unsupported-capability result; never silently switch profiles. |

Normal `rrd-server`, embedded, CLI, MCP, SDK, and Connectome startup resolves
this installed binding read-only and then opens the one engine. No client or
daemon startup path may call an `ensure`, `initialize`, `load-or-create`, or
directory-derived identity function to manufacture installation authority.

Moving the complete project tree does not rename its project, engram, or
instance and does not require an absolute-path rewrite. Relocating only the
data root, changing profile or endpoints, or rebinding project content is an
explicit previewed operation with a new plan digest. It validates containment,
preserves stable semantic identities where authorized, commits the new binding
and evidence atomically, and leaves the old locator unusable. RRFlow 1.0 has no
successful earlier manifest reader, implicit migration, or read-old/write-new
branch.

## Routing and authorization

Ingress resolves in this order:

```text
configured endpoint
  -> authenticated RRD instance identity and capability handshake
    -> installed engram/project binding
      -> typed public operation and operation-specific resource coordinate
        -> policy decision at the operation's read/transaction stamp
          -> RrdEngine execution
```

Network locality, a mesh peer, a project directory, an organization account,
or possession of a data path never grants access. HTTP, WebSocket, native SDK,
MCP, CLI, and Connectome must present the same logical operation and receive
the same authorization result, stamp, digest, and evidence.

A generic resource path validator that merely rejects empty or repeated kinds
is insufficient. A-07 must freeze operation-specific path grammars and parent
relationships. The public contract, security policy, engram state, installed
binding, and cluster placement must use one spelling and identity family; a
historical vocabulary table cannot constrain active public types.

## External systems and adapters

PostgreSQL, Turso, SQLite, Dragonfly, object stores, generators, test harnesses,
CI systems, deployment controllers, model providers, Wardenclyffe/Zuul Zero or
other meshes, and Connectome are optional external capabilities or clients.
Attunement can discover a typed inactive candidate. Activation requires an
exact previewed adapter binding, configuration, policy, authorization, limits,
and credential reference.

An adapter may contribute source data, evidence, an explicitly authorized
effect, endpoint resolution, or UI. It cannot become rrflowDB persistence,
rrflowQL planning, index authority, project identity, installation authority,
or a second reasoning/event/routine lifecycle. Generated or observed output is
re-inventoried and committed through `RrdEngine` before it becomes engram state.

## Current implementation audit

The passing current tests establish one canonical installed startup identity
and remove its competing pre-canonical product paths. They do not complete the
resource grammar, duplicate engram hierarchy, cluster binding, or release
qualification required by the accepted topology.

| Current construct | Useful behavior to preserve | Conflict to remove directly |
|---|---|---|
| `InstalledEngramIdentity`, `EngramLayout`, and `.rrflow/config.toml` | The install plan carries distinct project/engram/instance IDs, eight ordered project-relative managed paths, no fabricated organization or absolute host path, and one locator bound to an engine-owned installed record/configuration. Apply freezes first-apply time and secret bytes in a private plan-addressed intent, classifies partial state, resumes the exact plan across eight typed durable stages, publishes one locator, and removes acknowledged recovery state. Exact apply/open/verify, whole-project relocation, nested/neighbor/foreign rejection, interruption, tamper, and pre-canonical-collision tests pass. | This is the sole product startup destination, but not full D-01: hostile pathname/mount replacement, native ACL/platform qualification, durable attunement, repair/restore/uninstall, and signed offline release evidence remain open. |
| Removed pre-canonical startup authority | `.rrflow/instance.toml`, `InstanceManifest`, `InstanceMode`, `ProjectAuthorityBinding`, `open_project_store`, `open_bound`, `bind_project_authority`, `rrd-server initialize`, `ensure_dedicated*`, raw primary-CLI commands, the development supervisor, and the standalone security initializer have no current product source or successful reader. CLI, internal server, embedded MCP, SDK conformance, deployment-catalogue, and Kubernetes-rendered startup use installation/open instead. | Retain no reader, alias, migration lane, or second success path. Historical evidence may name deleted symbols but cannot make them callable. Create-or-open construction remains only in component tests that do not claim product installation. The retained application-backup driver is deliberately open-existing-only and fails on canonical installed layout until its later lifecycle package supplies locator/distribution-bound resolution. |
| `PLATFORM_TERMS`, `ResourceKind`, and `ResourcePath` | Bounded typed resource components and duplicate-kind rejection. | The active table mirrors a superseded historical document; environment is omitted; organization/engram/project/instance roles conflict; and arbitrary kind order is accepted. Replace with operation-specific canonical topology/resource contracts. |
| `EngramAuthorityResourceKind` and `rrd-engram::AuthorityResourceKind` | Bounded IDs, desired/observed validation, parent existence, receipt lineage, and some strict relationships. | The organization/account/project/environment/instance/node/shard/job/health/secret hierarchy duplicates installation, security, engram, job, health, and cluster owners inside a monolithic engram document. Split every useful semantic into its one owner and remove the catalogue. |
| `rrd-cluster` identity and placement contracts | Multi-zone voter checks, placement epochs, read stamps, snapshot-vector consistency, explicit cross-shard denial, transfer digests, and reshard cutover checks. | Cluster IDs use a separate loose string family and free-form scope strings; the contract is not yet bound to installed instance/engram identity or one engine transaction. Converge it at the cluster/deployment gates without making cluster an engram owner. |

The historical platform-vocabulary note remains provenance only. An active
test that requires `PLATFORM_TERMS` to match its word order is itself a tracked
defect, not proof of canonical terminology.

## Direct-convergence sequence

1. **A-07 vocabulary and traceability:** freeze the identities, relationship
   graph, operation-specific resource paths, module/type names, and exact
   preserve/remove map across contract, engine, engram, server, CLI, SDK/MCP,
   and cluster callers.
2. **C-02/C-03 storage semantics:** represent the installed binding, engram,
   topology relations, audit, outbox, and runtime log through the same
   authorized semantic transaction on rrflowMX and rrflowKV. Remove private
   JSON control authority rather than wrapping it.
3. **D-01 install:** retain the implemented bundle-resident configuration,
   portable `.rrflow/config.toml`, canonical installed-engram identity, no-write
   preview, exact apply/open boundary, eight-stage interruption recovery, and
   direct removal of the pre-canonical startup authorities. Complete the
   remaining hostile-path/native-platform, attunement, maintenance, and release
   proofs without adding another lifecycle lane.
4. **D-02/D-03 attunement:** persist jobs and commit the bounded project tree
   using the installed root and stable project/engram identities. Environments,
   workspaces, and external systems enter only as evidenced project data or
   inactive capability candidates.
5. **E/F query and indexes:** make graph, scalar, BM25, vector, and analytical
   reads engram-scoped at one stamp; Arrow/DataFusion computes over selected
   data and never changes topology or binding.
6. **H/I clients and automation:** expose one topology through every surface;
   routines and adapters resolve the installed engine and cannot create
   instances or cross engrams.
7. **J release proof:** reject every superseded manifest/path/shape, install an
   empty and existing project from the signed offline bundle, restart, and show
   identical authenticated identity plus durable query/reasoning evidence.

## Characterization evidence and missing proof

At the current bounded-package revision, these focused tests pass:

- `rrd-engine`'s installation unit corpus: sixteen tests for byte-identical
  planning, exact apply/open/verify, configuration sealing, symbolic and
  pre-canonical collisions, relocation, nested/foreign state, eight-stage
  retry, stable secrets/time, partial commit, and recovery-state tamper;
- `rrflow-cli/tests/installed_lifecycle.rs`: four primary-binary tests for the
  reduced command surface, required plan digest, installed service/authenticated
  discovery, quick verification, close/reopen, and raw/development-command
  rejection;
- `rrd-contract/tests/platform_terminology.rs`: three tests freezing the
  current term table and its historical-file order;
- `rrd-engram/tests/authority_catalogue.rs`: two tests for the duplicate
  authority catalogue and selected rrflowKV reopen behavior; and
- `rrd-cluster/tests/contracts.rs`: ten tests for placement, stamps, routing,
  transfer, telemetry, and reshard contract validation.

The first two bullets prove the local installed-startup replacement, not the
whole topology. The corpus does not prove one operation-specific resource
vocabulary, complete rrflowMX/rrflowKV semantic parity, clustered engine
operation, governed reconfiguration, hostile mount/path races, native-platform
ACLs, or signed offline deployment. Those remain open in the roadmap and
POA&M. The exact replacement map and commands are in the linked
[Gate D change journal](../evidence/change-journals/gate-d/d-01-precanonical-authority-convergence-and-install-recovery.md).

## Acceptance

The topology is implemented only when a clean bundle performs a no-write
preview and one explicit install for both an empty and existing project; every
surface resolves the same stable project, engram, and instance through one
authenticated `RrdEngine`; rrflowMX and rrflowKV pass the complete
storage-profile semantic differential; rrflowKV survives reopen;
environment/workspace/external capability
records cannot alter authority; physical placement cannot alter logical
identity; an authorized whole-project relocation succeeds while unauthorized,
nested, neighboring, escaped, foreign, partial-move, and superseded bindings
fail closed; and no `.rrflow/instance.toml`, initializer,
generic historical path hierarchy, private binding store, or alternate reader
remains.
