# POAM-015 — public-contract discovery consistency

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-015`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Public-contract discovery is not self-consistent: the frozen sample capability advertises
`POST /v1/backups/create` while the catalogue, server, and Rust client use
`POST /v1/backups`; runtime capability limitations cite retired `F5`, `G06`, and `G09`
labels; and successful claim/data scope plus field/metric vector-address alternatives
remain. The installed MCP fixture also exposed a wire-identity defect in
`ContextPacket`: a packet containing a valid finite `f64` source score can validate before
serialization, then deserialize from its own JSON bytes to a neighboring binary float and
fail its packet digest. The current digest therefore is not a stable cross-language JSON
identity for every valid packet.

A post-D-01 baseline check also found a partial generated-surface refresh: the Rust
contract, public reference, and TypeScript endpoints carried OpenAPI digest `0ef644d8…`,
while four endpoint projections and all five signal projections still carried
`8f9efc7b…`. Package A07-2b regenerated those replaceable views from the unchanged
contract and replaced five handwritten OpenAPI expectations with comparisons between
the generated endpoint and signal identities. The shared parity checker and focused
language tests now detect that split; this bounded repair does not resolve the other
contract deficiencies above or complete H-04's one-compiler target.

## Impact

Generated artifacts can pass while a sample or runtime description directs a client to a
nonexistent route or implies a superseded plan, and alternate pre-release branches can
survive as accidental compatibility promises. A valid context response can also be
rejected after transport or produce a different digest in another language even when no
semantic field was intentionally changed.

## Owning gates

A-07, C-03, H-04, J-01

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Trace every affected fixture, type, server/client path, and test; derive available surface
bindings from the executable catalogue; remove retired labels and superseded successful
branches directly; then pass negative old-shape tests plus contract/router/OpenAPI/client
cross-surface parity. Define one versioned, canonical wire normalization for finite scores
instead of rounding at call sites or weakening digest checks, publish cross-language golden
vectors including neighboring-float cases, and require encode/decode/digest parity in every
supported SDK before closing the context portion of this item.
