# C-07g concurrent session admission correction

**Status:** active implementation evidence; candidate acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-session-admission`
**Owner:** [C-07d/g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Starting canonical revision 903aeae58edcdb4c12f030f802f9e879ffcacce0.
PR 72 comments 6361112630 and 6361195878 bind the pre-edit scope.
The combined 9dddf9eb candidate's real two-estate concurrent flow first failed
InvocationInProgress (78.13s): workload reads shared constant invocation IDs.
workload.rs now assigns bounded checked monotonic IDs to each read invocation,
including verification reads. Mutation, transaction and idempotency identities
remain deterministic and unchanged. Counter exhaustion is explicit. Rust
1.99.1 warned that fetch_update was renamed; used the pinned toolchain's
try_update, with strict lint verifying the final source.

The next actual flow exposed a production control-state CAS conflict (80.62s):
concurrent begin/abort and commit each read and replace the same SessionState.
The canonical bounded TransactionGate already serializes commits but did not
cover those metadata transitions. begin_transaction_with_intent, abort_transaction
and preview_transaction now acquire that owner before loading session state.
Session renewal and closure use the same gate. There is no second mutex map,
unbounded retry, swallowed conflict, new configuration or public format.
Queue count/bytes/deadlines, cancellation and release use existing enforcement.

Reviewed complete begin/abort/preview transitions, commit gate acquisition and
receipt publication, renewal/closure state replacement, query_transaction's
begin/commit/cancel composition, TransactionGate admission, and all internal
call sites. Query-transaction composition owns no outer gate, so each phase
acquires/releases without reentrancy. Existing expiry, auth, policy and exact
replay behavior remain; this is not a claim that all unrelated control families
are globally serialized. The characterization starts eight concurrent workers,
each making eight begin/abort/exact-replay sequences, then checks no admission
leak and admits another transaction. Its native focused run passed (3.36s).

Files changed: workload.rs, engine/transaction.rs, engine/session.rs,
engine/tests/lifecycle.rs, this record, POAM-035 and generated navigation/inventory.
Source traceability stays at existing session/transaction authority; no
implementation was removed or moved. Research was unnecessary: the concrete
native failure and existing CAS/gate contracts define the repair. Existing
bounded error/receipt diagnostics suffice; no new payload trace is added.

Final combined acceptance and remaining native candidate evidence are recorded
below before publication. C-07 and alpha status remain open; no capacity or
24-hour result is inferred. Stop on new authority, altered idempotency, changed
storage format or hidden conflict handling. The PR candidate gate still applies.

## Final local acceptance

Combined baseline 9dddf9eb126cce2eed951e71028dfae8b45a53e3 with reviewed validation overlay
SHA256 1d1f6120ae31a8f3b87d6d75d3d75b4568350baa5e6354afced44115e486c87d passed:

- Native two-estate ordinary-binary concurrent flow: 116.61s, two estates,
  four clients each, insert/update/retire/recreate, six foreign-session denials,
  two exact reopen histories, unchanged retained page digests through source
  cancellation, and actual output/pin release after final owner drop.
- Engine library: 160 passed, 2 ignored, 23.96s.
- Public contract: 39 passed, 0.24s (includes separate cursor/checkpoint
  projection corrections being carried in their owning PRs).
- Direct-read source closure: 1 passed, 0.19s.
- Strict engine/contract all-target and CLI installed-reopen/lifecycle Clippy:
  passed, 14.68s.

These are local Windows changed-source observations, not a native fleet
capacity envelope or 24-hour qualification. The exact overlay and file hashes
were retained in the workspace before canonical propagation; validation-only
copies were removed only after their paths and hashes were recorded, leaving
the real edits in their owning worktrees. Final source equivalence is checked
after merges. Both earlier failure classes remain recorded above.
