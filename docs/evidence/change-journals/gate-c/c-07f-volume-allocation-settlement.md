# C-07f: settle admitted APFS volume allocation before exhaustion probes

**Status:** active implementation evidence; exact-candidate native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-volume-allocation-settlement`
**Owner:** OBJ-02 / C-07f / POAM-035 / PR129

## Brief and first failure

[PR129](https://github.com/rrflow/rrflow-development/pull/129) owns this shared
native fixture correction. Its description was extended before implementation,
then returned to draft. Starting revision is
`d0e7f54c1496bf55c1a0932c6f4d574cae21545f`. PR118 contains that revision and
[run 40156063858, macOS job 121057622160](https://github.com/rrflow/rrflow-development/actions/runs/40156063858/job/121057622160)
failed `segment_before_create` in existing mode: 8192 metadata files,
252706816 data bytes, 64 refills, 9311 ms, with no metadata capacity error.
The file-count oracle correctly failed. The complete raw job log is retained
locally at `target/pr118-native-40156063858/macos-121057622160.log`, 276718 bytes,
SHA256 `130f1c538136db405f11f8e4719c49ef0938063287a6f82c21f83b0d4013e8c1`.
PR118 was returned to draft after preserving that failure. PR129's preceding
run 40154271269 was cancelled on return to draft while the final Windows
reopen build remained pending; its passing native jobs are historical evidence
for d0e7f54c only, not qualification of this correction.

The current filler waits for individual file allocation. The target adds a
waited volume allocation barrier through a descriptor opened on the already
admitted private APFS root. The same `fill_data` owner subsequently probes and
drains released allocation. Metadata-phase refills use that barrier too. A
capacity error from file synchronization must still permit volume settlement;
an unrelated file error stops immediately, and a volume error is preserved.
Linux and Windows retain their existing file synchronization behavior.

Traceability: `Filler::fill`, `Filler::fill_metadata`, `fill_data` and
`fill_remaining_data` remain the sole fixture implementation. The extension
lives in `tests/support/capacity.rs` and macOS-only `capacity_apfs.rs`; no engine
implementation moves or disappears. Existing delayed-allocation, cumulative
byte/deadline and native error tests remain. The 25-second fill clock, 8192
metadata entry cap, aggregate byte cap, selected syscall identity/native error,
ACK history, atomic readback, retry and cleanup assertions are unchanged.
No provider API, production durability behavior or product version changes.

The repeated macOS CI step now covers both WAL and segment pauses for three
fresh/existing pairs. Its exact step name is required by the existing
`rrflow-eval` acceptance validator; missing/skipped repetitions remain failures.
The full eight-case native matrix still covers manifest and CURRENT cases.

## Research and diagnostics

Primary sources reviewed: Apple's [filesystem-control constants](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/fsctl.h),
[volume-control handler](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/vfs/vfs_syscalls.c),
and [Libc ABI declaration](https://github.com/apple-oss-distributions/Libc/blob/main/include/unistd.h).
`FSIOC_SYNC_VOLUME` with `FSCTL_SYNC_WAIT` requests waited volume synchronization.
The small ABI declaration uses the SDK request encoding and a live owned root
descriptor. No Apple implementation is copied. Ordinary unit tests do not
synchronize the developer volume; the macOS negative test uses a socket with
no filesystem. Positive native behavior belongs to the admitted-image corpus.

Missing volume settlement is an implementation distinction and a hypothesis
for the observed intermittent count failure. Neither research nor mocked
control flow proves the native cause repaired. Current bounded diagnostics and
the retained job log suffice; no engine contents or credentials are added to
output. Stop for a widened cap/deadline, unsupported native call treated as
success, external mount effects, missing selected native error, or damaged ACK
recovery. Native failure must remain a failure, never retry-until-green.

## Review scope

Read owners: README, change-authoring procedure, POAM035, prior metadata-pressure
journal. Read implementation: complete capacity.rs, owned native-image runner,
macOS control ABI/handler, current workflow native lanes, acceptance validator
and its native-step tests. Changed paths are those fixture/helper and workflow/
acceptance files, this journal and POAM035's supporting link. Navigation and
execution inventory are regenerated from their owners. No old evidence node is
rewritten; the previous cancelled/passing/failing runs retain original identity.

## Validation

Windows, locked dependencies, dev/test debug symbols disabled:

- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked filler_ -- --nocapture`: six passed. The new nine-combination file/volume error oracle confirms capacity errors still settle, unrelated errors stop, and native codes survive. Expected caught cap/deadline panics are passing negative checks.
- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked`: fourteen passed in 35.03 seconds, two existing dedicated-runner helpers ignored. Fresh/existing domain, publication, reclamation and selected-WAL histories passed. This is not an owned APFS image run.
- `cargo test --locked -p rrflow-eval native_apfs_wal_and_segment_repetitions_cannot_be_missing_or_skipped`: one passed.
- `cargo test --locked -p rrflow-eval --all-targets`: 41 library, three provider and eleven storage tests passed.
- `cargo clippy --locked -p rrflow-eval --all-targets -- -D warnings`: passed.

The initial formatting check reported the expanded step-name call wrapping;
formatting was corrected. Documentation initially rejected the new journal's
missing active/historical status, then detected its stale generated index;
the status and projections were corrected. These failures were not runtime
or native capacity evidence. The initially staged journal had CRLF bytes;
whitespace validation exposed them and the final LF record was restaged.

`cargo clippy -p rrflow-cli --test installed_crash --features crash-test --locked -- -D warnings`
passed. Rust formatting, documentation ownership, workflow policy, generated
navigation/inventory and whitespace checks passed on the final staged tree.
The complete source/helper and changed diff were reread before commit.

Not run locally: macOS compilation, the actual APFS synchronization call,
owned APFS/ext4/NTFS exhaustion and the complete candidate pipeline. They
require their native runner and are not replaced by the Windows component
suite. The twelve repeated APFS cases stop on the first failure; no deadline
or count adjustment is permitted to obtain a pass.

Checklist: owning brief before effects; exact native failure retained;
canonical fixture boundary and source map; primary ABI research with explicit
causal uncertainty; resource and recovery oracles preserved; focused then
owning checks; initial failures and omissions recorded; full diff review and
generated evidence updated. No roadmap or parent POAM status changes. New
candidate CI and normal merge-group success remain required before landing.

C-07 and POAM035 remain open. The separate descriptor exhaustion repair,
full capacity profiles and 24-hour soak are not completed by this fixture work.
