# C-07: integrated candidate and native WAL failure attribution

**Status:** active integration and diagnosis evidence; causal repair and native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07-native-wal-candidate`
**Owner:** OBJ-02 / C-07c,f,g / POAM-035 / PR129

## Brief and source integration

[PR129](https://github.com/rrflow/rrflow-development/pull/129) binds the brief
before implementation. Public main is
`f3c04e955fd8851c0fc466c45f008975e9d4c1a1`; opening commit is
`9f81efc5995b45b8ef8ab963a663c041a3a1e327`. Ordinary merges preserve PR128
`53e4ad6bd77dfd4d8fbbd62c448f2cd51e475199` and PR127
`c93a6f27c1fd1699e341dbc135298e270e0dd5d8`; integrated source before new
fixture edits is `af1bf27e` (full identity is retained by Git).
Both POAM-035 paragraphs were kept through the only prose conflict; the
file inventory was regenerated mechanically after staging the resolutions.
Original runtime/client source changes are carried intact, not reimplemented.

The [encoder record](c-07-encoder-working-set.md) retains its six exact-byte
baselines, storage suite and native working-set measurements. The
[readback record](c-07-integrated-readback.md) and
[main-integration record](c-07-readback-main-integration.md) retain the
real-client transaction/relationship/reopen proof. They are component evidence
and are not silently rebound to this integrated candidate.

The unchanged Linux `9957ed39` full-corpus preflight remains a separate
characterization; its running source, owned VM and retained failure estates
were not modified. No completed soak is claimed by this integration.

## Reproduced blocker and current interpretation

[PR128 run 40149841701, macOS job 121039196593](https://github.com/rrflow/rrflow-development/actions/runs/40149841701/job/121039196593)
failed in existing mode after reaching `wal_before_write` with payload
2,660,375 bytes. Filler wrote 247,463,936 bytes and received ENOSPC (28).
The client reported `api-ResourceExhausted`, but no selected native I/O error
arrived within the original 60-second bound. The harness terminated/reaped
the child. Raw completed-job log is retained in operator
`target/c07-pr128-macos-failure.log`, SHA-256
`f2b3f85b26a82a54664fd71ff36d59e0346686a48bada5529a87b918fb2e4a76`.

The earlier [command-attribution record](c-07f-command-attribution.md)
reported the same symptom in fresh mode. The paused WAL call occurs after
space admission and before actual write/sync; the existing observer reports
raw OS failures in those calls. A later admission failure, physical allocation
behavior or incomplete observation must be distinguished. Neither the API
error class nor logical file length proves selected WAL exhaustion.
The existing logs omit the API message and physical WAL allocation.

## New diagnostic and regression boundary

`tests/support/capacity_observation.rs` reads at most 128 owned WAL file
metadata records and emits logical size and Unix allocated bytes, with no
filenames, values, credentials or file content. Non-Unix allocation remains
explicitly unavailable. `capacity.rs` captures before fill, after fill and on
missing selected event, plus at most 512 characters of the public API error
message for the controlled synthetic request. No arbitrary project command,
authentication value or response body is captured. The original selected
native-error identity, 25-second fill, 60-second observation, byte/file caps,
ledger reconciliation, repair, retry and reopen assertions remain unchanged.

The existing macOS installed-crash lane adds three planned fresh/existing WAL
pairs on new owned images after the complete native matrix. Every failure
stops the runner; these are independent regression repetitions, not retries
until success. The other native lanes and all required checks remain.
This diagnostic step is not yet a causal repair and cannot close POAM-035.

Source map: logical WAL-length observation remains in the capacity corpus;
new read-only allocation snapshots belong to that same fixture, not runtime
storage authority. Integrated encoder and public-client destinations stay at
their original canonical boundaries. No ordinary engine algorithm, format,
version, external dependency, resource cap or readiness behavior changes.

## Research and reviewed inputs

Read README, AGENTS, change-authoring procedure, relevant C-07/POAM-035 and
prior attribution/metadata records. Complete function reads covered capacity
fill/resume/case, Estate install/invoke, WAL append/reservation, native file I/O,
disk-space admission and installed diagnostic control/error observation.
The encoder/client deltas, complete new helper and complete change diff were
reviewed. Owner chapters were read in relevant sections, not claimed as full
repository reads.

[Apple allocated-block documentation](https://developer.apple.com/documentation/system/stat/blocksallocated)
identifies 512-byte allocated blocks and filesystem-dependent semantics.
[Apple fsync documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fsync.2.html)
distinguishes file synchronization from stronger persistence. These support
measurement decisions, not a claim that preallocation or deferred reclamation
is already the cause. No upstream code is adapted; no speculative persistence
barrier change is made before attribution.

## Checks and remaining work

Windows checks on the integrated candidate:

- `cargo test --locked -p rrflow-cli --test installed_crash --features crash-test records_real_lengths_without_paths_or_file_contents -- --nocapture`: one passed.
- `cargo test --locked -p rrflow-cli --test installed_crash --features crash-test capacity:: -- --nocapture`: seven passed; native image test explicitly ignored here. Expected panic assertions in negative filler tests passed.
- `cargo clippy --locked -p rrflow-cli --test installed_crash --features crash-test -- -D warnings`: passed.
- `cargo fmt --all -- --check`, `python scripts/ci/check_documentation.py`, `python scripts/knowledge/render_navigation.py --check`, `python scripts/ci/check_workflow.py`, `python scripts/ci/build_execution_inventory.py --check`, and `git diff --check`: passed. Navigation and inventory were regenerated before these checks.

Native APFS, ext4 and NTFS complete capacity matrices and candidate CI have not
run on this revision. The existing Linux sustained workload remains pinned to
its separate encoder candidate; it is not evidence for this integrated tree.

The first attempted tool composition was rejected by the JavaScript parser
before execution; separating patch and command calls corrected it. One full
source glob was invalid on Windows and two inferred file paths were absent;
repository file discovery identified `filesystem/io.rs`. These were inspection
errors, not engine failures. No raw failure evidence was deleted or called green.

Checklist: brief before effects; exact input revisions and integration map;
reviewed canonical boundaries; smallest diagnostic oracle before owning suite;
research/trace/resource decisions; preserved original assertions; generated
navigation/inventory and complete diff review before push. C-07 and POAM-035
remain open. The next native run must identify the failing physical path, then
prove its causal repair and complete candidate pipeline before landing.
