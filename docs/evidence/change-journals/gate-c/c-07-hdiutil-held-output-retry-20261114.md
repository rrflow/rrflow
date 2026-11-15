# C-07: retry a busy macOS template create once its helper lets go

**Status:** active remediation evidence; POAM-074 remains open

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07-hdiutil-held-output-retry-20261114`

**Owner:** C-07 in [Gate C](../../../roadmap/rrflow-1.0/gate-c.md); [POAM-074](../../../poam/rrflow-1.0-alpha/poam-074.md)

**Starting revision:** `main` at `833f23466`.

**Where this came from.** The archived branch `agent/hdiutil-held-helper-20261111` (commits `4eaec4d29`, `70e4170f7`, `89911728c`, `81059c9df`) found the cause and fixed it on 2026-11-11, but never landed. Later the same day `c2cc5baa7` changed the rig to create one template per suite, so the branch's code no longer applied. This package ports the part that still matters, by hand, onto the template create. It is recorded in the [absorption ledger](../repository/archive-absorption-20261114.md).

**Cause, carried from the branch's journal.** Run 43017327376 attempt 1 is the evidence. Case 21 failed 4 seconds after the previous detach. Its receipt has `stderr_open: true` and `stdout_open: false`, the native text is `hdiutil: create failed - Resource busy` after `Finishing`, and the runner killed an orphaned `diskimages-helper` when the job ended. hdiutil starts that helper outside the command's owned process group, and the helper inherits hdiutil's stderr. The retry guard refused any create whose output was still open, so the one failure the retry was written for was the one it refused. I did not re-read that run's log; the facts are the branch's.

**Current behavior on `main` before this change.** `darwin_template()` in `scripts/ci/run_native_capacity.py` retried a busy `hdiutil create` only when both output streams had closed (`is_retryable_darwin_create_failure`). The template journal says the same: it "does not prove the busy failure cannot hit the one remaining create".

**Target behavior.** A busy template create whose output a holder still keeps open is released first: the half-made template is detached with the existing owned-image check. The create is retried only after the held output is seen to close, within `HELPER_EXIT_SECONDS` (20). A holder that outlives that bound gets no retry: the original failure is raised, the template is kept for inspection, and the receipt records `holder_outlived_release: true`. A timed-out create is still never retried. Each retry prints `capacity_create_retry` with the attempt, whether a leftover existed, whether output was held, and how long it took to close.

**Changed paths:**

- `scripts/ci/run_offline_lifecycle.py`: `CommandOutputNotClosed` keeps its capture threads and gains `wait_closed`. Without the threads, `wait_closed` always answers no.
- `scripts/ci/run_native_capacity.py`: `is_busy_darwin_create` and `held_output` replace `is_retryable_darwin_create_failure`; the retry loop in `darwin_template` changes; `NativeCommandFailure` and its receipt gain `holder_outlived_release`.
- `scripts/ci/test_offline_lifecycle.py`, `scripts/ci/test_native_capacity.py`: three tests, and the capacity tests capture their own output so simulated retries never appear in a real job's log.
- [POAM-074](../../../poam/rrflow-1.0-alpha/poam-074.md): the wrong file attribution is corrected, and the held-output cause and this remediation are added.

**Adapted from the branch:**

- The retry moved from `volume()` to `darwin_template()` and acts on `capacity-template.dmg`.
- The branch's two capacity tests were rewritten against the template and renamed `test_macos_template_…`.
- `holder_outlived_release` is a declared attribute of the failure instead of a `getattr` default.

**Dropped from the branch, with the reason:**

- `wait_for_device_release`, the `capacity_settle` line and four `DeviceRelease` tests (also the whole of the archived `agent/macos-image-teardown-20261111`). On `main` no create follows a detach, so the race they guard cannot occur in that form. The branch's own preflight weakens the theory: every `capacity_settle` wait was 0 ms.

**Failure oracles:**

- `test_held_output_is_reported_closed_only_after_its_holder_exits` runs a real child that exits while a descendant holds stderr. A short wait answers no while the holder lives and yes once it is ended.
- `test_macos_template_retries_a_busy_create_after_its_helper_lets_go` checks the order: create, detach the half-made template, then a create that starts from no image after the holder let go; one retry line is printed.
- `test_macos_template_keeps_a_busy_create_whose_helper_never_lets_go` checks that there is no retry, no sleep and no retry line, that the receipt has `holder_outlived_release`, and that the template is kept.
- Before and after: with the old guard put back, both template tests fail on assertions (the first sees `Resource busy` instead of the second create; the second finds `holder_outlived_release` false). With the change they pass.
- `test_macos_volume_does_not_retry_an_unfinished_busy_create` is unchanged and still passes, because a record without capture threads proves nothing.

**Acceptance, Windows, 2026-11-14:**

- `python -m unittest test_native_capacity` in `scripts/ci`: 49 ran, 6 skipped, all passed.
- `python -m unittest test_offline_lifecycle`: 29 ran, 7 skipped, all passed. The skipped tests need POSIX process groups.

**Carried from the branch, not re-run:** macOS preflight runs 43030750207 and 43032802682 passed 24 real APFS cases each on the branch's code. Neither hit a busy create.

**Not established:**

- Nothing in this package has run on macOS. The development gate runs the unit tests on Linux; the macOS capacity jobs run in the nightly matrix.
- The retry has never been seen to act on a real busy create. No macOS run has hit one since the template change.
- Whether the real helper exits within 20 seconds of the detach is unknown.
- Why the image service is busy seconds after a verified detach is unknown.

**Research decision:** none. **Trace decision:** a `capacity_create_retry` line on every retry, so frequency and release time can be counted from CI logs. **Roadmap/POA&M change:** POAM-074 corrected and extended; it stays open until repeated macOS runs pass.
