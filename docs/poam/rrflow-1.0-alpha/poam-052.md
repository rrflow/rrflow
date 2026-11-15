# POAM-052 — context ranking cannot be configured or experimented with

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-052`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

The repository owner directed on 2026-11-03 that Connectome's search, which calls
`context-assemble`, be "configurable to improve and experiment". `AssembleContext`
(`crates/transport/rrd-contract/src/memory_context.rs`) denies unknown fields and
takes only `scope`, `query`, `valid_at`, `seeds`, `max_graph_depth`, `max_items`,
`max_output_bytes` and `max_storage_keys`. The engine ranks by reciprocal-rank
fusion of seed anchors, BM25 text matches, exact vector scores and graph
expansion. A caller cannot turn a source on or off or change how the sources are
weighed, and the engram's `[recall]` configuration holds limits only.

This is a source finding. No ranking experiment has been run.

## Impact

An operator can change how much context is gathered, but not how it is ranked.
Tuning recall for a project, or comparing two rankings on the same query, needs a
source change and a new build.

## Owning gates

H-05 (selected and skipped context avenues) and OBJ-11 (each avenue's contribution
to retrieval). The [adaptive-reasoning decision](../../decisions/0002-adaptive-reasoning-governed-effects.md)
bounds the design: a control shapes one request or one sealed configuration, and
does not fix how an AI reasons.

Roadmap owner: [Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

A public, versioned, bounded control selects the context sources and their
weighing for one request, and the result states the control it ran under. Default
requests keep today's ranking, proved by a test. A test through the real engine
shows that the same query at the same read stamp under two controls returns
results whose differences the evidence explains. A measured improvement is a
separate claim that needs a labelled evaluation set.
