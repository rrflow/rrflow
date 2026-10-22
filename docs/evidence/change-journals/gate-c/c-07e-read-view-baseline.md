# C-07e projected reader ownership baseline

**Status:** active inspection baseline; hypotheses require failing oracles
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-read-view-baseline`
**Owner:** OBJ-02 prerequisite, [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md)

The clean isolated worktree starts at development main
`cae8f3d9e3e0f733720a2f60ff5a8a3ab3094bf3`, tree
`50c979b1bd13c72a0610d49a7db12cc11914dd70`. It shares no runtime or source
dependency with another checkout. GitHub requires a differing commit before
PR creation; this record supplies that seed before implementation.

Inspection identified two possible ownership defects at the canonical projected
reader: `ReadViewLease::release` equates zero pinned bytes with zero references,
although empty live views can pin zero bytes; `ProjectedReadStream::finish`
unregisters its lease while retaining cursors and loaded/pending buffers until
the caller drops the terminal stream. Neither hypothesis is called reproduced
by this baseline. Public admission/release tests and independent weak-owner
checks must fail before a repair is selected.

The bounded package must preserve copied Arrow output after cancellation,
completion or failure, while releasing no-longer-needed source owners. Existing
reader, registry, format, transaction authority and configuration remain the
canonical boundaries. No new public maintenance operation is selected. This is
a prerequisite to safe installed maintenance and C-07d/e, not completion of
hard aggregate limits, native RSS, capacity, disk exhaustion or soak.

The preimplementation PR brief binds exact files/symbols, characterization,
native commands, traceability, research/resource decisions and stop conditions.
The successor journal will retain red/green evidence and actual command results.
No runtime change, verified new deficiency or gate-status change occurs here.
