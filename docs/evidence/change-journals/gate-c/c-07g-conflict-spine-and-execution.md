# C-07g: conflict-probe read reduction and execution reset

**Status:** active implementation evidence; Desktop qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-conflict-spine-and-execution`
**Owner:** OBJ-02 / C-07g / POAM-035 / PR115

Baseline `170af0d0b1a7b2f579496606d5321811e5e36231` merges the existing PR111
failed-campaign evidence into current public main `66464e35`; its engine source
is identical to main. PR115's scoped brief preceded implementation edits.
The user's execution review identified fragmentation: native fixture failures
were receiving attention without independently resolving engine-scale failure;
the roadmap still described implemented drivers and bounds as future work.
The canonical C-07 chapter now distinguishes implementation inventory from
missing acceptance and names native qualification and engine growth as the two
blocking tracks. It retains the full workload, native matrix and 24-hour gate.
No new roadmap or status authority is created. Existing historical journals
and failed artifacts remain unchanged.

## Source-grounded repair

`Database::latest_sequence_for_key` used complete `SegmentVersion` reads for
conflict validation and continued searching immutable segments even when the
active memtable had the authoritative newer version. This loaded/copy-allocated
old value pages that the caller discarded. The existing Database::get invariant
already gives the active memtable precedence, including tombstones.

Reuse a single Segment::select_version helper for the existing authenticated
key/sequence selection. Segment::get_version still loads its selected value;
private Segment::get_sequence returns the selected sequence without value pages.
The database conflict probe returns an active-memtable version immediately or
selects the newest immutable sequence. Tombstone presence is retained, never
confused with key absence. No format, WAL/sync, conflict/snapshot rule, native
error handling, limits, maintenance scheduling, public API or external source
changes. Whole-file/open validation remains intact; consumed pages still use
the existing checksum/read path. This reduces a demonstrated cost; it is not
attribution of all 1.98TB of failed-campaign traffic.

Traceability: database.rs::latest_sequence_for_key remains the conflict owner;
segment/mod.rs::get_version shares its unchanged row selection with the new
sequence-only accessor. transaction_conformance.rs exercises actual native
commit rejection, retained state, fresh retry and reopen. Read the complete
affected functions, transaction commit/point-get ordering, segment point/record
cursor/page loading, compaction/maintenance selection, native admission and
existing transaction conformance fixtures. Owner review included README,
OBJ02, C07, POAM034/035, change-authoring and retained Desktop failure. Also
inspected installed corpus/diagnostic/smoke and acceptance-collection paths.
Changed paths are these three Rust files, the C-07 chapter, this journal,
POAM035 link and generated navigation/inventory. No implementation is removed
without its shared canonical replacement. No upstream research or adaptation
is needed: the existing native format/order/transaction contract defines it.
Existing physical counters provide the measurement; no new trace protocol.

## First failure and narrow result

Before repair, `cargo test --locked -p rrd-lsm --test transaction_conformance conflict_validation_reads_only -- --nocapture`
failed: the hot-memory winner case read six immutable pages / 65,583 bytes,
where no immutable read was needed. After repair the same command passed all
16 internal cases: bounded and mapped I/O, raw/adaptive-LZ4, active/immutable
winner and value/tombstone. Active winners read zero immutable pages/bytes;
immutable winners read six key/sequence pages totaling 60 raw or 54 compressed
bytes. Every case rejects the stale write at the exact winning sequence,
preserves domain state/releases transaction reservations, accepts a fresh retry
and reopens with the accepted value. The cache is disabled for this measurement.
The test runs in 0.89 seconds; this is a physical-work regression, not a latency
benchmark or proof of Desktop/soak performance.

The roadmap reset preserves all required gates: finish native qualification
and measured engine growth repairs, use bounded growth attribution, freeze one
integrated candidate, complete unchanged full corpus/readback/lifecycle/load,
run profile/repetition and actual soak, then collect exact acceptance evidence.
New unrelated UI/provider/configuration work must not displace these blockers.
No objective/roadmap checkbox or POAM closure is claimed.

## Owning validation

`cargo test --locked -p rrd-lsm --jobs 6` passed 213 tests with zero failures.
Two explicit subprocess helper entries remain ignored at top level and are
invoked by their parent tests. Native fault schedules, 100 kill/reopen cycles,
compaction, transaction, cache, segment and lifetime suites ran on Windows.
Linux-only fixtures executing zero cases here are not cross-platform evidence.
Log: operator-local `target/c07-conflict-lsm-suite.log`.


Documentation policy passed (496 statuses / 494 classified coordinates),
navigation passed (17 indexes / 495 nodes / 2067 edges), inventory passed
(1586 records) and staged whitespace passed. Generated roadmap navigation
retains the corrected status; no handwritten parallel inventory was added.

`cargo clippy --locked -p rrd-lsm --all-targets --jobs 6 -- -D warnings`
passed (14.87 seconds). The installed ordinary-binary check
`cargo test --locked -p rrflow-cli --test installed_reopen --jobs 6 -- sustained::installed_capacity_workload_smoke --exact --ignored --nocapture`
passed (75.38 seconds, debug build). It exercised eight records, 24 edges,
16 vectors, insert/update/retire/recreate transitions, a retained reader during
write/backup, actual process termination/reopen and 12 replayed transactions.
Retained projected bytes drained from 225 to zero. The receipt explicitly sets
capacity_qualification=false; this is integration coverage, not a scale result.
Ordinary primary SHA-256:
`e1d4f3259632d70bc0e1b697bc4e10b43794458e9549b38dbe18d33b923c65bd`.
Operator-local log: `target/c07-conflict-installed-smoke.log`.

All changed functions/tests and authored diff were reread. No surfaced failure
was hidden: the initial regression failed and the corrected oracle passed.
Remaining work is native candidate capacity, bounded growth attribution and
unchanged Desktop/profile/24-hour qualification. Those were not rerun or
claimed by this source package. C-07/POAM035 remain open. The pre-merge PR brief
also authorizes joining exact PR112 as this candidate's capacity prerequisite;
its separate integration record will bind that merge and final candidate.
