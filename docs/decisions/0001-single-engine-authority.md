# ADR-0001: One RRFlow engine authority with explicit execution profiles

**Status:** active accepted architecture decision
**Coordinate:** `rrflow://rrflow-instance/data/decision/0001-single-engine-authority`
**Owner:** rationale and consequences for RRFlow's single-engine composition
**Decision date:** 2026-09-05

## Context

RRFlow combines persistent and volatile execution, temporal records and graph
relations, lexical and vector retrieval, Arrow/DataFusion analytics, model
routing, security, and multiple client transports. Describing those
capabilities as independent engines or databases creates competing catalogues,
read stamps, authorization paths, and lifecycle state. It also makes an AI or
operator infer how data moves between names instead of following one explicit
authority.

The platform needs named components because their physical responsibilities
differ, but those names must not imply independent semantic ownership.

## Decision

RRFlow is one product and reasoning-data engine. RRD is its embedded and daemon
runtime. `RrdEngine` is the sole composition root, semantic coordinator,
security boundary, and mutation authority.

The accepted component names and meanings are:

- **rrflowDB** is one persistent project AI engram;
- **rrflowKV** is the WAL/MVCC/LSM physical persistence engine beneath
  rrflowDB;
- **rrflowMX** is the non-durable process-local implementation of the same
  semantic storage port;
- **rrflowQL** owns query syntax, binding, planning, and execution;
- the **Arrow substrate** carries eligible columnar storage and analytical
  batches;
- **DataFusion execution** performs bounded computation but owns no state or
  authorization;
- the **RRFlow vector subsystem** provides vector-database functionality within
  rrflowDB without becoming another database authority;
- **RRFlow inference** hosts replaceable embedding and LFG adapters that may
  propose but never commit; and
- HTTP, WebSocket, SDK, MCP, CLI, and Connectome remain clients of public RRD
  capabilities.

The rrflowKV memtable is the hot mutable tier of persistent rrflowDB. rrflowMX
is a separate volatile profile and cannot silently become durable. Promotion
or import requires an explicit authorized `RrdEngine` transaction.

Temporal graph, memory, BM25, scalar, exact-vector, HNSW, and quantization
capabilities remain native data families and indexes within rrflowDB. They do
not create `GraphDB`, `MemoryDB`, `VectorDB`, or DataFusion-owned authorities.

## Consequences

- Every public operation authenticates, authorizes, plans, reads, and mutates
  through `RrdEngine`.
- rrflowMX and rrflowKV must pass the same semantic conformance corpus where
  durability is not part of the operation.
- DataFusion and LFG return computed results or proposals to `RrdEngine`; they
  cannot write physical storage directly.
- A persistent read binds native access paths, Arrow batches, DataFusion work,
  fusion, and returned evidence to one `ReadStamp`.
- Package names use `rrd-*` for internal daemon/runtime boundaries and
  `rrflow-*` for outward product adapters; product documentation uses the
  component names frozen above.
- External databases remain explicit governed integrations and never become
  implicit rrflowDB persistence.
- Architecture, reference, roadmap, POA&M, and generated knowledge-package
  records retain distinct ownership.

## Rejected alternatives

- Independent graph, vector, memory, query, and lifecycle databases were
  rejected because they create multiple semantic authorities.
- DataFusion-owned persistence was rejected because an analytical executor
  does not own RRFlow transactions, authorization, or WAL recovery.
- Silent rrflowMX-to-rrflowDB promotion was rejected because volatile state
  cannot acquire durability without an explicit authorized commit.
- Provider- or editor-owned lifecycle hooks were rejected because clients
  cannot become RRFlow's reasoning or mutation authority. That rejection
  covers hooks that own lifecycle, state, or authority;
  [ADR-0004](0004-host-hooks-as-alias-stubs.md) permits a hook that is an alias
  stub onto one RRFlow owner.

## Links

- [Master system overview](../architecture/system-overview.md)
- [Detailed engine data flow](../architecture/engine-data-flow.md)
- [RRFlow 1.0 roadmap](../roadmap/rrflow-1.0.md)
- [RRFlow 1.0 alpha POA&M](../poam/rrflow-1.0-alpha.md)
