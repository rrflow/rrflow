# Gate C accepted evidence

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-c-evidence`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

## C-01 evidence (2026-09-08):

- `rrd-store` now owns one strict `RRKV0001` application-key grammar with typed
  `format / tenant / scope / family / tuple` coordinates. It freezes current,
  temporal, outgoing/incoming edge, scalar, unique, term dictionary/statistic/
  posting, vector, projection-delta, catalogue, runtime-commit, outbox, audit,
  engine-event, and system families plus six distinct function-catalogue
  subfamilies. Variable-width values are delimiter-free memcomparable groups;
  numeric and descending-version order is explicit.
- Every existing `RrflowKvStore` read, write, prefix, seek, snapshot inspection,
  and reopen path now uses the typed codec. The manifest authenticates
  `RRKV0001`; absent or different application identities fail closed. There is
  no dual write or earlier application-key reader. Physical-key construction,
  parsing, and its error variant were removed from `rrd-core`, leaving the
  kernel storage-independent.
- The frozen codec fixture covers every C-01 data family and catalogue
  subfamily; tag and round-trip tests additionally freeze engine-event and
  system families. Unit properties cover every possible final prefix byte,
  trailing `0xff` carry,
  embedded NUL/slash/`0xff`, component and tenant/scope isolation, signed and
  unsigned ordering, descending versions, malformed tags/types/padding/UTF-8,
  truncation, and exact round trips. A black-box fixture opens the underlying
  LSM after a real store commit, compares the exact persisted claim, sequence,
  and watermark bytes, then reopens through `RrflowKvStore` and reads the same
  claim.
- `cargo test -p rrd-core --locked`, `cargo test -p rrd-store --locked`, and
  strict all-target Clippy for both packages passed. The focused black-box
  codec test passed, all 23 workspace-architecture checks passed, and
  `cargo check --workspace --all-targets --locked` passed all 20 packages.
- This package closed only C-01; it supplied no C-02 transaction-parity,
  C-03 atomic multi-model write, C-04 direct stamped access, C-05 lower-level
  pre-1.0 reader-removal,
  C-06 hybrid Arrow-compatible pages, C-07 crash/lifetime proof, native graph/
  lexical/vector indexes, streamed DataFusion execution, and persistent
  reasoning/recall evidence.

## C-02 evidence (2026-09-09):

- `rrd-lsm` and `rrd-store` expose one consumed transaction with snapshot
  reads, point and half-open bounded range access, read-your-writes, put,
  delete, commit, rollback, typed write conflict, and snapshot pinning.
  rrflowMX and rrflowKV run the same corpus; only rrflowKV promises reopen.
- Claim, control, projection, runtime, and invocation semantics now live in
  concrete repositories over a borrowed transaction port. `StorageEngine`
  retains only transaction creation, repository access, and bounded physical
  evidence; rrflowMX's duplicate semantic maps and rrflowKV's parallel direct
  semantic implementations were removed rather than forwarded.
- Repository conformance proves identical stored semantics on both profiles
  and authoritative rrflowKV readback after reopen. Concurrent disjoint
  control writes replan boundedly around the shared hash-chained journal while
  true compare-and-swap changes still fail; logical snapshot leases create and
  reconcile their physical rrflowKV checkpoint with commit outcome.
- Source-boundary enforcement rejects semantic repositories that import
  rrflowKV, rrflowMX, `rrd-lsm`, or raw database writes, and rejects restoration
  of broad semantic methods on `StorageEngine`. Exact searches found no such
  write-around or former semantic trait method.
- All 157 `rrd-store` tests and all 119 `rrd-engine` tests passed, including
  MX/KV differential, conflict, rollback, snapshot, crash/reopen, semantic
  repository, engine-authority, and workspace-architecture cases. The changed
  inference and downstream package suites passed, as did strict locked
  workspace all-target Clippy. This closes C-02 only; C-03 atomic multi-model
  effects, C-04 direct stamped reads, hybrid Arrow pages, streamed DataFusion,
  and persistent reasoning/recall remain open.

## C-03 evidence (2026-09-09):

- One `SemanticCommitPlan` encodes current and temporal records and relations,
  current and temporal outgoing/incoming adjacency, schema-bound scalar and
  unique changes, BM25 and vector source deltas, runtime changes, durable
  projection work, outbox, semantic audit, commit cursor/outcome, and prepared
  function receipts. Both storage profiles consume that same plan through the
  C-02 transaction port; rrflowKV alone claims reopen durability.
- Governed-function catalogues now publish content-addressed binary artifacts,
  typed immutable definition/binding records, digest-only membership revisions,
  and one compare-and-swap head. Public admission caps decoded artifacts at
  3 MiB and canonical catalogue JSON at 4 MiB; the store also constructs the
  exact full publication closure and rejects it unless it fits one current
  16 MiB rrflowKV WAL batch before beginning a transaction.
- Deterministic low-level injection covers prepared, complete-WAL-appended,
  WAL-synced, and process-visible-before-acknowledgement boundaries for crash
  and storage-full modes, both borrowed and owned writes, and buffered and
  authoritative durability. The complete multi-family semantic key set is
  compared before and after reopen at every boundary; no family becomes partly
  visible, and post-WAL uncertainty requires reopen before another write.
- Function recovery rejects a validly resealed prepared receipt whose runtime
  build differs from the pinned definition. A separate lost-acknowledgement
  test makes the domain batch and receipt durable, advances the active
  catalogue to a rejecting function, reopens, and proves the exact receipt
  closes the session transaction with no second guest execution or cursor
  advance. A rejected receipt cannot leave domain state, outcome, or an orphan
  allowed receipt.
- The complete locked `rrd-contract`, `rrd-lsm`, `rrd-store`, and `rrd-engine`
  suites passed: 76, 76, 166, and 117 tests respectively. Strict affected-
  package Clippy, workspace all-target checking, architecture, documentation,
  generated-surface, inventory, formatting, and diff-integrity checks are
  recorded in the C-03d execution journal.
- This closes only C-03. C-04 still owns bounded direct reads; C-05 removes
  earlier physical readers; C-06 builds hybrid Arrow-compatible immutable
  pages; C-07 supplies final storage recovery/lifetime qualification; Gates E
  and F still own native materializers/access paths and streamed DataFusion;
  persistent reasoning/context, installation, routines/skills, Connectome,
  deployment, optimization, and release evidence remain open.

## C-04 evidence (2026-09-09):

- One authenticated direct reader selects typed semantic versions through
  bounded point/prefix/version access at a supplied `ReadStamp`. Current-head
  validation uses current-state point reads with zero change reads or proof
  nodes; retained historical stamps use bounded accumulator and direct
  semantic-version proofs. Repository snapshots, transaction previews,
  retirement validation, rrflowQL catalogue/execution, query, vector,
  retrieval, context, memory, seat, router, embedding, and operator-knowledge
  paths consume that boundary rather than reconstructing normal state from the
  runtime change log.
- The shared all-model corpus covers two schema revisions, claims, records,
  relations, events, vectors, series, geo values, immutable references, typed
  retirements, valid-time corrections, and a foreign scope. Direct results
  equal the authenticated-log oracle byte for byte on rrflowMX and rrflowKV at
  retained and current stamps, and again after rrflowKV close/reopen. Retained
  reads reported 49 point reads, 10 ranges, 68 keys/values, and 15,371 decoded
  bytes; current reads reported 89 point reads, 9 ranges, 107 keys/values, and
  17,109 decoded bytes. Reopened rrflowKV reported 9 additional block loads,
  16,128 loaded bytes, and 281 filter checks.
- Query physical plans name direct version sources and expose nonzero stamped
  path evidence. Tight key budgets fail closed rather than falling back to
  replay. Event identity lookup remains bounded, and the exact query corpus
  retains replay only as an independently compared test oracle.
- A workspace source-closure test enumerates every `read_changes`, alternate
  runtime-change reader, and kernel snapshot reducer occurrence by exact path,
  kind, count, and named operation. Normal query, vector, retrieval, context,
  memory, inference, embedding admission, vector-search admission, and
  operator-knowledge admission have no causal-log source. The remaining
  production log users are explicit diagnostic, rollback, cluster artifact-
  transfer, projection-catalogue rebuild, and trace-only conflict-recovery
  operations; reducers outside tests consume directly selected versions.
- The complete locked workspace all-target test suite, complete strict
  workspace all-target Clippy, workspace all-target check, focused source-
  closure and direct-read differential/reopen tests, all 24 architecture
  guards, and documentation/generated-surface/inventory/workflow/version/
  formatting/diff policies passed as recorded in the C-04c3 execution journal.
- This closes only C-04. C-05 still removes pre-1.0 physical readers; C-06
  builds the hybrid key/version spine and Arrow-compatible pages; C-07 owns
  final durability, maintenance, and buffer-lifetime qualification; E owns
  native graph/scalar/BM25/vector access paths; F owns streamed DataFusion and
  one cross-operator resource ledger. Persistent reasoning/context,
  installation/attunement, routines/skills, Connectome, deployment,
  optimization, and release proof remain open.

## C-04 reopened (2026-10-17):

- The C-04 evidence above records that vector-search admission has no
  causal-log source. That does not hold. Every vector search rebuilds the
  vector artifact catalogue (`changes_since` from cursor 0, unstamped) and the
  quantization catalogue (`read_changes` from cursor 0 at a fresh stamp)
  through `reopen_vector_runtime_metadata`. HNSW build, publication preflight,
  collection and payload-index deletion, and quantization operations rebuild
  them too, and the unwired `rrd-maintenance` crate rebuilds its run state the
  same way. [POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md) records the
  deficiency.
- The source-closure test behind that claim never searched for
  `changes_since(` or `snapshot_changes(`, and it allowlisted the per-search
  `read_changes(` call as an explicit quantization rebuild. It now enumerates
  every public runtime change-page reader and lists the three replays as
  POAM-028 known deficiencies.
- C-04 stays unchecked until both catalogues are served by direct stamped
  reads at the request's `ReadStamp` and POAM-028's closure evidence is
  recorded. Record, relation, query, retrieval, context, memory, and
  vector-candidate selection keep the direct-read behavior accepted above.
- The package record is the
  [C-04d source-closure audit correction journal](../../evidence/change-journals/gate-c/c-04d-source-closure-audit-correction-journal.md).

## C-05 accepted convergence evidence and audit correction (2026-09-09):

- C-05a at `b8f07c97775d4440df728bceb261a1128c4d816a` removes the
  pre-1.0 batch, manifest, and segment decoders and the nonempty
  cursor-zero accumulator reconstruction path. Earlier physical bytes now
  fail with an explicit unsupported-version result before another decoder or
  memtable publication; missing authenticated accumulator state at a nonzero
  cursor fails closed on rrflowMX, rrflowKV, and rrflowKV reopen.
- C-05b at `e1a855791fcc0dfcccb7ba72db10bc1804a045f3` requires
  `EstateDocument` authority and all eight backup/recovery maps and requires
  the bound recovery-policy snapshot in both internal and public backup-job
  state. The absence branches, legacy comment, and successful incomplete
  fixtures are removed. Negative tests reject omission of every named field;
  one fresh estate/backup/recovery corpus is identical on rrflowMX, rrflowKV,
  and rrflowKV reopen. This accepts the evidence required to close POAM-018.
- C-05c at `cd1bf495ece3c6698cde1cca413bfbe7be55588c` removes the unused
  `rrflow-cli` development dependency on `rrd-lsm` and adds an executable
  architecture guard over dependency kind, optionality, and feature
  resolution. The independently installable alpha has exactly one required
  production edge into `rrd-lsm`, from `rrd-store`; `rrd-cluster` defaults to
  no implementation feature and `rrd-engine` enables only its pure
  `object-transfer` contracts. The guard also pins the sole current batch v2,
  manifest v2, and segment v3 readers and rejects revival of retired reader or
  backend names.
- C-05e, in the reviewed change containing this record, removes the successful
  vector artifact catalogue v1 validation branch and its shorter identity
  tuple. One current v2 tuple includes build evidence, and an internally
  digest-consistent v1 entry fails before artifact decoding or engine replay.
  Current v2 publication remains identical on rrflowMX and rrflowKV, and the
  rrflowKV catalogue still reopens before missing artifact bytes fail closed.
- C-05f, in the reviewed change containing this record, makes the canonical
  logical table map required and nonempty in both the kernel and public schema
  contracts. Specialized record, relation, and event constraints must resolve
  to an explicit strict table of the matching family; vector, series, geo, and
  object mutations always resolve through the same map. Paired authoring
  methods publish each strict specialized schema with its table identity, and
  snapshot reconstruction has no caller-selected model fallback. Schema bytes
  and generated client projections carry that authority explicitly.
- C-05g, in the reviewed change containing this record, removes every
  collectionless runtime-vector representation. The public mutation, kernel
  value, inference job, persistent vector source, commit digest, source-delta
  key, and compact/quantized/TurboQuant artifact metadata all require the same
  canonical collection plus named-vector address. Omitted artifact metadata and
  public/kernel fields fail decoding, empty physical address parts fail before
  key encoding, and collection/name changes produce distinct commit and source
  identities. Search/list filters remain optional query selectors and are not
  persisted identity.
- C-05h, in the reviewed change containing this record, makes the generic
  artifact catalogue exact/compact/HNSW-only and makes quantization lifecycle
  restoration explicit and revision-neutral. Generic scalar/product/binary/
  TurboQuant publication fails closed; the public TurboQuant
  `ensure_vector_index` shape no longer decodes; runtime reconstruction has no
  TurboQuant suppression branch; and only an active lifecycle generation
  enters the shared planner. Explicit build/activate/search/reopen/retire,
  HNSW, hybrid RRF, memory-tier, and application-backup behavior remains.
- C-05a through C-05c and C-05e through C-05h pass their focused and owning suites, all 28 workspace
  architecture guards, the complete default workspace all-target test and
  strict Clippy matrices, and the repository inventory, documentation,
  generated-surface, workflow, version, formatting, and diff policies. Exact
  commands, counts, source probes, and scope exclusions are recorded in the
  C-05a through C-05h execution journals.
- C-05c closes only the independently installable alpha's physical dependency
  and reader slice. The first C-06 owner read then followed every active C-05
  reference and found compiled successful pre-release paths above that slice:
  vector artifact catalogue v1 beside v2; missing-table schema derivation and
  caller-selected snapshot model fallback; missing vector collection
  addresses; and suppression of the second vector/TurboQuant catalogue during
  runtime reconstruction. C-05e through C-05h directly remove those four
  items. The C-05d correction journal remains the audit trail that prevented
  lower physical closure from being mistaken for whole-executable closure.
- The optional post-alpha OpenRaft implementation still contains direct cluster
  openers and remains visible under POAM-023; it is neither enabled nor
  qualified by the alpha composition and cannot enter a release until a new
  distributed gate accepts it. The accepted single-node executable has no
  remaining confirmed C-05 alternate reader, store, migration, missing-
  authority shape, duplicate vector publication path, or suppression shim.
  C-06 may now begin the hybrid ordered-spine/Arrow-page work.

## C-06 accepted evidence (2026-09-10 through 2026-09-13):

- C-06a through C-06f established the now-superseded segment-v4 proof. At that
  revision, `rrd-lsm` wrote and exclusively read v4. Flush preserved one
  sorted `(key, sequence)` MVCC spine and emits Arrow-compatible key offsets,
  key data, sequence values, value validity, value offsets, and value data for
  each row group. A key's complete version chain is never split by the target
  byte or row budget. Point, range, compaction, and snapshot reads use the
  physical pages without invoking DataFusion.
- Manifest v3 authenticates each reachable segment's physical version, schema
  digest, key-codec digest, and page-format digest. Segment v1/v2/v3 and
  manifest v1/v2 have no accepted reader or migration path. The complete v4
  bytes and manifest v3 bytes are checked-in vectors.
- Every page is 64-byte aligned and independently authenticated. Explicit mmap
  returns an Arrow buffer whose allocation owner pins the mapping; bounded and
  io_uring reads allocate aligned Arrow buffers; snapshot-envelope validation
  copies. Page evidence distinguishes read, decoded, borrowed, allocated,
  copied, decompressed, cached, and filter activity. No end-to-end DataFusion
  zero-copy claim is made.
- `DatabaseOptions::segment_row_group_budget` now supplies validated byte and
  row targets to every new flush and compaction output. Each segment
  authenticates those targets in its header, so historical generations remain
  self-describing when future writer configuration changes; the checked-in
  default-format vector remains byte-identical.
- Exact point/range/MVCC comparisons, a version chain larger than one row-group
  target, page corruption, post-open tampering, cache bounds, mmap/bounded/
  io_uring ownership, snapshot, and compaction suites exercise this slice. Two
  implementation failures surfaced and were fixed: all-tombstone groups need
  a full Arrow validity bitmap, and compaction outputs must derive their actual
  maximum sequence instead of inheriting an unrelated durable watermark.
- `hybrid_segment.rs` now runs four fixed-seed histories (384 generated
  mutations) across control, graph-edge, record, lexical-term, and vector key
  families. An independent ordered model checks point, batched-point, full,
  bounded, and disjoint multi-range reads at every committed snapshot before
  and after reopen, then at protected snapshots after pruning compaction and a
  second reopen. Four row/byte budgets remain authenticated, reopen performs no
  semantic-page reads before a query, and 75 deterministic malformed or
  truncated segment files fail closed without a parser panic.
- C-06g captures one sequence, manifest, `Arc<Memtable>`, and eligible
  `Arc<Segment>` set without retaining a database borrow. A bounded registry
  pins the complete manifest closure for garbage collection; later writes use
  copy-on-write memtable generations, and flush/compaction can publish newer
  segments without changing the captured view.
- Its synchronous projected stream validates sorted disjoint half-open ranges
  and all request/resource ceilings, merges one forward cursor per eligible run
  by key, rejects equal key/sequence ambiguity, applies the greatest visible
  version, and suppresses a winning tombstone. It loads key offsets/data and
  sequences first, then only the winning validity page; value offsets and data
  are absent from keys-only evidence and deferred for key-value output.
- Output is emitted in bounded Arrow-compatible `i64` offset/data buffers.
  Stream-local evidence distinguishes page families, cache work, actual mmap/
  io_uring/bounded reads, logical and physical bytes, ownership, decode,
  allocation, copy, output, batches, cancellation, failure, and completion.
  Immutable rrflowKV open evidence separately reports whole-file segment
  validation and startup checkpoint reconciliation; later queries cannot
  mutate that record.
- The independent four-seed history also consumes projected streams with one-
  and three-row batches for keys-only and key-value projections at retained
  snapshots before/after reopen and protected compaction. Focused tests cover
  every declared request/operation/batch ceiling, cancellation/drop lease
  release, duplicate-sequence denial, and old-view survival through flush,
  compaction, and garbage collection. The complete `rrd-lsm` and `rrd-store`
  suites and strict affected-package Clippy pass at the C-06g candidate.
- C-06h adds one reusable deterministic state-machine model over audit,
  incoming/outgoing edge, record, runtime, scalar, term, and vector families.
  Stable tests compare point, broad/disjoint range, keys-only, and key-value
  projected reads at retained snapshots while mixing atomic writes, deletes,
  flush, protected compaction, reopen, garbage collection, pinned reads,
  cancellation, resource denial, and every write/flush/compaction fault
  boundary. Failures retain exact seed, input digest, and input bytes.
- A configurable `rrflowkv_stress` run passed 1,536 operations with 283
  injected failures, 382 reopens, 189 compactions, 264 garbage collections,
  and 1,334 projected reads. The complete `rrd-lsm` suite passed 94 tests and
  strict all-target Clippy.
- A locked nested cargo-fuzz project leaves the product dependency graph and
  root lockfile unchanged. The version policy separately validates its
  explicit cargo-fuzz marker, unpublished `0.0.0` package, one-member nested
  workspace, and location below a declared product crate instead of weakening
  product workspace parity. AddressSanitizer-backed bounded runs completed 512
  state-machine executions and 4,096 authenticated segment-v4 mutation/open
  executions without a crash, timeout, model difference, parser panic, or
  accepted-but-unreadable segment. These finite runs do not prove absence of
  defects or replace C-07/J continuous and cross-platform qualification.
- The first C-06i production slice directly replaces v4 with segment v5. Each
  row-group index authenticates its unique-key count and canonical ten-bit/
  seven-probe Bloom words. Normal manifest reopen parses nonzero filter
  count/bytes with zero semantic-page operations; standalone/snapshot
  validation reconstructs exact words from decoded unique keys; checksum-
  rewritten malformed filters fail; definite point misses prune page loads;
  and present/tombstone/MVCC reads remain exact. Segment versions 1 through 4
  are rejection inputs with no compatibility reader or positive fixture. The
  clean `07a6bb8` fixed-machine artifact records 139 filters/11,080 raw bytes,
  zero member false negatives, 0.634% observed false positives, zero semantic-
  page open work, and 114 page loads for 16,106 in-range miss checks; it is not
  release, cross-platform, all-workload, or competitor evidence.
- The adaptive-compression C-06i slice directly replaces v5 with segment v6.
  Its header authenticates none/adaptive-LZ4 policy; every descriptor
  authenticates raw/LZ4 identity plus stored/logical lengths and stored-byte
  digest. The reader verifies stored bytes before checked exact-length decode,
  rejects an aggregate logical envelope above 1 GiB, borrows only eligible raw
  mmap pages, and charges owned aligned decompression separately. Flush and
  compaction use the configured policy. None/adaptive independent-model
  histories agree through reopen and protected compaction; exact malformed
  metadata/block/length/envelope cases fail; segment versions 1 through 5 are
  rejection inputs; and the structure-aware v6 target completed 4,096 bounded
  sanitizer executions. Clean revision `eb7445e` records 50 raw/784 compressed
  reopened pages, 1,418,038 stored/8,988,877 logical page bytes, 336,041
  query-decompressed bytes, and three zero-exit retained children. This is one-
  host integration evidence, not release or superiority proof.
- C-06j integrates two closed production cache policies below the projected
  reader: selectable exact LRU and default scan-resistant
  probationary/protected LRU with an 8,000-basis-point protected target. One
  checked opaque scope is allocated per projected stream; repeated touches in
  that scope refresh probationary recency and count suppression, while later
  operations may promote. The scope is not public, durable, semantic,
  authorization, or trace identity.
- Clean revision `5b1c31d` and artifact SHA-256
  `8a008ee33bb50ca197783227cfcfbb4d58945dd2f26dca8cdf12e1804106b9ad`
  run one persisted eight-family corpus in three isolated release-profile
  children. Exact LRU performs 48 post-scan hot-page loads; scan-resistant LRU
  performs zero, records 18,200 same-scope suppressions, 48 promotions, and 48
  protected entries, stays within 1,048,576 bytes, and preserves identical
  manifest, semantic, and projected-row digests. The real reader, not the
  simulator, supplies the acceptance counters.
- The same package preserves all 114 all-target/all-feature `rrd-lsm` tests,
  strict Clippy, the complete `rrd-store` suite, all 28 workspace-architecture
  guards, and the locked workspace all-target check. It records rather than
  claims concurrent load coalescing. C-07 owns lock contention, load
  coalescing, sustained maintenance, installed-path recovery, and cross-
  platform lifetime.
- Current evidence rejects value separation because the byte-only model cannot
  establish atomic pointer publication, snapshot reachability, recovery,
  corruption denial, range-read behavior, or value-log garbage collection.
  It rejects semantic-family cache partitions because the canonical key
  ordering already supplies locality and physical quotas would strand capacity.
  Moka, TinyLFU, and caller-selected bypass remain unadopted.
- C-06 is accepted. F-01 still owns the stamped asynchronous DataFusion
  provider; C-06g/C-06j do not emit a DataFusion `RecordBatch` or claim end-to-
  end zero-copy.
