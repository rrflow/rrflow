# RRFlow security authority

**Status:** active implementation reference; single-engine alpha security convergence remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/security/authority`
**Owner:** identity, credential, session, policy, authorization, and audit semantics enforced through `RrdEngine`

RRFlow Security is a cross-cutting capability of the one RRFlow engine. It is
not an independently authoritative database or service. `RrdEngine` is the
only boundary allowed to authenticate an operation, compile its authorization,
capture its read or transaction stamp, authorize its semantic effects, and
commit state. The `rrd-security` package supplies useful identity, policy, and
audit vocabulary today, but its current public storage repository is
implementation inventory rather than the accepted final authority boundary.

The [system overview](../../architecture/system-overview.md#security-boundary)
owns the architectural security boundary, [ADR-0001](../../decisions/0001-single-engine-authority.md)
owns the single-engine decision, the [public contract](../protocol/public-contract.md)
owns transport-neutral wire shapes, and the [roadmap](../../roadmap/rrflow-1.0.md)
owns completion. This reference records the exact useful behavior to preserve,
the current deviations to remove, and the evidence required before security is
alpha-qualified.

## Authority invariant

Every non-public operation follows one engine-owned sequence:

```text
typed ingress + explicit instance/resource + credential
  -> RrdEngine authenticates identity and session
  -> capture exact policy revision/digest and authorized data constraints
  -> authorize operation and requested semantic effects
  -> capture one read or transaction stamp
  -> plan only over authorized fields, rows, graph paths, and index candidates
  -> execute bounded native or rrflowQL/Arrow/DataFusion work
  -> for writes, validate and encode one atomic semantic batch
       canonical state + indexes/projections + outbox + audit + cursor
  -> commit through the selected rrflowMX or rrflowKV transaction profile
  -> return result, denial, or failure with correlated evidence
```

Public liveness, readiness, catalogue, and capability inspection are explicit
catalogue dispositions, not an accidental result of missing security state.
They never receive mutation authority. A transport, mesh, model, DataFusion
operator, index, hook adapter, SDK, or Connectome cannot authorize itself or
write around this sequence.

## Component responsibilities

| Boundary | Owns | Must not own |
|---|---|---|
| `rrd-contract` | closed public actions, resource paths, request coordinates, audit envelopes, and error vocabulary | credential verification, policy decisions, storage, or a second operation registry |
| `rrd-security` target | bounded principal/role/grant/credential metadata, pure validation and policy compilation, audit value semantics, and cryptographic helpers | opening a physical store, committing control/runtime state, issuing an independent read stamp, or accepting an operation outside `RrdEngine` |
| `RrdEngine` | identity/session authentication, current-policy resolution, authorization, effect validation, stamps, transaction coordination, audit causality, and adapter-independent outcomes | transport parsing or provider-specific identity lifecycle |
| rrflowMX / rrflowKV | equivalent transactional point/range/CAS semantics; rrflowKV additionally owns durability and recovery | interpreting principals, granting actions, or selecting policy |
| rrflowQL, native graph/BM25/vector operators, and DataFusion | execute the already-authorized plan at its exact stamp and budget | widening rows/fields, bypassing filters, changing policy, or committing results |
| HTTP, WebSocket, SDK, MCP, CLI, mesh, and Connectome adapters | validate and carry typed credentials, identities, requests, responses, and TLS evidence | semantic authorization, hidden session state, or provider-owned lifecycle rules |
| External identity or secret provider adapter | verify the provider assertion or supply secret material under explicit configuration | becoming an RRFlow principal authority or persisting credentials into rrflowDB |

## Implemented identity and policy foundation

The current `SecurityState` has an exact format and monotonically replaced
revision. It contains bounded canonical maps for principals, roles,
third-party identity bindings, and JWT issuers. Complete-state replacement uses
compare-and-swap: the next revision must advance exactly once, exact replay is
a no-op, and stale or substituted state is rejected.

A principal is a user, service, or node with a credential-verifier digest,
credential revision, validity window, disabled flag, direct grants, and role
memberships. Roles form a bounded acyclic inheritance graph. A grant binds one
closed `SecurityAction`, one exact canonical resource prefix, and an optional
`DataPolicy`; wildcard strings and allow-on-missing-policy behavior do not
exist. The most-specific matching grants apply, and equally specific grants
with different data policies fail closed.

Current data policy can express one tenant equality predicate, bounded row
equality predicates, and an allowed-field set. Query execution applies those
predicates and projection before binding and physical planning, and its public
plan evidence carries the policy revision and authorization digest. An
operation that cannot enforce a constrained policy denies it rather than
silently widening access.

The policy evaluator itself fails closed when authority is uninitialized, a
principal is absent, disabled, or outside its validity window, a credential
does not match, or no action/resource-prefix grant applies. That behavior is
distinct from the server's current missing-policy loopback branch described
below, which bypasses normal principal authentication and is not an accepted
profile.

That implementation is a foundation, not complete authorization for the target
engine. The current transaction actions authorize a broad transaction
operation, not every table, record, relation, graph edge, field, vector,
projection, routine, or external effect inside the mutation plan. The accepted
write path must authorize those semantic effects after parsing and before one
atomic commit. Native access-path selection, DataFusion pushdown, spill files,
result shaping, explain output, errors, and caches must preserve the same
authorized row/field boundary; F-05 caches must include the authorization
digest and policy revision in identity and invalidation.

## Credentials, identity binding, and sessions

The repository currently implements these concrete mechanisms:

- API-key authentication compares the SHA-256 of supplied bytes against a
  persisted lowercase digest without an early byte-mismatch return; the engine
  helper generates a 256-bit random, owner-only printable key. This mechanism
  is for high-entropy API keys, not human passwords, and raw key material must
  never enter durable state, request logs, traces, errors, or audit.
- RRD-issued JWTs currently use bounded HS256 tokens. Header and claims bind
  issuer, audience, key identity, principal, token identity, time window,
  policy revision, and principal credential revision. Only the signing-key
  digest is policy state; the server receives key material separately.
- Verification reloads current policy, so principal disablement, issuer
  disablement, key mismatch, and credential-revision rotation reject an older
  token. Session state persists the principal credential revision and checks it
  again on later operations and after reopen.
- A third-party identity assertion is usable only after an external adapter has
  cryptographically verified it. RRFlow then resolves the exact persisted
  issuer/subject/audience tuple to a current principal and still applies
  `session_create`. Unsigned identity headers are never sufficient.

Transport authentication and application authorization remain distinct. A
valid client certificate proves the TLS peer accepted by the configured trust
roots; it does not itself select an RRFlow principal or grant. Likewise, mesh
reachability and provider identity cannot substitute for RRD server identity,
RRFlow credentials, or current policy.

The current closed action enum covers session, query, index, transaction,
changefeed/subscription, vector collection/point, backup/restore, estate,
audit, diagnostic, security-administration, and implemented governed-runtime
operations. This is implementation inventory, not permission for speculative
features; new install, attunement, routing, event, trigger, routine, skill, and
adapter actions are introduced only with their typed engine operation and
denial tests.

The current [local-estate authorization](local-estate-authorization.md) path
does not follow this authority. It evaluates a separate file policy and
permission enum, then opens an engine by caller-supplied path and performs
estate mutations without a canonical principal grant or invocation. Its useful
exact-scope and denial-before-creation properties must be absorbed directly;
the local file policy receives no permanent or compatibility status.

The final install path must create or bind all required key material through
the installation trust bootstrap below. Secret-provider integration, rotation,
revocation, and reload behavior are explicit configured adapters with failure
and restart evidence; they are never inferred from a path or deployment
environment.

## Installation trust bootstrap

Initial security authority is part of the canonical installation action. It is
not a separate executable, manifest dialect, database opener, HTTP endpoint, or
Kubernetes lifecycle. `rrflow install plan` resolves the complete action and
`rrflow install apply --plan <file> --expect <sha256>` submits the exact
reviewed digest to
`RrdEngine`. The server cannot listen for application traffic until that action
has committed an installed-estate binding and initial security authority.

An existing *application project* can still be a fresh RRFlow installation.
Cold-start authority is available only while RRFlow can prove, under an
exclusive create-new installation lease, that the target has no installed
binding, security authority, or canonical estate state. Once any installed
binding exists, absence or corruption of policy never re-enables cold start;
repair and policy replacement require an authenticated `security_admin`
operation or an explicit recovery procedure. The cold-start path is local and
privileged by the operator's OS/deployment boundary and is never remotely
callable.

### Previewed installation input

The installation action input digest commits to one normalized,
bounded document containing:

- project, estate, and instance identities plus the selected rrflowMX or
  rrflowKV storage profile and exact placement descriptor;
- distribution, template, configuration, specialization, and schema digests;
- every initial principal, principal kind, role membership, inherited role,
  closed action, resource prefix, and data/effect constraint;
- each credential's purpose, verifier scheme and parameters, revision,
  validity policy, rotation policy, source kind, and opaque source revision or
  version; and
- the installation identity, idempotency coordinates, resource limits, and
  expected subsequent locator and attunement-plan digests.

Preview renders those semantics for human and machine review. It does not open
rrflowKV, generate or read a credential, create a file, contact a provider, or
start RRD. Apply may resolve only a bundle-resident input or an explicitly
configured credential adapter named by that plan. A principal or grant absent
from the reviewed input cannot appear during apply, and a changed configuration,
source revision, adapter binding, template, placement, or action digest is a
conflict requiring a new preview.

### Credential sources and sinks

The provider-neutral engine sees credential-source descriptors and a bounded
secret-input capability, never an arbitrary host path. The initial alpha must
support engine-generated high-entropy API-key material; imported local files,
Kubernetes projected Secrets, hardware devices, and external secret managers
are optional outward adapters behind the same source contract. Their provider
names and filesystem conventions do not enter canonical security state.

For a file adapter, the plan names a normalized relative entry beneath an
explicitly opened capability root. The adapter opens the entry relative to
that directory handle, rejects traversal and unsupported link/mount behavior,
then validates type, identity, ownership/ACL policy, and size on the opened
handle before returning a bounded reader. A `canonicalize`-then-open sequence
is insufficient because the path can be replaced between those operations.
Kubernetes projected-volume symlinks and `fsGroup` rules are handled and tested
only by the Kubernetes adapter; a Windows adapter must validate ACLs rather
than silently accepting every file.

Generation uses the operating system cryptographic random source. Raw secret
bytes never enter a plan, command argument, environment variable, locator,
canonical record, audit event, trace, error, status response, or stdout. Apply
delivers generated material only to the explicitly previewed create-new secret
sink or provider handoff, with bounded permissions and durability evidence.
Buffers holding secret bytes are fixed-capacity where practical, are not
cloned for logging or serialization, and are zeroized immediately after
verifier construction and delivery.

Persisted verifier state is typed and versioned by credential kind. A
high-entropy machine API key may use a constant-time checked keyed cryptographic
verifier with its key held outside the policy record. A human-memorable secret
must use a salted, costed password-hashing scheme; plain SHA-256 is never a
password verifier. The current raw SHA-256 field is therefore retained only as
characterization for already generated 256-bit API keys and must be replaced
by the final typed verifier contract before security qualification.

### Commit, replay, and failure boundaries

`RrdEngine` obtains installation time through one engine-owned, versioned clock
boundary; public installation callers do not supply that time. Other runtime
operation timestamps remain a separate convergence problem and are not claimed
by this package as engine-owned.
The implemented source is deliberately classified `host_system_time` /
`unverified`. Its first fallible observation is sealed before project effects,
and recovery, replay, and installed open reject an unavailable observation or
rollback beyond the installed configuration's explicit bound. This provides a
relative rollback guard; it is not proof of correct UTC or authenticated
synchronization.
The accepted installation transaction atomically commits the installed-estate
binding, initial policy/verifier metadata, exact plan/action identity,
installation checkpoint, security-administration audit, outbox/commit evidence,
and authoritative cursor. rrflowMX and rrflowKV execute the same semantic
transaction; only rrflowKV can satisfy the durable installation and reopen
outcome.

An exact retry returns the committed receipt and never regenerates, rereads, or
redelivers a credential merely to reconstruct state. Reuse of a plan identity
with different input, a changed credential-source revision, a pre-existing
unowned sink, partial authority state, stale lease, or target identity mismatch
fails closed. External credential generation/delivery follows the same
prepared-effect-observation-receipt discipline as other non-transactional
effects; its bounded staging and cleanup are install-job state, never a second
security database.

D-01/D-02 tests must interrupt before and after plan acceptance, credential
prepare, secret delivery, engine commit, locator publication, attunement-job
creation, acknowledgement, and cleanup. For each boundary they must prove one
credential outcome, one policy and audit outcome, exact replay or explicit
recoverable failure, no open application listener before authority is ready,
no plaintext in every persisted/logged/error artifact, and successful rrflowKV
close/reopen authentication. Separate tests cover path replacement, symlink
and mount escape, oversized/empty/non-regular inputs, Unix modes, Windows ACLs,
Kubernetes projected volumes, provider revision drift, cancellation, and
revocation.

Clock diagnosis does not authorize clock correction. A future correction
operation must be a separately named governed external effect: preview the
platform command or service, time source and trust, network use, required
privilege, maximum step/slew and rollback impact; require an applicable grant;
then bind observation and result into a receipt. Unknown sources or effects
remain representable for analysis but cannot execute. Durable ordering and
authorization continue to rely on engine transaction/read/journal coordinates,
not on a model-authored timestamp or a silently adjusted host clock.

### Current installation disposition

The standalone `rrd-security-bootstrap` helper/binary, its manifest shape and
tests, the CLI supervisor call, and the Kubernetes bootstrap container are
removed. The canonical local installer now generates one high-entropy API key
and token key, freezes their exact bytes plus the first engine-owned clock
observation in an owner-only,
plan-addressed recovery intent, creates or verifies their owner-only files, and
atomically commits `SecurityState`, security audit, installed binding,
configuration, install checkpoint, attunement plan/job, outbox, and cursor
before publishing the locator. Deterministic failure injection at the intent,
directory, store, token, credential, commit, locator, and acknowledgement
stages proves exact retry without another credential, policy, or audit outcome.
Malformed, foreign, mismatched, symbolic, partly committed, missing-secret, or
unsafe-time state fails closed.

That is a bounded local pre-release result, not final secret qualification. The
recovery intent necessarily contains serialized raw secret staging bytes until
acknowledgement; it is private, size-bounded, digest-bound, redacted from
`Debug`, and removed on success, but it has not passed native Windows ACL,
crash-residue secret accounting, memory zeroization, provider handoff, rotation,
or handle-relative path-replacement proof. Public installation no longer
accepts `at_unix_ms`: the engine persists a format/source/trust/coordinate
observation, deterministic private tests cover unavailable, strict, tolerated,
and exceeded rollback, and read-only inspection preserves the diagnostic even
when open must fail. Absolute trust, platform synchronization/correction,
runtime-wide temporal high-water rules, and native qualification remain open.
The target capability-scoped secret source/sink, trusted-time upgrade, and
prepared-effect receipt rules above still govern D-02/D-06/J.

## Policy and data stamp

Authorization must be reproducible at the same logical observation as the
data it protects. An accepted read binds at least:

- instance, principal, credential revision, operation, and resource;
- policy revision plus a digest of the compiled row/field/effect constraints;
- schema and catalogue revisions;
- the rrflowKV or rrflowMX `ReadStamp` and valid-time coordinate;
- logical and physical plan digests, resource budget, and result evidence.

The current implementation loads JSON security state from the control keyspace
and then obtains the runtime data stamp separately. Its query result records a
policy revision and digest, but there is no atomic policy/data snapshot that
prevents a concurrent policy replacement between authorization and the read.
This is not sufficient evidence for the target. C-02 through C-04 must provide
one accepted transaction/snapshot relationship for policy, semantic data,
indexes, and audit on both profiles, with rrflowKV reopen and conflict proof.

## Audit and causal evidence

The implemented `AuditRecord` is immutable and binds audit identity, time,
optional principal, closed action, resource, request/operation coordinates,
authorization or completion phase, allow/deny/fail decision, status,
request/response digests, previous audit digest, and its own digest. It excludes
bodies, credentials, bearer tokens, signing keys, filesystem paths, and
arbitrary headers.

Audit records form an independent genesis-to-head chain. Record and head are
published atomically in one bounded control batch; concurrent head conflicts
retry within a fixed bound. Bounded read and canonical JSON Lines export expose
the global scanned control sequence plus chain anchor/head so a consumer can
validate lineage and make progress across unrelated control activity. Existing
tests cover exact replay, collision, concurrent linearity, head substitution,
reopen, redaction, protected read, and export validation.

The current value validator permits an authorization record only as
`authorized/allowed` with status 100 and requires completion records to carry a
final status and allowed, denied, or failed decision. Audit read and canonical
JSON Lines export require their separate actions; export verification parses
every line and rejects record-count, content-digest, framing, or chain
substitution.

An invocation currently appends `authorized` before executing and appends
`completed` after the outcome is known. A missing completion is therefore
visible, but authorization, domain mutation, and completion audit are separate
commits. A domain write can succeed before completion audit fails, and audit
does not yet carry the complete policy/read/plan/commit coordinates required by
the final causal trace. C-03 must include the accepted write's audit and outbox
in the same semantic batch; H-05 must correlate ingress, authorization, stamp,
native/DataFusion work, commit, and delivery without turning trace state into
authority. A denial or pre-execution failure remains an audit-only engine
transaction.

Current security-state initialization/replacement does publish its state and
`security_admin` completion audit in one control batch. Estate administration,
reconciliation, backup reconciliation, retention pruning, and restore recovery
also emit `estate_admin` phase records. Session-backed embedded work attempted
outside an active invocation writes an authorization reservation or terminal
denial so it cannot disappear entirely. These useful causal safeguards must be
absorbed into the final semantic transaction; they do not make the ordinary
three-commit invocation path atomic.

## Transport boundary

The server currently rejects cleartext non-loopback binding. Its remote server
configuration fixes TLS 1.3, requires a server certificate and client trust
roots, and uses a client-certificate verifier. A real client/server test proves
missing-client-certificate rejection, server-name rejection, authenticated
HTTPS, and WSS delivery.

This does not prove production transport security. Certificate rotation,
revocation, trust-root reload, external identity verification, rate limiting,
deployment-specific cipher and key policy, and mesh endpoint rotation remain
unqualified. The server currently permits anonymous application sessions and
operations on loopback whenever no security state exists. That absence-driven
development mode contradicts the final always-explicit authority invariant and
must be replaced by an explicit configured profile/bootstrap outcome; only
catalogued public inspection may remain anonymous.

## Current implementation deviations

The complete review found these direct-convergence requirements:

1. `rrd-security` depends on `rrd-store` and exports a `SecurityRepository`
   that can commit directly to any `StorageEngine`. A-07 must retain its useful
   values and pure decisions while making `RrdEngine` the only caller that can
   persist or replace authority and audit state.
2. Policy, sessions, and audit are JSON values under `server/state/*` control
   keys with a control-journal coordinate separate from runtime data. C-01
   through C-04 must give them the accepted binary key/transaction/snapshot
   relationship without adding another database or compatibility reader.
3. Query row/field policy is implemented, but mutation-effect authorization,
   graph/vector/index side-channel constraints, policy-aware caches, and one
   policy/data stamp are not.
4. The missing-policy loopback branch grants anonymous application operation;
   it is current characterization, not an accepted pre-release fallback.
5. Initial product security now has one exact-plan installation path and the
   standalone bootstrap is absent. Its current private recovery intent stages
   raw secret bytes on disk, Unix privacy checks are not native Windows ACL
   qualification, and external capability-scoped secret sources/sinks plus
   prepared delivery receipts are not implemented.
6. Authorization and completion audit are not atomic with domain mutation;
   audit values also lack the final stamp, plan, effect, and commit evidence.
7. Current actions cover existing endpoints but not all planned install,
   attunement, routing, engine-event, trigger, routine, skill, and adapter
   operations. Their vocabulary must expand only with the owning implemented
   public operation, never as speculative permission strings.
8. `rrd-estate::LocalOperatorPolicy` is a second file-backed identity and
   permission authority. Estate administration and recovery accept
   caller-selected database/policy/key/time coordinates, bypass canonical
   sessions and `RrdOperation::EstateAdmin`, and collapse narrower permissions
   into a broad allowed audit label. A-07 must absorb its seven real effect
   distinctions into this authority and delete the parallel types and path.

These are tracked by the [POA&M](../../poam/rrflow-1.0-alpha.md). The completed
installation convergence does not silently close them or create a parallel
pre-release security mode.

## Evidence and acceptance map

| Evidence or gate | Proven now or required outcome |
|---|---|
| `rrd-security` tests | Current validation, exact-scope denial, role inheritance failure, API-key/JWT/identity binding, rotation/revocation, audit chain, concurrent append, and rrflowKV reopen behavior. The suite is not an engine-authority or MX/KV semantic differential. |
| `rrd-engine` security tests | Current engine invocation, embedded denial, credential-revision invalidation, query policy injection, context fail-closed behavior, and audit phases. They do not prove atomic policy/data stamps or effect-level write authorization. |
| Server/client real-process tests | Current API-key sessions, denials, redaction, audit read/export, TLS 1.3 mTLS identity, authenticated HTTP/WSS, and selected reopen behavior. They do not qualify every adapter, provider, rotation mode, or deployment. |
| A-07 | Remove the direct-store/public-repository authority, the retired provisioning-role vocabulary, ambiguous action vocabulary, and dependency-direction conflicts while mapping every retained invariant to its one owner. |
| C-01 through C-04 | Encode and transact security/audit state through the one accepted rrflowMX/rrflowKV substrate; bind policy and data observations; atomically include audit with accepted writes. |
| D-01/D-02/D-06 | D-01 now proves exact local preview/apply, fresh-target checks, one engine-generated credential/token, atomic binding/policy/audit/checkpoint commit, eight-stage same-plan replay, and an engine-owned unverified clock anchor with bounded rollback diagnosis/enforcement. Complete capability-scoped secret I/O, typed verifiers, delivery/rotation receipts, native ACLs, trusted-time/correction qualification, external identity providers, and optional adapters without copying credentials into canonical state. |
| F-01 through F-05 | Prove native and DataFusion plans see only authorized stamped batches, enforce budgets, and never return stale authorization-scoped cache entries. |
| H-04/H-05/H-07 | Prove cross-surface authorization equivalence, complete redacted causal evidence, RRD TLS identity, and mesh independence. |
| J-01 through J-05 | Remove superseded branches and pass denial, crash/reopen, resource, clean-install, secret-accounting, and deployment qualification before release. |

## Implementation anchors and focused verification

- Policy, identity, JWT, audit values, and current direct repository:
  `crates/authority/rrd-security/src/lib.rs`
- Sole accepted composition boundary and invocation policy:
  `crates/authority/rrd-engine/src/engine/{security,session,invocation}.rs`
- Canonical installation and private recovery staging:
  `crates/authority/rrd-engine/src/engine/{installation,installation/recovery,token_key}.rs`
- HTTP credentials and invocation wrapping:
  `crates/transport/rrd-server/src/http/{auth,envelope,server}.rs`
- Public action and audit shapes:
  `crates/transport/rrd-contract/src/lib.rs`

Focused characterization starts with:

```text
cargo test -p rrd-security --all-targets --locked
cargo test -p rrd-engine --lib engine::tests::security --locked
cargo test -p rrd-server --test http_process initialized_security_authority_binds_sessions_and_denies_ungranted_routes --locked
cargo test -p rrd-client --test real_server remote_transport_requires_mutual_tls_and_exact_server_identity --locked
```

Those commands characterize present behavior only. Alpha acceptance additionally
requires the gate-specific semantic, differential, failure, resource,
cross-surface, and clean-deployment evidence above.

## Engineering sources

The target requirements above are grounded in the
[OWASP secrets-management lifecycle and least-privilege guidance](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html),
[NIST SP 800-63B verifier guidance](https://pages.nist.gov/800-63-4/sp800-63b.html),
[cap-std capability-relative filesystem model](https://docs.rs/cap-std/latest/cap_std/),
[Linux `openat2` resolution controls](https://man7.org/linux/man-pages/man2/openat2.2.html),
[RustCrypto zeroization guarantees and limits](https://docs.rs/zeroize/latest/zeroize/),
[Rust `SystemTime` fallibility](https://doc.rust-lang.org/stable/std/time/struct.SystemTime.html),
[NTP deployment guidance in RFC 8633](https://www.rfc-editor.org/rfc/rfc8633.html),
[Network Time Security in RFC 8915](https://www.rfc-editor.org/rfc/rfc8915.html),
and
[Kubernetes Secret least-privilege guidance](https://kubernetes.io/docs/concepts/security/secrets-good-practices/).
These sources constrain implementation and tests; none is imported as an
RRFlow runtime or authority.
