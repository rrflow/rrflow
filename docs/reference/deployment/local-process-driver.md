# RRFlow local process adapter

**Status:** active target contract; current implementation convergence is incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/deployment/local-process-driver`
**Owner:** host-local launch, observation, readiness, graceful shutdown, and forced-stop effects for one installed RRD process

This record defines the local operating-system process boundary. The
[instance topology](../../architecture/instance-topology.md) owns the project,
engram, instance, deployment-form, and storage-profile relationships. The
[engram-control reference](../operations/engram-control.md) owns durable
desired/observed state and external-effect reconciliation. The
[security authority](../security/authority.md) owns authentication,
authorization, and audit. The [server reference](../protocol/server.md) owns
RRD's HTTP/WebSocket process behavior. This adapter implements effects prepared
under those authorities; it cannot redefine them.

## Boundary and placement

A local process is one deployment effect for the single-node server form. It
is not rrflowDB, a storage profile, an installer, a project lifecycle, a
reasoning routine, or canonical evidence that an instance is ready. The target
implementation belongs in one outward `rrflow-local-process` adapter package,
created only by its owning implementation gate. `rrd-engram` retains pure
desired/observed and effect-plan validation; it must not import filesystem,
process, hashing, platform API, or storage implementations.

The dependency direction is:

```text
installed host composition root
       │
       ├── resolved installed-engram binding
       ├── RrdEngine ── prepared effect plan / accepted receipt
       └── rrflow-local-process
                  ├── artifact and process identity
                  ├── exact argument/environment construction
                  ├── readiness and shutdown control channel
                  └── bounded host observation and diagnostics
```

`RrdEngine` never imports the concrete adapter. A composition root injects an
implementation of the pure effect port. The adapter receives no storage,
policy, audit, project-discovery, attunement, query, graph, index, or model
handle. It cannot open rrflowMX or rrflowKV and cannot declare an effect
committed. Only an engine transaction can accept a receipt and advance
canonical desired/observed state.

## Installation and cold-start authority

Only an explicitly previewed and applied `rrflow install` operation may create
or change local launch registration. The applied plan resolves artifacts from
the self-contained RRFlow distribution, commits the installed-engram binding,
and installs the selected foreground or operating-system service integration.
Ordinary start never runs `initialize`, creates a manifest, infers an identity
from a directory, downloads an artifact, finds a sibling checkout, or invokes
an undeclared generator.

Cold start cannot depend on the daemon already being available. The installed
launcher may execute the exact immutable plan authorized during installation,
but it has no permission to edit semantic state. The child then opens the
installed binding through `RrdEngine`, verifies project/engram/instance and
configuration identity, and publishes readiness through the protocol below.
When the engine is available, any later start, stop, or restart request is an
authenticated public operation whose intent and prepared external-effect plan
are committed before the effect is attempted.

Offline recovery is a separate installed operator capability with explicit
host authorization. It may stop a positively owned process, but its local
observation is not canonical state. It emits a bounded, integrity-protected
receipt which the next `RrdEngine` open validates against the installed
binding, prepared plan, operation identity, and fence before committing. A
host service manager, PID file, exit code, log, or marker can never advance an
RRFlow job on its own.

## Prepared launch plan

The exact wire and Rust names are frozen during A-07. Semantically, a prepared
local-process effect binds all of the following before execution:

| Field family | Required binding |
|---|---|
| Authority | instance, engram, project, operation, idempotency identity, lease/fence epoch, issuer, and plan digest |
| Installed artifact | distribution manifest entry, version, platform target, content digest, protected installed location, and executable file identity |
| Runtime configuration | installed configuration revision/digest, storage profile, deployment form, admitted project root, and non-secret locator references |
| Invocation | executable operation, ordered typed arguments, working directory, permitted non-secret environment, and inherited-handle policy |
| Limits | start/readiness/shutdown deadlines, diagnostic-byte retention, process/resource ceilings, and cancellation behavior |
| Verification | one-use startup challenge, required protocol/capability identity, readiness probe, process identity checks, and receipt schema/digest |

Arguments are structured values rendered one argument at a time. No shell,
command string, glob, substitution, or user-controlled relative path is
allowed. Instance-relative paths reject roots, parents, prefixes, symlink
escapes, mount escapes, and paths outside their admitted owner. A caller cannot
supply an executable, state root, database path, clock, environment, readiness
file, or shutdown file through a public request.

The process starts from an empty inherited environment. Platform-required
variables are explicit, minimal, and verified by the platform backend rather
than copied wholesale. Plaintext secrets never appear in the plan, command
line, environment catalogue, process record, log, trace, or error. A required
secret is resolved from its installed reference at the last responsible
boundary and delivered through an explicitly supported operating-system
protected handle or channel; unsupported delivery fails closed.

## Artifact-to-process identity

Checking a path's digest and later passing that path to `Command` is not enough:
the file can change between authentication and execution. The adapter must
prove that the executed image is the protected object authorized by the plan.
Each platform backend therefore binds content digest, stable file identity,
ownership and write protections, open/execute semantics, and the observed
child image. A characterization test must replace or relink the path at every
verification/spawn boundary and prove that an unverified image never runs.

An owned running process is identified by at least:

- PID and operating-system process-start identity;
- executable file identity and authenticated artifact digest;
- installed instance and prepared operation/plan identity; and
- the startup challenge/readiness receipt described below.

PID alone, PID plus elapsed stability, a matching path string, or an existing
record is insufficient. Before a signal or forced termination, the adapter
re-observes all platform-available identity fields and fails closed on PID
reuse, image replacement, inaccessible identity, or foreign ownership.

## Start and readiness flow

```text
resolve installed binding and immutable prepared plan
  -> verify plan/fence/deadline and protected executable object
  -> remove or invalidate only plan-owned stale bootstrap artifacts
  -> spawn exact executable + typed argv + bounded environment/handles
  -> discover PID/start/image identity and keep the child reaped on failure
  -> receive bounded bootstrap-ready candidate
  -> authenticate RRD health/readiness/capabilities over the selected endpoint
  -> verify challenge + project/engram/instance + configuration/artifact/plan
  -> submit effect receipt to RrdEngine
  -> atomically accept observation, audit, outbox, trace links, and cursor
```

A bootstrap signal is only an invitation to probe. If a platform requires a
file, pipe, or local socket for that signal, it is private, bounded, strict,
one-use, and bound to the startup challenge and plan digest. File existence or
listener bind alone is never readiness.

The authenticated readiness result binds the expected project, engram, RRD
instance, protocol version, capability digest, configuration revision,
artifact/plan digest, endpoint identity, startup challenge, and observed
process identity. The probe also proves the selected storage capability: an
rrflowKV deployment must open its installed durable engram; an rrflowMX
deployment must identify itself as volatile and cannot claim reopen or
durability. A wrong instance, stale plan, incomplete recovery, unexpected
capability set, early exit, or timeout fails the start and triggers bounded
cleanup of only the positively owned child.

## Stop, restart, and deletion

A normal stop uses an authenticated, operation-correlated control request. RRD
first rejects new work, drains or cancels work according to the prepared
deadline, commits any required durable state, and returns a typed shutdown
receipt before exiting. The adapter waits for both that receipt and the
matching owned-process exit. A request file plus a completion-file existence
check is not an accepted protocol.

If graceful shutdown times out, the adapter reauthenticates the complete
process identity before the bounded platform termination sequence. The receipt
records graceful versus forced outcome, attempted signals/actions, timings,
and final observation without fabricating graceful completion. Restart and
upgrade are fenced stop-plus-start operations; a new image never starts until
the old owned process is confirmed absent, unless a separately specified
rolling-deployment profile proves safe coexistence.

Stop, uninstall, and delete-instance control do not erase rrflowKV data,
backups, project content, logs owned by another operation, or external-system
state. Data destruction is a separately named, previewed, authorized operation
with retention and recovery evidence. An absent or foreign process is handled
idempotently only when the prepared operation and prior accepted receipts make
that result unambiguous.

## State, receipts, and storage profiles

Canonical process intent, prepared plans, observations, and accepted receipts
are typed engram records related to the installed instance and committed
through `RrdEngine`. They participate in the same rrflowMX/rrflowKV semantic
contract as other non-durability-specific state. rrflowKV additionally proves
prepared-before-effect recovery, lost-ack replay, receipt ingestion, and
restart convergence. rrflowMX may launch and observe a process, but after its
state is lost it cannot claim durable ownership or resume a prior operation.

The adapter may retain bounded local bootstrap material while the engine is
unavailable. Such material is a cache or sealed handoff receipt, never an
authoritative JSON process registry. It has one format, strict unknown-field
and size rejection, exact installed-root ownership, create-new publication,
integrity binding, expiry, and cleanup. It is accepted only after comparison
with canonical state and is safe to delete without changing RRFlow truth.

## Diagnostics and resource control

Stdout, stderr, crash data, and host observations are bounded evidence. The
installed policy defines rotation, total bytes, file count, age, permissions,
redaction, backpressure, and disk-reserve behavior before launch. Diagnostic
tails returned in an error are structured, size-limited, and redacted before
crossing the adapter boundary. Logs and traces never contain credentials or
advance operation state.

The adapter measures at least start and stop latency, CPU time, peak resident
memory, child count, bytes read/written, diagnostic bytes retained/dropped,
timeouts, retries, and forced terminations. Platform-specific controls may use
job objects, cgroups, rlimits, service managers, or equivalent facilities, but
those mechanisms are adapter details. Unsupported enforcement is explicit in
capabilities and cannot be reported as applied.

## Current implementation disposition

The current passing code is characterization inventory, not this contract.
Every useful behavior is retained only after equal-or-stronger proof at the
canonical destination.

| Current implementation | Useful behavior to preserve | Conflict to remove directly |
|---|---|---|
| `rrd-engram/src/local_process.rs` | typed argv without a shell; strict relative paths; executable hashing; PID/start/image checks; bounded readiness and stop; child cleanup; durable staging pattern; retained data on delete | OS/process/filesystem/hashing dependencies inside the engram domain; arbitrary state root; standalone process JSON; marker existence as truth; digest-to-exec race; plaintext environment; unbounded logs and raw stderr tail |
| `LocalDeploymentCatalog` and `rrd-deployment-catalog` | bounded strict decoding, content digest, create-new publication, explicit timeouts, empty preparation arguments, and an installed-only server invocation bound to project and distribution-executable digests | still a second launch/configuration document with absolute paths and generic literals/environment; it has no engine-prepared effect/receipt port, signature/policy/secret-reference contract, or artifact-to-exec race protection |
| `rrd-engine::reconcile_engram_store` and `rrd-engram-controller` | one-step reconciliation, stable operation replay, and testable effect-gap boundaries | static arbitrary-path store opening, caller-selected clock/worker/catalogue, direct repository mutation, release debug holds, and a second control-plane executable |
| removed `rrflow-cli` development supervisor | Its identity-safe stop, bounded log tail, port probing, and child-reaping expectations remain represented by narrower installed-lifecycle or process-driver tests. | `supervisor.json`, duplicated startup implementation, manifest/security creation, and `dev` command success paths are absent; retain no replacement private authority. |
| `rrd-server` ready/shutdown marker support | installed-only engine open, listener publication after bind, synced bounded files, and graceful drain intent | unauthenticated file-existence readiness/completion, independently supplied marker paths, and no challenge/plan/process binding; the release initializer is now absent |
| `rrd-server/tests/local_engram_driver.rs` | canonically installs its temporary engram before starting a real child, then covers controller reopen, idempotent same-PID replay, forged-PID denial, graceful-timeout fallback, effect-gap kills, and retained data | rrflowKV-only private controller state, marker authority, debug controller dependence, and no public-operation, authenticated-readiness, resource, distribution, or cross-platform proof |

The superseded flat record cited
[GitHub Actions run 32667681611](https://github.com/EonsofStupid/rrflow/actions/runs/32667681611)
as Ubuntu, Windows, and macOS characterization of that implementation. It does
not qualify the target adapter; release evidence must be tied to the final
source, exact distribution, platform versions, and acceptance corpus.

## Direct-convergence sequence

1. A-07 freezes the pure effect port, adapter/package/module/binary names,
   process and artifact identities, plan/receipt schemas, causal links, and
   every caller/test/fixture disposition.
2. Move host implementation into the one outward local-process adapter and
   make `rrd-engram` pure. Remove the second CLI supervisor, deployment-
   catalogue binary, static engine store opener, and controller binary rather
   than wrapping or aliasing them.
3. D-01 makes the signed, repository-contained distribution plus install
   preview/apply the sole source of artifact, configuration, locator, service,
   and launch-plan state. The current source has removed `rrd-server
   initialize` and every product startup-creation path; distribution/service
   qualification remains open.
4. C/D persist prepared effects and accepted receipts through the installed
   `RrdEngine`; prove shared rrflowMX semantics and rrflowKV effect-gap
   recovery without a private JSON authority.
5. H/J expose authenticated readiness/control through the public contract,
   enforce diagnostics/resources, and qualify clean offline installation plus
   Linux, Windows, and macOS process identity and failure behavior.
6. Delete every prior successful catalogue/process/marker/supervisor shape.
   Retain old bytes only as fail-closed rejection vectors.

## Focused characterization and missing proof

The current source has focused tests for one hard-link-versus-copy executable
identity check, one installed-only deployment-catalogue generator, controller
argument cases, and four real-process/effect-gap cases over a canonically
preinstalled engram. The shortest replacement checks are:

```text
cargo test -p rrd-engram --lib local_process --locked
cargo test -p rrd-engram --test deployment_catalog --locked
cargo test -p rrflow-cli --bin rrd-engram-controller --locked
cargo test -p rrd-server --test local_engram_driver real_rrd_child_survives_controller_reopen_and_stops_without_data_deletion --locked
cargo test -p rrd-server --test local_engram_driver local_driver_refuses_to_signal_a_reused_or_forged_pid_identity --locked
cargo test -p rrd-server --test local_engram_driver graceful_timeout_reauthenticates_then_uses_the_bounded_kill_fallback --locked
```

These tests do not prove the target ownership boundary, engine-prepared
launch-plan resolution, signature or secret handling, artifact-to-exec race safety,
authenticated readiness, marker rejection, bounded diagnostics/resources,
rrflowMX/rrflowKV semantics, public operation equivalence, clean packaging, or
cross-platform final behavior. The separate full process-kill matrix repeatedly
terminates the controller around effect gaps; it is fault-injection evidence,
not a product-readiness workload or performance benchmark. Passing any of these
cannot close the associated POA&M row or a release gate.

## Acceptance

The local process adapter is implemented only when a clean offline
distribution installs one project engram, launches the exact authenticated
artifact without a shell or parallel initializer, and reaches an authenticated
RRD readiness receipt for the expected project/engram/instance/configuration;
the same non-durability-specific effect corpus passes through rrflowMX and
rrflowKV; rrflowKV converges after every prepared/effect/receipt crash boundary;
PID reuse, path replacement, image replacement, forged readiness/completion,
foreign process, stale fence, timeout, disk pressure, and secret/log leakage
tests fail safely; graceful and forced stop preserve data; Linux, Windows, and
macOS evidence is recorded; and no deployment catalogue, process-state JSON,
marker-file authority, startup initializer, duplicate supervisor, arbitrary-
path controller, or direct storage access remains.
