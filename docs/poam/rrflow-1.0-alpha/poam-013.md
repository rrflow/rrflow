# POAM-013 — self-contained signed native release

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-013`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

A-07.1h rejects source/build inputs outside the repository, but a self-contained release
assembler, signed byte manifest, offline installer proof, and clean-rollout comparison do
not exist. The current release build emits a Linux ELF `rrflow` plus separate `rrd-server`
and `rrflow-mcp`; no native Windows `rrflow.exe` was built or run, no single artifact owns
the default lifecycle, and no platform candidate carries complete checksum, SBOM,
provenance, signature, runtime-dependency, or installed-byte evidence.

At planning baseline `9bcc6154`, `Cargo.toml` declares `MIT OR Apache-2.0`,
but the inspected tracked tree contains no LICENSE/NOTICE-named files.
The declared licence expression is not the complete licence/notice material
required for a self-contained official distribution. J-05 must verify the
actual first-party texts, upstream notices and retained source inputs alongside
the [independent database release criteria](../../reference/protocol/postgresql.md#qualification-and-first-official-release).
No licence selection or change is authorized by this observation.

## Impact

The checkout source graph is guarded, but RRFlow can still omit required assets, depend on
an undeclared helper/fetch/toolchain/service, or claim cross-platform delivery from a
component build that cannot install and operate an estate.

## Owning gates

A-07, D-01, D-06, D-11, J-03 through J-05

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Retain repository-closure checks; ship one primary `rrflow`/`rrflow.exe` per native target;
account for every bundled/embedded/installed byte and runtime dependency in the signed
manifest; publish checksums, SBOM, provenance attestations, and native platform signatures;
and pass network-denied clean-machine
install/serve/ready/commit/reopen/verify/repair/restore/uninstall plus pinned
SurrealDB/Qdrant rollout evidence before any ease claim.
