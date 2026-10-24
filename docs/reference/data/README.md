# RRFlow data reference

**Status:** active stable-reference index
**Coordinate:** `rrflow://rrflow-instance/data/reference-index/data`
**Owner:** logical model, schema, temporal value, and multi-model contract discovery; linked from `docs/reference/README.md`

These records define canonical logical data semantics shared by rrflowMX and
rrflowKV. They do not claim that current physical storage or query access is
native, incremental, or complete. The
[engine data-flow architecture](../../architecture/engine-data-flow.md) owns
the target substrate, and roadmap Gates C and E own its delivery evidence.

| Subject | Durable warp | Checkout record | State |
|---|---|---|---|
| Logical schema catalogue | [`rrflow://rrflow-instance/data/reference/data/schema-catalogue`](rrflow://rrflow-instance/data/reference/data/schema-catalogue) | [`schema-catalogue.md`](schema-catalogue.md) | implemented logical validation and persistence; pre-1.0 derivation and replay paths remain to be removed |
| Multi-model commits and immutable objects | [`rrflow://rrflow-instance/data/reference/data/multi-model-object-contract`](rrflow://rrflow-instance/data/reference/data/multi-model-object-contract) | [`multi-model-object-contract.md`](multi-model-object-contract.md) | implemented semantic transaction and object-reference visibility; native Arrow-compatible storage and indexes remain open |
| rrflowDB capability: prompt graph families, memory, project links | [`rrflow://rrflow-instance/data/reference/data/rrflowdb-capability`](rrflow://rrflow-instance/data/reference/data/rrflowdb-capability) | [`rrflowdb-capability.md`](rrflowdb-capability.md) | target contract; native traversal implemented, families and links planned (H-09, H-10); PostgreSQL compatibility is owned by the PostgreSQL contract (H-08) |
| Temporal reads and forward compensation | [`rrflow://rrflow-instance/data/reference/data/time-travel-and-rollback`](rrflow://rrflow-instance/data/reference/data/time-travel-and-rollback) | [`time-travel-and-rollback.md`](time-travel-and-rollback.md) | historical reads are tested; forward compensation is plan-only and unqualified |
| Knowledge-tree connectors and Parquet exchange | [`rrflow://rrflow-instance/data/reference/data/knowledge-connectors`](rrflow://rrflow-instance/data/reference/data/knowledge-connectors) | [`knowledge-connectors.md`](knowledge-connectors.md) | target discovery, provenance, chunking and vendor adapter contract; execution unimplemented |
| DuckDB analytical interoperability | [`rrflow://rrflow-instance/data/reference/data/duckdb`](rrflow://rrflow-instance/data/reference/data/duckdb) | [`duckdb.md`](duckdb.md) | target export, source/import and optional execution profiles; unimplemented and unqualified |
