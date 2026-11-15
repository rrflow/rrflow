# POAM-074 — the macOS capacity rig fails when a new disk image is busy

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-074`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail is governed by the parent ledger; it cannot independently change
priority or closure status.

## Observed deficiency

On 2026-11-11, candidate run 42954943269 failed `installed domain crash (macos-latest)` on a pull request that changes no engine code: `hdiutil: create failed - Resource busy`. Six capacity cases had already passed in that job and the previous image reported as detached.

The first version of this record blamed `scripts/ci/run_filesystem_containment.py`. That was wrong. The create is in `scripts/ci/run_native_capacity.py`, which since 2026-11-08 retried a busy create up to five times and removed a leftover image between attempts.

A second occurrence, run 43017327376 attempt 1 on PR #335, shows why that retry never ran. Case 21 failed 4 seconds after case 20's image detached, so the backoff of 1, 2, 4 and 8 seconds was never used. Its retained receipt has `stderr_open: true`, and the runner killed an orphaned `diskimages-helper` when the job ended. hdiutil starts that helper outside the command's owned process group, and the helper inherits hdiutil's stderr. After the busy create it kept that pipe open. The retry guard refused any create whose output was still open, because a live holder might still be using the image. So the one failure the retry was written for was the one it refused.

Why the image service is busy seconds after `hdiutil info` stops listing the previous image is inferred, not reproduced.

Later on 2026-11-11 it recurred in the merge queue for PR #349, which also changes no engine code: run 43083711389, job 130742748923, `hdiutil: create failed - Resource busy`. By then `scripts/ci/run_native_capacity.py` already retried a finished create with backoff and removed a leftover image between attempts ([the retry journal](../../evidence/change-journals/gate-c/c-07-hdiutil-busy-retry-20261109.md)); that retry did not recover the case. `hdiutil create` attaches, formats and detaches the new image itself, so every case ran one more attach cycle than it needed.

## Remediation in progress

[The template journal](../../evidence/change-journals/gate-c/c-07-hdiutil-template-20261111.md): `hdiutil create` now runs once per suite, for a pristine template the rig never attaches. Each case copies the template, then attaches, verifies and detaches the copy as before, so it still starts from a new, never-written APFS filesystem. The suite removes the template at the end. This removes the per-case create. It does not prove the busy failure cannot hit the one remaining create, which keeps its bounded retry.

[The held-output journal](../../evidence/change-journals/gate-c/c-07-hdiutil-held-output-retry-20261114.md): that one remaining create now recovers from the failure that was observed. The open-output record keeps its capture threads, so the rig can see the moment the last holder closes the pipe. A busy template create with held output detaches the half-made template, then retries only after the held output closes, waiting at most 20 seconds per attempt. If the holder never lets go, the original failure is raised, no retry runs, the template is kept, and the receipt records `holder_outlived_release`. Every retry logs one `capacity_create_retry` line. A timed-out create is still never retried. This is proven by unit tests only: no macOS run has hit a busy create since the template change, so the retry has not been seen to act on a real one.

## Impact

A required candidate check can fail for a reason unrelated to the change under test, which blocks the merge queue and teaches people to rerun failures.

## Owning gates

C-07 owns the native capacity campaign.

Roadmap owner: [Gate C](../../roadmap/rrflow-1.0/gate-c.md).

## Closure evidence

Closed when image creation tolerates a busy image service with a bounded, attributed retry, or the cause is shown to be elsewhere, and repeated macOS runs pass.
