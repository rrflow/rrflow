# C-07d: atomic key-range flush partitions

**Status:** active evidence record; native comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07d-partitioned-flush`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [97](https://github.com/rrflow/rrflow-development/pull/97)

Starting revision `b9f5a5f8d515c62a36d94391e8f7d13327fc31e0`, tree
`93757ab99f54708536d343e74d3adbe7343aa37e`, combines fetched main with exact
PR96. The pre-edit PR brief binds the scope. Flush previously emitted one
segment for the entire mutable table while compaction emitted smaller key
partitions. That creates broad overlap ranges; it does not by itself prove
the complete cause of Linux14's hard L0 failure.

## Change and replacement map

`Database::flush_memtable_inner` now plans borrowed key boundaries using the
existing native immutable output target, checks the entire partition count
against unchanged output/L0 caps before WAL sync or file creation, and writes
one range at a time. Every key's complete MVCC chain stays together. A single
manifest publishes all outputs and its successor WAL. No accepted version is
pruned at flush, and no new mutable table or persistence authority exists.
Failed partial output is marked for the existing collector before writing;
the cooperative maintenance deadline is checked before publication.

`Memtable::range_versions` seeks directly into the ordered tree without
cloning keys or values. `segment::format::encode_versions` shares the v6 encoder
with whole-table writes; `Segment::write_encoded_with_cache` shares the existing
durable publication implementation. Standalone segment writes still produce
one segment. No schema, format, product version or hard ceiling changes.

Read README, alpha objective, Gate C/C-07, POAM-035, authoring and traceability
procedures and installed storage contract; complete affected database options,
preparation, flush, compaction/maintenance functions; memtable iteration and
version ownership; segment encoder/writer; compaction, maintenance and fault-I/O
fixtures. Changed those four production files, the fault-I/O test module,
compaction/maintenance fixtures and new `tests/partitioned_flush.rs`. Supporting
changes are this journal, installed reference, POAM-035 and generated navigation
and inventory. Review covered complete changed functions/test bodies and diff.

No external research or upstream adaptation is required to correct this local
physical-model asymmetry. Preserve existing trace/resource counters and actual
native timings; do not infer a latency improvement from component tests. Stop
on history loss, authority/format changes, whole-table cloning or relaxed caps.

## Evidence and surfaced failures

The initial new target failed all three tests: one output appeared instead of
12 and partition-count admission was absent. After implementation, those
histories pass: 12 nonoverlapping partitions with three versions per key,
unchanged historical reads after reopen, pre-effect output/L0 denial, and all
four flush boundaries under both logical failure modes. These injected modes
are not real disk exhaustion or device power loss.

A 32-cycle write/flush/compact/GC history offers finite maintenance with a
four-input limit and preserves original and latest values through reopen.
The native-call fault model interrupts the first, middle and last of six
partition renames both before and after the operation. It proves same-process
orphan cleanup and acknowledged-history recovery after modeled volatile loss.

The first owning-suite run exposed three old fixtures whose tiny compaction
target now also partitioned their setup flushes. They now explicitly create
wide input segments under a larger target and reopen under the original tiny
compaction/output budget. Their original relief, bounded-output and orphan
failure assertions are retained, rather than weakened. The corrected focused
compaction/maintenance/partition target passes 29 tests. Native partial-output
fault regression passes separately.

- `cargo test -p rrd-lsm --test partitioned_flush --locked`: initial three
  failures, then all four final tests pass, including the repeated history.
- `cargo test -p rrd-lsm --lib interrupted_multi_partition_flush --locked`:
  one pass covering six native-call schedules.
- `cargo test -p rrd-lsm --test compaction --test maintenance --test partitioned_flush --locked`:
  29 pass after correcting setup policy, with the original failure assertions.
- `cargo test -p rrd-lsm -p rrd-store --locked`: 440 pass, two existing ignored,
  206.16 seconds. The earlier owning-suite run retained the three fixture
  failures rather than hiding them.
- `cargo clippy -p rrd-lsm -p rrd-store --all-targets --locked -- -D warnings`:
  passes, 8.81 seconds.
- `cargo test -p rrd-engine --lib write_admission --locked`: seven pass,
  58.53 seconds including build. The initial `--test write_admission` invocation
  selected a nonexistent engine integration target and ran no tests; corrected
  to the existing library module.
- `cargo fmt -p rrd-lsm -- --check`, navigation rendering, execution inventory,
  `python -X utf8 scripts/ci/check_documentation.py` and `git diff --check`:
  pass. No workspace-wide, macOS or full capacity pass is implied.

Raw command logs use `target/c07d-partitioned-flush-*`. An attempted PowerShell
`HEAD^{tree}` argument was parsed incorrectly; the quoted revision produced
the tree above. An ad-hoc `gh --jq` query also lost shell quoting; it made no
repository changes. Neither is a product defect.

## Prior native failures retained

Linux14 source `5efe4647bb2d4244edf2c5a9d25b4ef2f32a6ada`, executable
`d85a3fa717cfa2cd99278270ba52c9cfb1f012c5ec6844bf0b8aed70e0c5a9a6`,
campaign `428dd0a73fa096d9402bcec4f17030ec179e4d049984730ff78835a1581cb57d`,
failed in 211384 ms before a workload segment was accepted. Last acknowledged
corpus ordinal was 970; the next operation returned the closed label
`api-resourceexhausted-maintenance-l0-segments`. Progress digest is
`61f59b2f5fc08554fad3de47d4412c515bc678e80e093f6a414330fba8044a73`.
Cleanup was confirmed. Raw receipts remain in `target/c07-linux-scale-evidence-14`.

Windows18 used the same source and executable
`7ef770632f165ad9ffe57ea4c171e3dfecade59c3374c96026d8420a40017448`,
campaign `2a1815e8141ee51ae3684fdb0c2d20c4769afae47a4dfad2d0b24806157eb4a3`,
harness `17901de9f11b81e4ba8028522cb8943b5f6f1e162570fb54b7ebda0c75a73219`.
It failed after 641327 ms and 62 retained segments, cleanup confirmed. Failed
segment `53979093efa66de9584a816661933439d6c0d8829ad2c45bb4f65e0aa81040da`
contains a 3069416400-ns resource gap for PID 71676 between 632470545300 and
635539961700 ns. Retained work reports no request failures or latency violations,
including 520 soak attempts; coverage still fails. Host compilation overlapped,
so this is not an isolated-host performance comparison or a 24-hour pass.
Raw evidence remains under `target/c07-windows-characterization-18/evidence`.

Linux13 on PR95 completed corpus and backup within 25.329 seconds but retained
31 debt segments and load failures. Inspection found the driver's segment-local
backup join can delay the next offered window; that requires a separately
bounded scheduling correction and cannot be treated as proven engine latency.
macOS native exhaustion and offline lifecycle failures remain unresolved.

Checklist: pre-edit brief and replacement map, red/green and multi-operation
recovery oracles, prior failure retention, diff review and linked journal are
recorded. Exact candidate CI, installed comparison, full capacity profiles,
macOS failures and 24-hour soak remain open. No roadmap/objective/POA&M closure
or alpha claim. Reasoning effort enforcement remains Gate G; this storage
package does not implement the unavailable governed reasoning runner.
