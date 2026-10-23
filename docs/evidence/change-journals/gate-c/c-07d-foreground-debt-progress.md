# C-07d: bounded foreground debt progress

**Status:** active evidence record; native comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07d-foreground-debt-progress`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [98](https://github.com/rrflow/rrflow-development/pull/98)

Baseline `20df9f6e9b53c853a1bfd963e57df045b5b76056`, tree
`943d8a84ef7e7cc2a2cfa0170597b11c3da59261`, combines fetched main with PR97.
The pre-edit brief binds the change. Partitioned flushes preserve atomicity
but can add several L0 inputs; offering only one merge after a flush leaves
writers dependent on idle work winning the lock between flushes. This package
corrects that missing foreground opportunity without an unbounded drain loop.

## Change and traceability

`Database::prepare_write` offers at most one immutable compaction quantum per
incoming write when debt reaches a full configured input-segment quantum,
bounded by hard L0 capacity if smaller. Small debt remains idle-eligible,
avoiding a maintenance interruption on every ordinary write. It offers before a needed flush, or after a
new flush if no earlier quantum was offered. The existing hard L0 check then
denies admission before WAL append if pressure remains. `compact_for_write`
now uses `compact_segments_inner` directly so housekeeping never flushes the
acknowledged mutable tail. Existing explicit compaction APIs still flush.
`offer_write_compaction` shares the prior typed soft-denial classification and
automatic counters; native I/O, corruption and publication uncertainty still
propagate. Retention ownership, format, policy, hard ceilings and authority
remain unchanged. There is no retry-until-success loop or new worker.

Reviewed owner chain, C-07/POAM-035, authoring/traceability, installed reference,
complete native preparation/compaction/maintenance/retention functions and
maintenance/compaction characterization tests and store operator history test.
Changed database.rs, tests/maintenance.rs, store tests/rrflow_kv_operator.rs,
this record, installed reference, POAM-035 and generated indexes
and inventory. Reread changed functions/test bodies and complete diff. The
pre-edit admission refinement binds the foreground threshold and store fixture
isolation. It does not change production caps or remove original assertions.

Research/source adaptation: local reproduced debt and established physical
ownership contracts suffice. No upstream source or new runtime machinery is
introduced. Preserve actual resource signals and budgets; do not infer native
timing improvements from the focused history. Stop on multiple compactions
per write, implicit mutable flush, hidden retries, history loss, ignored
uncertainty or increased caps.

## Evidence

The first oracle seeded 12 immutable partitions, then issued ten ordinary
writes below flush targets with no idle or explicit compaction. Before repair,
nine units of debt remained; afterward debt drains. Each write offers at most
one compaction, mutable versions rise by exactly one, WAL payload increases,
the immutable durable sequence/WAL start remains fixed, and original snapshots
plus all ten acknowledged tail writes survive reopen.

The initial eager-on-any-debt experiment consumed idle work during setup. Five
fixtures failed: explicit pinned-transaction input count, idle drain debt,
idle denial counter, explicit idle pruning count and store idle shrink. The
final admission design leaves small debt idle-eligible; existing native idle
and explicit-compaction fixtures retain their original inputs and assertions.
An added zero-foreground-compaction check proves the small-debt path. The
temporary explicit-compaction setup-trigger edit was reverted. The store's
idle ownership fixture explicitly selects a 64-input quantum within the
unchanged 64-segment hard cap, retaining all shrink, retained-reader, history,
deletion and reopen assertions. Its production configuration is unchanged.

The refined focused run passes 30 native tests and all five store operator
tests. The initial ordinary-write oracle remains a real regression: its own
four-input quantum is fully accumulated and must make bounded progress.

- `cargo test -p rrd-lsm --test maintenance ordinary_writes_pay --locked`:
  fails before repair with debt nine, passes after repair.
- `cargo test -p rrd-lsm --test maintenance --test compaction --test partitioned_flush --locked`:
  final refined policy passes 30 tests; the initial eager policy's failures
  are retained in `target/c07d-foreground-debt-green.log` and the initial suite log.
- `cargo test -p rrd-store --test rrflow_kv_operator --locked`: five pass.
- `cargo test -p rrd-lsm -p rrd-store --locked`: 441 pass, two existing ignored,
  126.70 seconds. No native profile qualification follows from this suite.
- `cargo clippy -p rrd-lsm -p rrd-store --all-targets --locked -- -D warnings`:
  passes, 5.59 seconds.
- `cargo test -p rrd-engine --lib write_admission --locked`: seven pass,
  45.47 seconds including build.
- Formatting, navigation rendering, execution inventory, documentation policy
  and diff checks pass. Exact candidate CI, new native comparison, macOS and
  full profiles/soak were not run for this journal's local acceptance.

Raw logs use `target/c07d-foreground-debt-*`. These commands ran on the final
production change; the journal and generated projections were completed
afterward. No earlier native result is reclassified as passing.

## Failed native comparison retained

Linux15 used PR97 source `d0f49126681468554f26cb022559cfa42c6df7bb`, executable
`8d08fb922d442ac55f50e719121eab6b0e83cc08c1cbcfa85d91f2e674cb3c43`,
campaign `c0d0606b01cf7e192970e7f1b05447dedcd1495fb9003eb1b7cfc563d85576e7`,
harness `e035c717ac9cd3a8a8821a9414ca2c384852f52028f425190c7db7d8b6d7ab33`.
The unchanged 4-CPU/2-GiB 1000-record/5000-edge/1000-vector corpus stopped after
acknowledged ordinal 290 with `api-resourceexhausted-maintenance-l0-segments`.
Elapsed 73634 ms, no accepted workload segments, cleanup confirmed; progress
digest `481596522746ad012aee916254d24197bab2d895a45ef72b0101d7989e9c5348`.
Raw receipts are `target/c07-linux-scale-evidence-15` and the full15 log.
This earlier rejection is a failed comparison, not an optimization pass.
PR97 returned to draft while this prerequisite is assessed.

Checklist: pre-edit brief/fixture clarification, owner/replacement map,
reproduced failure, bounded multi-operation history and linked journal are
recorded. Exact candidate CI, installed comparison, capacity profiles, macOS
native failures and 24-hour soak remain required. No alpha or C-07 closure;
reasoning-runner effort enforcement is still a separate unimplemented Gate G
obligation. A smaller optional maintenance quantum does not implement that UI
or durable run budget.
