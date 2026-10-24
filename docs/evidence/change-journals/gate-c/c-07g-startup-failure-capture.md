# C-07g: retain bounded startup failure diagnostics

**Status:** active implementation evidence; native growth acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-startup-failure-capture`
**Owner:** OBJ-02 / C-07g / POAM-035 / PR117

## Failure and bounded brief

Starting revision `3f84d1ce1fe7b5afd871be2583c70659ccc618ab`, tree `d5d1c970895a234049c942d717585693ba9d0978`.
The [PR117](https://github.com/rrflow/rrflow-development/pull/117) brief bound
this supporting qualification correction before edits. The
[backup/cache comparison](c-07g-backup-checkpoint-cost.md) retained a failed
10k run: every corpus write and independent readback completed, but first
forced restart produced only `bounded ordinary startup: Disconnected`.
No measured segment or actual 24-hour soak started. The engine's exit cause
was not captured; this repair does not establish or fix that cause.

`tests/support/process.rs::Process::finish` previously returned on the first
stdout-capture error before collecting stderr. `Estate::start_for` panicked
before calling finish when the readiness channel disconnected. The same
bounded capture now drains both streams, preserves capture failure, reaps the
owned child and reports status plus at most the last 2 KiB of captured stderr.
Captured stdout is excluded from failure diagnostics. Startup failure calls
this existing capture after terminating the exact owned process. Successful
output, deadlines, stream caps and failure acceptance remain unchanged.

Scope: `crates/adapters/rrflow-cli/tests/support/process.rs`,
`tests/installed_reopen.rs`, `tests/lifecycle_process.rs`, this record,
existing backup/POAM035 links and generated navigation/inventory. No production
runtime, new process manager, retry, storage identity, timeout or profile change.
Stop on weakened rejection, unbounded output/wait or lost child ownership.

## Research, traceability and review

Complete implementation files read: shared process capture, installed reopen
corpus and lifecycle-process tests. Also read sustained lifecycle backup/reopen
flow, native launcher/progress receipts, README/AGENTS, C-07, POAM035 and the
change-authoring routine. This is a local failure-capture defect; no external
algorithm or dependency is adapted. Existing capped process diagnostics are
the debugging boundary. Captures are from test-owned synthetic estates, not
user projects. The old early-return behavior maps directly to the same finish
method; deadline/flood/readiness/finite-output tests characterize its limits.

## Verification and limits

- First oracle: `cargo test --locked -p rrflow-cli --test lifecycle_process --jobs 6 -- failed_startup_retains_bounded_stderr_and_reaps_the_child --exact --nocapture`
  passed in 0.29 seconds on Windows. A real child exits without an announcement,
  emits over 3 KiB stderr, and its terminal cause survives while the prefix is
  excluded and its owned process is reaped.
- `cargo test --locked -p rrflow-cli --test lifecycle_process --jobs 6`: 11 passed,
  one explicitly invoked child-helper ignored, 2.02 seconds. Deadline, flood,
  invalid readiness, normal output, executable staging and response-loss bounds
  remain enforced.
- `cargo test --locked -p rrflow-cli --test installed_reopen --jobs 6 -- sustained::installed_capacity_workload_smoke --exact --ignored --nocapture`:
  passed in 8.58 seconds. Records/relations/vectors, mixed write transitions,
  backup, reader ownership drain, kill/reopen and receipt replay remain proved
  for this small corpus only.
- `cargo clippy --locked -p rrflow-cli --test lifecycle_process --test installed_reopen --jobs 6 -- -D warnings`: passed in 21.23 seconds.
- `cargo fmt --all -- --check`, documentation policy and generated navigation checks passed.
- Full authored diff review confirms no runtime/budget changes. Execution inventory and staged whitespace checks are regenerated before commit.

Native Linux 10k reproduction with repaired capture, complete candidate CI,
full capacity profiles and the actual 24-hour soak are not yet run/passed.
The original startup failure stays unresolved under C-07/POAM035. All code
changes here are qualification support; no alpha or complete-engine claim.

## Change checklist

- [x] Bind owner, observed failure, exact scope and stop conditions in PR before edits.
- [x] Inspect actual capture/reopen behavior and preserve failed native receipts.
- [x] Keep one bounded owner and fail closed after collecting diagnostic evidence.
- [x] Run the smallest real-process oracle and the affected capture/installed flow.
- [x] Finish strict affected checks and review the full authored diff.
- [ ] Pass exact native candidate CI before landing.

Roadmap and POAM lifecycle status: unchanged, open. No new version or release.
