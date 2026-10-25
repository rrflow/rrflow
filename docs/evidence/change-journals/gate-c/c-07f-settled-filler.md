# C-07f: probe capacity after settling filler allocation

**Status:** active fixture repair evidence; native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-settled-filler`
**Owner:** OBJ-02 / C-07f / POAM-035 / PR112

Baseline `0295c406c37a0e8252c18b445cce7ca3e201cbfe`. PR112's expanded brief
preceded edits. The prior [candidate failure](c-07f-allocation-sync-candidate.md)
reports a public ResourceExhausted response without the required native error.
The selected WAL append's admission, private pause/resume, scoped observation
and actual write/sync wrappers were traced; no observer defect was established.
Inspection did find a fixture gap: data fill ended on capacity failure before
synchronizing its final partial interval, or immediately on synchronization
failure. A native filler error alone did not establish post-barrier capacity.
The failed filler length leaves 1 MiB beyond its last exact 4 MiB interval.
That is evidence of the missing barrier, not proof of the macOS failure cause.

`fill_data` now owns the existing bounded write loop in the same test support
file. It attempts synchronization after the final partial interval and probes
again. A successful write invalidates the prior barrier; completion requires
an actual one-byte capacity write error after a barrier with no intervening
successful write. Capacity errors at synchronization trigger that probe;
unrelated native or non-native errors remain fatal. Partial writes trigger
periodic barriers by bytes since the previous attempt, rather than exact
modulo alignment. Each write is capped by the remaining fixture byte budget.
The original 25-second clock, metadata limit, owned-volume admission/cleanup,
nonzero payload, selected engine error and independent recovery ledger remain.
Engine authoritative durability, storage admission and public APIs are unchanged.

Traceability: Filler::fill's inline data loop moves to private `fill_data`;
its real File/sync helper call remains in Filler::fill. This creates no runtime
adapter or alternate capacity authority. Control-flow tests simulate partial
writes and allocation changes after a successful or failing synchronization;
those tests are not evidence of real APFS/ext4/NTFS exhaustion. The existing
Rust/Apple primary-source synchronization research in the
[allocation-sync journal](c-07f-filler-allocation-sync.md) applies; no upstream
code is copied. No new production trace or performance claim is introduced.

Read the complete changed Filler/Volume/data-fill/cleanup functions and tests,
resume_after_fill, capacity_case and the recovery/readback/ledger continuation;
also reviewed installed_crash's invoke/install/serve/controller, WAL append,
filesystem I/O and space admission, private crash observer and transaction
domain scope. Owner reading covered README, OBJ-02, C-07/C-07f, POAM-035,
change-authoring and the two prior filler journals. Authored paths: capacity.rs,
this linked journal, POAM-035 link, generated journal index and file inventory.
No other implementation move/deletion or gate status change.

First oracle: after extracting the original loop unchanged, the new allocation
settlement test failed: consumed 8 bytes, expected 24, no final synchronization.
Command: `cargo test --locked -p rrflow-cli --test installed_crash --features crash-test filler_settles_pending_allocation --jobs 6 -- --nocapture`.
After repair, `cargo test --locked -p rrflow-cli --test installed_crash --features crash-test filler_ --jobs 6 -- --nocapture`
passed all four Windows tests (0.01 seconds): settlement after success/error,
noncapacity error propagation, byte/deadline caps and real-file sync/readback.
The two caught cap/deadline panics are expected assertions, not hidden failures.
An initial command used an empty target directory; that build was explicitly
stopped and the existing target cache reused. No test result came from it.

Checklist: baseline/owner/pre-edit brief, source trace, failing oracle, smallest
repair, bounds/security and research decisions are recorded. Full native
candidate recovery and the 24-hour soak remain open. In particular, this local
regression does not demonstrate that the missing selected native event is fixed.

Owning support checks: `cargo test --locked -p rrflow-cli --test lifecycle_process --features crash-test --jobs 6`
passed 13 tests, with the existing explicitly invoked child fixture ignored
(2.02 seconds). `python scripts/ci/test_native_capacity.py` passed four owned
image/orchestration tests (0.023 seconds). Strict diagnostic-primary/support
lint passed: `cargo clippy --locked -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --jobs 6 -- -D warnings`
(20.18 seconds). Formatting and whitespace checks passed. Full-file changed
function/test and diff review confirms that production I/O wrappers, crash
observation, metadata fill, recovery/readback and limits are preserved.
Native three-platform image/recovery and Unix socket-error checks were not run
on this Windows host; candidate CI must supply them. A green local model is
not native capacity evidence and cannot close C-07 or justify starting the soak.

The first documentation check rejected an accidentally mis-decoded POAM heading
from a Windows-default text read. Restored its exact UTF-8 heading from Git and
reapplied only the intended link before regenerating and checking projections.

Final documentation policy passed (495 statuses / 493 classified coordinates),
navigation passed (17 indexes / 494 nodes / 2064 edges), execution inventory
passed (1585 records), and staged whitespace passed. The authored diff was
reviewed after correcting encoding; no unrelated source change remains.
