# C-07h: native candidate CI at dce14e51

**Status:** active evidence record; C-07 remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07h-native-candidate-ci`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [72](https://github.com/rrflow/rrflow-development/pull/72)

## Baseline and scope

Starting revision `dce14e51a224c891fe1568e5a226ef51bf6ef3db` completed
[candidate run 39944487586](https://github.com/rrflow/rrflow-development/actions/runs/39944487586):
22 candidate jobs and the final `pipeline / ci-gate` succeeded. This follow-up
adds only this immutable evidence node and generated navigation/inventory.
Its brief was added to PR 72 before editing. No code, threshold, release
status, version, configuration or authority changes.

## Parsed native exhaustion evidence

Downloaded completed job logs and parsed each `native_capacity_receipt` as
JSON, requiring exactly eight receipts per platform, zero unresolved outcomes
and an empty violation list. Each platform covers fresh and existing estates
at WAL write, immutable segment creation, manifest creation and CURRENT
creation. Native owned-volume filling and cleanup remain in the original
logs; this summary does not substitute an injected error for disk exhaustion.

| Platform | Exact native job | Receipts | Acknowledged attempts | Unresolved | Violations |
|---|---|---:|---:|---:|---:|
| Linux | [120384238039](https://github.com/rrflow/rrflow-development/actions/runs/39944487586/job/120384238039) | 8 | 44 | 0 | 0 |
| Windows | [120384238144](https://github.com/rrflow/rrflow-development/actions/runs/39944487586/job/120384238144) | 8 | 44 | 0 | 0 |
| macOS | [120384238374](https://github.com/rrflow/rrflow-development/actions/runs/39944487586/job/120384238374) | 8 | 44 | 0 | 0 |

The installed ordinary-primary 100-cycle reopen jobs, native domain crash
jobs, lifecycle, SDK conformance, engine partitions and optional-feature jobs
also passed in that run. Their original logs/artifacts own detailed histories.
This is evidence for the exact revision above, not later source-page,
checkpoint, publication or fleet revisions.

## Review, checks and landing limitation

Read the existing C-07 owner, CI workflow and candidate job results, the native
capacity receipt report shape and the three complete native receipt sets.
The failure oracle rejects a missing receipt, unresolved outcome or violation;
all 24 parsed receipts passed. `gh run view ... --json status,conclusion,jobs`
confirmed the successful candidate. No external research or new runtime trace
was needed: this package records existing primary native measurements.

GitHub refused the normal merge because a second PR initially shared this
revision and emitted duplicate named checks. Its superseded workflow
39951057270 was cancelled and retained; its aggregate failed because required
jobs were cancelled. Rerunning the successful original aggregate did not
remove the ambiguity. No check was deleted, branch policy weakened or
administrative merge used. This evidence-bearing follow-up establishes a
unique candidate revision and requires ordinary candidate CI again.

The full journal and generated diff were reviewed. Navigation, execution
inventory, documentation and whitespace checks were run before commit. No
runtime tests were rerun for this documentation-only package; its new
candidate must pass CI before merge. No native power-loss, full workload
ladder, competitive throughput or completed 24-hour soak is claimed. C-07,
OBJ-02 and POAM-035 remain open. The PR/CI records own eventual merge evidence.
