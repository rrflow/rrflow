# C-07f: settle APFS data and metadata pressure together

**Status:** active implementation evidence; native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-apfs-joint-pressure`
**Owner:** OBJ-02 / C-07f / POAM-035 / PR129

## Retained failure and brief

Driver: current Codex session. [PR129](https://github.com/rrflow/rrflow-development/pull/129)
received the joint-pressure brief before edits. Starting revision `689ade8246536d3e933e056f5c4c47b491207b05`,
tree `4ff568191a48b5d6cdb7695d122e2dce7a4e1855`.

[Run 40162479866, macOS job 121076741324](https://github.com/rrflow/rrflow-development/actions/runs/40162479866/job/121076741324)
passed 29 native cases before the third repetition's existing-project segment
case failed. It created 2,172 filler entries, charged 6,672,384 metadata bytes
and performed 17 data refills. After the final drain, no selected native error
arrived; a later admission error reported 5,939,200 available bytes. The
60-second event deadline expired. Cleanup detached the image and retained the
fixture. This is failed qualification, not a timeout to extend.

Raw log: `target/pr129-native-40162479866/macos-121076741324.log`, 575,929 bytes,
SHA-256 `f847ae0277ef4e5f6c602cf8988df8ee0c4ab2dea3d9c1f7e95a278540f23e32`.
Earlier passing receipts remain bound to their revisions. The candidate
returned to draft; dependent PRs remain draft and no failure is bypassed.

## Canonical change and traceability

Only `crates/adapters/rrflow-cli/tests/support/capacity.rs` changes behavior.
The prior metadata pass plus one data drain moves into `settle_pressure`,
which requires a complete round with neither more data bytes nor more owned
metadata files before returning. Every round shares the original 25-second
start time and original byte/8,192-entry allowances. Existing budget helpers
and native error propagation remain owners. This repeats fixture filling,
never the selected engine operation or a failed test case.

`Filler::fill_metadata` resets its native error observation per round and
continues unique registered names from the existing file count. It cannot
reuse an earlier ENOSPC to satisfy a later round or overwrite an earlier file.
Linux and Windows retain their existing category ordering; aggregate entry
limits and ownership-based cleanup remain unchanged. No native error,
recovery, ACK ledger, workload, production engine or public contract is relaxed.

Replacement map: same-file single-pass APFS orchestration -> bounded joint
rounds; existing data/metadata helpers and their tests remain. Two callback
regressions characterize cross-category progress, stable termination and
no effect after expiry. Complete native repeated CI remains the final oracle.

Read in full: capacity.rs, capacity_apfs.rs, capacity_observation.rs, prior
WAL-pressure journal and current diff; inspected installed native pause/error
observation and segment create/admission ordering, the retained failed log,
README, C-07/OBJ-02/POAM035 and change-authoring procedure. Created: this
journal. Supporting changes: POAM035 link and generated navigation/inventory.
No historical journal or roadmap status is rewritten.

Research/trace decision: [Apple's APFS reference](https://developer.apple.com/support/downloads/Apple-File-System-Reference.pdf)
remains the primary allocation-format source; no proprietary allocator
algorithm or causal timing is claimed. The failed native observation justifies
requiring both categories to settle. Existing bounded filler/WAL diagnostics
are retained; no new runtime instrumentation or copied upstream code is needed.
A quota substitute was not introduced: the owned physical image and selected
native capacity-error/recovery oracle remain.

## Checks and limits

`cargo test -p rrflow-cli --test installed_crash --features crash-test --locked settle_pressure -- --nocapture`
first failed both callback regressions against the modeled single-pass
orchestration; both passed after the bounded-round correction. These are
scheduling/budget regressions, not a local reproduction of proprietary APFS.

Native macOS qualification, complete candidate/merge-group CI, full sustained
profiles, descriptor-budget repair and the 24-hour soak remain open. No
broader engine suite is repeated locally for this fixture-only change;
candidate CI still requires all partitions. Stop for widened limits, wrong or
missing native failure, source/test loss, unowned effects or damaged ACK
recovery. Checklist: pre-effect brief, exact retained failure, one source owner,
replacement test map, primary-source/trace decisions, focused then owning
checks, immutable journal and explicit qualification limits. C-07 and POAM035
remain open; alpha status is unchanged.

Local Windows verification passed:

- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked`: 18 passed, two existing dedicated helpers ignored, 15.20 seconds.
- `cargo clippy -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --locked -- -D warnings`: passed.
- `cargo fmt --all -- --check`, `python scripts/ci/check_documentation.py`, `python scripts/ci/check_workflow.py` and `git diff --check`: passed.

The complete source change and evidence diff were reviewed. Navigation and
inventory were regenerated from the owning sources. `render_navigation.py --check`,
`build_execution_inventory.py --check` and staged whitespace validation passed.
Native evidence remains pending.
