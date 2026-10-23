# RRFlow Go SDK

**Status:** active implementation reference; alpha SDK and module qualification are incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/sdk/go`
**Owner:** Go client generation, public API, HTTP/WebSocket transport, operation validation, retry, cancellation, credential, module distribution, and conformance behavior

`sdks/go` is the current Go client foundation for the public RRD protocol. It
is an outward, concurrent client of the one RRFlow engine. It is not Clyffy's
routing or orchestration runtime, an installation controller, a project
supervisor, or a Go implementation of context, reasoning, storage, graph,
index, vector, rrflowQL, or DataFusion behavior. Those semantics remain behind
`RrdEngine` regardless of the calling language.

The [public contract](../protocol/public-contract.md) owns operation and wire
vocabulary. The [engine data flow](../../architecture/engine-data-flow.md)
owns the transaction, persistence, native-access, Arrow/DataFusion, reasoning,
and context work behind each call. The
[roadmap](../../roadmap/rrflow-1.0.md) alone owns acceptance. The
[Rust SDK reference](rust.md) owns the language-neutral operation-certainty,
cancellation, and cross-surface rules; this record applies them using
idiomatic Go rather than copying Rust implementation structure.

## Required end-to-end boundary

```text
Go application
  -> installed Go module + explicit endpoint/security profile
  -> HTTP or bounded multiplexed WebSocket RRD transport
  -> RrdEngine authentication, authorization, stamp, planning, and transaction
  -> rrflowMX or rrflowDB/rrflowKV + graph/BM25/vector + rrflowQL/DataFusion
  -> one validated result/denial/receipt with the same IDs, stamp, and trace
```

An `OperationID` constant, passing race test, standard-library-only module, or
successful HTTP response proves only a client property. It does not prove the
last two lines. The Go SDK never chooses a storage profile, raw key, physical
index, DataFusion plan, model backend, external project database, or reasoning
mutation. It submits public semantic intent and budgets and validates the
engine's public result.

## Audited current module

The nested module is `github.com/rrflow/rrflow/sdks/go` and declares Go 1.27.2.
It currently has no non-standard-library dependency, so the absence of a
`go.sum` is expected rather than proof of incomplete locking. The reviewed
layout is:

```text
sdks/go/
├── go.mod
├── doc.go
├── config.go
├── client.go
├── endpoint.go
├── errors.go
├── operation.go
├── retry.go
├── session.go
├── transport.go
├── endpoints_gen.go
├── client_test.go
└── cmd/
    ├── generate/main.go
    └── conformance/main.go
```

A-07.1e directly produced this layout from the former 498-line `client.go`
and catch-all `models.go`. `client.go` is now the 114-line public facade,
`models.go` is absent, and every current responsibility has one source owner.
This is source topology, not qualification of the incomplete behaviors below.

`Client` is a synchronous API whose methods accept `context.Context`. It has
helpers for capabilities, endpoint catalogue, OpenAPI, and session creation,
plus one generic `Call`. That generic call can address all 40 current HTTP
operation identifiers. This is route coverage, not operation-semantic
coverage. The module has no WebSocket implementation; subscription
administration and changefeed follow are ordinary HTTP calls.

The current client usefully:

- accepts a context as the first argument of every network operation;
- restricts cleartext endpoints to credential-free loopback hosts;
- explicitly rejects HTTP redirects;
- reuses one `http.Client` rather than constructing a client per request;
- validates selected correlation IDs, canonical identifiers, path parameters,
  and common resource segments;
- requires an idempotency key for catalogue-marked mutations;
- sends the same encoded request bytes on each client-controlled attempt;
- limits a response body to 4 MiB by default and at most 16 MiB;
- strictly rejects unknown fields in the common response and session-lease
  structs;
- checks request and operation correlation for non-GET calls; and
- has no third-party HTTP runtime dependency.

Those are characterization foundations. They do not establish complete
request/result validation, secret safety, semantic retry, server
cancellation, bounded WebSocket delivery, remote transport identity, module
release support, or engine conformance.

## Generated authority and operation coverage

`cmd/generate` locates the repository root, invokes the repository-local
`rrd-contract-export` Cargo binary as an argument vector without a shell,
reads its complete OpenAPI 3.1 output, sorts operations, formats generated Go
source with `go/format`, and supports an exact byte-for-byte drift check.

The current `endpoints_gen.go` contains:

- 40 `OperationID` constants;
- method, route template, first authentication scheme, and mutation flag for
  each operation; and
- OpenAPI SHA-256
  `28fdf404064be9b507ed2bc2780242ce15237eabe86cee5a7297af4470d4da3c`.

It does not generate request/result/error types, path-parameter sets, security
actions, exact success statuses, media types, causal identity rules,
stamp/receipt bindings, or runtime validators. `Call` accepts `payload any`
and returns `map[string]any`, so a constant and map entry do not make the
operation type-safe.

The target generator emits one closed descriptor and concrete request/result
binding per catalogue operation. Each binding carries the exact operation ID,
method, path parameters, authentication, mutation semantics, security action,
request validator, result validator, success status, and media type from the
same executable contract source. Coverage fails on any missing, extra, or
mismatched binding. No handwritten second route registry or Go-only wire
schema is permitted.

Generated structures still require semantic validation. The generator must
apply explicit length, count, numeric, nesting, discriminant, correlation,
stamp, and digest rules rather than assuming `encoding/json` field types are
sufficient. Generation time, compile time, binary contribution, heap use,
adversarial depth, and error volume remain measured budgets.

## Request and response enforcement

The current request path validates only the common envelope and selected
identifiers. It marshals an arbitrary value without an operation-specific
contract, maximum request-size check, item/depth limit, or pre-I/O semantic
validation. Unknown or extra path-parameter entries are ignored. Exported
maps and slices in options, resources, sessions, and payloads remain mutable
by callers.

The current response path strictly decodes the outer structs but leaves a
successful payload as `json.RawMessage`, then decodes it into an unrestricted
`map[string]any`. Concrete gaps are:

- response `Content-Type` is ignored;
- any `2xx` status agrees with an `ok` outcome instead of the operation's
  exact success status;
- GET response request and operation identities are not checked;
- operation payload schemas and semantic bindings are not validated;
- error codes, messages, and details are not closed or size bounded;
- protocol, instance, catalogue, OpenAPI, schema, read-stamp, resource,
  digest, and receipt bindings remain incomplete; and
- the body byte limit does not bound JSON nesting, object count, strings,
  numeric conversion, or the simultaneous encoded and decoded heap.

A reproducible injected-`RoundTripper` probe submitted an invalid unpinned
`query-execute`, failed the first attempt, and returned `201 text/plain` with
an invalid success payload on the second. The client attempted the query
twice and returned `{"invalid":true}`. H-04 must preserve this as a negative
test and reject every part before a value reaches the caller.

Direct convergence validates the complete request before network I/O and
before its allocation budget is exceeded. On response it enforces the declared
status and media type, bounds headers plus encoded and decoded bodies, decodes once, validates
the complete common and operation result, and checks every applicable causal
identity before returning a concrete type. Invalid representation is never
classified as a transient transport failure.

## Endpoint, HTTP transport, and concurrent use

`NewClient` currently accepts only `http:` at `localhost` or a loopback IP,
with no URL credentials, query, or fragment. That is an intentionally narrow
local profile. It has no HTTPS, mutual TLS, installed mesh identity, endpoint
rotation, or authenticated remote profile. A configured base path is retained
and operation paths are resolved relative to it, so configuration can relocate
nominal root routes without an identity check.

When no injected `RoundTripper` is supplied, the client directly reuses
`http.DefaultTransport`. That transport consults proxy environment variables,
caches connections, and applies library-default connection, header, idle,
compression, dial, and handshake behavior. Because the shared default
transport can also be observed or modified by process code, it is not a sealed
RRFlow transport profile. The wrapper exposes no close operation for idle
connections.

The target owns an explicit `http.Transport` clone or equivalent carriage
whose proxy policy, resolver/dialer, TLS roots and identities, protocol set,
maximum total and per-host connections, idle pool, header bytes, buffers,
compression, and phase timeouts are bounded configuration. A test transport
remains a clearly unsafe/injected test seam and cannot qualify endpoint
identity or deployment.

One qualified connection:

1. resolves bounded endpoint candidates and expected transport/RRD identities
   from the D-01-installed project binding;
2. connects with the selected loopback or authenticated network profile;
3. rejects redirects and verifies final transport identity;
4. checks liveness and authenticated readiness;
5. negotiates protocol, exact instance, catalogue/OpenAPI digests, limits,
   and available capabilities; and
6. authenticates an application session before protected operations.

Network reachability, Zuul Zero/Wardenclyffe mesh membership, TLS, a working
Go context, or a project directory never grants RRFlow permission and never
initializes a database.

The standard HTTP client and transport are safe for concurrent use, but the
current exported session lease and its `Limits` map are caller-mutable. The
qualified `Client` is safe to share across goroutines after construction,
owns deterministic close/idle-connection behavior, and uses immutable or
synchronized internal session state. Race tests cover concurrent calls,
cancellation, renewal/rotation, close, and subscriptions rather than only
three independent mocks.

## Retry, context cancellation, and outcome certainty

The current client-controlled attempt rule is:

```text
GET OR operation is not marked mutation OR idempotency key is present
```

That replays every non-mutating POST, including an unpinned query that can
observe a later read stamp. Default attempts are two and the accepted range is
one through eight. Failures are retried immediately without backoff, jitter,
attempt evidence, endpoint-rotation policy, or an uncertain-observation
result. A per-attempt timeout defaults to five seconds and is capped at five
minutes; per-attempt timeouts may themselves trigger replay while the caller's
context remains live.

Go's `http.Transport` may also retry some idempotent requests after network
errors. Logical RRFlow attempts and underlying wire attempts therefore cannot
be inferred from the outer loop count. The accepted transport/retry seam must
make those behaviors compatible with the same semantic policy and record safe
phase evidence; unspecified concrete `RoundTrip` error types or strings cannot
be the policy authority.

The target follows the shared
[operation-certainty matrix](rust.md#retry-deadline-and-outcome-certainty): a
safe public discovery may retry a classified transient failure; a read may
replay only when the engine preserves its accepted read coordinate; and a
mutation may replay only as identical bytes and coordinates bound to a durable
engine receipt. A lost response otherwise becomes a typed uncertain
observation or outcome requiring explicit reconciliation.

`context.Context` remains the first argument and carries caller cancellation
and deadlines through the call. The client preserves `context.Canceled`,
`context.DeadlineExceeded`, and safe cancellation causes as distinct local
facts. A canceled context stops local waiting; it does not prove the server
stopped compute or that a mutation did not commit. B-04 supplies the
correlated server-cancellation operation and terminal evidence shape; H-04
implements its Go carriage. Per-phase HTTP waits, the semantic deadline,
caller cancellation, server cancellation, and engine outcome remain separate.

## Credentials, errors, and causal evidence

`APIKeyCredentials`, `Session`, and `SessionLease` currently export every
field, including API-key and bearer strings. A direct probe showed the bearer
in both `encoding/json.Marshal(session)` and `fmt.Sprintf("%+v", session)`.
The nested `Limits` map is mutable. The flat record's credential-safety claim
was therefore false even before considering logs, `%#v`, reflection, or
accidental struct copies.

The target exposes an opaque session handle with deliberately narrow methods,
custom redacted formatting, no default JSON/text encoding, and no public token
field. Credential providers perform only the minimum operation needed to add
authorization and never become a session store. API keys, bearer tokens,
session identifiers, private keys, and authorization headers never enter
`String`, `GoString`, formatting, structured logs, errors, traces, metrics,
URLs, process arguments, marshaling, or response excerpts. Secret-bearing
values avoid unnecessary immutable-string copies where the selected Go API
allows it; lifetime and zeroing claims are never made where the runtime cannot
guarantee them.

Errors form a closed phase-aware API supporting `errors.Is`/`errors.As`:
local contract, endpoint resolution, TLS identity, transport, pool/resource
exhaustion, deadline, caller/server cancellation, protocol, API denial,
response validation, and uncertain observation or mutation outcome remain
distinguishable. Safe bounded causes and server details are retained without
embedding arbitrary server messages, URLs, headers, or transport strings into
the public error text. An error's `Retryable` field never overrides operation
semantics.

Every operation propagates canonical request and operation IDs plus valid W3C
`traceparent`/`tracestate`. Logical attempts are child spans of one semantic
operation; DNS, connect, TLS, pool wait, header, body, reconnect, and endpoint
rotation observations are bounded attributes or causal links. Standard
`httptrace` hooks may measure phases, but trace output is not completion truth,
durable state, or hidden model reasoning.

## Multiplexed WebSocket delivery

The current Go module has no WebSocket dependency, implementation, or test.
B-04 defines the closed language-neutral multiplexed protocol and proves its
Rust reference implementation. H-04 must implement Go state and carriage,
selecting and locking a maintained library only after its dependency,
context, deadline, concurrent-reader/writer, TLS, proxy, frame, compression,
and memory behavior is measured. Preserving a dependency-free claim is not a
reason to hand-write an unsafe WebSocket stack or omit the required surface.

The resulting client must:

- carry request/response, cancellation, subscription, delivery, cumulative
  ACK, heartbeat, backpressure, error, and terminal messages over one
  negotiated connection;
- enforce frame, reassembled-message, receive, send, in-flight, queue, decode,
  and decompression limits before allocation;
- validate protocol, connection generation, request/subscription identity,
  sequence, cursor, and bounded error on every message;
- define and test the exact goroutine/read/write ownership model;
- resume only from a durably acknowledged cursor and reject stale generations;
- distinguish socket closure, context cancellation, server cancellation, and
  terminal engine outcome; and
- share endpoint identity, sessions, operation bindings, errors, and traces
  with HTTP without adding Go-owned lifecycle state.

## Go module and offline distribution boundary

The `go 1.27.2` line declares the minimum toolchain; candidate CI runs that
same Go release, on Linux/amd64 only.
Automatic Go toolchain selection can download another toolchain, so a release
test must explicitly distinguish a locally available supported toolchain from
network-assisted success.

The nested module now has `doc.go`, but it still has no package examples,
exported-API documentation gate, tagged-version proof, module archive
inventory, external consumer test, supported OS/architecture matrix, or
signed provenance. A clean `go mod tidy -diff` and a dependency-free test with
`GOTOOLCHAIN=local GOPROXY=off GOSUMDB=off` prove only that this checkout builds
with the already installed toolchain.

RRFlow's signed offline distribution is the default release authority. It
contains the exact Go module source/archive, contract digest, module metadata,
license/provenance material, and every selected dependency archive/checksum;
default engine installation and readiness never require a Go compiler. A Go
consumer with an explicitly supported toolchain can import and test the module
with registry, VCS, checksum-network, sibling checkout, and repository cache
disabled. If an optional public module projection is published, its nested
module tag uses the repository-relative `sdks/go/` prefix and its bytes and
identity must match the signed RRFlow artifact; publication never becomes the
only installation path.

Minimum/current supported Go releases run unit, race, vet, generation,
external-consumer, HTTP, TLS, WebSocket, cancellation, and live conformance on
the J-defined Linux, Windows, macOS, architecture, and race-detector matrix.
Go's upstream support window and RRFlow's declared client support are recorded
separately and updated only with release evidence.

## Conformance that actually counts

Current evidence is deliberately bounded:

| Command or probe | Observed result | Honest boundary |
|---|---|---|
| `go test ./...` | package tests passed; two command packages have no tests | Three mock-focused tests and compilation only; no remote TLS, WebSocket, installed engine, or fault matrix. |
| `go test -race ./...` | the same package tests passed under the race detector | Selected mock concurrency only; no shared-session, cancellation, close, renewal, or subscription stress. |
| generator, `gofmt`, `go vet`, `go mod tidy -diff` | all passed | Source/generation/module hygiene only; descriptors still omit operation semantics. |
| conformance command without `RRD_SDK_CONFORMANCE_MANIFEST` | exited nonzero with the required-manifest panic | Fail-closed configuration only; no scenario executed. |
| conformance command against the live example harness | passes with corpus SHA-256 `b3977c57c8d268f861e9d5158e5609bf3e7d2e1db01f914a12911b95c3404cb2` | Real HTTP against a direct-seeded rrflowKV fixture; not installation, rrflowMX parity, or full labelled behavior. |
| generated-surface parity | 40 descriptors match the OpenAPI digest | Method/path/auth/mutation projection only; not runtime binding or semantic execution. |
| injected transport probe | replayed an unpinned query, accepted `201 text/plain` and invalid payload, and exposed the bearer through JSON and formatting | Reproducible open H-04 correctness/security defects. |
| network-disabled local-toolchain test | passed with no module dependencies | Existing-checkout/current-toolchain evidence only; not a clean external consumer or signed artifact. |

The live conformance command exercises capability retry/version rejection,
catalogue count, one authentication denial, session create/renew/close, vector
ensure/search, transaction begin/preview/abort/commit, one query, changefeed
read/follow/local cancellation, backup create/list, and estate read. Its
labels overstate CRUD, backup/restore, vector, live delivery, typed-error,
retry/failure, cancellation, and version/module coverage as catalogued in the
[Rust reference](rust.md#conformance-that-actually-counts).

No row proves RRFlow's persistent multi-model reasoning/recall engine. Release
conformance starts a D-01-installed project instance through public
operations, runs structural scenarios against rrflowMX and rrflowDB/rrflowKV,
adds durable crash/reopen cases, and compares result, denial, `ReadStamp`,
plan/projection digest, transaction receipt, reasoning/context evidence,
trace, and resource accounting across HTTP, WebSocket, Rust, Go, every other
supported SDK, CLI, MCP, GraphQL, and Connectome. The engine corpus exercises
native document/temporal-graph/scalar/BM25/vector access, bounded RRF context
selection, streamed Arrow/DataFusion analytics, and persisted
reasoning/feedback. Go proves only faithful access to that engine.

## Current and gated file plan

Go source remains one `rrd` package split by responsibility; it does not gain
a second service, runtime, or `internal` engine. A-07.1e established this
current source tree:

```text
sdks/go/
├── go.mod
├── doc.go                         # package contract and canonical links
├── config.go                      # validated public construction inputs
├── client.go                      # narrow concurrent public facade
├── endpoint.go                    # current loopback endpoint policy
├── errors.go                      # current API-error representation
├── operation.go                   # current generic operation construction
├── retry.go                       # current broad attempt/deadline behavior
├── session.go                     # current plain secret-bearing values
├── transport.go                   # current bounded HTTP response decoding
├── endpoints_gen.go               # catalogue-derived descriptor projection
├── client_test.go                 # existing mock characterization
└── cmd/
    ├── generate/main.go
    └── conformance/main.go
```

The following files remain deliberately absent until their assigned gate adds
real behavior: `subscription.go` (H-04), `models_gen.go` (H-04),
`operation_coverage_test.go` and `protocol_validation_test.go` (H-04),
`transport_faults_test.go` (H-04/H-07/J-02),
`subscription_test.go` (H-04/J-02), and
`package_consumer_test.go` (J-03/J-05).

The dependency order is:

1. **A-07 SDK split (completed by A-07.1e):** the former `client.go` and
   `models.go` responsibilities are directly split into the current tree
   above; package documentation, generation, loopback/redirect denial,
   context propagation, envelope construction, response byte limit,
   fail-closed harness input,
   race behavior, and `client_test.go` characterization are preserved.
   `models.go` is absent and `client.go` is only the narrow facade. No
   validation, socket, resolver, or package-release success was invented.
2. **B-04 (contract implemented):** consume the closed multiplexed golden
   protocol; no Go behavior is claimed by the Rust reference carriage.
3. **D-01:** replace direct fixture seeding with installed public bootstrap and
   endpoint identity.
4. **H-04:** implement bounded multiplexed Go carriage and goroutine/queue
   ownership; generate concrete operation models/bindings; enforce exact
   request/result/error/status/media/identity contracts, semantic certainty,
   opaque credentials, W3C propagation, and structural cross-surface cases.
5. **H-07:** add authenticated HTTPS/mTLS/mesh endpoint resolution and
   rotation without making reachability authority.
6. **J-02/J-03/J-05:** pass fault/resource, minimum/current Go,
   OS/architecture/race, clean-consumer, deterministic module artifact,
   network-denied installation, provenance, and release verification.

## Primary constraints

- [Go `context` package](https://pkg.go.dev/context) requires contexts to
  cross API boundaries explicitly and cancellation functions to release
  resources; RRFlow distinguishes local cancellation from server outcome.
- [Go `net/http` package](https://pkg.go.dev/net/http) defines reusable,
  concurrent transports, environment proxy behavior, connection/header
  limits, body ownership, redirect policy, and transport-level retry
  semantics; RRFlow configures and measures them explicitly.
- [Go `httptrace` package](https://pkg.go.dev/net/http/httptrace) exposes
  request-phase hooks; RRFlow bounds/redacts observations and links them to
  one semantic operation rather than treating them as completion truth.
- [Go toolchain selection](https://go.dev/doc/toolchain) makes the `go` line a
  minimum requirement and may switch or download toolchains; offline
  qualification therefore pins the available toolchain and disables fetches.
- [Managing Go module source](https://go.dev/doc/modules/managing-source)
  defines nested-module source and tag-prefix rules; the optional public
  projection must match the signed RRFlow module artifact.
- [Go module reference](https://go.dev/ref/mod) defines module archives,
  checksums, nested-module exclusions, and portable path/size constraints;
  J-03/J-05 inspect the exact archive and dependency closure.
- [Go structured secret-redaction example](https://pkg.go.dev/log/slog#example-package-Secret)
  demonstrates explicit log-value redaction; RRFlow additionally prohibits
  disclosure through general formatting, JSON/text marshaling, errors,
  traces, and metrics.
- [RFC 9110 HTTP semantics](https://www.rfc-editor.org/rfc/rfc9110.html#name-idempotent-methods)
  bounds HTTP replay and representation handling; stamped reads and durable
  mutation receipts impose stricter RRFlow semantics.
- [RFC 6455 implementation limits](https://www.rfc-editor.org/rfc/rfc6455.html#section-10.4)
  require protection against oversized frames and reassembled messages; B-04
  defines RRFlow queue, identity, cursor, generation, and backpressure bounds.
- [W3C Trace Context](https://www.w3.org/TR/trace-context/) defines
  `traceparent` and `tracestate`; RRFlow owns validation, causal links,
  redaction, and evidence meaning.
