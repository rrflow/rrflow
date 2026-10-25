# C-07f: bounded APFS metadata payload pressure

**Status:** active implementation evidence; native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-apfs-metadata-payload`
**Owner:** OBJ-02 / C-07f / POAM-035 / PR129

## Native failure and brief

[PR129](https://github.com/rrflow/rrflow-development/pull/129) received the
updated bounded brief before effects. Starting revision
`997ea0d0ab7d9448649aabe7322730c61533ea5d` passed six native macOS capacity
cases in [run 40159526278, job 121067972267](https://github.com/rrflow/rrflow-development/actions/runs/40159526278/job/121067972267),
then existing/manifest_before_create reached the 8192-file cap without a
metadata capacity error: 247463936 data bytes, 64 refills, 5657 ms. Native
cleanup succeeded; the fixture was retained. This disproves reliable metadata
exhaustion from the previous volume-barrier implementation alone.

Raw log: `target/pr129-native-40159526278/macos-121067972267.log`, 283882 bytes,
SHA256 `4de34f70551f25195098e7cfdf4a6dc32b333dd3d1ad61afaef4652551cbc9a2`.
The PR returned to draft; no failure is relabeled as success.

A separate run of accepted main 90373e90, launched to populate default-branch
build caches, also failed the old WAL native-event oracle on macOS. Its raw
log `target/main-903-macos-failure.log` is 250806 bytes, SHA256
`03285001eb76e517899fbe25a7bb0f2f6f60fd201c642c0b6192f735a726cad1`.
That cache operation is not qualification and is not rerun until green.

## Source ownership, research and correction

The existing `capacity.rs::Filler::fill_metadata` owns pressure on disposable
files inside the admitted private volume. It now gives each APFS filler file
one private 3072-byte attribute through its open descriptor. The new native
helper stays in `capacity_apfs.rs`; it uses create-only `fsetxattr`, never
replacing an existing attribute or modifying an engine-owned file.

Apple's [APFS format reference](https://developer.apple.com/support/downloads/Apple-File-System-Reference.pdf)
describes embedded attribute values and a 3804-byte maximum. Its
[descriptor attribute API](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsetxattr.2.html)
defines descriptor ownership, the create-only option and native errors.
These support a bounded metadata-pressure technique; they do not prove how
the current proprietary APFS allocator will behave under this workload.
The candidate's real native engine error/recovery remains the decisive oracle.
No upstream implementation or filesystem model is copied into the engine.

`allocate_metadata_payload` charges each attempted payload before the syscall,
including uncertain/error outcomes. Its allowance shares the original volume
byte bound with data; subsequent data refills subtract the metadata charge.
Attribute ENOSPC is pressure only: it cannot stand in for a failed create or
the selected engine syscall. Native creates continue, while unrelated errors
propagate. The 25-second fill deadline, 8192-entry cap, native-error identity,
recovery, replay and acknowledged-history assertions remain unchanged.
Bounded diagnostics include charged metadata payload bytes. Cleanup still
removes only the registered disposable filler files and uses the same limits.

The workflow now repeats all four APFS boundaries, both modes, three times:
24 additional cases after the eight-case full matrix. Its existing 15-minute
repetition deadline remains. The exact required step name is updated in the
acceptance validator and missing/skipped-step negative test together.

Read in full: capacity.rs, capacity_apfs.rs, shared containment helper and
native-capacity controller tests, run_native_capacity.py, owning repair
journals and POAM035; reviewed the relevant workflow and acceptance validator
and tests, README and change-authoring procedure. Changed source: those two
capacity files, workflow, acceptance validator/test. Evidence: this immutable
journal, POAM035 link and generated navigation/inventory. No engine source,
public API, product version, old journal or resource cap is replaced.

## Validation

The retained native failure is the initial failure oracle. The new focused
payload test passes: native errors are preserved, denied payloads cannot
reach the OS, and subsequent data writes honor the shared allowance. Expected
assertions are caught explicitly; they are not native qualification.

- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked metadata_payload_is_charged -- --nocapture`: 1 passed.
- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked`: 15 passed, 2 existing dedicated helpers ignored, 18.74 seconds.
- `cargo test -p rrflow-eval --all-targets --locked`: 41 library, 3 provider and 11 storage-contract tests passed; other targets have no tests.
- `cargo clippy -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --locked -- -D warnings`: passed.
- `cargo clippy -p rrflow-eval --all-targets --locked -- -D warnings`: passed.

Local tests run on Windows. The native macOS attribute test checks actual
readback, create-only duplicate rejection and unchanged file length; it is
compiled/executed only by native macOS CI and has not run locally. Full native
capacity, repeated cleanup and candidate checks remain pending. No full
capacity profile or 24-hour soak is running or qualified by this package.

Checklist: pre-effect brief; retained exact failure; canonical source and test
map; primary-source research; bounded diagnostics; focused then owning tests;
full diff review; explicit omissions; immutable linked journal. Stop for a
widened bound, nonowned effects, suppressed unrelated errors or missing selected
native event. C-07 and POAM035 remain open; alpha status is unchanged.

A format check initially identified the shorter acceptance-step call wrapping;
`cargo fmt --all` corrected it. No behavioral change followed that diagnostic.

The documentation check caught a Windows text-decoding mismatch in the POAM
heading. The original UTF-8 bytes were restored and the supporting insertion
reapplied with explicit UTF-8; no owner title was intentionally changed.

Final `cargo fmt --all -- --check`, documentation ownership, workflow policy,
`render_navigation.py --check`, `build_execution_inventory.py --check` and
working-tree whitespace checks passed after those corrections. Generated
indexes and inventory were refreshed from their owning sources.
