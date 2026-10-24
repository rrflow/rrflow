# C-07d configured restore and exact resume

**Status:** active locally verified implementation; dependency reconciliation and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-restore-limits`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Baseline and brief

[PR 68](https://github.com/rrflow/rrflow-development/pull/68) bound this scope
before implementation. Accepted main was
`dde6ef86d278b8c59cf1f61ebc2d3923186396d5`, tree
`bc76b62d5a500395fb8e1193dcd4d94eeac79ada`. The [baseline](c-07d-restore-limits-baseline.md)
was committed at `02951068179220d61ea5041dc95cff7b88c865b3`.
Local dependency merge `ddcb70bacb0158cb480f2609d51a2d1353ea4fe0`
incorporates PR 67 at `3bd777239b0eec1c4d94f78a9fd416ac6aea2e35`,
including PR 65. Generated navigation/inventory conflicts were regenerated,
not treated as another authority. Reconcile accepted PR 64/65/66/67 before
final candidate qualification; local dependency code is not acceptance.

Logical restore and nested catalogue/control restoration used default native
options. The target is one caller-selected process-local policy through every
owned staging, replay and verification open. Defaults delegate to the same
implementation. Preserve private staging, exact-prefix reconciliation,
uncertainty, source artifacts and publication ordering. No installed schema,
archive format, repair, dependency or product-version change.

## Implementation and traceability

- `rrd-store/src/archive/restore.rs`: options-aware canonical implementation;
  the existing default progress entry point delegates. Initial/resumed staging,
  post-flush verification and post-population verification all receive the same
  options. `archive.rs` and `lib.rs` reexport the entry point.
- `rrd-store/src/backup.rs`: options-aware catalogued restore and published
  verification delegate through existing logical/application branches,
  catalogue replay, control replay and their verification helpers. Default
  entry points delegate. No new restore algorithm or policy owner.
- Existing `logical_archive_resilience` and `backup_catalogue` fixtures are
  extended; all-family/object/control histories are parameterized rather than
  copied into another harness. Existing default cases remain independently run.
- Native workflow/policy require the six `configured_` tests on Linux, Windows
  and macOS. CI notes, C-07 and POAM-035 explain the bounded evidence; navigation
  and execution inventory are regenerated projections.

No implementation was deleted or moved. The canonical restore action/receipt
and control progress logic is unchanged. Callbacks still own policy for any
additional capabilities they invoke; the final native reopen independently
checks the state they leave. Native options do not budget object payloads,
archive decoding, full recovery CPU/I/O, heap or every retained owner.

## Flow evidence

The runtime rejection history executes both native resources: prior standalone
ACK, rejected 32-claim semantic batch, unchanged one-claim/cursor-zero private
prefix, absent rejected outcome, adequate-policy resume, exact 33-claim/cursor-32
readback and outcome, removed staging/receipt, unchanged archive bytes.

Configured all-family restore compares exact changes, audits, read stamps,
selected versions, materialized snapshots, outbox and projection deltas.
Fixed assertions require two records and one claim, relation, event, vector,
series, geo and object reference. The relation connects entity a to b and the
vector retains entity a and its fixed source digest. This is native semantic
restore evidence, not ANN search, generated embedding or model inference proof.

Application catalogue rejection preserves the three-mutation logical prefix
and zero catalogue revision; adequate resume restores the exact catalogue,
object bytes and transaction audit. Control rejection preserves five replayed
entries plus the restored catalogue, the old installed-record value and prior
claim. Resume yields nine filtered control entries in exact order, including
updates/deletion and audit-head changes; excluded sessions remain absent.
Both published low-budget verification attempts preserve every existing file
byte. The source control archive remains unchanged.

A callback deliberately writes under a larger policy: the final low-budget
reopen denies publication. Retrying under the same low budget denies replay
before invoking the callback and preserves staging files exactly. An adequate
resume publishes one control effect and the original semantic state. A separate
logical-only catalogued history verifies its options branch and exact resume.

## Research, diagnostics and review

No new external algorithm/API or adapted source is needed. Existing native
admission and restoration contracts define the oracle. Preserve runtime traces;
assert typed failures, independent requests, semantic/control/object identities
and exact filesystem contents. No extra runtime instrumentation is needed.
The baseline used the word authenticated for restore; the implementation's
existing local digests detect changed bytes, not producer identity, and remain
inside the filesystem trust domain. This package does not alter that boundary.

Read the README owner chain, C-07/POAM-035, change-authoring/traceability routine,
native options and typed store admission; archive restore/receipt/export
contracts and backup catalogue/control/object call paths. Reread changed
archive/restore, backup, exports and both complete test targets; workflow,
policy and affected CI/roadmap/POAM records and complete source diff. Reviewed
runtime `data_snapshot` and core `RuntimeDataSnapshot` for materialized oracles.
No generated inventory is hand-maintained; retain policy executable mode.

## Commands and surfaced failures

Commands used Windows and isolated target `target/c07d-restore-build`.

| Command / oracle | Result |
|---|---|
| Logical `configured_restore` characterization with unenforced options | 0 passed, 1 failed: restore wrongly succeeded with insufficient policy. |
| Backup `configured_` characterization before nested propagation | 0 passed, 1 failed: application restore wrongly published despite insufficient catalogue budget. Cargo stopped before the logical target. |
| First focused `configured_` after propagation | 2 passed, 0 failed. |
| Expanded `configured_` histories | 5 passed, 0 failed. |
| `cargo test -p rrd-store --locked` after logical-only case | 212 passed, 0 failed, 0 ignored across 36 result groups. |
| `cargo clippy -p rrd-store --all-targets --locked -- -D warnings` | Passed. |
| `cargo test -p rrd-store --test logical_archive_resilience --test backup_catalogue --locked` after stronger materialized assertions | 29 passed, 0 failed, 0 ignored. |
| Final strict all-target store clippy | Passed. |

A test initially named the staging directory with `.partial`; reading the
canonical receipt helper corrected it to `.staging` before the green run.
A log read used a worktree-relative path instead of the root target path and
failed without mutation; the absolute path resolved it. A PowerShell `rg` glob
was rejected by Windows; the explicit repository file supplied the API.
No failure was suppressed or ignored.

Format, frozen-version, workflow, documentation, generated navigation/inventory
and diff checks passed. Projections contain 17 indexes, 411 nodes, 1,793 edges
and 1,407 inventory records. No workflow-policy executable mode changed.


## Checklist, limits and landing

Preimplementation PR brief binds owner, current/target behavior, scope,
traceability, smallest oracle, research/trace/resource decisions and stops.
The final brief names the reused targets instead of a new duplicate harness.
Record failures, exact results, full-file/diff review and generated checks.
POAM-035 records the demonstrated default-policy bypass. No objective, alpha,
C-07 package or release-gate completion status changes.

Not run locally: complete engine/workspace/SDK/optional-feature suites, native
Linux/macOS candidate checks, installed nondefault-policy restore, real disk
exhaustion, capacity profiles or 24-hour soak. Existing default API signatures
and behavior remain exercised by store suites; final candidate CI supplies
cross-package qualification. Seed CI `39839610012` was canceled intentionally
and is not acceptance. Installed binding, aggregate recovery resources,
maintenance/GC and capacity remain open under C-07. Stop on lost ACK, duplicate
effect, changed source, public partial target, policy bypass, scope expansion
or failed candidate CI. Final reconciliation and exact-candidate CI are pending.
