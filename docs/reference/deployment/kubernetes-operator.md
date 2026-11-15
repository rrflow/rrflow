# RRFlow Kubernetes deployment adapter

**Status:** active target deployment reference; current controller is non-conforming implementation inventory and no Kubernetes profile is alpha-qualified
**Coordinate:** `rrflow://rrflow-instance/data/reference/deployment/kubernetes-operator`
**Owner:** Kubernetes projection, reconciliation, readiness, deletion, security, and qualification for one installed RRD instance

RRFlow is the database and reasoning system. Kubernetes is one optional outward
deployment environment for that system. The Kubernetes adapter may materialize
and observe an engine-authorized deployment plan; it cannot become a second
source of project, engram, instance, policy, transaction, job, or lifecycle
truth.

The [instance topology](../../architecture/instance-topology.md) owns project,
engram, RRD instance, and physical-placement identity. The
[deployment-profile reference](modes.md) owns deployment form, storage profile,
and endpoint presentation. The [engram-control reference](../operations/engram-control.md)
owns prepared external effects, fencing, observations, and receipts. The
[security authority](../security/authority.md) owns identity and authorization.
The [engine data flow](../../architecture/engine-data-flow.md) owns rrflowKV,
native graph and indexes, rrflowQL, Arrow/DataFusion, reasoning, and recall.
This record owns only their Kubernetes projection and proof boundary.

## Accepted decision

The target package is `crates/adapters/rrflow-kubernetes`, named as an outward
RRFlow adapter. The current `crates/operations/rrd-kubernetes` package is not
renamed in place and declared correct: A-07 must preserve its useful
Kubernetes mechanics while moving them behind the accepted engine plan and
receipt contracts, then remove the old package directly.

The first alpha Kubernetes profile is exactly:

```text
one project
  -> one engram / rrflowDB
  -> one installed RRD instance
  -> deployment form: single_node_server
  -> storage profile: rrflow_kv
  -> one RRD pod with one retained writable data volume
  -> authenticated network_http_websocket endpoint
```

It is not `clustered_server`, Multi-AZ, or a set of independent databases.
Until a later distributed gate is added and accepted, the adapter must reject
replica counts greater than one and must not advertise cluster availability.

## Authority and dependency boundary

```text
signed RRFlow distribution + explicit operator install request
                         |
                         v
              D-01 install preview/apply
                         |
              immutable installation/effect plan
                         |
             +-----------+------------+
             | bootstrap handoff      | subsequent operation
             v                        v
   bounded installer workload     RrdEngine prepared intent
             |                        |
             +-----------+------------+
                         v
               rrflow-kubernetes adapter
                         |
       Kubernetes resources + bounded observations
                         |
                         v
        authenticated RRD readiness / effect receipt
                         |
                         v
                    RrdEngine commit
```

The adapter may depend on implementation-free contracts and Kubernetes client
libraries. It must not open rrflowKV, construct `RrdEngine`, evaluate RRFlow
policy, parse rrflowQL, build an index, invoke DataFusion, or run reasoning.
An installed/offline composition root supplies its exact plan and later
submits observations and receipts through public engine operations.

Kubernetes object state is external observation. A successful API patch, Pod
phase, Job completion, Service address, TCP connection, or status condition
does not commit RRFlow state. Only an accepted `RrdEngine` transaction can
advance the canonical installed binding, engram operation, observation,
receipt, audit, outbox, or cursor.

## Installation and cold start

The empty-volume case cannot depend on an already-running daemon, and it does
not justify a second installer. D-01 must use one bootstrap path:

1. `rrflow install --target kubernetes` resolves only the verified distribution's
   versioned project template, installed profile, adapter capability, and
   credential references.
2. Preview performs no Kubernetes or volume writes and emits the exact
   immutable plan, resource effects, field ownership, required cluster
   capabilities, secret references, data-retention result, and artifact
   digests.
3. Explicit apply submits a sealed, digest-bound bootstrap handoff. It contains
   no plaintext secret and grants no general RRFlow mutation authority.
4. A bounded installer workload mounts the new volume and executes the same
   engine-owned D-01 install apply used by other deployment forms. It commits
   the project/engram/instance binding, installed profile, security bootstrap,
   and idempotent operation evidence through `RrdEngine`.
5. The RRD workload may start only from that committed binding. Missing,
   mismatched, stale, foreign, or partially committed installation state fails
   closed; startup never creates or repairs it implicitly.
6. Authenticated application readiness proves the expected project, engram,
   instance, storage profile, configuration, capability, and artifact digests.
   The adapter returns that observation for engine acceptance.

The bootstrap handoff is a transient, immutable projection needed to create
the first canonical state. It cannot be edited into a permanent Kubernetes
copy of rrflowDB and cannot authorize subsequent changes. After installation,
every deployment change begins as an authenticated `RrdEngine` operation with
prepared intent before the adapter performs an external effect.

There is no `rrd-server initialize`, separate database-path security bootstrap,
startup-created instance manifest, or provider-specific session hook in the
target path.

## Plan projection contract

The Kubernetes API carries the minimum non-secret projection needed to execute
one prepared plan. Exact public Rust and wire types are frozen in A-07/B-04;
the semantic fields must bind:

- operation, idempotency, project, engram, and RRD instance identities;
- installed deployment form, storage profile, and endpoint presentation;
- desired generation plus engine lease/fence or equivalent stale-worker token;
- plan, configuration, capability, distribution, and executable-image digests;
- resource, placement, disruption, timeout, retry, and diagnostic bounds;
- data-volume class, capacity, access mode, snapshot capability, and explicit
  retention/destruction decision;
- credential, trust, and configuration references without secret values;
- exact adapter contract/API identity and plan expiry; and
- issuer/verification evidence sufficient to reject a fabricated, replayed,
  foreign-engram, or altered projection.

The plan must not contain caller-selected absolute host paths, caller time as
authorization truth, raw credentials, mutable rrflowDB records, a second
policy document, or arbitrary command/argument/environment injection. A
Kubernetes user cannot acquire RRFlow authority by editing a Custom Resource.
A changed projection requires a new engine-authorized generation and plan
digest.

## Kubernetes API contract

`rrflow.io/v1alpha1` may remain the external Kubernetes API identity while it
is technically accurate. That API version is independent of RRFlow's frozen
`1.0.0` product version, rrflowKV formats, rrflowQL grammar, and protocol
versions. Pre-release convergence retains exactly one served/storage schema;
there is no conversion webhook, successful earlier-shape decoder, alias, or
migration promise.

The Custom Resource is a declarative Kubernetes projection, so its schema
must follow Kubernetes conventions without promoting Kubernetes desired state
to RRFlow truth:

- `.spec` is closed, bounded, validated, immutable where the plan requires it,
  and bound to one engine-issued generation/digest;
- `.status.observedGeneration` states exactly which projection was observed;
- `.status.conditions` uses standard `type`, `status`,
  `observedGeneration`, `lastTransitionTime`, `reason`, and `message` fields;
- stable conditions such as `Ready`, `Progressing`, `Degraded`, and
  `DeletionBlocked` expose independent facts instead of one extensibility-
  hostile phase enum;
- status may include bounded applied-plan, observed-resource, effect-receipt,
  artifact, and endpoint-candidate digests/identities; and
- no condition or status field claims the canonical operation completed until
  its receipt has been accepted by `RrdEngine`.

The CRD remains a cluster-scoped API extension installed explicitly by a
cluster administrator from the verified distribution. The runtime controller
does not create, update, or convert its own CRD.

## Reconciled Kubernetes resources

The exact set is emitted by the versioned installation/effect template, not
hardcoded as a hidden product configuration inside the controller. The first
alpha is expected to need:

- a dedicated ServiceAccount and namespace-scoped RBAC for the controller;
- one bounded installer Job or equivalent non-concurrent bootstrap workload;
- one single-replica StatefulSet for the installed RRD server;
- one retained RWO PersistentVolumeClaim, subject to an explicit admitted
  storage class/capability plan;
- a headless Service only if stable workload identity requires it;
- one client Service for the configured HTTP/WebSocket presentation;
- a NetworkPolicy with explicit selected clients, DNS/control dependencies,
  and required egress rather than a namespace-wide trust shortcut;
- a PodDisruptionBudget whose limited voluntary-disruption guarantee is
  described honestly; and
- optional configuration/trust projections containing references or public
  material, never plaintext credentials copied into rrflowDB or status.

Owner references and Kubernetes garbage collection handle ordinary dependent
resources. A finalizer is used only when a specific external cleanup or
retention decision cannot be expressed by ownership and must complete before
deletion.

Resource requests/limits, storage size/class, placement constraints, security
context, timeouts, and probe budgets are versioned template/policy inputs with
safe distribution defaults. They are visible in preview and bound to the plan
digest. They cannot remain unexplained constants in renderer code.

## Reconciliation, idempotency, and field ownership

For each active plan generation the adapter must:

1. validate the closed projection before Kubernetes I/O;
2. reject a stale generation, fence, expiry, foreign owner, digest mismatch,
   or unsupported cluster capability;
3. render deterministically from the exact accepted plan;
4. apply only fields assigned to its stable server-side-apply field manager;
5. treat an unexpected field-owner conflict as an observation requiring an
   explicit decision, not unconditionally force ownership away;
6. watch every owned resource kind and recover from relist, duplicate events,
   watch closure, controller restart, and a lost acknowledgement;
7. observe API-defaulted/live objects, normalize only documented volatile
   fields, and compute a distinct observed-resource digest;
8. submit bounded per-boundary evidence under the stable operation/effect
   identity; and
9. accept completion only after `RrdEngine` commits the matching receipt.

Controller leader election may use a Kubernetes Lease when more than one
controller replica is qualified. It never replaces the engine's operation
lease, fencing token, idempotency identity, or prepared-before-effect rule.

The adapter needs separate names for the planned-resource digest, observed-
resource digest, and engine-accepted receipt. A digest computed only from
locally rendered JSON is not an “applied” digest.

## Readiness and health

Kubernetes probes and RRFlow readiness answer different questions:

| Signal | Required meaning |
|---|---|
| Startup | the process has completed bounded startup and can answer the narrow health mechanism; it protects slow initialization from premature liveness restarts |
| Liveness | the process event loop is responsive enough to recover from a deadlock; it does not run a broad database query or depend on an external provider |
| Readiness | the installed binding is open, the expected rrflowKV writer is exclusive, required security/configuration revisions are active, and authenticated RRFlow operations can be served |
| Adapter deployment receipt | challenge-bound proof identifies the expected project, engram, instance, plan, image, configuration, storage profile, and capabilities; `RrdEngine` has accepted it |

A TCP probe proves only that a socket accepted a connection. It cannot prove
the database, authenticated identity, rrflowKV state, graph/index projections,
rrflowQL, Arrow/DataFusion, or reasoning flow. If the Kubernetes built-in probe
cannot carry the required client authentication, use a narrowly scoped local
health helper/command for Pod readiness and retain the authenticated remote
challenge for the engine receipt. Service traffic is admitted only after both
the Kubernetes readiness condition and RRFlow's application-level boundary
are satisfied.

## Storage and single-engine behavior

The operator does not deploy separate graph, vector, lexical, analytical, or
reasoning services. The one RRD process owns this flow:

```text
authenticated request
  -> RrdEngine authorization and stamp
  -> native temporal graph / scalar / BM25 / vector access
  -> rrflowQL physical choice
     -> bounded fast rrflowKV path
     -> or stamped Arrow RecordBatch stream into DataFusion
  -> persisted reasoning/context/evidence transaction
  -> rrflowKV WAL/MVCC/LSM acknowledgement
```

Kubernetes persistence is the volume carrying rrflowKV; it does not replace
rrflowKV and does not make a container filesystem or Custom Resource into
rrflowDB. rrflowMX is a separate explicitly installed volatile profile, not a
memtable that automatically flushes into this volume. The first Kubernetes
alpha targets rrflowKV because the product objective requires persistent
per-project reasoning and recall.

The PVC is retained by default. Deleting a workload, Custom Resource, release,
or namespace must not silently authorize data destruction. Snapshot, backup,
restore, relocation, expansion, storage-class change, and destruction are
separate engine operations with their own capability checks, prepared intents,
receipts, failure tests, and operator confirmation.

## Deletion and finalizers

Deletion is a fenced engine operation, not an eager five-resource loop:

1. the engine records the requested disposition, retention result, active
   holds, backup/snapshot requirement, target generation, and prepared effect;
2. the adapter prevents new service traffic and observes workload quiescence;
3. it deletes only resources owned by the exact operation/plan identity;
4. retained data remains untouched unless a separately previewed destructive
   operation was explicitly authorized;
5. it waits for required resources to be absent and records bounded evidence;
6. `RrdEngine` accepts the receipt; and
7. only then may the adapter remove a finalizer whose purpose is satisfied.

Retry, controller restart, missing resources, stuck termination, namespace
termination, unreachable engine, failed backup, stale fence, and lost receipt
must all converge without releasing protection early or deleting foreign data.

## Security and network boundary

- Namespace-scoped operation is the first-alpha default. Cluster-wide watch
  and mutation require a separate previewed mode, explicit operator approval,
  tenant/isolation analysis, and conformance evidence.
- RBAC grants only the exact read/watch/apply/status/finalizer verbs and
  resource kinds used by the chosen template. Secret `get`, `list`, and
  `watch` remain denied to the controller unless a later design proves a
  narrower unavoidable need; the kubelet may mount named Secrets into the RRD
  Pod without giving the controller their contents.
- The controller's service-account token is mounted only where Kubernetes API
  access is required. The RRD data-plane Pod has no API token by default.
- Pods meet the Restricted Pod Security profile or record an exact justified
  exception. Images are digest-pinned and must pass J-05 provenance,
  signature, SBOM, vulnerability, and multi-architecture verification.
- A namespace label alone is not RRFlow authorization. NetworkPolicy provides
  transport reachability defense-in-depth; mutual authentication and
  `RrdEngine` policy still govern every operation.
- Wardenclyffe/Zuul Zero or another mesh may carry or resolve the configured
  endpoint through the H-07 adapter. Mesh membership does not create an
  engram, identity, permission, deployment receipt, or storage authority.

## Current implementation audit

The current package and checked manifests are characterization inputs only:

| Current path or behavior | Observed reality | Required disposition |
|---|---|---|
| `crates/operations/rrd-kubernetes/Cargo.toml` | Depends on `rrd-contract` and `rrd-core`, not `rrd-engine`; the controller receives no prepared engine operation or receipt port. | Move useful pure/API mechanics into `rrflow-kubernetes`; consume the accepted implementation-free plan/observation contracts and an injected public client boundary. |
| `RrdInstanceSpec` | Now carries contract version, digest-pinned image, PVC policy, TLS Secret name, and one installation-plan ConfigMap name plus exact SHA-256. Caller-selected instance ID, caller time, bootstrap manifest, and credential Secret fields are absent. | This is the correct cold-start projection shape, but it is not yet an engine-issued/fenced effect contract and still exposes hardcoded controller policy. Preserve the sealed-plan handoff while moving lifecycle authority outward. |
| `desired_resources` | Deterministically hardcodes five resources, paths, commands, CPU/memory, ports, storage behavior, and security settings. It now renders one `rrflow install apply --plan ... --expect ...` init container and an installed-only `rrd-server --project ... --distribution-executable ...` main process. | Preserve the single plan handoff and deterministic closed rendering, but drive all operational values from a versioned, previewed, digest-bound template/plan with explained bounds. |
| removed initializer/bootstrap containers | The server initializer, standalone security bootstrap, caller time, raw database root, caller identity, and credential-file arguments are absent from the CRD and rendered startup. | Retain no second cold-start path; prove idempotent real-volume install/reopen and engine-accepted observations before this renderer counts as deployment evidence. |
| server probes | Startup, liveness, and readiness are all TCP socket checks. | Split their meanings and add authenticated, identity/digest-bound application readiness plus an engine-accepted receipt. |
| controller watch | Watches all namespaces and only declares the StatefulSet as an owned watch source. | Default to bounded namespace scope and reconcile/watch every owned kind; qualify relist/restart behavior. |
| server-side apply | Calls `.force()` for every rendered resource. | Own exact fields with a stable manager; surface foreign conflicts and require an explicit engine plan before any ownership transfer. |
| status | Declares Ready from `ready_replicas == 1`, fabricates a Service DNS endpoint, stores one phase enum, and labels a desired-JSON digest “applied.” | Use standard conditions, live/defaulted observation, authenticated readiness, distinct digests, and receipt acceptance. |
| finalizer | Issues background deletes for five names and immediately returns success without absence, backup, retention, fence, or receipt proof. | Use owner GC for ordinary objects; gate necessary finalization on the full deletion contract above. |
| checked RBAC/namespace | The controller is cluster-wide; the supplied Namespace labels every Pod in that namespace as an RRD client. It does correctly omit Secret read verbs. | Ship a namespace-scoped least-privilege default; select individual client namespaces/workloads explicitly; retain Secret-read denial. |
| StatefulSet/PVC/PDB/security context | One replica, digest-pinned image, retained RWO data, non-root/read-only-root/capability-drop/seccomp settings, no RRD service-account token, PDB, and NetworkPolicy are useful foundations. | Preserve through plan-bound templates and API-server tests; do not imply availability, safe upgrades, or application readiness. |
| CRD generator and tests | Prove deterministic JSON, local CEL shape, five-resource rendering, and string-level RBAC assertions only. | Keep golden/unit tests, then add real API-server, controller, effect-gap, security, installation, and engine corpus evidence. |
| example image | Contains an explicit placeholder digest and is not a runnable distribution. | Generate examples only from a verified release candidate; J-03/J-05 own deployable artifacts. |

## Preservation and rejection matrix

| Capability | Disposition | Equal-or-stronger evidence required before old code is removed |
|---|---|---|
| Closed namespaced CRD, status subresource, admission validation, deterministic generator | Preserve and strengthen | schema golden, unknown/old-shape rejection, API-server admission, one served/storage version, standard-condition tests |
| Digest-pinned image and bounded DNS/quantity validation | Preserve and generalize | distribution-manifest binding, image/signature verification, property/fuzz corpus, exact rejection reasons |
| Deterministic resource rendering and stable field manager | Preserve | same-plan byte/semantic equality, observed-default normalization, manager ownership/conflict corpus |
| Single replica and retained RWO volume | Preserve for first alpha | clean install/reopen, writer exclusion, node restart, retained deletion, capacity/ENOSPC, storage capability tests |
| Restricted container settings and data-plane token denial | Preserve | Pod Security admission and negative RBAC/secret/API-token tests |
| Owner references, PDB, and NetworkPolicy | Preserve with honest limits | API-server ownership/GC, voluntary/involuntary disruption, explicit network allow/deny corpus |
| Kubernetes-owned desired RRFlow state, phase lifecycle, and readiness | Reject | sealed engine plan, standard observations/conditions, authenticated readiness, accepted receipt, repository absence checks |
| Single sealed install-plan handoff and installed-only server invocation | Preserve and strengthen | engine-issued plan/fence, ConfigMap byte/digest verification, idempotent real-volume install/reopen, authenticated readiness, and accepted receipt |
| Remaining hardcoded commands, paths, probes, and resources | Reject | versioned preview/apply templates, explained bounds, exact artifact binding, and no hidden controller authority |
| Cluster-wide default watch and forced SSA takeover | Reject | namespace-scoped RBAC/watch, conflict/fence tests, explicit opt-in cluster mode if ever added |
| Eager finalizer deletion | Reject | retained-data, backup/hold, absent-resource, stuck-delete, stale-fence, lost-ack, restart, and receipt tests |

## Direct-convergence sequence

1. **A-07:** trace every current source/test/manifest/caller; freeze the outward
   package name and dependency direction; move the package to
   `crates/adapters/rrflow-kubernetes`; split API, plan, rendering,
   reconciliation, readiness, status, and finalization seams; update Cargo,
   CI, development diagnostics, and architecture checks in the same direct
   cutover. No forwarding crate remains.
2. **B-04/C-02/C-03:** freeze the minimal plan/observation/receipt envelopes
   and engine-owned installed/effect transaction. The adapter receives no
   storage handle and Kubernetes status remains external observation.
3. **D-01/D-02:** implement network-denied preview/apply, the bounded installer
   workload, idempotent engine job/receipt persistence, restart recovery, and
   the exact new/existing project engram binding.
4. **H-04/H-05:** expose the same authenticated operations, readiness facts,
   and correlated causal evidence through public RRD surfaces. Traces observe
   the operation; they never complete it.
5. **H-07:** add optional configured mesh carriage/resolution without changing
   identity, authorization, endpoint presentation, or operator authority.
6. **J-01/J-02:** retain the already-removed initializer/bootstrap paths as
   negative absence checks; remove the old package, phase schema, forced apply,
   and old successful controller fixtures; pass controller restart,
   effect-gap, field-conflict, deletion, security, resource, and failure
   matrices.
7. **J-03/J-05:** install from the signed offline-verifiable distribution into
   a clean cluster, run the complete engine corpus, reopen it, and verify
   multi-architecture images, SBOMs, signatures, and retained data.

The target package is planned as:

```text
crates/adapters/rrflow-kubernetes/
├── Cargo.toml
├── src/
│   ├── lib.rs              # outward adapter API only
│   ├── main.rs             # controller process composition
│   ├── api.rs              # closed Kubernetes CRD projection/conditions
│   ├── plan.rs             # engine-issued plan validation and identities
│   ├── render.rs           # deterministic owned-resource rendering
│   ├── reconcile.rs        # watch/relist/idempotent effect execution
│   ├── readiness.rs        # bounded application readiness observation
│   ├── status.rs           # standard Kubernetes observation projection
│   ├── finalize.rs         # engine-gated retention/deletion effects
│   └── bin/rrflow-kubernetes-crd.rs
└── tests/
    ├── contract.rs         # closed schema/rendering golden corpus
    ├── api_server.rs       # admission/defaulting/ownership/status behavior
    ├── install.rs          # preview/apply and cold-start/reopen behavior
    ├── reconcile.rs        # drift/relist/restart/conflict/idempotency
    ├── effect_gap.rs       # prepared/effect/receipt crash boundaries
    ├── engine_conformance.rs # installed graph/index/recall/DataFusion flow
    ├── deletion_recovery.rs  # retention/finalizer/backup/restore behavior
    ├── resource_limits.rs    # workload and engine resource enforcement
    └── security.rs         # RBAC, Secret, namespace, network, Pod security
```

These files are destinations, not evidence that the design exists. A-07 may
combine an internal module only if the same single responsibility and complete
test seam remain explicit in its reviewed package map.

## Acceptance evidence

| Proof family | Required corpus |
|---|---|
| Pure contract | closed decoding, bounds, digest/signature/fence/idempotency, deterministic rendering, old-shape rejection, no secrets or arbitrary commands/paths |
| Kubernetes API | real supported API server (for example kind), CRD admission/defaulting/status, SSA ownership/conflict, watch/relist, every owned-resource event, owner GC, controller restart |
| Installation | network-denied clean cluster; preview writes nothing; explicit apply initializes one engram once; kill/restart at every bootstrap/job/receipt boundary; close/reopen matches all identities/digests |
| Single-engine semantics | through the installed endpoint, commit and reopen representative documents, temporal graph edges, scalar/unique indexes, BM25, vectors/HNSW exact rerank, RRF, reasoning tree/context/evidence, and a stamped streamed Arrow/DataFusion query |
| Differential and durability | the same non-durability semantic corpus matches embedded/server baselines; Kubernetes rrflowKV alone proves WAL/crash/reopen, writer exclusion, PVC reattach, compaction/pinned-reader safety, and ENOSPC behavior |
| Readiness and updates | wrong engram/instance/image/config/profile/capability fails; socket-only false positive fails; update is prepared/fenced, conflict-safe, restartable, and never creates two writers |
| Deletion and recovery | retained default, backup/hold gating, explicit destruction separation, finalizer restart/stuck-delete/lost-ack convergence, foreign-resource denial, restore and reopened semantic verification |
| Security and networking | namespace-scoped least privilege, no controller Secret read/list/watch, no data-plane API token, restricted Pod admission, mTLS/credential rotation, exact NetworkPolicy allow/deny, mesh-independent authorization |
| Resources and release | CPU/memory/storage/diagnostic bounds, cancellation/timeouts, sustained operation, signed digest-pinned multi-architecture image, SBOM, clean offline install, deterministic checked manifests |

Passing `cargo check`, local JSON golden tests, a running Pod, a
`readyReplicas` value of one, or one query is characterization only.
Kubernetes support is claimable only when the installed process executes and
reopens the same complete RRFlow reasoning/recall system under these failure
and resource conditions.

## Primary Kubernetes contracts

This target follows the current upstream contracts rather than treating
framework defaults as RRFlow design authority:

- [Custom Resources](https://kubernetes.io/docs/concepts/extend-kubernetes/api-extension/custom-resources/): `.spec`/`.status`, status subresources, controller behavior, and the warning against storing application data in the Kubernetes API;
- [Kubernetes API conventions](https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md): standard conditions and observed-generation semantics instead of a new phase state machine;
- [Server-Side Apply](https://kubernetes.io/docs/reference/using-api/server-side-apply/): managed field ownership and explicit conflicts;
- [Finalizers](https://kubernetes.io/docs/concepts/overview/working-with-objects/finalizers/): deletion remains pending until required cleanup conditions are complete;
- [StatefulSets](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/): stable identity/storage and explicit PVC retention behavior;
- [Disruptions](https://kubernetes.io/docs/concepts/workloads/pods/disruptions/): PDBs constrain only cooperating voluntary eviction and do not provide database availability or govern workload-controller updates;
- [Liveness, readiness, and startup probes](https://kubernetes.io/docs/concepts/workloads/pods/probes/): distinct health semantics and Service traffic gating;
- [RBAC good practices](https://kubernetes.io/docs/concepts/security/rbac-good-practices/): namespace scope and least privilege, including the sensitivity of Secret list/watch; and
- [Leases](https://kubernetes.io/docs/concepts/architecture/leases/): optional controller coordination, distinct from RRFlow effect fencing.

## Completion boundary

This reference is accepted architecture, not completed implementation. Current
Rust tests prove only local schema/rendering determinism, one sealed-plan init
container, installed-only server arguments, and absence of the removed caller
time/identity/credential/bootstrap inputs. No Kubernetes API server or real
controller/Pod/volume is exercised. The adapter remains unavailable as an
alpha-qualified deployment until its dependent C/D/H/J work removes the
parallel authority and the complete acceptance matrix above passes at one
recorded revision.
