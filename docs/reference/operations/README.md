# RRFlow semantic operations reference

**Status:** active stable-reference index
**Coordinate:** `rrflow://rrflow-instance/data/reference-index/operations`
**Owner:** engine-owned semantic operation and external-effect reconciliation discovery; linked from `docs/reference/README.md`

This directory documents semantic operations accepted by `RrdEngine`. It does
not define operator runbooks, which remain under `docs/operations/`, and it
does not create a workflow, storage, transport, or lifecycle authority.

| Subject | Durable warp | Checkout record | State |
|---|---|---|---|
| Engram desired/observed control | [`rrflow://rrflow-instance/data/reference/operations/engram-control`](rrflow://rrflow-instance/data/reference/operations/engram-control) | [`engram-control.md`](engram-control.md) | useful reconciliation and recovery foundations exist; direct-store aggregate and controller authorities require convergence |
