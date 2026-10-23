# C-07d: separate optional compaction from hard write admission

**Status:** active evidence record; native comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07d-soft-maintenance-admission`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [94](https://github.com/rrflow/rrflow-development/pull/94)

Starting revision `a3976c568cf8af98bb64028bf2f86b4302489694` integrates PR93
from fetched main. The pre-edit brief binds the behavior, scope, failure oracle,
replacement map and stop conditions. Post-flush optional compaction previously
rejected the incoming write on any maintenance-budget denial, even when the
new write still fit hard L0 and mutable capacity. The replacement defers only
typed input-byte/logical-byte/version, output-count or elapsed-time compaction
denials at that soft trigger. The merge still stops at its hard limit before
publication. Its debt and failure counters remain visible. All other errors
propagate; the unchanged hard L0, reclamation and mutable checks still run
before WAL append. This is not permission to exceed any resource limit.

`compact_segments_inner` also marks reclamation pending before partition
creation. An idle quantum can fail after writing an unpublished partition
without a preceding flush; cleanup must remain owned in that case. The next
quantum reclaims before another merge. Publication uncertainty remains fenced
by the existing writer checks. There is one publisher and no parallel recovery
or compaction implementation.

## Scope, inspection and decisions

Changed `rrd-lsm/src/database.rs::prepare_write` and
`compact_segments_inner`, native `tests/maintenance.rs`, and the capacity
driver's `support/sustained/workload.rs::engine_error_label` and existing label
test. The label projection distinguishes six allowlisted maintenance resources
without copying server messages or deciding effects/retryability. Complete
labels plus their digest remain within the evidence identity bound. Unknown
spellings retain the prior generic label. Installed-limit reference, this
journal, POAM-035 and generated navigation/inventory record the change.

Read README and the objective/C-07/POAM chain, authoring/traceability, installed
storage limits, complete prepare-write, compaction candidate/admission/merge/
publication and reclamation functions, store/engine error conversions, existing
native maintenance/compaction/fault tests, driver error projection, and Linux11
receipts. Rechecked all changed functions/tests and the final diff. Research:
local failure and contracts suffice; no external source adaptation. Preserve
hard caps, native counters, exact source/failure identities and payload-free
signals. Stop on ignored I/O/corruption/uncertainty, admitted hard saturation,
orphan accumulation, changed logical history or unsupported performance claims.

## Evidence and remaining limits

The first native oracle failed on its third write: a one-byte logical compaction
budget rejected 49 bytes while L0 remained below its hard cap of four. After
the repair, four writes are acknowledged; the next write fails at hard L0
saturation without advancing its sequence. Reopen preserves every prior
acknowledgement and the absence of the denied mutation. Explicitly sufficient
compaction admission drains debt with the same hard L0 cap, then accepts the
next write exactly once. Historical reads survive both compaction and reopen.

A second history permits only one output partition. Each failed merge leaves
exactly one unpublished file; the next idle quantum removes it before retry.
Repeating the history does not accumulate orphan files, and a later acknowledged
mutable write reopens correctly. No ceiling was raised to pass this fixture.

- Focused red/green `cargo test --locked -p rrd-lsm --test maintenance soft_compaction_denial -- --nocapture`:
  failed before replacement, passed afterward.
- `cargo test --locked -p rrd-lsm --test maintenance failed_idle_partition -- --nocapture`:
  passed.
- `cargo test --locked -p rrd-lsm`: 201 passed, 2 existing ignored.
- `cargo test --locked -p rrd-engine --lib write_admission`: 7 passed.
- `cargo test --locked -p rrflow-cli --test installed_reopen diagnostic_labels`:
  1 passed.
- `cargo clippy --locked -p rrd-lsm -p rrflow-cli --tests -- -D warnings`:
  passed. Formatting/docs/navigation/inventory/diff checks passed before
  commit. Raw local logs are `target/c07d-soft-maintenance-*`.

Linux11 source `874fa9a9d133db2cc7ef8da65414d13ddae75945`, binary
`c5f4a20d515a7024c2ad972c52477b0b486e5bd09d6f598873ba81cf89ea0211`, campaign
`b3861df8a74043102a31ae784ddacbedb003ecd2f219bed435171557ee21a868`, harness
`16391df33033a38326a361b1fa0593529f76656c27f98448e2073c93ca54f904`
failed after 209892 ms during corpus setup, before any measured segment.
Progress retained 832 intents and 807 acknowledgements; this is not a complete
corpus/readback proof. The public failure was maintenance admission, detail
digest `a4d7f78e7e713810c1a9efe2da187d5ce93113dd81bdb20edaf26f1274a3b021`.
The earlier generic label does not identify which maintenance resource failed;
do not claim this component repair proves that native operating point fixed.
Cleanup is confirmed. Local receipts are `target/c07-linux-scale-evidence-11`.

Checklist: pre-edit brief, owner/source inspection, mapped replacement,
demonstrated red/green oracle, bounded cleanup history, owning/admission/label
checks, diff review, journal and projections are recorded. Unchanged native
comparison, sustained debt convergence/latency, the macOS I/O-receipt failure,
sampling coverage, full profiles, 24-hour soak and exact candidate CI remain
open. No roadmap, objective or POAM closure and no promotion claim.
