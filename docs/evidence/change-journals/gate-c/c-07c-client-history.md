# C-07c installed client response-loss history

**Status:** active bounded implementation evidence; candidate CI acceptance remains PR-bound
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07c-client-history`
**Owner:** OBJ-02 prerequisite, [C-07c](../../../roadmap/rrflow-1.0/gate-c-07.md)

## Baseline and change brief

[PR 58](https://github.com/rrflow/rrflow-development/pull/58) binds the brief
before implementation. The [baseline seed](c-07c-client-history-baseline.md)
starts at `ecfeea332e6d7a0138b2dbb6c393e0d734d50db9`, tree
`a1901442d72742d3941f8d1e998fdde43e67af0c`, clean public development main.
Branch `agent/c07c-installed-history` records exact development remote/ref/SHA
before each push in `target/c07c-pushes.jsonl` and the public PR history.

After PR 57 merged, public main advanced to
`b251877e3ed55bf1be75a38ea666d889e69ae023`. The implementation branch merged
that accepted planning tree as `444ee584` without rewriting history. Only the
generated file inventory conflicted. The generator initially refused the
unmerged index, as designed; after resolving that index entry it regenerated
from the combined owners. A task-owned stash preserved the in-progress code;
restoring it required the same mechanical inventory reconciliation. No source
implementation conflicted and no unrelated changes were discarded.

This first bounded C-07c package extends the existing real-primary-binary
fresh/existing fixtures. It adds client-observed response loss, independent
requested values, stale-transaction rejection and quota recovery, then three
forced process kill/reopens with exact receipt replay. Precise internal
WAL/visibility/publication pauses and the dedicated 100-cycle profile corpus
remain subsequent C-07c work. No C-07 or alpha completion is claimed.

## Implementation and traceability

Created `tests/support/response_loss.rs` and `storage_history.rs` under the
CLI adapter. The existing `installed_lifecycle.rs` invokes the history after
its original restore proof; all five original cases and receipts remain.
The relay forwards exactly one bounded authenticated commit to the fixture's
loopback engine and delivers zero response bytes to the client. It bounds
headers, request/response bytes, accept/connect/read/write time and thread
lifetime. Wire credentials remain private memory, never evidence output.
No production build gains a fault switch, endpoint or dependency.

Four predeclared logical commits use two claims each: A acknowledged, B with
an intercepted successful response but an unknown client result, C definitely
rejected at a stale cursor, D valid after C is aborted and both transaction
slots are demonstrably reusable. Three restarts retry A/B/D under their
original identities. The independent oracle requires six exact requested
rows, C absent, stable receipts and exact runtime cursor ranges 4-5, 6-7 and
8-9. The recovered head must remain 9 after retries; duplicate applications
cannot be hidden by an unchanged latest value.

The existing `rrflow-eval` ledger is a test-only dependency; Cargo.lock changes
only the CLI development-dependency edge. Its reconciler runs at each reopen.
The offline runner admits both histories and closed bounded fields, binds
their build to the existing lifecycle receipt and rejects incomplete schedules
or inconsistent attempt counts before retaining artifacts. It does not become
a second semantic reconciler. New Python cases falsify missing/duplicated
modes, changed build, schedule/count drift, nested unknown fields, invalid
timing/digests and duplicate readback identities.

No implementation is removed, moved or replaced; therefore replacement
traceability is not applicable. The test boundary reuses the existing process
guard, staged executable, engine installation, public client and C-07a oracle.
Production formats, authority, configuration, version and default dependencies
remain unchanged. Existing fixture behavior is characterized by the baseline
five-test pass and must remain green with the extension.

## Reviewed inputs and research

Reviewed C-07, OBJ-02, POAM-033/035, authoring/navigation and native CI owners;
the CLI Cargo manifest, installed lifecycle/process guard, client session,
operation/error/retry contracts, engine transaction intent/rejection/replay and
receipt lowering, query read-cursor binding, C-07a ledger/model/digest, native
database stale/rejection fixture, and offline runner receipt admission/policy
tests. Full modified-source rereads cover
`crates/adapters/rrflow-cli/Cargo.toml`, `tests/installed_lifecycle.rs`,
`tests/support/storage_history.rs`, `tests/support/response_loss.rs`,
`scripts/ci/run_offline_lifecycle.py`, `scripts/ci/test_offline_lifecycle.py`,
`docs/operations/ci.md` and the owning `gate-c-07.md` above. The parent test's
`RunningEngine`, staged-distribution and output guards remain unchanged;
the new helper owns `run`, independent expected values/receipt digests and
`ResponseLoss`. The collector adds `storage_history_receipts` at the existing
qualification boundary. Canonical `AcknowledgementLedger::reconcile` and
`rrd_client::Error` were checked against the retained observations. The full
authored diff is reviewed; the Cargo.lock dependency-edge diff and generated
navigation/inventory are mechanically reconciled rather than prose-copied.

Existing C-07 reliability/acknowledgement research suffices; no upstream code,
new algorithm or vendor runtime is adapted. Bounded monotonic attempt timing,
expected/readback digests and sanitized native receipts are the necessary test
traces. This is process-termination evidence, not loss of OS/device caches,
actual ENOSPC, a resource envelope or a capacity benchmark.

## Validation and surfaced failures

- Baseline `cargo test -p rrflow-cli --test installed_lifecycle --locked --
  --nocapture`: five passed on Windows, zero failed/ignored, 223.27 seconds.
  The tested primary executable was 156,127,744 bytes. This resolves the
  pending baseline result in the seed record.
- `cargo test -p rrflow-eval --test storage_contract --locked`: 11 baseline
  tests passed; canonical history checks are reused without alteration.
- Initial `--locked` compile correctly refused the new development-dependency
  edge; `cargo test -p rrflow-cli --test installed_lifecycle --no-run --offline`
  updated that edge and compiled successfully without dependency acquisition.
- First Python oracle failed because `storage_history_receipts` was absent;
  after implementation and adversarial extension, all 15 offline-runner tests
  passed with `python -m unittest discover -s scripts/ci -p test_offline_lifecycle.py`.
- First focused real-process run failed after the original lifecycle passed:
  the test expected A's last runtime cursor 4, but observed 5. Inspection of
  native receipt lowering confirmed cursors advance per mutation. The oracle
  was corrected to the explicit two-mutation ranges above; it also asserts
  the recovered head. Build receipt review restricted retained identity to
  the same four digest fields as the original offline receipt.
- Corrected focused fresh-project run passed in 290.40 seconds: one passed,
  four filtered as intended. Its ledger reconciled four logical commits,
  13 attempts, 11 ACKs, one rejection, one initially unknown response and no
  unresolved/violating result across three forced reopens.
- Subsequent relay review tightened the write path to recompute the remaining
  aggregate deadline after every short write; native per-call timeouts alone
  must not restart the whole allowance. Zero writes now fail explicitly.
- The first full owning-suite run passed all five cases in 290.53 seconds.
  Its binary preceded that last relay tightening; it is not evidence for the
  tightened write loop. The real output also passed `lifecycle_receipts` and
  `storage_history_receipts`, admitting both modes and their 13-attempt ledgers.
- `cargo clippy -p rrflow-cli --all-targets --locked -- -D warnings` passed.
  `cargo test -p rrd-engine --test workspace_architecture --locked` passed all
  31 cases. `cargo fmt --all --check` and `git diff --check` passed. The final
  Python offline-runner suite passed all 15 cases in 0.347 seconds.
- Final `cargo test -p rrflow-cli --test installed_lifecycle --locked --
  --nocapture`, including the tightened relay, passed all five tests in
  318.41 seconds, zero failed/ignored/filtered. Both emitted histories passed
  the real Python collector and its required-five-case result check; each
  retained four logical operations, 13 attempts, three forced reopens and zero
  unresolved results or violations. No test deadline was increased.
- `python -X utf8 scripts/ci/check_documentation.py` passed: 392 statuses and
  390 classified coordinates. `python scripts/knowledge/render_navigation.py
  --check` passed: 17 indexes, 391 nodes, 1,712 edges.
  `python scripts/ci/build_execution_inventory.py --check` passed: 1,374
  records. Both generators run after the final journal edit; whitespace is
  checked again before commit. Candidate CI is not represented as a local pass.

## Checklist and remaining work

The PR brief binds owning outcome, baseline/current/target, exact paths,
smallest failure oracle, commands, research, trace/resource decisions and stop
conditions. Unrelated state is preserved. No public API, storage format,
production effect or runtime permission is changed. The one existing ledger
checker is reused; native receipts distinguish a transport unknown from a
rejected mutation. The code comment explicitly limits quota proof to cleanup
after abort, rather than implying automatic transaction-slot release.
Candidate CI remains mandatory and its exact run is recorded in the PR.

Not run locally: Linux/macOS native isolation (requires those hosts), Windows
ACL/WFP/principal isolation (restricted to disposable hosted CI), internal
crash-barrier/100-cycle corpus, real ENOSPC, memory/capacity sweeps, 24-hour soak,
DuckDB execution and PostgreSQL compatibility. The first three native hosts'
ordinary installed and isolated fixtures must pass candidate CI; the broader
C-07/D/H capabilities remain unqualified rather than inferred from this test.

No roadmap or POA&M item closes. C-07c still needs precise internal pauses,
compound recovery and its larger dedicated native restart matrix; d-h retain
hard resource, lifetime, real exhaustion and sustained acceptance. PR 57's
DuckDB plan remains independent. Stop on lost ACK, partial batch, inconsistent
receipt, leaked owned resources/credentials, an unjustified timeout increase
or a larger production repair; preserve evidence and scope that repair before
mutation. The final PR records its exact CI and merge evidence.
