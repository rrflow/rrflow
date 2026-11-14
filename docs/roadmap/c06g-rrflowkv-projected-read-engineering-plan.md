# C-06g rrflowKV projected-read engineering plan

**Status:** active human-readable work-package plan; implementation, declared
verification, commit, and development-push handoff complete; C-06 remains open
**Coordinate:** `rrflow://rrflow-instance/data/work-package/c-06g-rrflowkv-projected-read`
**Owner:** subordinate C-06g file/symbol implementation and evidence sequence
**Canonical gate owner:** [`rrflow-1.0.md`](rrflow-1.0.md), C-06
**Machine-enforced plan:** [`rrflow-1.0-active-change.json`](rrflow-1.0-active-change.json)
**Research basis:** [`rrflowKV Rust storage-engine architecture research`](../research/rrflowkv-rust-storage-engine-architecture-research.md)
**Baseline:** `6de083f9cdf9a4ffe4a6904199745b49304d8fe5`
**Planning commit:** resolved by `scripts/ci/check_change_plan.py`
**Owner boundary:** `rrd-lsm`, with a narrow diagnostic integration in
`rrd-store`

This is the readable engineering plan the machine JSON enforces and the C-06g
execution journal records. It does not replace the canonical roadmap, create
another status authority, or authorize later query, index, installation,
attunement, reasoning, trigger, routine, or skill behavior.

## Decision

C-06g was accepted for implementation only for this exact bounded purpose:

- capture one stable logical and physical rrflowKV read generation;
- release the store mutex before iteration;
- merge memtable and immutable ordered runs without result-size
  materialization;
- read only the page families required by the projection;
- emit bounded Arrow-compatible buffers with honest ownership/copy evidence;
- keep garbage collection from deleting files used by a live reader; and
- separate segment-open, startup-reconciliation, and query physical work.

C-06g does **not** complete rrflowKV, DataFusion integration, graph traversal,
BM25, vectors, reasoning, install, Connectome, or the alpha. C-06h and C-06i
close the remaining C-06 adversarial and physical-policy evidence. The
canonical next product milestone is then D-01: a real primary
`rrflow`/`rrflow.exe` lifecycle. C-07 follows D-01 and qualifies recovery and
maintenance through the installed product.

## Current behavior to preserve

The implementation must retain these already useful properties byte-for-byte
or behavior-for-behavior unless a test proves an explicitly planned change:

- WAL v1 framing and CRC32C validation;
- write-batch v2 atomic encoding;
- application key format `RRKV0001`;
- manifest v3 and CURRENT publication/validation;
- immutable segment v4 bytes, alignment, digests and schema identity;
- snapshot-isolation visibility and existing write/write conflict rules;
- tombstone semantics across memtable, segment, flush and compaction;
- create/open failure behavior and single-writer exclusion;
- point reads, current range-read results and transaction results;
- current compaction publication and recovery ordering;
- snapshot bundle v1 and checkpoint reachability;
- mmap owner retention and bounded/io_uring fallback behavior;
- rrflowMX/rrflowKV semantic result parity; and
- no DataFusion, Tokio, provider, network, model or automation dependency in
  `rrd-lsm`.

## First failure oracles

These tests are written or extended before the corresponding production code.
Their initial failure is recorded in the execution journal.

1. `projected_stream_matches_model_across_ranges_snapshots_and_projections`
   initially fails to compile because no projected read request/stream exists.
2. `pinned_projected_stream_survives_flush_compaction_and_gc_until_drop`
   initially fails to compile because a snapshot owns only a sequence, not a
   physical generation or GC lease.
3. `projected_stream_enforces_bounds_and_reports_selected_pages` initially
   fails because current range readers cannot express projection or
   operation-scoped resource evidence.
4. `rrflow_kv_open_separates_segment_validation_and_reconciliation_io`
   initially fails because open, reconciliation, and later reads share
   cumulative cache/I/O counters without immutable phase evidence.

No assertion may be weakened to make an existing implementation pass.

## Target data flow

```text
Database::begin_projected_read(request)
  -> validate ranges, projection and every nonzero budget
  -> atomically capture
       sequence
       manifest identity
       Arc<Memtable generation>
       Vec<Arc<Segment generation>>
  -> acquire active-manifest lease
  -> release Database/RrflowKvStore mutex
  -> create one forward cursor per eligible run
  -> min-heap merge by application key
  -> inspect all entries for that key at the captured sequence
  -> reject equal key+sequence across live runs
  -> greatest visible sequence wins
  -> load winning validity page
  -> suppress winning tombstone
  -> load value offsets/data only for KeyValue projection
  -> enforce output limit before allocation/emission
  -> emit bounded aligned offset/data batch
  -> terminal evidence: completed / cancelled / failed
  -> release lease on terminal state or Drop
```

## Public and crate-private contract

Only the bounded request/result/evidence vocabulary leaves `rrd-lsm`.
Individual table cursors, page locators, manifest registries, cache entries and
raw mutable storage access remain crate-private.

### Public physical types

- `ProjectedReadRange`: validated half-open byte range;
- `ProjectedReadProjection`: closed `Key` or `KeyValue` enum;
- `ProjectedReadBudget`: every active-view/run/range/version/page/byte/row/batch
  limit required by the active machine plan;
- `ProjectedReadRequest`: ranges, projection, snapshot and budget;
- `ProjectedReadResource`: closed resource enum used by typed limit errors;
- `ProjectedReadOutcome`: `Creating`, `Running`, `Completed`, `Cancelled`, or
  `Failed`;
- `ProjectedReadBatch`: aligned offsets/data buffers plus row accessors;
- `ProjectedReadEvidence`: immutable identities, selected ranges/runs/pages,
  physical work, memory work, output work and outcome; and
- `ProjectedReadStream`: fused forward batch source with explicit cancellation.

### Crate-private types

- `ActiveReadViews` and RAII `ReadViewLease`;
- `ReadView` containing sequence, manifest identity, memtable and segments;
- `MemtableProjectedCursor`;
- `SegmentProjectedCursor`;
- `ProjectedEntry` carrying key, visible sequence and a late value locator;
- `ProjectedReadMeter`; and
- heap item/order helpers.

No stringly typed fallback is permitted for projection, outcome, resource or
I/O backend identity.

## Batch plan

Each batch is reviewable and records exact commands/results in the execution
map. A later batch does not begin while the smallest owning test for the
current batch is red for an unexplained reason.

### Batch C-06g.1 — characterize the missing behavior

**Files changed**

- `crates/persistence/rrd-lsm/tests/hybrid_segment.rs`
- `crates/persistence/rrd-lsm/tests/compaction.rs`
- `crates/persistence/rrd-store/tests/rrflow_kv_open.rs`

**Work**

- Add independent-oracle cases covering full, bounded, disjoint and empty
  ranges; key-only and key-value projection; tiny batches; memtable plus
  multiple segments; tombstones; historical snapshots; and writes after view
  capture.
- Add one physical-lifetime case that captures a view, then flushes, compacts
  and garbage-collects before consuming it.
- Add one open-phase case that distinguishes full segment validation,
  checkpoint reconciliation, and later query work.
- Add explicit invalid range, zero budget, every resource limit, cancellation,
  fused terminal state, duplicate key/sequence and lease-release assertions.

**Expected first result**

The new tests fail to compile on the absent types/methods. Existing tests stay
unchanged and green when run independently.

**Stop conditions**

- the oracle depends on production merge code;
- a test assumes Unix unlink semantics;
- a test uses timing/sleep as correctness; or
- an existing frozen-format assertion must be changed.

### Batch C-06g.2 — owned read generation and GC lease

**Files changed**

- `crates/persistence/rrd-lsm/src/database.rs`
- `crates/persistence/rrd-lsm/src/error.rs`
- `crates/persistence/rrd-lsm/src/lib.rs`
- `crates/persistence/rrd-lsm/tests/compaction.rs`

**Exact symbol work**

- Change `Database.memtable` and `Database.segments` to `Arc`-owned immutable
  generations at read capture points.
- Use `Arc::make_mut` only before a writer mutates a shared memtable
  generation; do not mutate an object already captured by a reader.
- Add an interior active-view registry keyed by authenticated manifest
  identity, with checked reference counts and pinned-byte accounting.
- Add `Database::begin_projected_read` acquisition: validate all acquisition
  limits, capture all owners atomically, acquire the lease, then return without
  retaining `&mut Database` or the store mutex.
- Extend `Database::garbage_collect` reachability with active read manifests.
- Release the lease exactly once on completion, cancellation, error or drop.
- Add typed active-view capacity, pinned-byte and poisoned-registry errors.

**Required proof**

- a pre-change view returns its exact old snapshot after later write, flush,
  compaction and GC;
- current reads return current data;
- old files are retained while the view lives and become reclaimable after it
  drops; and
- existing transaction/compaction/reopen/frozen-format suites stay green.

**Stop conditions**

- the store mutex must remain held during iteration;
- files are kept alive only by Unix open-file deletion behavior;
- a reader can observe a post-capture memtable mutation; or
- active-view accounting can saturate, underflow or be silently repaired.

If this behavior requires changing `transaction.rs` or another undeclared
path, stop and revise the machine plan before editing it.

### Batch C-06g.3 — selective run cursors

**Files changed**

- `crates/persistence/rrd-lsm/src/memtable.rs`
- `crates/persistence/rrd-lsm/src/segment.rs`
- new `crates/persistence/rrd-lsm/src/segment/reader.rs`
- `crates/persistence/rrd-lsm/src/lib.rs`
- `crates/persistence/rrd-lsm/src/error.rs`
- `crates/persistence/rrd-lsm/tests/hybrid_segment.rs`

**Exact symbol work**

- Add a memtable seek-after operation that returns the next eligible key and
  greatest visible sequence without cloning the remaining range.
- Expose crate-private authenticated row-group bounds and page acquisition to
  `segment::reader`; do not expose raw page addresses publicly.
- Implement `SegmentProjectedCursor` that reads the key-offset/key-data and
  sequence pages first and prunes row groups by authenticated bounds.
- Load the winning validity page only after the global merge selects a
  candidate; load value offsets and data only for `KeyValue`.
- Never request `ValueOffsets` or `ValueData` for `Key` projection.
- Charge page request/logical bytes before acquisition and record actual cache
  and selected I/O behavior after it completes.

**Required proof**

- exact ordered bytes match the independent MVCC oracle;
- tombstones never resurrect older values;
- key-only evidence reports zero value-page requests;
- range pruning never returns an out-of-range key; and
- malformed page bounds/digests fail before bytes are exposed.

**Stop conditions**

- a cursor materializes its remaining range;
- all six page families are loaded unconditionally;
- page-limit checking occurs after the read; or
- mapped bytes outlive their `Arc<Mmap>` owner.

### Batch C-06g.4 — global merge and bounded Arrow-compatible batches

**Files changed**

- `crates/persistence/rrd-lsm/src/segment/reader.rs`
- `crates/persistence/rrd-lsm/src/error.rs`
- `crates/persistence/rrd-lsm/src/lib.rs`
- `crates/persistence/rrd-lsm/tests/hybrid_segment.rs`

**Exact symbol work**

- Use one cursor per eligible memtable/segment run and a min-heap ordered by
  application key.
- Drain every live run at the selected key, compare visible sequence numbers,
  and fail closed on an equal key/sequence in different runs.
- Enforce versions-examined and run/range limits during merge.
- Build 64-byte-aligned i64-offset/data buffers bounded by row, buffer and
  allocation limits.
- Reject a single row that cannot fit; never report successful truncation.
- Make `next_batch` fused after any terminal result and make cancellation
  explicit and idempotent.
- Count logical bytes, mapped/borrowed bytes, decoded/decompressed bytes,
  allocations, copies, rows and batch peaks honestly.

**Required proof**

- varying tiny batch sizes produce the same concatenated result;
- output order is strict and keys are unique;
- every limit fails before excess work/allocation/emission;
- cancellation/drop releases the view; and
- no test or documentation calls an owned merge batch zero-copy.

### Batch C-06g.5 — truthful open and I/O evidence

**Files changed**

- `crates/persistence/rrd-lsm/src/io.rs`
- `crates/persistence/rrd-lsm/src/segment/format.rs`
- `crates/persistence/rrd-lsm/src/segment.rs`
- `crates/persistence/rrd-store/src/rrflow_kv.rs`
- `crates/persistence/rrd-store/src/lib.rs`
- `crates/persistence/rrd-store/tests/rrflow_kv_open.rs`

**Exact symbol work**

- Have `IoContext::read_exact_at` return the backend actually used after an
  io_uring fallback; failed I/O is not successful-read evidence.
- Aggregate format probe, full-file checksum, header/index metadata and
  semantic-page validation bytes into immutable `SegmentOpenEvidence`.
- Capture checked before/after page-cache and I/O snapshots only around
  startup checkpoint reconciliation while the store is not yet published.
- Store immutable `RrflowKvOpenEvidence` with separate segment-validation and
  reconciliation fields.
- Keep later query work on `ProjectedReadEvidence`; do not infer it by
  subtracting global counters during concurrent operation.
- Emit redacted low-cardinality trace fields only; never log keys, values,
  secrets or filesystem payloads.

**Required proof**

- clean reopen with no reconciliation reports its actual zero/nonzero phases;
- a reconciliation fixture attributes only its own reads;
- io_uring fallback is counted as bounded I/O when bounded I/O served it;
- counter regression/overflow fails open; and
- existing `PhysicalStoreEvidence` semantic meaning does not change.

### Batch C-06g.6 — package closure and handoff

**Files updated after code evidence exists**

- `README.md` current-status line only;
- `docs/objectives/rrflow-1.0-alpha.md` partial evidence only;
- `docs/roadmap/rrflow-1.0.md` C-06 evidence only, no checkbox unless all
  C-06g/C-06h/C-06i evidence exists;
- `docs/poam/rrflow-1.0-alpha.md` verified remaining gaps;
- `docs/architecture/engine-data-flow.md` implemented physical read flow;
- `docs/reference/storage/rrflowkv-current-format.md` current contract;
- `docs/evidence/test-plans/persistence-scenario-matrix.md` accepted cases;
- `docs/research/rrflow-system-convergence-architecture-research.md` current
  implementation audit only;
- `docs/roadmap/rrflow-1.0-execution-map.md` complete package journal; and
- the generated file inventory (the former tracked
  `docs/roadmap/rrflow-1.0-file-plan.jsonl` is now a descriptor; generate the
  current view under `target/` with `scripts/ci/build_execution_inventory.py`).

Documentation records only measured behavior and retained gaps. C-06g cannot
check C-06, claim a DataFusion stream, or claim alpha readiness.

## Code review checklist for every batch

- [x] Re-read every complete file to be changed at the current baseline.
- [x] Confirm the file and symbol are declared by the active machine plan.
- [x] Record the current behavior and preservation test before editing.
- [x] Add/run the smallest first-failure or characterization test.
- [x] Keep `RrdEngine`, storage and query authority boundaries unchanged.
- [x] Add typed failure semantics; no boolean/string fallback or silent
  truncation.
- [x] Define trace, resource and debugging evidence in the same batch.
- [x] Review cancellation, drop, retry, overflow and uncertain-I/O paths.
- [x] Run the smallest test, then the owning package suite.
- [x] Run strict affected-package Clippy and frozen-format regressions.
- [x] Re-read every changed file and review the complete diff.
- [x] Journal exact commands/results, failures, omissions and remaining gaps.
- [x] Commit one coherent change; do not report a roadmap status change unless
  the canonical acceptance evidence exists.

## Acceptance commands

Run in this order, stopping at the first unexplained failure:

```text
python3 scripts/ci/check_change_plan.py
cargo test -p rrd-lsm --test hybrid_segment projected_stream_matches_model_across_ranges_snapshots_and_projections --locked -- --exact --nocapture
cargo test -p rrd-lsm --test hybrid_segment projected_stream_enforces_bounds_and_reports_selected_pages --locked -- --exact --nocapture
cargo test -p rrd-lsm --test compaction pinned_projected_stream_survives_flush_compaction_and_gc_until_drop --locked -- --exact --nocapture
cargo test -p rrd-store rrflow_kv_open_separates_segment_validation_and_reconciliation_io --locked -- --exact --nocapture
cargo test -p rrd-lsm --test segment v4_bytes_match_the_checked_in_format_vector --locked -- --exact
cargo test -p rrd-store --test direct_read_paths --locked
cargo test -p rrd-lsm --locked
cargo test -p rrd-store --locked
cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings
cargo clippy -p rrd-store --all-targets --locked -- -D warnings
cargo test -p rrd-engine --test workspace_architecture --locked
cargo check --workspace --all-targets --locked
python3 scripts/ci/build_execution_inventory.py --check
python3 scripts/ci/check_documentation.py
python3 scripts/ci/check_workflow.py
python3 scripts/check_version.py
python3 scripts/knowledge/test_export.py
python3 scripts/ci/check_generated_surfaces.py
cargo fmt --all -- --check
git diff --check
```

Full workspace tests, crash stress, sanitizers, fixed-hardware benchmarks,
external SDK conformance, installer tests and Connectome tests are not C-06g
substitutes. They run at their owning gates, and their absence remains explicit.

## Package stop conditions

Stop and revise the plan before continuing if:

- a required path or behavior is outside the machine-enforced scope;
- DataFusion, Tokio, a provider SDK, network service, external database,
  sibling checkout, runtime fetch, hook, trigger, routine or skill dependency
  becomes necessary in `rrd-lsm`;
- correctness requires retaining the store mutex during stream consumption;
- a reader can observe post-capture state or lose its physical owners;
- GC can delete a live view's files;
- a limit can be exceeded before it is detected;
- a result can be partial while reporting success;
- a format fixture or semantic result must change unexpectedly;
- phase evidence can only be fabricated from concurrent cumulative counter
  subtraction; or
- documentation would imply universal zero-copy, RocksDB parity, DataFusion
  streaming, persistent reasoning, installation, Connectome readiness, alpha
  readiness or competitive performance.

## Immediate handoff after C-06g

The work does not branch into unrelated features:

1. C-06h attacks the new and old storage paths with generated histories,
   fuzzing, deterministic fail points and mixed-family interference.
2. C-06i measures compression, persisted filters, cache admission and optional
   key/value separation; unsupported ideas are rejected with evidence.
3. D-01 builds the primary `rrflow`/`rrflow.exe` install/create/open/serve/
   ready/commit/reopen/verify spine using the accepted C-06 substrate.
4. C-07 adds background maintenance, backpressure, complete recovery,
   storage-full and sustained-lifetime qualification through that installed
   product.
5. D/E/F build deterministic project state, native access paths and the true
   rrflowKV-to-Arrow/DataFusion stream.
6. G/H make persisted reasoning/recall and Connectome consume that one engine.
7. I implements the governed discussion-to-configuration, event, trigger,
   routine and skill chain described by the research draft.

The supporting execution map must list D-01 before C-07 to match the canonical
roadmap. That correction is part of this package's documentation review and
does not change either gate's completion status.
