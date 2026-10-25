# C-07f: allocation synchronization for disposable filler files

**Status:** active fixture correction; APFS acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-filler-allocation-sync`
**Owner:** OBJ-02, C-07f, POAM-035
**Pull request:** [112](https://github.com/rrflow/rrflow-development/pull/112)

Baseline `58cabe8cd09e1f5493180c082acbb4fc3a7e990e`, tree
`a70fd8ed8e764478c08daa4652a3182cd435d518`, joins current public main with
exact PR111 `c7959ca5989b6bd6ef3255c4c10e41cdd348e24c`. The draft PR brief
preceded source edits. This package changes only the native capacity fixture
in `crates/adapters/rrflow-cli/tests/support/capacity.rs`, this journal,
POAM-035 and generated navigation/inventory. No product version/gate change.

The first failure is retained in the
[preceding launch record](c-07g-desktop-preflight-launch.md): PR108's macOS
job 120691007445 reached `bounded metadata fill` at the existing 25-second
deadline. The stack identifies the failing step, not its complete timing
cause. Inspection found an unnecessary expensive barrier: every APFS empty
filler file used `File::sync_all`, and bulk filler writes used `sync_data`.
[Rust's Unix implementation](https://doc.rust-lang.org/src/std/sys/fs/unix.rs.html)
selects `F_FULLFSYNC` for both on Apple targets.
[Apple's fsync contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)
distinguishes host-to-device synchronization from hardware-cache persistence.
Disposable filler must consume real OS allocation; it need not survive a
power failure. No upstream implementation is copied or adapted.

`sync_filler_allocation` now selects native `fsync` on macOS, with other
platforms retaining `File::sync_data`. Its private `sync_filler_host` helper
returns the actual native error immediately without retry or fallback. Both
filler call sites use it. Engine WAL, immutable segments, manifests and
publication barriers are unchanged. Per-file metadata sync, the 25-second
deadline, 8192-file bound, byte cap, create_new ownership and cleanup remain.
The selected engine I/O must still report real capacity failure, followed by
complete independent acknowledgement reconciliation and recovery. No syscall
success is accepted as proof of exhaustion. Deadline failures now report
bounded counts, bytes and elapsed time without paths or payloads.

Traceability maps the two filler synchronization calls to the private fixture
helper; engine durability has no replacement destination because it is not
changed. Read the complete Filler/Volume ownership and error paths,
resume_after_fill and capacity_case, native runner and cleanup tests,
installed_crash registration, CI commands, README/OBJ-02/C-07f/g/POAM-035 and
authoring/traceability procedures. The engineering debug skill guided failure
isolation. Existing receipts remain; no new runtime trace or authority.
Stop for altered engine barriers, swallowed errors or weakened capacity proof.

`cargo test --locked -p rrflow-cli --test installed_crash --features crash-test filler_ --jobs 6 -- --nocapture`
passed the Windows regular-file synchronization/readback test in 94.56 seconds
including diagnostic build (test 0.08 seconds). The additional Linux/macOS
test synchronizes a live socket descriptor and requires the native EINVAL
error, which cannot be mistaken for capacity failure. It is not executed by
the Windows check. During authoring, the unsupported-descriptor test was
changed from a character device to a socket matching the documented error
case; no failed runtime check was hidden.

`python scripts/ci/test_native_capacity.py` passed all four existing
orchestration/cleanup tests (0.031 seconds).
`cargo test --locked -p rrflow-cli --test lifecycle_process --features crash-test --jobs 6`
passed 13 tests with one existing explicitly invoked child fixture ignored
(6.95 seconds including build; tests 2.02 seconds).
`cargo clippy --locked -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --jobs 6 -- -D warnings`
passed in 16.02 seconds. Formatting and diff checks pass. Logs are retained
under operator-local `target/c07-filler-sync-{focused,process,clippy}.log`.
The documentation checker rejected the initial journal status because it lacked
the required active/historical classification; the status was corrected before
regenerating navigation/inventory and rerunning the check.

The complete changed functions/tests and diff were reread. Native APFS timing,
the Unix error test and complete ext4/APFS/NTFS exhaustion/recovery remain
candidate-CI requirements; local administrative volume facilities are not
available. This source correction is not proof that the macOS timeout is
resolved. Linux25 remains frozen on c206c270 and continues independently;
Windows compilation overlapped that characterization, so it is not a controlled
performance comparison. Full capacity profiles and 24-hour qualification are
still open. POAM-035 remains open; no merge or alpha completion is claimed.
