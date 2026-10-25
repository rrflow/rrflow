# C-07f: pace owned-image teardown within the existing bounds

**Status:** active implementation evidence; native candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-owned-image-settlement`
**Owner:** OBJ-02 / C-07f / POAM-035 / PR129

## Brief and native failure

[PR129](https://github.com/rrflow/rrflow-development/pull/129) extended its
reviewable brief before this correction. Starting revision:
`d04a0dc5b3379d52fa893b12cd71bc30669fee16`. Its candidate
[run 40158423166, macOS job 121064657832](https://github.com/rrflow/rrflow-development/actions/runs/40158423166/job/121064657832)
passed the full eight-case capacity matrix and three additional WAL/segment
cases. All eleven receipts contain zero violations and unresolved operations.
The next image teardown exhausted five attempts with native EBUSY (16), about
three seconds after the last completed case. The remaining repeated cases did
not run. The job failed; this is not candidate acceptance.

Raw log: `target/pr129-native-40158423166/121064657832.log`, 353256 bytes,
SHA256 `256033d56524b0460144eb0d73bb4ff06a804bc26ab29743c27218c76f4d7e70`.
The failed image was retained by the existing controller. The same candidate's
Linux job 121064657605 and Windows job 121064657667 each passed eight native
cases, zero violations/unresolved operations, with OS capacity codes 28 and
112 respectively. Those logs remain under the same directory, with SHA256
`8a512f142ff628f3be1f7deee1d2edae28a27bec8fbdefbd7a608a9224f4c84a`
and `533b60378c63d3507d418317188c269779601c47cb4c8defcc1565cad7cb872a`.
Their results do not clear the failed macOS teardown.

## Canonical correction and traceability

The owner remains `scripts/ci/run_filesystem_containment.py::detach_owned_image`,
used by the mounted-filesystem adversary and native-capacity runner. Existing
250-ms spacing exhausted the fixed five attempts long before the existing
30-second deadline. The same five attempts now have 1, 2, 4 and 8 second waits
between them. Every command uses the original remaining deadline; every retry
rechecks exact image/device ownership. The final absence observation remains.
No additional sleep follows the last attempt.

The force-detach invocation, image identity guard, device-reuse/ambiguity denial,
nonbusy error propagation and refusal to delete an attached image are unchanged.
No timeout, attempt count, selected-storage oracle, runtime storage behavior or
version is widened. No OS service is killed and no new device target is added.
This is a scheduling correction inside the existing teardown owner, not another
cleanup implementation or retry-until-green policy.

Reviewed the containment runner/shared detach helper, native capacity runner,
`test_native_capacity.py`, retained failure diagnostics and all eleven native
receipts, prior volume-settlement journal and POAM035. Reviewed owning CI instructions and
candidate native steps. Changed source: those two Python helper/test files.
Evidence: this record, POAM035 supporting link and generated navigation/
execution inventory. Existing journals and failed fixtures are retained.

Research decision: source, deterministic clock and the exact native failure
suffice for this scheduling change. No new filesystem ABI or upstream code is
introduced. The identity of any OS component holding the disk busy is unknown;
this record does not assign a cause to a daemon. Existing bounded native error
and image identity diagnostics remain. Actual asynchronous teardown behavior
requires the next native candidate; a mocked clock cannot prove it repaired.

## Failure oracle and validation

Before the helper change,
`python -m unittest test_native_capacity.OwnedImageDetachment.test_busy_attachment_can_settle_within_existing_deadline`
failed with the same bounded-cleanup/native-busy error: five rapid attempts
expired before the modeled attachment disappeared after 2.5 seconds. After
the change that exact test passed, requiring two attempts, waits of 1 and 2
seconds, elapsed time below 30 seconds and the image still present.

From `scripts/ci`:

- `python test_native_capacity.py`: all 14 tests passed, including finite five-attempt failure with waits of 1/2/4/8, original shared deadline, changing/ambiguous identity, nonbusy errors, verified absence, failed-image preservation and case sequencing.
- `python test_offline_lifecycle.py`: all 16 policy/command-controller tests passed.
- `python -m py_compile run_filesystem_containment.py test_native_capacity.py run_native_capacity.py`: passed.

Documentation ownership, workflow policy, generated navigation/inventory and
working/staged whitespace checks passed after the correction. Rust and engine/SDK
suites were not rerun because no Rust, API, model or engine source changed;
the preceding volume-correction results remain bound to d04a0dc5. Native APFS
teardown, complete repeated-case coverage and the full candidate pipeline have
not yet run on this correction. No failed job is reclassified as passed.

Checklist: brief before effects; exact native failure retained; canonical owner
and characterization map; deterministic first failure then focused/owning
checks; unchanged resource and cleanup guards; research/diagnostic decisions;
omissions explicit; complete diff review. Stop for widened bounds, identity
drift, suppression of nonbusy errors or deletion without confirmed absence.
C-07, POAM035, capacity profiles and 24-hour soak remain open.
