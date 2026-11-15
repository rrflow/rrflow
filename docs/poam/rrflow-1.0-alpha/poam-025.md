# POAM-025 — verification, repair, restore, and salvage

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-025`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

RRFlow has narrow WAL recovery, torn-tail truncation, authenticated backup catalogue/restore
algorithms, object quarantine, and projection reset mechanics, but no installed read-only
physical/semantic verifier or engine-owned repair planner. Current database/manifest openers
create directories or locks and runtime open reconciles checkpoints; standalone
backup/recovery controllers accept arbitrary roots; there is no explicit distinction among
startup recovery, verification, repair, restore, and salvage.

## Impact

A diagnostic can mutate the target, a repair can bypass installed identity/authorization, an
arbitrary path can be treated as the engram, derived projection damage can be confused with
canonical corruption, or an unsafe reset can hide acknowledged data loss.

## Owning gates

C-07, D-01, D-11, E-01 through E-05, F-01 through F-05, J-02 through J-05

Roadmap owners: [Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate E](../../roadmap/rrflow-1.0/gate-e.md),
[Gate F](../../roadmap/rrflow-1.0/gate-f.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Implement an offline mutation-free inspector and quick/full verifier; deterministic repair
plan/apply with exact digest, exclusive lease, authenticated snapshot, absent reflink/clone
or fully accounted same-filesystem candidate, permitted-action taxonomy, audit receipt, full
candidate verification, atomic locator replacement, and retained/quarantined prior root;
installed backup/restore commands that publish only an absent verified target; explicitly
non-authoritative salvage; projection rebuilds from canonical source cursors; and negative
proof that complete-frame/canonical corruption, unknown identity, stale plans, live writers,
in-place mutation, or guessed state never repair successfully.
