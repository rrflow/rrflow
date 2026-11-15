# RRFlow engram control

**Status:** active implementation reference; direct convergence into the canonical engine transaction path remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/operations/engram-control`
**Owner:** desired/observed engram operations and fenced external-effect reconciliation

An RRFlow engram is one per-project AI governance, reasoning, recall, and
context boundary. Engram control manages that engram's installed RRFlow
instances and explicitly configured deployment, backup, and recovery effects.
The [local-process adapter](../deployment/local-process-driver.md) defines the
host boundary for one installed RRD process, and the
[Kubernetes adapter](../deployment/kubernetes-operator.md) defines its
Kubernetes projection/reconciliation boundary. Neither adapter's observations
nor receipts become engram state until accepted by `RrdEngine`.
It is not the reasoning-tree lifecycle, the routine executor, an application
database, or a second database control plane.

The [system overview](../../architecture/system-overview.md) owns the term
`engram` and the component boundaries. The
[engine data flow](../../architecture/engine-data-flow.md) owns the common
transaction, persistence, graph/index, Arrow/DataFusion, and context paths.
The [security authority](../security/authority.md) owns identity and policy.
This record preserves useful behavior in `rrd-engram`, exposes where the
current implementation violates those owners, and specifies the direct target
without treating pre-release shapes as a compatibility line.

## Authority boundary

`RrdEngine` is the only component allowed to authorize or commit an engram
operation. `rrd-engram` may define and validate transport-free desired,
observed, operation, lease, receipt, backup, and recovery values. It may not:

- accept or expose a physical `StorageEngine`;
- construct a repository that commits state independently;
- create an identity, policy, clock, audit, event, or transaction authority;
- represent project knowledge or reasoning work as deployment operations;
- infer completion from a process, trace, emitted event, client view, or
  missing error; or
- let a driver, controller, DataFusion plan, model, routine, or Connectome
  write authoritative engram state.

Engram control is one semantic operation family inside rrflowDB. “Authority”
means `RrdEngine` plus canonical RRFlow Security. The current
`EngramAuthorityState` name denotes an operational inventory catalogue and
must not be interpreted as a second security authority.

## Accepted single-engine flow

Every embedded, HTTP, WebSocket, SDK, MCP, CLI, Connectome, or future
mesh-reached request follows the same path:

```text
typed operation + installed instance/engram coordinate + credential reference
  -> adapter resolves one configured RrdEngine
  -> authenticate principal at engine-observed time
  -> capture policy/schema/catalogue revisions and one authorized read stamp
  -> authorize the operation and its complete semantic/external effect plan
  -> validate desired generation, idempotency, lease, and engram preconditions
  -> atomically commit canonical records + relations + indexes + operation
     intent/checkpoint + runtime log + audit + outbox + cursor
  -> for an external effect, a fenced worker acquires the committed operation
  -> atomically persist prepared intent before invoking the typed adapter
  -> adapter applies the stable operation identity and returns evidence
  -> atomically commit receipt, observation, indexes, audit, outbox, and cursor
  -> complete, retry, compensate, or fail from durable evidence
  -> return one typed result, stamp, digest, and correlated evidence
```

An external process, object-store, archive, or restore effect cannot be made
atomic with rrflowKV. RRFlow instead makes each semantic boundary atomic,
persists intent before the effect, fences workers by lease epoch, requires an
idempotent stable operation identity, and reconciles observed evidence after
the effect. A crash can repeat observation or an idempotent adapter call; it
cannot infer that an unrecorded effect succeeded.

## Canonical state families

The final C-01 key codec and C-03 transaction contract assign exact physical
tuples. Engram control requires these logical families rather than one opaque
aggregate:

| Family | Canonical role | Required relationships and access |
|---|---|---|
| Engram | per-project boundary, installed profile, activity policy, and current revision coordinates | point read by exact engram identity; relation to installed instances and policy/schema revisions |
| Managed instance | desired and observed generation, deployment reference, configuration digest, runtime evidence, and activity classification | exact instance read; scalar access by phase, generation, deployment, activity, and update time |
| Engram operation | requested semantic effect, target generation, state, attempts, error, and immutable request/effect digests | target-instance relation; deterministic open-operation ordering and idempotency lookup |
| Lease and checkpoint | current owner, epoch, expiry, prepared input, and current reconciliation boundary | expiry/runnable access; compare-and-swap fenced by operation revision and epoch |
| Receipt and observation | append-only boundary/effect evidence and observed external state | operation/evidence relations; ordered by operation and boundary without replaying the whole engram |
| Backup job and recovery point | quiesced source generation, archive identity, coverage, watermarks, policy snapshot, and completion evidence | instance/job/point relations; status, coverage, and retention access |
| Recovery policy, pin, restore evidence, and prune intent | RPO/RTO/retention policy, explicit holds, restore measurements, and deterministic retained/pruned partitions | exact policy and point reads; live-pin and prune-candidate access at an engine-observed time |
| Idempotency binding | operation identity plus canonical request digest and outcome coordinate | bounded exact lookup; conflicting reuse commits nothing |

Containment, targeting, evidence, backup coverage, recovery provenance, and
worker ownership are typed RRFlow relations. Their final relation labels and
ordered keys are frozen with C/E implementation; they are not hidden only in
nested JSON. Scalar indexes are maintained transactionally. BM25 and vector
indexes are used only for schema-authorized descriptive/evidence fields when a
context query warrants them; they are never required to determine control
state, ownership, ordering, or completion.

## Desired, observed, and activity semantics

Desired state expresses an authorized target, never proof that the target
exists. Each accepted change advances one monotonic generation and binds the
deployment identity, target version, configuration digest, request digest,
operation identity, actor, policy/schema coordinates, and transaction stamp.
A newer target supersedes unfinished work for an earlier generation before a
stale adapter can apply it.

Observed state is evidence about an external system. Its generation cannot be
ahead of desired state, and its timestamp and evidence digest are retained.
The currently useful phase vocabulary is:

- desired: `running`, `stopped`, and `absent`;
- observed: `unknown`, `provisioning`, `starting`, `running`, `stopping`,
  `stopped`, `deleting`, `absent`, and `failed`; and
- deployment operations: `provision`, `start`, `stop`, `restart`, `upgrade`,
  and `delete`.

Those names describe an installed RRFlow process or explicitly managed
deployment. They do not describe reasoning nodes, attunement phases, triggers,
skills, or generic routines.

Activity is a derived projection over persisted meaningful-runtime evidence
and a versioned engram policy. The current ordered classes `active`, `idle`,
`stale`, and `neglected` are useful only when their thresholds, last evidence,
and evaluation time accompany the result. Missing evidence yields `unknown`;
wall-clock absence cannot prove activity. Policy changes invalidate or
recompute the projection through an authorized engine operation.

## Reconciliation protocol

The current one-boundary-per-call discipline is retained as the generic
external-effect pattern:

1. Select the oldest eligible open operation deterministically.
2. Acquire or renew a bounded lease; each takeover strictly increases its
   epoch and makes stale workers unable to publish.
3. Commit `prepared` with the exact adapter input/effect digest before calling
   the adapter.
4. Invoke the typed adapter with the stable operation identity. Classify a
   returned error as retryable or permanent and retain bounded evidence.
5. Commit the applied-effect receipt only if the operation revision, owner,
   epoch, generation, and prepared digest still match.
6. Observe independently, retain its evidence, and never manufacture an
   observation from the requested target.
7. Complete only when the accepted observed state proves the target; otherwise
   retry, wait, compensate, or fail explicitly.

Receipts are append-only and bind operation, boundary, lease epoch, evidence
digest, and time. Restart reconstructs progress from canonical operation and
receipt records, not from a trace or a controller's memory. Lost
acknowledgements converge because the adapter sees the same stable identity.

Backup reconciliation uses the same pattern and admits a backup only from an
observably quiesced source generation. Recovery retains exact point identity,
coverage, holds, deterministic retention decisions, restore closure, and
measured RPO/RTO evidence. Filesystem paths, process IDs, archive handles, and
provider-specific locators remain adapter evidence or installed
configuration; they are not portable engram identities.

## rrflowMX, rrflowDB, rrflowKV, and rrflowQL

`RrdEngine` defines one engram semantics over two storage profiles:

- **rrflowMX** executes the same non-durability-specific validation,
  transaction, relation, index, idempotency, and query corpus in volatile
  memory. It does not silently promote into rrflowDB. An operation that
  requires restart durability, backup, or recovery must return an explicit
  unsupported-capability outcome on this profile.
- **rrflowDB backed by rrflowKV** persists the canonical engram records,
  versions, relations, indexes, checkpoints, receipts, audit, outbox, and
  cursors through WAL/MVCC/LSM recovery. It alone may claim crash/reopen
  continuity.
- **rrflowQL** plans point, range, scalar-index, and bounded graph work on the
  native fast path. Engram state navigation does not require DataFusion.
- **Arrow/DataFusion** receives read-stamped, projected batches for broad fleet
  or historical analysis such as phase distributions, recovery posture,
  activity trends, or failure evidence. DataFusion computes and returns a
  bounded proposal/result; it cannot acquire a lease, authorize an effect, or
  write engram state.

Analytical batches and native reads must resolve the same authorized stamp.
Every selected or skipped native/index/DataFusion avenue records its reason
and physical work. A DataFusion result that should change desired state,
policy, or scheduling returns through a separately authorized `RrdEngine`
mutation.

## Installation and attunement boundary

`rrflow install` creates or binds exactly one project engram from a previewed,
digest-bound plan. It establishes the engine location, storage profile,
security authority, initial principal and grants, project root, configuration,
and generic attunement profile before ordinary engram administration is
available. Adapters may resolve those installed coordinates; callers may not
choose arbitrary database, policy, key, state-root, catalogue, or
authorization-time values per operation.

Attunement inventories and derives project knowledge inside the installed
engram. Its phase jobs use the generic routine/checkpoint transaction pattern,
but they are not deployment operations. PostgreSQL, Turso, SQLite, Dragonfly,
object stores, model providers, generators, harnesses, and mesh endpoints are
operator-configured integrations discovered or proposed during attunement.
They never replace rrflowKV, rrflowQL, RRFlow Security, or engram authority,
and discovery never activates them without policy and explicit consent.

## Public and Connectome boundary

The implemented public operation is the session-authenticated
`POST /v1/engrams/{engram}/read`, returning the bounded
`rrd-contract::EngramSnapshot`. It enforces exact instance/engram resource
matching and omits raw idempotency keys. Runtime capability discovery marks
engram read experimental; no public engram mutation operation is catalogued.

Connectome must use the same public health, capability, session, engram,
query, trace, and subscription operations as every other client. It may
visualize desired versus observed state, operation boundaries, graph
relationships, index-backed views, and correlated evidence. It cannot create
synthetic authoritative rows, map local UI state to completion, or call a
private `/api/engram` path and claim cross-surface conformance. H-06 owns proof
against the separate Connectome repository.

## Current implementation foundation

The current code has substantial useful behavior, but its persistence and
composition are not accepted as the target:

| Current construct | Observed behavior | Target disposition |
|---|---|---|
| `EngramDocument` | serializes the entire engram, operational inventory, instances, operations, idempotency, backup, and recovery maps as one JSON value | Split into typed canonical record/relation/index families committed through the semantic transaction port. No successful missing-field shape remains. |
| `EngramRepository<'a, E: StorageEngine>` | publicly accepts either store and directly loads, mutates, validates, serializes, and commits engram state | Remove the public storage dependency. Retain pure domain validation; `RrdEngine` owns reads, authorization, transaction construction, and commit. |
| `server/state/engram/{engram}/document` | one compare-and-swap control key contains the whole aggregate | Replace with exact bounded keys and native access paths; do not preserve this key as a forwarding or compatibility record. |
| `ControlTransition`/`ControlJournalEntry` | atomically replaces the materialized control value and appends a hash-chained journal entry containing the full replacement | Preserve CAS and tamper-evident lineage in the canonical transaction/log, but eliminate full-engram duplication and include semantic state, indexes, audit, outbox, and cursor in one commit. |
| `ManagedInstance` plus `EngramOperation` | implements desired/observed generations, supersession, leases, receipts, observations, activity, and idempotency | Preserve and strengthen these semantics as typed records and relations under one stamp. |
| `EngramAuthorityState` | nests twelve organisation/account/entitlement/project/environment/instance/node/shard/job/assignment/health/secret-reference resource kinds, desired/observed state, receipts, and another idempotency map inside the engram document | Do not retain a generic “authority” catalogue beside security and managed instances. The accepted [instance topology](../../architecture/instance-topology.md) maps each useful resource/relationship to installation, topology, cluster, job, health, or secret-reference owners and requires the overlap to be removed. |
| `Reconciler<E, D>` and `BackupReconciler<E, D>` | directly construct `EngramRepository`, fabricate package-local mutation context from worker/time, and advance one durable boundary | Preserve deterministic one-boundary recovery and adapter traits, but execute each boundary as an authenticated, stamped engine operation with canonical event/evidence output. |
| Backup/recovery methods | enforce quiescence, leases, policy snapshots, holds, deterministic prune partitions, restore evidence, and bounded collections | Preserve safety invariants in separate typed families; remove old-shape defaults and bind all state/effects to canonical policy and transaction stamps. |
| Static `RrdEngine::*_store` methods | accept raw physical paths, caller time, worker, lease, and catalogue inputs; open a new local authority and append split authorized/completed audit records | Replace with operations on an installed engine and canonical invocation. The local authorization reference owns the exact permission convergence. |
| Controller binaries | expose one-step local reconciliation and test-only hold files around effect boundaries | Retain a thin installed adapter only if required; remove arbitrary-path authority and debug hold behavior from release surfaces. Failure injection belongs in test-only ports. |
| `EngramSnapshot` and engram-read endpoint | provide a strict bounded read-only projection with session authorization and resource matching | Retain the semantic projection, add a single stamped source coordinate and shared surface conformance, and derive it through native bounded reads. |

The aggregate cardinality constants do not establish a scalable layout.
`ControlTransition` caps each value at one MiB, while every mutation rewrites
the whole JSON aggregate and copies the replacement into its journal entry.
The advertised instance/operation/authority/backup/recovery maxima can
therefore become unreachable based on encoded byte size, and transition cost
grows with unrelated engram state. No focused resource or write-amplification
evidence currently bounds that behavior.

## Required capability disposition

No current implementation is deleted until every preserve/generalize row has
equal-or-stronger evidence at its canonical destination.

| Current capability | Disposition | Canonical destination and required proof |
|---|---|---|
| Strict identifiers, bounded strings/collections, digest validation, monotonic revisions, and timestamp ordering | Preserve | Pure engram domain validation plus contract/property corpora; invalid input commits nothing on rrflowMX and rrflowKV. |
| Monotonic desired generations and supersession of older unfinished work | Preserve | Engine transaction tests prove one atomic generation/operation/idempotency change and reject stale adapters after reopen. |
| Desired/observed separation and observed-not-ahead invariant | Preserve | Shared MX/KV semantic corpus and driver oracle prove observations cannot be fabricated or advance past desired state. |
| Stable request digest, operation identity, and conflicting idempotency rejection | Preserve | Exact native idempotency index and cross-surface corpus return the same result or conflict at one stamp. |
| Lease expiry, increasing epoch, owner fencing, deterministic oldest-open selection | Preserve | Concurrent worker, clock-boundary, takeover, crash/reopen, and stale-receipt tests under engine-observed time. |
| Prepared-before-effect, append-only receipts, lost-ack replay, retryable/permanent errors, and one-boundary stepping | Generalize | Generic engine external-effect reconciliation used by process, backup, recovery, attunement, and routines without creating a shared fixed lifecycle enum. |
| Activity policy and evidence-based classification | Generalize | Versioned engram policy plus derived projection; policy-change, absent-evidence, boundary-time, close/reopen, and bounded-index tests. |
| Quiesced backup admission and source-generation/policy binding | Preserve | Durable rrflowKV backup job/point records and effect-gap crash corpus; rrflowMX returns explicit unsupported capability. |
| Recovery policies, holds, prune intent, restore closure, and measured RPO/RTO | Preserve | Authenticated identity-based operations, catalogue/engram revision fencing, filesystem/object failure injection, reopen, tamper, and retention oracle tests. |
| Typed `EngramDriver` and `EngramBackupDriver` effects | Generalize | Installed capability adapters receive immutable effect plans and stable operation identities; no driver receives a store, credential, policy, or authority handle. |
| Whole `EngramDocument` JSON value and full replacement in every journal record | Reject | Typed records/relations/index deltas and one canonical runtime/transaction lineage; repository searches and negative old-key/shape fixtures prove absence. |
| Direct `StorageEngine` repository and direct-store reconciler constructors | Reject | Pure domain crate plus authorized `RrdEngine` operations; dependency and public-API checks reject physical storage imports. |
| Broad operational `EngramAuthorityState` hierarchy | Split and reconcile | The accepted [instance topology](../../architecture/instance-topology.md) classifies every kind and relation into its one owner; security authority, managed instance, job, health, and secret-reference concepts cannot remain duplicated. |
| Caller-supplied clock, database, policy, key, state root, catalogue, and debug hold paths | Reject | Installed binding, engine clock, canonical credential/policy, typed adapter configuration, and test-only failure ports. |
| Current read projection and exact resource match | Preserve and extend | One native read stamp and identical HTTP/WebSocket/SDK/MCP/Connectome result/digest/evidence corpus. |

## Direct-convergence requirements

1. Complete A-07 tracing for every engram, backup, recovery, topology, local
   process, cluster, Kubernetes, server, client, and CLI caller before moving
   code.
2. Make `rrd-engram` a pure domain/validation boundary. Remove its
   `rrd-store` dependency, `EngramRepository`, and direct-store reconciler
   constructors without a wrapper or alias.
3. Reconcile `EngramAuthorityState` against the accepted
   [instance topology](../../architecture/instance-topology.md).
   Assign each useful resource and relationship to exactly one canonical
   record family; remove duplicate managed-instance, job, health, secret, and
   authorization concepts.
4. Freeze engram semantic keys and relations in C-01/C-03, including exact
   idempotency, runnable/lease, operation/instance, activity, backup, recovery,
   and evidence access. Atomically maintain required scalar and graph indexes.
5. Implement each engram mutation and reconciliation boundary as a canonical
   operation on an already resolved `RrdEngine`, using the authorization and
   invocation convergence specified by
   [local-engram-authorization](../security/local-engram-authorization.md).
6. Commit state, relation/index deltas, operation/checkpoint, runtime log,
   audit, outbox, and cursor in one rrflowMX/rrflowKV semantic transaction.
   Use durable intent and fenced receipts for effects outside that transaction.
7. Replace whole-aggregate loads with exact or bounded native reads. Prove
   write amplification, scanned/read/decoded/copied/allocated bytes, retained
   snapshots, cache effects, cancellation, and compaction safety.
8. Stream authorized engram history/analytics through rrflowQL as projected
   Arrow batches at the same `ReadStamp`; enforce pushdown and one query
   budget. DataFusion remains read/transform/propose only.
9. Remove every successful older-shape decode, old control key, arbitrary-path
   entry point, broad audit-only authorization, and test-only release behavior
   in its owning gate. Preserve old bytes only as negative fail-closed vectors.
10. Expose the accepted operations through the shared public catalogue and
    cross-surface corpus. Connectome consumes only those public operations.

## Current evidence and missing proof

| Evidence | What it proves now | What it does not prove |
|---|---|---|
| `rrd-engram --all-targets` | current validation, desired/observed, idempotency, activity, authority-catalogue, backup/recovery, selected MX/KV reopen, lease takeover, lost-ack, and local authorization behavior | accepted transaction/storage boundary; it also requires successful older document/job shapes |
| `rrd-store/tests/control_journal.rs` | MX and KV materialize the same generic CAS transitions; KV state and hash chain survive restart | engram semantic parity, native record/index access, bounded write amplification, or one state/index/audit/outbox transaction |
| `rrd-server/tests/local_engram_driver.rs` | a real RRD child starts/stops, preserves its data directory, rejects forged process identity, uses bounded kill fallback, and converges across controller/effect-gap process kills | canonical authorization, installed adapter resolution, MX parity, release qualification, or absence of direct repository access |
| `rrflow-cli/tests/backup_controller.rs` | current backup controller converges after an archive effect-gap kill | one canonical engine operation, object/filesystem failure coverage, restore/prune equivalence, or shared public surface |
| `rrd-engine/tests/engine_authority.rs` | current local engram/recovery work coexists with engine state and survives selected reopen paths | one authority or atomic transaction; the engram operation bypasses canonical grants and audit remains split |
| engram public-contract and HTTP tests | strict empty request, bounded snapshot shape, authentication, exact engram match, and a real 200 response | public mutations, native bounded reads, source stamp, external SDK execution, WebSocket/MCP/Connectome equivalence, or target engine completion |

The current passing tests are characterization evidence. They do not close
POAM-017 through POAM-020, including the engram persistence, duplicate
topology, and installed-authority deficiencies recorded by the POA&M. The Rust
SDK conformance test also returns without execution unless
an external manifest is supplied; its presence is not a normal-run proof.

## Implementation anchors and focused characterization

- Domain aggregate and direct repository:
  `crates/authority/rrd-engram/src/lib.rs`
- Operational inventory catalogue:
  `crates/authority/rrd-engram/src/authority.rs`
- Desired/observed and backup reconciliation:
  `crates/authority/rrd-engram/src/{reconcile,backup_reconcile}.rs`
- Backup and recovery state:
  `crates/authority/rrd-engram/src/{backup_job,recovery}.rs`
- Current static composition and recovery operations:
  `crates/authority/rrd-engine/src/engine/engram_control.rs`
- Generic control-value/journal persistence:
  `crates/persistence/rrd-store/src/control.rs`
- Current public read:
  `crates/authority/rrd-engine/src/engine/engram.rs` and
  `crates/transport/rrd-server/src/http/handlers/data.rs`
- Real child/effect crash characterization:
  `crates/transport/rrd-server/tests/local_engram_driver.rs`

Focused current-behavior commands are:

```text
cargo test -p rrd-engram --locked
cargo test -p rrd-store --test control_journal --locked
cargo test -p rrd-engine --test engine_authority --locked
cargo test -p rrd-server --test local_engram_driver --locked
cargo test -p rrd-server --test http_process \
  authenticated_engram_read_returns_the_public_snapshot_only --locked
cargo test -p rrflow-cli --test backup_controller --locked
```

## Acceptance

Engram control is implemented only when one installed `RrdEngine` performs the
same non-durability-specific semantic corpus on rrflowMX and rrflowKV; the
durable profile survives every operation/effect crash boundary; state,
relations, indexes, checkpoints, runtime log, audit, outbox, and cursor commit
atomically; native access is bounded and measured; engram analytics stream at
one stamp through Arrow/DataFusion; and every public client resolves the same
typed state, result, denial, digest, and evidence. A JSON aggregate, a direct
repository hidden behind `RrdEngine`, a passing compile, or the current tests
alone do not satisfy that result.
