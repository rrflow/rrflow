# POAM-019 — native engram records and engine transactions

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-019`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

`rrd-engram` publicly depends on `rrd-store` through `EngramRepository`, `Reconciler`, and
`BackupReconciler`; every engram mutation decodes and rewrites one JSON `EngramDocument` at
`server/state/engram/{engram}/document`, and the control journal copies that full
replacement. The one-MiB value cap can preempt the advertised aggregate cardinalities. The
document also nests both `ManagedInstance`/`EngramOperation` and a twelve-kind
`EngramAuthorityState` hierarchy with overlapping instance, job, health, secret,
desired/observed, receipt, and idempotency concepts.

## Impact

Engram code can bypass the engine transaction; write and read cost grow with unrelated
state; state is not natively addressable as graph/scalar/index families;
audit/outbox/runtime-log/index changes are not atomic with the engram replacement; and
overlapping operational models can become a second topology, job, secret, or authorization
authority.

## Owning gates

A-07, C-01 through C-04, C-06, C-07, D-01, D-02, E-01, E-02, F-01, F-05, H-04, H-05, J-01
through J-05

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate E](../../roadmap/rrflow-1.0/gate-e.md),
[Gate F](../../roadmap/rrflow-1.0/gate-f.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Make `rrd-engram` pure domain validation; reconcile every broad authority resource in the
instance-topology package; remove direct storage/repository constructors and the monolithic
key without a wrapper; persist typed engram/instance/operation/lease/receipt/backup/recovery
records plus relations and indexes through one stamped engine transaction; prove MX/KV
semantics, effect-gap crash/reopen, bounded native reads, write/copy/decode/allocation
accounting, streamed same-stamp DataFusion analytics, cross-surface results, and
old-key/shape rejection.
