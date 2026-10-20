# D-01 native invocation ownership correction

**Status:** active bounded reliability evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-invocation-correction`
**Owner:** OBJ-07, D-01, POAM-006

Starting revision: `9de0a886c80551dd09ed2006ac65a28dd596aa62`.
Pull request: [development PR 32](https://github.com/rrflow/rrflow-development/pull/32).
This corrects the [initial native qualification receipt](d-01-native-installed-lifecycle.md)
without rewriting its local results. The PR's anticipated production-correction
scope was made exact before production edits.

## Failure and bounded change

[Candidate 39431752847](https://github.com/rrflow/rrflow-development/actions/runs/39431752847)
failed: Windows existing-project backup returned non-retryable
`IdempotencyConflict`; fresh-project passed. Linux fresh/existing startup
exceeded the 60-second announcement bound. macOS passed. These failures were
not waived. Large debug executables were a suspected Linux resource factor,
not a measured root cause.

`RrdEngine::begin_invocation` inserted into the active map before testing
occupancy, replacing the original owner's thread on rejection.
`complete_invocation` checked only caller coordinates, so a stale completion
could remove a later admission even with identical public metadata.
Both defects are independently reproduced in deterministic engine tests.

The canonical replacement remains in `engine/invocation.rs`: vacant-entry
admission preserves existing ownership; a private `Arc` token binds completion
to one admission; validation, completion audit and removal are serialized under
the active-map lock. Clones retain the same token. A rejected overlapping call
gets a completed failed audit and a retryable `InvocationInProgress` conflict.
`engine/core.rs` stores the reservation; `engine/error.rs` classifies that
transient error. Permanent payload/idempotency conflicts remain permanent.
Existing HTTP/WebSocket mappings already project the engine's retry flag.
No durable format, operation catalogue, automatic retry or parallel authority
is introduced.

`engine/tests/security.rs` characterizes both failures for persistent rrflowKV
and volatile rrflowMX. The first test uses a second thread to expose stolen
ownership and verifies the rejected attempt's completed audit. The second
reuses identical public metadata and proves an old handle cannot release the
new reservation.

The installed-lifecycle client now explicitly allows 60 seconds for physical
backup/restore verification, retaining its default retry count. Lifecycle CI
omits dev/test debug symbols and logs actual executable size. The actual
executable identity and 60-second startup bound remain. CI operations and
POAM-006 describe these limits; navigation and file inventory are regenerated.
No files are moved or deleted. Unrelated worktrees remain unchanged.

## Inspection, decisions and limits

Owner reads carry forward from the initial receipt: README/AGENTS, OBJ-07,
dependency spine, Gate D, POAM-006/-009/-010, ADR-0002, installed lifecycle,
change-authoring procedure and initial receipt. Correction inspection covers
the complete changed admission/completion/error/active-map definitions,
new tests, lifecycle client block, workflow jobs, and the security
`append_audit`/`commit_with_audit` path plus HTTP/WebSocket error projections.
The audit path does not reenter the invocation map. Changed-file diff review
maps each replacement to the two characterization tests and native lifecycle.

External research/source adaptation is unnecessary for these local defects.
Existing security receipts are the audit authority; the new completed rejection
uses them. No new runtime tracing authority or performance claim is added.
Executable size is a test diagnostic. Stop on unrelated scope, weakened
authorization/identity, lost project state or failed candidate checks.

This is not default five-second backup qualification, generic uncertain-outcome
recovery, model routing, durable reasoning, PostgreSQL compatibility, sync,
graceful Windows shutdown or signed clean-machine qualification. Their owning
gates remain open. No objective or roadmap status changes; POAM-006 records
this bounded correction and keeps its broader deficiency open.

## Executed evidence

The first filtered test reused stale shared build artifacts and ran zero
matching tests; it is not evidence. After `cargo clean -p rrd-engine`, a
missing `ServiceErrorKind` test import failed compilation and was corrected.
The two regressions then failed on the old implementation: rejected admission
stole ownership, and stale completion was accepted. After the production fix
and the final audit assertion, `cargo test --locked -p rrd-engine --lib
overlapping_invocation -- --nocapture` passed **2 tests**, zero failures,
104 filtered out, in 0.02 seconds on Windows.

`cargo test --locked -p rrd-engine -p rrflow-cli` passed **181 tests** across
21 result groups, zero failed/ignored tests and zero warnings. The native
lifecycle target passed all five tests, including both real process cases,
in 151.68 seconds. The raw local log is
`target/rrflow-native-correction-suite.log`.
`cargo clippy --locked -p rrd-engine --lib --tests -- -D warnings` and
`cargo clippy --locked -p rrflow-cli --test installed_lifecycle -- -D warnings`
both passed. Formatting and whitespace checks passed. Workflow policy passed;
navigation reported 323 nodes/1397 edges; inventory contains 1237 records;
documentation policy passed with 324 statuses/322 coordinates.

The complete correction diff was reviewed. Native Linux/macOS and the broader
candidate partitions are delegated to the mandatory CI run; their results are
not inferred from these Windows passes. Candidate CI and the exact merge
revision remain attached to PR 32. This linked record
owns scope, traceability, failure oracles, surfaced errors, limits and status
decisions for the correction; it is not a new planning or release authority.
