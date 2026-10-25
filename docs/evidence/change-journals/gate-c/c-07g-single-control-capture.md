# C-07g: one control-journal capture per backup

**Status:** active evidence record; native comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-single-control-capture`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [91](https://github.com/rrflow/rrflow-development/pull/91)

Starting revision `95a7c6d237b63b89c749d575fc05aae714c84848` integrates PR90
from fetched main. The pre-edit PR description binds scope, replacement map,
oracle, decisions and stop conditions. Application backup formerly traversed,
decoded and hash-validated the complete control journal twice: first for final
catalogue state, then for included control transitions. The private
`CatalogueCapture` now folds catalogue entries during the existing verified
control traversal. `retain_catalogues` retains its existing ordered manifest,
revision validation, digest and persistence responsibilities without a second
journal read. No fallback or parallel implementation remains.

The same `SnapshotStore` remains held through final backup publication. Every
journal entry, including excluded session history, is still validated. The
accumulator moves only catalogue values into its existing bounded map; it does
not retain another full journal. Control archive and catalogue manifest retain
their existing bytes/digests. Their internal creation order changes, but the
only application-complete publication remains the final catalogue entry.
An error leaves no successful backup claim; existing idempotent retained-file
verification and restore rules remain unchanged.

## Scope and decisions

Changed `rrd-store/src/backup.rs` at application backup composition, control
capture and catalogue folding/serialization. Added private
`backup/capture_tests.rs`; shared two existing test helpers within the parent
test module. This journal, POAM-035 and generated navigation/inventory record
the package. No public API, format, runtime default, ceiling, SDK or dependency
changed. No external source was adapted.

Read the README/objective/C-07/POAM owner chain and change-authoring/traceability
procedure; complete replaced backup functions, callers, manifest validation,
restore/replay, snapshot store, journal repository/key grammar, existing public
backup tests, engine backup composition and sustained lifecycle driver.
Complete changed functions and new test module were reread with the final diff.
Research: the local failure and existing contracts suffice. Resource decision:
count actual journal rows at the physical transaction port and preserve all
native ceilings. No new raw-content log or independent telemetry authority.
Stop on changed cut, identity, publication or corruption behavior, relaxed
limits or unsupported native-performance claims.

## Oracle and acceptance

A wrapped `StorageTransaction` counts actual journal rows returned to backup.
Before replacement, an eight-entry journal read sixteen rows; with 96 excluded
entries added, it required a second full traversal as well. The replacement
reads exactly one row per entry on both rrflowMX and native rrflowKV. The mixed
fixture covers catalogue update/delete/recreate, deletion of one catalogue
while another retains that scope, independent scope revisions, included
installation state and excluded session growth. Application backup, archive
verification, restore and reopened state agree. Golden backup identities
recorded before replacement bind all artifact digests and remain identical:

- Eight-entry cut: `9097d42dd0a6adc1f3f69ed82a3c47da199222a54000d3516205ab5f08fa7f4f`.
- After excluded growth: `53a0af37d1870a209c7cd937cda9e1e0d03c81746e3ed6f057bb27eb4dd6c446`.

Initial test compilation exposed private sibling helper visibility. The helpers
became test-only `pub(super)`. The first history also tried to archive a scope
with a positive catalogue revision but no remaining records; existing validation
correctly rejects that unsupported state. The fixture now retains a second
catalogue in that scope while still proving deletion of the first. Production
validation was not changed. A local runner script's BOM decoding was corrected
before the owning checks; it did not change repository behavior.

Acceptance commands: focused `cargo test --locked -p rrd-store --lib
backup::capture_tests -- --nocapture`; owning `cargo test --locked -p rrd-store`;
`cargo test --locked -p rrd-engine --lib backup`; strict store all-target Clippy;
format, documentation, navigation, inventory and diff checks. The focused
fixture passed; all 229 store tests, five engine backup tests and strict
Clippy passed. Formatting/documentation/navigation/inventory/diff checks passed. Local raw logs use `target/c07g-control-capture-*`.
Full capacity profiles and 24-hour qualification are not inferred from these
checks; unchanged native comparison remains required.

## Retained failures

Linux09 ran source `a811a89cfa877e24c152e1d1b8da361bbf33fcd0`, ELF
`e8bc60a964615e21b8e9cd68eb8b4e05fb4a26dc878d1dc9b01cec0ea5cebdd3`, campaign
`621254622e63aa22b7c500d4565d3e5325bfce4174fc22c328694a997464b7d6`, harness
`9e59319f7ec29308e82562a393e1d9bf6869d193eb9413a3ace7a20cd0beb30f`.
It completed 1,000 corpus acknowledgements/readbacks and two forced reopens,
then failed after three segments with `backup interference deadline` under the
unchanged 30-second operation deadline. Launch elapsed 411255 ms; cleanup was
confirmed. This is a failed operating point. It no longer reports the previous
output-buffer violation, but does not prove backup completion. Local receipts
remain in `target/c07-linux-scale-evidence-09` and the complete log in
`target/c07-linux-scale09-full.log`.

PR76's exact source `268552de92ffdfeb63d73f4fd096eecd76b3a992` also failed
[macOS native capacity job 120467760417](https://github.com/rrflow/rrflow-development/actions/runs/39969405244/job/120467760417).
The owned APFS filler received native ENOSPC (28) after 252706816 bytes.
The selected `wal_before_write` boundary was reached; the public request returned
`ResourceExhausted`, but the expected selected native-I/O receipt never arrived
within 60 seconds. The harness killed its child and detached its owned image.
This is not native engine I/O-exhaustion proof; the reason for the missing
receipt remains unresolved. The raw log is retained locally as
`target/pr76-failed-job-120467760417.log`. Do not replace the required native
error with an admission rejection or classify a rerun as a repair.

Checklist: pre-edit brief, replacement map, source inspection, counted-read
failure, corrected test fixtures, preserved golden/restore semantics, owning
checks, diff reread, linked journal and projections are recorded. C-07,
OBJ-02 and POAM-035 remain open. No alpha or promotion status changes. Required
candidate CI and native qualification remain separate acceptance work.
