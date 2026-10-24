# D-01.3 live installation exclusion failure

**Status:** active failing characterization; repair and candidate acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-install-exclusion-oracle`
**Owner:** OBJ-07 prerequisite, D-01.3, POAM-006

Starting revision `0a5078b58c48331f878332ead8e533ea60776ef3`, tree
`9fa25202c611ef7fa6a78c294bb546e9cc2a71d6`, was synchronized with development/main.
D-01.2 passed candidate run 39635463758 and merged through PR 49. This package
preserves its filesystem capabilities and storage writer locks.

The pre-edit change brief was prepared as `target/d01-exclusive-install-brief.md`.
GitHub rejected creation of the draft PR at that baseline with "No commits
between main and agent/d01-exclusive-install". This first commit therefore
contains an executable failing regression, rather than a planning-only commit.
The brief is attached to the package PR before production implementation.
This sequencing exception is explicit; it is not evidence of a pre-edit remote PR.

The target is one project-wide native lease from admission through final intent
acknowledgement. A contender must return a bounded conflict without advancing
an active owner's installation. No PID, file age, or timer grants ownership.
The lease, preview, retained empty file, and native failures will be implemented
at the existing installation/private-file boundary. Stop on duplicate authority,
unpreviewed effects, secret output, or failed native enforcement.

## First oracle

`cargo test --locked -p rrd-engine --lib
same_plan_contender_cannot_advance_an_active_installation_intent -- --nocapture`
failed with exit 101: zero passed, one failed. The owner was paused in a real
process at `IntentPublished`; a second process applied the same sealed plan and
returned `success` instead of `conflict`. This confirms recovery can advance
while the original owner is still alive, before its storage admission.

The parent launches its test executable with an exact ignored helper filter.
Barriers and result files live outside the project; waits are bounded, stdin and
output are closed, and owned children are killed/reaped on failure. The error
contains only an outcome label, not credentials or intent bytes. No production
failpoint or environment variable was added. The local log is retained under
ignored `target/d01-exclusive-first-oracle.log`.

## Scope, review and checklist

Created `engine/tests/installation_concurrency.rs` and registered it in
`engine/tests.rs` under `crates/authority/rrd-engine/src/`. These files were read
in full and the complete diff reviewed. No implementation was moved, removed,
or replaced. The reviewed production boundary is
`installation.rs::apply_installation_with_observer`, its eight observer stages,
`installation/inventory.rs`, `estate_layout.rs`, and private-file native opens;
the implementation receipt will enumerate complete source review for its edits.
Owner reads include README, alpha objective, dependency spine, Gate D,
POAM-006, change-authoring procedure and agent bootstrap.

Research uses Rust File::try_lock, Microsoft LockFileEx, and Linux flock
documentation linked in the change brief. No upstream source is adapted.
Preserve existing redacted lifecycle signals; this fixture supplies controlled
process evidence, not a performance benchmark or private reasoning trace.

Navigation and execution inventory are regenerated for this source/evidence
addition. The full engine suite and candidate CI are not claimed passing:
the intentionally failing oracle must pass after the repair. The linked repair
receipt and PR will record the final source, tests and candidate results.
POAM-006 gains this verified gap. D-01, OBJ-07 and alpha remain open.
