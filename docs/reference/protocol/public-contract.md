# RRD public contract

**Status:** active implementation reference; alpha protocol convergence remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/protocol/public-contract`
**Owner:** transport-neutral RRD wire vocabulary, common request coordinates, capability discovery, and generated protocol authority

`rrd-contract` is RRFlow's public transport-neutral contract package. It gives
embedded callers, HTTP, WebSocket, SDK, CLI, MCP, GraphQL, Connectome, and
future adapters one serialized vocabulary. It does not own authentication,
authorization, transaction decisions, persistence, index selection,
Arrow/DataFusion execution, reasoning-tree mutation, or client lifecycle.
Those operations remain composed through `RrdEngine`.

The [system overview](../../architecture/system-overview.md) owns component and
authority boundaries. The
[engine data-flow record](../../architecture/engine-data-flow.md) owns target
write, persistence, read, Arrow/DataFusion, reasoning, and context flow. The
[server reference](server.md) owns the current HTTP/WebSocket process boundary,
the [SDK reference](../sdk/README.md) owns language-client behavior, and the
[roadmap](../../roadmap/rrflow-1.0.md) owns implementation order and acceptance.
This record describes what the public contract can represent now; it does not
convert a type, route, schema, or passing serialization test into
engine-capability evidence.

## Executable authority and precedence

Public protocol truth is resolved in this order:

1. Rust wire types and their validation in `rrd-contract` define the serialized
   vocabulary and local bounds.
2. `endpoint_catalogue()` defines the HTTP operations and WebSocket upgrade
   descriptors that are public now.
3. `openapi_document()` is the current deterministic HTTP/OpenAPI projection.
4. H-04 must converge the fragmented generators behind one deterministic
   surface-compiler invocation that projects the executable sources into
   internal dispatch metadata, OpenAPI, Rust-client bindings, language SDK
   operations and models, reference coverage, and shared conformance cases.
5. Checked-in golden and conformance corpora freeze selected byte shapes and
   cross-client scenarios; they are samples, not another operation registry.
6. This reference explains those executable sources and records gaps. It
   cannot add an operation or mark a capability available.

A public Rust type is therefore not automatically an HTTP operation. Likewise,
an OpenAPI operation proves only that a transport shape is generated; the
owning engine gate must still prove its authorization, semantics, physical
access path, resource bounds, and persistence behavior.

No SDK, router, adapter, test harness, or documentation table may maintain a
second successful operation registry. Language-specific code owns transport,
resource, cancellation, packaging, and idiomatic API behavior only. Generated
output is a replaceable projection and never becomes semantic or lifecycle
authority. The
[generation-boundary design](../../roadmap/rrflow-1.0-execution/generated-surfaces.md)
defines the common maintenance flow.

## Protocol identity and version domains

The wire identity is exactly `rrd` and its current protocol version is `1`.
Request, response, service-capability, endpoint-catalogue, SDK-corpus, and
deployment-corpus validation rejects a different protocol identity or version.
This technical protocol version is separate from RRFlow's product release-train
version, which remains frozen at `1.0.0` by the
[version policy](../release/version-policy.md).

The deterministic OpenAPI projection is currently bound to SHA-256
`06cd649d80ec2de5fe152da49daa00533d2e5dbf56e0321fea4d86f7b432a7b3`.
Changing its bytes requires an explicit protocol/schema review and regeneration
of every checked-in SDK surface; `scripts/ci/check_generated_surfaces.py`
verifies the SDK projections, the signal reference, and this citation against
the live export. Neither the protocol number nor the product version may be
changed merely to describe implementation progress.

The same document projects the kernel-owned signal catalogue. It loads a
private generated Rust snapshot, exposes it as `x-rrd-signal-catalogue`, and
binds it with `x-rrd-signal-catalogue-sha256`. The
[generated signal reference](signal-catalogue.md) and five SDK constants are
derived from those fields. They expose stable diagnostic, trace, and metric
identity for tooling; they neither activate telemetry nor report runtime state.
`crates/kernel/rrd-core/src/telemetry.rs` remains the sole semantic owner. Its
test-verified golden projection drives the private generated contract source;
the focused parity test compares OpenAPI back to the live kernel value.

## Common coordinates and envelopes

| Contract | Current rule |
|---|---|
| `CanonicalId` | 1–128 bytes; lowercase ASCII letters, digits, and bounded `-`, `_`, or `.` placement; human labels remain separate. |
| `CorrelationId` | 1–128 ASCII letters, digits, `-`, `_`, `.`, or `:` for request, operation, transaction, session, subscription, and idempotency coordinates. |
| `ResourcePath` | One to sixteen explicit typed segments; a resource kind cannot repeat; nothing is inferred from cwd, connection state, or a display label. |
| `RequestContext` | Carries request ID, operation ID, optional idempotency key, and optional absolute deadline. Contract validation requires an idempotency key when the catalogued operation is a mutation. |
| `RequestEnvelope<T>` | Binds protocol identity, request context, explicit resource path, and typed payload. |
| `ResponseEnvelope<T>` | Returns the same request and operation coordinates with either a typed payload or a closed `ErrorCode`/`ErrorBody`. |
| Operation digest | Typed transaction mutations serialize into a stable ordered representation before SHA-256 calculation; the engine, not the envelope type, owns durable key-to-operation/result binding. |

The closed error vocabulary distinguishes invalid input, absence, conflict,
failed preconditions, authentication, permission, resource exhaustion,
deadlines, cancellation, unavailability, corruption, unsupported versions, and
internal failures. Error messages and detail values are bounded. Credentials,
secrets, arbitrary headers, and private storage types are not public contract
payloads.

## Deployment and capability discovery

The accepted
[deployment-profile contract](../deployment/modes.md) separates three
coordinates:

```text
deployment form:       embedded | single_node_server | clustered_server
storage profile:       rrflow_mx | rrflow_kv
endpoint presentation: in_process | loopback_http_websocket |
                       network_http_websocket
```

The former `DeploymentMode` scalar has been removed. Strict decoding rejects
its `deployment_mode` field and its mixed `rrflow_mx`, `embedded`,
`local_daemon`, `edge`, `remote`, and `distributed` values. The current
`DeploymentProfile` instead binds `contract_version`, `deployment_form`,
`storage_profile`, and `endpoint_presentation`. `ServiceCapabilities` also
projects the optional installed project/estate/instance identity and complete
effective `EstateConfiguration` revision/digest.

The sole accepted estate-configuration input is format `2`. It adds the closed
`clock.maximum_rollback_ms` policy to reasoning, recall, and query ceilings.
The engine assigns revision and the v2 domain-separated digest; the strict
default is zero and the compiled maximum is one hour. This field governs
relative rollback against an installed engine observation. It does not state
that the host clock is synchronized or trusted, and clients cannot use it as a
clock-correction grant.

The installed composition root supplies identity, storage, and configuration;
the server supplies endpoint presentation from the actual bound address.
Neither engine nor server infers a database kind from TLS, address, caller
location, or persistence alone. Each capability must still say
whether it is unavailable, experimental, or available and may publish bounded
limits plus an honest limitation. Product capabilities enumerate engine,
rrflowQL, GraphQL, HTTP, WebSocket, gRPC, MCP, CLI, SDK, and Connectome
dispositions. A planned or unavailable surface must not advertise an
entrypoint.

Capabilities are runtime discovery, not a release ledger. The roadmap and its
acceptance evidence remain the only authority for completion.

## Current operation exposure

`endpoint_catalogue()` currently contains 40 sorted HTTP operations and one
authenticated generic `/v1/ws` descriptor. The server router and
generated OpenAPI document are derived from that catalogue. The
[server reference](server.md#public-operation-families) groups the current
inspection, identity, transaction, query, context, recall, delivery, and
recovery operations without duplicating the route registry here.

The boundary is intentionally explicit:

| Surface | Current contract state | Still required |
|---|---|---|
| HTTP | 40 catalogued operations with method, path, authentication mode, mutation flag, security action, and request/response type names. | H-04 cross-surface semantic equivalence and Gate J release qualification. |
| WebSocket | One generic authenticated descriptor and one closed frame envelope for request/response, cancellation, subscription/delivery, ACK, heartbeat, unsubscribe, error, backpressure, and exact reconnect resume. The server intentionally rejects generic operation execution until H-04. | H-03 commit-impact delivery, H-04 shared operation dispatch and cancellation, generated-SDK carriage, and Gate J qualification. |
| Rust client | Uses `rrd-contract` as its only normal RRFlow dependency, implements 37 of 42 catalogued HTTP operations, and implements the bounded multiplexed WebSocket over loopback HTTP or explicit mutual TLS. | The [Rust SDK reference](../sdk/rust.md) records the five missing HTTP operations and the exact H-03/H-04/H-07/J convergence boundary. |
| TypeScript client | Generates compile-time types and a generic call over all 47 HTTP descriptors, but has no complete runtime payload validation, WebSocket, remote transport, or installable JavaScript/declaration artifact. | The [TypeScript SDK reference](../sdk/typescript.md) records the exact request/response, credential, retry/cancellation, Node/browser, packaging, and conformance boundary. |
| Python client | Generates an operation literal and generic synchronous call over all 47 HTTP descriptors, but has no complete runtime payload validation, async/WebSocket client, remote transport, opaque credential, or typed-package/interpreter qualification. | The [Python SDK reference](../sdk/python.md) records the exact request/response, credential, retry/cancellation, sync/async, packaging, and conformance boundary. |
| Go client | Generates constants and a generic context-aware synchronous call over all 47 HTTP descriptors, but has no concrete operation payload validation, WebSocket, remote transport, opaque credential, or module/toolchain qualification. | The [Go SDK reference](../sdk/go.md) records the exact request/response, credential, retry/cancellation, concurrency, transport, module, and conformance boundary. |
| Java client | Generates an enum and a generic synchronous call over all 47 HTTP descriptors, but has no concrete operation payload validation, async/WebSocket or remote transport, opaque credential, or reproducible offline Maven/JAR qualification. | The [Java SDK reference](../sdk/java.md) records the exact request/response, credential, retry/cancellation, concurrency, transport, artifact, JDK/toolchain, and conformance boundary. |
| .NET client | Generates an enum and a generic asynchronous call over all 47 HTTP descriptors, but has no concrete operation payload validation, WebSocket or remote transport, opaque credential, or reproducible signed NuGet/toolchain qualification. | The [.NET SDK reference](../sdk/dotnet.md) records the exact request/response, credential, retry/cancellation, concurrency, transport, artifact, trimming/AOT, SDK/runtime, and conformance boundary. |
| Generated signal discovery | OpenAPI, a coordinated reference, and TypeScript, Python, Go, Java, and .NET expose the exact kernel catalogue JSON, signal fingerprint, and OpenAPI fingerprint from one renderer. | H-05 still requires build identity, runtime levels, correlated emission, exporters, captures, parity, overhead, and release evidence. |
| Shared generated-SDK corpus | OpenAPI and one shared semantic corpus exist as generation/conformance inputs. | Every supported language must run the same real-daemon corpus; schema generation alone is not qualification. |
| GraphQL | B-05 supplies bounded, schema-derived query lowering into the same `Query`, `Parameters`, and `BoundQuery` path as rrflowQL. No public GraphQL operation, resolver, or second executor is established. | H-04 outward HTTP carriage and cross-surface authorization/semantic equivalence. |
| MCP, CLI, Connectome, and model adapters | May consume public contracts but cannot infer engine state or implement missing semantics. | H-04 through H-07 and the relevant D/G/I gates. |

The crate also exports attunement, router, reasoning-tree, knowledge-package,
function, inference, diagnostic, context, memory-estate, and other typed
contracts that are not all present in the current endpoint catalogue. Their
existence freezes a bounded representation only. Availability must be obtained
from executable capability and operation catalogues and proved by the owning
roadmap gate.

## Contract-to-engine proof map

The public contract is designed to expose one coherent engine, but it cannot
prove that engine by itself:

| Public family | Representation already present | Required behavioral proof |
|---|---|---|
| Deployment characterization | One strict two-document fixture is supplied to current engine, client, server-process, and edge tests. | The deployment-profile owner requires separate storage-semantic, durability, deployment-form, endpoint, cluster, and derived-artifact corpora; C/E/F/G/H/J must prove the complete rrflowMX/rrflowKV and cross-surface behavior rather than treating this seed fixture as conformance. |
| Multi-model transaction | Claims, schemas, records, relations, events, vectors, time-series samples, geo values, object references, retirement, preview, and commit receipts have typed forms. | Accepted C-03 evidence proves one storage-neutral semantic plan and transaction across canonical/temporal model state, both adjacency directions, index-source changes, runtime state, durable projections, function receipts, audit, outbox, cursor, and outcome on rrflowMX/rrflowKV. H-04/J must preserve those semantics across every public surface and remove the conflicting claim-only contract branch. |
| rrflowQL query | Query text, typed parameters, read coordinates, plan evidence, rows, and scan/memory/spill/time/output budgets have public forms. | Accepted C-04 evidence proves bounded authenticated direct reads at one `ReadStamp`. F-01 through F-05 still must prove bounded streaming Arrow/DataFusion execution and remove eager `Vec<QueryRow>` materialization. |
| Graph, BM25, vector, and retrieval | Typed graph mutations, scalar/BM25 catalogue kinds, named dense/sparse/multi-dense vectors, HNSW index configuration, explicit scalar/product/binary/TurboQuant lifecycle operations, filters, exact/approximate search, recursive retrieval, and RRF evidence can be represented. | E-01 through E-05 and F-03 must prove transactional native access paths, exact fallbacks, deterministic fusion, recall, update/delete, and reopen behavior. |
| Context and reasoning | Bounded context packets, reasoning trees, route requests/decisions, and B-03 model-manifest pre-load admission have provider-neutral contracts. | G/H must prove executable routing, persisted CAS tree execution, engine-selected fast or analytical paths, authorized same-stamp context, feedback, and replay. |
| Installation and knowledge | Provider-neutral installation/attunement jobs and content-addressed knowledge packages have contracts. The bounded D-01 slices prove deterministic plan/apply, one canonical project-local estate, sealed format-2 configuration, portable locator, installed close/open, capability discovery, real-process ceiling denial, and engine-owned unverified clock anchoring with rollback diagnosis/enforcement. | D-01 still owns trusted-time/platform qualification, native pathname/ACL qualification, remaining lifecycle operations, and release-contained install/start/verify assets; later D and KB-06 through KB-08 own complete attunement, project inventory, authorized import, readback, and warp resolution. |
| Delivery and clients | Changefeeds, durable subscriptions, endpoint discovery, OpenAPI, the B-04 closed multiplexed WebSocket protocol, its Rust client/server implementation, B-05 GraphQL-to-bound-query lowering, and SDK corpus types exist. | H-03/H-04 and J must prove commit-impact delivery, operation execution/cancellation, public GraphQL carriage, cross-surface equivalence, correlated traces, and a clean self-contained deployment. |

Passing a row's serialization test cannot satisfy the behavioral proof in the
last column. The proof must exercise `RrdEngine`, the selected rrflowMX or
rrflowKV profile, and every native/rrflowQL/DataFusion operator named by that
gate with exact stamps and measured resource evidence.

## Current inconsistencies and direct convergence

The complete-file review found public-contract drift that passing unit tests do
not currently reject:

- `public-contract-v1.json` advertises `POST /v1/backups/create` for its sample
  HTTP capability, while `endpoint_catalogue()`, `rrd-server`, and `RrdClient`
  use `POST /v1/backups`;
- runtime product-capability limitation strings still refer to retired `F5`,
  `G06`, and `G09` work labels, so discovery text can disagree with the
  canonical roadmap; and
- the transaction contract still accepts separate `claims`/`data` scopes and
  vector search still accepts the alternate field/metric address alongside the
  collection/vector address.

These are pre-release conflicts, not supported compatibility promises. They
are recorded in the [POA&M](../../poam/rrflow-1.0-alpha.md) for traceable direct
convergence through their owning H/J and specific remediation packages. A-07
remains the accepted discovery baseline; these conflicts remain separate
tracked convergence work and are deliberately not silently repaired by this
D-01 estate/configuration slice.

## Current evidence

| Evidence | What it establishes | What it does not establish |
|---|---|---|
| `cargo test -p rrd-contract --test deployment_configuration_contract --locked` | Strict project/estate/instance identity, valid deployment combinations, format-2 clock/AI workload ceilings, configuration digest binding, a valid zero-as-strict clock policy, and rejection of unknown/old shapes or invalid workload bounds. | Installation, runtime enforcement, clock synchronization/correction, reasoning execution, or profile equivalence. |
| `cargo test -p rrd-contract --test public_contract --locked` | Strict selected wire shapes, bounds, 42-operation catalogue, one generic WebSocket descriptor, OpenAPI digest, deployment/SDK corpus validation, and selected cross-language digest vectors. | Server dispatch, storage semantics, DataFusion streaming, native indexes, reasoning execution, or all-language SDK conformance. |
| `cargo test -p rrd-contract --test signal_catalogue_projection --locked` | Exact kernel-to-OpenAPI catalogue/fingerprint equality, 4/9/22 closure, and a discovery-only field boundary. | Signal emission, level activation, propagation, export, diagnostic parity, overhead, or engine behavior. |
| `cargo test -p rrd-contract --test websocket_contract --locked` | The B-04 golden frame contract, sender directions, sequence and connection integrity, exact correlations/resume coordinates, negotiated/hard resource limits, and malformed/unknown representation rejection. | H-04 operation execution, generated-language carriage, or release qualification. |
| `cargo test -p rrd-query --test graphql_equivalence --locked` | B-05 schema-derived GraphQL parsing, lowering, bound-query/digest equality with rrflowQL, historical-schema binding, typed variables, and fail-closed unsupported shapes. | A public GraphQL route, transport authentication, engine execution, storage/index behavior, or cross-surface qualification. |
| Normal-dependency inspection | `rrd-contract` retains no RRFlow workspace package in its normal dependency graph; `rrd-core` remains test-only for live-kernel parity. | That every adapter depends inward correctly or lowers every type through `RrdEngine`. |
| Generated-surface parity check | The private Rust bridge, coordinated reference, and five SDK identities reproduce exactly; OpenAPI retains 40 endpoint descriptors and exact kernel signal/fingerprint equality. | Real-process semantic equivalence, complete SDK operation coverage, or runtime telemetry. |
| Server and client suites | Separately documented real-process HTTP, mutual-TLS, WebSocket, and durable-subscription behavior. | The complete target engine or released deployment. |

## Implementation anchors and focused verification

- Shared wire types, validation, catalogue, and OpenAPI projection:
  `crates/transport/rrd-contract/src/lib.rs`
- Installed identity, deployment, and effective-configuration contract:
  `crates/transport/rrd-contract/src/deployment.rs`
- Signal semantic owner: `crates/kernel/rrd-core/src/telemetry.rs`
- Private generated contract projection:
  `crates/transport/rrd-contract/src/generated/signal_catalogue.rs`
- Generated signal reference: `docs/reference/protocol/signal-catalogue.md`
- Signal projection characterization:
  `crates/transport/rrd-contract/tests/signal_catalogue_projection.rs`
- Multiplexed WebSocket frame contract:
  `crates/transport/rrd-contract/src/websocket.rs`
- Frozen WebSocket protocol sample:
  `crates/transport/rrd-contract/fixtures/websocket-protocol-v1.json`
- Cross-surface capability schema:
  `crates/transport/rrd-contract/src/capability_surface.rs`
- Shared SDK corpus schema:
  `crates/transport/rrd-contract/src/sdk_conformance.rs`
- Frozen selected wire sample:
  `crates/transport/rrd-contract/fixtures/public-contract-v1.json`
- Frozen GraphQL/rrflowQL equivalence sample:
  `crates/transport/rrd-contract/fixtures/graphql-equivalence-v1.json`
- GraphQL lowering and equivalence proof:
  `crates/compute/rrd-query/src/graphql.rs` and
  `crates/compute/rrd-query/tests/graphql_equivalence.rs`
- Contract characterization:
  `crates/transport/rrd-contract/tests/public_contract.rs`
- Runtime capability composition:
  `crates/authority/rrd-engine/src/capabilities.rs`
- HTTP capability snapshot:
  `crates/transport/rrd-server/src/http/capabilities.rs`
- Supported Rust client: `crates/transport/rrd-client`

Focused verification starts with:

```text
cargo test -p rrd-contract --test public_contract --locked
cargo test -p rrd-contract --test signal_catalogue_projection --locked
cargo test -p rrd-contract --test websocket_contract --locked
cargo test -p rrd-contract --all-targets --locked
python3 scripts/ci/check_generated_surfaces.py --check
```

Full engine, client, cross-language, crash/reopen, resource, and deployment
suites are run only where their owning gate names them as acceptance evidence.

Default installation-policy fixture regeneration uses the typed owning contract:
`cargo run --locked -p rrd-contract --example refresh_installation_contract`.
After a sealed default configuration or bootstrap change, derive the shared
SDK corpus identities with
`cargo run --locked -p rrd-engine --example refresh_sdk_installation_identity`.
Its read-only installation planner uses the checked binary identity anchor;
`-- --check` verifies drift without changing the corpus.
This offline projection refresh performs no engine workload and produces no
native qualification evidence. Review the resulting policy and digest changes.


`diagnostics-storage-read` (`POST /v1/diagnostics/storage/read`) requires the
existing diagnostics permission and exact instance scope. It returns the
existing physical storage observation without graph, audit, catalogue or
history assembly. Counters are process-local observations, not a cross-model
consistent snapshot; the full diagnostic operation retains its existing
consistency requirements. This permits storage monitoring independent of
project-history size without creating another storage authority.


## Bounded storage inspection cursors

`POST /v1/diagnostics/storage/cursor` is an authenticated `DiagnosticsRead`
operation scoped to the exact instance. It retains an ephemeral native physical
snapshot for inspection; it is not a semantic query result, durable transaction
or resumable cross-restart cursor. No physical key or value is returned.

The request supplies a session-local `cursor_id` and an action. `open` declares
`batch_rows` (1?4,096), `batch_bytes` (1?4 MiB) and `lifetime_ms` (1?60,000).
Native installed view/pin/output ceilings also apply; the metadata registry
admits at most the smaller of 64 or the installed active-view ceiling. Open
retries with different parameters conflict; matching retries observe the
current retained page. Cursor IDs may be reused after release or expiry.

`observe` hashes the current retained page without advancing. `advance`
supplies `expected_ordinal`; replaying the immediately previous advance returns
the existing page. Older/future ordinals fail. `cancel_source` releases physical
source leases but retains the copied page; `release` drops both and is safe to
repeat. Session closure and monotonic lifetime expiry also drop owned data.
The installed server sweeps expiry once per second; embedded hosts call
`RrdEngine::reap_storage_cursors`, and cursor operations also sweep. No renewal
extends a captured cursor's deadline. Process restart discards every cursor.

The snapshot reports physical sequence, page ordinal, row count, actual batch
allocation, source state and `page_sha256`. The hash frames the domain
`rrflow-storage-inspection-page-v1`, big-endian u64 sequence, ordinal and row
count, then length-prefixed key/value bytes in native order. It proves observed
page identity only. Cursor state is not a provider's private reasoning trace,
and process counters remain observations rather than an authoritative read stamp.
