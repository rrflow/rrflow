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
and Connectome using correlated evidence from one engine.
