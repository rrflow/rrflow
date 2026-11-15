# POAM-038 — attunement acts on an unverified source binding

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-038`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

The parent ledger owns priority and lifecycle status. This detail records a
verified gap against [OBJ-14](../../objectives/rrflow-1.0-alpha.md#required-outcomes);
it cannot close a roadmap gate independently.

## Observed deficiency

The [selected-source worker package](../../evidence/change-journals/gate-g/task-worker-selected-source.md)
separates selected-file identity and current eligibility from unrelated inventory
head and Git changes. Its native prerequisites and matching Connectome PR39 have
landed through protected CI. The [assessment package's dependency evidence](../../evidence/change-journals/gate-g/task-controller-assessments.md#dependency-evidence-and-exact-limits)
records the actual source-bound worker and exact merged-desktop recovery; this
does not establish general attunement or project-effect closure.
The existing `make_attunement_replan` boundary accepts only eligible unresolved
jobs with Connect/Inventory checkpoints. Completed jobs and later-effect jobs
have no general inventory refresh path. Older inventories lacking the derived
`source_file` query projection therefore remain unavailable to the proposed
picker until a grounded refresh path is implemented. Preserve prior effects and
receipts; neither private head replacement nor treating old metadata as current
is remediation. This limitation remains open under D-02/D-03 and this POA&M.

Verified at development `main` `04fae276`:

**Correction, 2026-11-11, at main `ab32ec16`.** The first bullet below no longer holds: `control_attunement` re-observes `project_inventory_sha256(root, profile)` on `Start`, `Resume` and every non-cancel `Step` and returns `AttunementSourceChanged` on a mismatch (`attunement_source_observation`, `crates/authority/rrd-engine/src/engine/attunement.rs:244-256`, used at `:813-820`). The Connect checkpoint still takes `plan.source_sha256` as its `input_sha256` by the checkpoint-chain rule, and installation verification still reports as described, so the record stays open for those parts.


- [`control_attunement`](../../../crates/authority/rrd-engine/src/engine/attunement.rs)
  admits `Start` when the command's `plan_sha256` equals the pending plan's. It
  does not recompute the project inventory the plan was derived from.
- The Connect checkpoint copies `plan.source_sha256` into its `input_sha256`
  without recomputing it. The checkpoint, and the receipt committed with it,
  therefore claim a source binding that no one re-verified at that boundary.
- [Installation verification](../../../crates/authority/rrd-engine/src/engine/installation.rs)
  computes `attunement_source_current` and reports `Passed` when it is false;
  the [installation test](../../../crates/authority/rrd-engine/src/engine/tests/installation.rs)
  asserts exactly that after a README edit. Reporting drift during
  verification is correct; acting on the stale binding is the gap.
- [Restore publication](../../../crates/authority/rrd-engine/src/engine/restore_publication.rs)
  rejects with "a storage root changed since the publication plan; plan again",
  but no test changes a storage root between planning and applying it.

Installation apply, endpoint configuration, tail repair, attunement control and
transactions already refuse changed inputs with no write, with tests. This
record covers only the boundaries above.

## Impact

The [embedded replan candidate](../../evidence/change-journals/gate-d/d-02-observed-source-replan.md)
adds a bounded installation-precondition observation before attunement work
and an explicit successor job with atomic control receipts. Local ordinary-edit,
fencing and restart tests passed. That observation is not a D-03 snapshot and
does not close the race with an uncoordinated editor after the scan; public
operations, source revision, complete interruption and exact-candidate CI
remain outstanding. This POA&M remains open.

[ADR-0002](../../decisions/0002-adaptive-reasoning-governed-effects.md) requires
the engine to capture or refresh the affected source boundary and reject stale
or conflicting effects before commit. Once the Inventory and later phases act on
project sources, a plan made before an edit would drive those effects, and its
checkpoints would present evidence for sources that no longer exist.

The [installed build-output characterization](../../evidence/change-journals/gate-d/d-03-build-output-precondition-gap.md) demonstrates that changing only a Git-ignored `target` artifact changes the current whole-project installation precondition and prevents the original preview from applying. This is a source/effect-scoping gap and a usability problem for ordinary builds, not a reason to bypass the stale-input check. Remediate with an explicit, safely bound scope and the D-03 source-inventory boundary; preserve tracked source and project-mutation protection. The observed small fixture does not prove a large-project bound failure.

## Owning gates

D-02b durable worker, D-03 committed inventory and D-11 restore/repair.

Roadmap owner: [Gate D](../../roadmap/rrflow-1.0/gate-d.md#project-workflow-build-packages).

## Closure evidence

Every attunement phase recomputes its bound source identity (the project's
source revision where it has one, and its inventory digest) before its effect,
and commits a checkpoint only when that identity matches the plan. A mismatch
returns a typed stale-plan failure with no project or engram write and requires
an explicit re-plan; the accepted plan is never edited in place. Tests change a
project file between plan and `Start` and between two phases, through the
shipped `rrflow`, and a test changes a storage root between restore planning and
publication. All pass at an exact revision.
