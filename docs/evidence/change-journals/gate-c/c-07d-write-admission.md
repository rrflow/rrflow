# C-07d encoded write admission and rejected-commit recovery

**Status:** active bounded implementation evidence; candidate CI and merge required
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-write-admission`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Brief and baseline

[PR 63](https://github.com/rrflow/rrflow-development/pull/63) carried the
preimplementation brief. Accepted development main was
`952ec21b03a9e701967f2f15c0a37e3d34051fed`, tree
`12955e6e4d097f59401f5974eadad80e6bec8292`. The minimal PR seed
`cad6ab9f5d2fe70baacb9f2d098e65032c9f5918` contains the immutable
[baseline](c-07d-write-admission-baseline.md) and generated navigation.
Its cancelled seed CI `39813045617` is not acceptance. PR 62's independent
reader repair remains pending at this record; reconcile accepted main before
landing this candidate.

Previously final batch construction enforced the authenticated 16 MiB
ceiling, but native transactions, profile-neutral staged writes and semantic
plans could retain larger maps. The store converted final native rejection
into a generic substrate error. At the engine boundary this appeared as an
internal error and did not select the known-rejection intent cleanup path.

The repair checks exact encoded byte/count charges before changing staged
maps. Replacements subtract their actual prior value charge; tombstones still
charge key/header bytes. Rejection preserves the old map and accounting.
Final batch validation reuses the same format owner and still validates all
encoded operations, including duplicate keys. Native and store typed limit
errors retain resource, limit and observed counts. The engine reports
nonretryable resource exhaustion, clearing prepared intent only through its
existing known-rejection path after checking that no domain outcome exists.
Unknown I/O retains its existing uncertain-outcome behavior.

No serialized format, ceiling, default configuration, product version,
transaction authority or optional provider requirement changed. This bounds
encoded staged effects, not caller inputs, serialization copies, allocator
overhead, aggregate concurrent owners or process RSS. Stop conditions remain
partial effects, unsafe retry classification, changed accepted encoding,
undeclared scope and failed candidate CI.

## Traceability and reviewed files

Complete changed implementation/test files were read and the diff reviewed:

- `crates/persistence/rrd-lsm/src/batch.rs`, `error.rs`, `lib.rs`,
  `transaction.rs`, and `tests/write_admission.rs`.
- `crates/persistence/rrd-store/src/transaction.rs`, `error.rs`,
  `access/semantic_commit.rs`, `tests/write_admission.rs`, and
  `tests/semantic_commit_atomicity.rs`.
- `crates/authority/rrd-engine/src/engine/error.rs`, `transaction.rs`,
  `tests.rs`, and `tests/write_admission.rs`.
- `.github/workflows/ci-reusable.yml`, `scripts/ci/check_workflow.py`,
  C-07 and POAM-035; this record is the new evidence node. Generated journal
  navigation and execution-file inventory are replaceable projections.

README/AGENTS, alpha objective, roadmap/execution-map/change-authoring and CI
owners supplied the contract. Additional inspected definitions included the
native WAL limits; store profile implementations, runtime/claim repositories,
key grammar, direct semantic reads; engine public-to-runtime lowering and
the direct-read source-closure oracle. These supporting inspections do not
claim every unrelated module was read.

| Current behavior/source | Canonical destination and replacement proof |
|---|---|
| Native transaction map and final batch size walk | Same native transaction owner, using `batch::WriteBatchSize`; real encoding boundary, rejection/replacement/rollback/reopen tests. |
| Store transaction map and separate semantic-plan map | Existing `TransactionWriteSet` now owns both staged-map paths; semantic application still moves the complete plan into one transaction. Full physical-row comparisons and existing graph/function-receipt fixtures preserve behavior. |
| Generic substrate admission failure and retained intent | Typed store error and existing engine rejection branch; public semantic rejection, authenticated direct-version comparison, exact prior receipt replay, explicit abort, quota reuse and second reopen. |

No implementation was moved or deleted and no alternate staging/publication
authority was added. Native transactions cannot depend on the higher store
crate; both boundaries use the same format-owned accountant. All first-party
source remains in the repository.

Research/source adaptation: none needed for this bounded repair. The existing
authenticated encoding, ceiling and transaction contracts supply the oracle;
no upstream algorithm or external API was introduced. Trace/debug decisions:
preserve existing runtime signals, use content-free typed limit evidence and
bounded independent expected values. Encoded lengths are compared with actual
batch bytes. No reservation counter is presented as measured native memory.

## Executed verification

Local platform: Windows, repository-pinned Rust toolchain. Final local runs
use the isolated `CARGO_TARGET_DIR=target/c07d-isolated-build` beneath the
original workspace, not the shared worktree build cache. Commands below run
from the PR worktree. Logs are retained in the original workspace's ignored
`target/c07d-*.log`; native candidate CI must independently build its checkout.

| Command | Actual result |
|---|---|
| `cargo test -p rrd-lsm --test write_admission --locked` before repair | 2 failed: oversized staging was accepted. |
| `cargo test -p rrd-store --test write_admission --locked` before repair | 2 failed on rrflowMX/rrflowKV staging. |
| `cargo test -p rrd-engine --lib oversized_semantic_commit_is_rejected_abortable_and_reopens_without_effects --locked` before repair | Failed: internal substrate error instead of resource exhaustion. |
| `cargo test -p rrd-lsm --lib admission_tests --locked` | 4 passed: cap-minus-one/cap/cap-plus-one against real encoding, replacement/tombstones, repeated physical keys, operation cap and checked overflow/invalid prior charge. |
| `cargo test -p rrd-lsm --test write_admission --locked` in isolated build | 2 passed; final full suite also reran the strengthened typed-error assertions. |
| `cargo test -p rrd-store --test write_admission --test semantic_commit_atomicity --locked` | 6 passed, including complete physical-row comparison around rejected semantic fan-out, existing graph/function receipts, valid subsequent work and durable reopen. |
| `cargo test -p rrd-engine --lib engine::tests::write_admission --locked` after corrections | 1 passed. Authenticated versions, claims, projection work, prior audit/outcome and absent rejected audit/outcome stay exact across rejection/reopen; prior receipt replay, abort, a distinct new claim and a second reopen succeed. |
| `cargo test -p rrd-engine --test direct_read_source_closure --locked` | 1 passed after the oracle adopted bounded stamped version reads. |
| `cargo test -p rrd-engine --test workspace_architecture --locked` | 31 passed after new test sources were added to Git. |
| `cargo test -p rrd-lsm -p rrd-store --all-targets --locked` | 346 passed, 0 failed, 1 ignored across 59 result groups. The ignored owned-child fixture is selected explicitly by native crash parent tests; Linux-only memory targets execute no tests on this Windows host. |
| `cargo test -p rrd-engine --all-targets --locked` final run | 209 passed, 0 failed, 2 ignored across 12 groups. Installation child runs through its parent fixtures; privileged foreign-owner denial is required separately by native CI and was not run locally. |
| `cargo clippy -p rrd-lsm -p rrd-store -p rrd-engine --all-targets --locked -- -D warnings` | Passed. |
| `cargo fmt --all -- --check`; `python scripts/check_version.py`; `python scripts/ci/check_workflow.py`; `git diff --check` | Passed; version remains 1.0.0. |
| `python -X utf8 scripts/knowledge/render_navigation.py` and `--check`; `python -X utf8 scripts/ci/build_execution_inventory.py` and `--check`; `python -X utf8 scripts/ci/check_documentation.py` | Passed: 17 generated indexes, 400 nodes/1,748 edges, 1,391 inventory records and valid documentation authority/local links. |

The native topology/portability jobs now require four separate steps for the
accountant, native transactions, composed store atomicity and engine recovery.
Separate steps preserve failure propagation on Windows as well as Unix.
The workflow policy requires each exact command once per native job owner.
Existing cohesive suites and installed crash/reopen lanes remain required.

## Surfaced failures and corrections

1. The first engine fixture supplied a nonexistent `AbortTransaction.reason`;
   compilation exposed the mismatch and it was corrected to the existing empty
   request before the actual red behavior was recorded.
2. The first post-edit shared-target native run stayed red while Cargo reported
   the library fresh. It is not accepted evidence. An isolated target rebuilt
   the changed library and passed; all final runtime/lint runs use that target.
   No claim is made that the underlying shared-cache anomaly was fully diagnosed.
3. The strengthened engine readback initially reused the helper's exact claim
   identity for subsequent work. It therefore overwrote that same identity;
   the fixture now declares a distinct subject to prove prior/new values.
4. Full engine verification rejected a new unstamped causal-log test call.
   The oracle now uses the existing authenticated, bounded direct-version API.
   The source-closure allowlist was not weakened.
5. The next full engine run passed runtime tests but failed the tracked-source
   architecture check for the three untracked new test files. Adding those
   files resolved it; the final complete engine run passed.
6. Exploratory commands that used nonexistent guessed paths, an incorrectly
   quoted `gh --jq` filter, or an already completed tool session failed without
   repository effects. Follow-up discovery/bounded status reads corrected them.

## Checklist, limits and landing

The package checklist is satisfied by the PR brief and this record: one
OBJ-02/C-07d outcome; current/target/scope/stop decisions; reviewed owner and
implementation paths; red oracle before repair; source-convergence mapping;
research/trace/resource decisions; focused then package verification; retained
failures; full-file/diff review; and linked evidence with generated navigation.
Navigation, execution inventory and documentation checks passed before commit.
They are not runtime proof; accepted-main reconciliation still precedes landing.

POAM-035 now records the demonstrated admission gap and bounded remediation.
No objective, C-07 subgate or alpha status is closed. Configurable aggregate
limits, pre-lock queue admission, mutable/debt/recovery caps, installed GC,
retained-owner interleaving, real bounded-volume exhaustion, native capacity
profiles and the actual 24-hour soak remain required. Those experiments were
not run by this package; no full reasoning/model/knowledge-path or PostgreSQL
compatibility claim follows from its storage evidence.

The PR's exact candidate CI and merge record bind acceptance. Seed CI,
previously accepted binaries and local test counts cannot replace it. The
official private repository is not a push or promotion target for this work.
