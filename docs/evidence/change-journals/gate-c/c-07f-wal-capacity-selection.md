# C-07f: isolate native capacity cases and observe the selected WAL write

**Status:** active local repair evidence; native candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-wal-capacity-selection`
**Owner:** OBJ-02, C-07f and POAM-035
**Pull request:** [106](https://github.com/rrflow/rrflow-development/pull/106)

Baseline `581ce6432c86efbc86aeba11074a27492e44a681`, tree
`d18cb228a4b874cec4e658055c0895e780bda834`, combines fetched development/main
with exact C-07 candidate `30e5804f`. The draft PR recorded the initial brief
before source edits and was refined when installed measurement changed the
repair decision. Unrelated worktrees and changes were preserved.

## Failure and final behavior

[Run 39979675928](https://github.com/rrflow/rrflow-development/actions/runs/39979675928),
candidate `84a283819a60433c76340c90f1985e47fce48db6`, failed the macOS
existing-project `wal_before_write` case. The controller received its pause,
the owned filler returned native code 28 after 252,510,208 bytes, and the
client returned `ResourceExhausted`. No native-error receipt arrived in 60
seconds. The controller then killed the child: that SIGKILL is cleanup, not
evidence of an OS kill. The aggregate gate correctly failed. Ubuntu migration
and macOS queue notices are not that failure.

The runner reused one filesystem across eight exhaustion/recovery/delete
cases. It now creates and verifies a new native image for each case, detaches
it, validates every cleanup target, and removes only its known owned files
before the next case. This excludes prior cases' allocation/deletion state
from the next native failure oracle and retains one-image peak allocation.
Failure aborts later cases, retains the failed image and omits a success
receipt. Mounted, nonempty, symlink or invalid artifact paths fail before
cleanup deletes anything. The private suite receipt is version 2: each case
carries its own filesystem identity instead of asserting one shared identity.
No other checked-in consumer depends on the former top-level identity.

The existing private WAL pause now includes actual payload bytes. Selection,
I/O, admission, sync, native error, timeout and recovery behavior are unchanged.
The installed no-exhaustion characterization commits the same 180-claim
payload, proves stable receipt replay, and reads all records before and after
restart. It observed a first WAL append of **2,660,020 bytes** on Windows.

Initially a minimum-payload selector was explored because the failed macOS
log omitted the selected size. The real installed measurement showed that a
small-first-append hypothesis was insufficient. That uncommitted selector
and its configuration were removed; the final candidate adds no such knob.
This is not proof of the OS-level APFS cause. Candidate macOS CI must still
prove the failure is resolved. Native ENOSPC from the selected operation,
prior ACKs, exact retry, repair and reopened data remain mandatory: API
resource denial alone never satisfies the native oracle.

## Traceability and research

Read README/AGENTS, OBJ-02, Gate C/C-07 plan, POAM-035, CI ownership and
change-authoring. Reviewed complete private control/rendezvous, capacity
fixture/controller, native I/O wrappers, WAL integration tests and new
Rust/Python tests; traced the affected WAL append, engine domain scope,
runtime commit and physical write paths. Reviewed the complete patch and
native CI job. No production implementation moved or deleted. Existing WAL
golden/recovery tests characterize unchanged formats.

Changed paths: `scripts/ci/run_native_capacity.py`, new
`scripts/ci/test_native_capacity.py`, diagnostic `rrd-lsm` crash/installed/WAL
modules and new `tests/installed_wal_selection.rs`, installed CLI capacity
characterization, native CI job, POAM-035, this journal and generated
navigation/inventory. The owning runtime and private-volume controller remain
the canonical boundaries; no parallel harness or storage backend was added.

Consulted Apple's [fsync contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)
and [allocation/flush controls](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fcntl.2.html)
when considering buffering/allocation explanations. They do not prove this
specific failure's cause, and no native flush policy changed on that inference.
No upstream source or algorithm was imported. New diagnostic metadata is a
bounded byte count, not values, paths or credentials. Python tests substitute
only orchestration; they are not evidence of actual image mounting or ENOSPC.

## Validation

| Command | Actual result |
|---|---|
| Initial minimum-selector process regression before its experimental implementation | Failed: old control returned `selection:denied`; 14.12 s. The selector was later removed based on installed measurement. |
| `cargo test --locked -p rrd-lsm --features crash-test --test wal --test installed_wal_selection` on final diagnostic code | 11 passed, no failures/ignored, 6.48 s; includes child entry. |
| `python scripts/ci/test_native_capacity.py` | Four passed: eight distinct image lifetimes/identities, failure retention/early stop, detach-failure retention, cleanup refusal before deletion. No native volume effects. |
| `cargo clippy --locked -p rrd-lsm -p rrflow-cli --features rrd-lsm/crash-test,rrflow-cli/crash-test --lib --test installed_wal_selection --test installed_crash -- -D warnings` | Passed, 13.58 s. |
| Installed payload characterization before removing the unused selector | Passed, 137.48 s including build. First append measured 2,660,020 bytes; acknowledgement, exact replay and reopened records passed. |
| `cargo test --locked -p rrd-lsm --features crash-test -- --test-threads=4` on final code | 211 passed, 0 failed, 2 existing ignored child helpers across 29 targets including doc tests; 107.81 s. |
| `cargo test --locked -p rrflow-cli --features crash-test --test installed_crash installed_capacity_wal_payload -- --nocapture` on final code | One passed; 102.86 s including build (58.52 s test). The selected payload remained 2,660,020 bytes; exact replay and complete readback after restart passed. |

The earlier full `rrd-lsm --features crash-test` suite passed 213 tests, with
two existing ignored child helpers, in 132.44 seconds while the experimental
selector remained. That count is not presented as final-candidate acceptance;
the final owning rerun is recorded in the table. Local logs are `target/c07f-*.log`.

Workflow policy, formatting, documentation, generated navigation/inventory
and diff checks passed. Rewriting the new journal initially introduced a UTF-8
BOM, which made navigation report a missing title; normalization removed it
and the checks passed. No verification failure was suppressed.

No local APFS/ext4/NTFS image exhaustion, full installed crash matrix, capacity
profile, 24-hour soak or alpha qualification is claimed. The exact candidate
pipeline must prove all three native platforms. C-07 and POAM-035 remain open;
no objective, version, maturity, UI or release-gate status changed.

Checklist: pre-edit baseline/PR, reviewed owner/source map, bounded outcome,
first oracle, research/trace/resource decisions, failed hypothesis and changed
decision, focused validation, full source/diff review and linked evidence are
recorded. Exact candidate CI and any merge remain on the pull request.
