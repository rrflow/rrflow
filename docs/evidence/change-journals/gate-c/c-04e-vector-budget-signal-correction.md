# C-04e vector budget signal correction

**Status:** active change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-04e-vector-budget-signal-correction`
**Owner:** typed budget-denial correction receipt within PR 39; Gate C retains completion authority

This supplements the immutable
[stamped catalogue receipt](c-04e-stamped-vector-catalogues.md) within
[PR 39](https://github.com/rrflow/rrflow-development/pull/39). Starting revision
`61236f06cf43a12bb85a9d4ba7fbe21d83f049c5`. The PR brief was refined before
editing to include the shared storage-error conversion; owning outcomes remain
OBJ-03/OBJ-10, C-04 and POAM-028. No gate or ledger status changes.

Review found that a typed storage-read budget denial became `Internal` through
`From<rrd_store::Error>`. The first strengthened request-level test reproduced
that exact failure. `engine/error.rs` now maps `RuntimeReadBudgetExceeded` to
the existing `ConfigurationLimit` variant, which exposes `ResourceExhausted`
and is not automatically retryable. Unrelated storage faults remain `Internal`.
No new enum value, public schema, provider state, trace vocabulary, dependency,
format or durable effect is introduced. The request's measured budget still
fails closed before producing a result; only its error classification changes.

Reviewed in full: engine/error.rs and the focused catalogue_reads.rs test
module. The store error definition and HTTP/WebSocket error mappings were
inspected to confirm existing public code/status projections. Both the code
diff and preserved ordinary-storage error path were reviewed. Scope includes
those two source files, this correction receipt, the verified finding in
POAM-028, and regenerated navigation/inventory. The original journal is not
rewritten. Stop conditions remain a weakened error/authorization boundary,
a changed public schema, or a failing candidate corpus.

No new external research or source adaptation was needed: the defect is in
existing local typed-error conversion. Existing ReadEvidence and error
projections carry the signal, so no extra telemetry owner or debug state is
needed. Validation begins with the real budgeted search regression, requires
`ResourceExhausted` and non-retryable classification, and checks that an
ordinary substrate failure still classifies as internal.

The initial request regression failed with `Internal` versus
`ResourceExhausted`; after correction the focused catalogue tests, owning
engine suite and strict Clippy are rerun. Exact command/results and policy
checks are finalized below before commit. The final candidate CI must be
rerun at the corrected revision; the earlier candidate's checks cannot stand
in for that proof. Native/SDK/optional-feature coverage remains the CI chain,
not a local claim, and no production-scale or complete-engine claim is made.

Checklist: baseline/brief and replacement scope recorded; first failure
captured; changed source read and diff reviewed; research/resource/debug
choices recorded; POAM finding/remediation recorded. Package/lint/policy and
exact-candidate CI receipts remain required before landing through PR 39.


Final local validation: `cargo test --locked -p rrd-engine --lib catalogue_reads`
passed all 3 tests; `cargo test --locked -p rrd-engine --all-targets --no-fail-fast`
passed 173 tests across 12 targets; and
`cargo clippy --locked -p rrd-engine --all-targets -- -D warnings` passed.
Repository documentation, workflow, generated navigation/inventory, format and
diff checks pass after regeneration. No failed runtime tests remain. The only
failed oracle was the deliberate pre-fix error-kind assertion. The prior
candidate CI run 39494718969 passed, but it covers the prior revision only;
PR 39 must bind a fresh passing run to the corrected head before merge.
