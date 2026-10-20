# RRFlow protocol reference

**Status:** active protocol-reference index
**Coordinate:** `rrflow://rrflow-instance/data/reference-index/protocol`
**Owner:** public RRD transport and protocol-contract discovery; linked from `docs/reference/README.md`

These records describe RRD's implemented and target public transport boundaries;
each record identifies its maturity. They do not own engine semantics, release
status, or client-specific behavior.

## Protocol records

<!-- rrflow:generated-index:start -->
- [rrflowDB PostgreSQL compatibility and independent database contract](postgresql.md) — [`rrflow://rrflow-instance/data/reference/protocol/postgresql`](rrflow://rrflow-instance/data/reference/protocol/postgresql) — active target contract; implementation and qualification remain open
- [RRD public contract](public-contract.md) — [`rrflow://rrflow-instance/data/reference/protocol/public-contract`](rrflow://rrflow-instance/data/reference/protocol/public-contract) — active implementation reference; alpha protocol convergence remains incomplete
- [RRD HTTP and WebSocket server](server.md) — [`rrflow://rrflow-instance/data/reference/protocol/server`](rrflow://rrflow-instance/data/reference/protocol/server) — active implementation reference; alpha protocol convergence remains incomplete
- [RRFlow signal catalogue](signal-catalogue.md) — [`rrflow://rrflow-instance/data/reference/protocol/signal-catalogue`](rrflow://rrflow-instance/data/reference/protocol/signal-catalogue) — active generated signal-contract reference; H-05 runtime instrumentation remains incomplete
- [RRD durable subscriptions](subscriptions.md) — [`rrflow://rrflow-instance/data/reference/protocol/subscriptions`](rrflow://rrflow-instance/data/reference/protocol/subscriptions) — active implementation reference; B-04 multiplexing is implemented and commit-impact delivery remains incomplete
<!-- rrflow:generated-index:end -->
