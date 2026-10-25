# C-07f: recover bulk writes after capacity probes

**Status:** active implementation evidence; native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-fill-write-recovery`
**Owner:** OBJ-02 / C-07f / POAM-035 / PR129, integrated through PR114

## Failure and bounded change

Starting revision `f1b6bd851fffac703171e1b3e2e3aa5f2a8a740d`, tree
`436461c944ba78d046bf9f6d6305c841eb2c6757`. The current integration session
recorded the remediation brief in [PR129](https://github.com/rrflow/rrflow-development/pull/129)
before editing. [PR114 run 40167630906, macOS job 121092136271](https://github.com/rrflow/rrflow-development/actions/runs/40167630906/job/121092136271)
passed 24 native capacity cases, then failed the original 25-second disposable
fill bound inside `fill_data`, called by joint pressure. No selected engine
operation was resumed in that failed case. The retained receipts have zero
unresolved effects and violations; the candidate nevertheless failed.

Raw log: `target/pr114-native-40167630906/121092136271.log`, 499,793 bytes,
SHA-256 `debecb43a85b0876a8b4d16a9efdeb8eeb074ea645d67cb5cfeeb2ed5716075f`.
This failure does not erase earlier successful native runs, nor do those runs
qualify this revision.

`crates/adapters/rrflow-cli/tests/support/capacity.rs::fill_data` reduces
requests after native capacity errors. Previously a successful one-byte write
left requests at one byte until another barrier. If allocation became available,
the helper could drain megabytes with millions of syscalls. It now restores
the existing 64-KiB request after progress. Further capacity errors still reduce
the probe to one byte; settlement and a subsequent native one-byte failure
remain mandatory. The existing timeout now reports bounded bytes, write-call
count, request size and elapsed time. No paths or payloads enter that diagnostic.

Traceability: request scheduling is replaced in the same helper. Native error
classification, synchronized exhaustion, shared time/byte budgets, 8,192-file
limit, metadata pressure, engine pause/resume, ACK ledger, recovery and cleanup
remain unchanged. No production engine, workflow, schema or API changes.
The new adversarial writer regression first rejects larger requests, then
exposes one MiB after the first successful one-byte write. It asserts exact
bytes, settlement and native error while limiting write calls to fewer than
128. This proves a generic loop defect, not proprietary APFS causality; the
replacement native matrix is still required.

## Review and evidence

Read: README and objective/Gate C/C-07/POAM035 owner sections, change-authoring
procedure, full capacity.rs, capacity_apfs.rs, capacity_observation.rs, prior
joint-pressure journal, retained failure and the complete source diff.
Changed paths: capacity.rs, POAM035 and generated navigation/file inventory.
Created: this immutable journal. No existing evidence node is rewritten.

Research: local generic Write-loop behavior suffices; existing Apple source
provenance remains applicable to unchanged native calls. No upstream code or
new allocation guarantee is adopted. Resource/debug decision: preserve every
bound and add progress fields only to the existing timeout. Stop for widened
limits, nonowned effects, weakened native/recovery oracles, unrelated source
changes or missing final candidate evidence.

`cargo test -p rrflow-cli --test installed_crash --features crash-test --locked successful_small_write_recovers_bulk_allocation -- --nocapture`
failed against the prior helper with `available allocation drained byte by byte`,
then passed after correction (one test). Local Windows verification passed:

- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked`: 19 passed, two existing dedicated helpers ignored, 25.91 seconds.
- `cargo clippy -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --locked -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `python scripts/ci/check_workflow.py`: passed.

The ignored native-capacity entry requires the owned-image runner; CI invokes
it explicitly. The other ignored entry is the existing sleeping child helper.
Documentation ownership, regenerated navigation (527 nodes), execution inventory
(1,628 records) and whitespace checks passed. The source, evidence, owner-link
and generated diffs were reread; no unrelated paths changed.

Native macOS and Linux qualification are not local Windows results. The complete
integrated candidate and protected merge-group pipeline remain required. No
full sustained profile or 24-hour soak was run; descriptor exhaustion remains
open. C-07, POAM035 and alpha maturity remain unchanged. Checklist: pre-effect
brief, source/test map, deterministic red/green oracle, owner-suite validation,
trace/research decision, retained failed run, immutable record and full diff
review. Final landing is through PR114 only after its exact candidate passes.
