# C-07b I/O fault baseline

**Status:** active characterization evidence; implementation qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07b-io-baseline`
**Owner:** OBJ-02 prerequisite, [C-07b](../../../roadmap/rrflow-1.0/gate-c-07.md)

Public development main was fetched at
`fca1a980245a9ff8634fa70fe21ee79a27eb2047`, tree
`7678086381f4a9112d4dcf0d7acdb2463b16e494`. The worktree was clean.
Branch `agent/c07b-storage-io-faults` starts there. The PR brief was written
before this baseline seed; as in C-07a, this factual evidence commit enables
opening the PR before implementation.

`cargo test -p rrd-lsm --test wal --test failure_matrix --test manifest
--test compaction` passed 30 tests on Windows. These existing fixtures cover
framing, corruption, logical milestones and reachability, not failed native
I/O or modeled loss of volatile bytes.

Inspection followed README, AGENTS, OBJ-02, the C-07 plan/dependency spine,
POAM-035, authoring/navigation and CI owners. WAL, filesystem/native filesystem,
error/lib, WAL and failure-matrix tests were read; database open/write/flush/
compaction/GC, manifest publication and segment output were inspected.
Standalone WAL sync does not currently poison on failure, and publication can
return an error after rename before adopting the new in-memory root. These
are regression targets, not yet reproduced loss claims.

The implementation brief binds private unit-test I/O interception, bounded
stable/volatile byte and directory models, independently requested values,
and actual owner repairs demonstrated by fault tests. Existing native anchored
authority, formats and public APIs remain. SQLite anomaly testing and PostgreSQL
18 reliability were revisited as primary research; no upstream source is
adapted. Bounded test traces suffice; benchmarking and production telemetry
changes are not needed.

The final linked journal must retain complete changed-file review, commands,
failures, omissions, checklist and exact-candidate PR/CI evidence. No roadmap
or POA&M status changes. Process death, real capacity failure, power loss and
sustained scale are not claimed by this baseline or the planned I/O model.
