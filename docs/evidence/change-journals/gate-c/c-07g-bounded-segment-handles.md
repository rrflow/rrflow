# C-07g: bound immutable segment file ownership

**Status:** active implementation evidence; full native capacity qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-bounded-segment-handles`
**Owner:** OBJ-02 / C-07g / POAM-035 / PR130

## Brief and traceability

[PR130](https://github.com/rrflow/rrflow-development/pull/130) recorded the brief
before implementation. Driver: the current Codex integration session. The
opening branch ordinary-merged public main `dd225072bcc639c89b0eef968625cc73303ea998`
to starting revision `305e26f027108ec4a11e1199d36af8f7bbaa9a59`, with the same
source tree `0ee0344e039b94ea23f124bb2a50209fc7263882`. Prior PR114 dependencies
are already landed. No force push, official-repository promotion or gate bypass.

The [retained Desktop failure](c-07g-native-descriptor-failure.md) stopped at
77,945 acknowledgements with native EMFILE. Previously each immutable segment
kept an open file for its lifetime. The existing IoContext now owns a bounded
pool, including initial validation and concurrent reads. Admission, idle LRU
eviction and native opens share one lock. Read leases prevent eviction through
completion of positional I/O; dropping the last source removes its cached file.
The default is 128 files per native database, configurable from 1 to 4096 via
SegmentIoPolicy.max_open_files and installed StorageLimits.segment_open_files.
This is a segment-file limit, not a claim to bound all process descriptors.

Segment metadata, authenticated pages and read-view/manifest reclamation remain
in their original owners. Evicted sources reopen only through AnchoredPath;
native device/inode or volume/file-index identity, length and modification time
must match. Existing page digests still authenticate content. Standalone
segments retain their own context and sole file; no alternate persistence lane
is introduced. WAL, publication, compaction and recovery formats are unchanged.

Installed policy is optional and omitted when absent, preserving existing sealed
configuration bytes. An explicit limit is validated and lowered at engine open.
Diagnostics expose the configured/current/peak handle counts and eviction,
reopen and denial counters, with ownership invariants checked by the contract.
Store and query error conversions preserve typed pressure. Read-only pressure
can report resource exhaustion; durable callers keep uncertain outcomes as
internal errors. This change does not assert that a pressured commit is absent.

## Research, review and scope

The primary [RocksDB tuning guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide)
describes bounded table-cache descriptor eviction. Only that behavior informs
this repair; no upstream code, public model, runtime or crate tree is copied.
RRFlow supplies its own anchored identity, source ownership and failure rules.
Process-local handle counters provide the required resource signal without
recording filenames, secrets or project data. Stop conditions prohibit relaxed
OS/workload limits, forced mmap, weakened checksums, changed durability, or a
second transaction/reclamation authority.

Read owners: README, alpha objective, Gate C/C-07, POAM035, the change-authoring
procedure, retained descriptor-failure journal and the pre-effect brief.
Full new modules and tests reviewed: io/handles.rs, io/handles/tests.rs and
tests/segment_handles.rs. Existing I/O policy/read code, source-memory and
reader-lifetime tests were read; relevant filesystem traversal/native identity,
segment open/read/digest, store conversion/evidence, installed policy lowering,
diagnostic validation and existing admission-test sections were inspected.
Changes replace PageSource's direct File owner in place and add the pool and
regressions. No engine source is moved or deleted. Contract, SDK and synthetic
fixture/schema projections are regenerated from their existing owners. Current
OpenAPI references follow the reviewed new pin; historical receipts are retained.

## Failure oracle and validation

The native regression runs in an isolated Linux child with soft RLIMIT_NOFILE
256. It writes 384 immutable segments, reads every record, reopens the database
and reads them all again. Against unchanged main, the same workload (omitting
only assertions for counters absent in main) fails with native error 24 during
flush. With bounded ownership it passes. A second test uses one open file while
a projected reader survives compaction, garbage collection and database drop.

These are working-source characterizations, not an exact-candidate capacity
receipt. Retained Linux logs:

- `target/c07-handles-linux-baseline.log`: 1,559 bytes, SHA-256
  `84821c8faab4a6131eb5643782cb620988c49970346a6dca170c435c8ae94541`;
  one expected regression failure against main.
- `target/c07-handles-linux-working.log`: 743 bytes, SHA-256
  `0ce7bfe54ad6cfb474676976f5caa6d01ad8b9b301cc2db15dd4b8a735680a8d`;
  `cargo test --release --locked -p rrd-lsm --test segment_handles -- --nocapture`:
  two passed in 1.20 seconds.

Local Windows results:

- `cargo test -p rrd-lsm --lib io::handles --locked`: four passed (eviction,
  active leases, substituted identity and concurrent admission).
- `cargo test -p rrd-lsm --locked --all-targets`: passed, including native crash,
  failure sweep, reclamation and retained-reader suites; two pre-existing
  dedicated unit helpers ignored. This is not the installed capacity matrix.
- `cargo test -p rrd-lsm --test segment_handles --locked`: two passed, 20.53 seconds.
- `cargo test -p rrd-engine --lib engine::tests::write_admission --locked`:
  eight passed, including configured bounds, omission round-trip and error class.
- `cargo clippy -p rrd-lsm -p rrd-store -p rrd-contract -p rrd-engine -p rrd-query --all-targets --locked -- -D warnings`: passed.
- TypeScript `pnpm typecheck` and `pnpm test`: passed, six tests.

Initial compilation exposed PageSource clone ownership; the source now uses one
Arc<SegmentFile> so clones do not independently remove a cached identity. The
first contract suite correctly rejected the old OpenAPI pin; reviewed schema,
SDK and current-reference projections were regenerated. The Python generator's
project environment could not locate Python 3.15 locally; the existing generator
ran with the exact pinned Ruff 0.17.1 in an isolated uv environment. This does
not claim Python SDK runtime conformance. One Go generation invocation used the
repository root and failed to find its module; rerunning from sdks/go succeeded.

Full `cargo test -p rrd-contract -p rrflow-eval --locked --all-targets` passed
after the reviewed pin update. `cargo test -p rrd-engine --lib --locked`: 207
passed, two existing dedicated helpers ignored; `cargo test -p rrd-store --lib
--locked`: 73 passed, one existing dedicated helper ignored. The full validation
log is retained at `target/c07-handles-validation.log`. Candidate CI and the protected merge group remain required.
No macOS result, full Desktop corpus preflight or 24-hour soak is claimed here.
The original failed private estate remains untouched. C-07 and POAM035 stay open;
this package repairs a verified growth prerequisite, not the whole alpha.

Documentation validation caught a Windows text-encoding change in the POAM title;
the original UTF-8 bytes were restored and derived navigation regenerated.

`cargo test -p rrd-engine --test engine_authority --locked`: three passed,
covering coordinated data/security/catalogues/audit/reopen and fenced estate
recovery. Generated-surface parity, documentation policy, CI workflow policy,
format and whitespace checks passed. Navigation and execution inventory were
regenerated (608 knowledge nodes, 1,734 inventory records).

Checklist complete for candidate submission: pre-effect draft brief; canonical
source/replacement map; native red/green oracle; lease/identity/lifetime tests;
owning LSM suite and affected contract, store and engine tests; strict lint;
research and resource decisions; unchanged failed evidence; linked journal and
POAM update; reviewed source/test diff and mechanically checked generated
projections. No roadmap maturity or release-gate completion status changes.
Final candidate CI and merge evidence must be recorded separately once obtained.
