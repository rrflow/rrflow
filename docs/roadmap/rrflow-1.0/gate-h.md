# Gate H — prove context flow, feedback, live delivery, and Connectome

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-h`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | H-01 | Route each context request dynamically across eligible seed, BM25, vector, graph, and cached paths using the captured catalogue and budgets. | `rrd-engine` | Plan evidence states selected/skipped reason, source cursor, work, and contribution for every avenue. |
| [ ] | H-02 | Keep query-time RRF pure; persist explicit verified outcomes and learn versioned weight policies only for later read stamps. | `rrd-engine`, `rrd-query` | Replay at an old stamp is unchanged; feedback update, rollback, cold-start, and quality-regression tests pass. |
| [ ] | H-03 | Replace two-snapshot live-query diffing with commit-impact evaluation and predicate-specific deltas. | `rrd-query`, `rrd-engine` | Ordered update/delete/reconnect/backpressure tests emit each matching committed delta once without full-query rescans. |
| [ ] | H-04 | Serve HTTP, multiplexed WebSocket, Rust SDK, generated SDKs, CLI, MCP, and GraphQL adapter through the same catalogue-derived operations and authorization semantics. One deterministic compiler invocation projects the executable wire types and operation/capability catalogue into internal dispatch metadata, OpenAPI, Rust and language bindings/models, reference coverage, and the shared conformance matrix; no surface maintains a handwritten duplicate registry. Every supported SDK binds every available descriptor exactly once; validates the complete request, response, media, status, protocol, identity, stamp, and receipt contract; redacts credential-bearing state; and applies one explicit operation-semantic retry, cancellation, and uncertain-outcome policy. A missing generator or conformance configuration cannot report success. | transport/adapters | A D-01-installed real-process corpus structurally proves each operation and fault scenario against rrflowMX and rrflowKV where applicable, then sends the same requests through every surface and compares status, denial, stamp, digest, receipt, result, trace, restart, and resource evidence. Repeated clean generation is byte-identical; catalogue/router/OpenAPI/internal client/generated SDK/reference/conformance projections have no missing or extra operation; manual-registry source guards and credential/error/frame-limit adversarial cases pass. |
| [ ] | H-05 | Complete the per-gate trace work as one bounded causal chain across ingress, authorization, planning, selected/skipped context avenues, KV/page scans, graph, BM25, HNSW, DataFusion, LFG, cache and model-context compaction effects, model/tool attempts, verification, commit, attunement, and delivery; add W3C propagation plus redacted diagnostic export without creating another authority. | `rrd-engine`, adapters | Canonical-name, propagation, completeness, crash, export, redaction, and fixed-rubric before/after tests retain stamps, plan/projection/source identities, work/byte/token/latency accounting, contributions, and outcomes; repeated work, stale/duplicate/conflicting context, route misses, and model-context compaction loss are detectable; traces observe authoritative job/state records rather than becoming lifecycle state or hidden chain-of-thought. |
| [ ] | H-06 | Implement the separate [Connectome client contract](../../reference/client/connectome.md) completely on catalogue-derived public RRD operations. The client validates exact endpoint, transport, protocol, instance, deployment, catalogue, session, resource, stamp, cursor, evidence, completeness, and receipt coordinates; renders only engine-issued state/plans/traces/trees/context/deltas/proposals; and contains no alternate storage, query, retrieval, reasoning, attunement, automation, diagnostics, control, provider, or presentation authority. | separate Connectome repository, pinned at `connectome/` | A clean Connectome artifact consumes the generated public-contract/SDK digest and runs against D-01-installed rrflowMX and rrflowKV processes. HTTP bootstrap, authenticated session, multiplexed WebSocket resume/ACK/backpressure, graph/BM25/vector/RRF/Arrow/DataFusion/context/reasoning evidence, previewed mutations, denial/uncertainty/cancellation, restart, multi-instance isolation, credential redaction, bounded rendering, and accessibility pass with correlated identities/stamps/digests/traces. Mocks, screenshots, builds, and open ports remain non-qualifying. |
| [ ] | H-07 | Resolve loopback or explicitly configured network endpoint candidates, including shippin.cloud devspaces and optional Zuul Zero/shippin.ai mesh resolution, through an outward transport adapter, carrying the expected RRD instance and transport-security identities independently of reachability. Re-negotiate liveness, readiness, protocol, instance, catalogue digest, and authentication after rotation; no resolver initializes RRFlow or selects storage. | `rrd-client`, mesh adapter | Laptop/phone/devspace fixture proves bounded resolution, TLS identity, exact instance/capability negotiation, endpoint rotation, stale/foreign/offline denial, and no mesh-owned database, authorization, installation, or lifecycle state. |
| [ ] | H-09 | Implement configurable [prompt graph families](../../reference/data/rrflowdb-capability.md#prompt-graph-families), with baseline lexicon, classification, reasoning, plan/task, evidence, source, knowledge and governance definitions. Select the needed families and relationships from the request and effective policy. Keep prompt-local graphs bounded and ephemeral unless an authorized policy selects durable publication; committed relationships carry provenance and use the same transaction authority. Close the public claim gap so `AssertClaim` states `valid_to`, tier, promotion, and supersession. | `rrd-contract`, `rrd-core`, `rrd-engine` | Different requests select different family sets and report skipped reasons. Prompt-local state expires as configured; selected durable facts commit atomically, survive reopen and retain source/edge provenance. rrflowQL traverses admitted families; corrections preserve historical stamps and experiments promote only on qualified evidence. |
| [ ] | H-10 | Implement opt-in read-only [project links](../../reference/data/rrflowdb-capability.md#project-links): offer and acceptance records in both engrams, revocation, foreign references, and federated reads at the workspace layer that carry both read stamps and are audited in both engrams. | `rrd-contract`, `rrd-engine`, `rrd-client`, workspace layer | Two-party opt-in; revocation stops resolution at the next read; a linked read never writes the target and returns both stamps and both evidence sets; an unlinked or denied reference resolves to unavailable; links work across separate instances through H-07 resolution. |
| [ ] | H-11 | Implement configurable [engram organisation](../../reference/data/engram-organisation.md) (packages H-11a to H-11g). The schema registry gains domain and category definitions and the membership of every kind and vector collection, and the inert `namespace`/`database` label is retired. Record tags are kept in the reserved bi-temporal tag family. rrflowQL, context assembly, vector search, catalogue, diagnostics and changefeed reads accept a domain and tag scope applied before candidate generation. Isolated domains are enforced through domain grants, with their own lexical statistics. Diagnostics and retrieval evidence report per domain, including returned, delivered and used context. Attunement may propose an organisation from the project tree, and organisation experiments are promoted only by measurement. | `rrd-contract`, `rrd-core`, `rrd-query`, `rrd-security`, `rrd-engine`, `rrd-server`, generated SDKs | Golden registry and query vectors pass. Unknown domains, cycles, unassigned kinds and vocabulary violations fail before the cursor advances. Moving a kind keeps record identity, and old stamps read the old organisation. Scoped results equal an independent full-replay oracle with no post-filtered truncation, and every result names its scope and revision. Every read surface denies an isolated domain without its grant. Per-domain evidence reconciles with an independent recount. All of it is identical on rrflowMX and rrflowKV after close and reopen. |

H-07 also owns the explicit [DevPulse/shippin.cloud account and sync integration](../../reference/client/devpulse-cloud.md),
after D-06 authorized adapter bindings and H-04 public-operation prerequisites.
Local operation must remain account-independent. Account linking, explicit data
selection/consent, durable transfer checkpoints, duplicate/replay handling,
conflict and revocation behavior require the real-service acceptance in that
contract; endpoint reachability alone cannot qualify synchronization.

## Turnkey client and service proof

H-06 consumes the [minimum-input project flow](../../reference/client/connectome.md#turnkey-project-entry-and-progressive-detail)
and existing D-02/D-05 phase evidence. Its next client prerequisite is one
qualified current engine/SDK/install-adapter pair; the pinned older workbench
cannot establish current installation, effort or indexing behavior. Wire the
existing task-effort controls at their current operations before claiming a
complete effort UI; AI-inherited preference resolution remains the separately
specified G/D configuration work. No new top-level feature area substitutes for
clear nested project data and evidence views. The
[workspace learning and controller contract](../../reference/client/connectome.md#learning-in-the-workspace-and-controller-configuration)
requires contextual tooltips, evidence-based suggestions, novice-to-advanced
drill-down and global/project LFG and SWAG System One controls. Prove actual
configuration readback and observed behavior with the G-owned execution path;
labels, local UI preferences and tooltips alone do not qualify those controls.

H-05 requires causal signals for setup/admission, source inventory, parsing,
embedding/index publication, retrieval, model dispatch, commit, Stop and retained
readback. Each carries the available authoritative project/run/configuration/
source identity, outcome and bounded resource evidence. A missing stage is
unavailable, not a zero-duration success. UI progress reports admitted/completed
units separately; an unknown total cannot produce a fabricated percentage.
Distinguish installed, indexed, retrieval-ready and learning-enabled behavior
using current engine evidence rather than a single connected/green badge.

H-07's [rroflow.ai handoff](../../reference/client/devpulse-cloud.md#turnkey-rroflowai-handoff)
adds real account/transfer proof to G/I learning proof. Login alone closes no
learning/synchronization requirement. Keep the all-local project proof independent
of cloud availability and retain no-sharing defaults during parallel projects.

## Resource-managed project instances

This is a later H-06/H-07 target after the one-project developer journey is
usable. The operator can register multiple project engrams and set host-wide and
per-instance resource budgets; registered projects do not all have to remain
running. Start or attach to the exact authenticated instance, observe admitted
and queued work, and optionally stop an idle instance only under the operator's
configured policy. Resume reopens its durable engram and rechecks its identity.
Report effective limits, consumption, pending/denied admission, shutdown reason,
recovery and unavailable measurements. Do not promise an unbounded number of
simultaneous engines or turn a fixed client count into a product entitlement.

The Connectome revision pinned by RRFlow main at the starting baseline
(`c1cbff203ecee89013878609d11aed04e89d0e71`) uses `MAX_OWNED = 4` and
`MAX_TRACKED = 8` in `src-tauri/src/rrd/engram/lifecycle.rs`. These are current
implementation limits, not evidence of resource-aware admission. Replace the
limits only through the separate Connectome ownership boundary and its public
RRD lifecycle contracts, after characterizing existing shutdown and session
behavior; preserve protections against unbounded memory and process creation.

Acceptance requires a current packaged client and installed engines: register
more projects than the concurrent budget allows; exercise fair or documented
queueing, stop/resume and restart with durable readback; verify identity and
credentials remain isolated; deny or defer work under CPU/memory/process
pressure; and show truthful progress and resource measurements. Test a process
started elsewhere, a crash during shutdown, a live operation at an idle deadline,
and an unavailable measurement. Never kill an externally owned process, discard
an acknowledged write, or silently grant cross-project reads. Record the exact
host, budget, process ownership, revisions and observed outcomes. Cloud and
self-hosted control-plane profiles need separate isolation, identity, quota and
recovery evidence; a local connection does not qualify them.

## H-07 DevPulse, managed cloud and homelab packages

The [existing integration contract](../../reference/client/devpulse-cloud.md)
owns the required local VM/container experience and bidirectional transfer to
managed or operator-controlled destinations. All packages below remain open.
They are dependencies to implement, not completed work or new engine APIs.
The native engine must remain independently installable and operable.

The [Nomen identity and delivery priority](../../reference/client/devpulse-cloud.md#nomen-identity-and-delivery-priority)
binds the first managed profile: local RRFlow synchronizes both ways with
shippin.cloud using Nomen's central login. Resolve the actual Nomen contract in
H-07a; qualify account/service/project binding, expiry and revocation alongside
H-07b/c/d. First demonstrate that engine-only sync flow without waiting for
DevPulse VM/container provisioning, then complete the H-07e/f/g product flow.
Additional custom/homelab profiles use the same configurable contracts and
retain their own acceptance; this priority closes none of them. H-07g must
prove shared login does not confer cross-project/service access, local work
survives identity/cloud outages, and reconnect resumes authorized durable
changes without loss or duplication. An authenticated page alone is no sync
proof. No Nomen protocol or production endpoint is assumed by this plan.

H-06/H-07 must also prove the [guided sharing and resumption flow](../../reference/client/devpulse-cloud.md#required-user-behavior):
automatic local attachment; one clear service-sharing/sync choice; reuse of
valid Nomen and provider sessions; expired-session reconnect returning to the
pending action; automatic restart/offline recovery under saved authorization;
and a changed-scope prompt without restarting setup. Test a selected supported
AI client consuming actual project context, identify subscription versus API
billing, and retain a declined-sharing case. A novice must not configure
individual grants. Provider/client support can be delivered incrementally;
unsupported adapters stay explicit, not a reason to block the usable local flow.

| Package | Prerequisites and canonical boundary | Smallest oracle and required exit |
|---|---|---|
| H-07a: executable sync contract and admitted profile | Existing contract; inspect `rrd-contract` operation catalogue, `rrd-engine` transaction/idempotency boundary, `rrd-store` committed change groups and public export coverage. Specification/fixture work may begin now. | Versioned cases name each selected family, source/destination identity, atomic group, snapshot/cursor handoff, conflict policy and unsupported condition. Absent public import/export reports unimplemented, never a passing sync fixture. |
| H-07b: native durable import and origin receipts | H-07a, accepted D-01/C-07 durability, H-04 public-operation foundation; extend the existing core/store/engine transaction boundaries and generated public catalogue. | Two installed engines apply one selected source commit once across lost ACK, new session, restart and credential rotation. Source origin, deduplication and destination mutations commit atomically. Tampered digest, foreign binding, incompatible schema and partial group reject without effects. |
| H-07c: snapshot handoff and bidirectional reconciliation | H-07b; engine-owned bindings/checkpoints/conflicts, bounded public transfer adapter. | No gap between initial snapshot and changes; concurrent edits/deletes become declared conflicts or converge by the accepted policy; three peers do not echo commits. Retention gap, full disk, revocation and slow peers preserve acknowledged state and bounded work. |
| H-07d: native service distribution profiles | D-01/D-06/D-11 and C-07; current CLI/server lifecycle and distribution tooling, using the J-03/J-05 artifact requirements. Final release qualification consumes this package; it is not a circular prerequisite. | Exact Linux/Windows/macOS artifacts and supported service images install, authenticate, persist, recover, back up and restore with no checkout/toolchain. TLS/identity, tenant isolation, quotas and ownership-safe removal are proved for managed and self-hosted profiles; no default SaaS prerequisite. |
| H-07e: DevPulse project-runtime integration | Authoritative DevPulse checkout identified; H-07d and existing authorized VM/container providers; consume RRFlow public SDKs without a sibling build dependency. | A packaged DevPulse build provisions a supported project runtime, builds/runs Rust/Python/Go fixtures, opens their private UIs and reopens local RRFlow data without a hosted account. Failed/cancelled provisioning cleans only owned resources. |
| H-07f: selected project files and environment transfer | D-03/D-06 inventory/effect contracts, H-07e; existing project inventory and provider adapters. | File/content digests and environment revisions are independently verified; collisions, excludes, secrets, deletes and interrupted transfers are explicit. No implicit raw database or live-VM-memory copying. Database convergence cannot hide failed file/environment transfer. |
| H-07g: real managed/homelab end-to-end qualification | H-07c through H-07f; actual managed identity/control service and separately administered homelab. | One DevPulse project exercises both directions, offline concurrent changes, lost ACK, delete/schema conflict, credentials/revocation, restore and interrupted upgrade. Repeat with shippin.cloud unavailable on the homelab path. Correlated source/destination receipts, pending/conflict state and all required failures are retained. |
| H-07h: local served-endpoint record | Accepted D-01 `serve`/`ready` and private engram directories; `rrflow-cli` serve/ready, `rrd-engine` engram layout and open. It serves the automatic local attachment step of the guided flow and depends on none of H-07a–g. | Per the [served endpoint record](../../reference/deployment/installed-lifecycle.md#served-endpoint-record): a served engram publishes `.rrflow/serving.json` with no credential; `rrflow ready --project` without an address proves it; a killed server's record is not ready and the next serve replaces it; a clean shutdown removes it; a second serve fails before binding, names the recorded URL and leaves the running server and its record untouched; the locator still contains no endpoint. |

Each runtime package first resolves exact symbols and tests from its current
checkout and lands through its own PR/CI. Do not jump from an HTTP connection,
container startup or UI animation to H-07g completion. J-04's competitive proof
is separate from these functional/recovery cases.

## H-08 PostgreSQL compatibility packages

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | H-08 | Implement and qualify the [rrflowDB PostgreSQL core profile](../../reference/protocol/postgresql.md) through the one native engine, including independent data portability. | `rrd-core`, `rrd-query`, `rrd-engine`, `rrd-store`, `rrd-server`, installed lifecycle | All required SQL/type/wire/client cases pass against a locked PostgreSQL reference and installed native rrflowDB; no required skip, silent coercion, authorization bypass, partial commit, vendor runtime, or unqualified support claim. |

The contract owns feature scope; the packages below own dependency order and
completion evidence. All are open. Each package is one bounded PR with its own
code-grounded traceability and retained failure oracle; split a package into
smaller same-owner PRs if its concrete diff cannot remain reviewable.

| Package | Prerequisites | Exact existing boundaries to extend | Smallest failure oracle and required exit |
|---|---|---|---|
| H-08a: executable compatibility specification | This contract; no production listener | Test/fixture tooling at the `rrd-server` and `rrd-query` boundaries; repository source/dependency policy | Lock PostgreSQL 18 reference patch/image/source digest and seven client versions; assign case IDs to every required matrix cell and explicit exclusion; freeze parameter/function/type and resource-cap allowlists. Missing configuration or unavailable listener reports failure/unimplemented, never pass. Run reference cases; record the current server's expected incompatibility. Evaluate pinned pgwire/sqlparser without enabling a new dependency implicitly. |
| H-08b: canonical relational values and catalogue | H-08a; D-01 and the C-04/C-07 correctness evidence consumed by the changed operation | `rrd-core/src/runtime.rs`, `schema.rs`; `rrd-store` key codec and semantic commit; `rrd-query` catalogue | Exact type/OID/typmod/null/default/constraint-declaration round trips, atomic catalogue mutations, schema invalidation and native relational identity survive reopen; no opaque SQL-only store or unsafe numeric casts. Extend the canonical schema/values and replace affected serializers directly with traced tests. Full C-07 capacity/soak remains required for qualification, not for beginning safe implementation. |
| H-08c: SQL transaction semantics | H-08b | `rrd-engine/src/engine/transaction.rs`; `rrd-store/src/transaction.rs` and semantic repositories | Two-session schedules prove READ COMMITTED statement snapshots, REPEATABLE READ, predicate recheck, read-your-writes, savepoint/statement rollback, aborted-state handling, unique/FK races, lock timeout/deadlock behaviour and atomic all-family commit. Kill at each prepare/WAL/sync/ack boundary; no acknowledged write is lost. SERIALIZABLE requests are explicitly rejected. |
| H-08d: PostgreSQL SQL lowering | H-08b/H-08c; E and F shared access/streaming/budget prerequisites | `rrd-query/src/syntax.rs`, `plan.rs`, `execute.rs`, `fusion.rs`, `arrow.rs`; `rrd-engine` query/mutation operations | Every admitted AST/type/coercion/SELECT/DDL/DML case equals the reference and the native engine; unsupported syntax rejects before mutation. Data larger than memory streams within configured limits. No second executor, direct DataFusion commit, or eager whole-engram workaround. |
| H-08e: authenticated wire and bulk streaming | H-08c/H-08d; H-04 catalogue/dispatch foundation | `rrd-server` composition and new PostgreSQL module within that crate; `rrd-security`; `rrd-contract` catalogue projections; existing install/serve configuration | All seven real clients connect, prepare/bind binary values, transact, cancel and recover from errors; COPY abort rolls back. Fragmented/oversized frames, wrong principal, stale plan, expired credentials and cross-session cancellation fail safely. Adapter source checks prohibit storage access. |
| H-08f: catalogue and AI SQL surface | H-08e; E/F native vector and shared plans | `rrd-query` catalogue and native vector operators; `rrd-engine` discovery/diagnostics; generated contract projections | Real client discovery returns authorized metadata; vector columns map to the canonical collection/named-vector address; exact-distance and indexed-recall cases match admitted semantics. SQL/native writes and reasoning/context reads agree at one stamp; protected state cannot be mutated through SQL. |
| H-08g: source exit and portable data | H-08f; D-06 binding and D-11 backup/restore acceptance | Existing `rrflow-cli` installed lifecycle, `rrd-engine` authorized inventory/mutation and durable job boundaries | Synthetic SQLite/libSQL/Turso-style exports expose affinities, NULLs, binary, time and vector mismatches; preview rejects loss, resumable import duplicates no rows, per-table digests/counts match, logical export and native restore reopen identically. Cutover simulation retains source and reconciles post-cutover writes before rollback; no live migration without its own binding. |
| H-08h: installed compatibility qualification | H-08a through H-08g; H-05 evidence chain | Existing real-process/candidate-CI and release harnesses | Complete version-pinned support report on installed Linux/Windows/macOS servers, all required cases passing, native cross-surface identity, denial/resource/concurrency/fault corpus and source-exit rehearsal retained. J-02/J-03/J-05 consume these results; no mock or checkout-only proof substitutes. |

H-08a's reference/specification work can begin immediately; runtime edits obey
the [dependency spine](executable-dependency-spine.md). No dependency is deemed
complete by this plan. Before each implementation PR, resolve exact symbols
and runnable test commands from that checkout; proposed test names are not
reported as existing or passed. Completion is per oracle, never a fixed date.

## H-12 SQLite compatibility target

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | H-12 | Define and implement a versioned SQLite compatibility profile over the same native rrflowDB/rrflowQL authority. Specify separately the admitted SQL/type/transaction behavior, application API or ABI, and SQLite file import/export or direct file compatibility; select and document actual surfaces before claiming them. | `rrd-core`, `rrd-query`, `rrd-store`, `rrd-engine`, client adapters | A pinned SQLite reference and independent real application cases compare the admitted statements, parameters, affinity/NULL and transaction behavior, errors, constraints and restart. Any promised API/ABI or file behavior has its own exact cross-read/write and failure corpus; unimplemented surfaces are visible and never inferred from migration or SQL parsing. |

SQLite's value typing and affinity differ from PostgreSQL's type behavior; the
engine must preserve the selected profile's semantics without a second storage
authority. The native RRFlow format is not a SQLite database file merely because
some SQL is accepted. Source database access remains a separately configured
knowledge path, and no external SQLite service or runtime becomes a default
readiness prerequisite. Profile definition can proceed independently; its
implementation must converge with H-08's shared types, transactions and plans.

## Knowledge and project placement acceptance

H-09 uses the [knowledge connector provenance contract](../../reference/data/knowledge-connectors.md)
for source/chunk/transformation breadcrumbs and ACL/delete invalidation.
H-10 link consent is independent of copying, synchronization, model cache
sharing and physical placement. H-11 domain isolation is different: it divides
one engram, so an isolated domain stays in its engram's transactions, cursor
and backup, while a project link connects two engrams. H-07e implements the
[project sharing and isolation UI contract](../../reference/client/devpulse-cloud.md#project-preferences-sharing-and-isolation):
private by default, exact peer selection, separate VM/container constraints,
cost preview and observed enforcement. Test shared/private context crossed
with dedicated/explicitly permitted shared placement, plus revocation and
foreign-account/volume denial. Unsupported isolation fails; account login
and cost optimization cannot silently widen consent. These requirements do
not mark any H item complete.

## H-05 execution packages

H-05 is executed as five bounded packages rather than one late observability
rewrite:

### H-05a — build and signal identity

add the closed metric contract,
release-semantic diagnostic Cargo profile, machine-readable build identity,
runtime diagnostic levels, and automated release/diagnostic feature,
catalogue, format, result, receipt, and limit parity. This prerequisite must
exist before D-01 qualifies installed binaries or later packages emit new
signal families.

The first bounded implementation supplies [primary executable build provenance](../../reference/release/build-identity.md):
embedded compiler/source/actual-feature inputs, local version/startup identity,
dirty-source and drift rejection, and a symbols-only release-derived profile.
The [package journal](../../evidence/change-journals/gate-h/h-05a-primary-build-provenance.md)
records its tests. This does not complete H-05a: format/bundle identity, remote
discovery, runtime level enforcement and actual release/diagnostic semantic
parity remain required.

### H-05b — causal context

replace direct-store trace emission with one
`RrdEngine` path and implement bounded W3C ingress/egress continuation,
asynchronous links, terminal outcomes, crash-visible incomplete starts, and
cross-request isolation.

### H-05c — correlated diagnostics

project the same operation catalogue to
structured logs, OpenTelemetry traces, aggregatable histograms/counters/
gauges, trace exemplars, bounded queues/cardinality, and exporter
self-telemetry. Diagnostic loss never changes authoritative state.

### H-05d — physical path coverage

complete stage/work/resource evidence
with the owning C-through-I behavior for rrflowKV, graph, BM25, vector/
TurboQuant candidates, rrflowQL/DataFusion, context, LFG, routines,
attunement, commit, and delivery. DataFusion metrics are inputs to RRFlow
evidence, not another authority.

### H-05e — capture and proof

add the sanitized manifest-bound diagnostic
bundle and fixed corpus for propagation, redaction, exporter failure,
overhead/cardinality, deterministic faults, crash/reopen, repeated/missing
work, release/diagnostic parity, and the complete prompt-to-delivery chain.

The exact instruments, allowed dimensions, build modes, timing boundaries,
capture contents, and fault lanes are owned by the
[engine data-flow architecture](../../architecture/engine-data-flow.md#runtime-modes-build-profiles-and-build-identity).
No average-only report, development-profile timing, unbound debug log, or
passing exporter test qualifies H-05.

H-05 also owns the [project evidence and model-usage accounting contract](../../architecture/engine-data-flow.md#project-evidence-model-usage-and-reasoning-efficiency). Implement its public signals and project/session drilldown through the existing catalogue and Connectome boundary; a planned table is not emitted telemetry.

## Exit condition

Gate H exits only after one prompt can be followed from ingress through LFG or
analytical routing, storage/index work, fused context, mutation, live delivery,
and Connectome using correlated evidence from one engine, and after H-08's
required PostgreSQL core-profile and H-12's selected SQLite compatibility
corpora qualify that same installed engine.
