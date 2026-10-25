# C-07f bounded allocation settlement batches

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07f-batched-allocation-settlement`
**Owner:** OBJ-02 / C-07f / POAM-035 native capacity fixture repair

## Brief and baseline

[PR131](https://github.com/rrflow/rrflow-development/pull/131) binds the
preimplementation brief. Public main was
`dd225072bcc639c89b0eef968625cc73303ea998`, tree
`0ee0344e039b94ea23f124bb2a50209fc7263882`; the draft-opening revision was
`3b151ec8bae826ad6443ca356fd53aa9dcbf53e8`. This package replaces allocation
barrier scheduling in the existing disposable native capacity fixture. It
changes no engine storage, authoritative durability barrier, timeout, byte or
entry cap, selected-operation oracle, recovery contract or CI requirement.
Stop for any such expansion or an unowned filesystem effect.

PR130 candidate `8a6752b255cbd9c42636c506de54de06474776f9`, run
[40200131911](https://github.com/rrflow/rrflow-development/actions/runs/40200131911),
macOS job `121189374177` passed six capacity cases, then timed out before
resuming existing-project manifest creation. The shared 25-second clock expired
at 693 metadata files and 260046848 data bytes. The log did not attribute time
among initial data allocation, metadata creation and synchronization. This is
a verified fixture failure, not a demonstrated acknowledged-data loss. The
raw retained log is 286049 bytes, SHA-256
`e1eff71b95fa4b1c0eeef30908cca186f1eb200ef6e859704c8f7132544a43b5`, at
`target/pr130-macos-domain-crash-121189374177.log`. Returning PR130 to draft
cancelled unfinished lanes; their cancelled steps are not additional engine
failures or accepted results.

## Implementation and traceability

`capacity.rs::fill_data` now distinguishes periodic file allocation barriers
from final capacity settlement. Even an exact 4-MiB boundary requires final
settlement and a fresh one-byte denial. Any newly available allocation is
consumed using the existing bulk-write recovery and cumulative limits.
`Filler::fill` waits for the owned APFS volume only at capacity settlement,
while retaining periodic file barriers. Its bounded diagnostic records report
initial data time, barrier counts and barrier time.

`Filler::fill_metadata` delegates scheduling to the new
`capacity_metadata.rs::fill`: settle each 128 created entries, settle a partial
batch or first native create denial, then require a fresh denial with no
intervening create. APFS settlement drains data made available by the metadata
transaction and waits for the admitted volume. Per-file filler fsync is removed.
Metadata payload charging, ownership registration before attributes, joint
pressure convergence, native errno classification, selected engine syscall,
ACK reconciliation and cleanup retain their original owners. Non-APFS callers
retain native create denial and their existing platform-specific evidence.
Noncapacity create or settlement errors propagate. Diagnostics contain counts
and timing, never payloads or credentials.

The replacement maps the old per-file and per-interval synchronization schedule
to these same fixture boundaries. Characterization proves delayed allocation,
partial batches, capacity denial after settlement, noncapacity errors, expired
clock and entry bounds. The exact-interval regression prevents treating an
interval file barrier as proof of settled volume pressure. There is no parallel
fixture, hidden retry allowance or migration lane.

## Research and review

Apple's [fsctl definitions](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/fsctl.h)
and [handle_sync_volume implementation](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/vfs/vfs_syscalls.c)
identify FSCTL_SYNC_WAIT and the waited volume synchronization boundary.
No upstream source was copied. Batching remains a repair hypothesis until
native APFS candidate repetitions pass; deterministic allocator tests cannot
prove operating-system timing. This is fixture scheduling research, not a
change to engine durability or a performance claim.

Reviewed README, OBJ-02 and C-07/POAM-035 ownership, change-authoring and
traceability procedures, CI native capacity commands, the preimplementation
brief and retained failing job. Implementation files read and reviewed:
`crates/adapters/rrflow-cli/tests/support/capacity.rs`,
`crates/adapters/rrflow-cli/tests/support/capacity_metadata.rs`, and
`crates/adapters/rrflow-cli/tests/support/capacity_apfs.rs`.
The complete changed Rust files and diff were reread. New paths are the helper
and this journal; changed paths are capacity.rs, POAM-035 and mechanically
regenerated journal navigation/file inventory. No source moves or deletes.

## Validation and limitations

- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked -- capacity::metadata --nocapture`: four focused tests passed.
- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked -- --nocapture`: 24 passed, two existing owned-native-image tests ignored; Windows, 26.60 seconds. Includes the new exact-interval regression and actual installed crash/publication/recovery cases. Retained output: `target/c07-metadata-local-tests.log`.
- `cargo clippy -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --locked -- -D warnings`: initial manual modulo lint failed; changed to `is_multiple_of`, then passed. Both logs retained under `target/c07-metadata-clippy*.log`.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `python scripts/knowledge/render_navigation.py`: passed, 17 indexes, 608 nodes, 2412 edges.
- `python scripts/ci/check_documentation.py`: passed, 609 document statuses and 607 classified coordinates.
- `python scripts/ci/check_workflow.py`: passed.
- `python scripts/ci/build_execution_inventory.py` and `--check`: passed, 1732 records.

Local checks use the locked shared Windows build cache and debug information
level zero. No owned APFS/NTFS/ext4 image qualification or 24-hour soak is
claimed here. Dedicated candidate runners must execute actual selected native
failures and independent readback. PR130's separately frozen full-size Linux
preflight remains characterization under its own revision; it cannot qualify
this changed harness. C-07 and POAM-035 remain open.

Change checklist: owning outcome and baseline bound before edits; exact scope
and failure oracle published in the draft PR; replacement traceability mapped;
research and diagnostics decisions recorded; focused and owning tests run;
initial lint failure retained and corrected; complete changed-source review
performed; journal and generated navigation provided; native candidate CI and
merge remain required. No roadmap status or alpha maturity change.
