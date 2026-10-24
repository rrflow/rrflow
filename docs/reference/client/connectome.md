# RRFlow Connectome client contract

**Status:** active target client contract; canonical RRFlow discovery handoff exists, separate checkout remains non-conforming
**Coordinate:** `rrflow://rrflow-instance/data/reference/client/connectome`
**Owner:** Connectome connection, projection, interaction, local-state, and conformance requirements

The repository root [README](../../../README.md) owns RRFlow identity and current
status. The [system overview](../../architecture/system-overview.md) owns the
engine/client boundary, the
[instance topology](../../architecture/instance-topology.md) owns project,
estate, instance, and deployment identity, the
[engine data flow](../../architecture/engine-data-flow.md) owns query, context,
reasoning, automation, and presentation flow, the
[public contract](../protocol/public-contract.md) owns wire vocabulary and
operation discovery, and the [SDK reference](../sdk/README.md) owns reusable
language clients. Roadmap [H-06](../../roadmap/rrflow-1.0.md#gate-h--prove-context-flow-feedback-live-delivery-and-connectome)
owns implementation and acceptance. This record cannot add an engine operation
or declare Connectome qualified.

The [real-engine implementation guide](connectome-implementation-guide.md)
maps the audited separate checkout to concrete replacement packages and a
write/kill/restart/readback acceptance journey. It is supporting guidance, not
another contract or H-06 qualification record.

## Product and authority boundary

Connectome is RRFlow's optional operator and developer workbench. It is built,
versioned, tested, and released from its own repository. RRFlow can reach local,
remote, or explicitly mesh-resolved instances without Connectome, and the
default RRFlow distribution does not require the Connectome checkout or
artifact.

### Repository checkout

The RRFlow repository registers Connectome at `connectome/` as an optional Git
submodule of `https://github.com/rrflow/connectome-development.git`. The parent
Git tree pins the exact UI revision; `.gitmodules` owns the source URL. UI
changes are developed and reviewed in that separate repository, then a bounded
RRFlow change advances the pin with its integration evidence. A submodule pin
does not qualify the UI or create engine authority.

For UI development, explicitly acquire the pinned checkout:

```bash
git submodule update --init -- connectome
git submodule status -- connectome
```

The Connectome development repository is public and supports anonymous
acquisition. Engine-only local clones can omit initialization. Candidate CI
recursively checks out the pinned UI source under the repository source policy.
The [publication receipt](../../evidence/change-journals/gate-h/h-06-connectome-public-association.md)
records the authorized visibility change and anonymous-access verification. The
default engine build and installed distribution cannot read source or assets
from the UI checkout. A UI distribution must package its required assets;
installed startup cannot fetch this repository.

The parent inventory records the gitlink revision without traversing child
files. Local UI edits remain owned by the child repository. Never replace its
pin with an escaping sibling path or treat a dirty checkout as reproducible
integration evidence. Use `git submodule update --init -- connectome` to restore
the recorded revision, rather than updating to an unreviewed branch tip.

The [delivery architecture](../../architecture/system-overview.md#delivery-experiences)
relates this UI to shippin.cloud private devspaces and the DevPulse local
offering. Hosted project execution, previews, web launch, account creation,
and synchronization are product targets requiring their own public-contract
integration and proof; this source registration supplies none of those effects.

### Native local workbench

[Connectome PR12](https://github.com/rrflow/connectome-development/pull/12)
implements visible local startup, a DB/QL/KV guide, and a native model browser
on the existing authenticated Rust SDK session. The packaged engine/SDK input
is RRFlow `4860a253663c9c757800aafeb40890b76114c4f0`; the parent gitlink records
the reviewed UI revision independently. The
[workbench guide](https://github.com/rrflow/connectome-development/blob/main/docs/native-workbench.md)
owns build and desktop-use instructions.

[Connectome PR13](https://github.com/rrflow/connectome-development/pull/13)
adds selected-project workspaces with nested Overview, Knowledge, Activity,
Learning and Services sections. Basic descriptions lead to advanced engine
identity and evidence; native model, record/relationship, vector and storage
views remain bounded public-operation projections. Manage project engines owns
at most four local processes, and authenticated native connections are keyed by
project profile (at most eight), so opening or stopping one does not replace
another project's session. Context sharing is not implied by parallel execution.

The [project workspaces pin receipt](../../evidence/change-journals/gate-h/h-06-project-workspaces-pin.md)
binds the merged UI to its Windows/Linux/macOS, frontend and real-engine process
CI. Knowledge-path activation, replay, supervised/unsupervised learning and
cloud sync remain visibly unavailable. This UI revision retains the same older
engine/SDK pair named above; neither source association nor parallel process
control is proof of those downstream capabilities or full H-06 acceptance.

Newer RRFlow installation-plan contract v2 requires an explicit identity from
`rrflow install identity`, retained across preview regeneration and supplied
with `install plan --identity`. The pinned workbench above uses its named older
engine/SDK pair; it is not qualification of that new installation contract.
Updating its engine bundle requires updating and testing this adapter input as
described in the [implementation guide](connectome-implementation-guide.md#run-the-current-installed-engine).

The desktop shows the native installation preview, requires its exact accepted
digest, and regenerates the plan before apply. Existing estates reopen through
the same installed lifecycle. Data models display the authenticated catalogue,
bounded native query results and read cursors; graph, vector and storage lenses
remain public-operation projections. The local workflow requires no account.
The renderer holds a credential path, never the long-lived credential itself.

Real-process tests cover installation rejection/acceptance/reopen, independent
sessions, the digest-verified bundled engine, catalogue reads, key/value,
event, time-series and geo queries, and graph/vector/storage views. The Windows
development desktop was also driven through actual install, query, close and
reopen; native quick verification passed. The
[package receipt](../../evidence/change-journals/gate-h/h-06-native-turnkey-workbench.md)
links candidate, build and bounded acceptance evidence. Browser rendering
tests remain distinct from this actual native evidence.

Current limits: row queries for records, relations and events require a
registered typed schema; schemaless table metadata alone does not register a
query source. The UI explains unavailable browsing. Time-series queries select
the series record kind, not the sample table kind. Object metadata without a
public row source remains schema-only. Media understanding/generation requires
separately configured models and authorization. PostgreSQL compatibility,
DevPulse packaging, cloud accounts/synchronization and hosted devspaces are
still open. The unsigned Windows development bundle does not satisfy signed
release, complete H-06, or clean-machine installer acceptance.

### Projection authority

Connectome is a projection client. It may:

- discover and invoke available public RRD operations;
- render typed canonical state, plans, evidence, traces, deltas, and proposals;
- retain non-authoritative local presentation preferences; and
- ask the operator to preview and confirm an authorized operation.

It cannot own or recreate:

- installation, estate, project, instance, session, policy, or attunement
  authority;
- rrflowDB or rrflowKV state, rrflowMX semantics, read stamps, transactions, or
  commit decisions;
- rrflowQL parsing/planning, graph traversal, BM25, vector/HNSW/TurboQuant,
  reciprocal-rank fusion, Arrow batching, or DataFusion execution;
- reasoning-tree state, context selection, feedback policy, engine events,
  triggers, routines, skills, leases, checkpoints, or outcome verification; or
- an alternate diagnostics, control-plane, provider-flight, storage, query,
  subscription, or presentation protocol.

A client build, mock, screenshot, open port, health response, or rendered label
is never engine-capability evidence. The authoritative result is the validated
public response, persisted state, read stamp, commit receipt, or correlated
evidence supplied by the one `RrdEngine`.

## Connection and session sequence

Every new connection and endpoint rotation follows one fail-closed sequence:

```text
explicit endpoint or bounded candidate resolver
  -> validate URL, presentation, and expected transport identity
  -> GET /v1/health/live without a credential
  -> GET /v1/health/ready without a credential
  -> GET /v1/capabilities without a credential
  -> validate protocol/version, exact instance resource, deployment profile,
     operation catalogue and digest, security revision, and declared limits
  -> acquire the credential inside the applicable native/browser trust boundary
  -> create one authenticated RRD session through the public operation
  -> negotiate only advertised operations and subscriptions
  -> consume results and cursor-ordered deltas
```

Plain HTTP is loopback-only. A non-loopback candidate requires HTTPS and the
configured server identity; a mutual-TLS or mesh profile additionally validates
its own independent transport identity. A mesh says how to reach a candidate,
not what RRFlow instance it is and never who may use it. Rotation repeats every
probe, identity, capability, and authentication step before the replacement is
accepted.

Readiness is not authentication. Capability discovery is not authorization.
Connectome never creates, repairs, initializes, or selects storage while
connecting. A timeout or disconnect after a mutation was sent is an uncertain
outcome that must be reconciled by its idempotency and operation coordinates;
the client cannot replay it as a new request or silently fail over.

The desktop native process owns transport, TLS material, long-lived credential
references, session tokens, body/frame limits, retry classification,
cancellation, and connection disposal. The renderer receives only a
non-secret connection handle plus validated public results. A browser build
requires its own explicitly supported browser-authentication profile with
short-lived, origin-constrained credentials; it cannot expose or reuse the
desktop API key. Secrets never enter URLs, routes, local storage, serializable
view state, logs, traces, errors, screenshots, or diagnostic export.

## Deployment and capability truth

Connectome consumes the structured deployment descriptor:

```text
deployment form:       embedded | single_node_server | clustered_server
storage profile:       rrflow_mx | rrflow_kv
endpoint presentation: in_process | loopback_http_websocket |
                       network_http_websocket
```

It does not infer those coordinates from a path, hostname, TLS state, port,
process, or old scalar `deployment_mode`. The UI may expose durability actions
only when the storage profile advertises them. It must never describe rrflowMX
as rrflowDB's hot tier, imply that rrflowMX will flush into rrflowKV, or label an
unavailable clustered profile as connected.

Navigation and controls are derived from the validated operation and product
capability catalogue. An unavailable capability is absent or explicitly marked
unavailable with its stated limitation. A planned type, a client implementation,
or a familiar inherited screen cannot make it available.

The current generated handoff is:

```text
ServiceCapabilities
  installed_estate? { project_id, estate_id, instance_id }
  deployment {
    contract_version, deployment_form, storage_profile,
    endpoint_presentation
  }
  configuration {
    format_version, revision, configuration_sha256,
    reasoning { max_run_elapsed_ms, max_steps, max_step_elapsed_ms },
    recall { max_graph_depth, max_items, max_output_bytes, max_storage_keys },
    query { max_storage_keys, max_rows, max_output_bytes, max_batch_rows,
            max_memory_bytes, max_spill_bytes, max_elapsed_ms }
  }
  capabilities[]
  product_capabilities
```

`installed_estate` is required for the D-01-installed product even though the
field remains optional for embedded/test compositions. Connectome must reject
an identity mismatch between it and `instance`. It renders the effective
configuration revision and ceilings but never edits `.rrflow/config.toml` or
assumes that a ceiling activates a feature. In particular,
`governed-reasoning-runner` is currently unavailable; its configured limits
support future plan/apply and UI preparation, not a success badge. Context and
query controls must default at or below the advertised estate ceilings rather
than blindly sending the protocol hard maximum.

The [user-selected effort contract](../inference/lfg-swag.md#user-selected-effort-and-run-budgets)
owns reasoning controls. H-06 must show the selected preset or custom limits,
effective estate caps, remaining time/steps/attempts and known cost, plus Stop
and an explicitly previewed extension. A stopped run shows partial results and
unresolved work with the engine's reason. Resume cannot reset consumption.
Until the runner and configuration-apply capabilities are advertised, these
controls must be unavailable with an explanation; changing a local UI setting
cannot claim an enforced timeout. All clients share the engine's authority.

## One object, many bounded lenses

Connectome presents several lenses over the same engine result. Changing a
lens changes presentation, not identity, read stamp, completeness, or truth.
The client cannot issue an undisclosed query, merge results from different
stamps, or fabricate missing fields to make a view appear complete.

Each rendered object or aggregate retains, as applicable:

| Coordinate | Required client behavior |
|---|---|
| Instance and scope | Retain the exact RRD instance resource and project/estate/resource path supplied by the operation; never infer scope from a label or active tab. |
| Read coordinate | Retain runtime cursor, manifest generation, valid time, and schema/catalogue/policy revisions supplied by the `ReadStamp`. |
| Semantic identity | Retain record, relation, event, reasoning-run/tree, routine-run, attunement-job, projection, plan, trace, and evidence identities rather than array position or display text. |
| Causality | Retain request, operation, transaction, subscription, trace, causation, and idempotency coordinates supplied by the result. |
| Provenance | Preserve source/evidence identities, decision and plan digests, model or deterministic-compute attribution, and authoritative receipts. |
| Completeness | Display denial, truncation, staleness, uncertainty, budget exhaustion, cursor gaps, and partial capability state explicitly; none can render as verified or complete. |
| Physical evidence | When exposed, render selected/skipped access paths and measured key/page/row/edge/candidate/byte/memory/spill/time work without treating the client as the planner. |

The functional workspaces are engine subjects rather than alternate stores:

- instance and project status;
- governed knowledge and schema;
- query, context, and retrieval plans;
- reasoning trees and runs;
- graph, timeline, table, route, comparison, and inspector views;
- attunement and automation jobs;
- evidence, traces, authorization, audit, and recovery.

Those subjects need not dictate one permanent sidebar. Graph, timeline, table,
route, comparison, and inspector are reusable view modes over typed results,
not tables or databases that Connectome maintains. Saved client-only layout,
filters, focus, column widths, and panel state are preferences. A shareable
durable lens, if later accepted, is a versioned typed rrflowDB record created
through an authorized `RrdEngine` operation; local storage cannot promote one.

## Graph, timeline, and context interaction

The default graph is centered on an explicitly selected record, relation,
event, run, file, symbol, plan, or evidence item. It displays typed edge
identity and direction and retains the stamp and traversal bounds. A global
orientation graph is optional, capability-gated, and bounded. Expanding a node
submits one explicit graph/query intent to RRD; the renderer cannot perform
client-side N+1 lookups, scan the estate, or traverse a locally reconstructed
graph as authoritative state.

A timeline consumes cursor-ordered engine events, routine/reasoning transitions,
trace evidence, and live-query deltas. Selecting or freezing one item stabilizes
its inspector while preserving surrounding typed lanes. Playback, comparison,
and scrubbing never change the underlying records or imply access to hidden
model chain-of-thought.

Context and reasoning views render the engine-owned plan: selected and skipped
seed, graph, BM25, exact-vector, HNSW/TurboQuant, cached, Arrow, and DataFusion
avenues; source cursor and contribution evidence; budgets; truncation; and final
`ContextPacket`. Connectome does not choose an index, run RRF, submit raw Arrow
buffers to a model, or repair missing evidence.

## Live delivery and reconnection

After authentication, Connectome uses the public multiplexed WebSocket contract
when that capability is available. Each subscription has an exact identity,
resource, predicate, starting cursor, outstanding-delivery budget, ACK state,
generation, lease, and cancellation coordinate. The client applies ordered
deltas once and acknowledges only the highest durably applied cursor.

On disconnect it resumes from the acknowledged cursor. A retention gap,
generation conflict, instance change, catalogue change, authorization change,
or invalid frame stops incremental application and requires an explicit
revalidation/resnapshot path. Connectome does not hide a gap by polling, replay
from cursor zero, or repeatedly rebuilding full snapshots. Backpressure,
cancellation, and stale-response suppression are visible client states.

## Actions, proposals, and presentation

Every user-visible mutation names one catalogued typed operation and exact
resource. Before an effectful action, Connectome shows the engine-issued preview
or plan digest, affected resources, permissions, expected effects, resource
budget, reversibility or compensation, uncertainty policy, and verification
criteria. Confirmation authorizes only the exact current plan under ordinary
RRD policy; a button, keyboard shortcut, model recommendation, or presentation
proposal grants no capability.

After submission, Connectome renders the authoritative denial, operation state,
commit receipt, uncertain-outcome state, cancellation result, or verification
evidence. It never advances attunement, reasoning, trigger, routine, or skill
state from animation completion, a trace event, an HTTP success code alone, or
a locally inferred outcome.

Tooltips, panels, graphs, and modals are selected from a client-owned accessible
component registry using bounded schema-validated public fields. The engine may
return typed data and presentation hints defined by the public contract; it
cannot send executable UI, arbitrary styles, hidden prose tags, or regex-parsed
control tokens. Every displayed claim identifies whether it is canonical data,
deterministic computation, a model proposal, or an operator action. Connectome
must not disguise deterministic database/rule output as unexplained AI
reasoning.

## Performance, accessibility, and failure behavior

All list, table, timeline, graph, trace, and context views honor server limits
and add their own bounded render budgets. Large lists are virtualized; requests
are cancellable; stale responses cannot replace newer stamps; and expensive
layout cannot block credential, cancellation, or acknowledgement handling.
Worker, `OffscreenCanvas`, WebGL, or another renderer is adopted only after a
repeatable node/edge/frame-time/memory profile proves the simpler bounded
renderer misses its target. The fallback retains the same typed identities and
accessibility behavior.

Keyboard navigation, focus management, reduced motion, semantic labels,
contrast, zoom, and screen-reader alternatives are acceptance behavior, not
polish. Shortcuts never fire from an incompatible focus context or bypass a
preview/confirmation gate. Stable deep links identify the instance candidate,
resource, lens, read/cursor coordinate, and selection needed to reconstruct the
view without embedding a credential, session identifier, or mutable display
label.

The UI distinguishes at least unavailable, disconnected, authenticating,
denied, stale, truncated, uncertain, cancelled, failed, degraded, and verified
states. It does not collapse them into a generic empty view or success badge.

## Multi-instance and external-system boundary

Connectome may aggregate several independently authenticated RRD instances for
operator navigation. Each panel, cache entry, route, subscription, action, and
error retains its exact instance identity and connection. The client cannot
merge their estate state, claim cross-instance ACID, reuse a session across
instances, or turn the first-alpha per-project estate into a fleet database.

PostgreSQL, Turso, SQLite, Dragonfly, object stores, model providers,
generators, harnesses, and meshes appear only through their installed RRFlow
adapter/capability records and engine-issued observations. Connectome does not
connect to them as hidden RRFlow persistence or infer that discovery activated
them. A future enterprise fleet control plane requires its own accepted public
contract; a client-local `rrflow-control` protocol cannot create one.

## Historical separate-checkout audit

The historical reviewed implementation baseline is the clean separate repository at
`../connectome`, commit
`38f68ce7adda9d03501f3591165f0e14996899ec` (`feat: add authenticated native
RRD client`). Its successful checks characterize useful code; they do not prove
H-06.

| Current implementation | Useful behavior retained as a requirement | Direct-convergence gap |
|---|---|---|
| Separate React/Tauri repository and production build | Connectome remains independently built and released. | The production bundle still carries SurrealDB client/UI/CBOR/Wasm aliases, QL Wasm packages, inherited surfaces, and multi-megabyte chunks. They are removed when the corresponding RRFlow surface exists; no compatibility or migration mode survives. |
| `src-tauri/src/rrd/client.rs` and `transport.rs` | Native loopback validation, redirect denial, response limits, one authenticated session, and explicit close have four passing focused tests. | The client handwrites a partial RRD contract, exposes generic JSON, supports only capabilities/session/diagnostics/context, has no network TLS or WebSocket path, and is not generated from or conformed against the shared public contract/SDK corpus. |
| `src/rrflow/runtime.ts` | Bounded URL construction and response-shape checks are useful client defenses. | It duplicates protocol truth and consumes the now-rejected scalar deployment list (`memory`, `embedded`, `local_daemon`, `edge`, `remote`, `distributed`). Replace it with generated types bound to the packaged and advertised RRFlow OpenAPI identity; retain no fallback decoder. |
| `src/rrflow/attunement.ts` and runtime-connection UI | The eleven phase names and honest progress presentation can be projected from RRD. | The client duplicates attunement phases, estimates, trigger/routine/hook/skill definitions, status, and static automation rows. All lifecycle state must come from public engine job records and capabilities. |
| `src/rrflow/diagnostics.ts` | A detailed trace/run inspector and the prompt-flight lens remain required views. | It defines a retired diagnostics protocol, provider-flight state and runners, and private `/api/*` routes that no engine serves. The flight lens is rebuilt on `flight-launch`, `flight-list`, and `flight-read` plus `diagnostics-read`; the retired protocol and routes are replaced by those public operations, not kept as a parallel lifecycle ([POAM-032](../../poam/rrflow-1.0-alpha/poam-032.md)). |
| `src/rrflow/control-plane.ts` | Multi-instance navigation remains a valid client use case. | It invents a client-owned `rrflow-control` protocol and managed-instance state without an accepted engine/fleet contract. It cannot be treated as current RRFlow capability. |
| `src/providers/Context/index.tsx` | Connection state belongs in one narrow client service. | It hardcodes an RRFlow cloud hostname, namespace/database, old vendor client, and fixed reconnect loop. Configuration, topology, credentials, and reconnection must come from the accepted endpoint/session contracts. |
| `tests/smoke/connectome.spec.ts` | The production overview and bootstrap screen render in Chromium. | The two tests mock liveness/readiness/capabilities, use stale `backend`, `deployment_mode`, and `0.1.0` values, and exercise no installed RRD process, authentication, WebSocket, graph/index/DataFusion/context flow, restart, denial, or resource bound. |

At this baseline, `pnpm run check`, `pnpm run build`, and the two browser smoke
tests pass. The build reports unresolved/browser-externalized SurrealDB QL/Wasm
paths and oversized chunks. Four focused native `rrd` tests pass. These results
prove a compilable, renderable, partially connected rough draft only.

RRFlow's own installed-lifecycle oracle now commits and queries a claim, kills
the serving process abruptly, starts the same installed project again, performs
fresh readiness and session handshakes, and returns the same claim through the
public Rust client before final offline verification. That proves a concrete
persistence target on the engine side; it does not prove the separate
Connectome artifact consumes it.

## Direct-convergence sequence

Implementation remains one reviewed package at a time in the separate
Connectome repository:

1. Generate or consume one qualified public RRD client surface bound to one
   exact packaged OpenAPI digest; implement the structured identity/deployment/
   configuration bootstrap and delete handwritten/old-vocabulary branches in
   the same package.
2. Implement the native credential, endpoint, HTTP/WebSocket, session,
   cancellation, retry/uncertainty, and W3C propagation boundary; keep the
   renderer outside the secret boundary.
3. Replace static attunement, diagnostics, automation, control, and runtime
   models with capability-gated projections of public typed operations; delete
   the retired diagnostics protocol, client-owned lifecycle definitions,
   hardcoded cloud topology, and the unaccepted control protocol.
4. Fold useful inherited components into the functional RRFlow lenses above and
   remove SurrealDB package aliases, identities, query/runtime paths, and
   migration language as their exact replacements land. Do not preserve a dual
   client path.
5. Add cursor/ACK live views, selected-object graph/timeline interaction,
   stamped context/reasoning evidence, explicit action preview, bounded
   rendering, accessibility, and failure-state coverage.
6. Run the installed real-engine corpus below and publish its exact source,
   artifact, configuration, and result digests. Only that package may satisfy
   H-06.

Each package starts by reading the complete affected files and their tests,
maps every useful behavior to one destination and acceptance case, implements
equal-or-stronger behavior, and removes the conflicting path. A rename, mock,
build, or broad deletion cannot substitute for absorption.

## H-06 and release acceptance

Connectome is qualified only when a clean checkout and signed artifact consume
the generated public protocol/SDK input at an exact digest and pass one
D-01-installed real-process corpus:

| Scenario | Required proof |
|---|---|
| Bootstrap and identity | Loopback HTTP and configured HTTPS/mTLS candidates follow live/ready/capabilities/session in order; wrong protocol, instance, transport identity, catalogue digest, readiness, and credentials fail closed. |
| Storage profiles | The same non-durability semantic corpus renders equal identities, stamps, results, denials, plans, and evidence on rrflowMX and rrflowKV; only rrflowKV advertises and proves close/reopen/recovery behavior. |
| Complete engine flow | One installed project shows committed knowledge, temporal graph, scalar/BM25/vector candidates, exact rerank/RRF, stamped Arrow/DataFusion escalation, persisted reasoning-tree transition, context packet, feedback, audit, and trace through public operations without a client-side duplicate. |
| Live delivery | Ordered update/delete deltas, ACK/backpressure, cancellation, reconnect/resume, retention gap, authorization change, catalogue change, instance rotation, and rrflowKV restart produce no loss, duplicate applied effect, cursor-zero replay, or hidden resnapshot. |
| Actions | Preview, deny, approve, idempotent replay, conflict, timeout-after-send, cancellation, compensation, and verification display the exact authoritative operation/receipt state and never advance from UI state. |
| Security | Renderer, routes, storage, logs, errors, traces, screenshots, diagnostics, crash reports, and exports contain no long-lived credential or secret; remote transport, CSP, origin, redirect, body/frame, and untrusted-input cases pass. |
| Interaction and resources | Graph/table/timeline/context views retain resource/stamp/evidence/truncation coordinates; bounded large-result tests report request/render memory, frames, cancellation, stale responses, and accessibility results. |
| Multiple instances | Two independently authenticated project instances remain isolated through navigation, caching, subscriptions, endpoint rotation, actions, and errors. |
| Release closure | Repository and artifact searches find no retired pre-RRFlow identity, SurrealDB runtime/client authority, old deployment scalar, hardcoded RRFlow cloud endpoint, client-owned attunement/automation truth, unaccepted control protocol, sibling-source dependency, or mock-only success path. |

Browser mocks remain useful UI tests, but they cannot satisfy a row. The
published conformance result binds the RRFlow and Connectome revisions,
executables/artifacts, generated-contract digest, install/configuration digest,
profile, platform, test corpus, raw failures, resource measurements, and
environment. H-06 remains unchecked until the roadmap records that evidence.

## Source-pinned interaction research

The historical workbench study was revalidated against Anytype `v0.55.4`, whose
annotated tag resolves to source commit
`f4677a073e41be6bfcf34a21b433027a3b3851aa`. It is an interaction reference,
not a dependency, source import, topology, or product model.

| Upstream observation | RRFlow disposition |
|---|---|
| A React/Electron frontend is separated from Go middleware through typed calls and streams. | Preserve the client/engine separation; use public RRD HTTP/WebSocket/SDK operations, not Anytype middleware or gRPC. |
| The same underlying objects can be rendered through multiple views. | Preserve projection-based lenses with exact resource and read coordinates; a view creates no second truth. |
| Global and selection-rooted graph views coexist. | Preserve a bounded selection-centered default and optional bounded global orientation. |
| Expensive graph layout/rendering uses `OffscreenCanvas`, a worker, D3, and PixiJS/WebGL. | Preserve only the measured escalation boundary; do not mandate this stack or use rendering performance to hide unbounded engine queries. |
| Browser mode explicitly differs from native mode and mocks unavailable native features. | Preserve explicit platform capability and test profiles; a mock/no-op cannot report real RRD conformance. |
| Blocks, sidebars, popups, and inspectors are composable presentation components. | Preserve reusable accessible inspectors and presentation registry; reject server-supplied executable UI and any client lifecycle authority. |

Primary source anchors:

- [Anytype v0.55.4 architecture guide](https://github.com/anyproto/anytype-ts/blob/f4677a073e41be6bfcf34a21b433027a3b3851aa/CLAUDE.md)
- [Anytype v0.55.4 browser/native boundary](https://github.com/anyproto/anytype-ts/blob/f4677a073e41be6bfcf34a21b433027a3b3851aa/docs/src/ts/lib/web/README.md)
- [Anytype v0.55.4 graph renderer](https://github.com/anyproto/anytype-ts/blob/f4677a073e41be6bfcf34a21b433027a3b3851aa/docs/src/ts/component/graph/README.md)
- [Anytype view behavior](https://doc.anytype.io/anytype/organize/views)

No Anytype identity, space/channel hierarchy, middleware, command, schema,
storage, source layout, or renderer becomes an RRFlow contract. The accepted
ideas above are independently specified and must pass RRFlow's own security,
semantic, resource, and conformance evidence.
