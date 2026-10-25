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


### Prerequisite acceptance and normal integration

PR116 exact head `88fbc5b41128371eabff21928b48d84255372e90` passed all
23 checks in run 40118525639 and merged as
`94d0567fb865fb49a90aaeeef4b5861707c9f8a7` on 2026-10-24 at 19:02:36 UTC.
Retained native receipts show eight cases each on ext4, APFS and NTFS, covering
fresh/existing WAL, segment, manifest and CURRENT failures; every ledger has
zero unresolved effects and violations. All three 100-cycle reopen jobs passed.
Logs and parsed receipts: operator-local `target/c07-pr116-native-evidence`.
This accepts that prerequisite candidate, not PR117's changed runtime.

PR117 incorporated current public main by an ordinary, conflict-free merge
`d88d9e5cf69ecf5d458c327ce1323800ac24f901`. No source files changed in the merge.
The optimized Linux reproduction remains bound to its original clean
779b3341 revision and must not be relabeled as the later integration revision.


### Completed 10k reproduction: correctness flow, failed operating point

Clean source 779b33412a2c1fca7451e0aca9894652c92b0119 built optimized Linux
in 331.1 seconds. Ordinary executable SHA-256:
`5846173ff28f044e9a6b518869db6fdbff6ff3908cc51d2ec6a045d45d15fb5c`;
harness SHA-256:
`ea0c473ade62eb4a0d43f221c6742b6891c1ff54e8921831366f4b0747accda5`.
`wsl -d Ubuntu -- python3 /mnt/d/localdev/devlabs/rrflow/target/run_c07_capture64_5rps.py 10000`
completed in 1,101.71 seconds including preparation. Campaign digest:
`b21fef7d8b3e0945fd7063dca97ccb213b670b77750e93791ae8394877512cc7`.
The same 10k records / 50k edges / 10k 384-dimensional vectors, four clients,
5 RPS, four CPUs, 2-GiB aggregate limit and 64-MiB cache were retained.
No profiler attached to this reproduction. The shipped default is unchanged.

All corpus writes and independent readbacks completed. Both forced setup
reopens and exact receipt replays passed; final post-load restart/readback
verified all 10,000 seeded records plus 40 changed working slots. Final readback
took 431.358 seconds. Launcher cleanup is confirmed. The previous startup exit
was not reproduced; its cause remains unknown and is not declared repaired.

The operating point failed: 490 of 600 offers acknowledged, 110 rejected
(105 driver-concurrency-limit and five storage conflicts), 95 scheduled-latency
overruns, zero unresolved effects and no missing metrics. Warmup passed all 50;
measured had 40 failures / 41 overruns; burst had one failure / no overrun;
short endurance had 69 failures / 54 overruns. Maximum scheduled latency was
8,680.165 ms measured and 1,523.309 ms in endurance. Maximum sampled engine RSS
was 403,984,384 bytes. Native counters showed two successful compactions, no
failed maintenance and zero final compaction debt. Correct readback does not
qualify these response/error results.

Backup creation took 14,219.578 ms and verification another 1,606.331 ms for
10,014 actions / 82,095,210 payload bytes. `PointRun::segment` joins the backup
before returning; the driver caps reader-interference segments to ten seconds.
This wait delays later offers on their unchanged absolute schedule. It explains
a measured scheduling contribution, not all failures: the later endurance
phase fails without that backup barrier. Retain both problems separately.
Do not reset the offered clock, increase error/latency limits, disable audit,
weaken durability or raise the default cache to call this a pass.

Raw receipts are retained under
`/home/devcontainers/rrflow-c07-native-79663cdd/capture64-5rps-growth-10000-779b3341`.
Small copies and derived timing/resource summaries:
`target/c07-pr117-capture-evidence/10000`. The exact `analyze_c07_capture64_5rps.py`
and `retain_c07_capture_results.py` commands completed. Qualification is false;
no full Desktop profile or actual 24-hour soak ran. This diagnostic completion
keeps C-07/POAM035 open and supplies the next code-grounded failure oracles.
