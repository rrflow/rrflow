# POAM-018 — accepted complete engram and backup shapes

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-018`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

C-05b removed the successful incomplete engram and backup-job shapes. `EngramDocument` now
requires authority and all eight backup/recovery maps; internal and public backup jobs
require the bound recovery-policy snapshot; the absence branch, legacy comment, and
successful missing-field fixtures are gone.

## Impact

The specific multiple-shape risk is removed. POAM-019 separately tracks conversion of the
still-monolithic engram aggregate and direct repository into native engine records.

## Owning gates

C-05, J-01, J-02

Roadmap owners: [Gate C](../../roadmap/rrflow-1.0/gate-c.md) and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

The C-05b execution record traces every removed default and repeated public shape; negative
tests reject omission of all nine engram fields and both policy snapshots, a fresh
engram/backup/recovery corpus is identical on rrflowMX and rrflowKV after rrflowKV reopen,
and the complete default workspace test and Clippy matrices pass.
