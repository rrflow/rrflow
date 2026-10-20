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
| [ ] | H-09 | Implement the configurable [prompt graph families](../../reference/data/rrflowdb-capability.md#prompt-graph-families): family definitions in the schema registry, activation and experiment arms in the estate configuration revision, the bundled default set (lexicon, classification, reasoning, plan/task, evidence, source, knowledge including memory, governance), and atomic formation of every active family in the prompt's own commit with per-edge provenance. Close the public claim gap so `AssertClaim` states `valid_to`, tier, promotion, and supersession. | `rrd-contract`, `rrd-core`, `rrd-engine` | Golden registry and activation fixtures; one commit per flight with skipped families recorded by reason; rrflowQL traverses each family; a seeded two-arm experiment promotes only when grade and sample both pass; corrections supersede without loss and old stamps read unchanged after reopen on rrflowMX and rrflowKV. |
| [ ] | H-10 | Implement opt-in read-only [project links](../../reference/data/rrflowdb-capability.md#project-links): offer and acceptance records in both estates, revocation, foreign references, and federated reads at the workspace layer that carry both read stamps and are audited in both estates. | `rrd-contract`, `rrd-engine`, `rrd-client`, workspace layer | Two-party opt-in; revocation stops resolution at the next read; a linked read never writes the target and returns both stamps and both evidence sets; an unlinked or denied reference resolves to unavailable; links work across separate instances through H-07 resolution. |

H-07 also owns the explicit [DevPulse/shippin.cloud account and sync integration](../../reference/client/devpulse-cloud.md),
after D-06 authorized adapter bindings and H-04 public-operation prerequisites.
Local operation must remain account-independent. Account linking, explicit data
selection/consent, durable transfer checkpoints, duplicate/replay handling,
conflict and revocation behavior require the real-service acceptance in that
contract; endpoint reachability alone cannot qualify synchronization.

## H-07 DevPulse, managed cloud and homelab packages

The [existing integration contract](../../reference/client/devpulse-cloud.md)
owns the required local VM/container experience and bidirectional transfer to
managed or operator-controlled destinations. All packages below remain open.
They are dependencies to implement, not completed work or new engine APIs.
The native engine must remain independently installable and operable.

| Package | Prerequisites and canonical boundary | Smallest oracle and required exit |
|---|---|---|
| H-07a: executable sync contract and admitted profile | Existing contract; inspect `rrd-contract` operation catalogue, `rrd-engine` transaction/idempotency boundary, `rrd-store` committed change groups and public export coverage. Specification/fixture work may begin now. | Versioned cases name each selected family, source/destination identity, atomic group, snapshot/cursor handoff, conflict policy and unsupported condition. Absent public import/export reports unimplemented, never a passing sync fixture. |
| H-07b: native durable import and origin receipts | H-07a, accepted D-01/C-07 durability, H-04 public-operation foundation; extend the existing core/store/engine transaction boundaries and generated public catalogue. | Two installed engines apply one selected source commit once across lost ACK, new session, restart and credential rotation. Source origin, deduplication and destination mutations commit atomically. Tampered digest, foreign binding, incompatible schema and partial group reject without effects. |
| H-07c: snapshot handoff and bidirectional reconciliation | H-07b; engine-owned bindings/checkpoints/conflicts, bounded public transfer adapter. | No gap between initial snapshot and changes; concurrent edits/deletes become declared conflicts or converge by the accepted policy; three peers do not echo commits. Retention gap, full disk, revocation and slow peers preserve acknowledged state and bounded work. |
| H-07d: native service distribution profiles | D-01/D-06/D-11 and C-07; current CLI/server lifecycle and distribution tooling, using the J-03/J-05 artifact requirements. Final release qualification consumes this package; it is not a circular prerequisite. | Exact Linux/Windows/macOS artifacts and supported service images install, authenticate, persist, recover, back up and restore with no checkout/toolchain. TLS/identity, tenant isolation, quotas and ownership-safe removal are proved for managed and self-hosted profiles; no default SaaS prerequisite. |
| H-07e: DevPulse project-runtime integration | Authoritative DevPulse checkout identified; H-07d and existing authorized VM/container providers; consume RRFlow public SDKs without a sibling build dependency. | A packaged DevPulse build provisions a supported project runtime, builds/runs Rust/Python/Go fixtures, opens their private UIs and reopens local RRFlow data without a hosted account. Failed/cancelled provisioning cleans only owned resources. |
| H-07f: selected project files and environment transfer | D-03/D-06 inventory/effect contracts, H-07e; existing project inventory and provider adapters. | File/content digests and environment revisions are independently verified; collisions, excludes, secrets, deletes and interrupted transfers are explicit. No implicit raw database or live-VM-memory copying. Database convergence cannot hide failed file/environment transfer. |
| H-07g: real managed/homelab end-to-end qualification | H-07c through H-07f; actual managed identity/control service and separately administered homelab. | One DevPulse project exercises both directions, offline concurrent changes, lost ACK, delete/schema conflict, credentials/revocation, restore and interrupted upgrade. Repeat with shippin.cloud unavailable on the homelab path. Correlated source/destination receipts, pending/conflict state and all required failures are retained. |

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
| H-08b: canonical relational values and catalogue | H-08a; D-01 and C-04/C-07 accepted for the installed composition | `rrd-core/src/runtime.rs`, `schema.rs`; `rrd-store` key codec and semantic commit; `rrd-query` catalogue | Exact type/OID/typmod/null/default/constraint-declaration round trips, atomic catalogue mutations, schema invalidation and native relational identity survive reopen; no opaque SQL-only store or unsafe numeric casts. Extend the canonical schema/values and replace affected serializers directly with traced tests. |
| H-08c: SQL transaction semantics | H-08b | `rrd-engine/src/engine/transaction.rs`; `rrd-store/src/transaction.rs` and semantic repositories | Two-session schedules prove READ COMMITTED statement snapshots, REPEATABLE READ, predicate recheck, read-your-writes, savepoint/statement rollback, aborted-state handling, unique/FK races, lock timeout/deadlock behaviour and atomic all-family commit. Kill at each prepare/WAL/sync/ack boundary; no acknowledged write is lost. SERIALIZABLE requests are explicitly rejected. |
| H-08d: PostgreSQL SQL lowering | H-08b/H-08c; E and F shared access/streaming/budget prerequisites | `rrd-query/src/syntax.rs`, `plan.rs`, `execute.rs`, `fusion.rs`, `arrow.rs`; `rrd-engine` query/mutation operations | Every admitted AST/type/coercion/SELECT/DDL/DML case equals the reference and the native engine; unsupported syntax rejects before mutation. Data larger than memory streams within configured limits. No second executor, direct DataFusion commit, or eager whole-estate workaround. |
| H-08e: authenticated wire and bulk streaming | H-08c/H-08d; H-04 catalogue/dispatch foundation | `rrd-server` composition and new PostgreSQL module within that crate; `rrd-security`; `rrd-contract` catalogue projections; existing install/serve configuration | All seven real clients connect, prepare/bind binary values, transact, cancel and recover from errors; COPY abort rolls back. Fragmented/oversized frames, wrong principal, stale plan, expired credentials and cross-session cancellation fail safely. Adapter source checks prohibit storage access. |
| H-08f: catalogue and AI SQL surface | H-08e; E/F native vector and shared plans | `rrd-query` catalogue and native vector operators; `rrd-engine` discovery/diagnostics; generated contract projections | Real client discovery returns authorized metadata; vector columns map to the canonical collection/named-vector address; exact-distance and indexed-recall cases match admitted semantics. SQL/native writes and reasoning/context reads agree at one stamp; protected state cannot be mutated through SQL. |
| H-08g: source exit and portable data | H-08f; D-06 binding and D-11 backup/restore acceptance | Existing `rrflow-cli` installed lifecycle, `rrd-engine` authorized inventory/mutation and durable job boundaries | Synthetic SQLite/libSQL/Turso-style exports expose affinities, NULLs, binary, time and vector mismatches; preview rejects loss, resumable import duplicates no rows, per-table digests/counts match, logical export and native restore reopen identically. Cutover simulation retains source and reconciles post-cutover writes before rollback; no live migration without its own binding. |
| H-08h: installed compatibility qualification | H-08a through H-08g; H-05 evidence chain | Existing real-process/candidate-CI and release harnesses | Complete version-pinned support report on installed Linux/Windows/macOS servers, all required cases passing, native cross-surface identity, denial/resource/concurrency/fault corpus and source-exit rehearsal retained. J-02/J-03/J-05 consume these results; no mock or checkout-only proof substitutes. |

H-08a's reference/specification work can begin immediately; runtime edits obey
the [dependency spine](executable-dependency-spine.md). No dependency is deemed
complete by this plan. Before each implementation PR, resolve exact symbols
and runnable test commands from that checkout; proposed test names are not
reported as existing or passed. Completion is per oracle, never a fixed date.

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

## Exit condition

Gate H exits only after one prompt can be followed from ingress through LFG or
analytical routing, storage/index work, fused context, mutation, live delivery,
and Connectome using correlated evidence from one engine, and after H-08's
required PostgreSQL core-profile corpus qualifies that same installed engine.
