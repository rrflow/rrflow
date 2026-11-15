# RRFlow architecture

**Status:** active architecture index
**Coordinate:** `rrflow://rrflow-instance/data/architecture-index/rrflow-architecture`
**Owner:** detailed architecture discovery; linked from `docs/README.md`

The repository root [README](../../README.md) owns product identity and
non-negotiable invariants. The system overview is the master component and
boundary map; other records explain one bounded architecture subject without
owning delivery order, implementation status, or evidence.

| Subject | Durable warp | Checkout record | State |
|---|---|---|---|
| Master system overview | [`rrflow://rrflow-instance/data/architecture/system-overview`](rrflow://rrflow-instance/data/architecture/system-overview) | [`system-overview.md`](system-overview.md) | accepted terminology and boundaries; implementation incomplete |
| Engine data flow | [`rrflow://rrflow-instance/data/architecture/engine-data-flow`](rrflow://rrflow-instance/data/architecture/engine-data-flow) | [`engine-data-flow.md`](engine-data-flow.md) | accepted target; implementation incomplete |
| Source-bound plans and replanning | [`rrflow://rrflow-instance/data/architecture/source-bound-plan-design`](rrflow://rrflow-instance/data/architecture/source-bound-plan-design) | [`source-bound-plan-design.md`](source-bound-plan-design.md) | proposed design; engine source binding and replan unimplemented |
| One-engine execution and handoff | [`rrflow://rrflow-instance/data/architecture/one-engine-execution-plan`](rrflow://rrflow-instance/data/architecture/one-engine-execution-plan) | [`one-engine-execution-plan.md`](one-engine-execution-plan.md) | historical; superseded by the [kernel contract](../roadmap/kernel-contract.md); its handoff schema stays the one handoff schema (K6) |
| SDK and launch bridges | [`rrflow://rrflow-instance/data/architecture/rroflow-sdk-and-bridge-plan`](rrflow://rrflow-instance/data/architecture/rroflow-sdk-and-bridge-plan) | [`rroflow-sdk-and-bridge-plan.md`](rroflow-sdk-and-bridge-plan.md) | proposed plan; provider collection and native qualification incomplete |
| Scale and contract rationale | [`rrflow://rrflow-instance/data/architecture/engine-scale-and-contracts`](rrflow://rrflow-instance/data/architecture/engine-scale-and-contracts) | [`engine-scale-and-contracts.md`](engine-scale-and-contracts.md) | researched tradeoffs; capacity and implementation unqualified |
| Code structure and conventions | [`rrflow://rrflow-instance/data/architecture/code-structure-and-conventions`](rrflow://rrflow-instance/data/architecture/code-structure-and-conventions) | [`code-structure-and-conventions.md`](code-structure-and-conventions.md) | proposed standard awaiting owner approval; enforced by K12 and K13 |
| Instance topology | [`rrflow://rrflow-instance/data/architecture/instance-topology`](rrflow://rrflow-instance/data/architecture/instance-topology) | [`instance-topology.md`](instance-topology.md) | accepted project/engram/instance and physical topology; implementation incomplete |
| As-built baseline, 2026-11-13 | [`rrflow://rrflow-instance/data/architecture/as-built-baseline`](rrflow://rrflow-instance/data/architecture/as-built-baseline) | [`as-built-baseline.md`](as-built-baseline.md) | evidence of what exists; owner decisions pending |
