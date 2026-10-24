# C-07c ordinary installed 100-cycle reopen corpus

**Status:** active local qualification evidence; exact-candidate native CI pending; C-07 remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07c-installed-reopen-corpus`
**Owner:** OBJ-02 prerequisite, [C-07c](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Brief and baseline

[PR 61](https://github.com/rrflow/rrflow-development/pull/61) starts from
`cae8f3d9e3e0f733720a2f60ff5a8a3ab3094bf3`, tree
`50c979b1bd13c72a0610d49a7db12cc11914dd70`. The clean worktree and fetched
development main matched. The [baseline](c-07c-installed-reopen-baseline.md)
seed is `df513e995819b2425d2e66601ba3bb8cae23e799`. GitHub refused a draft PR
at the unchanged main revision because there was no commit difference; the
baseline seed permitted creation before implementation. Its canceled CI run
`39793275994` is not acceptance.

The preimplementation brief binds one extension: ordinary primary-executable
fresh/existing histories each perform 100 acknowledged kill/reopen cycles.
Existing three-reopen transport and diagnostic internal-boundary histories
remain unchanged. This package changes no runtime behavior, storage format,
configuration defaults, schema, dependency, release version or existing test
deadline. Stop conditions are lost/partial/duplicate effects, changed receipts,
wrong artifacts, incomplete histories, leaked children, missing native evidence
or a materially different repair. No alpha or C-07 completion is inferred.

## Oracle and implementation

`installed_reopen.rs` freezes all 100 two-claim requests and expected effect
digests before the first request. Every new ACK is followed by termination
of the exact owned child and a new ordinary server for the same estate.
Full independent data readback precedes retry, so a retry cannot conceal
missing acknowledged state. Every earlier transaction is retried after every
restart; receipts stay exact after normalizing only `idempotent_replay`.
Another readback proves retries did not advance the runtime cursor.

Each mode requires 100 logical operations, 5,150 acknowledged attempts, 100
resolved-present effects, zero violations and zero unresolved operations.
It binds every kill/reap/restart chain, final cleanup, actual artifact build,
installation-plan digest and effective configuration. Existing project-owned
README content must remain intact. The staged ordinary artifact must contain
no diagnostic rendezvous marker, including across scan-buffer boundaries.
The parent independently hashes the staged executable and compares the engine's
reported executable digest with those actual bytes. CI also checks the emitted
compiler profile, optimization level and disabled debug assertions.

The native candidate lane uses the existing optimized Cargo release profile;
it does not produce a signed distribution or promote anything to the official
repository. The debug corpus is separate characterization. Startup and request
bounds remain 60 seconds, each service child is bounded to 180 seconds, and
each fixed corpus to one hour. Captured streams are at most 1 MiB each;
each complete sanitized receipt is at most 8 MiB. The optional parent-only
receipt directory never enters the product child's cleared environment.

The required three-platform job explicitly runs both long tests, validates
both complete receipts and their process chains, and retains bounded JSON
artifacts. Default-suite ignored cases are not acceptance. Its 90-minute
budget includes the optimized build and fixed corpus; existing job budgets
are untouched. The workflow policy requires the lane and its gate reduction.

## Inspection correction and traceability

The baseline's wording about maintenance stopping at storage was too broad.
Ordinary engine writes already reach automatic flush and compaction through
`Database::prepare_write`. Native garbage collection has only storage wrapper,
test and example callers; installed reclamation remains absent. POAM-035 now
records that exact call-path gap. This package does not select a new public
maintenance endpoint or qualify automatic publication interruption.

Nothing was moved, deleted or replaced. The new fixture reuses the existing
owned-process guard and `rrflow-eval` acknowledgement ledger. Existing native
component and installed transport/internal-boundary suites remain the
characterization of their unchanged paths. No upstream source or algorithm
was adapted; repository contracts and implementation inspection were sufficient.
Trace/debug additions are sanitized qualification receipts only. Harness bounds
are not engine memory limits, filesystem capacity measurements or power-loss
evidence.

Changed files read in full and reviewed: the new installed test,
`.github/workflows/ci-reusable.yml`, `scripts/ci/check_workflow.py`, and the
POAM-035 detail. The existing `tests/support/process.rs`, `installed_crash.rs`,
`support/storage_history.rs` and CLI manifest were read in full. Relevant
ordinary lifecycle, engine installation/transaction/backup, native maintenance,
snapshot/checkpoint and projected-reader paths supplied context. README,
AGENTS, alpha objectives, C-07, CI operations, agent bootstrap and the
change-authoring procedure supplied the owners and rules.

Created implementation path: `crates/adapters/rrflow-cli/tests/installed_reopen.rs`.
Changed implementation paths: workflow and workflow-policy script. Supporting
changes: CI operations, C-07 owning link, POAM-035, this journal and baseline,
generated journal navigation and execution inventory. Full changed-file reread
and complete diff review precede candidate commit.

## Commands and results

Local platform: Windows x86_64, Rust 1.99.1. The focused command
`cargo test -p rrflow-cli --test installed_reopen --release --locked fresh_installed_100_acknowledged_reopens -- --ignored --nocapture`
passed one test in 113.31 seconds. The owning target command
`cargo test -p rrflow-cli --test installed_reopen --release --locked -- --ignored --nocapture --test-threads=2`
then passed both tests in 116.04 seconds: 200 forced reopens, 10,300
acknowledged attempts, 200 resolved logical effects, no violations or
unresolved operations. This is elapsed qualification time, not a throughput
benchmark. Both modes used ordinary executable SHA-256
`d742e2aa5701fb0f60c8976547fe27a6b278f385641d4acdf2d4c78835f1df05`.

The final command used parent-only `RRFLOW_REOPEN_RECEIPT_DIR` set to
`target/c07c-reopen-release-final` (absolute when passed to the test).
Fresh receipt: 1,669,105 bytes, SHA-256
`0b93e40371c4aea58aaf64f12f9b469dd04488bce16e6c4ba30a43fdb646045a`.
Existing receipt: 1,669,091 bytes, SHA-256
`309b156bb9f5ecf6327d02894ee99294fa2f3dbb6cf63fd0edf0f03ec9641937`.
The actual inline candidate collector accepted both files. A local extracted
collector rehearsal (`python -X utf8 target/c07c-check-reopen-receipts.py`)
also rejected six separately mutated fixtures: missing estate, missing cycle,
changed attempt count, debug artifact, broken process chain and wrong cleanup
PID. An initial local extraction used the wrong heredoc indentation and failed
with `IndentationError`; delimiter-aware extraction fixed the rehearsal. The
workflow's collector did not require a code change.

`cargo clippy -p rrflow-cli --test installed_reopen --release --locked -- -D warnings`
passed in 57.50 seconds. `cargo fmt --all -- --check`,
`python -X utf8 scripts/ci/check_workflow.py`,
`python -X utf8 scripts/ci/check_documentation.py`,
`python -X utf8 scripts/knowledge/render_navigation.py --check`,
`python -X utf8 scripts/ci/build_execution_inventory.py --check` and
`git diff --check` passed. Navigation regeneration updated the nearest journal
index; the inventory contains 1,386 path records. Changed-file reread and full
diff review found no runtime/dependency changes. Exact-candidate CI remains
the cross-platform acceptance boundary.

The first focused debug 100-cycle run uses an earlier harness revision and
is separate characterization. At this record's preparation it was still
running, so it contributes no completed acceptance claim. Its log is retained
at `target/c07c-reopen-focused.log`; subsequent completion can be recorded in
the PR without rewriting this immutable journal.

Initial strict Clippy rejected an assertion on a compile-time constant.
The redundant assertion was removed; actual artifact marker exclusion and
the CI default-feature policy still enforce the ordinary-binary boundary.
Strict default-profile Clippy then passed. Initial workflow policy rejected
incorrect indentation in the new gate dependency; correcting indentation made
the policy pass with nine substantive jobs. No runtime deadline, cycle count
or correctness oracle was weakened to address either failure.

The owner's joined-flow clarification is recorded in C-07's existing handoff,
linked to the knowledge and LFG/SWAG contracts. It requires source changes,
authorization changes, retries/recovery and declared effort settings to be
tested across components; it neither closes those downstream gates nor expands
this package into implementing them. No measured-hours requirement or private
thought schema was introduced.

## Remaining work and acceptance boundary

This small fixed corpus is not a constrained operating point, J-04 benchmark,
or 24-hour soak. Installed publication/reclamation interruption, hard resource
admission, retained-buffer accounting/lifetime, genuine exhaustion and the
capacity profile sweep remain required. The whole-scope validation cost keeps
its separate E-01/E-02 owner. No roadmap, alpha objective or POAM closure changes.

Exact-candidate native CI and the PR merge remain required. The PR records
candidate revision, workflow result and merge identity once those exist.
The full workspace, native Linux/macOS lanes, genuine exhaustion, constrained
capacity and 24-hour soak were not run locally. Candidate CI runs the existing
workspace/client/lifecycle partitions and all three new native lanes; it does
not supply the still-unimplemented exhaustion/capacity/soak evidence.
