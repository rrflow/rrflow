# C-07 offline lifecycle: preserve limits during timeout-policy integration

**Status:** active component evidence; native candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07-offline-timeout-integration`
**Owner:** OBJ-02 / C-07 lifecycle prerequisite / POAM-035 / PR46

PR46 began at `aae443db18f59357d3ef1bc857045f78a4efa15e` with a proposal to
increase the offline harness deadline from 540 to 720 seconds. The integration
brief was updated before rework to preserve the accepted ceiling and retain
only the common timeout-policy improvement. Accepted main
`f3c04e955fd8851c0fc466c45f008975e9d4c1a1` enters by an ordinary merge.

Both source files conflicted because main now has native Windows/macOS
isolation, bounded process capture, retained failure receipts and stricter
complete-corpus checks. The current main implementations and all their tests
remain intact. Only the original policy helper and its nesting test are adapted:
`lifecycle_timeouts` returns 540/600 seconds; the Linux outer process retains
30 seconds of cleanup headroom (630 total). The macOS wrapper uses the same
unchanged 600-second ceiling. Windows isolation ownership is unchanged.

Traceability: hardcoded harness/wrapper values in `qualify_payload` and `main`
remain owned by `scripts/ci/run_offline_lifecycle.py`; they consume the shared
function. Existing native isolation, PID cleanup, output limits, partial-progress
redaction, history and exact five-case acceptance remain their characterization
oracles. The retained timeout test now rejects an unreviewed budget increase.
No timeout failure is reclassified as success, and this package does not claim
to repair native WAL attribution or segment descriptor exhaustion.

Read the two-file original diff, current timeout/cleanup/corpus implementations,
policy tests and the complete final delta. No upstream code, external research,
new runtime trace or resource experiment is needed for identical deadline
values. Changed paths are the two scripts, this journal and generated indexes.
No engine source, release version or persisted format changes.

Validation: `python scripts/ci/test_offline_lifecycle.py` passed all 17 tests,
including real child deadline/output exhaustion and retained failure checks.
`python -m py_compile` passed for both scripts. Documentation, workflow,
navigation, inventory and whitespace checks passed on the final journal
state before commit. A Windows policy test is not Linux namespace or APFS
qualification; the candidate native lanes have not run on this revision.

Checklist: pre-edit brief and explicit no-increase boundary, normal main merge,
preserved current native implementations/tests, complete source diff review,
smallest owning suite, named omissions and linked evidence. The initial merge
conflicts were resolved without restoring the older unbounded subprocess calls.
C-07, POAM-035 and alpha status do not advance. PR46 remains draft behind the
shared native qualification; exact candidate CI and normal merge controls are
still required.
