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
rrflow install identity
rrflow install plan   --project <root> --identity <256-bit-hex> --mode fresh|existing --profile <id> [--configuration <toml>]
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

First obtain one explicit identity input with `rrflow install identity`. It
returns `installation_identity`, generated from the OS entropy source, and
writes no project or engine state. Retain that input across previews of this
installation; obtain a new value for each independent project. The identity
is not a credential and must not be reused to claim that two independent
estates are one. A UI can allocate it once when beginning installation and
retain it with its pending preview, requiring no extra user entry.

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

Installation-plan contract version 4 seals the explicit identity input and
`project_root_sha256`, a digest of the initial canonical root. Identical project
content does not allocate an estate identity. New apply and interrupted recovery
must match that root before effects. Once the installed locator and record are
published, relocation, reopen and exact completed replay retain the persisted
identity; the live path is not its permanent identity. Prior version-1/2/3 plans
are rejected. Attunement and result envelopes retain their independent version
1; product version remains `1.0.0`. This is not a binary-upgrade or estate-
migration facility: retain the executable that created an existing pre-alpha
estate until its qualified upgrade path exists.
`rrflow --json version` advertises `installation_plan_contract_version` so
clients can check the installation boundary without treating the frozen
product version as a capability signal.

The plan also previews `acquire_installation_lease` and the ninth
managed path, `.rrflow-install.lock`. Its precondition means absent or a
validated private, empty, single-link regular file; its removal rule is
`retain_for_exclusion`. File presence does not mean an installation is active
or complete. The native handle lock is the only admission owner. Apply and
recovery retain it through locator publication and intent acknowledgement;
contenders fail immediately with a storage conflict. Closing the handle or
process termination releases ownership without unlinking the file. Lease
contents are never repaired, PID/age never permits takeover, and unrelated
projects have independent admission. Read-only preview creates nothing and
excludes this path from project inventory only after validating it. Invalid
new-install preconditions are checked before creation, then checked again
under the lock. This local native-file contract establishes no network-share
locking qualification.

Version 4 seals the linked generic template bytes and selected specialization
in `InstallationBootstrap`, persisted within the installed binding. `--profile
default` selects a provider-neutral seat; `--profile clyffy` explicitly selects
the bundled Clyffy seat. Both keep endpoint candidates absent and external
process, network invocation and activated-provider lists empty. Installation
never activates a provider or rewrites project-owned instructions.

The thirteen managed paths include the lease, intent, locator staging file,
`.rrflow/bootstrap.md` and its staging file. Ten ordered actions distinguish
credential generation in the intent from later credential delivery. The plan
binds the ten initial logical records (seat schema/record, security policy/audit/
head, configuration, installed binding, checkpoint and pending attunement
plan/job), owner-only directories and secret files, entropy sizes, and the
rrflowKV owner's format coordinates. Initial semantic payloads are bounded to
one MiB each and four MiB together; these are semantic admission limits, not a
claim that physical storage uses exactly that number of bytes. Template and
rendered instruction text are each limited to 64 KiB.

Owned assets pin LF checkout bytes. Preview seals their exact digests; apply
rejects altered or rehashed bundle input before effects. Instructions publish
through an exact-content staging file. Recovery checks its bytes, and installed
inspection/open/replay reject instruction drift without silently repairing it.
Dynamic time and secret inputs are obtained only during authorized apply through
the existing engine clock and OS entropy authority. Actual installed/transaction,
credential and manifest digests remain in existing receipts. The attunement job
remains pending; this bootstrap is not verified source indexing or provider
configuration. The [D-01 accepted matrix](../../roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21)
binds native qualification and foundation acceptance to exact candidate evidence.

## Install apply

`rrflow install apply` accepts only the named plan and expected digest. It:

1. resolves and revalidates the exact bundle and project preconditions;
2. acquires a nonblocking native lock on the previewed private empty
   `.rrflow-install.lock` file without following an unreviewed link or mount
   change; retains that handle through final acknowledgement;
3. prepares the accepted clock anchor and secrets once in the private intent;
4. creates the private estate directories and publishes exact bundled instructions;
5. creates the plan-addressed rrflowKV root and delivers the prepared credentials;
6. commits the installed binding, initial seat, security authority, checkpoint,
   audit, configuration and pending attunement plan/job through `RrdEngine`;
7. verifies the committed instance and atomically publishes its minimal locator; and
8. acknowledges the exact intent without treating file presence as job completion.

Every crash boundary resumes the same plan or fails closed. It cannot create a
second credential, policy, estate, instance, audit outcome, or project edit.
No listener becomes ready before authentication policy and the installed
binding are committed.

## Current pre-release slice and rollout

Packages `D01-02-canonical-estate-layout-and-configuration-v1`,
`D01-04-precanonical-authority-convergence-and-install-recovery-v5`, and
`D01-05-engine-owned-installation-clock-v1` implement the current concrete
startup slice without claiming the complete target above:

- `rrd-contract` owns `InstalledEstateIdentity`, three independent deployment
  coordinates, and a strict self-digesting format-3 `EstateConfiguration`,
  including `clock.maximum_rollback_ms` with a zero default and required native
  storage limits;
- the bundled default configuration or one explicit regular, non-symlinked
  TOML file up to 64 KiB is read during planning, validated below hard compiled
  maxima, assigned revision 1, and sealed into the plan;
- apply never rereads that mutable input. It consumes the sealed value and
  creates, in declared order, `.rrflow/`, `.rrflow/rrd/`,
  `.rrflow/rrd/roots/`, `.rrflow/credentials/`, bundled bootstrap instructions,
  one create-new storage root, token key, credential document and `.rrflow/config.toml`;
- the active configuration is committed inside rrflowDB at
  `server/state/<instance>/configuration/active` before locator publication;
- locator format 2 projects the sealed storage policy for admission before
  WAL replay. Installed open and read-only inspection validate that projection
  against the installed record and effective configuration before readiness;
- query, live-query, query-index, and context requests above the installed
  ceiling fail before their data operation; and
- server capabilities and generated OpenAPI/SDK types expose the effective
  configuration and exact deployment coordinates. Reasoning ceilings are
  visible, while `governed-reasoning-runner` remains honestly unavailable;
- before the first `.rrflow` directory exists, `RrdEngine` obtains one fallible
  `host_system_time` / `unverified` observation and publishes one owner-only,
  plan-addressed recovery intent containing that frozen anchor and the exact
  generated token/credential bytes. Ordinary callers cannot supply time. Retry
  accepts the intent only when its own
  digest and the plan, target, project precondition, profile, executable, and
  configuration still match;
- apply creates or exactly verifies each declared directory, store, token,
  credential, one semantic installation commit, pending locator, and active
  locator. It classifies empty, committed, published, and complete states and
  removes only the exact pending locator and acknowledged intent it owns;
- deterministic test-only observation covers lease acquisition and ten
  durable stages: intent, directories, bootstrap staging/publication, store,
  token, credential, engine commit, locator publication and intent acknowledgement. Retrying the same plan
  preserves the first clock anchor and leaves one token, credential, installed
  record, policy, audit/checkpoint result, attunement plan/job, and locator;
- recovery, exact replay, and installed open compare a fresh observation with
  the anchor and fail closed when time is unavailable or rollback exceeds the
  sealed policy. Read-only inspection still completes structural checks and
  reports `current`, `rollback_within_tolerance`, `rollback_exceeded`, or
  `unavailable` with exact non-secret coordinates; the last two make overall
  verification fail;
- malformed, symbolic, foreign, mismatched, partly committed, or missing-secret
  recovery state is preserved and rejected rather than guessed or overwritten;
  and
- primary CLI, internal server process, embedded MCP, checked six-language SDK
  conformance daemon, deployment catalogue, and Kubernetes rendering now use
  canonical installation/open. The pre-canonical manifest/private binding,
  raw product CLI, development supervisor, standalone security initializer,
  and server initializer are removed with no successful compatibility reader.

### Reasoning-limit configuration

Installation can seal reasoning ceilings from `install plan --configuration
<toml>`. Copy the complete bundled
[`default-estate-configuration-v3.toml`](../../../crates/authority/rrd-engine/assets/install/default-estate-configuration-v3.toml)
as the input and edit its existing `[reasoning]` table, for example:

```toml
[reasoning]
max_run_elapsed_ms = 120000
max_steps = 16
max_step_elapsed_ms = 15000
```

This is a two-minute, sixteen-step, fifteen-second-per-step configuration
example, not a complete configuration file. The current engine embedding path
uses the fifteen-second step ceiling for waiting and result acceptance. Total
run time and step-count enforcement still require the governed reasoning runner.
The full input must retain its required storage/clock/recall/query sections.
Preview and apply the sealed installation plan through the existing commands;
do not edit an installed `.rrflow/config.toml` to bypass the active revision.
Post-install governed configuration changes remain D-06 work.

The existing parser requires positive limits, per-step time no greater than
total time, and compiled maxima of one hour per run, 10,000 steps and five
minutes per step. The default is fifteen minutes, 256 steps and one minute
per step. `governed-reasoning-runner` is unavailable. A synchronous embedding
adapter may continue after its caller times out; its execution slot remains
occupied until it actually returns. The engine returns a non-retryable deadline
error and does not accept late output. This bounded wait is not proof of
end-to-end loop prevention, cancellation or a provider cost ceiling. The
[LFG/SWAG budget contract](../inference/lfg-swag.md#user-selected-effort-and-run-budgets)
owns aggregate accounting, retries, effort presets, stop/resume and required
runtime proof. Existing query/context limits remain separately enforced.

The Rust embedded generation API additionally accepts a process-local
cancellation handle. FeatureHash observes it cooperatively; other adapters
may finish their current blocking call before releasing admission. This is
not an installed daemon/UI Stop operation or durable cancellation after restart.

### Native storage limits

The required `[storage]` configuration separates active-state, output, cache,
and maintenance controls:
`mutable_accounted_bytes` (128 MiB default), `mutable_versions` (1,048,576),
`projected_output_bytes` (128 MiB), `page_cache_bytes` (4 MiB, zero disables
caching), `wal_flush_target_bytes` (64 MiB) and
`memtable_flush_target_versions` (524,288). The first two are native active-state
hard caps; the next two bound retained projected allocations and cache bytes;
the last two are soft synchronous maintenance targets. A single atomic batch
may exceed a soft target only within hard admission. These are not aggregate
RSS bounds. Protocol ceilings and checked native conversion
reject invalid configuration. Current limits require reopening; no live
configure operation or qualified capacity profile is implied.

Immutable maintenance adds `compaction_input_bytes` (1 GiB),
`compaction_input_logical_bytes` (256 MiB), `compaction_input_versions`
(4,194,304), `compaction_output_segments` (256), `compaction_elapsed_ms`
(30,000) and `compaction_l0_segments` (64). Verified segment metadata admits
compressed and decoded input, including complete hot-key histories, before
cursor decoding. Logical bytes count page payloads, not heap overhead or RSS.
The deadline is cooperative between records/partitions and before publication;
it cannot interrupt a filesystem operation. Output partitions are bounded
before creation. L0 saturation denies further write admission if synchronous
maintenance cannot make room, including repeated attempts after a prior flush.
Explicit flushes also cannot exceed the L0 cap. Pressure never authorizes
pruning protected history. Unpublished partition files from interrupted work
remain subject to the existing validated garbage collector. Total on-disk
history, higher-level debt, retained snapshots and process memory are separate
qualification obligations; these controls do not claim those are bounded.

Retained mutable generations share an ownership-based pool per opened database:
`retained_mutable_bytes` (512 MiB), `retained_mutable_versions` (4,194,304) and
`retained_mutable_generations` (64). A writer reserves a complete copy before
copy-on-write and before WAL append; cancellation releases only the owners it
actually drops. Readers can retain their charge after the database closes.
The byte charge covers keys, values and version records, excluding allocator
and tree overhead. Independently reopened pools do not establish a host quota.

The normal write path now invokes bounded reachability reclamation after
publication and on reopen, before appending the incoming batch. Controls are
`reclamation_inventory_entries` (262,144), `reclamation_metadata_bytes` (64 MiB),
`reclamation_delete_files` (1,024), `reclamation_delete_bytes` (1 GiB), and
`reclamation_elapsed_ms` (30,000). Current, captured-reader and checkpoint
manifests protect their files. Complete admitted inventories and protected
references are checked before deletion; damaged references fail closed.
Partial deletion cycles synchronize completed removals and retain pending work.
A partial cycle returns reclamation pressure before the incoming WAL append;
retries drain that finite work before admitting additional growth.
A single file larger than the deletion budget reports explicit pressure.
Elapsed limits are cooperative and cannot interrupt native filesystem calls.
These controls do not qualify actual disk exhaustion or sustained capacity.

Engine transaction serialization uses a FIFO admission gate with
`transaction_admission_requests` (64 active plus queued requests),
`transaction_admission_bytes` (64 MiB aggregate serialized logical request
charge), and `transaction_admission_wait_ms` (30,000). A bounded counting sink
measures each request without allocating a second serialized buffer. The
active owner remains charged until its actual guard drops. Timeout and pending
cancellation release queued reservations; a panic in effect code fences future
admission. These controls cover the existing transaction/unique-index/function/
flight/task/operator gate, not transport parsing or total process allocation.
`AuthorizedInvocation::cancel_pending` controls an authenticated embedded
invocation waiting for that gate; it is not an HTTP disconnect hook or rollback
of effects already admitted. Request-context deadlines are checked before
admission. Native `transaction_admission_snapshot` exposes content-free current
and peak charge; no new wire operation or independent lifecycle is introduced.

Recovery and mutation-free inspection use `recovery_metadata_bytes` (64 MiB),
`recovery_files` (262,144 admitted file reads), `recovery_read_bytes` (1 TiB
conservative whole-file charge), `recovery_wal_bytes` (256 MiB),
`recovery_wal_frames` (1,048,576), and `recovery_elapsed_ms` (120,000). Metadata
is admitted before allocation, immutable files before segment opening, and WAL
size/frames before payload materialization. Inspection charges repeated reads
again, including its full-file digest pass. Its hashing loop checks elapsed
work and rejects changed file lengths. Budget exhaustion preserves bytes and
returns explicit non-readiness; it does not repair tails or relax the installed
policy. Elapsed checks are cooperative, not interruption of an OS call. These
work bounds do not establish recovery disk reserve or total process RSS.

Apply, interrupted installation, reopen, inspection, backup restore and restore
publication lower the same sealed policy into native storage options. A low
replay limit denies access without modifying authoritative bytes; inspection
does not silently retry with larger defaults. Configuration format 2 and locator
format 1 are replaced pre-release shapes and are rejected, not migrated or
silently defaulted. Product version remains 1.0.0.

The implementation rejects any pre-existing `.rrflow` tree for a new install.
It does not migrate, merge, or delete a user's pre-canonical
`.rrflow/instance.toml` state; that collision remains a fail-closed diagnostic.
The tracked development manifest has been deleted because no source path reads
it. The clock substrate proves only relative rollback against a sealed anchor:
it does not authenticate or correct UTC, detect every forward error or drift,
or establish runtime-wide timestamp high-water rules. Platform synchronization
diagnosis, explicitly authorized correction, and native clock qualification
remain open. D-01's bounded native filesystem and ACL qualification is accepted
in the linked matrix. Configuration plan/apply, attunement phase execution, full
verify/repair/restore/uninstall, service integration, release assembly, and
clean-machine qualification remain separate packages. The staged rationale and
exit proofs are in the
[canonical estate/configuration research](../../research/rrflow-canonical-estate-configuration-and-runtime-controls.md#rollout-sequence).

## Native secret leaf-file boundary

Token keys, operator credentials, installation recovery intents and restore
publication intents use one engine-owned bounded reader. It opens once,
validates that handle before and after reading, and rejects multiple hard
links, non-regular objects, oversized content and observed length/mtime drift.
Exclusive creation establishes permissions before writing any secret bytes;
reading never repairs permissions or replaces an unsafe file.

On Unix, the leaf open uses no-follow and nonblocking flags, requires the
effective user's ownership, and denies group/other permission bits. Creation
requests mode `0600`. On Windows, creation supplies a protected DACL granting
only the current process user and LocalSystem. The reader checks the opened
disk object's owner, protected non-null DACL, basic non-inherited allow entries
for those principals, regular/non-reparse attributes and single link. The
read handle excludes concurrent write/delete sharing. Unsupported native
security enforcement fails closed; no runtime ACL utility is required.

These private-object checks compose with retained filesystem authority below.
They do not establish an atomic project snapshot, resistance to every
same-owner race, a credential migration facility, or qualification for arbitrary
filesystems. Privileged foreign-owner cases now pass explicitly on Linux,
macOS and Windows in the
[D-01 accepted matrix](../../roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21).
The
[native secret boundary journal](../../evidence/change-journals/gate-d/d-01-native-secret-boundary.md)
records real filesystem denial and retained installation/recovery evidence.
D-01's installed foundation is accepted; wider operational and release gates remain open.

## Open, serve, and readiness

### Private estate-directory admission

The four engine-owned boundaries `.rrflow`, `.rrflow/rrd`,
`.rrflow/rrd/roots` and `.rrflow/credentials` are created with private native
security. Unix creation requests mode `0700`; admission requires that mode
and effective-user ownership on the opened directory. Windows creation
supplies a protected owner/LocalSystem DACL with object/container inheritance;
admission inspects the directory handle's owner, DACL and non-reparse disk
attributes. The default project-parent ACL is not used as the security policy.

Apply and recovery validate every present boundary before reading recovery
secrets or advancing store work. Installed locator, credential, open and
inspection operations require all four boundaries. Missing paths are allowed
only during new or interrupted apply. A changed mode, unsupported ACL or
non-directory object is refused without permission repair; project-owned
parents are never tightened. Creation does not replace an existing object.

These checks extend the private-object boundary. Existing pre-alpha estates
with incompatible directory permissions
are refused rather than silently upgraded.

### Retained filesystem authority

Installation retains the admitted project directory and each private estate
boundary. Store, manifest, WAL, segment, object, backup, archive and restore
operations carry directory capabilities through their existing owners.
Diagnostic path strings cannot be used as capability inputs. Each relative
component is opened without following links and checked against the admitted
native mount identity. Payload admission rejects nonregular files and multiple
hard links before reads or truncation; this also applies to installation's
project-input inventory. Public standalone path APIs establish authority once
at entry. Host parent aliases may be resolved before initial admission; the
selected root leaf itself cannot be a link.

WAL replay and writable recovery retain the same file. Object digest
verification hands its opened file to vector mapping and snapshot transport;
consumers do not reopen a previously verified pathname. Publication and cleanup
operate relative to retained parents. Locator and vector publication use atomic
no-replace operations, preserving a concurrent winner. Windows publication holds
all ancestors without delete sharing and uses write-through rename. Unix uses
directory-relative rename and directory synchronization. Linux requires native
mount IDs; macOS uses the filesystem identity and mounted-on name; Windows
rejects reparses and volumes without persistent ACL support. Unsupported native
enforcement is an error, with no pathname fallback.

The [containment journal](../../evidence/change-journals/gate-d/d-01-anchored-filesystem.md)
records platform-specific tests and qualification limits. These operations do
not make mutable project content an atomic snapshot or protect against every
same-owner modification. Whole-install exclusivity, bounded installed bootstrap
and the native offline matrix are accepted in D-01.3 through D-01.6. Network
filesystems and arbitrary filesystem implementations are not qualified by the
local native runner results.

### Installed engine admission

Normal open is `open_existing`, never create-or-open. A read-only inspector
first resolves the project locator, validates the installed closure, and
assesses the engine-owned host observation against the installed clock anchor
and policy. Unsupported, corrupt, or unsafe-time state is rejected before the
persistent engine is opened. Accepted open then validates the same locator,
record, anchor, and configuration again while authenticating the CURRENT
manifest closure, replaying only complete contiguous WAL frames, and restoring
the MVCC view. A torn final frame is reported as a
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
- engine clock source/trust, sealed anchor, current observation or failure,
  rollback, and configured tolerance;
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
| Restore | Creates absent target | digest-verified backup plus engine installer | verified replacement candidate |
| Salvage | Writes separate output only | best-effort reader | explicitly incomplete, non-authoritative export |

The first repair classes are:

1. truncate only an incomplete final WAL frame after proving the valid prefix;
2. remove only temporary/orphan physical objects proven unreachable from every
   authenticated current manifest, snapshot, and checkpoint;
3. rebuild graph, scalar, BM25, vector, TurboQuant, and analytical projections
   from canonical committed semantic state and exact source cursors;
4. resume or reconcile install/attunement actions from durable checkpoints and
   exact effect receipts; and
5. restore a digest-verified backup into an absent staging target, verify it,
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

### Implemented incomplete-tail candidate repair

The current `repair plan --project <root>` command emits JSON to stdout;
`repair apply --project <root> --plan <file> --expect <sha256>` accepts that
exact artifact. Only an incomplete final WAL frame qualifies. Planning and
ordinary verification preserve bytes; complete checksum corruption, missing
authoritative files, a stale physical inventory or an insufficient recovery
budget deny repair.

The bounded copy includes every regular file in the active root, including
object payloads and the private token. Its plan declares source inventory,
copy bytes, no fallback, same-filesystem publication and quarantine. Only the
candidate WAL is truncated. The original root is retained under a
content-addressed quarantine; a private intent resumes interrupted copy and
publication, and the existing control journal records completion. The candidate
carries before/after inventory identities. This implementation retains the
existing installed root name and locator binding, matching the current restore
publication profile. General locator replacement, other repair classes and
full semantic/index verification remain D-11 work. The
[C-07f journal](../../evidence/change-journals/gate-c/c-07f-native-capacity.md)
separates implementation and compilation from native qualification results.

## Backup and restore

Backup creation is an authenticated engine operation at one snapshot boundary.
The application-complete form includes logical semantic commits, catalogues,
immutable objects, the estate's control state, configuration/schema
identities, projection source coordinates, and the receipts needed to prove
completeness. Backup verification is read-only and verifies the digests of the
whole catalogue closure. A digest detects changed bytes; it does not
authenticate a producer, because the catalogue shares the estate's filesystem
trust domain and a keyed authenticator would sit beside the key it protects.
Verification creates nothing: object stores and restored roots are opened as
existing, and a catalogue file that is absent while retained artifacts exist
beside it is reported as a loss rather than read as an empty catalogue.

The control state travels as a content-addressed control-journal archive
(`control-archives/<sha256>.rrd-control-archive`, format `RRDCJA01`), not as
materialized values: readers such as the security audit chain derive state from
the control journal's actions and order, so a restore replays the same
transitions in the same order. The engine declares the families it carries,
and the archive records them:

| Family | Content |
|---|---|
| `server/state/install` | installed record and install checkpoint |
| `server/state/<instance>/configuration` | active estate configuration |
| `server/state/<instance>/security` | security policy (principals, roles, bindings, issuers; credentials as digests) |
| `server/state/<instance>/audit`, `server/state/<instance>/audit-head` | the hash-chained audit records and their durable head |
| `server/state/<instance>/attunement` | attunement plan and job state |
| `server/state/estate` | estate documents |

Every value in these families is non-secret. The token key (`RRD.TOKEN`) and
the operator credential are files outside the store and are never part of a
backup; a restored root has no token key until publication supplies one. Not
carried: session leases, subscriptions, backup and restore operation receipts,
and estate-recovery receipts, which are transient or belong to the instance
that performed the operation. The two catalogue families travel in the
catalogue manifest. A backup whose scope would overlap the catalogue families,
or whose families overlap each other, is refused before any artifact is
written.

The backup catalogue is format version 2. Version 1 catalogues, whose
application-complete entries omitted the control state, are refused rather
than read as complete: `application_complete` is true only when the logical
archive, object payloads, catalogue manifest, and control-journal archive are
all present and digest-verified. The public `InstanceBackupSnapshot` projects
the control state as its own coverage dimension, `installed_control`, beside
`claims`, `catalogues`, and `object_payloads`, so a client reads what a backup
carries rather than inferring it from `application_complete`.

Restore replays the archive after the catalogue and object payloads, one
control transition per archived entry, each committed in the same physical
transaction as a replay-progress marker in the restored root's system space.
An interrupted replay resumes after its last committed entry; a target whose
control state does not hold an entry's recorded before-state fails closed
before any further mutation. The marker stays in the root as restore
provenance, and restore verification checks that the restored journal carries
every archived entry contiguously and in order with the same key, before-state,
value, time, actor, action, and request identities.

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

Publication is offline and takes the plan/apply shape of installation.
`rrflow restore plan --restore <id> --backup <sha256>` emits a deterministic,
digest-bound plan that binds the staged candidate, the active storage root it
replaces, the backup that produced the candidate, and read-only evidence of
both roots (physical digest, runtime manifest, runtime cursor, claim sequence,
control sequence); the candidate's installed record must equal the active
record and its configuration must match the locator, and the plan refuses a
served estate because the read-only inspector cannot take its shared lock under
a running writer. `rrflow restore apply --plan <file> --expect <sha256>`
executes only that plan: it holds an exclusive publication lease for the
instance, re-verifies both roots and the candidate's backup closure, carries the
token key into the candidate (a backup never contains it), renames the active
root to `.rrflow/rrd/restores/<instance-id>/<restore-id>.quarantined-<storage-root-id>/`,
renames the candidate into the storage root's own name, opens the result
through `open_installed`, records the publication receipt in its control
journal, and passes quick verification. Because the candidate takes the
storage root's own name, the locator, the installed record, and the single-root
install-recovery rule are unchanged; the quarantined root is never overwritten.
Every stage is durable in a private intent beside the candidate, so a rerun
after a crash at any stage completes the same publication once, and a rerun
after completion returns the recorded receipt as a replay. A candidate whose
configuration revision differs from the active locator is refused; locator
replacement for that case is D-11 work.

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

Clock traces contain the lifecycle boundary, source/trust, assessment, anchor,
observation, rollback, tolerance, and failure class. They do not contain
credentials, project content, prompts, private model reasoning, or arbitrary
paths. `SystemTime` supplies fallible wall observations; monotonic process time
is reserved for elapsed waits and measurements and is never persisted as civil
authority.

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
- Installation, retry, replay, open, and quick verification now share one
  engine-owned clock boundary. The native lifecycle combines abrupt termination,
  authenticated restart and backup publication/readback, with graceful SIGINT
  on subsequent Unix shutdowns. The [integration repair receipt](../../evidence/change-journals/repository/interrupted-local-integration-repair.md)
  records local Windows results and binds candidate CI; broader native platform
  and signed distribution qualification remain open.
- The internal `rrd-server` process and embedded `rrflow-mcp` mode require an
  installed project plus the exact distribution executable and call
  `RrdEngine::open_installed`; neither can initialize absent state. They remain
  internal/adapter artifacts rather than additional operator lifecycle roots.
- `RrflowKvStore` has explicit `create_new` and `open_existing` constructors
  beside the retained create-or-open `open`. Every restore path, including the
  local estate authority's, checks a restored target with `open_existing`, so
  an empty or foreign target fails closed; `open` remains for component roots
  and tests that do not claim installation. Inspection is separate and
  read-only.
- `rrflow-cli::dev::supervisor`, its private files, the standalone security
  initializer, `rrd-server initialize`, and create-on-start product paths are
  absent after the D-01 replacement tests passed.
- the estate controller's backup, prune, and restore paths resolve a managed
  instance as the installed project `instances/<instance>` under the estate
  state root: they read its locator, require the controller's
  `--distribution-executable` to be the installed distribution, and act on the
  instance's own `.rrflow/rrd/backups/<id>` catalogue and
  `.rrflow/rrd/restores/<id>/` staging roots — the catalogue the served engine
  lists, and the staging roots `restore plan|apply` publishes from.
  The backup driver opens the instance through `open_installed` with its real
  token key; a project that is not an installed estate, a pre-canonical
  `.rrflow/rrd` component store, or a different distribution is refused
  permanently without creating anything. The former `<state-root>/backups` and
  `<state-root>/restores` lane is gone. A retention decision applies only when
  the estate's recovery points are exactly the instance's catalogue; a
  catalogue that also holds backups created through the served engine's
  `backup-create` is refused until the estate can register them (open D-11
  work).
- the engine's `backup-create`, `backup-list`, and `restore-create` operations
  on an installed estate (opened through `open_installed` by `serve`) write to
  `.rrflow/rrd/backups/<instance-id>/` and `.rrflow/rrd/restores/<instance-id>/`.
  Before that correction they wrote below `roots/`, where install recovery
  rejects any entry but the storage root. Restore stages; publication is the
  offline `restore plan|apply` described above. A restore into an already
  staged target is recovered by opening it as an existing store and verifying
  it, never by creating a database there.
- a backup of an installed estate carries the installed control state as the
  control-journal archive above, and `rrflow restore plan|apply` publishes a
  staged restore into the storage root's own name with the previous root
  quarantined, so a restored estate opens through `open_installed` and reaches
  authenticated readiness ([POAM-031](../../poam/rrflow-1.0-alpha/poam-031.md),
  closed). Still D-11 work: locator replacement for a candidate whose
  configuration revision differs from the active locator, and the remaining
  restore and salvage items under POAM-025.
- current WAL torn-tail repair is retained only as a physical primitive invoked
  by an exact engine repair plan.

## Alpha acceptance

The lifecycle is accepted only when the same release candidate executes the
matrix above on empty and existing projects, with no manual source-tree command
or hidden companion service. That proves an operable product spine; it does not
by itself complete native graph/BM25/vector, streamed DataFusion, reasoning,
attunement, automation, Connectome, or the full RRFlow 1.0 objective.
