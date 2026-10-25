# C-07 native WAL repetitions: acceptance binding and retained result

**Status:** active component evidence; sustained qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07-native-wal-acceptance-binding`
**Owner:** OBJ-02 / C-07f,h / POAM-035 / PR129

Starting revision `392ca9f6645268228a6bd00e5214e2557a706539` adds a required
macOS repetition step, but its evidence validator did not yet require that
step. The PR description bound this refinement before editing. The existing
`CandidateCiReceipt::validate` now requires exactly one successful repeated
APFS qualification step on macOS. Missing or skipped macOS steps are rejected;
other platforms may skip that platform-specific step. This changes the existing
acceptance boundary, not storage behavior or a second evidence authority.

Read the complete validation and fixture/test functions in
`storage/acceptance.rs` and `storage/acceptance_tests.rs`, the workflow's native
capacity steps, previous native-WAL journal and owner procedure. Scope is those
two evaluation files, this journal, the separately linked
[descriptor failure record](c-07g-native-descriptor-failure.md), owner links and
generated indexes. Existing publication receipt assertions remain required.
No replacement of engine implementation, external research, new runtime trace,
resource-budget change or filesystem mutation is needed for this validation
binding. Original native diagnostics and error/recovery oracles are preserved.

## Native result and its limits

[macOS job 121049051197](https://github.com/rrflow/rrflow-development/actions/runs/40153255185/job/121049051197)
completed successfully for candidate head `392ca9f6`. Its eight full-matrix
cases and six additional WAL repetitions produced 14 receipts, each with zero
unresolved calls and zero violations. All eight WAL cases observed real native
ENOSPC at the selected write. Before fill their WAL files had roughly 2.18 MB
logical length and 3.15-3.21 MB allocated; allocation did not change during fill.
The prior intermittent failure did not recur. This passing characterization
does not establish its cause or close the recorded defect.

The complete job log is retained at operator
`target/pr129-macos-40153255185.log`, SHA-256
`78a4e22cc85ab7c8904810f98910fba53d5b30f5be59f9458bd4d684e23eb67e`.
Native source/binary identities stay attached to those receipts, not this later
validator revision. The full candidate pipeline failed its verify job because
the journal text changed after the file inventory was generated. Retained
verify log SHA-256 is
`1ac16517abbb9304fd3ad578538ecd23abb22bee6c6e9dfe443270cb246d6213`.
The inventory is now regenerated after final journal text, with its check run
again before commit. The earlier journal's local check does not claim the
subsequently changed committed inventory passed.

## Validation and handoff

- Focused `native_apfs_wal_repetitions_cannot_be_missing_or_skipped`: one passed.
- `cargo test --locked -p rrflow-eval`: 55 passed (41 library, three provider,
  eleven storage tests).
- Strict all-target evaluation Clippy and workspace format check passed.
- Documentation, workflow policy, navigation, inventory and whitespace checks
  passed on the final evidence/projection state before commit.

The final source diff preserves the original fixture assertions and existing
negative receipt tests. Checklist: pre-edit brief, exact baseline, canonical
validator ownership, missing/skipped-step oracle, owning-suite evidence,
retained native and verify results, immutable follow-up journal and generated
projections. A Windows console output encoding error interrupted one read-only
conflict inspection; explicit UTF-8 output corrected the inspection, with no
source change from that failed command.

No complete candidate CI, new full-corpus readback or 24-hour soak is claimed
for this revision. The independently retained EMFILE failure still requires
its bounded-handle repair. C-07, POAM-035 and alpha status remain open. The next
action is full candidate CI through normal PR and merge-queue controls.
