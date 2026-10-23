# RRFlow TypeScript SDK

**Status:** active implementation reference; alpha SDK and package qualification are incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/sdk/typescript`
**Owner:** TypeScript client generation, runtime contract enforcement, Node/browser transport, retry, cancellation, subscription, credential, packaging, and conformance behavior

`sdks/typescript` is the current TypeScript client foundation for the public
RRD protocol. It is an outward client of the one RRFlow engine, not another
query, context, reasoning, or lifecycle implementation. Authentication,
authorization, read stamps, transactions, rrflowMX/rrflowKV access, native
graph/BM25/vector indexes, rrflowQL/Arrow/DataFusion execution, reasoning-tree
mutation, installation, and attunement remain behind `RrdEngine`.

The [public contract](../protocol/public-contract.md) owns the operation and
wire vocabulary. The [Rust SDK reference](rust.md) owns no TypeScript behavior,
but its operation-semantic retry, outcome-certainty, cancellation, trace, and
cross-surface requirements apply equally here. The
[engine data flow](../../architecture/engine-data-flow.md) owns what occurs
behind the public operation, and the
[roadmap](../../roadmap/rrflow-1.0.md) alone owns acceptance.

## Required end-to-end boundary

```text
Node or browser application
  -> built TypeScript SDK + explicit endpoint/security profile
  -> HTTP or bounded multiplexed WebSocket RRD transport
  -> RrdEngine authentication, authorization, stamp, planning, and transaction
  -> rrflowMX or rrflowKV + native graph/BM25/vector + rrflowQL/DataFusion
  -> one validated result/denial/receipt with the same IDs, stamp, and trace
```

Compile-time types, generated files, mock fetches, and a successful HTTP call
do not prove the final line. The SDK cannot choose a storage profile, raw key,
physical index, DataFusion plan, model backend, application database, or
reasoning mutation. It expresses only public semantic intent and declared
budgets.

## Audited current package

The current package has one runtime dependency, ArkType 2.2.8, and a locked
Node/TypeScript generation and test toolchain. Its executable layout is:

```text
sdks/typescript/
├── package.json
├── pnpm-lock.yaml
├── pnpm-workspace.yaml
├── biome.json
├── tsconfig.json
├── scripts/generate.ts
├── src/
│   ├── index.ts
│   ├── client.ts
│   ├── endpoint.ts
│   ├── error.ts
│   ├── operation.ts
│   ├── retry.ts
│   ├── session.ts
│   ├── transport.ts
│   └── generated/
│       ├── endpoints.ts
│       └── rrd-openapi.ts
├── tests/client.test.ts
└── tests/sdk-conformance.ts
```

A-07.1c removed the 394-line implementation body from `src/index.ts`. The root
now exports the direct client, endpoint, error, operation, retry, session, and
transport responsibilities; it is not a forwarding copy of the old runtime.
The client still exposes four ergonomic calls—capabilities, endpoint
catalogue, OpenAPI, and session creation—and one generic
`call<K extends OperationId>()`. The generic call can address all 40 current
HTTP operation identifiers because `OperationId` is the key set of the
generated endpoint object. That is compile-time and routing coverage, not
behavioral coverage. There is no `subscription.ts`, WebSocket implementation,
or socket test to move: subscription open/close and changefeed follow remain
ordinary HTTP operations. B-04 has frozen the real multiplexed boundary and
proved its Rust reference carriage; TypeScript carriage remains explicit
H-04 cross-surface work.

## Generated authority and operation coverage

`scripts/generate.ts` invokes the repository-local `rrd-contract-export`
binary without a shell. It feeds the complete OpenAPI 3.1 document to
`openapi-typescript` and derives a runtime endpoint object from each
operation's identifier, method, path, first authentication scheme, and
mutation extension. `generate:check` compares both complete generated files
byte for byte.

`openapi-typescript` 7.13.0 prints the schema through the classic TypeScript
compiler API, which TypeScript 7 removed from its package entry point. The
package therefore pins TypeScript 6.0.3, the newest release that keeps that
API. Because the generator still declares a `typescript` `^5.x` peer,
`pnpm-workspace.yaml` allows exactly that pair, and `generate:check` proves the
output is unchanged. Moving to TypeScript 7 requires a generator that runs on
the TypeScript 7 API.

The audited projection contains:

- 42 HTTP operations;
- only `200` success envelopes plus the default error envelope;
- `application/json` as the only declared response media type; and
- OpenAPI SHA-256
  `28fdf404064be9b507ed2bc2780242ce15237eabe86cee5a7297af4470d4da3c`.

The generated `rrd-openapi.ts` is a compile-time projection of roughly 1.6
MiB. Its `paths`, `components`, and `operations` types are valuable compile-time
projections. TypeScript erases them at runtime. `RequestPayload<K>` and
`SuccessPayload<K>` therefore prevent many mistakes during type checking but
cannot validate an untrusted JavaScript caller, server response, stored value,
or generated object.

The target generator emits one closed runtime descriptor per operation with
method, route parameters, authentication, mutation semantics, security action,
request validator, response validator, exact success status, and media type.
Every descriptor must be bound exactly once and the coverage test must fail on
a missing, extra, or mismatched binding. Runtime validators are generated from
the same contract source; a handwritten second schema is forbidden. The
chosen validation representation must pass measured browser-bundle,
initialization-time, heap, adversarial-depth, and error-size budgets before it
becomes a default dependency.

## Request and response enforcement

The current client usefully validates canonical instance, correlation, path,
and resource identifiers; requires an idempotency key for a mutation; constructs
the common JSON request envelope; rejects repeated resource kinds; and caps a
streamed response before concatenation. Those behaviors remain.

The remaining boundary is substantial:

- request payloads receive no runtime validation;
- direct generic API-key calls do not reject an empty credential;
- unused or extra path parameters are not rejected;
- the runtime endpoint descriptor omits security action, request/response
  validator, success status, and media type;
- response `Content-Type` is ignored;
- every successful `2xx` is accepted when the envelope says `ok`, rather than
  requiring the operation's exact status;
- the ArkType envelope deliberately treats the success payload as `unknown`;
- the error code is any string and `details` is any object rather than the
  closed bounded public error contract;
- operation-specific response semantics, stamp, digest, receipt, and resource
  identities are not validated; and
- capability negotiation checks protocol/version/instance but does not bind
  the retrieved endpoint catalogue and OpenAPI bytes to their advertised
  digests.

A direct adversarial probe against the audited code first injected a transport
failure into an unpinned `query-execute`, then returned `201`, `text/plain`,
and `{ "invalid": true }` as the success payload. The client retried the query
and accepted that response. H-04 must turn each of those observations into a
negative test and reject it before exposing a typed result.

The response byte limit is real, but current accumulation retains each chunk,
allocates a combined byte array, decodes a string, parses JSON, and validates
the envelope. A configured N-byte limit is therefore not an N-byte peak-heap
bound. Resource qualification measures peak live bytes and rejects excessive
nesting, item counts, strings, decompression, declared-length mismatch, and
stream failures without classifying invalid representation as transient
transport loss.

## Bootstrap, Node, browser, and endpoint profiles

The current constructor accepts only credential-free `http:` URLs whose host
is `localhost`, `127.0.0.1`, or `[::1]`. That restriction is useful for the
present loopback profile. It also means the current client has no HTTPS,
mutual-TLS, installed-mesh, endpoint-rotation, or authenticated remote profile,
even when a caller injects a capable fetch implementation.

A qualified connection performs these steps explicitly:

1. resolve bounded endpoint candidates plus expected transport and RRD
   identities from installed configuration;
2. connect with the environment-specific transport security;
3. reject redirects rather than silently changing the selected endpoint;
4. call liveness and readiness;
5. negotiate exact protocol, instance, operation-catalogue digest, OpenAPI
   digest, limits, and available capabilities; and
6. authenticate an application session before protected work.

The client invokes its Fetch transport as a free function, never as a method
of the client, because a browser's `fetch` throws "Illegal invocation" when
called with any other `this`; a unit test injects such a transport. The
current fetch call does not set redirect mode, so Fetch's default `follow`
policy applies. The target uses `redirect: "error"` and verifies the final
response URL/transport identity. Network reachability, CORS success, browser
origin, TLS, and mesh membership never grant RRFlow authorization or initialize
an instance.

Node and browser transports share semantic operations but not credential or
TLS mechanics:

- a Node transport may consume an installed TLS/client-identity adapter and
  endpoint resolver;
- a browser relies on the user agent for TLS and is subject to CORS/preflight;
- a long-lived bootstrap API key is never shipped to untrusted browser code;
  browser session acquisition requires an explicitly installed, scoped,
  short-lived authorization or trusted mediator; and
- browser WebSocket authentication cannot depend on arbitrary HTTP headers or
  put bearer material in a URL. H-04 must add a typed, bounded browser
  connection-authentication exchange without weakening or forking the B-04
  application-frame protocol.

Connectome may consume the browser profile only after those constraints pass
real browser tests. It cannot add its own endpoint, session, retry,
subscription, or reasoning authority to compensate for missing SDK behavior.

## Retry, deadline, cancellation, and outcome certainty

The current attempt count is selected by:

```text
GET OR descriptor.mutation == false OR caller supplied idempotency key
```

That is not the required semantic policy. It replays every non-mutating POST,
including an unpinned query that may observe a later read stamp, and retries
every thrown value not already wrapped as `RrdApiError` or `RrdClientError`.
Consequently a local timeout, body-stream failure, invalid UTF-8 decoder error,
or arbitrary injected-transport exception can enter the same immediate retry
path. There is no backoff, jitter, attempt evidence, endpoint rotation policy,
or uncertain-observation result.

The target follows the operation-certainty matrix in the
[Rust SDK reference](rust.md#retry-deadline-and-outcome-certainty). In
particular, a read may replay only when the engine preserves the same accepted
read coordinate, and a mutation may replay only as identical bytes bound to an
engine-durable operation/idempotency receipt. Timeout and caller abort end
local waiting; they do not prove server cancellation or non-commit. B-04
defines a correlated server cancellation operation and terminal outcome;
H-04 implements that carriage here. Errors retain
the abort reason and safe cause without turning it into retry truth.

## Credentials, errors, and causal evidence

`Session` is currently a plain exported object containing the bearer token.
Its fields are public, mutable, enumerable, JSON-serializable, and inspectable.
The adversarial probe serialized the complete synthetic bearer. `apiKey` is
also a plain request-options object, and the final transport error incorporates
the message from an arbitrary thrown `Error`.

Direct convergence replaces that representation with an opaque session handle
whose secret is private, non-enumerable, non-serializable, and redacted from
inspection. Credential providers expose the minimum operation needed to build
an authorization header and do not create a second session store. Errors use a
closed phase-aware union for local contract, resolution, TLS, transport,
timeout/cancel, protocol, API denial, response validation, resource exhaustion,
and uncertain outcome. They retain safe causes and bounded server details but
never headers, credentials, uncontrolled response excerpts, or arbitrary
transport strings.

Every operation propagates canonical request and operation IDs plus valid W3C
`traceparent`/`tracestate`. Attempts are child spans of the same semantic
operation; reconnects and endpoint rotation are causal links. Browser and Node
telemetry apply the same redaction and bounded-attribute contract. Traces
observe authoritative engine state and never become completion or hidden
reasoning state.

## Multiplexed WebSocket delivery

The current TypeScript package has no WebSocket code or tests. B-04 supplies
the closed environment-neutral protocol and Rust reference implementation;
H-04 must add thin TypeScript Node/browser state and carriage adapters. They
must:

- route request/response, cancellation, subscription, delivery, ACK,
  heartbeat, backpressure, error, and terminal frames over one negotiated
  connection;
- enforce message, frame, receive, send, in-flight, queue, and decode limits
  before allocation;
- validate protocol, connection generation, request/subscription identity,
  sequence, cursor, and bounded error on every frame;
- resume only from a durably acknowledged cursor and reject stale generations;
- expose receive deadline and cancellation semantics without equating socket
  closure with server completion; and
- share the HTTP client's endpoint identity, session, errors, traces, and
  operation catalogue without creating browser- or Node-owned lifecycle state.

## Installable package boundary

The present `package.json` is deliberately private, exports raw `.ts` source,
has no build or package-consumer script, and runs `tsc --noEmit`. It emits
neither JavaScript nor declarations. An `npm pack --dry-run` audit produced a
70,658-byte archive with 1,529,059 unpacked bytes and included source,
generator, tests, configuration, and the 1.49-MiB generated schema; it included
no built JavaScript or `.d.ts` consumer entrypoint. This is not a qualified
Node 24 or browser distribution.

The release artifact is an ESM package built from the repository and included
in RRFlow's signed, offline-verifiable distribution or an explicitly selected
registry projection. It has:

- JavaScript and generated declaration entrypoints under a closed export map;
- an explicit included-file set containing no tests, generator, local config,
  secret, or repository-only path;
- a documented supported Node range and real minimum/current-version consumer
  tests;
- bundler and real-browser import, HTTP, cancellation, CORS, and WebSocket
  tests;
- deterministic pack contents, dependency closure, integrity/provenance,
  license and package metadata, size budgets, and install-with-network-denied
  proof; and
- no install-time contract generation, Cargo invocation, sibling checkout, or
  runtime fetch of RRFlow-owned assets.

ESM is the one pre-release module format unless the roadmap later accepts a
measured consumer requirement for another format. No CommonJS compatibility
lane or source-export fallback is added.

## Conformance that actually counts

The current evidence is bounded:

| Command or probe | Observed result | Honest boundary |
|---|---|---|
| A-07.1c public-surface inventory | All twelve pre-split root exports, the `RrdClient` constructor, and all five public methods remain. | Source/API-shape preservation only; not runtime operation conformance. |
| `pnpm --dir sdks/typescript check` | generation check, Biome over 14 files, typecheck, and 4 tests passed | Generator freshness and selected mocked HTTP behavior; no browser, WebSocket, package-consumer, or fault matrix. |
| Conformance entry with `RRD_SDK_CONFORMANCE_MANIFEST` absent | failed immediately with the required-manifest assertion | Correct fail-closed harness configuration; no scenario executed. |
| Conformance entry against the live example harness | passed and reported corpus SHA-256 `b3977c57c8d268f861e9d5158e5609bf3e7d2e1db01f914a12911b95c3404cb2` | Real HTTP against the present direct-seeded rrflowKV fixture; not D-01 installation, rrflowMX parity, browser/HTTPS/WSS, or complete labelled behavior. |
| Generated-surface parity | 40 HTTP endpoint descriptors matched the OpenAPI digest | Method/path/auth/mutation projection only; not runtime validation or semantic execution. |
| Adversarial injected-fetch probe | retried one unpinned query, accepted `201 text/plain` and invalid payload, serialized synthetic bearer | Reproducible open H-04 security/correctness defects. |
| `npm pack --dry-run --json` | raw-source/test/generator package, no JS/declarations | Packaging inventory only; not installability or publication. |

The live script exercises only capability retry/version rejection, catalogue
count, one authentication denial, session create/renew/close, vector
ensure/search, transaction begin/preview/abort/commit, one query, changefeed
read/follow/local abort, backup create/list, and estate read. Its shared domain
labels overstate CRUD, backup/restore, vectors, live delivery, typed errors,
retry/failure gaps, cancellation, and version/package coverage exactly as
catalogued in the [Rust reference](rust.md#conformance-that-actually-counts).

Release conformance starts a D-01-installed instance through public operations,
runs structural scenario assertions against rrflowMX and rrflowKV where
applicable, adds rrflowKV crash/reopen evidence, and compares the exact engine
result, denial, stamp, digest, receipt, trace, and resource accounting across
HTTP, multiplexed WebSocket, Rust, TypeScript, every other supported SDK, CLI,
MCP, GraphQL, and Connectome.

## Direct-convergence source boundary

A-07.1c established the source seams for behavior that actually exists. It did
not create an empty subscription module or future validators/tests as false
success surfaces:

```text
sdks/typescript/src/
├── index.ts                 # narrow public exports
├── client.ts                # current construction/discovery/generic dispatch
├── endpoint.ts              # current credential-free loopback profile
├── error.ts                 # current string-bearing error classes
├── operation.ts             # current generated typing/request coordinates
├── retry.ts                 # current broad attempt/deadline calculation
├── session.ts               # current plain enumerable credential object
├── transport.ts             # current bounded Fetch/envelope handling
├── subscription.ts          # planned H-04; absent until behavior exists
└── generated/
    ├── endpoints.ts
    ├── rrd-openapi.ts
    └── validators.ts        # planned H-04

sdks/typescript/tests/
├── operation-coverage.test.ts    # planned H-04
├── protocol-validation.test.ts   # planned H-04/J-02
├── transport-faults.test.ts      # planned H-04/H-07/J-02
├── package-consumer.test.ts      # planned J-03/J-05
├── browser-conformance.test.ts   # planned H-04/H-07/J-03
└── sdk-conformance.ts
```

The execution order is:

1. **A-07.1c/A-07 (implemented):** split every current
   responsibility directly, rename the conformance file, preserve the public
   surface plus generator/mock/live characterization, and leave no old
   implementation or forwarding module.
2. **B-04 (contract implemented):** consume the closed multiplexed golden
   protocol; no TypeScript behavior is claimed by the Rust reference carriage.
3. **D-01:** replace direct fixture seeding with installed public bootstrap.
4. **H-04:** implement bounded browser/Node WebSocket carriage, then generate
   and enforce complete runtime contracts, semantic retry/certainty/
   cancellation, opaque credentials, W3C propagation, and the structural
   cross-surface corpus.
5. **H-07:** add installed endpoint resolution, HTTPS transport identity, and
   authenticated rotation without making mesh reachability authority.
6. **J-02/J-03/J-05:** pass adversarial resource/failure tests, deterministic
   offline packaging, clean consumer installation, browser/Node matrices, and
   release verification.

## Primary constraints

- [WHATWG Fetch Standard](https://fetch.spec.whatwg.org/) defines redirect and
  cross-origin fetch behavior. RRFlow explicitly rejects redirects and binds
  the selected endpoint identity rather than inheriting `follow` silently.
- [WHATWG DOM Standard](https://dom.spec.whatwg.org/#interface-AbortSignal)
  defines composed and timeout abort signals; an abort signal remains local
  cancellation unless the RRD protocol returns terminal server evidence.
- [Node package entry points](https://nodejs.org/api/packages.html#package-entry-points)
  define the export-map boundary used by the built ESM artifact.
- [TypeScript declaration publishing](https://www.typescriptlang.org/docs/handbook/declaration-files/publishing.html)
  requires generated declarations to be included and identified with the
  runtime package rather than exporting repository TypeScript as a substitute.
- [npm registry package controls](https://docs.npmjs.com/using-npm/registry.html#how-can-i-prevent-my-package-from-being-published-in-the-official-registry)
  confirm that `private: true` prevents publication; RRFlow must deliberately
  select a signed bundle or registry delivery path.
- [RFC 9110 idempotent methods](https://www.rfc-editor.org/rfc/rfc9110.html#name-idempotent-methods)
  is only the HTTP baseline. RRFlow retry additionally requires read-coordinate
  or durable mutation-receipt certainty.
