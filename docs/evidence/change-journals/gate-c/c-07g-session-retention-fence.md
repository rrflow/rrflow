# C-07g: fence session readers that cannot discover retained receipts

**Status:** active replay-safety refinement; final native validation pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-session-retention-fence`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [110](https://github.com/rrflow/rrflow-development/pull/110)

Baseline `994d278cf0b95c13161b7db72e88098cb31d9480`, tree
`8d3f71c4ee6860425ce29c9bfdc22aee7b68e2a0`. Review of the complete changed
session encoding and lookup paths found a downgrade hazard: an earlier binary
understands only inline transactions but could accept the same session format
after a terminal record moved, miss its idempotency record and begin new work.
The PR brief was expanded before this refinement's source edits, within its
existing preservation-of-replay scope. No product version changes.

`SessionState` now persists `terminal_transaction_format`, owned by the same
constant as the terminal record envelope. The single current decoder defaults
an absent marker for existing inline state and rejects unknown marker values.
Creation and every existing session transition serialize the marker; terminal
placement and that session update remain one atomic control batch. Earlier
`SessionState` readers use `serde(deny_unknown_fields)`, so the added field
fences them before they can overlook terminal records. No parallel reader,
migration runtime, fallback authority or new product operation is introduced.

Read the authoring/traceability owners, preceding implementation/native
records, complete `SessionState`, creation/decode/replacement and transaction
lookup/replay paths. Changes are limited to `engine/{mod,session,session_transactions}.rs`,
the existing focused session test file, this journal, POAM links and generated
navigation/inventory. Local source suffices; no upstream research or new
runtime trace. Stop for a second session authority or weakened idempotency.

The regression constructs the earlier all-inline encoding without a marker,
then begins/commits, requires the marker after terminal placement, reopens and
compares the full replay receipt. An unknown marker must reject access without
a new journal or domain effect. An initial test-only compile error used
`CorrelationId` as Display; it was corrected to `as_str()`. The actual
pre-repair oracle then failed (`Null` versus marker `1`) in 15.91 seconds
including build, recorded in `target/c07g-session-fence-oracle.log`.

`cargo test --locked -p rrd-engine --lib session_transactions --jobs 6 -- --nocapture`
passed all seven focused tests in 38.67 seconds including build (tests 21.67
seconds). `cargo test --locked -p rrd-engine --jobs 6` passed 234 tests, with
the same two existing ignored fixtures, in 85.92 seconds including build.
`cargo clippy --locked -p rrd-engine --all-targets --jobs 6 -- -D warnings`
passed in 11.22 seconds. Formatting, navigation/inventory, documentation
(488 statuses, 486 coordinates) and diff checks pass. Complete changed
functions, regression and final diff were reread. Logs use
`target/c07g-session-fence-*.log`.
The prior Linux23 result belongs to the unfenced baseline, not this refinement.
The complete new candidate still requires installed comparison and native CI;
full profiles and 24-hour qualification remain open. No roadmap status changes.
