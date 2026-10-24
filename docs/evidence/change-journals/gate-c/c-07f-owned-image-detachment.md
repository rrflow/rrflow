# C-07f: bounded owned-image detachment

**Status:** active implementation evidence; native candidate acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-owned-image-detachment`
**Owner:** OBJ-02 / C-07f / POAM-035 / PR116

## Brief and observed failure

The [PR116](https://github.com/rrflow/rrflow-development/pull/116) description
was extended before this CI-exposed repair. Starting revision is
`469a4a568139f1d1a17a65225e2ce2da41505dbc`. Candidate run 40115318651,
macOS job 120935676134, passed all four fresh native capacity cases and the
existing-project WAL case, then failed before detached confirmation with exit
16. No failed storage assertion was reported for that case. The shared command
wrapper suppressed command identity and native diagnostics. The original log
is retained operator-locally as `target/pr116-macos-job.log`.

Code inspection locates this interval at the shared owned-image cleanup. Exit
16 is Darwin EBUSY according to [Apple XNU errno.h](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/errno.h).
The exact native stderr was not retained by the old wrapper, so the holder of
the busy device is unknown. Do not claim engine recovery or all eight cases
accepted from five passing case receipts.

## Canonical repair and traceability

`run_filesystem_containment.py::detach_owned_image` remains the shared owner
used by native containment and capacity qualification. It re-reads plist image
identity before every detach, rejects duplicate image identities, missing whole
devices and changed attachment identity, and verifies absence before returning.
Only EBUSY can retry, with at most five attempts inside one 30-second command
budget. Every command receives the remaining deadline and an owned process
group. Other failures retain bounded command diagnostics and fail immediately.
A success that still leaves the image attached is denied. Existing caller
cleanup retains failed fixtures and never deletes an attached image.

No storage operation, transaction, resource acceptance limit or production
runtime changes. No generic retry or new lifecycle authority. The previous
single detach maps directly to the same canonical function; existing caller
retention assertions remain in `test_native_capacity.py`, augmented with
command-response oracles. Native macOS CI remains required; simulations do not
establish APFS acceptance. Stop on uncertain ownership or failed cleanup.

Complete files read: shared `run_filesystem_containment.py`, native capacity
runner and tests; relevant bounded `run` helper in `run_offline_lifecycle.py`;
CI native-capacity invocation, README/AGENTS, C-07, POAM-035 and authoring routine
already reviewed for the parent package. Read the full macOS failure interval,
including successful capacity receipts and missing detached confirmation.
Changed paths: shared detach helper, existing native-capacity tests, this linked
journal, POAM-035 link and generated navigation/inventory.

## Acceptance and auditable checklist

`python scripts/ci/test_native_capacity.py`: 13 passed in 0.049 seconds.
These include existing fresh-image orchestration/failure retention plus busy
retry, confirmed disappearance, nonbusy failure, changed/ambiguous identity,
shared deadline, finite attempts, false successful detach, missing device and
failed inspection. No native disk was created by these unit checks.
`git diff --check` passed. Full native macOS matrix is pending on the repaired
candidate; earlier failed run stays failed. No C-07 completion or soak claim.

- [x] Bound outcome, files, failure oracle and stop conditions in PR before edits.
- [x] Read current implementation and retained failure rather than retry blindly.
- [x] Keep one cleanup owner and preserve failed-image retention/deletion guard.
- [x] Map replacement behavior and test adversarial ownership/error cases.
- [x] Preserve native acceptance requirement and distinguish simulated evidence.
- [ ] Require repaired native candidate CI before merge.

Documentation initially rejected an unclassified status label; changed it to the required active evidence status without changing qualification. Re-run results follow.

Corrected documentation policy, generated navigation, execution inventory (1,592 records), and staged whitespace checks passed. Native acceptance remains pending at commit.
