# RRFlow Java SDK

**Status:** active implementation reference; alpha SDK and artifact qualification are incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/sdk/java`
**Owner:** Java client generation, public API, HTTP/WebSocket transport, operation validation, retry, cancellation, credential, Maven/JAR distribution, and conformance behavior

`sdks/java` is the current Java client foundation for the public RRD protocol.
It is an outward client of the one RRFlow engine. It is not another database,
query planner, project supervisor, installation controller, or Java
implementation of rrflowMX, rrflowDB, rrflowKV, rrflowQL, Arrow/DataFusion,
graph, lexical, vector, reasoning, attunement, routine, or skill behavior.
Those semantics remain behind `RrdEngine` for every calling language.

The [public contract](../protocol/public-contract.md) owns operation and wire
vocabulary. The [engine data flow](../../architecture/engine-data-flow.md)
owns the transaction, persistence, native-access, Arrow/DataFusion, reasoning,
and context work behind each call. The
[roadmap](../../roadmap/rrflow-1.0.md) alone owns acceptance. The
[Rust SDK reference](rust.md) owns the language-neutral operation-certainty,
cancellation, and cross-surface rules; this record applies them with Java
semantics rather than copying Rust implementation structure.

## Required end-to-end boundary

```text
Java application
  -> verified Java artifact + explicit endpoint/security profile
  -> HTTP or bounded multiplexed WebSocket RRD transport
  -> RrdEngine authentication, authorization, stamp, planning, and transaction
  -> rrflowMX or rrflowDB/rrflowKV + graph/BM25/vector + rrflowQL/DataFusion
  -> one validated result/denial/receipt with the same IDs, stamp, and trace
```

An enum constant, successful Maven build, passing mock, generated OpenAPI
projection, or decoded JSON object proves only a Java-client property. It does
not prove the final two lines. Java never selects a storage profile, raw key,
physical index, graph traversal implementation, DataFusion plan, model
backend, project database, or reasoning mutation. It submits public semantic
intent and budgets and validates the engine's public result.

Java consumers receive typed public protocol values, not private storage
objects or an alternate Arrow execution authority. If a future catalogued
operation carries Arrow IPC, that media type, schema, bounds, ownership, and
lifetime must be defined by the shared public contract before Java exposes it.
Internal RRFlow use of Arrow/DataFusion is not a reason to make the SDK depend
on DataFusion or reconstruct engine planning in Java.

## Audited current artifact

The current Maven coordinate is `io.rrflow:rrd-client:1.0.0`; the frozen
product version does not imply alpha readiness. `pom.xml` compiles for Java 25
and directly declares Jackson Databind 3.2.3 plus JUnit 6.1.3 for tests. The
resolved runtime tree is Jackson Databind 3.2.3, Jackson Core 3.2.3, and
Jackson Annotations 2.22. That locally resolved tree is
not a signed dependency lock or offline distribution.

The complete reviewed source layout is:

```text
sdks/java/
├── pom.xml
├── scripts/
│   └── generate.py
└── src/
    ├── main/java/io/rrflow/rrd/
    │   ├── package-info.java
    │   ├── ClientConfig.java
    │   ├── EndpointResolver.java
    │   ├── HttpTransport.java
    │   ├── OperationBinding.java
    │   ├── OperationExecutor.java
    │   ├── OperationId.java
    │   ├── ProtocolCodec.java
    │   ├── RequestOptions.java
    │   ├── ResourceSegment.java
    │   ├── RetryPolicy.java
    │   ├── RrdApiException.java
    │   ├── RrdClient.java
    │   ├── RrdClientException.java
    │   └── Session.java
    └── test/java/io/rrflow/rrd/
        ├── RrdClientTest.java
        └── SdkConformanceTest.java
```

A-07.1f directly produced this layout from the former 390-line
`RrdClient.java`. That class is now the 72-line public facade; every extracted
collaborator is package-private, and every current responsibility has one
source owner. This is source topology, not qualification of the incomplete
behaviors below.

`RrdClient` is a synchronous generic HTTP facade. It has helpers for
capabilities, endpoint catalogue, OpenAPI, and session creation; `call` can
address all 40 current HTTP operation identifiers. That is route reachability,
not complete operation semantics. There is no Java WebSocket, async call,
remote-TLS profile, server-cancellation path, installed endpoint resolver, or
release-qualified artifact.

The current client usefully:

- reuses one JDK `HttpClient` rather than constructing one for every request;
- restricts cleartext endpoints to credential-free loopback hosts without
  resolving arbitrary hostnames;
- explicitly disables redirects;
- validates selected canonical and correlation identifiers, resource kinds,
  and required route parameters;
- requires an idempotency key for catalogue-marked mutations;
- calculates an absolute-deadline remainder for each attempt;
- sends the same pre-encoded request bytes on client-controlled retries;
- caps encoded response bodies at 4 MiB by default and 16 MiB at maximum;
- rejects unknown fields in the common response envelope, outcome, and error;
- checks request and operation correlation for non-GET responses;
- restores the thread interrupt flag after `InterruptedException`; and
- supplies a manifest-gated shared live-conformance test.

Those are characterization foundations. They do not establish typed
operation validation, safe credentials, semantic retry, caller/server
cancellation, bounded concurrent delivery, endpoint identity, deterministic
packaging, or complete-engine conformance.

## Generated authority and operation coverage

`scripts/generate.py` invokes the repository-local `rrd-contract-export`
binary as an argument vector without a shell, reads the complete OpenAPI 3.1
document, sorts operations, and supports a byte-for-byte drift check. The
checked-in `OperationId.java` contains:

- 40 enum constants;
- wire operation name, HTTP method, route template, first authentication
  scheme, and mutation flag for each constant; and
- OpenAPI SHA-256
  `06cd649d80ec2de5fe152da49daa00533d2e5dbf56e0321fea4d86f7b432a7b3`.

An earlier flat record incorrectly said 34 routes when the generator, enum,
unit assertion, public catalogue, and OpenAPI projection contained 33. The
prompt-flight family (`flight-launch`, `flight-list`, `flight-read`) then
raised every projection to 36, and the task family (`task-open`,
`task-close`, `task-list`, `task-read`) to 40; this reference records 40.
None of these changes is a language-binding capability claim.

The generator does not emit request/result/error types, allowed route
parameters, security actions, exact success statuses, media types, causal
identity requirements, stamp/receipt bindings, or runtime validators.
`call(OperationId, Object, RequestOptions)` accepts any Jackson-serializable
payload and returns a `JsonNode`, so an enum member does not make an operation
type-safe.

The target generator emits one closed descriptor and concrete request/result
binding for every catalogued operation. Each binding carries the exact
operation ID, method, route parameters, authentication, mutation semantics,
security action, request validator, result validator, success status, media
type, and applicable causal bindings from the same executable contract
source. Coverage fails on a missing, extra, duplicate, or mismatched binding.
No handwritten Java route registry or Java-only wire dialect is permitted.

Generated Java types still require semantic validation. Record components and
Jackson types alone do not enforce string bytes, item counts, numeric domains,
nesting, discriminants, resource grammar, correlations, stamps, or digests.
Generation time, compiled bytecode, JAR contribution, heap use, adversarial
depth, and validation-error volume remain measured budgets.

## Request and response enforcement

The current request path validates the common envelope and selected
identifiers, then serializes an arbitrary object. It has no operation-specific
schema, maximum encoded request size, item/depth/string/numeric budget, or
complete pre-I/O validation. Extra route parameters are silently ignored, and
a caller can mutate a payload object while another thread is preparing it.

The current response path reads a bounded byte array and strictly checks the
outer envelope shape. It nevertheless:

- ignores response `Content-Type`;
- accepts any `2xx` status when the outcome says `ok` rather than the exact
  status declared for the operation;
- does not correlate GET request and operation identities;
- accepts any object as a successful operation payload;
- accepts arbitrary error-code strings and coerces error-detail values to
  strings;
- does not bind protocol, instance, catalogue, OpenAPI, schema, read stamp,
  resource, digest, or receipt completely; and
- does not explicitly budget headers, JSON nesting, tokens, object count,
  strings, decoded values, or simultaneous encoded/decoded heap.

A temporary same-package characterization test, removed after execution,
submitted an invalid unpinned query, dropped the first connection, and served
`201 text/plain` with an invalid success payload on the second attempt. The
client made two attempts and returned the invalid object. H-04 preserves this
case as a negative test: the final client rejects the request before I/O and
rejects the response status, media type, and payload before returning a value.

Direct convergence validates a complete concrete request before network I/O
and before exceeding its allocation budget. On response it enforces declared
status and media type, bounds headers plus encoded and decoded representation,
decodes once under explicit parser constraints, validates the common envelope
and concrete operation result, and checks every applicable identity, stamp,
digest, and receipt. Duplicate members, unknown variants, invalid Unicode,
excessive depth, non-finite/out-of-range numbers, and coerced types fail
closed. Invalid representation is never reclassified as a transient transport
failure.

## Endpoint, HTTP transport, and concurrent use

The constructor currently accepts only `http:` at `localhost` or a loopback IP
with no URL credentials, query, or fragment. That is an intentionally narrow
local profile. It has no HTTPS, mutual TLS, installed mesh identity, endpoint
rotation, or authenticated network profile. A configured base path remains
part of route resolution but is not bound to installation identity.

The default `HttpClient` builder sets only redirect denial. Java 25 otherwise
permits a default proxy selector, default SSL context, preferred HTTP/2,
implementation-managed connection pools, and an unexposed default executor.
The client does not set a connect timeout and `RrdClient` does not expose
`HttpClient.close()`, even though Java 25 defines the client as
`AutoCloseable`. An injected arbitrary `HttpClient` is a useful test seam but
cannot qualify transport identity or production resource policy.

The accepted carriage is selected only after its supported JDKs and per-client
limits are measured. The JDK client may remain the implementation if the
RRFlow layer can prove bounded endpoint count, in-flight work, executor,
connections, idle lifetime, proxy behavior, headers, body streaming, TLS,
protocols, and close behavior without mutating unsafe process-global policy.
If it cannot, RRFlow selects and bundles a maintained transport dependency.
It does not hand-write HTTP/WebSocket framing merely to claim no dependencies.

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

Network reachability, Zuul Zero/Wardenclyffe mesh membership, TLS, a Java
thread, or a project directory never grants RRFlow permission and never
initializes a database.

The target `RrdClient` is immutable after construction, safe to share across
threads, and `AutoCloseable`. It owns a bounded executor and deterministic
shutdown; caller-supplied executors and transports have explicit ownership.
Request bytes and options are frozen before asynchronous dispatch. Concurrency
tests cover calls, cancellation, renewal/rotation, close races, queue
exhaustion, and subscriptions rather than only independent loopback handlers.

## Retry, deadlines, interruption, and outcome certainty

The current client-controlled attempt rule is:

```text
GET OR operation is not marked mutation OR idempotency key is present
```

That replays every non-mutating POST, including a query that has no pinned read
coordinate and can observe different state. Default attempts are two and the
accepted constructor range is one through eight. Retries are immediate, use
no backoff/jitter or endpoint-rotation policy, and produce no attempt evidence
or uncertain-observation result. Request timeouts and connection failures both
arrive through `IOException`, so a per-attempt timeout can replay work while
the semantic deadline still has time remaining.

The client correctly preserves the interrupt bit when blocking `send` throws
`InterruptedException`, but interruption proves only that local waiting was
interrupted. It does not prove the server stopped compute or that a mutation
did not commit.

The target follows the shared
[operation-certainty matrix](rust.md#retry-deadline-and-outcome-certainty): a
safe public discovery may retry a classified transient failure; a read may
replay only when the engine preserves its accepted read coordinate; and a
mutation may replay only as identical bytes and coordinates bound to a durable
engine receipt. A lost response otherwise becomes a typed uncertain
observation or mutation outcome requiring explicit reconciliation. Server
`retryable` metadata never overrides operation semantics.

Async work uses an RRFlow call handle that binds request identity, local
future, semantic deadline, and server-cancellation state. Raw
`CompletableFuture.cancel` is insufficient authority: Java documents
cancellation as exceptional completion because a `CompletableFuture` does not
control the underlying computation. B-04 supplies the correlated server cancel
message and terminal evidence shape; H-04 implements its Java carriage.
Blocking helpers wait on the same async semantic operation and preserve typed
interruption, deadline, local cancellation, server cancellation, and
uncertain-outcome distinctions.

## Credentials, errors, and causal evidence

`RequestOptions.ApiKey`, `Session`, and `Session.SessionLease` are public
records containing credential strings. Java records synthesize `toString`
from every component value, and Jackson discovers the public components. The
review probe confirmed that a bearer appeared in both `Session.toString()` and
Jackson serialization. Defensive copying elsewhere does not repair this
secret boundary.

The target uses opaque final credential/session handles, not records or public
token accessors. They provide deliberately redacted `toString`, do not
implement Java serialization, are rejected by the SDK's default JSON mapper,
and expose no API-key, bearer, private-key, or authorization-header value.
Credential providers perform only the minimum header operation and never
become a second session store. Secrets do not enter exceptions, logs, traces,
metrics, URLs, process arguments, object inspection, request excerpts, or
equality/hash output. Java `String` cannot be reliably zeroed, so the SDK
makes no false erasure claim and avoids unnecessary copies where supported.

Errors become a closed phase-aware hierarchy beneath `RrdClientException`:
local contract, endpoint resolution, TLS identity, transport, pool/resource
exhaustion, deadline, thread interruption, caller/server cancellation,
protocol, API denial, response validation, and uncertain observation or
mutation outcome remain distinguishable. Bounded structured server fields and
safe causes are retained without embedding arbitrary messages, URLs, headers,
or transport dumps in public error text.

Every operation propagates canonical request and operation IDs plus valid W3C
`traceparent`/`tracestate`. Logical attempts are children of one semantic
operation; executor wait, DNS, connect, TLS, pool, headers, body, reconnect,
and endpoint rotation are bounded attributes or causal links. Java Flight
Recorder, OpenTelemetry, or application logging may observe those facts only
through the H-05 redacted adapter. Telemetry never becomes completion truth,
durable lifecycle state, or hidden model reasoning.

## Multiplexed WebSocket delivery

The current Java artifact has no WebSocket implementation or test. B-04 owns
the closed language-neutral multiplexed protocol and Rust reference
implementation; H-04 must supply Java's qualified carriage and idiomatic API.

Java 25's JDK WebSocket API provides explicit receive demand through
`request(n)`, sequential listener callbacks per socket, asynchronous sends,
and an abrupt `abort`. It also rejects a concurrent pending text/binary send.
Those primitives are useful, but they do not by themselves implement RRFlow
message identity, reassembly limits, queues, ACK durability, reconnect, or
terminal engine meaning. The carriage is retained only if tests prove the
required bounds before unsafe allocation; otherwise a maintained bundled
implementation is selected through the same conformance port.

The resulting Java subscription client must:

- carry request/response, cancellation, subscription, delivery, cumulative
  ACK, heartbeat, backpressure, error, and terminal messages over one
  negotiated connection;
- enforce frame, reassembled-message, receive-demand, send, in-flight, queue,
  parser, and decompression limits before allocation where the carriage can;
- validate protocol, connection generation, request/subscription identity,
  sequence, cursor, and bounded error on every message;
- serialize pending sends explicitly and define listener/executor ownership;
- resume only from a durably acknowledged cursor and reject stale generations;
- distinguish socket close, local cancellation, server cancellation, and
  terminal engine outcome; and
- share endpoint identity, sessions, operation bindings, errors, and traces
  with HTTP without adding Java-owned lifecycle state.

## Maven, JAR, and offline distribution boundary

The present POM pins compiler and Surefire plugins but inherits other lifecycle
plugins and has no dependency-convergence, reproducibility, source/Javadoc,
consumer, signature, SBOM, or provenance gate. Invoking the unconfigured
`dependency:tree` goal during review contacted Maven Central to resolve a
plugin and its dependencies. That is development behavior, not an acceptable
default install or release-verification path.

Two consecutive clean package builds from unchanged source produced different
JAR SHA-256 values. The POM has no `project.build.outputTimestamp`, and the
artifact has neither `module-info.class` nor `Automatic-Module-Name`; Java
therefore derived the automatic module name `rrd.client` from the filename.
The JAR contains only compiled classes plus Maven metadata. No source or
Javadoc JAR was produced.

An empty temporary Maven repository with offline mode failed before compilation
because even the resources plugin was absent. A warm developer cache can make
`mvn --offline` pass, but it is not an independently reproducible dependency
closure.

The target POM and release process:

- pin every build/test/release plugin actually invoked and enforce Java/Maven
  plus dependency convergence explicitly;
- set a revision-bound `project.build.outputTimestamp` and compare two clean
  builds byte for byte;
- publish a stable `Automatic-Module-Name: io.rrflow.rrd` unless an explicit
  module descriptor passes both classpath and module-path consumers;
- generate warning-clean Javadoc and deterministic main, source, and Javadoc
  JARs plus the exact POM;
- record the full effective dependency/plugin/toolchain graph, licenses, SBOM,
  checksums, and provenance in the signed RRFlow distribution;
- install those artifacts into an empty temporary file-backed Maven repository
  and compile/run an external consumer with all network access denied; and
- test the declared minimum and current JDK/Maven matrix on the J-owned Linux,
  Windows, macOS, and architecture set.

The signed offline RRFlow distribution is the default release authority. It
contains the prebuilt Java artifacts and every runtime dependency byte; engine
installation and readiness never require Java or Maven. A source consumer may
use the bundle's verified file repository and supported toolchain without
Central, another checkout, or a prewarmed user cache. Maven Central, if later
used, is an optional byte-identical projection and never the only installation
path.

## Conformance that actually counts

Current evidence is deliberately bounded:

| Command or probe | Observed result | Honest boundary |
|---|---|---|
| `mvn --batch-mode -f sdks/java/pom.xml test` | Three loopback unit tests passed; the manifest-gated test was reported as one skip. | Compilation plus selected retry/auth/envelope/error checks; no configured engine corpus. |
| `python3 sdks/java/scripts/generate.py --check` | Checked-in enum matched 40 operations and the OpenAPI digest. | Method/path/auth/mutation projection only. |
| full shared conformance runner | Java's configured test passed with corpus SHA-256 `b3977c57c8d268f861e9d5158e5609bf3e7d2e1db01f914a12911b95c3404cb2`. | Real HTTP against one direct-seeded rrflowKV daemon fixture; not installation, rrflowMX parity, or complete labelled behavior. |
| temporary adversarial test | Replayed an unpinned invalid query, accepted `201 text/plain` plus invalid payload, and exposed the bearer through record formatting and Jackson. | Reproducible open H-04 correctness/security defects. |
| two clean Maven packages | Both built, but their JAR SHA-256 values differed. | Current artifact is not byte-reproducible. |
| empty-repository offline Maven test | Failed resolving the inherited resources plugin before compilation. | The checkout does not contain a verified offline Maven closure. |
| `jar --describe-module` | Reported a derived `rrd.client@1.0.0` automatic module. | No stable explicit module contract or classpath/module-path consumer proof. |

The live corpus currently exercises capability retry/version rejection,
catalogue count, one authentication denial, session create/renew/close, vector
ensure/search, transaction begin/preview/abort/commit, one query, changefeed
read/follow/local timeout, backup create/list, and estate read. Its declared
domain labels overstate CRUD, backup/restore, vector, live delivery,
typed-error, retry/failure, cancellation, and version/artifact coverage as
catalogued in the [Rust reference](rust.md#conformance-that-actually-counts).

No row proves RRFlow's persistent multi-model reasoning/recall engine. Release
conformance starts a D-01-installed project instance through public operations,
runs structural scenarios against rrflowMX and rrflowDB/rrflowKV, adds durable
crash/reopen cases, and compares result, denial, `ReadStamp`, plan/projection
digest, transaction receipt, reasoning/context evidence, trace, and resource
accounting across HTTP, WebSocket, Rust, Java, every other supported SDK, CLI,
MCP, GraphQL, and Connectome. The engine corpus exercises native
document/temporal-graph/scalar/BM25/vector access, bounded RRF context
selection, streamed Arrow/DataFusion analytics, and persisted
reasoning/feedback. Java proves only faithful access to that engine.

## Current and gated file plan

Java remains one client artifact and one client namespace; responsibility
splits do not create another runtime. A-07.1f established this current source
tree, and the generated enum remains subordinate to the executable public
contract:

```text
sdks/java/
├── pom.xml
├── scripts/
│   └── generate.py
└── src/
    ├── main/java/io/rrflow/rrd/
    │   ├── package-info.java            # public boundary and canonical links
    │   ├── ClientConfig.java            # current validated construction
    │   ├── EndpointResolver.java        # current loopback route resolution
    │   ├── HttpTransport.java           # current bounded response carriage
    │   ├── OperationBinding.java        # current common request binding
    │   ├── OperationExecutor.java       # current synchronous coordination
    │   ├── OperationId.java             # catalogue-derived identifier map
    │   ├── ProtocolCodec.java            # current partial envelope codec
    │   ├── RequestOptions.java          # per-call semantic coordinates
    │   ├── ResourceSegment.java         # typed public resource component
    │   ├── RetryPolicy.java             # current broad replay/deadline rule
    │   ├── RrdApiException.java         # typed engine denial
    │   ├── RrdClient.java               # narrow synchronous public facade
    │   ├── RrdClientException.java      # closed redacted error root
    │   └── Session.java                 # current plain secret-bearing record
    └── test/java/io/rrflow/rrd/
        ├── RrdClientTest.java
        └── SdkConformanceTest.java
```

The following files remain deliberately absent until their assigned gate adds
real behavior: `RrdCall.java`, `Subscription.java`, and
`WebSocketTransport.java` (H-04); `generated/OperationModels.java` (H-04);
`OperationCoverageTest.java` and `ProtocolValidationTest.java` (H-04);
`TransportFaultsTest.java` (H-04/H-07/J-02);
`SubscriptionTest.java` and `ConcurrencyTest.java` (H-04/J-02); and
`PackageConsumerTest.java` (J-03/J-05).

The dependency order is:

1. **A-07 SDK split (completed by A-07.1f):** the former
   `RrdClient.java` responsibilities are directly split across config,
   endpoint, HTTP transport, binding, executor, codec, and retry files, with
   package documentation added. Generation, loopback/redirect denial,
   envelope construction, mutation idempotency, response byte limit,
   interrupt preservation, explicit manifest-absent skip, current public
   types, and unit tests are preserved. No validation, async, socket, remote
   resolver, or artifact success was invented.
2. **B-04 (contract implemented):** consume the closed multiplexed golden
   protocol; no Java behavior is claimed by the Rust reference carriage.
3. **D-01:** replace direct fixture seeding with installed public bootstrap and
   endpoint identity.
4. **H-04:** implement bounded multiplexed Java carriage, call/subscription
   APIs, and listener/executor/queue ownership; generate concrete operation
   models/bindings; enforce exact request/result/error/status/media/identity
   contracts, semantic certainty, opaque credentials, W3C propagation, and
   structural cross-surface cases.
5. **H-07:** add authenticated HTTPS/mTLS/mesh endpoint resolution and
   rotation without making reachability authority.
6. **J-02/J-03/J-05:** pass fault/resource, minimum/current JDK and Maven,
   OS/architecture/concurrency, clean-consumer, byte-reproducible artifact,
   empty-repository offline, provenance, and release verification.

## Primary constraints

- [Java 25 `HttpClient`](https://docs.oracle.com/en/java/javase/25/docs/api/java.net.http/java/net/http/HttpClient.html)
  defines reusable synchronous/asynchronous clients, connection pooling,
  default proxy/executor behavior, and close semantics; RRFlow configures,
  owns, and measures those resources explicitly.
- [Java 25 `HttpClient.Builder`](https://docs.oracle.com/en/java/javase/25/docs/api/java.net.http/java/net/http/HttpClient.Builder.html)
  exposes redirect, proxy, connect-timeout, TLS, protocol, and executor inputs;
  absence of explicit inputs is not an RRFlow transport profile.
- [Java 25 `CompletableFuture`](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/util/concurrent/CompletableFuture.html)
  treats cancellation as exceptional completion because it does not control
  the computation; RRFlow therefore requires correlated server cancellation
  and outcome evidence separately.
- [Java 25 records](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/lang/Record.html)
  derive string representation from component values; secret-bearing values
  are not transparent records.
- [Java 25 WebSocket](https://docs.oracle.com/en/java/javase/25/docs/api/java.net.http/java/net/http/WebSocket.html)
  defines receive demand, pending sends, close, and abort, while
  [its listener contract](https://docs.oracle.com/en/java/javase/25/docs/api/java.net.http/java/net/http/WebSocket.Listener.html)
  defines sequential callbacks; B-04 supplies the RRFlow protocol, identity,
  queue, ACK, resume, and resource semantics that H-04 must carry in Java.
- [Maven repository and offline behavior](https://maven.apache.org/guides/introduction/introduction-to-repositories)
  distinguishes a local cache from remote/file repositories and offline mode;
  J-05 proves an empty-cache file-backed closure rather than a warm-cache run.
- [Maven reproducible-build guidance](https://maven.apache.org/guides/mini/guide-reproducible-builds.html)
  requires reproducibility-capable plugins and `project.build.outputTimestamp`;
  RRFlow additionally compares exact artifacts and binds their provenance.
- [Maven dependency convergence](https://maven.apache.org/components/enforcer/enforcer-rules/dependencyConvergence.html)
  detects divergent transitive versions; the signed manifest still owns exact
  artifact bytes and checksums.
- [The JAR specification](https://docs.oracle.com/en/java/javase/25/docs/specs/jar/jar.html)
  defines explicit and derived automatic-module identity; RRFlow publishes one
  stable tested module name.
- [RFC 9110 HTTP semantics](https://www.rfc-editor.org/rfc/rfc9110.html#name-idempotent-methods)
  bounds replay and representation handling; stamped reads and durable mutation
  receipts impose stricter RRFlow semantics.
- [RFC 6455 implementation limits](https://www.rfc-editor.org/rfc/rfc6455.html#section-10.4)
  require protection against oversized frames and reassembled messages; B-04
  defines RRFlow queue, identity, cursor, generation, and backpressure bounds.
- [W3C Trace Context](https://www.w3.org/TR/trace-context/) defines
  `traceparent` and `tracestate`; RRFlow owns validation, causal links,
  redaction, and evidence meaning.
