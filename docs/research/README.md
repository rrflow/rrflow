# Platform industry research index

**Status:** active platform-industry research index; linked source records remain partially unclassified
**Coordinate:** `rrflow://rrflow-instance/data/research-index/platform-industry`
**Owner:** primary-source external-system research discovery; not RRFlow terminology or delivery planning
**Reviewed:** 2026-10-24
**Scope:** `docs/research/`

This directory is the single entry point for industry research used to shape
the RRFlow platform. This index lists source systems and repository research;
it is not a second terminology list or an implementation plan. Canonical RRFlow
terms are defined only in the repository [`README.md`](../../README.md).

## File naming

Use lowercase kebab-case and one of these forms:

- `<subject>-capability-inventory.md` for a source-system inventory;
- `<subject>-architecture-research.md` for bounded technical research; and
- `<subject>-comparison.md` for source-backed comparative analysis that makes
  no benchmark or acceptance claim.

Accepted decisions belong in `docs/decisions/`, measured artifacts and their
exact reproduction metadata belong in `docs/evidence/`, and delivery plans
belong in `docs/roadmap/`.

This file is the active research index. New platform-industry research must be
linked here. Existing flat documents remain at their current paths until each
receives a separate full review and link-preserving classification.

## External reference systems

| Subject | Terms to inventory | Primary sources | Repository note |
|---|---|---|---|
| Qdrant | collection, point, vector, payload, segment, HNSW, quantization, shard, replica, tenant, strict mode | [`v1.19.1` release](https://github.com/qdrant/qdrant/releases/tag/v1.19.1), [pinned source](https://github.com/qdrant/qdrant/tree/6ab21cac18ebb6f4ae29102c7f8f5cc11affd5de), [collections](https://qdrant.tech/documentation/manage-data/collections/), [hybrid queries](https://qdrant.tech/documentation/search/hybrid-queries/) | [`qdrant-capability-inventory.md`](qdrant-capability-inventory.md) |
| HelixDB | labeled property graph, node, edge, label, property, index, query, workspace, project, cluster | [introduction](https://docs.helix-db.com/database/helix-db/start-here/introduction), [data model](https://docs.helix-db.com/database/helix-db/core-concepts/data-model), [run modes](https://docs.helix-db.com/database/helix-db/start-here/run-modes), [repository](https://github.com/HelixDB/helix-db) | No peer capability inventory exists yet. |
| SurrealDB | query/document transaction pipeline, ordered keys, temporal graph, indexes, live/changefeed, permissions, RPC/MCP, installation | [`v3.2.4` release](https://surrealdb.com/releases/3.2), [pinned source](https://github.com/surrealdb/surrealdb/tree/93ab219d69f09d8f999851b0359c80ebe6726102), [architecture](https://surrealdb.com/docs/learn/data-models/architecture) | [`surrealdb-capability-inventory.md`](surrealdb-capability-inventory.md) |

### Owner-requested reference families

By the repository owner's direction of 2026-10-24, RRFlow also folds in the
useful behavior of the AI memory, columnar/vector and graph-retrieval systems
below. The current references above remain in scope. Each subject needs a
`<subject>-capability-inventory.md` with a pinned source revision before any
adaptation. That inventory must follow the source-adaptation rule in
[`AGENTS.md`](../../AGENTS.md), mapping each useful behavior to one canonical
RRFlow boundary and gate rather than copying a foreign model. The releases
listed were the latest stable ones on 2026-10-24.

| Family | Subject | Terms to inventory | Primary sources | Repository note |
|---|---|---|---|---|
| AI memory | Mem0 | memory extraction, add/update/delete decisions, user/agent/run scopes, vector and graph memory, memory history | [`v2.2.1` release](https://github.com/mem0ai/mem0/releases/tag/v2.2.1), [repository](https://github.com/mem0ai/mem0) | No capability inventory exists yet. |
| AI memory | Graphiti (Zep) | bi-temporal knowledge graph, episodes, entity and edge extraction, edge invalidation, communities, hybrid search | [`v0.31.2` release](https://github.com/getzep/graphiti/releases/tag/v0.31.2), [repository](https://github.com/getzep/graphiti) | No capability inventory exists yet. |
| AI memory | Letta | stateful agents, memory blocks, core/archival/recall memory, context-window management, sleep-time agents | [`0.16.8` release](https://github.com/letta-ai/letta/releases/tag/0.16.8), [repository](https://github.com/letta-ai/letta) | No capability inventory exists yet. |
| Columnar/vector | Lance and LanceDB | columnar file format, fragments, versioned datasets, vector and full-text indexes, random access, schema evolution | [Lance `v13.0.0`](https://github.com/lancedb/lance/releases/tag/v13.0.0), [LanceDB `v0.40.0`](https://github.com/lancedb/lancedb/releases/tag/v0.40.0) | Already named as an upstream reference in `AGENTS.md`; no capability inventory exists yet. |
| Columnar/vector | DuckDB | vectorized execution, morsel-driven parallelism, Arrow and Parquet exchange, out-of-core operators, extensions | [`v2.0.0` release](https://github.com/duckdb/duckdb/releases/tag/v2.0.0), [repository](https://github.com/duckdb/duckdb) | The [DuckDB interoperability contract](../reference/data/duckdb.md) owns exchange; no capability inventory exists yet. |
| Columnar/vector | pgvector | vector, halfvec, sparsevec and bit types, HNSW and IVFFlat indexes, iterative index scans, filtered search | [`v0.8.7` tag](https://github.com/pgvector/pgvector/tree/v0.8.7), [repository](https://github.com/pgvector/pgvector) | Informs the [PostgreSQL compatibility contract](../reference/protocol/postgresql.md); no capability inventory exists yet. |
| Graph retrieval | Microsoft GraphRAG | entity and relationship extraction, community detection, community reports, global/local/DRIFT search, indexing cost | [`v3.3.0` release](https://github.com/microsoft/graphrag/releases/tag/v3.3.0), [repository](https://github.com/microsoft/graphrag) | No capability inventory exists yet. |
| Graph retrieval | Cognee | extract-cognify-load pipeline, knowledge graph plus vector store, ontology grounding | [`v1.6.5` release](https://github.com/topoteretes/cognee/releases/tag/v1.6.5), [repository](https://github.com/topoteretes/cognee) | No capability inventory exists yet. |
| Graph retrieval | LightRAG | dual-level local/global retrieval, graph plus vector indexing, incremental graph update | [`v1.5.7` release](https://github.com/HKUDS/LightRAG/releases/tag/v1.5.7), [repository](https://github.com/HKUDS/LightRAG) | No capability inventory exists yet. |

## Cross-system lifecycle research

- [`rrflow-canonical-estate-configuration-and-runtime-controls.md`](rrflow-canonical-estate-configuration-and-runtime-controls.md)
  maps primary-source configuration, canonicalization, filesystem creation,
  Arrow/DataFusion, OpenTelemetry, OpenAPI, and release-trust guidance to the
  project-local estate, runtime ceilings, generated discovery, and staged
  rollout. It distinguishes the implemented D-01 slice from later legacy
  convergence, governed reconfiguration, reasoning, Connectome, and release
  qualification.
- [`rrflow-build-debug-trace-optimize-architecture-research.md`](rrflow-build-debug-trace-optimize-architecture-research.md)
  provides primary-source implementation guidance for reproducible builds,
  release-equivalent diagnostics, causal signal projections, deterministic
  failure analysis, native profiling, benchmark validity, and optimization.
  It does not change engine semantics, gate order, or acceptance.
- [`rrflow-installation-repair-and-distribution-research.md`](rrflow-installation-repair-and-distribution-research.md)
  compares primary database and release-tool lifecycle guidance with the
  current checkout and records the evidence behind RRFlow's single-executable,
  plan/apply, verification, repair, restore, salvage, and artifact target.
- [`rrflowkv-rust-storage-engine-architecture-research.md`](rrflowkv-rust-storage-engine-architecture-research.md)
  is the draft, source-backed qualification study for a Rust-native rrflowKV,
  its hybrid ordered/Arrow immutable layout, the DataFusion boundary, and the
  later governed discussion-to-configuration flow. It does not make an ADR,
  benchmark, completion, or roadmap-status claim.
- [`rrflowkv-page-cache-and-mixed-workload-research.md`](rrflowkv-page-cache-and-mixed-workload-research.md)
  is the bounded primary-source study and completed handoff for C-06j. It
  distinguishes rrflowKV's exact-byte immutable-page cache from future
  DataFusion query caches, records the real-reader scope-aware scan-resistance
  result, and rejects Moka, TinyLFU, family quota, caller bypass, and value
  separation for the accepted format. It is not architecture or completion
  authority.

The pinned SurrealDB reference explicitly rejects its namespace → database →
table/record hierarchy as RRFlow authority. `estate` is an RRFlow-owned term;
the accepted project ↔ estate ↔ instance topology remains defined only by the
[RRFlow topology owner](../architecture/instance-topology.md).

## RRFlow vocabulary reference

The research above informed the one standardized list in
the repository [`README.md`](../../README.md). Research notes may compare foreign
terms, but they must link to that list instead of restating or redefining RRD
terminology.

## Existing RRFlow research and supporting notes

The following reviewed history, supporting references, and unclassified flat
records participate in this research trail; listing them here does not transfer
their authority into this index:

- [`rrflow-system-convergence-architecture-research.md`](rrflow-system-convergence-architecture-research.md)
  (active primary-source basis for the 1.0 execution map)
- [`rrd-data-services-architecture-research.md`](../history/rrd-data-services-architecture-research.md) (historical)
- [`estate-control.md`](../reference/operations/estate-control.md)
  (active implementation reference)
- [`local-estate-authorization.md`](../reference/security/local-estate-authorization.md)
  (active supporting security reference)
- [`instance-topology.md`](../architecture/instance-topology.md)
  (active accepted RRFlow architecture; research records cannot redefine it)

## Historical research follow-ups

These captured follow-ups are not the current roadmap; the executable gate
order lives in [`docs/roadmap/rrflow-1.0.md`](../roadmap/rrflow-1.0.md).

- Add a HelixDB capability inventory at
  `docs/research/helixdb-capability-inventory.md`.
- Complete the code audit against the canonical platform list across contracts,
  engine, storage, services, SDKs, tests, and documentation.
- Classify and migrate each conflicting use without retaining aliases or a
  second glossary.
- Recheck time-sensitive implementation guidance when its owning execution
  gate begins; the current researched implementation map is linked from the
  [RRFlow 1.0 roadmap](../roadmap/rrflow-1.0.md).
