# C-07e native retained output admission

**Status:** active bounded implementation evidence; exact candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07e-output-budget`
**Owner:** OBJ-02 prerequisite, [C-07d/e](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Baseline and change brief

[PR 66](https://github.com/rrflow/rrflow-development/pull/66) carries the
preimplementation brief. Its [baseline](c-07e-output-budget-baseline.md) starts
at accepted `dde6ef86d278b8c59cf1f61ebc2d3923186396d5`, tree
`bc76b62d5a500395fb8e1193dcd4d94eeac79ada`. The baseline seed is
`c5888e2373ba03155774361e8105639dcf709363`; seed CI 39834399126 was
intentionally canceled and supplies no acceptance. PR 64/65 must be reconciled
after acceptance before this candidate is qualified.

Previously only each batch and each stream's cumulative output were bounded.
The native reader now reserves the full output allocation before constructing
its buffers. `DatabaseOptions::projected_output_bytes` supplies one finite
per-open-instance pool (default 128 MiB); zero is rejected before create.
Each output allocation owns its reservation through immutable Arrow clones
and slices. Allocation destruction precedes charge release. Complete, cancel,
failure and drop release source pins without releasing still-retained output.
`ProjectedOutputMemory` is a read-only diagnostics handle exposing coherent
limit/current/peak/denial observations, including after database close.

The default is an unqualified native safeguard, not a measured capacity profile.
This charge excludes independently reopened instances, caller-created copies,
source pages, merge temporaries, allocator metadata, cache, memtables and RSS.
No installed configuration or public UI field is claimed. No format, version,
dependency, semantic authority, scheduler or adapter changes are included.
Custom immutable buffers cannot be converted in place to mutable allocations;
the batch API documents this. Repository consumer search found buffer getters
used by native fixtures, with no downstream `into_mutable` dependency.

## Source and adaptation review

Runtime files read/reviewed in full: native `database.rs`, `lib.rs`,
`segment/reader.rs`, `segment/mod.rs`, new `segment/output.rs`, and the new
`tests/projected_output_limits.rs`. Reviewed owner records: README/AGENTS,
change-authoring, C-07/POAM-035 and CI operations. Reviewed supporting symbols:
existing lifetime fixtures, compaction/GC tests, native `borrow_mmap`, projected
buffer accessors and required workflow/policy partitions. No implementation is
moved or deleted. Exact changed paths are in Git and the generated inventory.

The [baseline research record](c-07e-output-budget-baseline.md) links primary
Arrow/DataFusion documentation and distinguishes latest docs from locked
Arrow 59.2.0 source. Reviewed pinned allocation, bytes/deallocation and mutable/
immutable buffer code: exact-capacity zeroed allocation; custom-owner lifetime,
clone/slice behavior and conversion rejection. Arrow's optional post-allocation
claim API is not the admission boundary. No feature/dependency is added.
The narrow unsafe construction retains the original immutable Arrow Buffer;
its pointer, initialized extent, alignment and drop order are documented.
Native tests exercise ownership through actual values and independent capacity
expectations, including zero-length slices that still retain their allocation.

## Failure oracle and executed evidence

All commands ran in the isolated Windows worktree with
`CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target/c07e-output-build`.
Redirected local logs are under the main checkout's ignored `target/` with
prefix `c07e-output-`; CI remains the portable retained acceptance record.

| Command | Actual result |
|---|---|
| `cargo test -p rrd-lsm --test projected_output_limits --locked` with policy surface but no enforcement | 0 passed, 2 failed: second retained 97-byte batch admitted under nominal 97-byte cap; zero cap admitted |
| Same target after enforcement | 2 passed; expanded target subsequently 7 passed, 0 failed |
| `cargo test -p rrd-lsm --lib segment::output::tests --locked` | 1 passed; maximum-integer reservation denial, partial transfer, failed wrap, slice and conversion release |
| `cargo test -p rrd-lsm --locked` | 152 passed, 0 failed, 1 owned child fixture ignored, 22 result groups |
| `cargo test -p rrd-store --locked` | 203 passed, 0 failed, 0 ignored, 35 result groups |
| `cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings` | Passed |
| `cargo clippy -p rrd-store --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all --check`; `python -X utf8 scripts/check_version.py` | Passed; frozen 1.0.0 preserved |
| `python -X utf8 scripts/ci/check_workflow.py`; `python -X utf8 scripts/ci/check_documentation.py` | Passed |
| `python -X utf8 scripts/knowledge/render_navigation.py --check`; `python -X utf8 scripts/ci/build_execution_inventory.py --check`; `git diff --check` | Passed after regeneration; final journal/index regeneration follows this record |

The seven integration tests include 12 cap boundary cases across key-only and
key/value output, empty values and non-aligned lengths; 16 physical/terminal
histories (memtable, raw mmap, bounded raw/LZ4 crossed with complete/cancel/fail/
drop); and 32 rounds of competing streams. Explicit bounded channels and joined
threads retain the winning batch while the second must reject, then reuse the
same capacity after last drop. The partial-construction history exhausts the
page-request budget after reserving/allocating output but before value copy:
no batch escapes, charge drains, and a later valid read succeeds. The maintenance
history proves exact old/new values through overwrite, flush, compaction, actual
source-file reclamation, database close and reopen. It explicitly proves the
old and reopened pools are independent, not a hidden process-global bound.

## Surfaced errors, review and remaining work

The first enforcement compile failed because a new export was accidentally
placed in the existing `physical-policy-lab` gated block. Moving the export to
the ordinary native export list fixed it; no feature was enabled to hide it.
Diff review caught a Windows default text-decoding change to an existing arrow
character in a comment; explicit UTF-8 restored the original bytes. Several
exploratory guessed paths/globs failed; discovery and correct paths followed.
These are retained authoring errors, not passing runtime evidence.

Changed runtime/test files and the complete behavioral diff were reread. Native
ownership invariants, source cleanup, immutable conversion behavior and checked
admission were reviewed against pinned Arrow source. Existing traces remain;
pool diagnostics describe reservations rather than claiming RSS. Explicit
focused integration and arithmetic steps are required in Linux topology and
Windows/macOS estate partitions; workflow policy requires each once.

Full engine/query suites were not run locally for this native-only package:
consumer search found no direct projected-buffer use there, and the affected
store suite passed. Required candidate CI still includes those suites. Linux/
macOS native execution, cross-platform output steps, native memory sampling,
installed configuration, real disk exhaustion, capacity profiles and the
24-hour soak have not run for this candidate. Finite thread schedules are not
an exhaustive concurrency proof. No runtime test failure remains locally.

Checklist: owning outcome/scope/stop conditions bound before implementation;
baseline and primary-source decision recorded; behavioral red precedes repair;
actual passes and failures retained; package and affected consumer suites run;
changed code/diff reviewed; immutable evidence and generated navigation/inventory
recorded; exact accepted-base reconciliation and full candidate CI still required.
POAM-035 records the reproduced missing output bound without closing its broader
gap. C-07, all parent alpha outcomes and the release decision remain open.
