# C-07d typed mutable admission and configured inspection

**Status:** active locally verified implementation; accepted-base reconciliation and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-limit-propagation`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Baseline and change brief

[PR 67](https://github.com/rrflow/rrflow-development/pull/67) bound the scope
before implementation. Accepted main was
`dde6ef86d278b8c59cf1f61ebc2d3923186396d5`, tree
`bc76b62d5a500395fb8e1193dcd4d94eeac79ada`. The
[baseline record](c-07d-limit-propagation-baseline.md) was committed at
`fb4f1e880fbede3a097ea58f6464c2dd82012094`. Local dependency merge
`610ba809` incorporates PR 65's native policy at `948f7170`; its two generated
index/inventory conflicts were resolved by regeneration after staging the
conflict resolution. That pending dependency is not accepted-main evidence.
Reconcile accepted PR 64/65/66 before this package's final candidate.

Native byte/version rejection previously became generic substrate/internal
failure and retained prepared domain intent. Store inspection could not select
native replay limits. Preserve the typed failure across store write/open and
engine adapters, clear intent only in the existing absent-outcome branch, and
allow options-aware read-only inspection. Unknown I/O remains uncertain.
No runtime policy, installed configuration, format, version, dependency,
recovery repair or independent authority was introduced.

## Implementation and traceability

- `rrd-store/src/error.rs`: retain native `MutableResource`, `usize` limit and
  observed charge exactly; native units are process-local and unchanged.
- `rrd-store/src/rrflow_kv.rs`: both native-open wrappers preserve typed limit
  errors while retaining contextual diagnostics for other failures. Inspector
  default methods delegate to options-aware methods and the existing native
  read-only reader; inspection has no write/repair path.
- `rrd-store/src/lib.rs`: reexport the existing native option/limit types.
  There is no second policy type or engine-to-native crate dependency.
- `rrd-engine/src/engine/error.rs` and `transaction.rs`: classify known mutable
  admission as nonretryable resource exhaustion. Only confirmed absence of a
  durable domain outcome permits clearing intent; the existing present-outcome
  reconciliation and unknown-error paths remain intact.
- `engine/core.rs::compose_persistent` gains parent-module visibility only.
  Tests compose the actual persistent store, object tier and engine using this
  production helper. Both reopens reuse the same configured limits.
- New `rrd-store/tests/mutable_admission.rs`; extend the existing
  `semantic_commit_atomicity` and engine `tests/write_admission` histories.
  The original encoded-limit history remains as a separate regression.
- Candidate workflow/policy require the new store target in all three native
  partitions. Existing semantic/engine admission commands execute expanded
  histories. C-07, POAM-035 and CI notes describe applicability; generated
  journal navigation and file inventory remain replaceable projections.

No implementation was deleted or moved. The existing canonical evidence
helpers were parameterized rather than copied. All physical fixture rows,
including derived graph/index/audit/receipt bytes, must remain identical after
rejection; a subsequent valid claim preserves graph and function receipts and
reopens exactly. Raw store histories cover prior ACK, rejected deletion plus
inserts, reopen, valid deletion/insertion and another reopen for both resources.
Inspection covers eight typed denial paths, zero-budget denial, byte-for-byte
filesystem preservation, adequate-budget exact read/control identity and
successful writer reopen. The engine history adds authenticated transaction
intent, stable replay, explicit abort/quota reuse and a distinct subsequent
claim, with exact domain evidence before and after restart.

## Research, diagnostics and review

No new external algorithm, API or source adaptation was needed. The existing
native admission and engine uncertainty contracts define the oracle. Preserve
runtime traces; bounded assertions report resource identity and whether intent
was retained. Logical resource admission is not allocator/RSS measurement.
Finite component histories do not prove installed configuration, complete
knowledge/model flow, whole-startup memory, native disk exhaustion or soak.

Read and reviewed the README owner chain, C-07/POAM-035, change-authoring and
traceability procedure, native mutable policy/prepare/replay and inspection
boundaries; complete changed store error/lib/rrflowKV files, core/service-error/
transaction source and admission histories, semantic atomicity and installed
inspection fixtures; workflow/policy and affected CI/roadmap/POAM records.
Changed source and tests were reread, including unchanged transaction outcome
and snapshot-checkpoint behavior surrounding the edits. Reviewed the complete
diff and preserved executable mode `100755` on the workflow policy.

## Commands, failures and results

Local commands used Windows and an isolated Cargo target directory for this
worktree; no shared-target cache result is used as evidence.

| Command / oracle | Observed result |
|---|---|
| Initial engine focused build | Failed on a missing test import for `RrflowKvStore`; imported the existing store type. |
| Engine `write_admission` before propagation | 1 passed, 2 failed: encoded regression passed; both native resources became `Internal`. |
| Mutable-only reproduction with intent diagnostic | 0 passed, 2 failed; both recorded `intent_retained=true`. Byte limit 131,072 versus observed 1,408,663; version limit 256 versus observed 1,543. |
| `cargo test -p rrd-store --test mutable_admission --test semantic_commit_atomicity --locked` | 7 passed, 0 failed. |
| `cargo test -p rrd-engine --lib engine::tests::write_admission --locked` | 3 passed, 0 failed. |
| `cargo test -p rrd-store -p rrd-engine --locked` | 417 passed, 0 failed, 2 existing ignored cases across 49 result groups. |
| Initial strict all-target clippy | Failed on one unnecessary test clone into a one-item slice; replaced it with `std::slice::from_ref`, without a suppression. |
| `cargo test -p rrd-store --test mutable_admission --locked` after lint correction | 2 passed, 0 failed. |
| `cargo clippy -p rrd-store -p rrd-engine --all-targets --locked -- -D warnings` after correction | Passed. |

Exploratory path lookups for nonexistent `crates/runtime/rrd-engine`, store
`tests/inspection.rs` and `src/repository.rs` failed without effects; tracked
path discovery resolved the actual authority and installed-lifecycle files.
The staged new target participates in architecture inventory; no architecture
exception or ignored case was added.

The first documentation check rejected this uncommitted record's missing
`active`/`historical` status classification; the metadata was corrected before
commit. Formatting, frozen-version and workflow checks passed. After that
correction, documentation, generated navigation/inventory and staged diff checks
passed: 17 indexes, 409 navigation nodes, 1,785 edges and 1,405 inventory records.
The final reconciled candidate must repeat the applicable checks.

## Checklist, limits and landing

The preimplementation brief binds outcome, scope, owners, smallest oracle,
research/trace/resource decisions and stop conditions. Exact before/after
histories, failures, corrected code, review, generated projections and not-run
checks are recorded here. No alpha/objective/gate completion status changes.
POAM-035 records the reproduced adapter gap and remaining installed binding.

Not run for this local implementation: native Linux/macOS candidate lanes,
full workspace/SDK/optional-feature CI, installed nondefault-budget startup or
restore, native capacity exhaustion, capacity profiles or a 24-hour soak.
The seed CI `39837148937` was intentionally canceled; it is not acceptance.
Final candidate CI and accepted-base reconciliation remain required before
merge. Stop on unknown-error misclassification, unproven intent clearing,
partial domain effects, inspection mutation, expanded scope or failed CI.
