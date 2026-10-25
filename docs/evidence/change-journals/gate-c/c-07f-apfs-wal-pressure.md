# C-07f: include APFS WAL in metadata pressure

**Status:** active implementation evidence; native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-apfs-wal-pressure`
**Owner:** OBJ-02 / C-07f / POAM-035 / PR129

## Failure, baseline and bounded brief

The current Codex session owns this repair in
[PR129](https://github.com/rrflow/rrflow-development/pull/129). Its description
received the bounded repair brief before implementation. Starting revision:
`817a1b8b9c99fbc3dcc1907075333398c1f5f4b8`; tree:
`d4718d9c32c043aee7a52a27579d3d3155b088ec`.

The shared fixture failed in the integrated endpoint candidate PR113 at
`143ef2739d6657a74ddaa9e40ee46d2959dfb617`,
[run 40161414466, macOS job 121073590343](https://github.com/rrflow/rrflow-development/actions/runs/40161414466/job/121073590343).
The existing-project WAL case filled 251,658,240 bytes and reported ENOSPC,
but the selected 2,660,375-byte payload grew the WAL from 2,177,721 to
4,838,128 logical bytes. A later storage-admission error reported 10,317,824
bytes available. The required selected native WAL error never arrived;
the existing 60-second controller deadline expired. Cleanup detached the
owned image and retained the failed fixture. This is not a passed capacity
case or a timeout to extend.

Raw log: `target/pr113-native-40161414466/macos-121073590343.log`, 254,387 bytes,
SHA-256 `d1d2b63efbdfb39a443fb0a0a3feb5a11acedc4693c8bfec4db61c772be035db`.
The same run passed fresh segment and manifest cases while actually consuming
metadata attributes. The manifest case used 2,212 files, 6,795,264 charged
metadata bytes and 17 data refills, then observed the selected native error
and recovered with zero unresolved operations or ledger violations.

The preceding PR129 run 40160626287 passed all 48 native cases (macOS 32,
Linux eight, Windows eight), but its macOS cases did not need metadata
attributes. That result does not clear the later WAL failure. All ten
integrated candidate PRs returned to draft while this common fixture is
corrected. Existing results retain their original source identities.

## Implementation and traceability

`crates/adapters/rrflow-cli/tests/support/capacity.rs` remains the sole owner.
Its extracted `metadata_pressure_directory` includes APFS WAL in metadata
pressure using the native storage root. Disposable entries stay outside the
authoritative WAL directory. Linux/Windows WAL selection and all existing
segment/manifest/CURRENT destinations are preserved.

`Filler::fill` performs a final APFS data drain after metadata creation,
including the last partial interval below 128 files. That drain uses the
original start time and subtracts all charged metadata bytes from the same
aggregate allowance. It cannot restart the deadline or relax a resource cap.
The existing selected-engine native-error identity, recovery, idempotent
replay and exact acknowledged-history assertions remain the final oracle.
No runtime engine, public API, workload payload, storage format, deadline,
8,192-entry limit or external dependency changes.

Source replacement map: the inline directory-selection match moves into the
same file's helper; the new characterization covers the APFS WAL mismatch and
all preserved platform/boundary destinations. Existing cumulative refill,
barrier-error, byte-limit and original-deadline tests characterize the reused
final-drain helper. No implementation is deleted into a fallback lane.

Read in full: capacity.rs, capacity_apfs.rs, capacity_observation.rs and the
current metadata-payload journal; inspected the retained native failure log,
README, OBJ-02, Gate C/C-07 plan, POAM035, change-authoring procedure and native
CI ownership. Changed source: capacity.rs only. Evidence changes: this new
journal, a POAM035 supporting link and generated navigation/inventory.
Historical journals and roadmap completion status are unchanged.

Research: the existing [Apple APFS reference](https://developer.apple.com/support/downloads/Apple-File-System-Reference.pdf)
and [descriptor attribute contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsetxattr.2.html)
remain the primary source basis. The observed native WAL growth establishes
the fixture mismatch; no proprietary allocator algorithm is inferred as fact.
No upstream implementation is copied. Existing bounded WAL/file and filler
signals already identify the failure, so no additional runtime trace or
telemetry surface is introduced.

## Verification and limitations

- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked apfs_wal_consumes_metadata_pressure -- --nocapture`: first failed with the old selector (`None` versus the owned storage root), then passed after correction.
- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked`: 16 passed, two existing dedicated helpers ignored, 15.78 seconds.
- `cargo clippy -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --locked -- -D warnings`: passed.

Tests ran on Windows with dev/test debug symbols disabled. The focused test
proves selection, and existing refill tests prove bound/error handling; neither
proves APFS allocation behavior. Native repeated capacity, full candidate and
merge-group CI remain required. Engine-wide suites were not repeated locally
for this fixture-only change; candidate CI still runs them. Full sustained
profiles, descriptor-budget repair and the 24-hour soak remain unfinished.

Checklist: pre-effect brief; exact failed source/log retained; one canonical
fixture owner; replacement map and red/green characterization; existing
primary research and bounded diagnostics; focused and owning tests; explicit
native omissions; new immutable journal. Stop for widened limits, lost or
wrong-boundary native errors, unowned effects or changed ACK recovery.
C-07 and POAM035 remain open. This is not alpha qualification.

Full source reread and complete diff review found only the described fixture,
regression and evidence changes. `cargo fmt --all -- --check`,
`python scripts/ci/check_documentation.py`, `python scripts/ci/check_workflow.py`
and `git diff --check` passed. Navigation and execution inventory were
regenerated from their owners; `render_navigation.py --check`,
`build_execution_inventory.py --check` and staged whitespace checks passed. No failure was suppressed and no other source behavior was changed.
