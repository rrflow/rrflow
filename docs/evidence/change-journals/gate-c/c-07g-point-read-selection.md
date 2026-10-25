# C-07g: select immutable point-read winners before loading values

**Status:** active implementation evidence; candidate CI and growth qualification remain open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-point-read-selection`
**Owner:** OBJ-02 / C-07g / POAM-035 / PR119

## Baseline and change brief

Public main `94d0567fb865fb49a90aaeeef4b5861707c9f8a7` is the starting
revision. Empty opening commit `88f8b139fef9140a3dadec8bd11e9e872a468fe3`
created [PR119](https://github.com/rrflow/rrflow-development/pull/119) with its
scope, oracle, resource decisions and stop conditions before source edits.
PR117 is a separate pending candidate; none of its checks or native receipts
is rebound to this package.

`Database::get` and multi-segment `get_many` loaded each visible immutable
candidate's value before comparing sequence numbers. Large old values were
read, decoded and copied even when a newer small value or tombstone won.
The target is the same snapshot result with only the winner's value loaded.
Memtable priority, first-segment tie precedence, durable formats, authentication,
transaction authority, admission limits and default cache policy stay intact.
Stop on changed results, lost/partial effects, retained page owners, changed
format/authority or an unsubstantiated capacity/soak claim.

## Implementation and traceability

The canonical selection remains `segment/mod.rs::select_version`. Its private
`SelectedVersion` binds an immutable segment reference, row group, row and
sequence. The handle owns no loaded page or payload; its fields cannot be
constructed at a database call site. `get_version` and the existing conflict
`get_sequence` reuse it. `value` still uses authenticated `value_at` loading.

`database.rs::get` selects the highest visible sequence before materializing.
`get_many` resolves memtable values/tombstones first, selects within each
segment, and then materializes winners within each segment. Output indexes
preserve caller order and duplicate keys. This keeps small-cache locality in
both passes. Single-segment and empty-segment batches reuse single-key reads.
The private eager `Segment::get_versions` wrapper and its database call sites
are removed; no external caller used it. Existing MVCC, transaction,
source-page and corruption suites remain characterization coverage.

Per unresolved batch key, temporary state is one index and one borrowed row
identity. The former key-slice vector, sequence vector and eager segment-value
batch are gone. No source-page allocation is retained across selection. The
second metadata-only traversal has the same O(keys * segments) bound as the
existing selection; no background tasks or global state are introduced.

Created source evidence: `crates/persistence/rrd-lsm/tests/point_read_selection.rs`.
Changed runtime: `crates/persistence/rrd-lsm/src/database.rs` and
`crates/persistence/rrd-lsm/src/segment/mod.rs`. Changed records: this journal,
the C-07/POAM-035 evidence links, generated Gate C journal navigation and
execution inventory. No physical fixture, schema, lockfile or version changes.

## Research, reads and diagnostics

This repairs RRFlow's existing MVCC algorithm; no upstream code, dependency,
new research claim or alternate database is introduced. Existing authenticated
page-load/byte counters give the necessary physical oracle. Profiling is not
needed to establish discarded value hydration; it is still needed if later
end-to-end measurements cannot attribute remaining growth cost.

Full reads include the supplied AGENTS instructions, change-authoring procedure,
rrd-lsm Cargo manifest, the complete new test and the final changed functions.
Scoped reads: README bootstrap/owners; alpha objective and C-07/POAM-035
outcomes and evidence; CI operations; database point reads, conflict selection
and compaction contract; segment selection/materialization and page-cache
counters; MVCC snapshot/batch/memtable/tamper tests; transaction-conformance
conflict-spine fixture; page-cache policies and source-page limit histories.
These scoped reads are not claimed as full reads of the large source files.

## Failure oracles and results

On unchanged main code, `cargo test -p rrd-lsm --test point_read_selection
--locked -- --nocapture` failed as intended: a six-byte current value over
two superseded 64-KiB values required 18 page loads and 131219 read bytes.
The required winner-only oracle allows 12 loads and less than 1024 bytes.

The final focused target passes both tests. Across bounded/mmap and
raw/adaptive-LZ4, the exact expected current and historical values, tombstones,
missing and repeated batch keys, resurrection, flush, protected compaction
and reopen all pass. With zero resident cache, raw point reads use 12 loads /
113 bytes for a value and 10 / 91 for a tombstone; adaptive-LZ4 uses 12 / 101
and 10 / 82. Single and batched reads agree with the independent expected
history. Source-page reserved bytes return to zero after each checked call.

The first implementation's per-key batch loop caused 240 loads / 28320 bytes
for 16 keys across four overlapping segments under a 1024-byte cache.
Segment-major selection fixes that to 15 / 1770. A further disjoint-key test
with a 256-byte cache exposed 60 loads / 1568 bytes when final materialization
followed caller order. Grouping that pass by segment fixes it to 24 / 692.
Both surfaced regressions were corrected before acceptance; the assertions
and small caches were not relaxed. These are component I/O measurements,
not a claim about installed throughput or all remaining growth cost.

## Acceptance and remaining work

All commands ran on native Windows against this candidate's source:

- Focused command above: 2 passed, 0 failed, 1.72 seconds after the final source
  change.
- `cargo test -p rrd-lsm --locked`: 215 passed, 0 failed, 2 intentionally ignored
  child-process helpers; 137.641 seconds including build. The suite includes
  native crash/reopen, MVCC, compaction, source-page and retained-output limits.
- `cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings`: passed,
  25.203 seconds.
- `cargo fmt --all -- --check`: passed.
- `cargo test -p rrd-store --test semantic_repository_conformance --test
  unified_data --test runtime --locked`: 13 passed, 0 failed, 30.594 seconds
  including build. These exercise semantic
  records and runtime state above the repaired physical reads, including
  unified evidence, restart/retry and failed-commit rollback.
- `python scripts/ci/check_documentation.py`: passed (503 document statuses,
  501 classified coordinates).
- `python scripts/knowledge/render_navigation.py --check`: passed (502 nodes,
  2091 edges); the owning index was generated with the same script.
- `python scripts/ci/build_execution_inventory.py --check`: passed (1594
  records), after staging new paths and regenerating the inventory.
- `git diff --cached --check`: passed.

Operator-local command output is retained under `target/c07-pr119-evidence/`.

The full installed native/capacity matrix is delegated to exact candidate CI;
no local full installed growth/soak result is claimed by this package. The
backup join's dispatch delay, later operating latency, intermittent startup
failure, frozen Desktop/service capacity profiles and the actual 24-hour soak
remain open under C-07/POAM-035. No objective or roadmap status is advanced.

## Change checklist

- Outcome, baseline, scope and stop conditions bound in PR119 before edits.
- Current behavior and replacement mapped to one native persistence boundary.
- Independent expected history and physical I/O failure measured first.
- Both cache-locality regressions retained and corrected with explicit oracles.
- Research/source-adaptation and resource/diagnostic decisions recorded.
- Changed functions and complete test reread; complete diff reviewed.
- Owning checks and generated projections passed before candidate push.
- Candidate CI and merge receipt remain pending; this is not C-07 completion.
