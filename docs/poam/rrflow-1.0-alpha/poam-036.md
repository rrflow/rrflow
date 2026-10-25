# POAM-036 — DuckDB analytical interoperability is absent

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-036`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

## Observed deficiency

At `ecfeea332e6d7a0138b2dbb6c393e0d734d50db9`, repository search outside the
optional Connectome checkout found no DuckDB integration. The query package
pins DataFusion/Arrow/Parquet and the engine exports bounded authenticated
Parquet results, but there is no qualified real DuckDB consumer, source/import
adapter, optional bundled execution or native containment/resource corpus.
The owner requested DuckDB support; format availability alone cannot satisfy it.

## Impact

AI clients and Connectome cannot honestly advertise DuckDB integration.
Directly adding a SQL connection would leave authorization, consistency,
native-code isolation, aggregate memory and offline delivery unresolved.
The [contract](../../reference/data/duckdb.md) defines distinct export, source
and execution profiles while preserving native rrflowDB authority.

## Owning gates

[D-06's ordered DuckDB packages](../../roadmap/rrflow-1.0/gate-d.md#duckdb-interoperability-packages)
own implementation after C-07; F owns streaming/budgets, H owns client parity,
and J owns signed distribution. The parent row is sequenced, not a discovered
native-storage defect.

Roadmap owners: [Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate F](../../roadmap/rrflow-1.0/gate-f.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md) and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Close only after actual pinned-version interoperability,
governed import/restart, native worker containment/resource and offline delivery
evidence passes for the declared profiles. Unsupported profiles remain explicit.
