# POAM-029 — diagnostic snapshot under concurrent audited traffic

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-029`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

`RrdEngine::read_diagnostic_snapshot`
(`crates/authority/rrd-engine/src/engine/diagnostic.rs`) assembles under a
before/after stamp that includes the control-journal sequence, and retries up
to `MAX_DIAGNOSTIC_ASSEMBLY_ATTEMPTS` (3) times. Every audited request, reads
included, appends to the control journal. One audited request inside an
attempt's window therefore discards that attempt although no runtime state
changed. When every window sees one, the read answers 409 `StorageConflict`,
marked retryable.

The engine test
`an_audited_read_inside_every_assembly_window_exhausts_the_diagnostic_read`
reproduces this deterministically through a test-only assembly observer. Alone,
the read assembles on its first attempt. With one change-feed read inside each
window, all three attempts fail while the runtime cursor does not move.

## Impact

Under steady authenticated traffic the diagnostic read is unavailable. On the
Connectome demonstration estate, the Storage lens could not read for the whole
47 s similarity-map build, whose 270 vector searches are audited reads. A
client can only retry, and the read cannot report the load that prevents it.

## Owning gates

H-05 (diagnostic evidence); the H-06 client contract consumes the read.

Roadmap owner: [Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

The observed read in the engine test assembles while an audited read runs
inside every window, and every section still verifies against one stamp (for
example, by assembling from one storage snapshot, or by reading the audit
section through the captured control sequence). The test is inverted, and the
live read completes during a vector-map build on an installed estate.
