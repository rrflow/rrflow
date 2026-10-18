# RRFlow installed lifecycle

**Status:** active target contract; canonical local-estate startup and retry slice implemented, operation and release qualification remain open
**Coordinate:** `rrflow://rrflow-instance/data/reference/deployment/installed-lifecycle`
**Owner:** default-distribution acquisition, project installation, readiness, verification, repair, restore, salvage, and removal semantics
**Reviewed:** 2026-09-15

This record defines the stable target behavior of an installed RRFlow 1.0
estate. The [alpha objective](../../objectives/rrflow-1.0-alpha.md) owns the
measurable result, the [roadmap](../../roadmap/rrflow-1.0.md) owns completion
order, and the [execution portal](../../roadmap/rrflow-1.0-execution-map.md)
routes to the active exact-path record, generated inventory, and linked
evidence. This document cannot mark any of them complete.

## Product boundary

The default release has one operator entry point:

- `rrflow` on Linux and macOS;
- `rrflow.exe` on Windows.

The executable contains or is manifest-bound to every first-party engine,
rrflowKV, rrflowMX, rrflowQL, Arrow/DataFusion, graph, lexical, vector,
TurboQuant, installation, verification, recovery, and default adapter asset
needed after bundle acquisition. Internal Rust crates remain explicit library
boundaries. Optional Kubernetes, mesh, provider, project-database, Connectome,
and host-tool integrations remain adapters and are not required for local
readiness.

The default alpha does not require an operator to discover or launch
`rrd-server`, `rrflow-mcp`, a security bootstrap binary, a supervisor, or a
controller. Their reusable behavior must be linked behind the primary command
or moved to an explicitly selected, manifest-accounted optional artifact.

## Two installations, two meanings

Do not conflate these boundaries:

1. **Distribution acquisition** obtains and verifies the platform archive,
   executable, manifest, checksums, signatures/attestations, SBOM, licences,
   schemas, templates, default profiles, and required runtime/model assets.
   It never creates project state.
2. **Project installation** runs `rrflow install` against an explicit project
   root and creates exactly one estate plus its initial authenticated RRD
   instance. It never downloads a missing prerequisite.

Every installed byte is either immutable distribution content, a previewed
RRFlow-owned project integration, canonical rrflowDB state, or a declared
operator-controlled output such as a credential sink or backup target.

## Canonical command tree

The target command vocabulary is:

```text
rrflow version
rrflow install plan   --project <root> --mode fresh|existing --profile <id> [--configuration <toml>]
rrflow install apply  --project <root> --plan <file> --expect <sha256>
rrflow install status --project <root>
rrflow attune status|resume|cancel ...
rrflow serve          --project <root> [endpoint options]
rrflow ready          --project <root> [endpoint options]
rrflow verify         --project <root> --level quick|full
rrflow repair plan    --project <root> --output <file>
rrflow repair apply   --project <root> --plan <file> --expect <sha256>
rrflow backup create|list|verify ...
rrflow restore plan|apply ...
rrflow salvage        --project <root> --output <absent-path>
rrflow uninstall plan --project <root>
rrflow uninstall apply --project <root> --plan <file> --expect <sha256>
rrflow query ...
rrflow context ...
rrflow mcp ...
```

Names may change only through the roadmap owner before implementation. The
semantics may not be weakened by aliases. `dev up`, `dev doctor`,
`rrd-server initialize`, and the standalone security initializer have been
removed rather than retained as alternate commands. Existing standalone
backup/recovery controller flows remain implementation inventory, not an
alternate supported lifecycle.

## Installed state and ownership

The project locator is `.rrflow/config.toml`. It contains only bounded,
non-secret coordinates needed to locate and authenticate the installed estate:
format and product identity; project, estate, and instance IDs;
project-relative rrflowDB, token-key, and operator-credential locations; and
the plan, profile, executable, installed-record, and active-configuration
digests. Canonical mutable state never lives in this file. In particular, it
contains no absolute host path, endpoint, plaintext secret, model-provider
setting, or UI preference.

The managed persistent container is `.rrflow/rrd/` in the project's writable
layer. Each install, repaired candidate, or restored candidate has a distinct
root below `.rrflow/rrd/roots/<storage-root-id>/`; the locator selects exactly
one active root and its publication receipt. An active root remains normally
mutable by rrflowKV. Repair and restore never reuse it as their output.
Publication fsyncs the candidate, then durably replaces the single locator
file with a platform-specific atomic file-replacement primitive and fsyncs the
parent where supported. Cross-device or otherwise non-atomic publication
fails before mutation. The engine-persisted installed record owns project,
estate, instance, storage-root, profile, specialization, bundle, schema,
policy, and plan identities. The locator and record must agree before open;
neither is synthesized on startup.

Existing-project integration obeys file ownership at action granularity:

- an absent `AGENTS.md` may be created from a selected bundle template;
- an existing instruction file is changed only through an exact previewed,
  bounded RRFlow-managed inclusion and explicit apply;
- provider-specific files contain forwarding behavior only and cannot copy the
  instruction, lifecycle, product, or memory authority;
- every create/edit records the before digest, after digest, ownership marker,
  permissions, and removal rule; and
- uninstall removes an RRFlow-owned file or managed region only when its
  current digest/marker still matches, otherwise it reports drift and leaves
  project-owned content untouched.

Application databases, generators, harnesses, CI, models, meshes, and clients
are discovered only as inactive candidates. Activation requires a separate
typed binding, exact configuration, permissions, budgets, and authorization.

## Lifecycle state machine

Planning has no installed state: a plan is an immutable external artifact.
The engine-owned project lifecycle is:

```text
ABSENT
  -> APPLYING
  -> INSTALLED
  -> ATTUNING
  -> READY

READY <-> DEGRADED
READY|DEGRADED -> QUIESCED -> READY
APPLYING|ATTUNING|QUIESCED -> RECOVERY_REQUIRED
INSTALLED|DEGRADED|RECOVERY_REQUIRED -> REPAIRING
REPAIRING -> INSTALLED|DEGRADED|QUARANTINED
INSTALLED|READY|DEGRADED|QUIESCED -> REMOVING -> RETAINED
RETAINED -> APPLYING
```

`QUARANTINED` is fail-closed and cannot serve or accept normal mutations.
`RETAINED` has no active service, credential, provider binding, or project-file
integration. Its locator permits only authenticated status, verify, backup,
restore, and an exact reinstallation plan over the retained estate. Because
the alpha has no purge operation, uninstall never claims that retained bytes
are `ABSENT`.

Attunement's own seven-state job machine remains nested beneath the installed
estate and does not redefine these states. Events and traces observe committed
state; they never manufacture a transition.

## Install plan

`rrflow install plan` is pure. It may read project metadata and bundle content,
but it does not create a database, generate/read a credential, write a file,
invoke a project command, contact a provider, bind a socket, or activate an
adapter. The plan is canonical serialized data whose SHA-256 covers:

- binary/bundle, template, specialization, attunement-profile, schema, runtime,
  model, and configuration identities;
- stable project/estate/instance identity, fresh/existing mode, and the exact
  project-inventory precondition digest without embedding an absolute host
  path;
- the full effective revisioned configuration, not an opaque or caller-supplied
  digest, plus independent deployment-form, storage-profile, and endpoint-
  presentation coordinates;
- every create, bounded edit, permission/ACL, directory, locator, state,
  principal, role, grant, seat, representation candidate, adapter candidate,
  credential generation/delivery, service, verification, and removal action;
- execution ordering, rollback or resume semantics, resource limits,
  authorization requirements, and expected receipts; and
- files intentionally preserved and capabilities intentionally inactive.

Repeated planning over unchanged inputs is byte-identical. A stale plan fails
before mutation.

## Install apply

`rrflow install apply` accepts only the named plan and expected digest. It:

1. resolves and revalidates the exact bundle and project preconditions;
2. acquires an exclusive create-new lease without following an unreviewed link
   or mount change;
3. creates a plan-addressed staging root and uses a create-new rrflowKV API;
4. commits the installed binding, initial seat, security authority, install
   checkpoint, audit, outbox, and commit evidence through `RrdEngine`;
5. performs credential delivery through a capability-scoped, create-new,
   idempotency-aware sink and records prepared/effect/observation/receipt state;
6. fsyncs the staged root and atomically publishes the minimal locator only
   after the committed instance is self-verifying;
7. acknowledges the exact receipt; and
8. creates the durable attunement job without treating an event as completion.

Every crash boundary resumes the same plan or fails closed. It cannot create a
second credential, policy, estate, instance, audit outcome, or project edit.
No listener becomes ready before authentication policy and the installed
binding are committed.

## Current pre-release slice and rollout

Packages `D01-02-canonical-estate-layout-and-configuration-v1` and
`D01-04-precanonical-authority-convergence-and-install-recovery-v5` implement
the current concrete startup slice without claiming the complete target above:

- `rrd-contract` owns `InstalledEstateIdentity`, three independent deployment
  coordinates, and a strict self-digesting `EstateConfiguration`;
- the bundled default configuration or one explicit regular, non-symlinked
  TOML file up to 64 KiB is read during planning, validated below hard compiled
  maxima, assigned revision 1, and sealed into the plan;
- apply never rereads that mutable input. It consumes the sealed value and
  creates, in declared order, `.rrflow/`, `.rrflow/rrd/`,
  `.rrflow/rrd/roots/`, `.rrflow/credentials/`, one create-new storage root,
  token key, credential document, and `.rrflow/config.toml`;
- the active configuration is committed inside rrflowDB at
  `server/state/<instance>/configuration/active` before locator publication;
- installed open validates locator, installed record, configuration revision
  and digest, then binds the same identity/configuration into `RrdEngine`;
- query, live-query, query-index, and context requests above the installed
  ceiling fail before their data operation; and
- server capabilities and generated OpenAPI/SDK types expose the effective
  configuration and exact deployment coordinates. Reasoning ceilings are
  visible, while `governed-reasoning-runner` remains honestly unavailable;
- before the first `.rrflow` directory exists, apply publishes one owner-only,
  plan-addressed recovery intent containing the frozen first-apply time and the
  exact generated token/credential bytes. Retry accepts it only when its own
  digest and the plan, target, project precondition, profile, executable, and
  configuration still match;
- apply creates or exactly verifies each declared directory, store, token,
  credential, one semantic installation commit, pending locator, and active
  locator. It classifies empty, committed, published, and complete states and
  removes only the exact pending locator and acknowledged intent it owns;
- deterministic test-only observation stops before or after each of the eight
  durable stages: intent, directories, store, token, credential, engine commit,
  locator publication, and intent acknowledgement. Retrying the same plan
  preserves first-apply time and leaves one token, credential, installed
  record, policy, audit/checkpoint result, attunement plan/job, and locator;
- malformed, symbolic, foreign, mismatched, partly committed, or missing-secret
  recovery state is preserved and rejected rather than guessed or overwritten;
  and
- primary CLI, internal server process, embedded MCP, checked six-language SDK
  conformance daemon, deployment catalogue, and Kubernetes rendering now use
  canonical installation/open. The pre-canonical manifest/private binding,
  raw product CLI, development supervisor, standalone security initializer,
  and server initializer are removed with no successful compatibility reader.

The implementation rejects any pre-existing `.rrflow` tree for a new install.
It does not migrate, merge, or delete a user's pre-canonical
`.rrflow/instance.toml` state; that collision remains a fail-closed diagnostic.
The tracked development manifest has been deleted because no source path reads
it. The source-built CLI obtains first-apply time from its host wall clock and
passes it into the engine; a versioned trusted-clock capability and its
rollback/skew/failure qualification remain open. Handle-relative race
hardening, complete native ACL qualification, configuration plan/apply,
attunement phase execution, full
verify/repair/restore/uninstall, service integration, release assembly, and
clean-machine qualification remain separate packages. The staged rationale and
exit proofs are in the
[canonical estate/configuration research](../../research/rrflow-canonical-estate-configuration-and-runtime-controls.md#rollout-sequence).

## Open, serve, and readiness

Normal open is `open_existing`, never create-or-open. It resolves the project
locator, validates containment and digests, acquires the appropriate process
lease, authenticates the CURRENT manifest closure, replays only complete
contiguous WAL frames, restores the MVCC view, reconciles durable engine jobs,
and refuses unsupported or corrupt state. A torn final frame is reported as a
repairable condition; it is not truncated by a read-only open.

`rrflow serve` composes the HTTP, WebSocket, and selected MCP presentation in
the primary process. An optional outward process adapter may supervise it, but
cannot initialize state or define readiness.

`rrflow ready` succeeds only when an authenticated challenge proves the exact
installed project/estate/instance, binary/bundle digest, policy revision,
storage profile, commit/read stamp, and required subsystem status. For alpha,
the readiness proof includes a bounded engine operation and persistence health.
A PID, port, TCP handshake, marker, JSON file, source path, or compile result is
not readiness.

## Verification levels

Verification has a dedicated read-only inspection API and never calls a path
that creates directories, lock files, databases, checkpoints, projections, or
repairs. The alpha `verify` command acquires an offline inspection lease after
the serving process has stopped. Live health is the separate authenticated
`ready` operation; it cannot substitute for byte-stable verification.

`quick` verifies:

- locator syntax, containment, permissions, and installed-record identity;
- distribution/binary/configuration/schema/profile identities;
- CURRENT pointer, manifest chain head, reachable file inventory, sizes, and
  authenticated checksums;
- WAL header plus contiguous complete-frame prefix without truncation;
- security authority, install/attunement state, audit head, and last commit;
- projection source cursor/generation identities; and
- no active conflicting writer for an offline-only check.

`full` additionally scans all reachable segment pages and value/object payloads,
checks ordered key/version and MVCC invariants, verifies semantic record/link
and unique-index invariants, compares graph directions, verifies BM25/vector
source cursors and artifacts, checks backup catalogues, and executes bounded
exact-oracle samples for approximate projections. Its report records bytes,
pages, records, indexes, elapsed work, limits, omissions, and the exact stamp.

## Recovery, repair, restore, and salvage

These operations are deliberately distinct:

| Operation | Mutates installed target | Authority | Permitted result |
|---|---:|---|---|
| Startup recovery | No repair; valid WAL replay only | normal engine open | exact acknowledged state or fail closed |
| Verify | No | read-only inspector | digest-bound report |
| Repair plan | No | engine coordinator over physical/semantic inspectors | digest-bound proposed actions |
| Repair apply | Publication only; never edits the live root in place | `RrdEngine` maintenance coordinator with exclusive lease | verified replacement candidate plus atomic publication receipt |
| Restore | Creates absent target | authenticated backup plus engine installer | verified replacement candidate |
| Salvage | Writes separate output only | best-effort reader | explicitly incomplete, non-authoritative export |

The first repair classes are:

1. truncate only an incomplete final WAL frame after proving the valid prefix;
2. remove only temporary/orphan physical objects proven unreachable from every
   authenticated current manifest, snapshot, and checkpoint;
3. rebuild graph, scalar, BM25, vector, TurboQuant, and analytical projections
   from canonical committed semantic state and exact source cursors;
4. resume or reconcile install/attunement actions from durable checkpoints and
   exact effect receipts; and
5. restore an authenticated backup into an absent staging target, verify it,
   then require an explicit publication plan.

Every `repair apply` is offline. It first preserves an authenticated,
content-addressed pre-repair snapshot and then materializes an absent candidate
at `.rrflow/rrd/roots/<candidate-storage-root-id>/` through a filesystem
reflink/clone or fully accounted copy. The plan declares the selected
mechanism, worst-case allocated bytes, fallback, same-filesystem publication
precondition, and locator replacement action before mutation. Repair changes
only the candidate, rechecks the input report/plan digest, records before/after
inventories and an audit receipt in the candidate, and runs full verification
before atomically publishing it through the locator. The prior root remains
retained or quarantined under the exact plan; it is never overwritten.
Acknowledged complete-frame corruption, an unauthenticated manifest choice,
irreconcilable semantic divergence, or unknown authorization truth can never
be "fixed" by guessed values.

## Backup and restore

Backup creation is an authenticated engine operation at one snapshot boundary.
The application-complete form includes logical semantic commits, catalogues,
immutable objects, configuration/schema identities, projection source
coordinates, and the receipts needed to prove completeness. Backup verification
is read-only and authenticates the whole catalogue closure.

On an installed estate the backup catalogue lives at
`.rrflow/rrd/backups/<instance-id>/` and restore staging roots at
`.rrflow/rrd/restores/<instance-id>/<restore-id>/`, both inside the managed
persistent container beside `roots/` and never below it: `roots/` holds only
storage roots the locator can select, and install recovery rejects any other
entry there. Both directories are created on first use, not by installation,
and uninstall retains them with the rrflowDB data. A component or test root
opened outside an installation keeps its sibling `rrd-service/<instance>` tree;
that tree is not an installed-estate location.

Restore never overlays the live data root. It writes an absent staging root,
validates format and application identity before replay, restores canonical
semantic state, validates every object, rebuilds or authenticates projections,
opens it through the same installed engine path, and emits a publication plan.
Publication requires the operator to choose the retained/quarantined old root
and accept the exact digest.

## Uninstall and data destruction

Uninstall uses plan/apply and owns only RRFlow-installed integration. It
retains rrflowDB data and backups, stops RRFlow-owned services through
authenticated receipts, revokes active bindings, removes only unchanged
managed files/regions, and durably marks the estate `RETAINED`. The bounded
retention locator remains so status, verification, backup/restore, or an exact
reinstallation can find the bytes without treating them as active. Project
source, application databases, provider state, mesh state, generator output,
and harness state are never deleted.

Data purge is not an uninstall default and is outside the alpha command until a
separate destructive-operation contract proves target resolution, retention,
backup, hold, authorization, and recoverability rules.

## Trace, resource, and debugging evidence

Each lifecycle operation records one correlated trace rooted at the public
request. Required attributes are typed identities/digests and bounded counters,
not secret values or arbitrary paths. Install/repair spans identify plan and
action digests, state transitions, effect receipt identities, fsync/publication
boundaries, elapsed time, and bytes created/read/copied. Verify records every
check class and omission. Serve/readiness records the binary/bundle identity
and authenticated instance coordinates. Restore records source backup,
staging, verification, and publication receipts.

Failures must identify the exact stage, immutable input identity, safe retry
classification, remaining state, and next permitted operation. Logs and error
details are bounded and redacted.

## Artifact acceptance matrix

The default distribution is not qualified until all rows pass for every
supported target:

| Proof | Linux | Windows | macOS |
|---|---|---|---|
| Native archive and primary executable | `rrflow` | `rrflow.exe` | `rrflow` |
| Archive/executable SHA-256 and manifest inventory | required | required | required |
| Platform signature/attestation and SBOM provenance | required | Authenticode plus required provenance | required |
| Clean machine has no compiler, checkout, sibling repo, registry, or network | required | required | required |
| `version`, install plan/apply, serve, authenticated ready | required | required | required |
| Commit, query/context, close, reopen, verify | required | required | required |
| Crash/torn-tail report, repair plan/apply, full verify | required | required | required |
| Backup verify, restore to absent root, readback | required | required | required |
| Uninstall plan/apply preserves project and retained data | required | required | required |

## Current-to-target disposition

- `rrflow-cli` is now the primary executable for version, deterministic install
  plan/apply, foreground serve, authenticated ready, and quick verification;
  it remains a thin adapter over lifecycle operations owned by `RrdEngine`.
- The new portable locator and engine-owned active configuration are the only
  accepted product destination. The pre-canonical manifest/private binding,
  raw-root startup openers, and creation callers are removed; explicit
  create-or-open engine construction remains only in component tests that do
  not claim installation.
- The internal `rrd-server` process and embedded `rrflow-mcp` mode require an
  installed project plus the exact distribution executable and call
  `RrdEngine::open_installed`; neither can initialize absent state. They remain
  internal/adapter artifacts rather than additional operator lifecycle roots.
- `RrflowKvStore` has explicit `create_new` and `open_existing` constructors
  beside the retained create-or-open `open`, which the local estate authority
  and both restore paths still call when they check a restored target;
  inspection is separate and read-only. Replacing those create-or-open calls
  is D-11 work.
- `rrflow-cli::dev::supervisor`, its private files, the standalone security
  initializer, `rrd-server initialize`, and create-on-start product paths are
  absent after the D-01 replacement tests passed.
- the retained application-backup component now opens only an existing complete
  component root. It intentionally cannot resolve the canonical installed
  `roots/<install-id>` estate yet and cannot create a second store at the old
  `.rrflow/rrd` parent. Installed backup/restore commands and
  locator/distribution-bound controller inputs remain D-11/POA&M work.
- the engine's `backup-create`, `backup-list`, and `restore-create` operations
  on an installed estate (opened through `open_installed` by `serve`) write to
  `.rrflow/rrd/backups/<instance-id>/` and `.rrflow/rrd/restores/<instance-id>/`.
  Before that correction they wrote below `roots/`, where install recovery
  rejects any entry but the storage root. Restore still stages without a
  publication plan; publication into `roots/<candidate-storage-root-id>/` with
  locator replacement remains D-11 work.
- current WAL torn-tail repair is retained only as a physical primitive invoked
  by an exact engine repair plan.

## Alpha acceptance

The lifecycle is accepted only when the same release candidate executes the
matrix above on empty and existing projects, with no manual source-tree command
or hidden companion service. That proves an operable product spine; it does not
by itself complete native graph/BM25/vector, streamed DataFusion, reasoning,
attunement, automation, Connectome, or the full RRFlow 1.0 objective.
