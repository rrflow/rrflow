# C-07g: defer projected page reads past the active key range

**Status:** active implementation evidence; native comparison pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-projected-frontier`
**Owner:** OBJ-02, C-07e/g, POAM-035
**Pull request:** [108](https://github.com/rrflow/rrflow-development/pull/108)

Starting revision `f2d0f01551358f5f694b951888d6574b035f9c96`, tree
`8c1289def013f36b2732ef7a6103deff711fe41f`, integrates fetched main with PR107
`8d2755a451c9523072bf1e34a7c95e8ea697c98f`. The draft PR recorded the owning
outcome, exact scope, oracle and stop conditions before repository edits.

## Current and replacement behavior

Every projected page previously opened all matching segment cursors before
returning its first row. The transaction scan used by repository pagination
thus read disjoint future segments even when the caller requested one row.
The focused oracle seeds twelve independent immutable key ranges. Before the
repair the first-row request visited twelve row groups instead of one, while
returning the expected value. This is a measured unnecessary-work defect; it
does not establish the complete cause of the installed backup deadline.

`ProjectedReadStream` retains a bounded heap of unopened run lower bounds from
the same segment descriptors already used for range admission. Before selecting
the smallest loaded key, it opens every remaining run whose bound could precede
or equal that key, reevaluating after each open. Equal-key versions still merge
before winner selection. Empty runs and tombstones continue through this same
merge. Later disjoint segments incur page reads only when needed.

All selected runs and generations remain admitted and pinned at view creation.
Existing page, version, output and retained-buffer ceilings still apply. Each
run enters the unopened heap once and is removed once; its size is bounded by
the existing admitted run count. Cancellation, failure, completion and drop
release this heap with the other cursor state before releasing the view lease.
Unread future page corruption is checked when visited; full traversal retains
the existing checksum validation. No budget, archive verification, timeout,
snapshot format, dependency, SDK or external capability changes.

## Review, tests and decisions

Read README/AGENTS, C-07 plan, POAM-035 and the authoring procedure; prior
native/backup-capture receipts; complete stream constructor, initialization,
merge, batching, finish, run-cursor and segment next-candidate/read-row code;
database read admission, transaction scan, backup snapshot/export/control
capture callers; existing output/lifetime/adversarial fixtures. Reviewed the
complete changed functions, new fixture and diff. Canonical replacement stays
in `rrd-lsm/src/segment/reader.rs`; no parallel reader is added. New tests live
in `rrd-lsm/tests/projected_frontier.rs`. This record, POAM-035 and generated
navigation/inventory carry evidence.

The first test requires one visited group for the first one-row page, all twelve
runs still admitted, exact complete traversal after reopen, cancellation and a
returned value remaining valid after database drop. A second test compares
declared independent expected rows across old/intermediate/current snapshots,
overlapping immutable and mutable versions, a deletion and sparse ranges.
Existing adversarial histories, resource denials, mapped/bounded and raw/LZ4
buffer lifetimes remain in the owning suite.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-lsm --test projected_frontier -- --nocapture` before repair | One expected failure: 12 visited groups versus 1; overlap fixture passed. 9.31 seconds including build. |
| Same focused command after repair | Both tests passed; 4.17 seconds. |
| `cargo test --locked -p rrd-lsm -- --test-threads=4` | 212 passed, 0 failed, 2 existing ignored child helpers; 74.89 seconds. |
| `cargo clippy --locked -p rrd-lsm --all-targets -- -D warnings` | Passed; 3.16 seconds. |

Local raw logs use `target/c07g-frontier-*.log`. Formatting, generated navigation,
inventory, documentation and diff checks passed (482 document statuses,
480 classified coordinates and 1,570 inventory records). No external code
was adapted: local inspection and the falsifying work oracle suffice. Existing
resource evidence supplies the measurements without content/credential logging.
Native throughput, backup timing and qualification cannot follow from these
component passes; an unchanged-campaign comparison remains required.

## Retained diagnostic-repeat failure

Linux19 used the same pre-repair binary as Linux18, source
`245220065260b6e15869233569e453a8988daede`, executable
`26ec0178d5b9944418e2f03991449ab7f26daef4d884adbf81f364027326c63b`.
The operator-only observer watched backup filenames and sizes, never payloads
or credentials. It produced no backup samples because corpus setup first failed
at acknowledged ordinal 608 with `rejected-api-internal` and detail digest
`e038cc5d2672248a6de97918b3ffc401d061b83f9f7e52d78e2f25ecd62e6d64`.
The failure observation showed 15 L0 segments, debt 8, 437,886,976 resident
bytes, and no failed flushes or compactions. The internal cause remains unknown;
this change is not claimed to fix it. The one successful Linux18 setup is not
repeatability proof.

Campaign `843599bcc6ebd9f36e537f4c47cebe2f28fba40ab8b8fae38bf1240ed1d2fed7`
failed with zero sealed workload segments; owned cleanup was confirmed after
94,343 ms. Progress SHA-256:
`8b55f56d91dda0e688d782011b762c42d4421b34aa5d9e07accbf3021357a069`.
Original artifacts and digest manifest remain at operator workspace
`target/c07-linux-scale-evidence-19/`, command log
`target/c07-backup-profile19.log`; bootstrap credentials are excluded.

Checklist: pre-edit PR/baseline and source map, failing oracle, bounded canonical
replacement, corruption/resource/trace decisions, full changed-function and
diff review, exact test results and retained native failures are recorded.
Candidate CI, native comparison, complete profile ladder and 24-hour soak remain
pending; no merge, gate completion, alpha or version change is claimed.
POAM-035 retains both backup timing and the new unclassified setup rejection.
