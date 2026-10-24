# C-07g: integrated native growth and captured reopen failure

**Status:** active retained failed native characterization; C-07 remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-integrated-native-growth`
**Owner:** OBJ-02 / C-07g / POAM-035 / PR120

## Exact candidate and scope

[PR120](https://github.com/rrflow/rrflow-development/pull/120) candidate
`037635afbf202fe8379ebec647f51bd57b410a79` combines PR117's digest/export/startup
capture, PR119's point-read selection and the bounded backup scheduler.
Its clean Linux release build passed in 224.7 seconds. The ordinary executable
SHA256 is `284d5a45f59f2ef63e906b136237e23ca23cbe29bf8b6466957a9d2263cf203c`;
the installed harness SHA256 is
`b9cacb54bf22f0bbdc64486933a2dae5ef2c175fe8e2a4431922b62ab61d54d9`.
The PR brief included the native comparison before implementation and named
this follow-up record before its creation. This record changes evidence only.

Both frozen characterization inputs use four clients, 5 requests/second,
a 500 ms scheduled-latency budget, zero allowed errors, a 2x burst,
a 64 MiB page cache, four aggregate cgroup CPU equivalents and 2 GiB engine
memory on the actual 33,352,753,152-byte WSL host. The larger corpus has
10,000 records, 50,000 edges and 10,000 384-dimensional vectors. Neither
corpus nor host matches the full Desktop qualification allocation. The
60-second phase named `soak` in these diagnostic inputs is not a 24-hour soak.
The shipped 4 MiB cache default and all qualification thresholds are unchanged.

## Results and retained failures

The 1,000-record campaign
`84f95dab72955f45b9c93a5f038cc078facc6cef95cd7e58767e6f86c2f39ca6`
failed after 204.68 seconds of launcher elapsed time. All corpus writes and
independent readbacks, both setup reopens and archive verification completed.
Backup creation took 1.399811 seconds and verification 0.206619 seconds.
The accepted segment chain contains 500 offers: 268 ACKs, 232 rejections and
107 scheduled-latency overruns. Zero unresolved effects were reported in that
accepted chain; this is not final restart/readback proof. The rejected next
segment contains another 50 offers (four ACKs, 46 rejections, 45 overruns) and
a 3.793841-second sampling gap with one unchanged process identity. The
validator refused the segment. Final independent readback did not execute.
Owned process/cgroup cleanup was confirmed. No failed point is reclassified.

The 10,000-record campaign
`06e145324859dacc5817295a5a7f8b90f24fda7ea2d17381478b9bd6fde6596f`
failed after 477.74 seconds of launcher elapsed time. All 10,000 writes
acknowledged and all 10,000 independent corpus readbacks completed. The first
forced restart then exited with status 1 before readiness. The bounded stderr
capture now reports `Storage("multiply linked file denied")`. There were no
measured segments and no final readback. This reproduces the formerly hidden
startup exit with a specific admission error, not an identified file or cause.
The temporary estate was removed by the harness's ordinary TempDir unwind;
the result/progress/logs survive, but those failed filesystem bytes do not.
That diagnostic-retention deficiency must be repaired before another growth
reproduction. Keep strict link/mount/symlink admission; do not treat a zero-link
open handle and a multiply linked file as the same diagnosed cause.

## Diagnostics and interpretation

The first run's post-run Linux PSI observation reported I/O full avg60 21.45%,
with near-zero CPU/memory pressure. A separate bounded 20-write, 80 KiB fsync
probe later completed in 65.6 ms (maximum 23.74 ms). These observations identify
stall timing, not an external-host explanation or a passing engine point.
The [kernel PSI definition](https://docs.kernel.org/accounting/psi.html)
distinguishes some stalled tasks from all non-idle tasks stalled simultaneously.
The source was reviewed for this interpretation; no upstream code was adapted.

The larger diagnostic run recorded bounded one-second host pressure samples,
a 30-second owned-thread wait sample and two 30-second CPU captures at 49 Hz.
The wait sample included 15 fdatasync observations, 11 filesystem-journal waits,
four page waits and idle futex/epoll waits. Ingestion CPU sampling retained 390
samples and readback 527, with no lost samples. The samples distribute across
filtering, compression, hashing, page access and version selection; no single
function explains the failure. Profiling prevents treating this as clean
capacity qualification. Source inspection also confirms durable public
invocation audit commits; their cost is not removable by dropping receipts.

## Commands, reads and next boundary

Operator-local commands were `build_c07_integrated_linux.py <revision>`,
`run_c07_integrated64_5rps.py <revision> 1000`, and
`run_c07_integrated64_pressure.py <revision> 10000`. They invoke the repository's
unchanged `scripts/ci/run_sustained_capacity.py` and exact ordinary installed
harness, freeze source/tree/configuration/artifact identities and enforce the
native aggregate allocation. Both launchers returned exit 1. Their original
stages remain under `/home/devcontainers/rrflow-c07-native-79663cdd/` with names
`integrated64-5rps-growth-{1000,10000}-037635af`; safe summaries/logs are retained
under operator `target/c07-pr120-evidence/037635af/`.

Reviewed the complete admission/link checks in `filesystem/native.rs`, anchored
open/reopen/publication functions in `filesystem.rs`, installed startup capture,
corpus seed/readback and lifecycle restart functions, plus the linked source
and native receipts. Read invocation audit/security/control commit functions
for latency attribution; no changes to their authority are proposed here.
The next bounded package must retain failed owned state and identify the exact
file/link-count admission before repairing its canonical lifetime/publication
owner. No security guard relaxation, hidden retry, increased deadline or
qualification-status change is authorized by this observation.

Checklist: exact source/artifacts and initial failures retained; bounded native
resource/debug decisions recorded; source reads and uncertainty identified;
new owning gap linked; docs/navigation/inventory required before commit.
Remaining not-run evidence: successful integrated growth, full profile and
seed matrix, final exact-candidate CI/native acceptance, actual 24-hour soak.
C-07, POAM-035 and the alpha objective remain open.
