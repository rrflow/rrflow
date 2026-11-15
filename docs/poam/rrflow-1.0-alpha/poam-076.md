# POAM-076 — the MCP context tool advertises limits the engine refuses

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-076`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail is governed by the parent ledger; it cannot independently change
priority or closure status.

## Observed deficiency

A code read at `de435238` on 2026-11-11 found that the MCP context tool's input schema is hand-written (`crates/adapters/rrflow-mcp/src/main.rs:145-151`). It allows the following:

| Field | MCP schema allows | Shipped sealed ceiling |
|---|---|---|
| `max_graph_depth` | 32 | 4 |
| `max_items` | 512 | 128 |
| `max_output_bytes` | 786432 | 524288 |

The ceilings come from `crates/authority/rrd-engine/assets/install/default-engram-configuration-v3.toml:50-52`. The schema is not generated from the operation catalogue.

## Impact

An AI client can send values the schema accepts and the engine then refuses. This is a second, drifting owner of one contract, contrary to the single-owner projection rule in `AGENTS.md`.

## Owning gates

H-04 owns serving MCP through the same catalogue-derived operations.

Roadmap owner: [Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

Closed when the MCP schema is generated from the catalogue, reflects the installed ceilings, and a test fails if they drift.
