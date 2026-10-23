# C-07g idle-segment deadline reservation

**Status:** active native fleet repair evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-idle-segment-deadlines`
**Owner:** [C-07g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 / POAM-035

Starting revision b234f427b12557acd47ceba058304fd59e415cdb, tree 3c8d34e02620d449f7dfdadd0d151393d4aa81b7. The PR75 description was extended
with the exact brief before implementation. This is the existing fleet-driver
package; it does not add engine runtime state or another evaluation authority.

## Failure, scope and replacement

The first actual two-estate Windows Job campaign on combined candidate
79663cdd4d162379c82a2fab0ff3f87d5cbf907e failed in the quiet trial with
`invalid client timing boundary`. The idle member collected a full 120000ms
of resource observations, then executed its required baseline probe. Its
receipt necessarily exceeded the manifest's 120000ms per-segment ledger bound.
The launcher retained a failed fleet result and confirmed owned cleanup.
This is a harness budgeting defect, not permission to weaken the ledger.

`StorageCampaign::idle_segment_ms` now derives an idle span from the frozen
run limit minus its operation deadline, the existing sample quota, requested
phase duration and finite driver cap. Both ordinary drain and quiet-neighbour
waiting use this owner. `PointRun::drain` checks admission and wraps sampling,
probe and independent verification in the same outer deadline as offered-load
segments. Quiet waiting also caps each chunk at the measured duration so a
short trial need not wait a full two-minute polling interval.

Actual probe timestamps, receipt identities, all ledger checks, resource
samples, foreign-session denials and shared kernel allocation remain intact.
No default rate, rejection threshold, manifest schema, engine API, persistent
state, admission limit or isolation rule changes. Stop if success requires
fabricated timings, omitted probe/readback, a relaxed cap or unknown effects.

Reviewed: README and C-07/POAM owners; the change-authoring procedure;
`storage::ledger::reconcile`, campaign validation/result verification and
fixtures; `PointRun::drain/segment`, resource sampling, session rotation,
phase chunking; `Participation::wait_quiet/boundary` and fleet coordination;
actual failed member progress/result and launch evidence. The complete changed
methods and source diff were reread. No external research/adaptation is needed
for this deterministic local mismatch. Existing bounded raw traces, native
resource observations and exact histories remain the evidence mechanism.

## Validation and checklist

- First failure oracle: actual installed quiet-neighbour trial described above;
  retained at `target/c07-windows-fleet-characterization-01/evidence` in the
  operator workspace. Failed, no qualification, cleanup confirmed.
- Focused `idle_segments_reserve_probe_time_and_respect_sample_capacity` passed:
  120000ms run/30000ms probe admits at most 90000ms idle; shorter phase/sample
  caps stay binding; exhausted time/sample room and zero duration are denied.
- `cargo test --locked -p rrflow-eval`: 40 passed, zero failed. Existing
  schema goldens, independent ledger, retained evidence and fleet isolation
  negatives pass unchanged.
- `cargo clippy --locked -p rrflow-eval --all-targets -- -D warnings`: passed.
- `cargo clippy --locked -p rrflow-cli --test installed_reopen -- -D warnings`:
  passed in the dedicated fleet target, 1m04s. The first shared-target attempt
  reused an older contract artifact and reported a missing StorageCursorSnapshot;
  no source workaround was added. The isolated target resolves the mismatch.
- Formatting passed. Navigation/inventory/documentation and whitespace checks
  run before commit. Exact native fleet rerun and current-candidate CI remain
  pending for this repair; these component results do not substitute for them.

Changed files: evaluation campaign helper and regression; CLI drain admission
and chunking; fleet quiet chunking; this journal, POAM-035 and generated indexes.
No files moved or deleted. C-07, alpha and POAM-035 stay open. Current source
reaches main only through its candidate CI gate. No merge is claimed here.
