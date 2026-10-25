# C-07 planning baseline

**Status:** active measured component characterization; C-07 remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07-planning-baseline`
**Owner:** OBJ-02 prerequisite and C-07 implementation planning

Baseline revision `b6052feef8df2aa86e5a866ff0c8da196bc8f2ab`, tree
`64e50acd608bc9a400808a25e9fad4d4894205da`. Public development main was fetched
and matched HEAD; the worktree was clean. The user requested a thorough C-07
plan, including scale, UI operation, dogfooding and downstream integrations.
No runtime implementation changes occur in this package.

## Characterization

`cargo test -p rrd-lsm --test maintenance --test failure_matrix --test compaction`
passed on this Windows checkout: maintenance 3, failure matrix 4, compaction 8;
zero failures or ignored cases. `cargo test -p rrd-lsm` then exited 0,
including the independent projected-read model, WAL, transaction, manifest,
segment, inspection and cache targets. `snapshot_memory.rs` is Linux-only;
its target executed zero tests here. This is not native RSS qualification.

Full fixture reads: `crates/persistence/rrd-lsm/tests/maintenance.rs`,
`failure_matrix.rs`, `snapshot_memory.rs`, `projected_read_adversarial.rs`.
Targeted production reads: `database.rs` policy structs, `prepare_write`,
`flush_memtable_inner`, `compact_if_needed`, `compact_inner`, candidate
selection and `garbage_collect`; `wal.rs::append_inner` and `sync`;
`manifest.rs::publish`, checkpoint and publication helpers;
`segment/reader.rs::ActiveReadViews` and `ReadViewLease`.
Compaction/manifest test names and the evaluation runner's `Trial`/dispatch
were inspected; those larger source files were not read in full.

The matrix's Crash/StorageFull variants return `Error::InjectedFailure` at
declared boundaries and reopen after dropping the database. This is useful
atomicity evidence, not actual disk exhaustion or loss of OS/device caches.
Maintenance thresholds permit one oversized atomic batch. Automatic compaction
retains history for untracked copyable snapshots. The plan must distinguish
these behaviors from hard aggregate resource caps and bounded retention.
No new acknowledged-write-loss defect was reproduced.

## Planning boundary and procedure

The local change brief was written before this seed. GitHub needs an ahead
commit to create a PR; as in D-01, this minimal factual evidence seed enables
the PR description before canonical roadmap/POA&M edits. The requested plan
is the deliverable, not an extra planning-only lifecycle checkpoint. This
journal and generated navigation/inventory are the seed's only changed paths.
The PR will bind the chapter, ledger and final journal scope; no code is moved,
deleted, copied from upstream or replaced.

Reviewed documents include the current product/status entry points, alpha
objectives, Gate C, dependency spine, change-authoring procedure, implementation
navigation, POA&M ledger/governance and 002/025/026/033/034, knowledge-connectors,
Gate E, and J-04's measurement/capacity sections. Primary-source research informs
fault-model, resource, connector and quantization decisions; the final plan
will retain exact links. The HydraDB documentation-index web fetch failed;
available introduction/architecture sources do not establish an admitted API.
Initial path/glob discovery misses were resolved with `rg --files`; they are
not source defects. No runtime trace or service/model call was needed.

Checklist: baseline and owners inspected; existing code/tests characterized;
scope and failure oracle bound in the brief; research separated from proof;
local targeted and owning suites passed; platform omissions explicit; generated
navigation/inventory required before commit. Canonical gate status does not
change. Native disk-full/power-loss, installed growth/soak, connector, model,
UI, competitor and signed-release qualification were not run. The smallest
plan oracle is complete requirement-to-package-to-test coverage; candidate CI
will gate the final documentation revision before landing.

The first documentation check rejected this journal's status header because
it omitted the required active/historical classification. The header now says
active; navigation and inventory were regenerated before repeating the check.
