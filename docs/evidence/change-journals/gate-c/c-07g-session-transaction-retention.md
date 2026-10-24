# C-07g: bounded session transaction retention

**Status:** active implementation evidence; native qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-session-transaction-retention`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [110](https://github.com/rrflow/rrflow-development/pull/110)

Baseline `ac39ca617c22d15d8a80a7a578688337d172fada`, tree
`4559f550f69bad144eb8b49b6f196a7eacef50a5`, integrates fetched public main
with PR109 `9767864468555b16641b62358cd88672646711f9`. The draft PR bound
this production repair before source edits; PR109 only changes diagnostic
attribution. Product version, maturity and roadmap completion are unchanged.

The session encoded all historical transactions in one control value. The
new regression failed on both rrflowMX and native rrflowKV after 32 completed
transactions: session state was 52280 bytes, exceeding its fixed 16-KiB test
bound. The existing one-MiB control limit was not changed. This proves
unbounded session-history growth, independently of the earlier installed
campaign's generic substrate errors; it is not proof of their exact cause.

The new `engine/session_transactions.rs` owns the existing session replacement
operation and terminal-record placement within `RrdEngine`. Each replacement
moves at most one terminal transaction into a versioned, individually addressed
control record. The existing control batch commits that record, its journal
entry, and the session CAS together. A failed CAS publishes neither placement;
the caller's working state changes only after success. Open transactions,
prepared work and uncertain commit intents stay in the session. Bulk close or
expiry leaves a bounded terminal tail rather than exceeding the 64-transition
batch limit. Subsequent transitions drain that same working set. There is no
second authority, pruning, legacy decoder or runtime fallback.

Begin, commit, abort and preview perform a point lookup for a requested terminal
record when absent from the working set. The record is bound to instance,
session and transaction identity; unknown versions, mismatched identities and
open records fail closed. Complete begin/preparation/commit/abort identities,
read stamps, function receipts and commit receipts remain available. Existing
authorization, expiry ordering, quotas and uncertain-outcome handling remain
in their original operation paths. The closed-record journal event is
`transaction.retained`; session operation events remain last in their batch.

The traceability review covered README, OBJ-02, Gate C/C-07, POAM-035, authoring,
traceability and CI procedures; complete affected session and transaction
operations and types in `engine/{mod,session,transaction}.rs`; the rrflowQL
transaction caller; control validator and repository batch implementation;
and affected lifecycle, recovery, transaction-stamp, write-admission and
function test consumers. Replacement evidence maps old session persistence to
the new module's atomic CAS and point records. The source map changes only
those engine modules and their test registration; affected lifecycle/function/
recovery assertions now follow the exact terminal record and extra hash-chain
entry. New tests live in `engine/tests/session_transactions.rs`. This journal,
POAM link, generated journal index and file inventory supply supporting records.

Local source and retained failures suffice; no external implementation is
adapted. Keep existing public diagnostic categories and digests; no payload or
credential trace. Stop for changed authorization, lost replay, unbounded
publication, non-atomic placement or a broader physical-format redesign.
The complete changed functions and diff are reviewed before commit.

`cargo test --locked -p rrd-engine --lib session_transactions -- --nocapture`
first failed both growth tests and passed the existing concurrent-session
test (172.80 seconds including a dependency rebuild; tests 6.08 seconds).
After the production repair, the same filter with `--jobs 6` passed all three
(29.73 seconds including build; tests 5.96 seconds). The native test commits
768 transactions, reopens, and retries every begin/commit after absolute
expiry, including wrong-key denial, with no new journal or domain effects.
The volatile profile exercises the same semantics over 96 transactions.

The expanded focused filter passed six tests (65.19 seconds including build;
tests 38.56 seconds). It adds atomic session-CAS/terminal-key conflict, no
partial publication, immutable collision/reopen, full 256-slot close in one
two-entry batch, session/token isolation, changed begin/commit denial and
invalid terminal version/identity/state rejection. Existing concurrent-session
quota and replay coverage also passes. Logs use `target/c07g-session-*.log`.

Linux22 on the preceding PR109 candidate completed 1000 corpus acknowledgements
and readbacks plus two reopens, then failed the unchanged backup-interference
deadline after three retained segments. Campaign identity is
`ef25d276b572680dbb764fdcb9e59e38a2f13f216662b434d6dd6e93c257ea7e`;
progress SHA-256 is
`1503dfe5048a1ba6812ca3a6fa2e6a72a865243cf8392f594e16237e9a6a7713`.
Launch lasted 437324 ms with cleanup confirmed. Safe artifacts and their
individual digests are in `target/c07-linux-scale-evidence-22/`. Windows
compilation overlapped this shared-host characterization. The earlier setup
error did not recur, so it supplies no confirmed control-limit attribution or
performance comparison for this new production repair. No profile or 24-hour
qualification follows.

Backup archive framing and `installed_control_families` were also inspected:
session leases and their operational transaction receipts remain excluded from
application backup just as before; this package does not promise old-session
replay after application restore. Native reopen does preserve their history.
`cargo test --locked -p rrd-engine --jobs 6` passed 233 tests in total
(98.45 seconds including build), with two existing ignored privileged/child
fixtures; doc tests contain zero cases. This includes actual installation,
recovery, prepared-function, write-admission, query, vector and authority
tests. `cargo clippy --locked -p rrd-engine --all-targets --jobs 6 -- -D warnings`
passed (26.10 seconds tool time). The focused and full-suite logs preserve
both the original failures and passing replacement histories.

Formatting, generated navigation/inventory, documentation policy (486 statuses,
484 coordinates) and diff checks pass. A diff check caught incidental CRLF
conversion in two edited files; source line endings were normalized before
the final suite and review. Complete changed source/test functions and the
final diff were reread. The checklist's scope, traceability, oracle, research,
debug/resource, failure and not-run decisions are bound here and in the PR.
Native candidate crash lanes, macOS qualification, installed comparison of
this new binary and the 24-hour soak were not run by these component commands.
They remain required and cannot be inferred from these passes. No C-07
completion is claimed; exact-candidate CI and merge remain required.
