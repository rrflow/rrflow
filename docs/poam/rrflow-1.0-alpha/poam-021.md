# POAM-021 — local process-control authority

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-021`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Local RRD process control is split between `rrd-engram::LocalProcessDriver`, a standalone
deployment-catalogue generator, static direct-store reconciliation plus
`rrd-engram-controller`. The second `rrflow-cli` development supervisor and
`rrd-server initialize` are removed; generated launch catalogues now have no preparation
command and start the internal server with an installed project plus explicit distribution
executable. The remaining controller path still creates private JSON process/catalogue
state, accepts arbitrary physical roots and caller time/configuration, treats file
existence as readiness/shutdown completion, and does not bind the authenticated digest to
the exact executed image. Plaintext environment values and unbounded diagnostic logs/tails
also cross this boundary.

## Impact

Process launch no longer runs a second product installer, but the remaining controller can
still become a storage and lifecycle authority; a replaced executable or forged/stale
marker can be trusted; restart recovery can depend on private files instead of engine
state; secrets can leak; and diagnostics can exhaust the host. Passing PID-reuse and
effect-gap tests can falsely qualify this topology.

## Owning gates

A-07, C-02, C-03, D-01, D-02, H-04, H-05, J-01 through J-03, J-05

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Make `rrd-engram` pure; implement one outward local-process adapter consuming only immutable
installed, fenced effect plans; bind artifact digest to the executed image; use
authenticated challenge-bound readiness/control receipts; commit canonical
intent/observations/receipts through `RrdEngine`; remove both supervisors,
catalogue/controller binaries, process JSON, marker authority, initializer, arbitrary-path
entry points, and successful prior shapes; then pass MX/KV semantic, every effect/crash gap,
path/image replacement, PID reuse, forged receipt, timeout, disk/resource/secret,
public-surface, clean offline install, and final Linux/Windows/macOS corpora.
