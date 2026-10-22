# C-07e projected reader admission and source release

**Status:** active local regression evidence; exact-candidate native CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-read-view-release`
**Owner:** OBJ-02 prerequisite, [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Brief and baseline

[PR 62](https://github.com/rrflow/rrflow-development/pull/62) binds the
preimplementation scope. Its clean isolated worktree starts from development
main `cae8f3d9e3e0f733720a2f60ff5a8a3ab3094bf3`, tree
`50c979b1bd13c72a0610d49a7db12cc11914dd70`. The
[baseline seed](c-07e-read-view-baseline.md) is
`a59cff6eadb9f1070ab8522aa7d9b8836a8e8167`; seed CI `39799044813` was
canceled and supplies no acceptance. The branch is independent of PR 61's
installed reopen fixture; main must be reconciled before landing.

Current-to-target behavior: overlapping empty readers must release their own
admission slots without poisoning the registry, and terminal streams must
release captured generations and temporary buffers before unregistering their
pins. Returned copied Arrow batches must remain readable. No format, public
schema, dependency, default configuration, version or engine authority changes.
Stop on absent reproduction, premature live-buffer invalidation, unknown owner,
changed semantics or materially larger scope. This is a prerequisite to safe
installed maintenance and hard admission, not either feature's completion.

## Reproduction and repair

Four public API tests failed before the repair. Completing/canceling one of
two empty views returned `PoisonedReadViewRegistry`; dropping it leaked its
slot so the next view observed three owners against a limit of two. Releasing
a populated view while a zero-byte generation remained live also poisoned
the registry. `ReadViewLease::release` wrongly required zero bytes to imply
zero references. It now requires no remaining charged bytes only when the
last reference disappears; checked subtraction and inconsistent-last-owner
denial remain intact.

Three independent weak-owner tests also failed before repair: completed,
canceled and budget-failed streams retained their memtable while unregistering
their lease. `ProjectedReadStream::finish` now drops pending rows, heads,
heap allocations and source cursors before releasing the pin. It retains its
bounded evidence and terminal outcome. Already emitted batches contain copied
buffers and own their lifetime independently. This does not invent a borrowed
DataFusion output path or claim retained output admission accounting exists.

The public corpus repeats admission denial/reuse 32 times for each of
completion, cancellation and drop while another empty reader stays live. Its
mixed-generation case also verifies independent expected data after reopen.
The ownership matrix covers completion/cancel/budget failure across memtable,
raw mmap, raw bounded and adaptive-LZ4 paths. Weak references observe actual
source and loaded-page lifetime, separately from registry counters. Physical
path counters must confirm the requested path; fallback is disabled. Retained
cloned output remains exact after database/stream disposal and successful
removal of the disposable database directory. Repeated terminal calls retain
the original outcome.

Review strengthened the page oracle from two to three source rows so a loaded
spine remains live at observation. Segment cases require three actual live
page owners before release; empty weak-reference lists cannot pass that check.
The strengthened matrix passed again. This was a test improvement, not a new
runtime repair or a substitute for native RSS measurement.

## Traceability and engineering checklist

| Existing behavior/source | Characterization | Canonical destination and replacement proof |
|---|---|---|
| `segment/reader.rs::ReadViewLease::release` | New public empty/mixed view cases fail before repair; existing hybrid admission/cancel tests retain ordinary nonempty behavior. | Same registry and method; four public cases pass and the full native storage suite remains green. |
| `segment/reader.rs::ProjectedReadStream::finish` | New weak-owner completion/cancel/failure tests fail before repair. | Same stream/method; twelve physical-path/outcome combinations release actual owners and preserve copied batches. |

No source was moved or deleted, and no parallel fallback or coordinator was
added. Source edits are the reader and its test-module declaration. New tests
are `src/segment/reader_lifetime_tests.rs` and `tests/projected_lifetime.rs`.
Workflow/policy changes require both fixture commands in existing Linux
topology and Windows/macOS estate jobs. Separate steps ensure a first native
command failure cannot be hidden by a later command's success. Supporting
changes are CI operations, C-07, POAM-035, this journal/baseline and generated
navigation/inventory.

Read in full: `reader.rs`, new test files, crate manifest, README, AGENTS and
change-authoring procedure; relevant database capture/options/GC, segment page
ownership and hybrid-reader characterization supplied context. The existing
objective/C-07/POAM/CI owners were reviewed. Changed files and complete diff
were reread before commit. The PR records the baseline, exact scope, first
oracle, acceptance commands, traceability and stop decisions.

Research/source adaptation: no new external API or upstream algorithm; existing
Rust ownership and RRFlow contracts suffice. Trace/debug: preserve existing
runtime trace, use test-local weak owners and registry checks; no new public
diagnostic state. Resource decision: release actual sources before their pin
and retain independent output. Weak ownership does not measure aggregate RSS,
allocator retention or caller-held output budgets.

## Commands and results

Local Windows x86_64, Rust 1.99.1, unchanged debug profile:

- `cargo test -p rrd-lsm --test projected_lifetime --locked`: four reproduced
  failures before repair, then four passes in 0.02 seconds.
- `cargo test -p rrd-lsm --lib reader::lifetime_tests --locked`: three reproduced
  failures before repair, then three passes; strengthened final page-owner
  oracle, including explicit directory cleanup, passes all twelve combinations
  in 0.06 seconds.
- `cargo test -p rrd-lsm --locked`: 138 passed across 20 test/doc-test result
  groups, no failures, one ignored owned-child fixture invoked explicitly by
  native crash tests. Linux-only memory fixtures execute zero cases here;
  their platform evidence is not inferred. This run preceded the final
  three-row page-oracle strengthening, which was rerun separately afterward.
- `cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all -- --check` and workflow policy: passed.

Local logs are `target/c07e-empty-red.log`, `target/c07e-owners-red.log`,
`target/c07e-empty-green.log`, `target/c07e-owners-final.log`,
`target/c07e-lsm-suite.log` and `target/c07e-clippy.log` in the parent workspace.
They are test evidence, never a runtime dependency.
`python -X utf8 scripts/ci/check_documentation.py`,
`python -X utf8 scripts/knowledge/render_navigation.py --check`,
`python -X utf8 scripts/ci/build_execution_inventory.py --check` and
`git diff --check` passed. The generated inventory contains 1,387 path records
before reconciliation with PR 61. No unrelated changes were present.

Not run locally: Linux/macOS native fixtures, whole workspace/SDK corpus,
aggregate native memory, real exhaustion, constrained capacity and 24-hour
soak. Exact-candidate CI supplies native regressions and existing product
partitions; it cannot supply the latter unimplemented qualifications. PR 62
records exact candidate, CI and merge once available. POAM-035 records the
verified deficiencies and bounded remediation; no C-07/alpha closure changes.
