# POAM-050 — a task's attempts can be read only by an id the reader already holds

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-050`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Found by source review while checking what Connectome's restored Runs view can
read. `task-attempt-read` returns one attempt for a known `scope`, `task_id` and
`attempt_id`, and `RrdEngine::read_task_attempt`
(`crates/authority/rrd-engine/src/engine/reasoning_budget/public.rs`) serves it
with a point read of a control-store key. No public operation lists a task's
attempts. Because attempts live in the control store rather than the runtime
graph, rrflowQL cannot traverse to them as it can to a task's flights through
`rrflow-task-includes`. A task's budget exposes only `active_attempt`.

This is a source finding. No client has been run against it.

## Impact

A client that did not create an attempt cannot discover it, so it cannot show
what a task tried, when, or how each try ended. Connectome's Runs view can show
only a task's goal and outcome until this is served. An operator investigating a
task after a restart has the same limit.

## Owning gates

G-02a (durable tree and dispatch identity) and H-05 (the causal chain includes
model and tool attempts). The related record is [POAM-032](poam-032.md).

Roadmap owners: [Gate G](../../roadmap/rrflow-1.0/gate-g.md) and
[Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

A public, authorized, paged read returns a task's attempts in order with their
status and times, at a stated read position and within a stated budget. A test
through the real engine creates attempts, reopens the engram, and lists the same
attempts with no id supplied by the caller. Reading never re-executes an effect.
