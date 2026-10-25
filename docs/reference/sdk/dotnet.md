# RRFlow .NET SDK

**Status:** active implementation reference; alpha SDK and artifact qualification are incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/sdk/dotnet`
**Owner:** .NET client generation, public API, HTTP/WebSocket transport, operation validation, retry, cancellation, credential, NuGet distribution, and conformance behavior

`sdks/dotnet` is the current .NET client foundation for the public RRD
protocol. It is an outward client of the one RRFlow engine. It is not another
database, query planner, project supervisor, installation controller, or .NET
implementation of rrflowMX, rrflowDB, rrflowKV, rrflowQL, Arrow/DataFusion,
graph, lexical, vector, reasoning, attunement, routine, or skill behavior.
Those semantics remain behind `RrdEngine` for every calling language.

The [public contract](../protocol/public-contract.md) owns operation and wire
vocabulary. The [engine data flow](../../architecture/engine-data-flow.md)
owns the transaction, persistence, native-access, Arrow/DataFusion, reasoning,
and context work behind each call. The
[roadmap](../../roadmap/rrflow-1.0.md) alone owns acceptance. The
[Rust SDK reference](rust.md) owns the language-neutral operation-certainty,
cancellation, and cross-surface rules; this record applies them with .NET
semantics rather than copying Rust implementation structure.

## Required end-to-end boundary

```text
.NET application
  -> verified NuGet artifact + explicit endpoint/security profile
  -> HTTP or bounded multiplexed WebSocket RRD transport
  -> RrdEngine authentication, authorization, stamp, planning, and transaction
  -> rrflowMX or rrflowDB/rrflowKV + graph/BM25/vector + rrflowQL/DataFusion
  -> one validated result/denial/receipt with the same IDs, stamp, and trace
```

An enum member, successful `dotnet build`, passing mock, generated OpenAPI
projection, packed `.nupkg`, or decoded `JsonElement` proves only a .NET-client
property. It does not prove the final two lines. .NET never selects a storage
profile, raw key, physical index, graph traversal implementation, DataFusion
plan, model backend, project database, or reasoning mutation. It submits
public semantic intent and budgets and validates the engine's public result.

.NET consumers receive typed public protocol values, not private storage
objects or an alternate Arrow execution authority. If a future catalogued
operation carries Arrow IPC, that media type, schema, bounds, ownership, and
lifetime must be defined by the shared public contract before .NET exposes it.
Internal RRFlow use of Arrow/DataFusion is not a reason to expose DataFusion
objects or reconstruct engine planning in this SDK.

## Audited current artifact

The current package ID is `Rrflow.Rrd.Client`, the assembly namespace is
`Rrflow.Rrd`, and the package version is the repository's frozen `1.0.0`.
That version does not imply alpha readiness or authorize publication. The
workspace pins SDK `10.0.402` with roll-forward disabled, targets only
`net10.0`, and centralizes nullable analysis, warnings-as-errors, the latest
analysis level under that pinned SDK, deterministic compilation, and lock-file
generation in `Directory.Build.props`. `Directory.Packages.props` is the sole
source for the xUnit v3 4.0.2 test version. The client has no third-party
runtime dependency. These source/build-policy facts do not qualify a release
toolchain or offline dependency closure.

The complete reviewed source layout is:

```text
sdks/dotnet/
├── Directory.Build.props
├── Directory.Packages.props
├── README.md
├── Rrflow.Rrd.slnx
├── global.json
├── scripts/
│   └── generate.py
├── src/Rrflow.Rrd.Client/
│   ├── ClientOptions.cs
│   ├── EndpointResolver.cs
│   ├── Errors.cs
│   ├── Generated/
│   │   └── OperationId.g.cs
│   ├── HttpTransport.cs
│   ├── OperationBinding.cs
│   ├── OperationExecutor.cs
│   ├── ProtocolCodec.cs
│   ├── RequestOptions.cs
│   ├── ResourcePath.cs
│   ├── RetryPolicy.cs
│   ├── RrdClient.cs
│   ├── Rrflow.Rrd.Client.csproj
│   ├── Session.cs
│   └── packages.lock.json
└── tests/Rrflow.Rrd.Client.Tests/
    ├── RrdClientTests.cs
    ├── Rrflow.Rrd.Client.Tests.csproj
    ├── SdkConformanceTests.cs
    └── packages.lock.json
```

`RrdClient` is now the narrow asynchronous generic HTTP facade implementing
`IDisposable`. Package-internal classes own validated construction, loopback
endpoint/route resolution, HTTP carriage, common operation binding/execution,
partial protocol coding, and broad retry/deadline behavior. Public value types
have one responsibility file, and the generated operation projection has one
path. Helpers cover capabilities, endpoint catalogue, OpenAPI, and session
creation; `CallAsync` can address all 40 current HTTP operation identifiers.
That is structural source convergence and route reachability, not complete
operation semantics. There is no .NET WebSocket, D-01-installed endpoint
binding, authenticated remote resolver/TLS profile, server-cancellation path,
source-generated operation model set, or release-qualified package.

The current client usefully:

- reuses and disposes one `HttpClient` rather than constructing one per call;
- restricts cleartext endpoints to credential-free loopback hosts without
  resolving arbitrary hostnames;
- disables redirects on its owned default handler;
- validates selected canonical/correlation identifiers, resource kinds, and
  required route parameters;
- requires an idempotency key for catalogue-marked mutations;
- calculates an absolute-deadline remainder for each attempt;
- serializes the request once so client-controlled retries send the same body
  bytes;
- caps encoded response bodies at 4 MiB by default and 16 MiB at maximum;
- rejects unknown field names in the common envelope, outcome, and error;
- checks request and operation correlation for non-GET responses;
- returns a cloned `JsonElement` detached from the response document; and
- supplies a manifest-gated shared live-conformance entry.

Those are characterization foundations. They do not establish typed
operation validation, safe credentials, semantic retry, caller/server
cancellation, bounded concurrent delivery, endpoint identity, deterministic
packaging, or complete-engine conformance.

## Generated authority and operation coverage

`scripts/generate.py` invokes the repository-local `rrd-contract-export`
binary as an argument vector without a shell, reads the complete OpenAPI 3.1
document, sorts operations, and supports a byte-for-byte drift check. The
checked-in `Generated/OperationId.g.cs` contains:

- 40 enum members;
- wire operation name, HTTP method, route template, first authentication
  scheme, and mutation flag for each member; and
- OpenAPI SHA-256
  `06cd649d80ec2de5fe152da49daa00533d2e5dbf56e0321fea4d86f7b432a7b3`.

An earlier flat record incorrectly said 34 routes when the generator, enum,
unit assertion, public catalogue, and OpenAPI projection contained 33. The
prompt-flight family (`flight-launch`, `flight-list`, `flight-read`) then
raised every projection to 36, and the task family (`task-open`,
`task-close`, `task-list`, `task-read`) to 40; this reference records 40.
None of these changes is a language-binding capability claim.

The generator does not emit request/result/error models, allowed route
parameters, security actions, exact success statuses, media types, causal
identity requirements, stamp/receipt bindings, or runtime validators.
`CallAsync(OperationId, object?, RequestOptions, CancellationToken)` accepts
any `System.Text.Json`-serializable payload and returns a `JsonElement`, so an
enum member does not make an operation type-safe.

The target generator emits one closed descriptor and concrete request/result
binding for every catalogued operation. Each binding carries the exact
operation ID, method, route parameters, authentication, mutation semantics,
security action, request validator, result validator, success status, media
type, and applicable causal bindings from the same executable contract
source. Coverage fails on a missing, extra, duplicate, or mismatched binding.
No handwritten .NET route registry or .NET-only wire dialect is permitted.

Generated DTOs and a `JsonSerializerContext` use `System.Text.Json` source
generation, but generated CLR types alone are not semantic validation. The
binding also enforces string bytes, item counts, numeric domains, nesting,
discriminants, resource grammar, correlations, stamps, and digests. Generation
time, compiler output, package contribution, heap use, adversarial depth, and
validation-error volume remain measured budgets. Reflection fallback is
disabled on the qualified path so trimming and Native AOT claims can be tested
rather than inferred.

## Request and response enforcement

The current request path validates the common envelope and selected
identifiers, then serializes an arbitrary object. It has no operation-specific
schema, maximum encoded request size, item/depth/string/numeric budget, or
complete pre-I/O validation. Extra route parameters are silently ignored.
`RequestOptions` exposes mutable dictionary/list interfaces and record `with`
copies are shallow, so concurrent caller mutation is not prevented.

The current response path bounds and copies the encoded body, parses it as a
`JsonDocument`, and strictly compares the outer field-name set. It nevertheless:

- ignores response `Content-Type`;
- accepts any `2xx` status when the outcome says `ok` rather than the exact
  status declared for the operation;
- does not correlate GET request and operation identities;
- accepts any JSON object as a successful operation payload;
- accepts arbitrary error-code strings and assumes detail values are strings;
- includes the server-controlled error message in the public exception text;
- does not bind protocol, instance, catalogue, OpenAPI, schema, read stamp,
  resource, digest, or receipt completely;
- collapses duplicate member names while comparing field-name sets and does
  not explicitly reject duplicate JSON properties; and
- does not explicitly budget headers, JSON tokens, strings, decoded values,
  object count, or simultaneous stream/`MemoryStream`/array/document heap.

A temporary characterization test, removed after execution, submitted an
invalid unpinned query, failed the first dispatched attempt, and served
`201 text/plain` with an invalid success payload on the second. The client
made two attempts and returned that object. H-04 preserves this as a negative
test: the final client rejects the request before I/O and rejects the response
status, media type, and payload before returning a value.

Direct convergence freezes a complete concrete request before network I/O and
before exceeding its allocation budget. On response it enforces declared
status and media type, bounds headers plus encoded and decoded representation,
decodes once under explicit parser constraints and source-generated metadata,
validates the common envelope and concrete result, and checks every applicable
identity, stamp, digest, and receipt. Duplicate members, unknown variants,
invalid Unicode, excessive depth, non-finite/out-of-range numbers, and coerced
types fail closed. Invalid representation is never reclassified as a transient
transport failure.

## Endpoint, HTTP transport, and concurrent use

The constructor currently accepts only `http:` at `localhost` or a loopback IP
with no URL credentials, query, or fragment. That is an intentionally narrow
local profile. It has no HTTPS, mutual TLS, installed mesh identity, endpoint
rotation, or authenticated network profile. A configured base path remains
part of route resolution but is not bound to installation identity.

The owned `HttpClientHandler` sets only redirect denial. The client inherits
runtime defaults for proxy, cookies, DNS/connection pooling, connection count,
idle/lifetime policy, connect timeout, response-header limit, and diagnostics.
`HttpClient.Timeout` also retains its independent default while RRFlow adds a
per-attempt linked token; a configured RRFlow timeout above that hidden client
timeout can therefore observe a different limit. The handler injection seam
is useful for tests but does not qualify production endpoint or resource
policy. The current client has no in-flight/queue bound beyond external
runtime behavior.

The accepted default uses an explicitly configured `SocketsHttpHandler` with
RRFlow-owned limits: no ambient proxy or cookies for the local profile,
redirect denial, no implicit decompression, bounded connection establishment,
connections, pooled lifetime/idle time, response headers, drains, HTTP
versions, in-flight calls, and shutdown. `HttpClient.Timeout` is disabled so
the semantic deadline/caller-cancellation classifier owns cancellation
meaning. An optional host-integrated transport is accepted only through the
same conformance port and cannot weaken endpoint identity or resource bounds.

One qualified connection:

1. resolves bounded endpoint candidates and expected transport/RRD identities
   from the D-01-installed project binding;
2. connects with the selected no-proxy loopback or authenticated network
   profile;
3. rejects redirects and verifies final transport identity;
4. checks liveness and authenticated readiness;
5. negotiates protocol, exact instance, catalogue/OpenAPI digests, limits,
   and available capabilities; and
6. authenticates an application session before protected operations.

Network reachability, Zuul Zero/Wardenclyffe mesh membership, TLS, a managed
thread, dependency injection container, or project directory never grants
RRFlow permission and never initializes a database.

The target `RrdClient` is immutable after construction and safe to share
across concurrent tasks. It owns a bounded transport and deterministic
`IDisposable`/`IAsyncDisposable` shutdown; caller-supplied transports have
explicit ownership. Request bytes and options are frozen before dispatch.
Concurrency tests cover calls, cancellation, session renewal/rotation, close
races, queue/connection exhaustion, handler faults, and subscriptions rather
than only independent mock handlers.

## Retry, deadlines, cancellation, and outcome certainty

The current client-controlled attempt rule is:

```text
GET OR operation is not marked mutation OR idempotency key is present
```

That replays every non-mutating POST, including a query that has no pinned read
coordinate and can observe different state. Default attempts are two and the
accepted range is one through eight. Retries are immediate, use no bounded
backoff/jitter or endpoint-rotation policy, and produce no attempt evidence or
uncertain-observation result. `HttpRequestException` and any
`OperationCanceledException` not caused by the caller token enter the same
retry path, conflating RRFlow attempt timeout, the independent `HttpClient`
timeout, and other cancellation sources.

Caller cancellation propagates to the local HTTP operation, but it proves
only that local waiting/transport work was canceled. It does not prove the
server stopped compute or that a mutation did not commit. Local deadline time
also uses static `DateTimeOffset.UtcNow`, making deterministic scheduling and
clock testing implicit.

The target follows the shared
[operation-certainty matrix](rust.md#retry-deadline-and-outcome-certainty): a
safe public discovery may retry a classified transient failure; a read may
replay only when the engine preserves its accepted read coordinate; and a
mutation may replay only as identical bytes and coordinates bound to a durable
engine receipt. A lost response otherwise becomes a typed uncertain
observation or mutation outcome requiring explicit reconciliation. Server
`retryable` metadata never overrides operation semantics.

An `RrdCall<TResult>` binds request identity, completion task, semantic
deadline, caller token, and server-cancellation state. B-04 supplies the
correlated cancel frame/operation and terminal evidence shape; H-04 implements
its .NET carriage. Canceling a local `Task` or token alone never manufactures
server completion. The client uses an
injected `TimeProvider` for deterministic local deadline/backoff behavior.
Errors distinguish caller cancellation, semantic deadline, transport timeout,
server cancellation, and uncertain outcome without relying on exception text.

## Credentials, errors, and causal evidence

`Session`, `SessionLease`, `ApiKeyCredentials`, and `RequestOptions` are public
records containing or referencing credential strings. C# records synthesize a
string representation from public members, while `System.Text.Json` serializes
public properties. The review probe confirmed that the bearer appeared in both
`Session.ToString()` and JSON serialization; `RequestOptions` can transitively
expose either credential family as well.

The target uses opaque sealed credential/session handles, not records or
public token properties. They deliberately redact `ToString`, are rejected by
the SDK's public serializer context, and expose no API-key, bearer, private-key,
or authorization-header value. Credential providers perform only the minimum
header operation and never become a second session store. Secrets do not enter
exceptions, logs, traces, metrics, URLs, process arguments, object inspection,
request excerpts, equality, or hash output. Managed `string` instances cannot
be reliably zeroed, so the SDK makes no false erasure claim and avoids
unnecessary copies where supported.

Errors become a closed phase-aware hierarchy beneath `RrdClientException`:
local contract, endpoint resolution, TLS identity, transport, pool/resource
exhaustion, deadline, caller/server cancellation, protocol, API denial,
response validation, and uncertain observation or mutation outcome remain
distinguishable. Bounded structured server fields and safe causes are retained
without embedding arbitrary messages, URIs, headers, or transport dumps in
public error text.

Every operation propagates canonical request and operation IDs plus valid W3C
`traceparent`/`tracestate`. Logical attempts are children of one semantic
operation; queue wait, DNS, connect, TLS, pool, headers, body, reconnect, and
endpoint rotation are bounded attributes or causal links. `Activity`,
`DiagnosticSource`, OpenTelemetry, or application logging may observe those
facts only through the H-05 redacted adapter. Ambient .NET diagnostics never
become completion truth, durable lifecycle state, or hidden model reasoning.

## Multiplexed WebSocket delivery

The current .NET package has no WebSocket implementation or test. B-04 owns
the closed language-neutral multiplexed protocol and Rust reference
implementation; H-04 must supply .NET's qualified carriage and idiomatic
asynchronous API.

`ClientWebSocket` permits one send and one receive in parallel but requires
same-direction operations to be serialized. Its options expose proxy,
credentials, certificate validation, buffers, keepalive, subprotocol, and
compression decisions. Those primitives do not implement RRFlow message
identity, reassembly limits, bounded queues, ACK durability, reconnect, or
terminal engine meaning. Compression stays disabled for secret-bearing
traffic unless a reviewed message policy proves separation and size bounds.

The resulting .NET subscription client must:

- carry request/response, cancellation, subscription, delivery, cumulative
  ACK, heartbeat, backpressure, error, and terminal messages over one
  negotiated connection;
- enforce frame, reassembled-message, receive/send, in-flight, channel/queue,
  parser, and decompression limits before unsafe allocation;
- validate protocol, connection generation, request/subscription identity,
  sequence, cursor, and bounded error on every message;
- serialize same-direction I/O and define pump/task/channel ownership;
- expose cancellation-aware `IAsyncEnumerable<T>` delivery without an
  unbounded hidden buffer;
- resume only from a durably acknowledged cursor and reject stale generations;
- distinguish socket close, caller cancellation, server cancellation, and
  terminal engine outcome; and
- share endpoint identity, sessions, operation bindings, errors, and traces
  with HTTP without adding .NET-owned lifecycle state.

## .NET build, NuGet, and offline distribution boundary

The solution and two SDK-style projects now select SDK `10.0.402` through
tracked `global.json` with roll-forward disabled. Common compiler, analysis,
deterministic-compilation, target-framework, and lock-file-generation policy
lives in `Directory.Build.props`; the xUnit version lives once in
`Directory.Packages.props`. Exact locked restore still requires
`--locked-mode`. The runtime lock has no package dependencies; the test lock
records xUnit and sixteen transitive packages. A library lock does not control
the graph selected by a consuming application, so neither the pin nor either
lock substitutes for the signed release closure.

The A-07.1g package probes emitted 24,983-byte and 24,982-byte six-entry
packages containing the assembly and the README from the workspace root. The
moved package input therefore resolves, while the differing bytes preserve a
concrete reproducibility failure. The
package still omits XML API documentation, symbol/source package, consumer
fixtures, SBOM, signature, and complete provenance. Its manifest uses default
author/description metadata and has no repository URL, project URL, license,
tags, or release notes. The existing `dotnet nuget verify --all` probe fails
with `NU3004` because the package is unsigned.

Two consecutive Release builds produced byte-identical DLL and PDB bytes, but
two consecutive `.nupkg` files did not. Inspection localized the difference to
the random Open Packaging Convention core-properties part/relationship plus
archive timestamps. Passing deterministic compiler output therefore does not
prove a reproducible NuGet artifact. Supplying `DeterministicTimestamp` to the
installed .NET 10 SDK did not make those package bytes identical.

An empty file-backed NuGet source restored the dependency-free runtime project
but failed the solution before tests because xUnit was absent. A warm global
package cache can make locked restore pass; it is not an independently
reproducible source/test closure.

The remaining qualification target:

- treats the tracked SDK pin and shared build/dependency files as reviewed
  inputs, then qualifies their exact bytes rather than assuming the pin makes
  the toolchain or dependency closure reproducible;
- targets only supported modern TFMs that pass the full matrix—initially
  `net10.0`; it adds no older compatibility target or conditional shim merely
  to increase a badge count;
- enables warning-clean XML documentation, package validation, trimming and
  AOT analyzers, and a real trimmed/Native-AOT consumer where claimed;
- emits complete non-placeholder metadata, main package, symbols/source
  package, exact project file, checksums, SBOM/licenses, and provenance;
- uses a repository-owned package builder to canonicalize entry order,
  timestamps, OPC part/relationship identities, and metadata under the pinned
  toolchain, then compares two unsigned packages byte for byte before signing;
- signs the accepted artifact, verifies signer/timestamp/trust policy on every
  supported host, and binds the signed bytes into the RRFlow distribution
  manifest; and
- creates an empty temporary file-backed feed/cache, installs the package into
  an external consumer, then compiles/runs HTTP, WebSocket, trimming, and
  Native-AOT cases with every network source disabled.

The signed offline RRFlow distribution is the default release authority. It
contains prebuilt .NET artifacts and every required source/test dependency
byte; engine installation and readiness never require the .NET SDK or NuGet.
A source consumer may use the bundle's verified file feed and pinned toolchain
without nuget.org, another checkout, or a prewarmed user cache. A public NuGet
feed, if later used, is an optional byte-identical projection and never the
only installation path. The frozen repository version remains `1.0.0`; this
pre-release package is not published merely to manufacture progress.

## Conformance that actually counts

Current evidence is deliberately bounded:

| Command or probe | Observed result | Honest boundary |
|---|---|---|
| `dotnet restore ... --locked-mode` plus `dotnet build ... --no-restore` | Before and after the split, the solution restored and built with zero warnings under pinned SDK 10.0.111 on Linux x64. | One installed SDK/cache; no toolchain/platform/offline closure. |
| `dotnet run --project ...Tests.csproj --no-restore` | xUnit reported four tests passed, but the manifest-absent conformance method returned before executing a scenario and was not reported skipped. | Three mock-focused tests plus one false pass; no configured engine corpus. |
| temporary baseline/candidate assembly reflection audit | All 140 exported type/member signatures matched: zero removed and zero added. | Public assembly shape only; it does not prove behavior or packaging compatibility. |
| `python3 sdks/dotnet/scripts/generate.py --check` | The sole enum at `Generated/OperationId.g.cs` matched 40 operations and the OpenAPI digest. | Method/path/auth/mutation projection only. |
| `dotnet format ... --verify-no-changes` and Release `dotnet pack` | Formatting passed; both package observations resolved the moved root README and emitted six entries, but their sizes were 24,983 and 24,982 bytes. | Source style and package topology pass; byte reproducibility, signature, and consumer proof fail or remain absent. |
| full shared conformance runner | The configured .NET entry passed with corpus SHA-256 `b3977c57c8d268f861e9d5158e5609bf3e7d2e1db01f914a12911b95c3404cb2`. | Real HTTP against one direct-seeded rrflowKV daemon fixture; not installation, rrflowMX parity, or complete labelled behavior. |
| temporary adversarial test | Replayed an unpinned invalid query, accepted `201 text/plain` plus invalid payload, and exposed the bearer through record formatting and JSON. | Reproducible open H-04 correctness/security defects. |
| two clean Release builds and packages | DLL/PDB pairs were byte-identical; `.nupkg` SHA-256 values differed. | Compiler determinism exists locally; package determinism does not. |
| empty-source locked restore | Runtime project restored; test project failed resolving xUnit. | Runtime has no NuGet dependency, but the checkout lacks a complete offline build/test feed. |
| `dotnet nuget verify --all` | Failed with `NU3004: The package is not signed`. | Current package has no verified origin/signature. |

The live corpus currently exercises capability retry/version rejection,
catalogue count, one authentication denial, session create/renew/close, vector
ensure/search, transaction begin/preview/abort/commit, one query, changefeed
read/follow/local cancellation, backup create/list, and estate read. Its
declared domain labels overstate CRUD, backup/restore, vector, live delivery,
typed-error, retry/failure, cancellation, and version/artifact coverage as
catalogued in the [Rust reference](rust.md#conformance-that-actually-counts).

No row proves RRFlow's persistent multi-model reasoning/recall engine. Release
conformance starts a D-01-installed project instance through public operations,
runs structural scenarios against rrflowMX and rrflowDB/rrflowKV, adds durable
crash/reopen cases, and compares result, denial, `ReadStamp`, plan/projection
digest, transaction receipt, reasoning/context evidence, trace, and resource
accounting across HTTP, WebSocket, Rust, .NET, every other supported SDK, CLI,
MCP, GraphQL, and Connectome. The engine corpus exercises native
document/temporal-graph/scalar/BM25/vector access, bounded RRF context
selection, streamed Arrow/DataFusion analytics, and persisted
reasoning/feedback. .NET proves only faithful access to that engine.

## Current and gated file plan

.NET remains one client package and one `Rrflow.Rrd` client namespace;
responsibility splits do not create another runtime. Generated files remain
subordinate to the executable public contract.

```text
sdks/dotnet/
├── Directory.Build.props               # one compiler/build/package policy
├── Directory.Packages.props            # one dependency-version authority
├── README.md                            # narrow SDK workspace/package entrance
├── Rrflow.Rrd.slnx
├── global.json                         # exact accepted SDK selection
├── scripts/
│   └── generate.py                     # sole catalogue projection generator
├── src/Rrflow.Rrd.Client/
│   ├── ClientOptions.cs                # public options + validated internal copy
│   ├── EndpointResolver.cs             # current loopback URI/path validation
│   ├── Errors.cs                       # current public client/API errors
│   ├── Generated/
│   │   └── OperationId.g.cs            # sole catalogue-derived projection
│   ├── HttpTransport.cs                # current response-bounded HTTP carriage
│   ├── OperationBinding.cs             # current envelope/resource/auth binding
│   ├── OperationExecutor.cs            # current encode/send/decode coordination
│   ├── ProtocolCodec.cs                # current partial envelope codec
│   ├── RequestOptions.cs               # current public per-call values
│   ├── ResourcePath.cs                 # current public resource component
│   ├── RetryPolicy.cs                  # current broad retry/deadline behavior
│   ├── RrdClient.cs                    # narrow public facade
│   ├── Rrflow.Rrd.Client.csproj
│   ├── Session.cs                      # current public credential records
│   └── packages.lock.json
└── tests/Rrflow.Rrd.Client.Tests/
    ├── RrdClientTests.cs
    ├── SdkConformanceTests.cs
    ├── Rrflow.Rrd.Client.Tests.csproj
    └── packages.lock.json
```

`scripts/build_package.py`, `RrdCall`, `Subscription`,
`WebSocketTransport`, generated operation models/JSON metadata, and the
focused operation, protocol, transport, subscription, concurrency, and
package-consumer tests remain absent. Their B/H/J gates create them only with
real behavior and evidence; the current similarly named internal seams do not
claim those future semantics.

The dependency order is:

1. **A-07.1g .NET structural slice:** complete. The exact SDK pin and shared
   policy inputs exist; the README and generated enum have one direct path;
   `Models.cs` is gone; and `RrdClient.cs` is the narrow facade over the named
   internal responsibilities. All 140 exported signatures, current generation,
   loopback/redirect denial, envelope/resource construction, mutation
   idempotency, identical retry bytes, absolute deadline, response limit,
   disposal, manifest-absent behavior, and unit characterization remain. This
   checks only the A-07.1g supporting slice, not canonical A-07 or H/J behavior.
2. **B-04 (contract implemented):** consume the closed multiplexed golden
   protocol; no .NET behavior is claimed by the Rust reference carriage.
3. **D-01:** replace direct fixture seeding with installed public bootstrap and
   endpoint identity.
4. **H-04:** implement bounded multiplexed .NET carriage, call/subscription
   APIs, and pump/channel ownership; generate concrete operation models/
   bindings/JSON metadata; enforce exact request/result/error/status/media/
   identity contracts, semantic certainty, opaque credentials, W3C
   propagation, and structural cross-surface cases.
5. **H-07:** add authenticated HTTPS/mTLS/mesh endpoint resolution and rotation
   without making reachability authority.
6. **J-02/J-03/J-05:** pass fault/resource, exact SDK and supported runtime,
   Linux/Windows/macOS/architecture/concurrency, package-validation,
   trimmed/Native-AOT/external-consumer, byte-reproducible package,
   empty-feed offline, signature/provenance, and release verification.

## Primary constraints

- [.NET `HttpClient` lifetime guidance](https://learn.microsoft.com/en-us/dotnet/fundamentals/networking/http/httpclient-guidelines)
  requires intentional client/pool lifetime and DNS renewal; RRFlow owns and
  measures its standalone handler rather than inheriting ambient defaults.
- [`SocketsHttpHandler`](https://learn.microsoft.com/en-us/dotnet/api/system.net.http.socketshttphandler?view=net-10.0)
  exposes connect, proxy, cookie, TLS, connection, header, drain, pool, ping,
  version, tracing, and decompression controls used by the transport profile.
- [`ClientWebSocketOptions`](https://learn.microsoft.com/en-us/dotnet/api/system.net.websockets.clientwebsocketoptions?view=net-10.0)
  exposes handshake/TLS/proxy/buffer/keepalive/compression policy, while
  [`ReceiveAsync`](https://learn.microsoft.com/en-us/dotnet/api/system.net.websockets.clientwebsocket.receiveasync?view=net-10.0)
  permits only one parallel receive; B-04 supplies RRFlow identity, queue,
  framing, cancellation, ACK, and resume semantics that H-04 must carry in
  .NET.
- [`System.Text.Json` source generation](https://learn.microsoft.com/en-us/dotnet/standard/serialization/system-text-json/source-generation)
  provides compile-time metadata, and Microsoft's
  [reflection/source-generation guidance](https://learn.microsoft.com/en-us/dotnet/standard/serialization/system-text-json/reflection-vs-source-generation)
  identifies trimming, Native AOT, memory, and startup consequences; RRFlow
  still adds exact semantic validation and budgets.
- [NuGet lock-file guidance](https://learn.microsoft.com/en-us/nuget/consume-packages/package-references-in-project-files#locking-dependencies)
  defines locked restore and explains why a library's lock does not govern its
  consumer; the signed RRFlow manifest owns the shipped closure.
- [NuGet package-authoring guidance](https://learn.microsoft.com/en-us/nuget/create-packages/package-authoring-best-practices)
  defines complete package metadata and README/license expectations; RRFlow
  additionally requires offline, byte, signature, and semantic qualification.
- [NuGet deterministic-package guidance](https://learn.microsoft.com/en-us/nuget/create-packages/deterministic-packages)
  makes package determinism toolchain-dependent; the pinned RRFlow builder
  compares exact canonical bytes instead of assuming deterministic assemblies
  imply a deterministic archive.
- [.NET package validation](https://learn.microsoft.com/en-us/dotnet/fundamentals/apicompat/package-validation/overview)
  checks package/API consistency. Because no RRFlow .NET release baseline
  exists, A-07 converges directly; the accepted 1.0 artifact becomes the first
  future compatibility baseline only after release qualification.
- [.NET trimming guidance](https://learn.microsoft.com/en-us/dotnet/core/deploying/trimming/prepare-libraries-for-trimming)
  requires both library analysis and a real trimmed consumer; Native-AOT
  support likewise requires the analyzer and published consumer, not a flag.
- [NuGet signature verification](https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-nuget-verify)
  verifies package signatures; J-05 binds the verified signer, timestamp, and
  exact package digest into the distribution manifest.
- [The .NET support policy](https://dotnet.microsoft.com/en-us/platform/support/policy/dotnet-core)
  determines supported runtime lines and patch currency; the release matrix
  records exact SDK/runtime bytes rather than relying on a TFM label alone.
- [RFC 9110 HTTP semantics](https://www.rfc-editor.org/rfc/rfc9110.html#name-idempotent-methods)
  bounds replay and representation handling; stamped reads and durable mutation
  receipts impose stricter RRFlow semantics.
- [RFC 6455 implementation limits](https://www.rfc-editor.org/rfc/rfc6455.html#section-10.4)
  require protection against oversized frames and reassembled messages; B-04
  defines RRFlow queue, identity, cursor, generation, and backpressure bounds.
- [W3C Trace Context](https://www.w3.org/TR/trace-context/) defines
  `traceparent` and `tracestate`; RRFlow owns validation, causal links,
  redaction, and evidence meaning.
