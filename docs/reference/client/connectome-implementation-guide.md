# Connectome real-engine implementation guide

**Status:** active supporting implementation guide; source-audited, not H-06 qualification
**Coordinate:** `rrflow://rrflow-instance/data/reference/client/connectome-implementation-guide`
**Owner:** [RRFlow Connectome client contract](connectome.md)

This guide turns the Connectome contract into an executable handoff for the
separate Connectome repository. It does not own RRFlow operations, product
status, or Connectome acceptance. The [public contract](../protocol/public-contract.md)
owns wire behavior, the [SDK records](../sdk/README.md) own reusable clients,
the [installed lifecycle](../deployment/installed-lifecycle.md) owns the
operator journey, and roadmap [H-06](../../roadmap/rrflow-1.0/gate-h.md) owns
qualification.

## The honest starting point

The reviewed Connectome checkout was clean at commit
[`38f68ce7adda9d03501f3591165f0e14996899ec`](https://github.com/EonsofStupid/rrflow-connectome/commit/38f68ce7adda9d03501f3591165f0e14996899ec)
on local branch `main`, one commit ahead of `origin/main`.

**Verdict:** Connectome is not yet connected to RRFlow as a persistent product.
It contains a useful native RRD client prototype, but no visible screen invokes
that authenticated client. The current runtime screen calls only public health
and capability endpoints and labels their success `RRD connected` and every
stage `verified`. Its browser smoke test intercepts those requests and returns
fixed data. That proves rendering and a three-request parser, not
authentication, mutation, rrflowDB persistence, restart, or recall.

The distinction is exact:

| Present at the audited commit | Not present at the audited commit |
|---|---|
| Native loopback URL validation, redirect denial, bounded HTTP responses, a handwritten API-key/session flow, correlation checks, diagnostic/context calls, and explicit close | A screen wired to that authenticated flow; an installed credential handoff; generated RRFlow client/models; transaction or query UI operations; WebSocket use; a real `rrflow` child in UI tests; post-restart readback; or H-06 conformance |
| Browser rendering tests | A real desktop artifact test |
| Fake TCP tests for the native client | The native client running against `RrdEngine` |
| Static attunement, automation, diagnostics, and control-plane models | Engine-issued job, capability, operation, trace, or status records |

No later implementation may summarize the left column as the right column.

## Product boundary

RRFlow and Connectome are two products with one public boundary:

```text
Connectome renderer
  non-secret view intent and validated view models
        |
        v
Connectome native Rust process
  endpoint/TLS, installed credential reference, session, retry,
  cancellation, frame/body limits, generated RRFlow client
        |
        v
RRD HTTP + multiplexed WebSocket
  executable operation catalogue + OpenAPI identity
        |
        v
installed rrflow process -> RrdEngine -> rrflowDB/rrflowKV
                                      -> rrflowQL
                                      -> graph/BM25/vector/RRF
                                      -> Arrow/DataFusion when selected
```

The renderer does not open rrflowKV, execute DataFusion, choose an index, hold
an API key or bearer token, infer a commit, or maintain another copy of engine
state. Arrow/DataFusion are RRFlow execution internals; a UI may receive an
explicit public result representation but does not become the analytical
engine.

Fjall is not in this topology. It is a comparative benchmark opponent used to
challenge RRFlow's own storage implementation.

## Run the current installed engine

These commands exercise the current source-built pre-release. They are a
development workflow, not signed-distribution evidence.

```bash
export RRFLOW_BIN=/absolute/path/to/rrflow
export RRFLOW_PROJECT=/absolute/path/to/project
export RRFLOW_PLAN=/absolute/path/to/rrflow-install-plan.json

"$RRFLOW_BIN" --json version
"$RRFLOW_BIN" --json install plan \
  --project "$RRFLOW_PROJECT" \
  --mode existing \
  --profile default > "$RRFLOW_PLAN"

export RRFLOW_PLAN_SHA256
RRFLOW_PLAN_SHA256="$(jq -r '.installation.plan_sha256' "$RRFLOW_PLAN")"

"$RRFLOW_BIN" --json install apply \
  --project "$RRFLOW_PROJECT" \
  --mode existing \
  --plan "$RRFLOW_PLAN" \
  --expect "$RRFLOW_PLAN_SHA256"

"$RRFLOW_BIN" --json verify \
  --project "$RRFLOW_PROJECT" \
  --level quick
```

Start the installed service in a foreground terminal:

```bash
"$RRFLOW_BIN" --json serve \
  --project "$RRFLOW_PROJECT" \
  --bind 127.0.0.1:9477
```

Probe it from another terminal:

```bash
"$RRFLOW_BIN" --json ready \
  --project "$RRFLOW_PROJECT" \
  --address 127.0.0.1:9477
```

`serve` announces the exact URL and installed instance. `ready` performs a
real authenticated probe and returns readiness, capabilities, the endpoint
catalogue, OpenAPI, and its digest, then closes its probe session. It does not
hand Connectome a reusable session.

For a temporary project that must not preserve data, use an explicit temporary
directory and `--mode fresh`. Never point automated destructive tests at a
human project.

## Discovery and connection rules

Connectome must implement this sequence, with a distinct UI state for each
boundary:

```text
candidate selected
  -> live
  -> ready
  -> capabilities
  -> endpoint catalogue + OpenAPI identity
  -> exact protocol/instance/deployment/configuration validation
  -> native credential acquisition
  -> authenticated session
  -> authorized operation
  -> validated result/receipt/stamp
```

Use these words precisely:

| UI state | Minimum evidence |
|---|---|
| Reachable | Liveness response from the candidate only |
| Ready | Readiness response with expected installed identity; still not authenticated |
| Discovered | Capabilities, endpoint catalogue, OpenAPI identity, deployment, and limits validate together |
| Authenticated | Native client created a valid RRD session for the expected instance/principal |
| Connected | Discovered and authenticated; renderer has only an opaque connection handle |
| Persisted | A commit receipt was reconciled and a later query at an adequate read coordinate returns the value |
| Recovered | A different RRFlow process reopened the same installed estate and a fresh authenticated query returned the same value |

An HTTP 200, green dot, animation, process ID, open port, generated interface,
mock response, or screenshot cannot advance these states.

## Generated contract handoff

Connectome must not maintain the operation names, paths, request models, result
models, limits, or deployment vocabulary by hand.

The handoff order is:

1. Fetch `/v1/capabilities`, `/v1/schema/endpoints`, and
   `/v1/schema/openapi` from the selected process.
2. Validate their protocol, instance, catalogue, configuration, and OpenAPI
   identities against the client artifact built into Connectome.
3. Use a pinned, packaged RRFlow client generated from the executable
   catalogue. Do not use a sibling path dependency or generate from a running
   server during installation.
4. Reject an unknown or mismatched operation/model/digest. Do not fall back to
   the handwritten decoder.
5. Display the live digest for diagnosis; never hard-code today's digest as a
   permanent product constant.

The current RRFlow Rust client is the strongest native foundation, but its
external package/distribution qualification and complete operation coverage
remain H-04/J work. The current TypeScript source is useful as a generated
type projection; it is not yet a qualified browser package or a reason to move
credentials into the renderer.

## Secure installed credential handoff is a blocker

The audited native command accepts `api_key` from a renderer-serializable
`RrdConnectRequest`. That is not the target product boundary. The RRFlow test
can read its internal installed credential because it is an engine test; an
external UI must not copy that internal reader or parse private installation
files.

The accepted target is a packaged RRFlow installed-client boundary consumed
only by Connectome's native process:

```text
InstalledConnectionRequest
  project selected explicitly by the operator
  expected instance identity, if already known

InstalledConnectionSummary returned to the renderer
  opaque connection handle
  endpoint and exact instance resource
  non-secret principal/role summary
  deployment/configuration/catalogue/OpenAPI identities
  session expiry class, not bearer or session identity
```

The native side resolves the installed locator and credential reference through
that public library/broker, checks platform permissions, creates the session,
and keeps every credential and token private. Until RRFlow ships that boundary,
Connectome may develop public discovery and non-secret presentation, but it
must label authenticated turnkey connection blocked. Do not normalize any of
these shortcuts:

- API key in a React field, Zustand/config store, URL, route, local storage,
  screenshot, log, error, or Tauri command result;
- direct parsing of `.rrflow` credential files in Connectome;
- engine-crate or sibling-checkout dependency;
- fixed development credential;
- anonymous access presented as a local convenience; or
- `rrflow ready` output presented as a reusable UI session.

## Audited Connectome source map

All paths below are relative to the separate Connectome repository at the
audited commit.

| Path/symbol | Evidence | Required disposition |
|---|---|---|
| `src-tauri/src/rrd/client.rs::NativeRrdClient` | Real HTTP code with useful loopback, redirect, size, envelope-correlation, session, and close defenses; all models/paths/limits are handwritten `serde_json::Value`; tests use a fake TCP server. `RrdConnectionSummary` exposes `session_id` to the renderer. | Replace wire/model/operation code with the packaged generated Rust client. Keep native ownership and measured bounds. Return an opaque handle and non-secret summary only. Add real installed-process tests; retain fake servers only for malicious-frame unit cases. |
| `src-tauri/src/rrd.rs` | Registers native commands and stores one client, but returns generic JSON and accepts renderer-supplied API-key material. | Make this a narrow typed composition layer over the generated client and secure installed handoff. No credential-bearing command argument/result and no generic operation tunnel. |
| `src/rrflow/runtime.ts` | Duplicates protocol models and accepts retired `deployment_mode` values: `memory`, `embedded`, `local_daemon`, `edge`, `remote`, `distributed`. | Replace with generated non-secret view types. Validate structured deployment form, storage profile, endpoint presentation, installed identity, configuration, catalogue, and OpenAPI identity. Remove the old decoder directly. |
| `RuntimeConnectionPage` | Invokes only `RrdRuntimeClient.handshake()`, then renders `RRD connected` and every stage `verified`. It never uses `RrdAuthenticatedClient`. | Render reachable/ready/discovered/authenticated separately. Wire only the native authenticated service. Remove static success and any capability/action not returned by the engine. |
| `tests/smoke/connectome.spec.ts` | `page.route()` intercepts all `/v1/**` calls and returns fixed `0.1.0`, `local_daemon`, `rrd-lsm`, and one capability. | Keep as an explicitly named renderer-fixture test only. It cannot be called engine, connection, persistence, or conformance evidence. Add a separate real-artifact suite with no route interception. |
| `package.json`, `src/vendor/rrflow-client.ts` | `@rrflow/client` aliases `surrealdb`; `Surreal` is exported as `RRFlow`. CBOR, UI, Wasm, and two SurrealQL Wasm packages remain. | Map useful presentation behavior to typed RRFlow results, then remove the renamed runtime/client/query authority and lockfile closure. Do not preserve a compatibility lane. |
| `src-tauri/src/database/mod.rs` and `shell.rs` | Launches `rrflow start --user --pass ... --allow-all` with `rocksdb://`, `rrflowkv://`, or `tikv://`; joins shell command strings and can kill all `rrflow.exe` processes. This is not the current RRFlow CLI. | Delete after the installed lifecycle/launcher replacement exists. Never translate it into a second supervisor. The current product commands are `install plan/apply`, `serve`, `ready`, and `verify`. |
| `src/providers/Context/index.tsx` | Hard-coded cloud WebSocket, namespace `rrflow`, database `cloud`, vendor client, fixed reconnect loop. | Remove. Live context must use the one native RRD connection and multiplexed public WebSocket contract with cursor/ACK/resume evidence. |
| `src/rrflow/attunement.ts` | Client-authored phases, 30–45 minute estimate, statuses, and automation primitive definitions. | Remove as lifecycle truth. Render only engine-issued capability and durable job records. Client copy may survive only as non-authoritative fixture data under tests. |
| `src/rrflow/diagnostics.ts` and `RuntimeDiagnosticsPage` | Separate retired diagnostics protocol, prompt-flight runner, replay ledger, provider state, and private `/api/*` operations. | Remove. Project accepted diagnostics/traces/reasoning evidence from public RRD operations. |
| `src/rrflow/control-plane.ts` and `ControlPlanePage` | Client-invented `rrflow-control` protocol and managed-instance state. | Remove until a separately accepted fleet contract exists. Multi-instance navigation uses independent RRD connections and never invents fleet authority. |
| `types.tsx::Connection` and inherited authentication fields | Runtime profiles reuse namespace/database/user/password/token fields from the old database client and serialize them in the renderer configuration model. | Introduce a minimal non-secret RRD profile: label, endpoint candidate, expected instance, project reference/opaque native handle, and presentation preferences. No secret, namespace, or database authority. |
| `README.md` and `connectome-ci.yml` | README overstates strict identity/real connection; CI builds, renders against mocks, and tests the native client only against a fake server. | State current behavior exactly. Add the installed RRFlow artifact/restart suite and make absence of its artifact/configuration fail or explicitly skip without reporting conformance. |

## Implementation order in the separate repository

Each package is independently reviewable and deletes its conflicting path only
after replacement evidence exists.

### C0 — truth before features

- Change the current runtime screen from `connected/verified` to
  `reachable/ready/discovered` according to the table above.
- Remove static attunement/automation cards from the live capability view.
- Rename the intercepted Playwright case to make `mock` or `renderer fixture`
  explicit and prevent it from contributing to conformance status.
- Add a source gate for the retired paths/patterns in the audit table.
- Correct README/CI claims.

Acceptance: no screen says authenticated, connected, persisted, recovered, or
verified from public GETs or a fixture.

### C1 — one generated native client

- Consume the pinned packaged RRFlow Rust client and generated models.
- Replace `serde_json::Value`, copied endpoints, copied limits, and local
  envelope construction.
- Validate liveness, readiness, capabilities, endpoint catalogue, OpenAPI,
  exact instance, deployment, configuration, session, response identity,
  receipt, stamp, and completeness.
- Return only bounded non-secret view models to the renderer.

Acceptance: catalogue coverage and generated artifact identity match RRFlow;
removing or adding one operation fails generation/conformance rather than
silently degrading.

### C2 — installed native handoff

- Consume the qualified installed-client credential boundary described above.
- Store only an opaque native connection handle in renderer state.
- Make project selection explicit and verify expected instance identity.
- Close/revoke the native session on disconnect, profile removal, and app exit.

Acceptance: renderer storage, IPC results, logs, errors, traces, screenshots,
and crash output contain no API key, bearer, session identifier, or credential
file path.

### C3 — smallest real persistence lens

Build one intentionally small view before graph polish:

1. create a session;
2. begin a transaction;
3. commit a uniquely generated claim through the catalogued mutation;
4. display its commit receipt and engine-issued identity;
5. query it with bounded rrflowQL;
6. display `known_at_cursor`, read-manifest, plan, truncation, and row identity;
7. stop RRFlow abruptly;
8. show disconnected without changing the prior receipt into failure or
   success inference;
9. restart the same installed project;
10. perform discovery and fresh authentication; and
11. execute the same query and display the same value at or beyond the commit
    sequence.

Use a per-run random value so a fixture or hard-coded UI cannot pass. This is
the first product demo that proves the UI is looking at RRFlow persistence.

### C4 — absorb useful lenses and remove the renamed database

Port graph, table, timeline, inspector, query-editor, and accessibility behavior
one lens at a time onto typed RRD results. For each lens, record its old source,
new typed input, resource/stamp identity, bounded render test, and removal.
Then remove its SurrealDB client/query/schema dependencies and old connection
route. A compile after deleting behavior is not absorption evidence.

### C5 — live delivery and full H-06 corpus

Add the public multiplexed WebSocket, exact subscription identity, ordered
cursor application, ACK, backpressure, cancellation, resume, gap handling,
catalogue/authorization change, restart, and bounded rendering. Then run the
full H-06 scenarios against rrflowMX and rrflowKV. Only that later artifact can
qualify Connectome.

## Required real-persistence test

The Connectome repository must own a test that launches an explicit RRFlow
candidate artifact. It may not build or import RRFlow from a sibling checkout
inside the test.

Required inputs:

```text
RRFlow binary path and SHA-256
RRFlow revision/build identity
Connectome revision/artifact identity
temporary fresh project path
install-plan and configuration digests
expected instance identity
per-run random claim value
```

Required journey:

```text
version -> install plan -> exact apply -> serve -> ready
  -> native Connectome discovery/authentication
  -> transaction commit -> UI readback with receipt/stamp
  -> abrupt kill -> disconnected state
  -> same binary + same project serve -> ready
  -> fresh native authentication -> same query -> same random value
  -> close -> kill/stop -> rrflow verify
```

The evidence record includes the exact binary/revision/digests, instance,
commit sequence, post-restart read coordinate, read-manifest identity, result
identity/value, process exit, raw failures, and commands. A screenshot may be
attached for review but is not the persistence proof.

## Anti-mock and anti-hard-code review

Run these in the Connectome repository on every convergence package:

```bash
git status --short --branch
git rev-parse HEAD^{commit}

git grep -n -E \
  '@rrflow/client|npm:surrealdb|@surrealdb|Surreal as RRFlow|@rrflow/wasm|rrflowkv://|--allow-all|rrflow-control|vy[r]m-diagnostics|rrflow-cloud-|local_daemon|implementation_version.*0[.]1[.]0'

git grep -n -E \
  'route[.]fulfill|page[.]route|fake RRD|mock.*RRD|RRD connected|verified' \
  -- src src-tauri tests

rg -n 'new RrdAuthenticatedClient|rrd_connect' src/screens src/rrflow
rg -n 'api_key|Authorization|session_id|token' src src-tauri/src
rg -n 'path\s*=\s*"\.\./|\.\./vy[r]m|\.\./rrflow' src-tauri/Cargo.toml package.json
pnpm why surrealdb
cargo tree --manifest-path src-tauri/Cargo.toml --locked
```

These commands are review inputs, not a magical zero-match policy. Mocks and
malicious fake servers remain valuable in explicitly named unit tests. They
must be absent from the real-process conformance lane, and production code must
have no old runtime authority. Every retained match needs an owner, reason,
replacement package, and test.

The reviewer should be able to follow one real action without guessing:

```text
button/view intent
  -> generated renderer request type
  -> named Tauri command
  -> generated native SDK method
  -> endpoint catalogue operation
  -> RrdEngine receipt/result
  -> validated non-secret view model
```

If the chain jumps to a static array, vendor alias, generic JSON route, local
database, renderer fetch with a credential, or unexplained success state, the
feature is not converged.

## What the current RRFlow proof means

RRFlow's primary installed lifecycle now has a focused engine-side oracle for:

```text
install -> serve -> authenticate -> commit -> query
        -> abrupt kill -> serve again -> authenticate again
        -> query the same value -> offline verify
```

That establishes a real persistence target for Connectome. It does not prove
that Connectome consumes it. Connectome earns that claim only when its own
clean artifact completes the required journey above with no sibling source,
mock transport, direct storage read, hard-coded result, or renderer credential.

## Open product blockers

The following remain open and must stay visible in the UI/release plan:

- packaged, qualified generated Rust/TypeScript client inputs;
- secure installed credential/connection handoff;
- complete catalogue operation coverage and response validation;
- multiplexed WebSocket operation execution and resume/ACK behavior;
- rrflowMX/rrflowKV differential UI corpus;
- graph/BM25/vector/RRF/Arrow/DataFusion/context/reasoning evidence produced by
  completed engine gates;
- remote TLS/mTLS and endpoint rotation;
- signed offline RRFlow and Connectome artifacts on supported platforms;
- repair/restore/uninstall presentation; and
- complete H-06 security, accessibility, resource, failure, and multi-instance
  conformance.

None prevents honest source review or the C0 work. The secure installed handoff
and packaged client block calling the current prototype a layman-turnkey
authenticated product; they must be implemented rather than hidden behind a
demo credential or a green badge.
