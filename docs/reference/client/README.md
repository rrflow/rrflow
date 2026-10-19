# RRFlow client reference

**Status:** active stable-reference index
**Coordinate:** `rrflow://rrflow-instance/data/reference-index/client`
**Owner:** operator and developer client behavior that is not owned by the public protocol or an SDK

The [RRFlow reference index](../README.md) is the parent of this directory.
Client records define how a user-facing application consumes public RRD
operations without acquiring engine, persistence, query, reasoning, or
automation authority. Wire shapes remain owned by the
[public contract](../protocol/public-contract.md), language bindings by the
[SDK reference](../sdk/README.md), and delivery status by the
[roadmap](../../roadmap/rrflow-1.0.md).

| Client | Durable warp | Checkout record | State |
|---|---|---|---|
| Connectome operator/developer workbench | [`rrflow://rrflow-instance/data/reference/client/connectome`](rrflow://rrflow-instance/data/reference/client/connectome) | [`connectome.md`](connectome.md) | active target contract; separate implementation is partial and non-conforming |
| DevPulse local engine and optional shippin.cloud account | [`rrflow://rrflow-instance/data/reference/client/devpulse-cloud`](rrflow://rrflow-instance/data/reference/client/devpulse-cloud) | [`devpulse-cloud.md`](devpulse-cloud.md) | local native database integration exists; account synchronization is not implemented |
