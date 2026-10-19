# RRD HTTP and WebSocket server

**Status:** active implementation reference; alpha protocol convergence remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/protocol/server`
**Owner:** RRD HTTP/WebSocket process boundary, endpoint presentations, transport security, route discovery, and current conformance

RRD is RRFlow's embedded and daemon runtime. The `rrd-server` package exposes
public HTTP and WebSocket operations over the same `RrdEngine` used by
embedded callers. It is not a second engine, an HTTP wrapper around the CLI,
or an extension of the MCP protocol. HTTP, WebSocket, SDK, MCP, CLI, and
Connectome are clients of the engine's public contracts and own no persistence
or lifecycle semantics.

The [system overview](../../architecture/system-overview.md) owns component and
authority boundaries. The
[engine data-flow record](../../architecture/engine-data-flow.md) owns target
transactional, query, Arrow/DataFusion, context, and trace flow. The
[deployment-profile reference](../deployment/modes.md) owns the independent
deployment-form, storage-profile, and endpoint-presentation classification and
its cross-profile conformance. The
[release roadmap](../../roadmap/rrflow-1.0.md) owns implementation order and
completion evidence. This record describes the server that exists now and
names the gaps without promoting them to completed behavior.

## Executable protocol authority

`rrd-contract::endpoint_catalogue()` is the authoritative public operation
catalogue. Every descriptor binds an operation name, HTTP method and path,
authentication mode, mutation classification, security action, and public
request and response types. The server router resolves operations from that
catalogue, and a router test requires the catalogue and executable dispatch
sets to match exactly.

Two unauthenticated discovery operations project the same authority:

- `GET /v1/schema/endpoints` returns the sorted runtime catalogue.
- `GET /v1/schema/openapi` returns deterministic OpenAPI 3.1 generated from
  the catalogue and public Rust wire types.

This record deliberately does not copy the complete endpoint list. Adding or
changing an operation starts in `rrd-contract`, updates generated surfaces,
and must pass catalogue/router/OpenAPI/SDK parity. Documentation cannot make an
unimplemented route public.

Every JSON operation returns `ResponseEnvelope`; failures use the closed
`ErrorCode` vocabulary. Request envelopes carry request and operation IDs, an
optional deadline, an operation-specific resource path, and an idempotency key
for mutation. Public payloads are contract types rather than private kernel or
storage types.

## Client bootstrap

A client discovers one configured instance in this order:

```text
GET /v1/health/live
GET /v1/health/ready
GET /v1/capabilities
GET /v1/schema/endpoints   # when operation discovery is required
GET /v1/schema/openapi     # when a generated HTTP description is required
```

Liveness proves only that the process answers. Readiness opens the configured
instance and checks its current format and operational state. Capabilities
characterize the installed profile and supported operations; they are not
release evidence. The current response carries independent deployment form,
storage profile, and endpoint presentation plus the installed identity and
effective configuration. Endpoint presentation comes from the listener that
the process actually bound; TLS remains a separate security property and does
not select storage or deployment form. Connectome and SDKs must reject an
unexpected protocol version or instance resource rather than inferring
compatibility.

The accepted bounded D-01 bootstrap is the source-built `rrflow install plan`,
`rrflow install apply`, `rrflow serve`, authenticated `rrflow ready`, and
read-only `rrflow verify --level quick` flow. Planning previews and seals the
canonical estate and configuration; apply consumes those sealed bytes; serve
opens that installed identity read-only before composing the listener. This is
pre-release implementation evidence, not complete D-01 or release evidence.
The internal `rrd-server` process likewise requires an installed project plus
the exact distribution executable and calls `RrdEngine::open_installed`;
`initialize`, raw database-root, manifest, and private-binding startup are
absent. Low-level generic server composition remains available only for
explicit component tests and embeddings that already own a composed engine. The
[local-process adapter reference](../deployment/local-process-driver.md) owns
the exact target launch, authenticated readiness, and shutdown boundary.

## Endpoint presentations and transport security

| Presentation | Current enforced boundary | Still open |
|---|---|---|
| Loopback HTTP/WebSocket | Clear HTTP may bind only to an explicit loopback address. Library and binary startup reject non-loopback cleartext before opening a listener. | Released installation, service supervision, and client conformance remain Gate D/J work. |
| Configured network HTTP/WebSocket | A non-loopback listener requires TLS 1.3, a server certificate and key, a client CA, mandatory client-certificate validation, and initialized RRFlow security state. | Certificate reload, revocation handling, external identity-provider/JWK adapters, and deployment-secret integration are not complete. |
| WebSocket | The authenticated generic `/v1/ws` endpoint configures validated message/frame/buffer/write limits before upgrade and carries the closed B-04 protocol. One connection multiplexes bounded requests, cancellations, durable subscriptions, ACKs, heartbeats, errors, and backpressure with exact connection/sequence/generation/cursor correlation. | H-03 must make delivery commit-impact driven; H-04 must bind generic operations and real cancellation to the shared dispatcher; other SDKs and release deployment remain unqualified. |

The current explicit configured-network invocation supplies all TLS inputs
together:

```text
rrd-server --project PROJECT --distribution-executable /installed/rrflow \
  --bind 0.0.0.0:9477 \
  --tls-cert SERVER_CHAIN.pem --tls-key SERVER_KEY.pem \
  --tls-client-ca CLIENT_CA.pem \
  --jwt-key-file JWT_SIGNING_KEY
```

The current mutual-TLS loop serves HTTP/1.1 and handles one request per
connection. This is an implementation limit, not the target connection model.
A mesh may resolve and carry the endpoint, but mesh reachability does not
authenticate the RRD instance or grant an engine capability.

## Request boundary

An ordinary HTTP request follows this implemented sequence:

```text
bounded body and route
        |
RequestEnvelope validation
  content type, IDs, instance resource, deadline, idempotency
        |
credential or session extraction
        |
RrdEngine::begin_invocation
  authentication + exact action/resource authorization + audit reservation
        |
typed RrdEngine operation
        |
RrdEngine::complete_invocation
  outcome + response digest
        |
ResponseEnvelope
```

The HTTP boundary reads at most one MiB, rejects an elapsed deadline before
the operation, and rejects a resource that does not target the configured
instance. Authenticated routes require a matching session ID and bearer lease.
On a secured instance, session creation accepts a principal/API key or a
configured RRD-issued JWT; each later operation rechecks the stored principal,
credential revision, exact action, and resource policy through `RrdEngine`.

An explicit low-level component engine with no security authority still
advertises an anonymous loopback mode. A product process cannot reach that
state through startup because canonical installation commits security before
locator publication and installed open requires it. Configured-network TLS and
JWT configuration require initialized security. The component-only branch is a
tracked authorization deficiency, not a product installation fallback.

The installed token derivation secret is generated once from operating-system
entropy during installation, recovered exactly across retry, and
permission-checked on Unix. Installed open resolves it through the locator; the
optional JWT-key file is read under bounded file rules. Persisted session and audit records
store digests and credential revisions, not raw API keys, JWTs, signing keys,
or bearer lease tokens.

The WebSocket upgrade separately authorizes `websocket_connect`, sends one
`connected` frame containing the session identity and negotiated limits, and
then applies a direction-specific contiguous frame state machine. Attaching a
durable subscription separately authorizes `subscription_connect` and its
underlying changefeed or live-query action. The socket can detach a stream but
cannot close its durable record; administrative close remains the catalogued
HTTP mutation. B-04 freezes generic request and cancellation correlation, but
the server truthfully returns `failed_precondition` for operation execution
and terminal `not_found` for cancellation until H-04 binds the shared
dispatcher. No transport frame or connection sequence enters `RrdEngine`.

## Public operation families

The generated catalogue currently covers these capability families:

| Family | Server responsibility | Semantic authority |
|---|---|---|
| Inspection | Liveness, readiness, capabilities, endpoint catalogue, OpenAPI, and diagnostics. | `RrdEngine` and public contract projections. |
| Identity and control | Session create, renew, close, estate read, audit read, and capability administration. | RRFlow security and `RrdEngine`. |
| Transactions | Begin, preview, commit, and abort typed mutation work. | `RrdEngine`; rrflowKV owns durable physical commit. |
| Query and context | rrflowQL reads, live-query poll, index administration, and context assembly. | `RrdEngine`, rrflowQL, and the selected storage profile. |
| Prompt flights | Launch one governed prompt flight, list flight records, and read one flight with its events. | `RrdEngine`; flight rows and events are lifecycle data in the canonical runtime log. |
| Recall | Vector collection, point, and search operations. | RRFlow's vector subsystem under `RrdEngine`; never a separate database authority. |
| Delivery | Changefeed read/follow and durable subscriptions over the generic multiplexed WebSocket. | Committed engine state and durable subscription coordinates; WebSocket connection/sequence state remains transport-local. |
| Recovery | Logical backup catalogue, backup creation, and restore into a generated inactive root. | Engine-authorized logical recovery; no active-root switch. |

Existence in this surface proves only that a typed path is callable. Native
graph, scalar, BM25, vector, Arrow/DataFusion, and reasoning-tree execution are
qualified by their owning Gate E through H evidence, not by the route.

## Idempotency and lifecycle durability

Mutation envelopes require an idempotency key. The engine durably binds the
instance, session, key, operation digest, and accepted result identity. An
exact replay returns the accepted result; reuse with a different digest
conflicts. Runtime content identity is an independent guard.

Session and transaction control state is durable rather than process-local.
Lifecycle transitions use compare-and-swap materialization plus a sequenced,
digest-chained journal. Renewal, expiry, close, prepare, commit intent,
terminal commit, and abort have explicit states. A disconnect is not a commit
or abort decision, and cleanup does not delete canonical data.

The public transaction lifecycle brackets the authoritative semantic commit:

```text
durable transaction prepare
        -> authoritative runtime commit and receipt
        -> durable terminal transaction state
```

Restart reconciliation uses the frozen operation identity and runtime receipt
to close either process-failure gap without duplicating data. Accepted C-03
evidence establishes that the authoritative middle step is one
`SemanticCommitPlan` applied through one storage transaction: canonical and
temporal model state, both graph directions, synchronous index-source changes,
runtime state, durable projection work, function receipt/proposal, audit,
outbox, cursor, and outcome become visible together or not at all. The durable
prepare and terminal control states make a lost acknowledgement recoverable;
they do not split that semantic data commit or create a second data authority.

The public transaction contract also still accepts separate `claims` and
`data` scopes. That is conflicting pre-release inventory, not a compatibility
promise. Gates H-04 and J-01 must preserve accepted C-03 claim semantics across
the public surfaces and remove the claim-only successful path, contract branch,
and fixtures.

## Query, context, and delivery limits

Query and context operations pass through the engine's binder, planner,
authorization, and explicit public budgets. Their transport does not prove
that every target physical access path exists. Accepted C-04 evidence
establishes bounded, authenticated point/prefix/version reads at one
`ReadStamp`; normal query, vector, retrieval, context, memory, and inference
paths no longer reconstruct state from the runtime log. Current query output
may still allocate materialized rows, and current live-query polling executes
two stamped snapshots and computes a deterministic difference. Roadmap Gates
E, F, and H own native graph/lexical/vector indexes, streamed Arrow batches,
bounded DataFusion execution, and commit-impact delivery without changing
transport authority.

`changes/follow` is a bounded waiting read over the durable cursor contract.
It is neither the canonical multiplexed stream nor a promise to retain two
delivery designs. The generic WebSocket provides push, cumulative ACK,
backpressure, lease, retention-floor, exact reconnect replay, and independent
generation fencing for multiple subscriptions. H-03 owns direct
commit-impact delivery and the final convergence of the polling surfaces.

The server records a process-local request span today, but it does not yet
accept and propagate the canonical W3C trace context or emit the complete
low-cardinality `rrflow.<boundary>.<operation>` causal graph. A-07 froze the
vocabulary; H-05 owns complete propagation, persistence, and export proof.

## Current evidence and remaining qualification

| Evidence boundary | What current tests establish | What they do not establish |
|---|---|---|
| Contract/router parity | Every catalogued HTTP operation has exactly one dispatch; generated OpenAPI is derived from the same catalogue. | Released SDK or GraphQL conformance. |
| Installed startup | Primary `rrflow serve`, the internal server process, embedded MCP, and the shared SDK daemon all resolve canonical installed state; manifest/private-binding/raw-root product startup and the server initializer are absent. | Signed bundle/service integration, hostile-path/native ACL qualification, and the complete cross-surface semantic matrix. |
| Real socket process | Loopback enforcement, liveness/readiness, authentication and scope denial, bounded envelopes, session/transaction replay, concurrent idempotency, multi-model commit/reopen, query, vector, changefeed, backup/restore, secret exclusion, and B-04 generic WebSocket request/cancel plus two-subscription/replay behavior. | Native target storage/index execution, H-04 operation cancellation, or full resource/trace export. |
| Mutual-TLS transport | The CLI requires the certificate, key, and client CA together; a real-process Rust client test rejects a missing client certificate and wrong server name, accepts the configured identities, and carries HTTPS plus generic multiplexed WSS subscription traffic. | Untrusted-client-chain coverage, certificate rotation and revocation, external identity, and production deployment integration remain open. |
| Local process implementation | Installed-only child startup, root containment, restart/reopen, identity-safe process control, bounded stop escalation, and controller kill-gap convergence. | Engine-prepared process authority, artifact-to-exec race binding, authenticated receipt-based readiness, rrflowMX parity, or a released service manager; the canonical target and disposition are in the local-process adapter reference. |

The focused characterization commands are:

```text
cargo test -p rrd-server every_catalogued_http_operation_resolves_to_one_executable_dispatch --locked
cargo test -p rrd-server --all-targets --locked
cargo test -p rrd-client --test real_server --locked
cargo test -p rrd-client --test transport_faults --locked
python3 scripts/ci/check_generated_surfaces.py
```

Remaining product work is governed by the roadmap and POA&M. In particular,
this server reference cannot close the remaining C-07 recovery and maintenance
qualification, native access paths in Gate E, streamed Arrow/DataFusion
execution in Gate F, reasoning/context integration in Gates G/H, the remaining
install and attunement work in Gate D, automation in Gate I, or release proof
in Gate J. It preserves accepted C-03 atomic-commit and accepted C-04
direct-read evidence rather than rescheduling either as server work.

## Implementation anchors

- Public catalogue and wire types: `crates/transport/rrd-contract`
- Server composition and transport: `crates/transport/rrd-server`
- Sole semantic authority: `crates/authority/rrd-engine`
- Target architecture: [engine data flow](../../architecture/engine-data-flow.md)
- Observed gaps: [RRFlow 1.0 alpha POA&M](../../poam/rrflow-1.0-alpha.md)
