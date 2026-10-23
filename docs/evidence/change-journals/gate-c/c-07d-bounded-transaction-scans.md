# C-07d bounded transaction range reads

**Status:** active implemented candidate; native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-bounded-transaction-scans`
**Owner:** OBJ-02, [C-07d/e](../../../roadmap/rrflow-1.0/gate-c-07.md), POAM-035

[PR 73](https://github.com/rrflow/rrflow-development/pull/73) carries the
pre-implementation brief. Starting revision is
`69e415526fa9d5efc2a17e3d4111c1bc8906d435`; dependency correction
`52706d6ba38afc4919ca0606c007610ffeff2f64` was incorporated before this package
commit. PR #72 must be accepted first. Version remains 1.0.0.

## Change brief and traceability

`Transaction::scan(start,end,limit)` previously called `Database::scan`,
materialized the full range in a second ordered map, overlaid all pending
mutations and only then applied the limit. Thus a one-row request could
allocate and read the entire matching range. Its canonical replacement is
`transaction/scan.rs`, invoked by that same public method: a bounded projected
reader merges the transaction's captured snapshot with borrowed ordered
pending writes. It preserves insertion, replacement, tombstones, lexical
range order and snapshot isolation, and stops at the requested visible count.
No historical implementation/fallback remains in Transaction::scan.

The existing C-06 projected reader owns source work, leases and output
reservations. Old batches drop before requesting another batch. Error and
normal paths release the stream; no timeout invalidates a live buffer. Range
bounds are validated against native key size before cloning; empty lower
bounds remain admissible. Result payload plus Vec representations are checked
against the configured projected-output ceiling before row copies, with
bounded amortized capacity growth. Returned Vecs remain caller-owned and do
not retain a global output-pool reservation; allocator metadata and aggregate
caller-held results are not claimed as process-RSS proof. Work-budget failures
are explicit errors rather than partial successful results.

Exact changed implementation: native `transaction.rs` scan delegation/imports
and new `transaction/scan.rs`. New characterization: `tests/transaction_scan.rs`.
No WAL/segment format, semantic schema, SDK operation, installation policy,
isolation level, or provider dependency changed. Existing full-range Database
scan APIs remain outside this bounded-transaction package; their callers are
not silently relabeled streaming.

## Grounded reads and decisions

Read in full: Transaction::scan/get/pending-prefix and owner validation;
Database::scan/scan_each/scan_ranges and begin_projected_read; ProjectedReadRange,
ProjectedReadBudget validation, ProjectedReadStream::next_batch and terminal
release; OutputRows/BaseRows and the complete new scan helper and regression
file; existing point/range overlay and transaction-pin compaction oracles.
Read the owning C-07 allocation/retention requirements, linked authoring
checklist/template, and existing journal/index conventions. The source map
above is package-specific, not another product/planning authority.

Research decision: reuse the accepted C-06 physical projection and ordered
merge contract; no upstream code/algorithm/dependency imported. Existing page
load and owner/output counters are sufficient debug evidence; no payload logs
or runtime lifecycle were added. Stop on snapshot drift, an omitted tombstone,
a hidden full-range copy, invalid owner release, or failed native CI.

## Oracles and authoring results

The first native oracle independently specifies a three-row historical result
through later updates, deletion, flush and compaction, then requires fewer
than 128 page loads from a 1,024-row one-row-group-per-row cold-cache fixture.
It checks source/output release, conflict preservation and exact latest values
after reopen. A second oracle compares multiple requested limits, including
usize::MAX, against independently constructed final ordered state with mixed
pending shadows. A third rejects an oversized pending result before copying,
keeps the original transaction bytes/read intact, proves source-owner release,
and then succeeds after a smaller replacement. Existing wrong-owner, empty
and reversed-range tests remain unchanged. Runtime results are pending.

`cargo clippy -p rrd-lsm -p rrd-store -p rrd-engine --all-targets --locked -- -D warnings`
passed in this worktree (initial 19.56s, focused definitions 0.41s, final range
admission refinement 4.81s). `cargo fmt --all` completed. No local runtime
suite, workload, native-control effect, disk fill or soak was launched, per
owner direction to prioritize implementation. Candidate CI must execute the
native transaction suite and owning kernel/storage/installed regressions.

Full helper reread and diff review confirmed no eager full-range map, no
clone of the entire pending write set, checked payload/capacity growth and
ordinary owner-drop behavior. Exploratory searches for guessed cache filenames
returned missing paths; located the actual PageCacheStats owner in segment/mod.rs.
No compilation failure occurred in this package. Superseding dependency
failures remain recorded in C-07g, not counted as this package's acceptance.

## Change checklist and remaining work

- [x] Bind one alpha outcome and owning C-07 allocation/retention package.
- [x] Post current/target behavior, exact scope, oracles and stop conditions in PR before implementation.
- [x] Map the replaced behavior and retain independent characterization.
- [x] Reuse canonical native types and read/resource authority.
- [x] Record research, telemetry, dependency and failure decisions.
- [x] Compile/lint affected targets and review full new helper/diff.
- [ ] Accept exact-candidate runtime CI; no local runtime acceptance claimed.

Navigation and inventory regeneration completed. Documentation policy first rejected
the missing active/historical qualifier in this record; corrected to active and
reran the policy before publication. `git diff --check` passed.
C-07, OBJ-02 and POAM-035 remain open. This package does not establish installed
retained-reader interference, fleet isolation capacity or a 24-hour soak.
No merge or release evidence exists yet.
