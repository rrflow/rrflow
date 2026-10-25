# C-07: compaction shadow work and installation lease release

**Status:** active implementation evidence; full-corpus and candidate acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07-final-growth-candidate`
**Owner:** OBJ-02 / C-07c,g,h / POAM-035; D-01 installation-recovery prerequisite

## Brief and integrated source

[PR126](https://github.com/rrflow/rrflow-development/pull/126) carried the
change brief before implementation. It starts from public main
`94d0567fb865fb49a90aaeeef4b5861707c9f8a7`; its empty opening commit is
`14b76cc258f801915815bd05df84db9067e999d9`. Ordinary merges preserve PR123
`1880734887b1624247f623bc8966be8946912fb7`, PR124
`018e2ce12b19b97e7cfeb6fa300e2252d4e02b7a` and PR125
`ae93174f0dda83609935dea7bdf78a7e139f26fa`, including their source and journals.
Only additive C-07/POAM links and generated inventory conflicted; both owner
paragraphs were retained and navigation/inventory regenerated. No runtime/test
source conflict occurred. The PR's traceability table binds each input to its
existing implementation, characterization tests and owner; ancestry alone is
not acceptance.

The new implementation stays in the existing compaction, installation lease
and installed campaign boundaries. No format, public model, configuration,
request/maintenance deadline, corpus size, native test or resource limit is
changed. Full candidate CI and qualifying native campaigns remain required.

## Retained failures

The full Desktop preflight on tested revision
`7b6e56a5f2228078f309a124c3bada94cc29e9cd`, campaign
`22d4fc3911f49359b4618d0c1b9f22854f3b2650dadd0091d0f5ebfc3afc063c`, failed
after 55,047 acknowledged corpus records and 55,104 recorded intents. The
reported error was `corpus write deadline`; the operation limit remains
30 seconds. Total launcher elapsed time was 4,062,190 ms, exit 101, with
cleanup confirmed. No workload segment or actual 24-hour soak completed.

The terminal observation recorded 602,894,336 resident bytes under the
2-GiB engine cap, 696 segments / 1,539,232,271 segment bytes, 617 automatic
compactions, one failed compaction, 88 flushes and four debt segments.
Compaction input/output counters were 13,604,646,633 / 13,492,746,997 bytes.
Segment I/O reported 27,111,813,840 bytes; OS read bytes were 7,327,744 and
OS written bytes 22,849,536,000. These are distinct boundaries, not interchangeable
physical-read measurements. There were no denied source allocations. This
observation motivates compaction diagnosis; it does not establish the sole
cause of the timeout or prove freedom from every memory-pressure effect.

Raw bounded artifacts were copied without rewriting to operator
`target/c07-final-integration-evidence/preflight-7b6e56a5/`:

| Artifact | SHA-256 |
|---|---|
| result.json | `b08b524b35ec2cbcf4f55678e1db2a79fc1f132f94b34c3c98ba7f52778c2499` |
| launch.json | `5ce5c22eba1d85be8796bafe6ac9c2657d2534178ec0c295885f89f14a6a9f9b` |
| launcher.stdout.log | `754a03ff442e1a9ec18ed588110ff1f19301322fbf5693e0aa452c46b6e703f0` |
| launcher.stderr.log | `7daee8dde5e9e7b20cb12b1b4c086304c61d2e432448ab5986357f91789ba6c5` |

PR123 [run 40132112852](https://github.com/rrflow/rrflow-development/actions/runs/40132112852)
finished with all three installed crash/capacity jobs and all three 100-reopen
jobs passing. Its only failing substantive job was optional `rrd-engine`:
`interrupted_apply_recovers_at_every_durable_stage_without_duplicate_authority`
failed at `installation_recovery.rs:231` with installation-lease contention.
There were 173 passed, one failed and two ignored engine unit cases in that job.
The exact job log SHA-256 is
`5c48ecf74b3e147f0d82dbf39afe587c180948fa7444a5319e373b762a8bd308`.
The overall gate failed; the successful native jobs cannot qualify PR126.

## Changes and failure oracles

`rrd-lsm/src/database.rs::merge_compaction_candidate` previously scanned
unselected segment ranges for every compacted key, even when retained live
history made the answer irrelevant. `retain_compacted_key` now calls that
reachability probe only for tombstone-only pruning. Empty output is discarded;
retain-all mode and live/protected values survive without the probe; pruning
still requires the same full unselected-range decision. No snapshot or
tombstone reachability rule changes.

Three focused unit cases characterize that boundary. Against the extracted
old eager behavior, the live/protected and empty/unpruned cases failed on
forbidden shadow access, while tombstone reachability passed (two failed,
one passed). After the lazy change all three pass. Timing thresholds are not
used as a substitute for these structural work and semantic assertions.
Existing physical compaction/snapshot/reopen tests remain required, as does
the full installed corpus to determine whether this removes the observed
growth timeout.

`rrd-engine/src/engine/installation/lease.rs::InstallationLease` now explicitly
unlocks during owner drop, matching the existing manifest-writer practice.
It still retains the empty inode and denies nonblocking contenders while
owned. The Unix regression duplicates the locked description, verifies live
exclusion, drops the original owner, admits its successor, then closes the
old duplicate and verifies that the successor still excludes another owner.
No retry, sleep or lease timeout masks contention.

The Linux [flock manual](https://man7.org/linux/man-pages/man2/flock.2.html)
documents shared lock ownership by duplicate open-file descriptions. An
independent native Linux characterization confirmed that close-only release
denies immediate reacquisition while a duplicate remains, explicit unlock
admits the next owner, and closing the old duplicate does not release that
new owner's lock. This proves the mechanism, not that a duplicate was captured
in the original CI failure. The real Rust regression and installation recovery
suite provide the implementation oracle. No upstream source is adapted.

CLI `tests/support/sustained.rs` now stops and collects the owned server after
the supplemental setup-failure observation, then retains the original private
estate through the existing `OwnedDirectory` helper. It emits only the retained
path/privacy marker and still returns the original setup failure. It does not
upload credentials or treat retained state as completed acceptance. Successful
campaign cleanup is unchanged.

## Observed-process fixture input race

While this package was being validated, PR125's native Ubuntu job
`121000145879` failed `observed_events_preserve_complete_lines_and_both_streams`
at `lifecycle_process.rs:68` with `BrokenPipe`. The raw job log is retained at
operator `target/c07-final-integration-evidence/pr125-native-crash-121000145879.log`,
SHA-256 `4a346e808fa299184691e500fd28535fd7d9da4826021ef8f1154549fc9ad1af`.
The existing fixture could exit before `Process::start_observed` wrote the
mandatory private input. The PR brief was amended before the repair to bind
this additional source and its oracle to the same installed-harness boundary.

`tests/lifecycle_process.rs::observed_fixture` now explicitly marks observed
children, and `owned_child` reads and asserts the exact four-byte `null` input
before emitting output or exiting. The queue-overflow case emits 64 bounded lines after input admission into its
32-line queue; libtest startup lines cannot trigger that intentional failure
before admission. Ordinary fixtures are unchanged. Production
`support/process.rs` retains its existing input-error propagation, process
cleanup, output/queue bounds and deadlines. The complete feature-enabled
`lifecycle_process` suite is the acceptance oracle; no sleeps or retries hide
the race. This additional source was read together with the production input
write and monitor/cleanup paths. It replaces no production implementation.

## Acceptance, review and remaining work

Source review follows the C-07 owner, POAM035, change-authoring/traceability
procedure, the component integration journals, compaction selection/merge and
limits, the installation apply/lease/recovery/concurrency paths, the manifest
guard, and installed workload/process/owned-directory cleanup. Changes are
confined to the three implementation files above, their inline tests, the
observed subprocess fixture repair, this
journal, C-07/POAM links and generated discovery. The complete candidate also
preserves the three integrated inputs listed above.

Validation is recorded in operator
`target/c07-final-integration-evidence/validation/commands.json` and bounded
per-command logs. `cargo fmt --all` passed. With the shared operator build
cache and dev/test debug info disabled, `cargo test -p rrd-lsm --lib --locked
compaction_shadow_tests` passed all three regressions in 25.656 seconds after
the expected baseline failure. `cargo test -p rrd-lsm --all-targets --locked`
passed in 242.687 seconds, including the fault-injection/compound-recovery
sweep, physical compaction, snapshot and reopen cases. The inline test module
was subsequently placed after production items; its code is unchanged.

`cargo test -p rrd-engine --lib --all-features --locked` passed: 172 tests,
two explicitly ignored, 165 seconds including compilation. The Windows run
does not select the Unix-only duplicated-description regression.
`cargo test -p rrflow-cli --test installed_reopen --locked` passed: 12 tests,
seven explicitly ignored native campaigns, 93.594 seconds including compilation.
The crash-test command-attribution regression passed (one test), as did the
five filler allocation/time/metadata regressions. The first observed-fixture
draft incorrectly expected a newline from `start_observed`; two fixture tests
failed with `UnexpectedEof`/unsuccessful child status. Source inspection showed
only `start_controlled` appends that newline. The fixture was corrected to the
actual four-byte observed input, preserving both protocols; the corrected
fixture suite then passed all 14 tests (one child-fixture entry intentionally
ignored), 58.282 seconds including compilation/lock wait. Strict
`cargo clippy -p rrd-lsm -p rrd-engine --all-targets --all-features --locked --
-D warnings` passed in 88.718 seconds. `cargo clippy -p rrflow-cli --test installed_reopen --test installed_crash
--features crash-test --locked -- -D warnings` passed in 41.766 seconds;
its `--test lifecycle_process` counterpart passed in 39.047 seconds.
`cargo fmt --all -- --check` and both candidate-admission/CI-policy checks
passed after the fixture correction.
Native Linux installation tests, full candidate CI, exact-source preflight and
qualification remain pending; this record does not claim their success.
Their exact source-bound results must accompany candidate review. No previous
revision's completed tests qualify this candidate's runtime campaign.

Repository controls passed before the repair commit:
`python scripts/ci/check_documentation.py` (516 records),
`python scripts/knowledge/render_navigation.py --check` (515 nodes, 2,146 edges),
`python scripts/ci/build_execution_inventory.py --check` (1,613 records),
`python scripts/ci/test_candidate_admission.py` (eight tests),
`python scripts/ci/check_workflow.py`, and `git diff --check`.
The navigation and execution inventory were regenerated after journal edits.

Checklist: one OBJ-02 prerequisite and pre-edit PR brief; explicit source/test
traceability and normal merges; retained original failures; primary native-lock
reference; lazy semantic-preserving work decision; private failure retention;
first-failure oracle; full package/candidate validation distinguished; no
acceptance, resource or deadline relaxation. Stop for undeclared scope,
changed durable semantics, a weakened oracle or a source conflict without a
reviewed replacement. C-07, POAM035 and alpha remain open.

Linux preparation on initial repair test merge `d429b1ff` first stopped before
compilation because the offline cache lacked locked crate `ahash 0.7.8`.
The operator preparation now fetches locked Linux-target dependencies once,
then returns to offline tests/builds. This is build preparation, not a runtime
fetch or a workload failure. No campaign or qualification was launched from
that initial revision before the fixture scope amendment.

PR123 and PR125 were closed as superseded after the normal merges and scoped
source comparison; their failed native/engine evidence remains retained.
PR125's obsolete CI was cancelled after its fixture failure was captured;
PR124's useful native jobs were allowed to finish. The integrated reusable
workflow also retains PR123's `serde`/`sha2` kernel dependency assertion, and
the capacity fixture combines PR124 attribution with PR123 metadata refills;
these are intentional preserved behaviors, not byte-identical inputs. No
closed input PR is represented as merged or accepted.
