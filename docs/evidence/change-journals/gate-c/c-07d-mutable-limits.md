# C-07d native mutable-state admission and recovery limits

**Status:** active locally qualified prerequisite; accepted-base reconciliation and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-mutable-limits`
**Owner:** OBJ-02 prerequisite, [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Change brief and baseline

[PR 65](https://github.com/rrflow/rrflow-development/pull/65) bound the brief
before implementation. Accepted baseline `f4e404be38a0c21a4a3665b1907bad2226f5c13a`,
tree `e10ae21641df4eeea7ac647979d4f9268bce70ec`; evidence seed
`e0e52d4a7ade63bbf52ce4c5060a8cd72eadc584`. The
[baseline record](c-07d-mutable-limits-baseline.md) has no runtime changes.

Soft maintenance allowed an oversized atomic batch and native open/inspection
replayed without a hard active-state materialization budget. `MutableLimits`
now binds accounted active bytes and physical versions to native database
options, create/open, borrowed/owned writes and read-only inspection. A batch
that cannot fit empty state rejects before encoding, maintenance or WAL effects.
An admissible batch that would exceed the active limit triggers existing
synchronous flush/compaction before append. Replay checks each decoded batch
before adding its versions. Failure does not invoke repair or truncate bytes.
Defaults are 128 MiB of accounted charge and 1,048,576 versions. Zero limits
fail validation. Native callers may select a sufficient explicit budget for
reopen; this is not an installed configuration/UI binding.

Charge is repeated key bytes plus value bytes plus `size_of::<VersionedValue>()`
per physical version, including deletes and repeated keys. Checked arithmetic
replaces saturation in mutable application and compaction-table construction.
It excludes allocator/tree overhead, retained COW generations, caller batches,
transient WAL decoding, immutable data, caches and compaction working state.
Native diagnostics expose effective limits and peak accounted mutable bytes;
these are logical charges, not measured allocations or RSS.

## Traceability and review

The owner chain was followed from README/AGENTS through OBJ-02, the execution
procedure, C-07, POAM-035 and CI operations. Read/review coverage includes native
`database.rs` options/create/open/write/transaction/flush/compaction/GC/read and
diagnostic functions; `memtable.rs`, `inspection.rs`, `error.rs`, `lib.rs`, new
`mutable.rs` and the new test target; batch validation, WAL replay and the
existing maintenance, inspection, failure and projected-lifetime fixtures.
The exact source diff was reviewed with surrounding callers. Generated path
identities come from repository inventory, not a duplicate handwritten tree.

Canonical destinations remain `Database`, `Memtable` and `ReadOnlyDatabase`.
No implementation is moved/deleted and no alternate owner is added. The existing
soft oversized-batch characterization and native failure/lifetime suites are
replacement obligations. WAL/batch/segment formats, application identity,
transaction publication, dependencies, adapters and product version are unchanged.

Created: `crates/persistence/rrd-lsm/src/mutable.rs`,
`crates/persistence/rrd-lsm/tests/mutable_limits.rs` and these linked journals.
Changed: native database/memtable/inspection/error/export owners, the two
existing native CI partitions and their workflow policy, CI operations,
C-07/POAM-035 and generated navigation/file inventory. CI runs the focused
mutable-limit target separately on Linux, macOS and Windows with a five-minute
step bound. No extra job or diagnostic production switch is added.

Research/adaptation: no external implementation or new algorithm is imported.
The native charge, maintenance and replay contracts supply the oracle. Preserve
existing failure/write-phase diagnostics. Arithmetic overflow has an independent
extreme-value test without allocating a giant input. Actual capacity/RSS and
performance attribution remain C-07g/J-04 work; no speed claim is made.

## Executed evidence

Commands used an isolated `CARGO_TARGET_DIR` at
`target/c07d-mutable-build` outside this worktree. Logs are retained under the
parent checkout's ignored `target/c07d-mutable-*.log` (PowerShell UTF-16).

- `cargo test -p rrd-lsm --test maintenance one_oversized --locked`: **1 passed**.
  This characterizes the existing soft-target behavior, which still passes.
- Initial new target compile failed because the fixture guessed a nonexistent
  `Database::rollback_transaction`; inspection corrected it to transaction-owned
  `rollback()`. This compiler failure is not the behavioral oracle.
- Policy surface without enforcement, `cargo test -p rrd-lsm --test mutable_limits --locked`:
  **0 passed, 5 failed**. Zero limits created storage; excessive versions were
  acknowledged; hot-key state exceeded its cap; pressure did not flush; over-budget
  writer replay opened successfully. An unnecessary mutable binding was removed.
- After enforcement, the initial target passed **5/5**. Expanded final target:
  **6 passed, 0 failed**. It covers byte/count cap-minus-one/cap/cap-plus-one,
  borrowed/owned parity, full file-byte inventory before/after rejection, valid
  work after denial, deletes/repeated keys, two 40-write hot-key histories with
  held transaction reads and 19 flushes each, actual automatic compaction,
  one-frame/multi-frame recovery, writer/read-only budget denial with unchanged
  files, exact-cap reopening, subsequent delete and final inspection.
- The sixth target joins pressure flush with prepared and post-WAL-sync injected
  failures for both write APIs: rejected writes stay absent, uncertain durable
  frames reappear exactly, post-WAL writers stay fenced, and valid subsequent
  work reopens correctly. These are injected logical failures, not native process
  crashes or disk exhaustion.
- `cargo test -p rrd-lsm --lib mutable::tests --locked`: **1 passed**.
- Final `cargo test -p rrd-lsm --all-targets --locked`: **147 passed, 0 failed,
  1 ignored**, 21 result groups. The ignored owned process child is invoked by
  parent tests. Linux-only memory fixtures run zero cases on this Windows host.
- `cargo test -p rrd-store --all-targets --locked`: **203 passed, 0 failed,
  0 ignored**, 38 result groups.
- `cargo test -p rrd-engine --all-targets --locked`: **207 passed, 1 failed,
  2 ignored**, 12 result groups. The only failure was the architecture check
  requiring the new Cargo test target to be tracked. After staging source/test
  files, `cargo test -p rrd-engine --test workspace_architecture --locked`:
  **31 passed, 0 failed**. No runtime failure was hidden or allowlist added.
  The full engine command was not repeated after this index-only correction.
- `cargo clippy -p rrd-lsm -p rrd-store -p rrd-engine --all-targets --locked -- -D warnings`:
  **passed**. `cargo fmt --all --check`, `python -X utf8 scripts/ci/check_workflow.py`,
  `python -X utf8 scripts/check_version.py`, documentation policy, navigation
  check, execution-inventory check and staged/unstaged diff checks passed after
  the documented metadata correction. Final projections: 17 indexes, 403 nodes,
  1,758 edges and 1,395 inventory records. Workflow policy retains mode 100755.

Documentation validation caught that the seed baseline's status lacked the
required active/historical lifecycle word. This successor record documents the
one-word metadata correction (`active`); the original seed remains in Git and
its baseline, claims and evidence body are unchanged. The final record uses
active status. Navigation/inventory were regenerated after the correction.

Read-only discovery also encountered guessed nonexistent test/script paths;
`rg --files` resolved the actual paths. The guessed version-check command did
not run a check; final verification uses `scripts/check_version.py`.

## Change checklist and remaining proof

The preimplementation PR binds one outcome, scope, failure oracle, acceptance,
research/resource/debug decisions and stop conditions. Existing authority and
formats are preserved. Behavioral failures precede enforcement; tests cover
continued histories and observable effects. Source changes, callers and test
oracles were reviewed; linked evidence, navigation and inventory are required
before commit. Reconciliation and candidate CI remain mandatory before merge.
The seed-only CI run `39826720539` was canceled intentionally, not accepted.

Stop on early ACK, partial state, changed uncertain-I/O classification, unsafe
reclamation, unbounded exemptions, expanded ownership or failed candidate CI.
No C-07, POAM-035 or alpha status is closed. Aggregate live-owner/queue limits,
installed bootstrap/configuration binding, hard compaction debt/work bounds,
installed reclamation, native disk exhaustion, measured capacity and the actual
24-hour soak are not proved by this package. No soak was started here. PR 63/64
changes must be reconciled from accepted main with a successor evidence record;
this local record cannot claim their combined acceptance.
