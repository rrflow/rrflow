# RRFlow Python SDK

**Status:** active implementation reference; alpha SDK and package qualification are incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/sdk/python`
**Owner:** Python client generation, synchronous/asynchronous API behavior, transport, operation validation, retry, cancellation, subscription, credential, packaging, and conformance behavior

`sdks/python` is the current Python client foundation for the public RRD
protocol. It is an outward client of the one RRFlow engine, not a Python query,
context, reasoning, storage, or lifecycle implementation. Authentication,
authorization, read-stamp selection, transactions, rrflowMX, durable
rrflowDB/rrflowKV access, native graph/BM25/vector indexes,
rrflowQL/Arrow/DataFusion execution, reasoning-tree mutation, installation, and
attunement remain behind `RrdEngine`.

The [public contract](../protocol/public-contract.md) owns operation and wire
vocabulary. The [engine data flow](../../architecture/engine-data-flow.md)
owns what occurs behind each public operation. The
[roadmap](../../roadmap/rrflow-1.0.md) alone owns acceptance. The
[Rust SDK reference](rust.md) owns the language-neutral operation-certainty,
cancellation, trace, and cross-surface requirements; this record applies them
to Python without copying Rust implementation choices.

## Required end-to-end boundary

```text
Python application
  -> installed Python SDK + explicit endpoint/security profile
  -> HTTP or bounded multiplexed WebSocket RRD transport
  -> RrdEngine authentication, authorization, stamp, planning, and transaction
  -> rrflowMX or rrflowDB/rrflowKV + graph/BM25/vector + rrflowQL/DataFusion
  -> one validated result/denial/receipt with the same IDs, stamp, and trace
```

A generated operation literal, Pydantic envelope, passing mock, built wheel,
or successful HTTP call does not prove the last two lines. The SDK cannot
choose a storage profile, raw key, physical index, DataFusion plan, model
backend, application database, or reasoning mutation. It expresses public
semantic intent and declared budgets and validates the engine's public result.

## Audited current package

The package requires Python 3.15 or later and has two runtime dependencies:
HTTPX 0.28.1 and Pydantic 2.14.1. `pyproject.toml` pins Hatchling 1.32.4 as
the build backend, while the uv project lock fixes Ruff 0.17.1, mypy 2.4.0,
and pytest 9.1.1 for development. The reviewed executable layout is:

```text
sdks/python/
├── pyproject.toml
├── uv.lock
├── scripts/generate.py
├── src/rrd_client/
│   ├── __init__.py
│   ├── client.py
│   ├── endpoint.py
│   ├── error.py
│   ├── operation.py
│   ├── retry.py
│   ├── session.py
│   ├── transport.py
│   ├── py.typed
│   └── generated/
│       ├── __init__.py
│       └── endpoints.py
└── tests/
    ├── test_client.py
    └── sdk_conformance.py
```

`RrdClient` is a synchronous HTTP client. It exposes ergonomic calls for
capabilities, endpoint catalogue, OpenAPI, and session creation plus one
generic `call(OperationId, payload, options)`. That generic call can address
all 40 current HTTP operation identifiers. This is routing coverage, not
operation-semantic coverage. There is no asynchronous client and no WebSocket
implementation; subscription administration and changefeed follow are
ordinary HTTP calls.

The A-07.1d split is direct rather than a forwarding layout. `client.py` owns
only synchronous construction, discovery helpers, and generic dispatch;
endpoint validation, errors, request/resource construction, attempts and
deadlines, the current session representation, and response carriage each
have one named module. The former catch-all `models.py` does not remain as an
alias. This makes the current defects visible at their future correction
boundaries; it does not qualify those boundaries.

The current client usefully:

- restricts cleartext endpoints to credential-free loopback hosts;
- explicitly disables HTTP redirect following;
- uses a reusable HTTPX client rather than creating a connection per call;
- validates selected canonical identifiers and common resource segments;
- requires an idempotency key for operations marked as mutations;
- caps decoded streamed response bytes before JSON parsing;
- rejects unknown common-envelope fields through frozen Pydantic models;
- checks request and operation correlation for non-GET calls; and
- closes the HTTP client through a context manager or explicit `close()`.

Those foundations remain characterization evidence. They do not establish
remote transport, complete operation validation, secret safety, semantic
retry, server cancellation, bounded WebSocket delivery, installed-project
resolution, or release support.

## Generated authority and operation coverage

`scripts/generate.py` invokes the repository-local `rrd-contract-export`
binary as an argument vector without a shell. It reads the complete OpenAPI
3.1 document, derives an `OperationId` `Literal` and endpoint map, formats the
result with the locked Ruff executable, and compares the complete generated
file byte for byte in check mode.

The current generated projection contains:

- 47 HTTP operations;
- method, route template, first authentication scheme, and mutation flag for
  each operation; and
- OpenAPI SHA-256
  `06cd649d80ec2de5fe152da49daa00533d2e5dbf56e0321fea4d86f7b432a7b3`.

It does not generate request models, response models, error-code constraints,
path-parameter sets, security actions, exact success statuses, media types,
stamp/receipt identities, or runtime validators. `OperationId` and
`TypedDict` improve static checking but disappear at runtime, and the public
`call()` returns `dict[str, Any]` for every operation.

The target generator emits one closed runtime descriptor for every catalogue
operation. Each descriptor binds the exact operation identifier, method,
route parameters, authentication, mutation semantics, security action,
request validator, response validator, success status, and media type from the
same contract source. A coverage test fails for a missing, extra, or mismatched
binding. No handwritten second schema or client-only route registry is
permitted.

Generated Pydantic models or equivalent measured validators must reject
untrusted runtime values before I/O and before returning a result. The chosen
representation must pass import-time, wheel-size, heap, adversarial-depth,
item-count, string-size, and error-size budgets. Static annotations alone are
not runtime validation.

## Request and response enforcement

The current request path validates only parts of the common envelope. Payloads
are copied from an arbitrary mapping without operation-specific validation.
Unused path parameters are ignored. Direct generic API-key use rejects
`None`, but not an empty credential. A public `Session` accepts an arbitrary
mutable lease dictionary, and missing lease keys surface as ordinary Python
key errors rather than closed client errors.

The current response path validates a common envelope whose successful
payload is `Any`. It then checks only that the payload is a dictionary. The
remaining defects are concrete:

- response `Content-Type` is ignored;
- any successful `2xx` agrees with an `ok` outcome, rather than the exact
  operation status;
- GET response request and operation identities are not checked;
- successful payload schemas and semantic validators are not applied;
- error codes and strings are not closed or length bounded;
- error details are parsed but discarded from `RrdApiError`;
- protocol, instance, catalogue, OpenAPI, schema, stamp, resource, digest,
  and receipt bindings are incomplete; and
- decoding materializes chunk objects, their joined bytes, decoded text, JSON,
  and Pydantic objects, so the configured byte limit is not a peak-memory
  limit.

A reproducible injected-transport probe sent an invalid `query-execute`
payload, dropped the first response, then returned `201`, `text/plain`, and an
invalid success payload. The client replayed the unpinned query and returned
`{"invalid": true}`. H-04 must convert every part of that observation into a
negative test.

Direct convergence validates the complete request before network I/O. On
response it enforces the declared status and media type, limits encoded and
decoded bytes before allocation, parses once, validates the full common and
operation payload, and verifies every applicable causal identity before
exposing a typed value. Invalid representation is never recast as transient
transport loss.

## Endpoint, bootstrap, and HTTP transport

The constructor currently accepts only `http:` at `localhost` or an IP
address classified as loopback, with no URL credentials, query, or fragment.
That is an intentionally narrow local profile. It has no HTTPS, mutual TLS,
installed-mesh identity, endpoint rotation, or authenticated remote profile.
It permits a base path and resolves operation routes with `urljoin`, so the
configured path can alter where nominal root routes are sent.

Although redirects are disabled, the underlying HTTPX client currently keeps
its default environment-trust behavior and default connection-pool limits.
The SDK therefore has no qualified statement about environment proxies,
certificate inputs, maximum connections, keep-alive connections, or pool
waits. These become explicit installed transport configuration and resource
budgets; ambient host settings cannot silently change endpoint identity or
trust.

A qualified connection performs these steps:

1. resolve bounded endpoint candidates and expected transport/RRD identities
   from the D-01-installed project binding;
2. connect using the selected loopback or authenticated network profile;
3. reject redirects and verify the final endpoint and transport identity;
4. call liveness and readiness;
5. negotiate protocol, exact instance, catalogue/OpenAPI digests, limits, and
   available capabilities; and
6. authenticate an application session before protected work.

Network reachability, Wardenclyffe/Zuul Zero mesh membership, TLS, liveness,
or a project directory never grants RRFlow permission and never initializes a
database. Endpoint resolution is an outward adapter; installation and
authorization remain engine-owned.

## Synchronous and asynchronous APIs

Python needs both a blocking API for scripts and a native asynchronous API for
servers, agents, MCP clients, and Connectome-side services. The target exposes
`RrdClient` and `AsyncRrdClient` as thin carriage/lifecycle facades over one
generated operation, validation, retry, error, session, endpoint, and trace
implementation. It does not maintain two semantic clients or implement the
async API by hiding calls in a thread pool.

Both clients reuse bounded connection pools and have explicit close/context
manager behavior. The asynchronous client is safe to share across tasks, does
not instantiate clients in a hot loop, and supports structured task
cancellation. Local task cancellation ends waiting; it is not evidence that
the engine stopped or that a mutation did not commit.

## Retry, deadline, cancellation, and outcome certainty

The current attempt rule is:

```text
GET OR operation is not marked mutation OR idempotency key is present
```

That rule replays every non-mutating POST, including an unpinned query that can
observe a different read stamp. It retries HTTPX transport and timeout errors
immediately, without bounded backoff, jitter, attempt evidence, endpoint
rotation policy, or an uncertain-observation result. Each attempt receives a
local HTTPX timeout derived from the wall-clock deadline. There is no external
cancel handle or correlated server cancellation operation.

The target uses the shared
[operation-certainty matrix](rust.md#retry-deadline-and-outcome-certainty):
public safe discovery may retry classified transient failures; a read may
replay only when the engine preserves the same accepted read coordinate; and a
mutation may replay only as identical bytes bound to a durable engine
operation/idempotency receipt. Otherwise a lost response returns a typed
uncertain observation or outcome for explicit reconciliation.

HTTPX connect/read/write/pool timeouts, the overall RRFlow deadline, caller or
task cancellation, the B-04 server-cancellation request, and a terminal engine
outcome are separate facts. H-04 implements that carriage in Python. Backoff,
jitter, maximum attempts, endpoint
rotation, and total elapsed time are bounded inputs and observable evidence,
not process-global policy. An error's server `retryable` field or Python
exception message never overrides operation semantics.

## Credentials, errors, and causal evidence

`Session` is currently a frozen dataclass, but its `lease` is a public mutable
dictionary containing the bearer. Freezing the outer attribute does not make
that secret opaque. The adversarial probe serialized the entire synthetic
bearer with `dataclasses.asdict`; normal dataclass representation also exposes
the dictionary. `RequestOptions.api_key` is another plain string.

The target session handle is opaque, redacted, non-serializable by default,
and does not expose a token-bearing mapping. Credential providers perform only
the minimum operation needed to authorize a request and do not become another
session store. API keys, bearer tokens, session identifiers, private keys, and
authorization headers never enter `repr`, `str`, exceptions, traces, metrics,
URLs, process arguments, dataclass/Pydantic dumps, pickles, or response
excerpts.

Client errors form a closed phase-aware hierarchy: local contract, endpoint
resolution, TLS identity, transport, pool/resource exhaustion, deadline,
caller/server cancellation, protocol, API denial, response validation, and
uncertain observation or mutation outcome. They retain safe bounded causes and
server details without embedding uncontrolled transport strings or secrets.

Every operation propagates canonical request and operation IDs plus valid W3C
`traceparent`/`tracestate`. Attempts are child spans of one semantic operation;
reconnects and endpoint rotation are causal links. Trace output is redacted
and bounded and never becomes completion truth, durable state, or hidden model
reasoning.

## Multiplexed WebSocket delivery

The Python package currently has no WebSocket dependency, source, or test.
B-04 defines the closed language-neutral multiplexed protocol and proves the
Rust reference implementation. H-04 must implement Python state and carriage,
selecting and locking a maintained asyncio-compatible library only after
measuring its dependency, platform, frame, memory, cancellation, and TLS
behavior; this record does not choose a library by assertion.

The resulting client must:

- route request/response, cancellation, subscription, delivery, cumulative
  ACK, heartbeat, backpressure, error, and terminal frames over one negotiated
  connection;
- enforce frame, reassembled-message, receive, send, in-flight, queue, decode,
  and decompression limits before allocation;
- validate protocol, connection generation, request/subscription identity,
  sequence, cursor, and bounded error on every message;
- resume only from a durably acknowledged cursor and reject stale generations;
- distinguish socket closure, local cancellation, server cancellation, and
  terminal engine outcome; and
- share endpoint identity, sessions, operation bindings, errors, and traces
  with HTTP without creating Python-owned lifecycle state.

## Installable package boundary

An offline `uv build` currently succeeds. Under Python 3.14.4 and uv 0.11.21,
the A-07.1d output was a 9,294-byte pure-Python wheel and a 48,486-byte source
distribution. The wheel has 14 entries; the source distribution has 18. Under
Python 3.15.0 and uv 0.13.1 on 2026-10-16, the output was a 12,654-byte wheel
with 15 entries and a 33,113-byte source distribution with 20. Each build
contains `rrd_client/py.typed` and omits the removed `rrd_client/models.py`.
That proves the intended file projection, not reproducibility or consumer
typing behavior.

Project metadata still lacks the deliberate license, readme, authorship,
project-link, support, and artifact-provenance surface needed for release. The
exact runtime dependency pins make this checkout repeatable but have not been
justified as a compatible consumer-library range. Only Python 3.14.4 was
exercised in that review; the declared minimum is now Python 3.15, the one
interpreter candidate CI runs. Normalized archive timestamps aid
reproducibility, but one local build is not a reproducible-build claim.

The qualified distribution contains an sdist and pure-Python wheel produced
from one source identity, includes `py.typed`, has a closed included-file set,
records complete metadata, and passes artifact-content and installed-consumer
tests. RRFlow's signed offline bundle contains the selected wheel and complete
hashed dependency closure, so installation does not require a registry, build
backend, Cargo, sibling checkout, contract generation, or runtime download.
An explicitly selected registry projection may mirror the same artifacts but
is not the sole deployment path.

Supported minimum and current CPython versions run import, type-consumer,
sync/async HTTP, cancellation, TLS, WebSocket, and live conformance tests on
Linux, Windows, and macOS as required by J. Dependency constraints are tested
against their admitted range, while the release bundle locks exact artifacts.

## Conformance that actually counts

Current evidence is deliberately bounded:

| Command or probe | Observed result | Honest boundary |
|---|---|---|
| `uv lock --check` | resolved the locked 24-package environment | Lock consistency only; not clean offline installation or supported-version coverage. |
| generator, Ruff, mypy, and pytest package checks | generation passed; Ruff passed over 13 Python files; strict mypy passed over 10 source files; 4 mock-focused tests passed | Source hygiene and selected mocked HTTP behavior; no async, WebSocket, TLS, installed engine, or fault matrix. |
| conformance entry without `RRD_SDK_CONFORMANCE_MANIFEST` | exited 1 with the required-manifest error | Correct fail-closed harness configuration; no scenario executed. |
| conformance entry against the live example harness | passed with corpus SHA-256 `b3977c57c8d268f861e9d5158e5609bf3e7d2e1db01f914a12911b95c3404cb2` | Real HTTP against the present direct-seeded rrflowKV fixture; not installation, rrflowMX parity, or full labelled behavior. |
| generated-surface parity | 40 descriptors match the OpenAPI digest | Method/path/auth/mutation projection only; not runtime binding or semantic execution. |
| injected HTTPX transport probe | replayed an unpinned query, accepted `201 text/plain` and invalid payload, serialized bearer | Reproducible open H-04 correctness/security defects. |
| offline package build | 9,294-byte wheel and 48,486-byte sdist built; both contain `py.typed` and omit the removed catch-all module | Artifact creation/content inventory only; no clean consumer, dependency closure, reproducibility, signing, or platform matrix. |

The live script currently exercises capability retry/version rejection,
catalogue count, one authentication denial, session create/renew/close, vector
ensure/search, transaction begin/preview/abort/commit, one query, changefeed
read/follow/local deadline, backup create/list, and estate read. Its shared
domain labels overstate CRUD, backup/restore, vectors, live delivery, typed
errors, retry/failure gaps, cancellation, and version/package coverage as
catalogued in the [Rust reference](rust.md#conformance-that-actually-counts).

No row proves RRFlow's persistent multi-model reasoning/recall system.
Release conformance starts a D-01-installed project instance through public
operations, runs structural scenarios against rrflowMX and rrflowDB/rrflowKV,
adds durable crash/reopen cases, and compares the same result, denial,
`ReadStamp`, plan/projection digest, transaction receipt, reasoning/context
evidence, trace, and resource accounting across HTTP, WebSocket, Rust, Python,
every supported SDK, CLI, MCP, GraphQL, and Connectome. The engine corpus must
exercise native document/temporal-graph/scalar/BM25/vector access, bounded RRF
context selection, streamed Arrow/DataFusion analytics, and persisted
reasoning/feedback; the Python client merely proves faithful access to it.

## Direct-convergence file plan

A-07.1d has frozen the current synchronous responsibilities without keeping a
forwarding module. The executable source layout is:

```text
sdks/python/src/rrd_client/
├── __init__.py              # narrow public exports
├── client.py                # synchronous construction and public calls
├── endpoint.py              # current loopback endpoint policy
├── error.py                 # current client and API error types
├── operation.py             # request coordinates and resource validation
├── retry.py                 # current broad attempt/deadline rule
├── session.py               # current public bearer-bearing representation
├── transport.py             # bounded HTTP response/envelope handling
├── py.typed
└── generated/
    ├── __init__.py
    └── endpoints.py

sdks/python/tests/
├── test_client.py
└── sdk_conformance.py
```

`async_client.py`, `subscription.py`, generated `models.py`, and the planned
operation-coverage, protocol-validation, transport-fault, async-client, and
package-consumer tests deliberately do not exist yet. They are created only
with the real behavior and negative evidence owned by their later gates.

The dependency order is:

1. **A-07:** completed for this package: the current synchronous behavior is
   split into the accepted seams, deterministic generation and mock
   characterization remain, `py.typed` is packaged, and `models.py` is absent.
   `test_client.py` remains the honest characterization corpus until later
   gates implement the direct negative-test seams; no async, socket, resolver,
   or validation success was invented.
2. **B-04 (contract implemented):** consume the closed multiplexed golden
   protocol; no Python behavior is claimed by the Rust reference carriage.
3. **D-01:** replace direct fixture seeding with installed public bootstrap and
   endpoint identity.
4. **H-04:** generate complete models/bindings, implement the native async
   client and bounded multiplexed WebSocket carriage, enforce request/result/
   error/status/media/identity contracts, semantic certainty, opaque
   credentials, W3C propagation, and structural cross-surface scenarios.
5. **H-07:** add authenticated HTTPS/mTLS/mesh endpoint resolution and rotation
   without making reachability authority.
6. **J-02/J-03/J-05:** pass adversarial resource/failure tests, supported
   interpreter/platform matrices, reproducible signed artifacts, network-
   denied installation, consumer typing, and release verification.

## Primary constraints

- [HTTPX clients](https://www.python-httpx.org/advanced/clients/) establish
  connection pooling and explicit client lifetime as the reusable transport
  foundation.
- [HTTPX asynchronous support](https://www.python-httpx.org/async/) supplies
  native async streaming and warns against client construction in a hot loop;
  RRFlow keeps sync and async semantics in one operation layer.
- [HTTPX timeouts](https://www.python-httpx.org/advanced/timeouts/) distinguishes
  connect, read, write, and pool waits. RRFlow additionally owns one semantic
  deadline, server cancellation, and outcome certainty.
- [HTTPX resource limits](https://www.python-httpx.org/advanced/resource-limits/)
  provides pool bounds; release values are explicit RRFlow configuration and
  measured tests rather than inherited defaults.
- [RFC 9110 HTTP semantics](https://www.rfc-editor.org/rfc/rfc9110.html#name-idempotent-methods)
  bounds automatic replay, status, and representation handling; RRFlow's
  stamped read and durable receipt rules are stricter application semantics.
- [RFC 6455 implementation limits](https://www.rfc-editor.org/rfc/rfc6455.html#section-10.4)
  require protection against oversized frames and reassembled messages; B-04
  defines RRFlow message, queue, identity, cursor, and backpressure bounds.
- [PEP 561](https://peps.python.org/pep-0561/#packaging-type-information)
  requires an inline-typed package to include `py.typed` for downstream type
  checkers.
- The [Python packaging flow](https://packaging.python.org/en/latest/flow/)
  distinguishes source and wheel artifacts; RRFlow additionally requires a
  deterministic signed offline dependency closure and network-denied install.
- [W3C Trace Context](https://www.w3.org/TR/trace-context/) defines
  `traceparent` and `tracestate`; RRFlow owns its redaction, bounded causal
  attributes, and evidence semantics.
- [OWASP logging guidance](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html#data-to-exclude)
  excludes access tokens, authentication secrets, and session identifiers from
  logs; RRFlow applies the same prohibition to Python representation,
  serialization, exceptions, metrics, and traces.
