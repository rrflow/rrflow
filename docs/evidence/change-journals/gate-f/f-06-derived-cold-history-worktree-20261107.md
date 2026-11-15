# F-06 derived cold-history publication and readback: the worktree's own record

**Status:** historical record of an uncommitted worktree; F-06 remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-f/f-06-derived-cold-history-worktree-20261107`
**Owner:** F-06 package driver
**Superseded by:** [F-06 cold history hardening](f-06-cold-history-hardening-20261114.md)
**Provenance.** This is the longer journal that `wip/f06-main-integration-20261107` kept beside its uncommitted work. It was absorbed on 2026-11-14 ([ledger](../repository/archive-absorption-20261114.md)); the body below is unchanged except its title, status and coordinate, which were changed so it does not collide with the shorter [journal on main](f-06-derived-cold-history.md). Its statements describe the worktree as it was on 2026-11-07, not `main` today. What was recovered from it is in [the hardening journal](f-06-cold-history-hardening-20261114.md).


## Scope and authority

This draft advances a prerequisite of OBJ-05, OBJ-07, F-06 and POAM-004. The
starting public main was `ffbb91f6bf06cee7f0fde6004f3e3d66bded4a36`; draft
[PR #269](https://github.com/rrflow/rrflow-development/pull/269) contains the
change brief and its refinements. Identity PR #266 overlaps the contract,
query, and engine boundaries and must land before this candidate is refreshed.
The existing rrflowKV WAL, runtime change history and control journal retain
authority. No source history is reclaimed by this package.

The baseline had bounded uncompressed query export but no background Parquet
history publication or authenticated cold-generation read. This draft adds an
opt-in installed policy, bounded source-page selection, compressed Parquet
encoding, source/policy fingerprints, a prepared control record, anchored and
synced immutable publication, a receipt chain, and authenticated status and
generation readback. The cold policy sets an offer interval; native and cold
offers share one coalescing worker; missed timer ticks are skipped and each
work kind retains at most one pending offer.
A stale source stamp or changed policy clears only pending
work; an ordinary later write does not invalidate the original captured page.
Status without a generation describes the recorded head and does not verify an
artifact. Selected-generation reads verify the receipt chain and file.

The affected owners are `rrd-store` runtime change paging and native admission,
`rrd-query` Parquet encoding/decoding, `rrd-engine` maintenance and governed
control records, `rrd-contract`/`rrd-server`/`rrd-client` public readback,
their generated SDK projections, and installed lifecycle evidence. No provider
or external service is needed. Changed and generated paths are mechanically
available in the PR diff; the work does not delete or move an implementation.
The changed policy remains restart-bound rather than hot-applied.

## Design and review decisions

The [Parquet format](https://parquet.apache.org/docs/file-format/) places
column chunks before the footer. Publication therefore syncs the completed
artifact before advancing its durable receipt, and does not equate an existing
partial file with a committed generation. [PostgreSQL WAL guidance](https://www.postgresql.org/docs/current/wal-intro.html)
supports retaining the durable log before considering data-page recovery.
[Parquet page indexes](https://parquet.apache.org/docs/file-format/pageindex/)
are optional; these tests do not establish predicate pushdown or byte skipping.
Tokio documents [zero-period rejection and the immediate first tick](https://docs.rs/tokio/1.53.1/tokio/time/fn.interval.html),
[skipped missed ticks](https://docs.rs/tokio/1.53.1/tokio/time/enum.MissedTickBehavior.html),
and [cancel-safe interval ticks](https://docs.rs/tokio/1.53.1/tokio/time/struct.Interval.html#method.tick).
Those facts inform the scheduler; they do not establish fairness or latency under load.
No upstream source was copied. The configured byte ceilings are accounted
bounds, not measured process RSS: native point-read can materialize one record
before its size is known and Arrow/Parquet buffers require separate measurement.

Read in full for this review included the repository change-authoring procedure,
the POAM-004 detail, the new cold-history contract, and the engine maintenance
entry point. Relevant portions of the roadmap, implementation, installation
driver, API dispatch, existing tests and generator sources were inspected.
This record makes no whole-repository or every-line review claim. Runtime
control receipts carry source/configuration identities; content is excluded
from maintenance warnings. A bounded failure class is persisted when control
storage is available; detailed measurements and reporting when that storage
is unavailable remain open.

## Local evidence and surfaced failures

On Windows x86-64, using `CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target/f06-cold-build`:

- `cargo check --offline --locked -p rrd-engine -p rrd-server -p rrd-contract --tests` passed.
- `cargo test --offline --locked -p rrd-store --test cold_history_paging` passed one scoped paging/reopen test.
- `cargo test --offline --locked -p rrd-query --test cold_history --test parquet_export` passed four tests, including compressed stamped readback and corrupt/limit rejection.
- `cargo test --offline --locked -p rrd-engine --lib engine::cold_history::tests` passed five tests, including actual maintenance entry, replay, source and policy changes, authenticated readback, reopen, corrupt-file rejection and durable deduplicated failure/recovery status.
- `cargo check --offline --locked -p rrd-client --tests` passed.
- `cargo test --offline --locked -p rrflow-cli --test installed_lifecycle configured_cold_history_publishes_and_reads_after_process_restart -- --nocapture` passed: the installed server published a configured generation and returned it through the client before and after forced termination and restart. This does not inject termination within publication.
- After correcting the shared fixture, `cargo test --offline --locked -p rrd-contract` passed; `cargo test --offline --locked -p rrd-engine --lib` passed 336 tests with 4 ignored.
- `python -X utf8 scripts/ci/check_generated_surfaces.py --write` and `--check` reported 76 HTTP operations and matching SDK/signal projections; this is catalogue parity, not proof of every SDK behavior.
- TypeScript `corepack pnpm check` passed generation, lint, typecheck and six SDK tests. Python `uv run --offline python scripts/generate.py --check` and Go `go run ./cmd/generate -check` passed. The installed-process test passed again after the OpenAPI digest update.
- `cargo test --offline --locked -p rrd-query` passed the query package suite before the subsequent cooperative source-deadline change. Two focused cold policy and request validation tests also passed.
- The final grouped run passed `cargo test --offline --locked -p rrd-contract`, the rrflowKV cold paging test, both query cold-history/Parquet tests, all five `engine::cold_history::tests`, and the installed lifecycle cold-history test after the generated Rust-client binding and response validation were added. The 336-test engine suite and complete query suite precede those final focused changes; they are not evidence for a later integrated main.
- `cargo clippy --offline --locked -p rrd-engine -p rrd-query -p rrd-store -p rrd-client -p rrd-contract -p rrd-server --all-targets -- -D warnings` passed on Windows.
- `cargo clippy --offline --locked -p rrflow-cli --test installed_lifecycle -- -D warnings` also passed.
- After adding configurable scheduling, the paused-clock cadence test and both server worker coalescing/shutdown tests passed. The five cold-engine tests and real Windows installed-process test passed with a 25 ms interval. Earlier full-package and lint results precede this scheduler revision.
- After updating the generated OpenAPI identity to `60d01903f277eede2c2a585d20477e213a64af19471a902df87fcfb24f89b99e`, a grouped Windows run passed the full contract suite, the scheduler and worker tests, all five cold-engine tests, and the installed publication/restart/readback test. Generated-surface parity and TypeScript `corepack pnpm check` also passed for that identity.
- The first post-scheduler `cargo clippy --offline --locked -p rrd-server -p rrd-engine -p rrd-contract -p rrd-client --all-targets -- -D warnings` failed because the scheduler test module preceded another item. Moving the module to the end fixed the violation; that command and `cargo clippy --offline --locked -p rrflow-cli --test installed_lifecycle -- -D warnings` then passed. The relocation did not change the scheduler implementation.
- Review found that encoder budget errors were mapped to a generic query error and therefore the wrong durable failure class. The engine now preserves its existing typed query-error conversion and maps an already elapsed cooperative deadline to deadline exceeded. `cargo test --offline --locked -p rrd-engine --lib engine::cold_history::tests` passed six tests, including a real-engine output-limit regression that verifies resource-limit status and no generation advance. Earlier installed-process evidence predates this error-classification correction.
- `cargo fmt --all -- --check`, `python -X utf8 scripts/ci/check_documentation.py`, `python scripts/ci/build_execution_inventory.py --check`, `python -X utf8 scripts/knowledge/render_navigation.py --check`, and `git diff --cached --check` passed on the local draft. The inventory was regenerated after the journal changed; an initial check reported drift before regeneration.

The first locked query test stopped because enabling the pinned Parquet zstd
feature changed its dependency closure; an offline Cargo resolution updated
`Cargo.lock`. A closure lifetime error in the row encoder was corrected before
the passing tests. Running the Python SDK generator outside its managed
environment failed because `ruff` was not on PATH; `uv run --offline` in that
SDK succeeded. These focused tests do not establish a
candidate or release qualification.

The first full contract-suite attempt failed the shared SDK conformance corpus:
its endpoint count was 75 while the new catalogue has 76 operations. The shared
fixture was updated; historical evidence hashes were not rewritten. The
existing application-backup family list excludes cold receipts and artifacts;
restored source can be reprocessed, but existing generation identities are not
preserved across that restore. This is an explicit open boundary.

Review also found that the source loop did not check its elapsed budget while
scanning; the draft now checks a cooperative deadline between entries and maps
an expired deadline to a typed error. Individual reads, stamp validation,
compression and sync cannot be forcibly interrupted by this check.
The predecessor record was also charged against the next page after it had
been dropped, which could prevent progress when both records fit individually.
It is now independently bounded and only its digest is retained for the page;
the real-store regression covers that boundary.

The original stamped page reader verified each local change digest and its
predecessor chain but did not bind the page entries to the captured accumulator
root. A regression that changed a non-head entry and recomputed its own digest
was initially accepted on rrflowMX: `cargo test --offline --locked -p rrd-store
--lib bounded_page_rejects_a_self_consistent_change_outside_the_captured_root`
failed its rejection assertion. The reader now checks an RFC 9162 inclusion
proof for every scanned entry against the read stamp before advancing. The
same regression passed first on rrflowMX and then on native rrflowKV; the
real-store scoped paging test and six cold-engine tests also passed. Inclusion
proof checks do not make a standalone scoped artifact a completeness proof.
The installed-process publication and restart/readback command passed again
on Windows after the proof change; it did not terminate during publication.
`cargo clippy --offline --locked -p rrd-store -p rrd-engine --all-targets --
-D warnings` passed after this change.
Proof-node reads add work per cursor; their actual storage-key and elapsed-time
costs under large logs and their peak memory use remain unmeasured. The source
deadline is cooperative and the validation counter saturates at its existing
`u16` representation.

## Open boundaries and checklist

The change brief is attached to draft PR #269 and identifies the owner,
before/after oracle, source and configuration identities, acceptance commands,
primary research, diagnostic choices, and stop conditions. The implementation
is local and uncommitted; its changed paths are represented by the working-tree
diff, and generated outputs remain projections of the operation catalogue.
The smallest real-store, codec, engine, server, and installed-process checks and
the surfaced failures are recorded above. The full changed-file reread and
complete diff review have not been completed; the draft must remain uncommitted
until that procedure is finished. The full codebase has not been reviewed.

Scoped source rows do not independently prove no eligible rows were omitted;
security/control audit streams are not archived. There is no hot/cold rrflowQL
planner, safe source reclamation, orphan cleanup, complete error measurement,
per-generation paging, allocator-level memory proof, native full-disk/crash
matrix, nonblocking storage admission, starvation bounds, measured scheduling
under load, cross-platform soak,
or benchmark against Fjall and other systems.
POAM-004 records the verified coverage and cleanup gaps without a status change.
The complete diff and each changed file must be reread, generated navigation and
inventory refreshed, the installed driver and owning suites run, and current
main integrated before the draft is ready. Candidate CI, merge-group CI and
merge are not yet evidence. No F-06, C-07, alpha or release gate changes status.

## Local current-main integration preview, 2026-11-07

The original staged F-06 worktree was left unchanged. Prerequisite #266 is
merged as `7b9345d2c77129733ba88c8396c073c7ad5cbf7c`. A Git-only snapshot
`9dc7d8ac670e5b444106eda8e62af643cae382c0` and local synthetic preview
commit `1296c3837a02b595af8b8a629f02fc4045831ecb` captured its staged tree
for an isolated, uncommitted integration with public main
`838c85120621d5d9dde989d7677cc6ec89d20f03`. The synthetic preview commit is
not a push candidate. The staged combined source tree before this journal
update was `739f9a75fe396480b33c0be32aeede49ae22e717`. The preliminary
merge reported 22 conflicting files: the installed lifecycle and client
validators, contract digest, SDK fixture, reference citations and generated
SDK/signal projections. The isolated worktree keeps both installed test
functions and the cold-history and task-worker validators. Canonical combined
OpenAPI SHA-256 is `bf2878a2a1cc1bca8ad65ff3000f25788cb7b5d5f8bbea724eea0f688130beb6`;
all 76 earlier and all 82 mainline operation IDs survived, with 83 total.
Projections were regenerated from the merged contract.

The first combined library build failed with E0599 because main changed the
rrflowKV database owner from a mutex to an `RwLock`, while cold-history space
admission still called `try_lock`. The preview uses `try_write`, preserving
the earlier exclusive nonblocking behavior. The new real rrflowKV regression
`rrflow_kv::tests::immutable_file_admission_skips_held_database_locks` passed
with both reader and writer contention and after release. The first focused
command mistakenly used an unqualified name with `--exact` and ran zero tests;
the fully qualified executable invocation then ran and passed one. The
check remains a free-space observation, not a reservation.

On Windows, the original staged source passed
`cargo test --offline --locked -p rrd-query -p rrd-store -p rrd-engine -p rrd-server --lib`
and its installed `configured_cold_history_publishes_and_reads_after_process_restart`
test. On the combined preview, the corrected six-package library command
(`rrd-contract`, `rrd-client`, `rrd-query`, `rrd-store`, `rrd-engine`, and
`rrd-server`) passed; its engine suite reported 397 passed and 4 ignored.
The combined `rrd-store --test cold_history_paging` command also passed. The
subsequent query integration-test build was terminated deliberately when disk
free space fell below 6 GB; those query tests and the combined installed
lifecycle test did not run. A scoped attempt to remove the prior 69 GB Cargo
build-output directory was rejected by automatic policy; it remains intact.

`check_generated_surfaces.py --write` first failed before SDK generators ran
(missing the new operation); after each SDK's generator and signal rendering,
`--check` passed for 83 operations. `corepack pnpm check` failed because its
nested pnpm invocation selected 12.16.1 instead of the pinned 12.14.0. Running
the same four steps individually through `corepack pnpm` passed generation,
lint, typecheck and seven tests. Documentation policy, navigation, format and
CRLF-aware staged whitespace checks passed. The ordinary whitespace check
against the merge's old parent reported inherited mainline CRLF evidence; it
was not treated as a clean result.

Actually inspected in this integration include the full cold-history policy
and read contract, query codec, source page diff, engine publication/read
boundaries and maintenance scheduler, the anchored rename implementation,
the affected lifecycle/client conflicts, SDK generators and current roadmap
and F-06 journal. This is not an every-line review of all 53 changed files.
Before any branch update, complete that review, finish the interrupted tests
and installed readback on the combined tree, integrate any newer main, record
exact results and rerun the required candidate and merge-group checks. The
source-completeness, security/control audit, hot/cold query, reclamation,
resource, crash and cross-platform gaps above remain open. No gate status
changes.

## Bounded fair maintenance preview

The earlier draft dropped every offer while the joined worker was busy, so
repeated native or cold work could repeatedly suppress the other kind. In the
local integration candidate, each kind now has one coalesced pending flag and
selection alternates when both are pending. Shutdown discards pending work and
joins the active operation. This preserves one worker and bounded queue size;
it does not establish a time bound or prevent an active storage operation from
blocking. The embedded `maintain_storage` prioritization and actual loaded
scheduler performance remain unqualified.

A temporary isolated `rustc --test` harness included the actual server
maintenance module and linked its existing `tracing` library. Its three
module tests passed, including a blocked worker, 100 duplicate offers of each
kind, alternating delivery and joining on shutdown. The tested module Git blob
was `56ffb1493e6cc0f5015240f102aa8eedebfbd04c`; the local executable SHA-256
was `400157fde7ac773914e45204ff4d3f248e9b11c4ca349734bfeabf640e2ff576`.
The harness is not an
installed-server or owning-suite test; rerun the server suite and installed
readback when disk space is available. The panic printed by the shutdown test
was deliberately injected and the test passed.

The interval documentation changed the description in both the success and
error OpenAPI envelope. An initial comparison incorrectly expected one change
and failed; a corrected structural comparison verified exactly those two
description projections and no other schema change. The regenerated current
OpenAPI SHA-256 is `b99c0ee5e62cd0afc1fef72bb82626b860959c14d8330fb3b8ece7ea009753d5`;
generated-surface parity passed for all 83 operations. Separate TypeScript
generation, lint, typecheck and seven tests passed again for this digest. The
preceding combined
source-tree hash `739f9a75fe396480b33c0be32aeede49ae22e717` was an index
identity, not the complete tested working tree: the then-current contract
digest had not been staged. Treat the earlier tests as working-tree evidence,
not exact-tree evidence. The current generated digest and staged tree must be
verified again with the final tests.

## Real installed record-history readback on the combined candidate

The previous low-space restriction on D: was avoided without deleting any
source or old build output by using a new Cargo target at
`C:/Users/jessa/AppData/Local/Temp/rrflow-f06-combined-20261107` with
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, and `CARGO_BUILD_JOBS=3`. On Windows, the
combined final scheduler's `rrd-server --lib` suite passed 7 tests and
`rrd-store --test cold_history_paging` passed one. The first
`rrd-query --test cold_history --test parquet_export` attempt failed because
the old fixture used `note:one` as an ID; current canonical IDs forbid a colon.
The fixture was corrected to kind `document` and ID `note-one`, without
changing production validation. The cold-history test and all three Parquet
export tests then passed.

The installed test was expanded through the generated client to add a
schemaless `cold-fixture` table, commit a uniquely named record, update it
and retire it in separate accepted transactions, retain each mutation and
acknowledged runtime cursor, and read every relevant published generation.
Before and after forcing a process restart, each cursor appeared exactly once
with the independently expected mutation, and the selected generation
readbacks matched. The first expanded attempt failed because its test schema
set `allow_additional_properties=true` for a schemaless table, contrary to
the existing contract; setting that strict-contract flag to false corrected
the fixture without changing the validator. The command
`cargo test --offline --locked -p rrflow-cli --test installed_lifecycle configured_cold_history_publishes_and_reads_after_process_restart -- --nocapture`
then passed one test. The exact staged source tree before this journal update
was `15d523ea51de7f5678d0eed28d26196155cda063`, with no unstaged source
changes. The built primary binary SHA-256 was
`f953f6845217a1da1ed4889c71062e923d057ff7d8e91912b025ef0fc89691ea`; the
installed test executable SHA-256 was `bace2ca1d91939451dd37c7fa40a967da21ca7a6bc5b9e36d2fffdaec28bb71a`.

This small installed fixture proves these particular archived mutations and
restart readback only. It does not establish independent scoped completeness,
transactional hot/cold query planning, reclamation, interruptions within
publication, full disk, 100,000 records, cross-platform soak, or F-06 closure.
The complete changed-file review, owning suites after final source changes,
current-main reconciliation, exact candidate CI and protected merge remain.

## Combined candidate owning suites

On Windows with the C: Cargo target and offline, locked dependencies described
above, `cargo test --offline --locked -p rrflow-cli --test installed_lifecycle`
passed all nine installed tests, including the exact-mutation and restart
readback. `cargo test --offline --locked -p rrd-query -p rrd-store` then
completed successfully, including the model soak and the query, graph,
archive and authenticated cold-paging tests. This is local Windows evidence,
not candidate CI, the required native C-07 capacity matrix or a production
soak. The tracked working tree had no unstaged source changes; its staged
tree before this journal update was
`8878af948f3a9c92ffb776a6d7c518a16cccfb62`. A strict all-target Clippy
run for the six affected engine/query/store/client/contract/server packages
passed after the suites: `cargo clippy --offline --locked -p rrd-engine -p rrd-query -p rrd-store -p rrd-client -p rrd-contract -p rrd-server --all-targets -- -D warnings`.

The current publication review also followed the anchored `rename_new_to`
implementation: it synchronizes source and target parent directories after
the rename. That code inspection does not replace a forced-interruption or
power-loss test of cold publication. The full changed-file review and the
remaining cross-platform and performance evidence are still required.

## Engine source-closure and security integration

`cargo test --offline --locked -p rrd-engine` passed 397 library tests with
four ignored, then failed `direct_read_source_closure` on newly introduced
source-read uses; the rest of the package did not run. The first narrow guard
update accounted for the engine cold-history test's one stamped read and the
query codec test's two snapshot reads, but the guard still failed. A comparison
of the complete actual and expected maps found a third call: the store paging
test reads the log once for its independently calculated entry-size oracle.
The final source-closure change names those three exact test paths, read kinds
and counts; the guard itself remains enabled. The production cold maintainer
uses a separate bounded authenticated read, not those test calls. The guard
passed after the three entries. The full failed output was retained at
`%TEMP%/rrflow-f06-closure-failure.txt` in this working session.

The public cold-history read exposes complete mutations and deliberately
refuses grants with data policies. A new engine security test initializes
authenticated restricted and unrestricted principals and checks status and
generation readback: restricted principals must be denied, while the
unrestricted principal reads the actual two changes. The first field-only
version passed one focused test; a row-restricted principal was then added.
The exact staged tree at the start of the row/field focused test and complete
engine-suite rerun was `a1ba537e026b779c165a4d085b44f354c94eaf05`, with no
unstaged source changes. Results of that rerun and strict lint remain pending.
No F-06 or C-07 status changed.

The first row/field expanded focused test failed. Its initial assertion hid
the returned error; after adding a diagnostic, the row-restricted status call
returned `IdempotencyConflict` because the fixture reused one audit request ID
across different principals. The fixture now derives unique request identities
for each principal, session and generation; it still requires the exact
`PermissionDenied` result for both policies. This was a test setup correction,
not evidence that authorization passed before the corrected rerun.

On staged tree `df85b53a9cfd7d07b63d064f12fef017f17b542f`, the corrected
row/field security test passed, and `cargo test --offline --locked -p
rrd-engine` completed successfully: 398 library tests passed, four ignored,
and the package integration and architecture suites passed, including the
source-closure guard. `cargo clippy --offline --locked -p rrd-engine -p
rrd-query -p rrd-store -p rrd-client -p rrd-contract -p rrd-server
--all-targets -- -D warnings` also passed. These are local Windows results;
they do not establish an installed project-to-Parse result, native capacity
qualification, or candidate CI. The closure scanner counts uses by file;
for the cold-history source file it does not itself prove the permitted call
is inside the test module, so review of the actual source remains necessary.


## Test-only source-closure oracle

The file-level source-closure allowlist previously named the production
`engine/cold_history.rs` file to permit one read in its test module. That
entry could conceal a future production use. The same stamped expected-source
read now resides in `engine/tests/cold_history_oracle.rs`, declared only from
the engine test module; the exact one-call exception now names that test-only
file. The runtime compactor and restart/corruption assertions were not
changed. The prior and successor tests use the same stamped source oracle.

Before this move the focused closure and restart/corruption tests passed.
After the move the closure test passed, but the restart test initially failed
to compile because the new module lacked the `StorageEngine` trait import.
Adding that import corrected the test-only module. On staged tree
`89831b1631083f6102213b3574f2cb0b50d00a24`, the focused closure and
restart/corruption tests passed again. The complete command
`cargo test --offline --locked -p rrd-engine` passed 398 library tests with
four ignored, all engine integration suites, and all 33 topology tests. The
full suite used the Windows C: target and environment recorded above. These
checks do not prove source-generation atomicity, platform capacity, or alpha.
`cargo clippy --offline --locked -p rrd-engine --all-targets -- -D warnings`
passed on the same staged source tree. No roadmap status changed. The required complete changed-file review, exact
branch update, candidate CI and protected merge remain.


## Embedded native/cold scheduling fairness

Source review found that the server's worker alternates the two pending kinds,
while the embedded `RrdEngine::maintain_storage` path always returned on native
progress before offering cold history. A continuously replenished native debt
could therefore prevent the embedded path from preparing cold history. Before
implementation the PR brief bound a real-store oracle and the ownership scope.
The first fixture attempt failed before the oracle because its fixed count of
flushes left no compaction debt: foreground write admission had already
compacted it. The corrected fixture creates debt as needed between actual
maintenance calls. On the starting scheduler it failed at `cold work must be
offered while native debt remains`, establishing the intended red result.

The engine now alternates the preferred embedded kind with a process-local
atomic flag only when cold work is configured; it tries the other kind once
when the preferred one makes no progress. This changes no durable owner and
is not a wall-clock, storage-lock or concurrent-caller fairness guarantee.
The real rrflowKV test seeds semantic history, repeatedly replenishes native
debt, observes native work on the first and third calls, and confirms cold
preparation and publication on the second and fourth with debt present. A
subsequent attempt to assert a compaction-specific counter failed because the
native quantum can instead perform reclamation. The final oracle checks the
unchanged cold state across the native call and does not mislabel reclamation.

On staged tree `2026106a07f966d97da06c228d81b312b25bebd3`, both focused
`cargo test --offline --locked -p rrd-engine --lib` tests for embedded
fairness and existing idle-maintenance cursor preservation passed; the full
`cargo test --offline --locked -p rrd-engine` passed 399 library tests with
four ignored and all integration and architecture tests;
`cargo clippy --offline --locked -p rrd-engine --all-targets -- -D warnings`
passed. `cargo test --offline --locked -p rrflow-cli --test installed_lifecycle`
then passed all nine tests on the same source tree, including configured cold
history readback after forced process restart. This suite does not establish
installed source Parse. Source review in this pass covered the full new
`rrd-query/src/cold_history.rs`, `rrd-query/tests/cold_history.rs`,
`rrd-store/tests/cold_history_paging.rs`, `rrd-contract/src/cold_history.rs`,
and `rrd-engine/src/engine/maintenance.rs`, plus the changed storage, server,
engine and contract diff ranges. This is not a complete changed-file review.
Cross-platform candidate checks, full F-06 evidence, C-07 and alpha remain open.


## Review coverage on the local integration tree

On local staged tree `0b2de1603629eb92e4126bde3f84153c5a5a3aeb`, this
review pass read the entire following changed source files; Git blob IDs bind
the actual bytes reviewed, not the later journal update:

- `crates/authority/rrd-engine/src/engine/core.rs`:
  `77f5645091d678418ebc3740a2497c9592512fd2`. Both persistent and volatile
  constructors initialize the embedded scheduling preference; only an
  installed configuration or internal test enables the optional policy.
- `crates/transport/rrd-server/src/http/maintenance.rs`:
  `56ffb1493e6cc0f5015240f102aa8eedebfbd04c`. Each kind has at most one
  pending offer, pending kinds alternate, and shutdown joins active work.
- `crates/compute/rrd-query/src/parquet.rs`:
  `3930e5535fc1960daf3120cf08a4de0c982119c8`. Parquet writing applies
  configured encoding and checks the declared resource estimate and sampled
  Arrow/writer buffers; those checks do not prove peak process RSS.

The prior pass read the following complete files, with their unchanged blobs
on this tree: `rrd-engine/src/engine/maintenance.rs`
`300090aa8cfcf8054438d00465a00d894266f03a`,
`rrd-query/src/cold_history.rs` `fa2e58eb0ff00867247f1b672237d11ad22a2d1d`,
`rrd-query/tests/cold_history.rs` `94156e5d70446208e7199c52b1b51d14c2f684e4`,
`rrd-store/tests/cold_history_paging.rs` `cfcab247fbfb4c452e5e17631256f00ee403fcd1`,
and `rrd-contract/src/cold_history.rs` `e82582785162c5a280d9163fa84c5284fb04f957`.
Changed ranges of additional files were inspected, including source paging,
HTTP scheduling and installed lifecycle assertions, but their complete files
have not all been reread. The full 56-file candidate includes generated SDK
projections and remains incompletely reviewed. No candidate readiness or
roadmap status is inferred from this coverage.

The subsequent pass read all of `crates/transport/rrd-server/src/http/server.rs`
(blob `0e3d1107bdeec569fd16ef58ac33af0db2f2409e`). The server creates the
optional cold sweep only from the installed policy, skips missed ticks, offers
work to the shared maintenance worker, and preserves the separate native
sweep. This review does not establish an upper bound for storage syscalls or
the worker join. `git diff --check 838c85120621d5d9dde989d7677cc6ec89d20f03
--` passed for the candidate comparison. Remaining changed files have not all
been reread, so the full review and branch update remain open.

`python scripts/ci/check_documentation.py` passed on the local integration
worktree after this review evidence update.

The review also covered the full engine cold-history module
`crates/authority/rrd-engine/src/engine/cold_history.rs` on blob `f0002b37f76f4c6ce96662da06c6ee64dd6f59b1` and the
test-only oracle helper on blob `f83ba181509d0021c9389cfdd35d9ccad9fce0b9`.
Preparation binds the stamp, policy and artifact digest; publication re-reads
the stamped source before its receipt/head transaction; generation reads
authorize before exposing payload and verify the recorded digest chain. The
per-generation reader still does not independently establish scoped
completeness, and cancellation or power-loss coverage remains incomplete.

`python scripts/ci/check_generated_surfaces.py --check` passed on the local
integration candidate: 83 HTTP operations, signal fingerprint
`239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07`, and
OpenAPI digest `b99c0ee5e62cd0afc1fef72bb82626b860959c14d8330fb3b8ece7ea009753d5`.
This is the checker's projection parity, not a full SDK file review or SDK
conformance run.

Further complete-file review covered these candidate blobs:

- `crates/persistence/rrd-store/src/access/runtime_state.rs`
  `5dc3470b9640a279b811e2f846b04e7c0a5cc6ba`: each admitted cold page entry
  is bound to the runtime accumulator; a point value is materialized before
  its length can be checked, so this is not a hard allocation limit.
- `crates/transport/rrd-server/src/http/handlers/data.rs`
  `b3913c58bd4d0e9275b3390ff510842a67f72945`: cold read uses the authenticated
  envelope and existing ChangefeedRead operation before engine-level checks.
- `crates/compute/rrd-query/src/lib.rs`
  `cebb1588dc84ae7dcb64c038fecf3d72af258744`; query, server and engine
  Cargo manifests `55848c65dd4c0b447f357e7fdf03ecfac4f68317`,
  `727d0cd2390443f79ce828c059cea25e5526211e`, and
  `9c584c96bfdfcd568f7b4c831f09402784b9934b`: the added Parquet codec is
  exposed from the existing query crate, with zstd enabled on the pinned
  Parquet dependency; the engine's direct Parquet dependency is test-only.

Other changed files, including the large generated SDK schema, still require
the repository's full-file and complete-diff review before branch update.

This pass also read complete files `crates/persistence/rrd-store/src/repository/runtime.rs`
(blob `7aa0a69cb2c84d0890fbadea8fdacae7f5b80e05`) and
`crates/transport/rrd-client/src/catalogue_call.rs` (blob
`c9ba61e49731f80607438d3d44e07bad84e00bc1`). The bounded cold reader
validates the supplied stamp inside the storage transaction before paging;
the Rust client's cold operation validates request and response identity.
Neither fact establishes independent scoped completeness.

Complete-file review also covered the engine composition module
`crates/authority/rrd-engine/src/engine/mod.rs` (blob
`e417fb5cd055a291dc912c54568b1c4af45e67aa`), its test module
`crates/authority/rrd-engine/src/engine/tests.rs` (blob
`a2416a03d81b92b3139305c663ef56faef613673`), and the
`crates/authority/rrd-engine/tests/direct_read_source_closure.rs` scanner
(blob `486afc86d3967a67e0faf13e656b5d474894fb45`). The oracle helper is
compiled under the existing test module; the scanner counts named textual
uses by file and is a regression guard, not a semantic proof that all replay
paths are absent.

Complete-file review covered `docs/reference/storage/tiered-persistence.md`
(blob `407c866b599eb53d9e31e055b189ee30d4075905`),
`docs/poam/rrflow-1.0-alpha/poam-004.md` (blob
`d263aa2e9c894bd64cd126a46cf1980800dc2298`), and the generated change-journal
root and Gate F indexes (blobs `d35ff8331291b3216759800c0dc5a51d707bd0b1`
and `bb600efeaa71870c95666ae79c65e653a2826dab`). The reference explicitly
identifies backup omission, non-authenticated scoped completeness and
cooperative rather than hard resource limits as outstanding; these are not
closed by the partial cold-history implementation.


## Real-daemon SDK cold-history conformance

The six-language SDK corpus previously did not call the new cold-history
operation. The PR brief bound an extension before implementation. The shared
corpus now contains the explicit installed cold policy, generation-one read
request and seeded actor. The real installed conformance harness uses that
policy, prepares and publishes the seeded runtime history through RrdEngine,
then exposes the existing authenticated server. Each language driver checks
the returned scope, generation, cursor, row count and seeded actor. No stub
response or separate engine was added. The existing read-only identity
projection now uses the same corpus-owned configuration and updates every
fixture scope, including the vector-index requests; `cargo run --offline
--locked -p rrd-engine --example refresh_sdk_installation_identity -- --check`
passed without installing a project.

The first compile failed because the shared fixture helper lacked its
configuration imports; that was corrected. The first public contract suite
then failed its old fixed 13-domain assertion. The test now checks that
`cold_history` is present and that removing it is rejected. The corrected
`cargo test --offline --locked -p rrd-contract --test public_contract` passed
47 tests. The real conformance harness was built with
`cargo build --offline --locked -p rrd-client --example sdk_conformance_server
--profile test`. On staged source tree `d4e6fff0c10310ac1896a159014d3de692148625`,
a temporary local subset runner invoked the existing harness with its private
manifest and the standard language drivers; Rust, Python, TypeScript and Go
all passed against one real daemon and it exited 0. A second owned daemon
passed the .NET driver (five xUnit tests, none skipped) after an offline
locked restore to C: build artifacts; it also exited 0. Both runs used corpus
SHA-256 `c89802d2a1ccaeefe943f896cfa1483a8e06e5aec6801eff3c4e2916ea0a9148`.
Java/Maven are unavailable locally, so Java has not been compiled or run.
The local TypeScript run and typecheck used Node 24.21.0, below the declared
24.22.0 minimum; candidate CI must verify the supported runtime. Python Ruff,
TypeScript Biome and TypeScript typecheck passed. Generated-surface parity
passed again with 83 operations and unchanged OpenAPI digest. These tests
prove the named client calls reached real archived data in this fixture; they
do not prove scoped completeness, large-scale load, the project-to-Parse
journey, full SDK conformance across supported toolchains, or F-06/C-07/alpha.
The Java check, complete changed-file review, exact candidate CI and protected
merge remain open. No roadmap or POAM status changed.

`cargo fmt --all -- --check`, `git diff --check` against public main
`838c85120621d5d9dde989d7677cc6ec89d20f03`, and the documentation policy
check passed after the SDK work. The full Python conformance driver was
reviewed at blob `b0ba746a3a6ddff46f4c297678e43cfc1535ee69`; it checks
the cold result in the same authenticated session as the other corpus domains.
This does not qualify the still-unreviewed language drivers.

Following the SDK conformance additions, the local candidate comparison to
main `838c85120621d5d9dde989d7677cc6ec89d20f03` has 68 changed files, not
the earlier 56-file review snapshot. The complete-file and complete-diff
review remains outstanding for the expanded candidate; this is not readiness.


### Local Java conformance follow-up

The earlier note that Java was unavailable describes the starting host, not
the final local test. An isolated temporary Windows toolchain was obtained from
Eclipse Adoptium's JDK 25 API and Apache Maven's published distribution. The
JDK 25.0.5+8 ZIP matched SHA-256
`636bdc13dc649ddc7d5921c5c7d23cca6e44fd19d662b45cc9c02135a6c96c42`;
the Maven 3.9.16 ZIP matched SHA-512
`ed41650d42485cfc243fad22158caf9cbb5dc408ce7a09ddb94dd42a019de929ca43065bfa450612cf12bf78b5cafa3884b96c090de326ff590448c933454af3`.
These checks matched the publishers' download metadata; no signature
verification is claimed. The archives were extracted only under a C: temporary
toolchain directory, without changing the system-wide configuration.

The first real-daemon Java run used offline Maven and failed before compiling:
the local cache lacked `maven-resources-plugin:3.4.0`. The owned daemon exited
0. A subsequent normal Maven repository-backed run compiled the Java 25
sources and passed `SdkConformanceTest` (one test, zero failures or skips)
against the same corpus SHA-256
`c89802d2a1ccaeefe943f896cfa1483a8e06e5aec6801eff3c4e2916ea0a9148`; its
owned daemon also exited 0. The Java SDK was copied to an isolated C: directory
for build output, and all 21 tracked SDK source files matched the checkout by
SHA-256. The reviewed and tested Java driver blob was
`01cde3dd6a7cbddffdddf4e87c02dd914944a27a`; its formatting changed after the
five-language tree, but no production source or corpus changed. All six
clients have now passed this corpus locally across three separate daemon runs.
This is not the single-daemon full CI run, does not cure the local unsupported
Node version, and does not prove F-06, C-07 or alpha. Complete changed-file
review, exact candidate CI and protected merge remain open. No status changed.


### Full local six-language conformance command

The repository command `python scripts/ci/run_sdk_conformance.py` completed
successfully on staged tree `c2c33b811ab2288085858c8da19925f1c8329bee`:
Rust, TypeScript, Python, Go, Java and .NET all passed sequentially against
one real installed conformance daemon and the same corpus SHA-256
`c89802d2a1ccaeefe943f896cfa1483a8e06e5aec6801eff3c4e2916ea0a9148`.
A second run retained the complete local output at
`%TEMP%/rrflow-sdk-full-c2c33b81-20261107.log`; it exited 0 and its SHA-256
was `7aa7078920b6b8fd9b594e85c4d9b802e950b067d206f438ea04326b8674986a`.
The repeated run was for an untruncated retained log; it used a fresh owned
temporary estate and the standard harness shut down cleanly.

Node 24.22.0 and pnpm 12.14.0 matched the declared CI versions. Local
Python was 3.15.0rc2, Java was Temurin 25.0.5+8, Maven was 3.9.16 and .NET
SDK was 10.0.401; these do not exactly match all CI-pinned toolchains. The
local run does not replace candidate CI, multi-platform evidence, source
completeness, the project-to-Parse journey or F-06/C-07/alpha qualification.
No roadmap status changed. Complete changed-file review and protected merge
remain outstanding.

The subsequent complete-file review covered the TypeScript, Java and .NET
conformance drivers (blobs `837bbbf5e44daa445b7687ddbe3591108e7ae250`,
`01cde3dd6a7cbddffdddf4e87c02dd914944a27a`, and
`de0eff69687f557429cc4f3700e1386068916dbb`), the shared corpus contract
`crates/transport/rrd-contract/src/sdk_conformance.rs` (blob
`5a0467f12b33dca9ec96a9ab32d66c4aff89fb66`) and
`scripts/ci/run_sdk_conformance.py` (blob
`b884a3f4337139bd3dc5aa9c2f2a4c2b6fabecd8`). Each reviewed driver checks
its named cold-generation response against the corpus in an authenticated
session; the orchestrator owns a single temporary daemon and stops it in a
finally block. Other changed files remain unreviewed in full.

The complete conformance harness file
`crates/transport/rrd-client/examples/sdk_conformance_server.rs` was reviewed
at blob `921887f25866c73d60a6be4ca2b3283eded24afc`, and the complete
installation-identity projection example at blob
`e10eaedde2eeb5e8872bb89610333906e4e2bb61`. The harness plans and applies
the same explicit policy, publishes actual seeded history through the engine,
and gives all six clients one authenticated endpoint. The temporary manifest
remains private; the conformance run is not a source-indexing test.

Complete-file review covered the Go conformance driver
`sdks/go/cmd/conformance/main.go` (blob
`ab9cf776de9aa262b1a50c18672a7820b5976e87`) and Rust driver
`crates/transport/rrd-client/tests/sdk_conformance.rs` (blob
`c0bbee06e888da001dd9c29dfef3ed0e149484fd`). Both require the shared corpus
and real daemon manifest for the evidenced run; the Rust test returns early
without that environment, so a bare `cargo test` is not equivalent evidence.

### Windows publication interruption and SDK identity regression

The existing F-06 draft brief was extended before edits to cover process
termination at the actual publication boundaries. The engine test module now
places test-only checkpoints after staging-file write, staging-file sync,
durable rename, and the atomic generation/head control commit. In each case
the parent prepares actual rrflowKV source history, terminates its owned child
at that checkpoint, opens the same store independently, verifies that the
committed head is unchanged before publication (or present after the control
commit), replays pending work, and compares the resulting Parquet mutations to
the source oracle. A further maintenance offer must not duplicate the
generation. The marker wait requires its complete contents and has a timeout;
the hooks are absent from normal builds. The focused Windows test
`cargo test --offline --locked -p rrd-engine --lib
engine::cold_history::tests::publication_survives_process_termination_at_each_boundary
-- --exact --nocapture` passed all four checkpoints inside one test. This is
process termination, not loss of cached storage writes or a full-disk, power
failure, concurrent-read, installed-daemon, or cross-platform crash matrix.

The first full engine library run exposed an integration failure in
`shared_sdk_corpus_identity_matches_bundled_installation`: the old test planned
the default configuration (`instance-e94433b2dad856172e39d9e9`) while the
corpus and actual SDK harness use the explicit cold policy
(`instance-89934856ffa8d7922e791fb9`). Its focused correction uses the
corpus-derived policy file outside the project and the canonical installation
planner, asserts the exact fixture instance and estate, and additionally checks
that the default and configured instance identities differ. The focused test
passed. The subsequent `cargo test --offline --locked -p rrd-engine --lib
--quiet` passed 400 tests with 4 ignored, including the interruption test.
After that test binary was compiled, a formatting-only line wrap was corrected
in the identity test. `cargo fmt --all -- --check` and
`cargo clippy --offline --locked -p rrd-engine --all-targets -- -D warnings`
passed with that final formatting. The first formatting checks reported the
line-wrap differences and were corrected. No runtime identity calculation or
default policy was changed.

The resulting reviewed test-bearing blobs are
`crates/authority/rrd-engine/src/engine/cold_history.rs`
`20d4cf2add10a627a0d705cb90d9aa109e413c5f` and
`crates/authority/rrd-engine/src/engine/tests/installation_bootstrap.rs`
`f79c67239b6b0f070dfca37b1645305442d15372`. The tests remain local in the
uncommitted integration preview; the draft PR head does not contain them.
Full changed-file review, an exact integrated candidate, required CI, the
independent scoped-completeness and retention evidence, F-06 and C-07 remain
open. No alpha or roadmap status changed.

### Child cleanup, build isolation, and further review

The interruption-test child had no bounded wait and a bare `Child` would not
automatically be terminated on a parent assertion failure. After extending the
draft PR brief, the test-only helper now has an owned child guard that kills
and reaps on drop, and its checkpoint wait has a 90-second fallback if the
parent disappears. The four real-process checkpoint cases exercise guard drop
before reopening the same store. No production process or publication path is
changed.

The first focused test attempt reused the C: Cargo target directory after it
had built the separate D-03 worktree. It failed to compile `rrd-engine` with
E0432/E0609 because the selected `rrd-contract` lacked the cold-history types
and field. An isolated target directory,
`C:/Users/jessa/AppData/Local/Temp/rrflow-f06-isolated-20261107b`, compiled
the candidate's contract and engine from scratch and the focused four-point
test passed (one test, 403 filtered out; 2m30s build). This supports the
cross-worktree artifact-contamination diagnosis; it is not candidate CI.
`cargo clippy --offline --locked -p rrd-engine --lib --tests -- -D warnings`
then passed using that same isolated target directory.
The local `rrflow-joined-delivery` routing skill was updated to keep Cargo
targets per worktree and to preserve unrelated work during cleanup. The
skill's validator passed; it is not a repository runtime or lifecycle owner.
The earlier formatting check reported the guard's line wrapping; after
correction `cargo fmt --all -- --check` passed.

Further complete-file review covered
`crates/persistence/rrd-store/src/rrflow_kv.rs` (blob
`5ea3b11807fee7d70edce1b4995126379c84e765`),
`crates/persistence/rrd-store/src/error.rs` (blob
`57742d03d8249fc5c73f862936044e0c357be678`), and
`crates/transport/rrd-contract/src/deployment.rs` (blob
`10f4c72df1a592eb88b7cb93f0c39b5f0202d509`). Cold-file space admission
uses the native recovery reserve through a nonblocking exclusive store lock;
available disk can change after admission, and I/O errors still require
handling. The optional policy is sealed into the existing configuration
digest and its absence preserves the default. The prior interrupted-test blob
was superseded by `343c014d184b3f960ff026c95710f4f555f8476f` for the
test-only child guard. This is not a complete candidate or codebase review;
unreviewed files and the complete diff remain outstanding.

The full native database owner `crates/persistence/rrd-lsm/src/database.rs`
was subsequently reviewed at blob `a4a48cb358d70d868a25a29b7ebb6294d378a8e7`.
The new immutable-file admission calls the existing anchored disk-reserve
check; it does not reserve space or alter WAL, manifest, or reclamation
authority. The native scan API still materializes full ranges in several
callers; this review does not establish a global bounded-scan claim or change
the separately recorded C-07/F resource gaps. Other candidate files and the
complete diff remain unreviewed.

The complete server router `crates/transport/rrd-server/src/http/router.rs`
was reviewed at blob `5c1fddfd71e13d164682f82d3c240c549187d33d`.
Cold-history dispatch resolves through the existing endpoint catalogue and is
classified as a changefeed read for auditing; the already reviewed handler and
engine apply the authenticated request and data-policy restrictions. Router
inspection does not substitute for independently testing each permission or
prove complete archive contents.

The full Rust-client operation module
`crates/transport/rrd-client/src/operation.rs` was reviewed at blob
`60e78edfda8cf719bb33097b8f36b90fb21e5230`. Its cold-history entry delegates
to the generated operation; the separately reviewed catalogue validator
checks request and response identity before exposing the result. This review
does not qualify other SDK operations or replace the real-daemon conformance
run.

The complete installed lifecycle test file
`crates/adapters/rrflow-cli/tests/installed_lifecycle.rs` was reviewed at
blob `4b703a479f07a66c7e09073ea81a2c64ab93aadf`. Its cold-history test plans
and applies an explicit policy, commits separate record create/update/retire
mutations, waits for their acknowledged cursors to be archived, and compares
their decoded values before and after primary-process termination/reopen.
The same file's loopback worker is a controlled response fixture; it is not a
live LFG/SWAG model or reasoning-quality comparison. This review does not
cover the complete joined project-to-Parse journey or cross-platform CI.

The complete shared SDK fixture `fixtures/rrd-sdk-conformance-v1.json` was
reviewed at blob `9467cd58adef67d75bab62c56baeb3ed05fb286c`, together with
the complete client manifest `crates/transport/rrd-client/Cargo.toml` at
`f66569737f495eb810b41539266fe42a92d75502`. The cold-history policy and
generation request share the fixture's planner-derived instance scope; the
added TOML dependency is test-only. The fixture includes controlled data and
does not establish product-scale behavior or the complete source journey.

## TypeScript integer precision finding

A Node 24.22.0 controlled-response reproduction called the public `RrdClient`
`cold-history-read` operation with a valid JSON envelope whose
`through_cursor` was `9007199254740993`. The returned JavaScript number was
`9007199254740992`; no error was raised. The transport parses the response
with `JSON.parse`; its source blob `a3fde460889c3c6ad40f381d8d319afefead161e`
and the client source blob `c2758bbf2e8fe5b5e5a1792c306f70e7179d134e` match
public main `838c85120621d5d9dde989d7677cc6ec89d20f03`. Reproduction from
`sdks/typescript` (the `fetch` callback supplies the controlled response):

```powershell
@'
import { RrdClient } from './src/index.ts';
const raw = '{"protocol":"rrd","protocol_version":1,"request_id":"r","operation_id":"o","outcome":{"status":"ok","payload":{"scope":"instance:test","enabled":true,"current_generation":1,"through_cursor":9007199254740993,"pending":false,"changes":[]}}}';
const client = new RrdClient({baseUrl:'http://127.0.0.1:1',instance:'test',maxAttempts:1,fetch:async () => new Response(raw,{status:200,headers:{'content-type':'application/json'}})});
const actual = await client.call('cold-history-read',{scope:'instance:test',max_artifact_bytes:1024},{requestId:'r',operationId:'o',session:{principalId:'p',lease:{session_id:'s',token:'t'}}});
console.log(JSON.stringify({wire_cursor:'9007199254740993',returned_cursor:String(actual.through_cursor),returned_type:typeof actual.through_cursor,exact:BigInt(actual.through_cursor)===9007199254740993n}));
if (BigInt(actual.through_cursor) === 9007199254740993n) process.exitCode=1;
'@ | fnm exec --using 24.22.0 node --import tsx --input-type=module
```

The command exited zero and reported `exact:false`. The generated TypeScript
model exposes full-width contract integer fields as `number`. The earlier
six-language real-daemon corpus used small values and did not establish
full-width fidelity. The reproduction used a controlled transport response;
it did not establish that a real estate reached that cursor, and it was not
a complete numeric-domain survey.

[POAM-011](../../../poam/rrflow-1.0-alpha/poam-011.md) and H-04 own the
cross-surface remedy and its generator, supported-runtime, full-width request
and response, and real-daemon interoperability evidence. This candidate does
not contain that remedy; its cold-history readback must not be represented as
full-width TypeScript conformance. F-06, C-07 and alpha remain open.

## Further shared SDK corpus source review

The full `crates/transport/rrd-contract/src/sdk_conformance.rs` at blob
`5a0467f12b33dca9ec96a9ab32d66c4aff89fb66` and the full
`crates/authority/rrd-engine/examples/refresh_sdk_installation_identity.rs`
at blob `e10eaedde2eeb5e8872bb89610333906e4e2bb61` were reviewed on this
local integration preview. The former binds the cold policy and read request
to the shared validated corpus; the latter uses that same policy in the
read-only planner and rewrites the fixture's scoped requests to the resulting
instance. These are corpus setup and validation paths, not independent proof
of exact numeric transport, archive completeness or a complete installed
project journey. The previously recorded TypeScript 64-bit defect remains
unfixed. Other changed files, the complete candidate diff, exact candidate
CI and protected merge remain outstanding. No roadmap status changed.

## Rust conformance missing-setup correction

The PR brief was amended before changing the shared conformance test and
runner. The entire previous `crates/transport/rrd-client/tests/sdk_conformance.rs`
at blob `c0bbee06e888da001dd9c29dfef3ed0e149484fd` was reviewed. Its
real-daemon test returned normally when `RRD_SDK_CONFORMANCE_MANIFEST` was
absent, so a standalone test could be counted as passed without the corpus.
The test now marks itself ignored for ordinary package runs and requires the
manifest when explicitly selected. The existing runner selects that named
ignored test, streams its output with bounded marker look-behind, and requires
the corpus-digest success marker before reporting Rust conformance. The
existing real-daemon test assertions and other language runners remain in use.

On staged tree `b0b9309ed92da5f3d4e371f13977c931f9798d48`, Windows x86-64,
with isolated `CARGO_TARGET_DIR=C:/Users/jessa/AppData/Local/Temp/rrflow-f06-isolated-20261107b`
and `CARGO_INCREMENTAL=0`:

- `cargo test --offline --locked -p rrd-client --test sdk_conformance
  rust_sdk_passes_shared_real_daemon_conformance -- --exact --ignored
  --nocapture` with the manifest variable absent failed at the explicit
  manifest requirement, as intended. The initial full build took 1m36s.
- The same named test without `--ignored` reported zero passed, one ignored,
  and exited 0.
- `python scripts/ci/test_sdk_private_manifest.py` passed seven tests, one
  platform-skipped; the new subprocess oracles reject a missing success marker
  and accept a marker crossing a stream-chunk boundary.
- `python scripts/ci/run_sdk_conformance.py` exited 0 using the selected
  local JDK 25.0.5+8, Maven 3.9.16, Node 24.22.0 and pnpm 12.14.0; all six
  language clients ran sequentially against one real daemon and corpus
  SHA-256 `c89802d2a1ccaeefe943f896cfa1483a8e06e5aec6801eff3c4e2916ea0a9148`.
  The Rust test emitted the required marker. The complete local log is
  `C:/Users/jessa/AppData/Local/Temp/rrflow-sdk-conformance-marker-20261107.log`,
  SHA-256 `cd125fa0ce0495fa1016c859824595f048ec05c27f7e9b5b7a6455e1c911af36`.
- `cargo fmt --all -- --check`, Python compilation of the two scripts, and
  `git diff --check` passed.

The tested Rust test, runner, and harness-test blobs are respectively
`9ae435c8f246781223e64e3b70ee41d168dd8a0d`,
`13c0f46d884fcc15a4ddeb2c3eeb93e377cabd08` and
`0eefbfc268323b25b91b2f686b0327bc43ea5a3f`. This does not fix the
TypeScript precision defect, prove archive completeness, complete the
whole-candidate full-file/diff review, or provide candidate/merge CI. Main
subsequently advanced through PR #296 and must be reconciled before landing.
No roadmap or POA&M status changed; F-06, C-07 and alpha remain open.

A subsequent plain `git diff --cached --check` failed with extensive
carriage-return whitespace reports on receipt files inherited from main. This
detached integration preview has a synthetic parent, so its staged diff
includes hundreds of inherited paths; the failure is not a clean comparison
of the F-06 candidate against main. `git diff --check
838c85120621d5d9dde989d7677cc6ec89d20f03 --` had passed before main advanced.
Do not rewrite inherited receipts or claim the plain check passed: reconcile
the actual candidate with `9af4cb7d198b479e58269fe4b7464c49ade4764a` or
newer main and run the required check on that real comparison before pushing.

The full operation-reference source
`crates/transport/rrd-contract/src/operation_reference.rs` was reviewed at
blob `95ed554ff2e8fcdf0dba6236174c1a358a0aeba9`. Its added
`cold-history-read` description explicitly limits the operation to derived
history and does not claim scoped completeness, audit archival or source
reclamation. The complete generated
`docs/reference/protocol/signal-catalogue.md` at blob
`59fb94bcaebd82ffeb5af79e43d1fe02172c1e19` lists catalogue descriptors and
states that generation does not prove runtime collection or release readiness.
This source review does not establish either claim or complete the remaining
full-file, generated projection and whole-diff review.

## Public main PR #296 reconciliation in the local preview

The PR brief was amended before reconciling public main from
`838c85120621d5d9dde989d7677cc6ec89d20f03` to
`9af4cb7d198b479e58269fe4b7464c49ade4764a`. Git-derived path sets had no
overlap: 71 candidate paths versus seven main changes. The old-main-to-new-main
diff was examined; each target was checked for candidate edits and untracked
collisions before restoring the seven exact new-main files into the staged
preview. Comparing staged trees `e050ab94bfb3416727225d7034a7f6ae94babc7c`
and `f1195c94a332b1ae17d41f847505bd5956e2e4a2` shows exactly those seven
main paths changed; the prior candidate blobs were preserved. The latter tree
precedes this journal addition. This is still a detached synthetic-parent
preview, not an actual main-parent merge, commit, push, candidate CI or merge.

`python scripts/knowledge/render_navigation.py` passed with 1,016 nodes and
no regenerated indexes; `python scripts/ci/check_workflow.py` passed;
`python -m unittest discover -s scripts/ci -p test_cache_warm_policy.py`
passed seven tests; `python scripts/ci/check_documentation.py` passed; and
`git diff --check 9af4cb7d198b479e58269fe4b7464c49ade4764a --` passed before
this journal addition. Generated-surface parity passed for 83 HTTP operations
and seven signal projections with the F-06-specific C: Cargo target, OpenAPI
SHA-256 `b99c0ee5e62cd0afc1fef72bb82626b860959c14d8330fb3b8ece7ea009753d5`.

The first generated-surface invocation accidentally used the default shared
D: target. Its process finished successfully before the identified process
tree could be stopped (`taskkill` reported the parent was no longer present).
That result is not relied on because a shared target had previously selected
stale cross-worktree artifacts. The check was rerun with
`CARGO_TARGET_DIR=C:/Users/jessa/AppData/Local/Temp/rrflow-f06-isolated-20261107b`
and `CARGO_INCREMENTAL=0`, producing the result above. Neither build directory
was deleted. The 64-bit TypeScript defect, full candidate review, true-parent
reconciliation, required CI and alpha evidence remain open.

The PR brief was updated before recording a true-parent tree check.
`git merge-tree --write-tree 5d712f54dc689597a21c73ec41a51ca435c9c8c7
9af4cb7d198b479e58269fe4b7464c49ade4764a` yielded conflict-free tree
`7d8b5f37ba3907d601530e136a743156f70b4808`. A task-owned temporary Git index
was initialized with `git read-tree` of that result and populated with the
71 exact candidate index entries obtained by `git ls-files --stage -z` and
`git update-index -z --index-info`. Both `git write-tree` results matched at
`bce7d824c9ac8d63fbc4a39967bd94e2f54f3ad2`; there were no candidate deletions
and no unstaged files before this journal update. The temporary index is at
`C:/Users/jessa/AppData/Local/Temp/rrflow-f06-reconcile-index-20261107`.
This proves tree reproducibility for those exact parents and bytes only; it is
not a real merge, a complete source review or candidate/merge-group CI.
Further journal edits change the tree; preserve those and repeat the true-parent
check before creating the actual candidate. No roadmap status changed.

## Cooperative cold-artifact read deadline

The PR brief was amended before the change. The generation read previously
passed the whole file to `read_to_end` after checking its size; that call did
not check the query deadline between underlying reads. The existing engine
reader now allocates the admitted length plus one detection byte, reads at
most 64 KiB at a time, checks the same deadline before and after each call,
retries `Interrupted`, and denies short or extra input before existing digest
and Parquet validation. The bound is cooperative: a single operating-system
read can block past the deadline, and the allocation is not a measured RSS
limit. There is no new wire type, trace, authority or copied upstream code.
[Rust's Read documentation](https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read)
states that interrupted reads are retryable.

The first focused run passed ten tests but warned that the newly written
interrupted-reader fixture was unused; that test mistake was corrected and the
retry is now asserted. The second focused suite passed ten tests, including
the controlled slow-reader deadline, exact/short/extra inputs, real rrflowKV
generation read/reopen/corruption and four publication-process interruptions.
The engine library suite passed 401 with four ignored on source blob
`f26ee4e50ecea77c8b6c3c265b3d6458a66e8e3c`. The only subsequent change
widened the test's scheduling window; the final focused suite again passed
ten tests and strict engine library/test Clippy passed on source blob
`fcdee84742a8ce005a76e3325cea6f9ed7ba822c`. Final `cargo fmt --all --
--check` passed. Commands used the isolated F-06 C: Cargo target and
`CARGO_INCREMENTAL=0`:

- `cargo test --offline --locked -p rrd-engine --lib engine::cold_history::tests -- --nocapture`
- `cargo test --offline --locked -p rrd-engine --lib -- --nocapture`
- `cargo clippy --offline --locked -p rrd-engine --lib --tests -- -D warnings`

The final focused log SHA-256 is
`cfafde62a7aaacad3fc0cdefdb41d286d8dbf94b8d2506e9279d1b16c0c9e329`;
the full engine suite log is
`71f5a330442b40b7204f759e095b3d2a61a30fa85c120c50a5cfeec604b24742`;
and final Clippy log is
`9d9cc144f1365994d88253885f5693085c58e20cdc5531a89482bb9e4ad91b63`.
Logs are under `C:/Users/jessa/AppData/Local/Temp/` with the
`rrflow-f06-deadline-` prefix. The full suite is on the F-06 preview and does
not contain PR #252's still-failing interrupted-refresh oracle. This is not
power-loss, hard-syscall-deadline, installed-daemon interruption, archive
completeness, competitive performance, candidate CI, C-07 or alpha evidence.
The draft remains uncommitted; the TypeScript precision defect and complete
changed-file review are still open. No roadmap status changed.

A further PR-scoped test-only addition uses a reader larger than two 64-KiB
chunks: returned bytes match exactly and the largest read request is at most
64 KiB. The final cold-history source blob
`206daa43070b9409cb90ba2aebed24e768364b39` passed the same ten focused tests,
strict engine library/test Clippy and formatting. The focused log SHA-256 is
`530c99d920f5d4d466788d019fafea4f157cf275a5b9017d0963c17bbbd9a5ad`;
the Clippy log is
`773ec53e05f63e08c70b0f4b9cdeaeecd85dc47af5ebd4d580c387b2ebe0418c`.
The earlier 401-pass engine suite used the identical production change before
this test-only addition. Request size is measured; total allocation, RSS and
performance are not. The final source remains local and uncommitted.

The complete final `crates/authority/rrd-engine/src/engine/cold_history.rs`
was reread at blob `206daa43070b9409cb90ba2aebed24e768364b39`. The
chunked reader is called only after the existing request, authorization,
receipt, scope, row and artifact-size checks and before digest/Parquet
validation; it does not alter cold publication or any public contract. This
is one file's final reread, not a claim that the full candidate diff or every
changed file has been reviewed.


## Cold-read error identity

The PR brief was amended before changing the decoder. Its deadline and
resource checks previously both returned `rrd_query::Error::Budget`; the
engine therefore reported a decoder deadline as a configuration limit. An
initial local attempt to infer the cause from the clock after the error was
incorrect: a resource error could be relabeled if the deadline elapsed in the
meantime. It was replaced before staging with `ColdHistoryReadError`, which
identifies the decoder's own deadline separately and preserves other query
errors. `rrd-engine` maps this typed result at the existing cold-read boundary.
The separate encoder's pre-existing clock-based mapping was not changed and
remains a limitation for error attribution. This does not preempt a blocking
system call or prove a hard overall deadline.

At query decoder blob `795542c8c2791af157bae99765b48b8e3ae663f8`, query
export blob `bbc22ee4f451df36b686f645926b94964e62803e`, and engine cold
history blob `030f3df6a2ff40cc0c526eea6e900fa467592a8e`, the isolated C:
Cargo target and `CARGO_INCREMENTAL=0` produced:

- `cargo test --offline --locked -p rrd-query --test cold_history`: 1 passed.
- `cargo test --offline --locked -p rrd-engine --lib`: 402 passed, 4 ignored;
  the new test checked a decoder timeout and preserved a resource error even
  after its deadline elapsed. The separate D-03 failing oracle was not in this
  candidate.
- `cargo clippy --offline --locked -p rrd-engine --lib --tests -- -D warnings`
  and `cargo fmt --all -- --check`: passed.

The query, engine and Clippy logs under `C:/Users/jessa/AppData/Local/Temp/`
with prefix `rrflow-f06-typed-cold-` have SHA-256 values respectively
`86bf0627c9a54b3f5b304b8eae817a34313024ac80e10dbcef6edbfb7bdc44a1`,
`b82a9075024ca539301fef48be22e62d4bb169b9b0d603aa454d951ec2493cba`, and
`b28ac29557edc6faf7d2c7f1bd2f5f54f4e9ed5b03ef4a7ff5553775fc4749b5`.
This local work is neither committed nor pushed; full changed-file review,
true-parent reconciliation, candidate CI, native matrix, scoped completeness,
and the TypeScript integer defect remain open. No roadmap status changed.

The complete `crates/compute/rrd-query/src/cold_history.rs` was reread at
blob `795542c8c2791af157bae99765b48b8e3ae663f8`. Decoder checks are
cooperative at entry and after obtaining each batch; they do not preempt
metadata construction, an iterator call, or individual row decoding, and a
final row can finish after the deadline without a further check. The encoder
uses budget errors and the separate clock-based mapping described above.
These are explicit remaining deadline limitations, not an exhaustive candidate
review or an alpha claim.

The PR brief was amended before recording the newly observed decoder
completion and encoder-attribution limitations in the existing POAM-004. No
roadmap status changed and no decoder-completion repair is claimed.


## Decoder completion deadline follow-up

The PR brief was amended before this follow-up. The local decoder now checks
the existing monotonic deadline between decoded rows and before returning,
including an empty valid artifact. The engine had a separate final response
check already; this corrects the decoder contract and bounds work between
cooperative checks rather than proving that every previously late engine
response escaped that check. The internal clock parameter supports a
deterministic test; the public function uses `Instant::now`. A blocked Arrow
call, metadata processing or individual row cannot be preempted by this
change. Encoder error attribution remains open in POAM-004.

The first focused command used an incomplete fully-qualified name with
`--exact` and selected zero tests; it did not prove anything. The corrected
`cargo test --offline --locked -p rrd-query --lib
cold_history::deadline_tests::empty_artifact_cannot_finish_after_deadline --
--exact` selected and passed one test against a valid generated Parquet file.
At query cold-history blob `acb517e2bd2c05ed02af1b6aa48f14fa30978fcc`,
with the existing isolated C: target and `CARGO_INCREMENTAL=0`:

- `cargo test --offline --locked -p rrd-query`: 82 passed, zero failed.
- `cargo test --offline --locked -p rrd-engine --lib`: 402 passed, four ignored.
- `cargo clippy --offline --locked -p rrd-query -p rrd-engine --lib --tests
  -- -D warnings` and `cargo fmt --all -- --check`: passed.

The respective query, engine and Clippy logs under
`C:/Users/jessa/AppData/Local/Temp/rrflow-f06-cold-` have SHA-256
`fcd21a7a962952593c37b4992010259422383f34e91c0dcb49bd4e8e3751ac00`,
`308d833316b7d5cc2a43396622ade6f9240152603a246387a3cb8d735ea65971`, and
`12a295cfad798bb9673dfb894a5f0cd08a125c8892f4bae3d54424ca2c7fc842`.
The POAM-004 description now distinguishes the corrected cooperative
checks from the still-unresolved encoder and blocking limitations. This
source is local and uncommitted; the complete candidate review, D-03 red
oracle, TypeScript precision defect, CI, native matrix and alpha gates
remain open. No roadmap status changed.


## Parquet encoder failure identity and buffered footer

The PR brief was amended before each scope extension. Reading locked
`parquet-59.2.0/src/file/writer.rs` showed `TrackedWrite::into_inner` can
convert a buffered sink error into string-only `ParquetError::General`. A
real `ArrowWriter::into_inner` test reproduced a wrong budget classification
for a deadline; the first version using the standard error source chain was
insufficient. The first attempt at that test also failed to compile due to an
incorrect Arrow import; the corrected import ran the test and exposed the
classification failure. The PR stop condition was observed, and its design
was amended before further code.

The bounded output sink now retains the first failure identity in a marker
owned by its individual export; the writer maps that identity even if Parquet
stringifies the underlying I/O error. JSON timeout errors are separately
typed; a real byte limit remains a budget error. The canonical query error
passes through `rrd-engine` as a deadline instead of using the clock after an
error. Cold maintenance retries a smaller source page only for budget errors,
not a known deadline; general query export uses the same conversion. The
engine trace class includes the deadline. The marker does not change
publication, authorize effects, preempt a blocked call or prove performance.

On the Windows x86-64 F-06 preview, focused tests exercised actual buffered
Parquet footer timeouts and byte-limit errors, bounded JSON, and deadline
classification. With isolated C: Cargo target and `CARGO_INCREMENTAL=0`:

- `cargo test --offline --locked -p rrd-query`: 86 passed.
- `cargo test --offline --locked -p rrd-engine --lib`: 403 passed, 4 ignored.
- `cargo clippy --offline --locked -p rrd-query -p rrd-engine --lib --tests
  -- -D warnings` and `cargo fmt --all -- --check`: passed.

Final blobs: `rrd-query/src/parquet.rs`
`9a53614dbccdc070f01019bb5c4f7727020351a2`, `cold_history.rs`
`622c9726da49079254ccb49dafb55386b47e561c`, `error.rs`
`449da0424a0ddc0d1976cc0d770f51788c66d371`; engine `error.rs`
`8255ddfd25c0ab6403617482f06b3b6a30dfefc2`, `query_export.rs`
`ad46be182e7b1608c2fc23f99e7f4007766e2b5e`, `runtime/query.rs`
`6ed7ef92355945b98c7326aa4eb3638d3bdf5fd7`, and `engine/cold_history.rs`
`c246907e6fde46522d8fab5620ba62b533ab8584`. The query, engine and Clippy
logs in `C:/Users/jessa/AppData/Local/Temp/rrflow-f06-encoder-final-*.log`
have SHA-256 `543bfdb8e72bbc641e6f3cf6da98116dcfdc9fb7170b8899e4a11a5e900543ec`,
`57d9999f4a00a50c451cae3817ece21230f19559084de26215d70c51a18d9b36`,
and `5f7dce37e3a13bc4aaf5ae5826ae15bb5f4929eedd32c6c8ed7db8ab32668d02`.
The full `parquet.rs` and query `error.rs` were read before modification;
relevant engine export, error and trace paths and the final patch were read,
but this is not the full candidate review. The changes are staged locally,
not committed or pushed. The D-03 visibility regression, SDK integer defect,
source completeness, power-loss and native CI are still open. No gate status
changed.


## Current encoder candidate: real-daemon SDK run

On local staged preview tree `68a65bdf0266e0866cd68c7b5eb00579f4904190`,
`python scripts/ci/run_sdk_conformance.py` exited 0. Its owned daemon served
the same corpus to Rust, TypeScript, Python, Go, Java and .NET sequentially;
each emitted the expected corpus marker
`c89802d2a1ccaeefe943f896cfa1483a8e06e5aec6801eff3c4e2916ea0a9148`.
The selected Windows toolchains included Node 24.22.0, pnpm 12.14.0, JDK
25.0.5+8 and Maven 3.9.16; the isolated F-06 C: Cargo target and
`CARGO_INCREMENTAL=0` were used. The local log
`C:/Users/jessa/AppData/Local/Temp/rrflow-f06-encoder-six-sdk-20261107.log`
has SHA-256 `9dcb04a1b6fdf2ddbcfeb1c71ad82e6d39bb6eb176a991fe8fa2cfa760f9d3be`.
This is the shared installed-daemon corpus, not a source-project test,
large-integer conformance, independent C-07 qualification, or CI.

The complete final `crates/compute/rrd-query/src/parquet.rs` at blob
`9a53614dbccdc070f01019bb5c4f7727020351a2`, `rrd-engine/src/engine/error.rs`
at `8255ddfd25c0ab6403617482f06b3b6a30dfefc2`, and
`rrd-engine/src/engine/query_export.rs` at
`ad46be182e7b1608c2fc23f99e7f4007766e2b5e` were reread. This does not
represent a full reread of all candidate files. The tested tree precedes
this journal update; the draft remains local and uncommitted. No roadmap or
POAM closure changed.

Additional complete rereads at the same preview covered query `error.rs`
`449da0424a0ddc0d1976cc0d770f51788c66d371`, query `lib.rs`
`bbc22ee4f451df36b686f645926b94964e62803e`, query cold-history integration
test `94156e5d70446208e7199c52b1b51d14c2f684e4`, and Rust SDK conformance
test `9ae435c8f246781223e64e3b70ee41d168dd8a0d`. The query test uses an
actual rrflowKV store, typed changes, a Parquet reader and reopen; the Rust
SDK test is explicitly ignored outside the configured daemon harness. These
are reviewed files and exercised paths, not full candidate review or proof
of 64-bit TypeScript correctness.

The complete final cold read contract (`e82582785162c5a280d9163fa84c5284fb04f957`),
HTTP maintenance worker (`56ffb1493e6cc0f5015240f102aa8eedebfbd04c`) and
HTTP server (`0e3d1107bdeec569fd16ef58ac33af0db2f2409e`) were reread.
The contract validates request coordinates and budgets but explicitly does
not prove scoped coverage. The worker coalesces offers and joins its thread
on shutdown; cooperative bounds do not establish preemption of blocked I/O.
This is not a full review of the candidate or a new native soak result.

The complete final `rrd-store/src/access/runtime_state.rs` was reread at blob
`5dc3470b9640a279b811e2f846b04e7c0a5cc6ba`, together with the repository
entry for bounded cold change reads. The latter validates the read stamp
before the scoped page, and each scanned change is verified against that
stamp; it does not meter all accumulator-node reads as an aggregate memory or
I/O budget. Existing POAM-004 retains that limitation. This does not establish
the missing source-generation selection at a project snapshot or a complete
review of the whole candidate.

The complete final `rrd-engine/src/runtime/query.rs` was reread at blob
`6ed7ef92355945b98c7326aa4eb3638d3bdf5fd7`. It records the typed deadline
as a denied execution and emits a bounded error class and digest in the
existing durable trace; this review does not prove every query deadline path
emits this new error or complete the full candidate review.


## Real-daemon TypeScript 64-bit reproduction

The PR brief was amended before recording this follow-up in POAM-011. On
Windows x86-64, the existing locally built `sdk_conformance_server.exe`
(SHA-256 `a270c752288937eebf33d2edb47475525998d807173249d199f574a881fd8e67`)
opened a newly owned hermetic instance. The shared SDK corpus digest was
`c89802d2a1ccaeefe943f896cfa1483a8e06e5aec6801eff3c4e2916ea0a9148`.
The first probe was rejected by the real engine because its transaction
commit omitted the mandatory deadline; no successful mutation was claimed.
The corrected probe used the Python SDK to create a session and collection,
begin a transaction and commit a vector property with unsigned value
`9007199254740993`, using a typed-mutation digest checked against the
unchanged fixture's known digest. It read the accepted value from the
changefeed through Python, then read the same changefeed and session through
the TypeScript SDK on Node 24.22.0. Python returned the exact value once;
TypeScript returned `9007199254740992` once and reported the resulting
number unsafe. The test daemon exited cleanly; scripts did not print tokens.
The scratch Python and TypeScript probe SHA-256 values are respectively
`7a93ec99bb4ad98195fa1d40716fcde1beeead324defa7b4ea6e8381ac7117d2` and
`d21d16cfed2e1aa0fdbc5da56d4b2355704c8f8f210ccc84560b1a659ab1a94a`;
they remain under `C:/Users/jessa/AppData/Local/Temp/` with prefix
`rrflow-real-u64-probe-20261107` and are not a portable maintained harness.
This validates the observed corruption on an accepted synthetic transaction,
not after restart, and does not solve request serialization, schema-aware
64-bit decoding or cross-language conformance. The binary was rebuilt by the
prior six-SDK run on tree `68a65bdf0266e0866cd68c7b5eb00579f4904190`; only
review-journal changes intervened. The 76-path local candidate is uncommitted.
No alpha or POAM closure changed.


## Precision repair schema scope

The PR brief was amended before this evidence. The candidate's executable
contract export produced 8,659,599 bytes with SHA-256
`b99c0ee5e62cd0afc1fef72bb82626b860959c14d8330fb3b8ece7ea009753d5`
and 83 operations. A structural walk counted 206 `int64` integer occurrences,
3,066 `uint64` integer occurrences and 409 nullable `uint64` occurrences;
these are occurrences in the exported document, not distinct semantic fields.
The export also contains 623 `oneOf` and 14 `anyOf` occurrences, 401 `$ref`
occurrences and both float and double schemas. `QueryValue` is a tagged
union with recursively referenced list and map members. Consequently a
repair must derive numeric treatment from the operation schema, preserve
floating values, handle recursion and discriminated unions, and coordinate
generated TypeScript types with both request and response serialization;
matching arbitrary field names is insufficient. This walk is a mechanical
inventory, not a complete manual review of the generated 52,363-line
TypeScript file or an implementation of lossless integers. The export is
retained only at `C:/Users/jessa/AppData/Local/Temp/rrflow-f06-openapi-int64-analysis-20261107.json`.
The first journal edit attempt failed because Python used Windows cp1252 to
read UTF-8; the corrected explicit UTF-8 edit succeeded. The defect remains
open; no SDK or alpha qualification changed.
