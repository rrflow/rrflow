# RRFlow local-estate authorization

**Status:** active implementation reference; direct convergence into the canonical security and transaction path remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/security/local-estate-authorization`
**Owner:** authorization specialization for local estate administration, subordinate to the RRFlow security authority

“Local” describes how an operator reaches an RRFlow instance. It does not
create another identity, permission, policy, session, audit, storage, or
transaction authority. The [security authority](authority.md) owns those
semantics for every adapter. This record preserves the useful constraints in
the current local estate path, exposes where that path bypasses the accepted
authority, and defines its direct-convergence target.

Estate desired/observed state, reconciliation, backup, and recovery semantics
belong to the [estate-control boundary](../operations/estate-control.md). They
are summarized here only where authorization must bind a specific semantic
effect.

## Accepted engine flow

An installed local operation must follow the same engine path as an equivalent
HTTP, WebSocket, SDK, MCP, or future mesh-reached operation:

```text
typed estate operation + installed instance coordinate + credential reference
  -> adapter resolves one configured RrdEngine; it cannot select a database
  -> RrdEngine authenticates one canonical principal at engine-observed time
  -> capture policy revision/digest and exact instance/estate resource
  -> authorize both the operation and every requested estate/filesystem effect
  -> capture one transaction stamp and validate estate preconditions
  -> commit estate state + indexes + audit + outbox + cursor atomically
  -> execute any fenced external effect and durably reconcile its outcome
  -> return the same typed result and correlated evidence on every surface
```

A new database has no ordinary estate-administration bypass. Initial trust is
established only by the previewed, digest-bound `rrflow install` apply plan in
D-01. That plan creates or binds the instance, security authority, principal,
credential reference, and initial grants before normal operations become
available. Denial must still occur before an unauthorized target or secret is
created.

## Current file-policy foundation

The current `rrd-estate::LocalOperatorPolicy` is a strict JSON file with these
implemented validations:

| Field or boundary | Current behavior | Target disposition |
|---|---|---|
| `format` | exactly `1` | Remove the independent policy format after its fields are represented by canonical installation and security records. |
| `operator_id` | one canonical identifier | Resolve to the same revisioned RRFlow principal used by every other adapter. |
| `key_sha256` | lowercase SHA-256 for one exact 32-byte file | Replace the package-local credential scheme with a configured secret/credential adapter and canonical credential revision; never persist raw bytes. |
| validity | inclusive `not_before_unix_ms`, exclusive `expires_at_unix_ms` | Evaluate against engine-observed time, not caller-supplied `--at`. |
| estate grants | 1 through 1,024 exact canonical estate IDs, each with a non-empty permission set | Compile into canonical grants over an exact instance/estate resource path. |
| JSON/file limits | unknown fields denied; file must be regular and at most one MiB | Preserve bounded strict decoding in the install/config adapter. |
| local file protection | Unix rejects group/world mode bits; non-Unix performs no metadata check | Require provider-neutral secret references plus platform-qualified ownership, symlink/race-safe opening, ACL, rotation, and revocation evidence. |

Authorization currently rejects an unknown estate, ungranted action, invalid
window, malformed key digest, non-32-byte key, or mismatched key. It has no
wildcard estate or wildcard permission. The policy-derived operator identity,
not an independent CLI actor argument, becomes the estate journal actor.

These are useful fail-closed properties. They do not make the JSON file a
permitted source of policy truth. The policy has no persisted identity,
revision, authority-instance binding, configuration digest, revocation record,
or relationship to the canonical `SecurityState` observation used by normal
engine operations.

## Implemented permission/effect inventory

The seven local permissions are real least-privilege distinctions and must be
absorbed before the independent enum is removed:

| Current permission | Current operation | Semantic effect that canonical authorization must bind |
|---|---|---|
| `create` | `rrd-estate-admin create` | create the exact estate aggregate under the installed authority instance |
| `set_desired` | `rrd-estate-admin set-desired` | change one managed instance's desired phase, deployment, version, and configuration digest |
| `schedule_backup` | `rrd-estate-admin schedule-backup` | create one backup job only for a desired/observed-stopped instance at the same generation with no process ID |
| `manage_recovery_policy` | `rrd-estate-admin set-recovery-policy` | replace bounded RPO, RTO, minimum-point, and retention policy for one managed instance |
| `manage_recovery_holds` | `rrd-estate-admin pin-recovery-point` and `release-recovery-pin` | create or release an explicit hold on a known, unpruned recovery point |
| `prune_recovery` | `rrd-recovery-controller prune` | commit and reconcile one revision/catalogue-bound retained-versus-pruned partition; caller supplies no file list |
| `restore_recovery` | `rrd-recovery-controller restore` | restore one known recovery-point identity below the fixed installed restore root and record closure plus measured RPO/RTO evidence |

The canonical action/effect vocabulary may group only effects that retain this
least privilege. The current single `SecurityAction::EstateAdmin` audit label
cannot replace these distinctions by itself. Each retained operation needs a
typed request, exact resource/effect plan, denial corpus, and one mapping in the
public operation registry; speculative permission strings remain forbidden.

## Current adapter and engine path

`rrd-estate-admin` exposes six commands and `rrd-recovery-controller` exposes
prune and restore. Both accept raw database, authority-instance, policy, key,
estate, time, request, and operation coordinates; recovery and the backup
controller also accept a raw state root and the distribution executable, and
resolve the managed instance below that root through its installed locator
rather than a caller-named catalogue or store path. The adapter calls static
`RrdEngine::*_store` functions rather than an operation on an already resolved
engine and authenticated invocation.
There is no HTTP estate-mutation route or shared public-surface conformance
corpus today.

Current administration returns `EstateAdminResult`, wrapping
`EstateMutationResult`, `EstateBackupMutationResult`, or
`EstateRecoveryMutationResult`. These expose the public estate or recovery
projection and an `idempotent_replay` flag; backup scheduling additionally
returns its job projection. Desired, backup, and recovery replay use aggregate
idempotency bindings. Create replay instead scans at most 65,536 control
journal entries for an exact estate key, actor, request, operation, and time;
an older ambiguous match fails rather than creating again. The typed result and
fail-closed replay behavior should remain, while its control-log lookup must
converge on the final bounded transaction/idempotency index.

For ordinary administration, the static function loads the policy and key
before `RrflowKvStore` can create the database. The black-box test proves an
ungranted estate does not create the supplied database, and that property must
survive installation-based resolution. After authorization, however, the
function opens a new local engine from the caller's path, constructs
`EstateRepository` directly over its storage, writes an `EstateAdmin`
authorization audit record, performs the repository mutation, and writes a
separate completion record.

Prune and restore repeat the independent file authorization and use multiple
estate, operation-state, physical catalogue/filesystem, and audit transitions.
They contain useful fences: fixed direct directories, exact catalogue and
estate revisions, durable prepared/completed operation records, recovery-point
holds, watermarks, closure verification, idempotent replay, and rejection of a
divergent existing restore. Those mechanics remain valuable, but they are not
one atomic semantic mutation and their authorization is not bound to their
operation state or filesystem effect receipts.

## Proven dual authority

The current `one_authority_coordinates_security_data_catalogues_audit_and_reopen`
test actually demonstrates two authorities:

1. It bootstraps canonical `SecurityState` for `authority-operator` without an
   `EstateAdmin` grant.
2. It separately writes `LocalOperatorPolicy` for `operator-one` with only the
   package-local `create` permission.
3. `RrdEngine::administer_estate_store` successfully creates and replays the
   estate without `begin_invocation`, a canonical session, or evaluation of
   `SecurityAction::EstateAdmin`.

The resulting audit record is useful evidence of what happened, but appending
an allowed audit record is not authorization. Its action is broader than the
permission checked, and it carries no local policy identity, revision, digest,
credential revision, engine read/transaction stamp, or effect-plan digest.

## Direct-convergence requirements

1. Move the seven implemented permission distinctions into the canonical
   security operation/effect vocabulary and remove `LocalEstatePermission`,
   `LocalOperatorAuthorization`, and `LocalOperatorPolicy`; do not retain a
   wrapper or compatibility reader.
2. Remove `rrd-estate`'s authority to load credential/policy files. It owns
   pure desired/observed estate values and validation only; persistence and
   authorization are composed by `RrdEngine` through the accepted transaction
   port.
3. Replace caller-selected `--db`, `--authority-instance`, policy path, key
   path, state root, and authorization timestamp with an installed instance
   binding plus explicit operation input. An adapter may select a configured
   credential reference, never redefine its policy or target root.
4. Authenticate the canonical principal and compile its exact grant before
   state access. Bind the principal/credential revision, policy revision and
   digest, authority instance, estate, managed instance, operation, semantic
   effects, schema/catalogue revision, and transaction stamp.
5. Preserve denial-before-creation through the D-01 bootstrap trust boundary.
   Ordinary estate creation occurs only inside an initialized engine; fresh
   instance creation is a separately authorized install apply.
6. Commit accepted estate state, indexes, audit, outbox, idempotency, and cursor
   through one rrflowMX/rrflowKV semantic transaction. External process,
   backup, prune, and restore effects use durable intent/receipt reconciliation
   and compensating recovery rather than pretending filesystem work is atomic.
7. Run the same non-durability-specific estate authorization and mutation
   corpus through rrflowMX and rrflowKV. Only durable backup/restore operations
   may reject rrflowMX, and that rejection must be an explicit capability
   outcome rather than an alternate semantic authority.
8. Expose equivalent typed operations through CLI and every declared public
   surface. Local, remote, mesh, SDK, MCP, and Connectome callers receive the
   same decision, result, stamp, and redacted causal evidence.

## Current evidence and missing proof

| Evidence | What it proves | What it does not prove |
|---|---|---|
| `rrd-estate/tests/local_authorization.rs` | one valid key/estate/action/time combination and denials for two actions, another estate, and expiry | strict JSON loading, file bounds/ownership, key mismatch, time provenance, canonical security integration, or cross-profile behavior |
| `rrflow-cli/tests/estate_admin.rs` | denial before database creation; successful create/desired/recovery-policy/backup replay; rrflowKV reopen; journal actor | canonical `SecurityState` authorization, every permission, crash atomicity, rrflowMX equivalence, install resolution, or another public surface |
| `rrd-engine/tests/engine_authority.rs` | current local create/replay and fenced prune/restore behavior alongside other engine features | one authorization authority; the test currently proves the opposite because canonical security does not grant the successful estate creation |
| `rrd-estate --all-targets` | current bounded estate/reconcile/backup/recovery behavior on selected MX/KV cases | target authority convergence; it also contains successful older-shape tests tracked separately in the POA&M |

Closure is owned by A-07, C-02/C-03, D-01/D-06, H-04/H-05, and J-01 through
J-03/J-05. Passing the current tests is characterization evidence only.

## Implementation anchors and focused characterization

- File policy and permission enum:
  `crates/authority/rrd-estate/src/local_authorization.rs`
- Estate aggregate, backup, and recovery behavior:
  `crates/authority/rrd-estate/src/{lib,backup_job,recovery}.rs`
- Current engine-owned-looking but bypassing entry points:
  `crates/authority/rrd-engine/src/engine/estate_control.rs`
- Canonical invocation mapping that those entry points do not use:
  `crates/authority/rrd-engine/src/engine/invocation.rs`
- CLI adapters:
  `crates/adapters/rrflow-cli/src/bin/{rrd-estate-admin,rrd-recovery-controller}.rs`

Focused current-behavior commands are:

```text
cargo test -p rrd-estate --test local_authorization --locked
cargo test -p rrflow-cli --bin rrd-estate-admin --locked
cargo test -p rrflow-cli --test estate_admin --locked
cargo test -p rrd-engine --test engine_authority --locked
```

They do not close the direct-convergence requirements above.
