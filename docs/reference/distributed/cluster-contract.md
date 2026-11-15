# RRFlow distributed cluster contract

**Status:** active target reference; `clustered_server` is unavailable and the current implementation does not conform
**Coordinate:** `rrflow://rrflow-instance/data/reference/distributed/cluster-contract`
**Owner:** distributed consistency, consensus, placement, replica recovery, and qualification for one installed RRD instance

RRFlow does not become a federation of databases when it is distributed. One
installed project, one engram/rrflowDB, one RRD instance, one logical
`RrdEngine`, and one operation catalogue remain the authority. A cluster is
only a physical execution and fault-tolerance form of that instance.

The [instance-topology architecture](../../architecture/instance-topology.md)
owns logical and physical identities. The
[engine data-flow architecture](../../architecture/engine-data-flow.md) owns
semantic writes, rrflowKV persistence, native access paths, Arrow/DataFusion,
reasoning, and context flow. The
[deployment-profile reference](../deployment/modes.md) owns valid combinations,
and the [roadmap](../../roadmap/rrflow-1.0.md) alone can schedule implementation
or accept evidence. This record specifies the distributed contract and the
disposition of current cluster code; it does not complete a roadmap gate.

## Availability boundary

The first usable alpha is an authenticated `single_node_server` over rrflowKV.
It may expose a configured network endpoint carried by a mesh, so laptops,
phones, Connectome, and other clients do not require a database cluster to use
the same project engram.

`clustered_server` is a target value but is not an active capability, an alpha
exit requirement, or a supported installation choice. The current roadmap has
no distributed implementation gate. It must remain absent from installed and
runtime capability values until the roadmap owner adds a dependency-ordered
distributed gate and its independent-host evidence passes. Existing types,
feature flags, binaries, simulations, OpenRaft adapters, and one-host tests do
not grant that authority.

## Fixed system boundary

Only `RrdEngine` may authenticate an invocation, authorize every semantic
effect, capture the read/transaction stamp, select native or analytical work,
compile the mutation batch, or accept its commit receipt. The distributed
boundary is an injected execution port beneath that authority:

```text
public operation
  -> authenticate instance and principal
  -> resolve installed project/engram binding
  -> RrdEngine authorize + capture semantic read stamp
  -> parse/bind/plan and optional bounded compute
  -> RrdEngine compile one effect-complete commit proposal
  -> distributed submission port: route + order + replicate
  -> node-local engine apply port
  -> one rrflowKV semantic batch + node-local Raft state
  -> quorum-durable receipt
  -> RrdEngine result, impact, trace, and delivery
```

The cluster package may validate placement, choose a route, drive consensus,
transfer a snapshot closure, and report observations. It may not:

- expose a public raw mutation or `RuntimeCommit` endpoint;
- authenticate or authorize application work independently;
- open `RrflowKvStore`, `rrd_lsm::Database`, or an object store from a
  caller-selected path as its semantic authority;
- create a cluster-owned tenant, table, schema, project scope, event family,
  trace lane, job repository, or monolithic topology document;
- let a supervisor, TLS identity, mesh peer, node config, or filesystem marker
  manufacture the installed instance/engram binding;
- let DataFusion, an index builder, or an inference adapter commit state;
- use rrflowMX as replicated canonical state; or
- accept an earlier pre-release physical or wire shape through a compatibility
  reader.

Node-local consensus metadata is necessary but is not engram truth. Votes,
terms, uncommitted logs, peer progress, transport counters, and snapshot spool
files live in a bounded node-local domain. Committed records, graph adjacency,
indexes, reasoning state, audit, outbox, runtime log, and projection deltas live
in the sole rrflowKV semantic domain and are applied through the engine port.

## Identities and cardinality

All distributed identifiers must reuse or be derived from the installed
identity vocabulary frozen by A-07. Separate permissive string wrappers cannot
define another resource hierarchy.

| Term | Meaning | Required relationship |
|---|---|---|
| RRD instance | Stable installed runtime serving one project engram. | Exactly one logical instance is deployed by the cluster. |
| cluster | Consensus and placement domain for that instance. | Belongs to exactly one instance; never contains several engrams. |
| node | Authenticated process/host member. | Has one installed node identity, placement attributes, endpoint set, and credential reference. |
| shard | Routing and consensus partition of instance state. | Belongs to one instance and one current placement epoch. |
| replica | One shard copy placed on a node. | Has one role and one observed applied state at a time. |
| voter | Replica participating in quorum decisions. | Voter membership is odd, unique, and policy-valid across failure domains. |
| learner | Non-voting catch-up replica. | Cannot become a voter or serve an eligible read until exact catch-up evidence passes. |
| placement epoch | Monotonic binding of a shard to ordered replica identities and roles. | Only the exact successor can activate; commands at another epoch fail. |
| shard read stamp | Term, applied/commit index, placement epoch, and state digest for one shard. | Equal indexes with unequal digests are corruption/conflict, not equivalence. |
| snapshot vector | Sorted map from participating shard identity to exact shard read stamp. | Retains partial order; no synthetic global cluster cursor is invented. |

Tenant, namespace, database, and logical-table identifiers are not implicit
cluster parents. If a later semantic schema introduces them, they remain
engram data and policy scope. Routing is derived from the installed engram,
canonical record family, and declared partition key—not from a second cluster
catalogue.

## Installed composition and startup

Installation preview must enumerate the exact nodes, endpoints, placement
policy, storage roots, credential references, resource limits, and process
effects without writing or contacting them. Apply commits the cluster
descriptor and desired placement through `RrdEngine` only after the ordinary
project/engram/instance binding exists. Startup resolves that descriptor
read-only; it never synthesizes a cluster from a config file.

Every node must prove before readiness:

1. its installed instance, engram, cluster, and node identities match;
2. its admitted rrflowKV root belongs to that exact replica and contains the
   one accepted physical format;
3. its endpoint and workload identity match the installed descriptor;
4. its configuration, policy, schema, and credential revisions are supported;
5. its node-local consensus domain cannot collide with semantic rrflowKV keys;
6. its resource/admission policy is valid and bounded; and
7. no missing state is silently initialized or repaired.

Wardenclyffe/Zuul Zero, shippin.ai, or another mesh may resolve and carry a
configured endpoint. Mesh membership is neither node admission nor application
authorization. Mutual identity, capability negotiation, and the installed
binding are evaluated independently after reachability exists.

## Replicated write contract

The replicated command is an engine-compiled proposal, not arbitrary serialized
application mutations. Its frozen representation must bind at least:

- protocol and command-format identity;
- instance, engram, cluster, shard, and placement epoch;
- invocation/idempotency identity and authenticated principal evidence;
- operation, effect-complete authorization decision, policy revision, and
  resource scope;
- semantic read stamp plus expected schema/catalog/security/projection
  revisions;
- deterministic mutation-batch digest, object/projection closure, and budgets;
- required current/temporal records, both graph directions, scalar/unique and
  BM25 changes, canonical vectors, projection deltas, runtime-log, audit,
  outbox, and cursor effects; and
- expected prior shard state where conflict detection requires it.

The leader accepts only a proposal produced through its local `RrdEngine`.
Consensus orders that exact proposal. Every replica then invokes the same
engine-owned deterministic apply port, which revalidates identity, placement,
format, revision, digest, and conflict preconditions before producing one
rrflowKV write batch. Semantic data and the applied consensus pointer must
become durable in one failure-atomic local publication; the node-local vote and
uncommitted log are not copied into the semantic snapshot.

An accepted response is returned only after the proposal is durably committed
by the configured voter quorum and the leader has applied the ordered entry
through the engine port. Followers apply the same committed entry
deterministically; acknowledgement does not wait for every follower state
machine. A semantic conflict becomes the replicated typed result and produces
no partial semantic mutation. Reusing an idempotency identity with different
coordinates or bytes fails closed. Losing the client response after quorum
commit is recovered by the canonical commit identity, not by replaying a second
mutation.

Cross-shard mutation remains denied until a separately specified and tested
atomic protocol owns durable intents, conflict handling, recovery,
idempotency, and audit. The metadata shard cannot be treated as a shortcut for
cross-shard data atomicity.

## Read consistency and stamps

Supported read intent is explicit:

| Intent | Minimum behavior |
|---|---|
| `linearizable` | Establish a quorum-confirmed read barrier for the current leader and read no earlier applied state; deny when quorum or leadership cannot be proved. |
| `bounded_stale` | Select only an active eligible replica whose lag satisfies the requested index/time bound and whose placement epoch is current; return observed lag and stamp. |
| `exact_snapshot` | Select only a replica matching the complete requested shard stamp, including digest; otherwise deny rather than approximate. |

Every routing result records requested consistency, candidate/selected replica
identities, health observations, placement epoch, observed stamps, allow/deny
reason, and resource cost. Caller-supplied health maps are test inputs, not a
production authority; the installed control plane must produce authenticated,
fresh observations.

A multi-shard read captures a `SnapshotVector` and carries it through native
graph/BM25/vector work and rrflowQL/DataFusion. The planner may compare vectors
only by their real partial order. It must not collapse them into the existing
process-global runtime cursor or report a transactionally consistent cut when
the protocol did not create one.

## Native indexes, Arrow, and DataFusion

Distribution does not create an analytics sidecar. The ordinary engine rules
still apply:

- current/temporal records, both graph adjacency directions, scalar/unique
  keys, incremental BM25 postings, canonical vectors, projection deltas,
  runtime log, audit, and outbox cross the replicated semantic commit together;
- HNSW, TurboQuant, and other heavy search structures are source-stamped
  projections with exact fallback/rerank, never independent truth;
- Arrow-compatible rrflowKV pages and index artifacts bind the shard snapshot,
  schema/configuration, source coverage, and content digest;
- rrflowQL plans at one captured vector and uses bounded local/remote access
  paths selected by cost and authorization;
- DataFusion consumes stamped bounded Arrow batches and returns a result or
  proposal to `RrdEngine`; it does not perform consensus or write to a replica;
  and
- physical evidence reports network, key/page, decoded, copied, allocated,
  cached, spilled, and output bytes instead of labeling mmap as universal
  zero-copy.

A distributed conformance corpus must include the complete graph, BM25,
vector, RRF, reasoning-tree, context, and DataFusion operations that qualify
single-node rrflowKV. Replicating one schema record, opaque probe, JSON trace,
or vector object reference is not engine conformance.

## Membership, placement, and resharding

Placement policy declares voter/learner counts, allowed failure domains,
capacity, and acknowledged fault tolerance. Replicas are uniquely and
deterministically ordered. No node may satisfy two required failure domains by
renaming itself.

Membership and placement changes follow a fenced sequence:

1. `RrdEngine` commits an authorized desired transition and exact successor
   placement epoch.
2. A new learner receives an authenticated snapshot plus contiguous ordered
   log tail and complete referenced artifact closure.
3. Its read stamp and object/projection coverage are compared with the leader's
   committed cut; a membership API response alone is not catch-up evidence.
4. Consensus performs joint-to-uniform voter change where required.
5. The engine commits observed membership and activates the new routing epoch.
6. Old replicas retire only after the retention, recovery, and rollback bounds
   are satisfied.

Resharding additionally binds the source placement/vector, target ranges,
snapshot cut, copied state, ordered delta, routing revision, and final cutover
vector. Missing or unequal source stamps, skipped epochs, overlaps, gaps, or
unreconciled writes deny activation. Until this sequence has failure and
independent-host evidence, metadata-shard resharding and dynamic discovery are
unavailable.

## Snapshot and immutable-artifact recovery

A replica snapshot is issued from an engine-owned pinned snapshot handle. It
contains exactly one shard's accepted rrflowKV semantic closure at an exact
stamp. Node-local votes, peer addresses, TLS material, supervisor state, and
uncommitted logs are excluded. Only the current 1.0 rrflowKV physical reader is
accepted; prior bundle, segment, adapter-domain, or missing-field shapes are
negative rejection fixtures.

Large immutable objects and derived index artifacts may travel outside the
consensus log, but activation is bound to a versioned manifest containing the
installed identities, shard and placement, source/target, semantic snapshot,
read/projection stamps, sorted object identities, lengths, media/configuration
identities, and content digests. Recovery performs:

```text
authenticate source and target
  -> validate snapshot and manifest identity
  -> reserve bounded session resources
  -> resume exact-offset chunks
  -> verify every length and digest
  -> prove complete snapshot object/projection closure
  -> install semantic snapshot
  -> replay contiguous ordered log tail
  -> compare final stamp
  -> commit engine-owned completion receipt
  -> admit reads/membership
```

Transfer staging can use bounded fsynced node-local files, but those files,
their names, and their existence are never completion authority. Durable job,
lease, attempt, checkpoint, observation, and receipt state is accepted through
`RrdEngine`. Garbage collection validates containment, refuses symlink or
ambiguous entries, respects pins/retention, and removes only unreachable
staging metadata or content.

## Security, transport, and evidence

Internal consensus and transfer protocols are distinct from the public RRD
operation protocol. Their envelopes bind version, installed identities, source
and target, shard/epoch, operation kind, request digest, size, deadline, and
causal trace coordinates. Frames, connections, in-flight work, per-identity
rates, session counts, reserved bytes, retries, and durations are bounded before
large allocation or effect.

The existing TLS 1.3 mutual-certificate, exact URI identity, CA overlap,
rotation, revocation, frame-bound, and identity-admission behaviors are useful
characterization. The target obtains credential material from an installed
provider reference and can accept a Workload API adapter; node configuration
does not own credential issuance or authorization. A raw inherited
stdin/stdout command capable of submitting application commits is not a release
administration surface.

Durable traces use the one A-07/H-05 vocabulary and flow through authorized
engine operations. They correlate ingress, proposal, routing, consensus,
replica apply, snapshot/artifact transfer, acknowledgement, and delivery while
excluding credentials, payload bodies, prompts, embeddings, and object bytes.
Process-local counters report attempted/allowed/denied/failed work, saturation,
latency, bytes, concurrency, health, and reset identity; they observe state and
cannot advance it. The current `cluster.*` trace names and direct-store trace
adapter are inventory, not accepted names or authority.

## Current implementation disposition

The current `rrd-cluster` code is not renamed into compliance. Each module has
an explicit preserve/replace boundary:

| Current module | Preserve with equal-or-stronger evidence | Replace directly before availability |
|---|---|---|
| `contract.rs` | Placement epochs, odd voter/quorum and zone validation, explicit read modes, shard stamps, partial-order vectors, route evidence, transfer bounds, reshard cutover, and cross-shard denial. | Loose parallel cluster/node/zone/region/tenant/table IDs, free-form scope, `enforce_m7`, and permissive/defaulted serialized shapes. |
| `authority.rs` | Deterministic placement and capacity/failure-domain validation. | `DistributedAuthorityCatalogue`, its cluster-owned tenant/table hierarchy, one 4 MiB JSON topology record, self-installed schema, private scope, and direct-store commits. Placement becomes typed engine-owned records/relations and pure proposals. |
| `openraft_adapter.rs` | OpenRaft storage conformance, monotonic votes, log-hole rejection, shard binding, idempotency, membership/epoch checks, bounded file snapshots, corruption/stale denial, and separation of node-local Raft metadata. | Direct `rrd_lsm`/`rrd_store` semantic application, raw probe/runtime commands, defaulted old state, pre-1.0 snapshot readers, hardcoded private key domains, and arbitrary-root openers. Inject the engine apply/snapshot ports. |
| `transport.rs` | Mutual identity, envelope/digest/source binding, revocation/rotation, bounded frames/deadlines/admission, and resumable transfer mechanics. | Raw `RuntimeCommit` RPC, project-scope-only authorization, direct state-machine/object-store access, and identity derived outside installation. |
| `node_runtime.rs` and `rrd-cluster-node` | Bounded config/control decoding, correlated status, readiness after validation, wait-at-least behavior, storage release, and controlled shutdown. | Caller-authored roots/cert paths/project scopes, independent initialization/membership authority, raw probe/runtime-commit control, fabricated actors, and private config/control version lineage. Startup consumes the installed descriptor and immutable effect plans only. |
| `artifact_transfer.rs` | Digest/length closure, exact-offset resume, idempotent receipts, peer binding, chunk/session/quota bounds, safe concurrency, containment, and stale/receipt GC behavior. | Cursor-zero semantic replay, direct `StorageEngine`, and `transfer-sessions-v1` JSON/marker state as durable authority. Use engine-issued closure plus engine job/effect records; leave only bounded staging local. |
| `artifact_trace.rs` and `rrd-engine/runtime/cluster_transfer.rs` | Causal prepared/progress/completed/failed evidence, privacy bounds, and receipt/resource attributes. | `cluster.*` names, synthetic snapshot identities, direct-store trace commits, and effect execution outside an authorized engine operation. |
| `telemetry.rs` | Saturating/reset-explicit, content-free global/per-identity operation and resource counters. | Package-local coordinate vocabulary and any interpretation of counters as durable health, completion, or lifecycle state. |
| `sim.rs` | Replayable explicit partition/heal/delay/duplicate/reorder/crash/restart/clock/disk events and acknowledged-copy safety checks. | Single-term/single-shard coverage presented as model checking of real elections, reconfiguration, storage, network, or engine semantics. Extend against the final state machine. |
| `rrd-engine/engine/distributed.rs` | A composition point for pure placement preparation and read routing. | Private `instance:*` scope, whole-log catalogue snapshot, untrusted caller observation authority, and preparing a commit that tests later publish by opening the store directly. |

No listed replacement authorizes deletion before its preserved behaviors have
focused acceptance at the canonical destination.

## Review-baseline evidence

At clean baseline `03ba77c` on 2026-09-08, the complete package and test files
were read before this classification. The current tests demonstrate useful
mechanics, but the strongest suite is not green:

- `cargo test -p rrd-cluster --all-features --all-targets --locked` passed 12
  unit tests, nine artifact-transfer tests, ten contract tests, three
  distributed-catalogue tests, and two small simulator schedule tests before
  failing in `openraft_cluster`; the other all-target binaries were not reached.
- The isolated
  `real_consensus_replicates_canonical_runtime_truth_to_every_voter` test fails
  at `openraft_cluster.rs:424` because state-machine apply returns
  `unsupported rrflowKV application format None`.
- The isolated
  `real_consensus_elects_fails_over_installs_snapshot_and_changes_membership`
  test did not terminate within 120 seconds and was stopped; no passing result
  is claimed.

Even if every current test passed, the corpus would still not prove installed
identity, effect-complete authorization, current-format engine apply, native
graph/scalar/BM25/vector behavior, Arrow/DataFusion at a snapshot vector,
persisted reasoning/context, resource exhaustion, multi-shard recovery, or
independent hosts. The [POA&M](../../poam/rrflow-1.0-alpha.md) owns this observed
deficiency; the execution map owns exact file/test disposition.

## Qualification boundary

Before a roadmap gate may advertise `clustered_server`, all of the following
must pass at one revision:

1. repository and dependency checks prove no cluster store opener, semantic
   repository, raw mutation ingress, old-shape reader, private topology/job
   authority, or alternate trace/event lane remains;
2. a clean offline install creates one cluster for one installed engram and
   every node resolves the same binding, capability digest, and security
   authority;
3. the complete single-node rrflowKV transaction, temporal graph, scalar,
   BM25, vector/HNSW/exact-rerank, RRF, reasoning/context, Arrow/DataFusion,
   audit, outbox, subscription, and public-surface corpus runs unchanged through
   cluster routing;
4. accepted writes survive leader/follower crash, partition, delay, duplicate,
   reorder, torn/corrupt I/O, disk loss/full, snapshot interruption, artifact
   loss/corruption, restart, and lost acknowledgements within the declared
   fault tolerance;
5. linearizable, bounded-stale, exact-snapshot, vector-cut, membership,
   learner, snapshot, purge, reshard, rollback, and cross-shard-denial or atomic
   protocol properties pass deterministic model, property, and real-process
   tests;
6. at least three independent hosts and declared failure domains pass sustained
   workload, rolling credential/configuration change, node replacement,
   rebalancing, recovery, and network/disk fault campaigns;
7. all network, storage, memory, cache, spill, compaction, transfer, retry,
   backpressure, and admission limits have measured denial/recovery evidence;
   and
8. HTTP, WebSocket, every SDK, CLI, MCP, LFG, and Connectome return the same
   authorization, result/denial, semantic stamp/vector, digest, and causal
   evidence as the embedded/single-node oracle.

Passing compilation, OpenRaft's storage suite, a loopback quorum, an opaque
probe, replicated JSON, a copied vector artifact, or a UI cluster lens cannot
substitute for this matrix.
