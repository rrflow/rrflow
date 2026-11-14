# C-06i rrflowKV adaptive page-compression engineering plan

**Status:** active supporting work-package evidence; segment-v6 adaptive-LZ4
implementation committed at `6d4e5aed1498200ec34315b63688e034bc6dc24c`
with evidence wording correction at `eb7445e399e393b37f8e788fd5c61cd3419725cb`;
C-06 remains open
**Coordinate:** `rrflow://rrflow-instance/data/work-package/c-06i-rrflowkv-adaptive-page-compression`
**Owner:** segment-v6 adaptive page-compression implementation order and proof
**Machine plan:** [`rrflow-1.0-active-change.json`](rrflow-1.0-active-change.json)
**Repository baseline:** `b2aa2fed46f598f99e0810e563f4ae4f390ebb14`
**Planning commit:** `59d17e215d3ec9189ad2b0b0f40fa8698856563b`

The canonical [RRFlow 1.0 roadmap](rrflow-1.0.md) owns completion status. This
record makes one C-06i production slice executable; it does not authorize a
second persistence engine, make DataFusion a storage owner, or complete C-06,
F-01, F-05, J-04, POAM-002, or an alpha outcome.

## Implementation result

The planned direct replacement is implemented. Segment v6 is the sole reader;
versions 1 through 5 fail unsupported. `SegmentCompressionPolicy` exposes
authenticated `none` and adaptive-LZ4 modes, with the planned default threshold
and page cap. Flush and compaction propagate the policy. The reader verifies
stored-page digests before checked exact-length LZ4 decode, bounds the aggregate
logical envelope before allocation, borrows eligible raw mmap pages, and owns
aligned decoded pages under exact physical/decode/cache evidence.

The owning segment, hybrid, tiered-I/O, manifest, snapshot, store-open,
architecture, all-target/all-feature, strict-Clippy, and structure-aware v6 fuzz
proofs pass. The fixed-machine artifact generated from clean revision
`eb7445e` records one warm-up and three retained zero-exit children, 50 raw and
784 compressed reopened pages, 1,418,038 stored versus 8,988,877 logical page
bytes, and 336,041 query-decompressed bytes. The artifact is integration
evidence only. Value placement, mixed-workload interference, cache admission,
C-07 installed recovery/lifetime, F-01 streaming, and release proof remain open.

## Decision

The next rrflowKV production slice is per-page adaptive LZ4 block compression
inside the sole immutable segment format. Segment v6 directly replaces v5.
The default writer attempts LZ4 independently for each of the six existing
Arrow-layout buffers and retains the raw buffer unless the stored block saves
at least 1,250 basis points (12.5 percent). A configurable `none` writer policy
remains available for measurement and deliberately uncompressed estates.

This is the narrowest retained change supported by both repository evidence
and primary sources:

| Candidate | RRFlow evidence | Production decision |
|---|---:|---|
| adaptive LZ4 | 1,426,554 stored bytes from 8,988,877 logical page bytes; 677/834 pages selected; exact round trips | implement now as the common page codec |
| adaptive Zstandard level 1 | 1,226,437 bytes; the same 677 pages; materially higher measured encode and decode CPU | retain only in the laboratory until C-07 owns real level/cold-placement policy |
| segmented LRU | 96/96 modeled post-scan hot hits versus 84/96 for current LRU | do not replace the cache without integrated admission, concurrency, pinning, and lifetime evidence |
| separated values | modeled compaction-byte saving only | reject; no pointer, recovery, snapshot, range-read, or value-log GC proof exists |

The fixed corpus is a selection input, not a release or superiority claim.
The production package must reproduce correctness and physical-byte reduction
through the real writer, reader, reopen, snapshot, and compaction paths.

## Boundary ownership

| Boundary | Owns | Must not own |
|---|---|---|
| `RrdEngine` | authorization, transaction stamp, semantic mutation, operation budget | codec choice for already-authorized physical pages |
| rrflowKV | page policy, immutable bytes, checksum verification, bounded decode, decoded-page cache, physical evidence | query semantics, model routing, or external lifecycle |
| rrflowMX | volatile implementation of the same semantic transaction/read contract | durable segment bytes |
| rrflowQL/DataFusion | stamped logical and physical planning over bounded streams of Arrow buffers | WAL, manifest publication, MVCC commit, page authentication, decode allocation, or cache ownership |
| native graph/BM25/vector operators | exact or approximate access paths selected by the RRFlow planner | a second persistence or transaction authority |

Compression is below the future `RrflowKvTableProvider`. rrflowKV reads and
authenticates stored bytes, decodes only selected pages, and hands
Arrow-compatible `Buffer` ownership to the projected stream. DataFusion polls
that stream during execution. It never sees an encoded LZ4 page and never
writes through a `DataSink` directly to disk.

## Public writer policy

Add one canonical type in `segment/format.rs` and re-export it from `rrd-lsm`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SegmentCompressionPolicy {
    None,
    AdaptiveLz4 {
        minimum_savings_basis_points: u16,
        maximum_page_logical_bytes: u64,
    },
}
```

The exact defaults and validation are:

- `DEFAULT_SEGMENT_COMPRESSION_POLICY` is `AdaptiveLz4` with `1_250` basis
  points and a 16 MiB maximum compressible logical page;
- the scale is exactly `10_000` basis points;
- `None` has no threshold and never invokes a codec;
- `AdaptiveLz4` accepts `1..=9_999`; zero is rejected because it admits blocks
  with no useful reduction, and 10,000 cannot select a nonempty block;
- `maximum_page_logical_bytes` accepts `1..=MAX_SEGMENT_BYTES`; a page above
  it is written raw without allocating codec scratch;
- the 16 MiB default covers the existing 8 MiB maximum individual value page
  with headroom, while bounding LZ4's documented worst-case scratch; it is a
  configurable safety default, not a performance claim;
- `DatabaseOptions.segment_compression` controls future flush and compaction
  output only;
- opening an existing current-format segment uses its authenticated metadata,
  not the process's current writer policy; and
- policy changes do not rewrite existing segments implicitly.

The selection predicate uses integer arithmetic only. Given `logical` and
`stored` byte lengths, a nonempty LZ4 block is selected exactly when:

```text
stored < logical
and
stored * 10_000 <= logical * (10_000 - minimum_savings_basis_points)
```

Both products use checked `u128` arithmetic. Equality selects compression.
No floating-point rounding, family-specific exception, environment probe, or
silent codec fallback may affect durable bytes.

## Segment v6 byte contract

### Identity and replacement

- segment version: `6`;
- segment magic: `RRDSEG06`;
- row-group index magic: `RRDIX006`;
- header: 256 bytes;
- row-group header: 32 bytes;
- page descriptor: 96 bytes;
- pages per row group: six;
- page alignment: 64 bytes;
- footer: the existing 64-byte lowercase SHA-256 text over every preceding
  stored byte;
- manifest version remains 3 and snapshot-bundle envelope remains 1;
- `SEGMENT_PAGE_FORMAT_DIGEST` changes; schema and key-codec digests do not;
- segment versions 1 through 5 are rejection inputs only; and
- there is no v5 reader, migration, alias, fallback, positive fixture, or
  dual-format branch.

### Header compression-policy fields

Existing fields through byte 175 keep their current meaning. The previously
reserved range begins with this authenticated policy record:

| Offset | Width | Field | Canonical values |
|---:|---:|---|---|
| 176 | 1 | writer-policy kind | `0` none, `1` adaptive |
| 177 | 1 | candidate codec | `0` none, `1` LZ4 block |
| 178 | 2 | minimum saving in basis points | `0` for none; `1..=9_999` for adaptive LZ4 |
| 180 | 8 | maximum compressible logical page bytes | `0` for none; `1..=MAX_SEGMENT_BYTES` for adaptive LZ4 |
| 188 | 68 | reserved | all zero |

The only accepted records are `(0, 0, 0, 0)` and
`(1, 1, valid_threshold, valid_page_limit)`. The
header records how the immutable file was produced for inspection and
reproducibility. It does not override each page's codec discriminator.
It does constrain that discriminator: a `none` segment may contain only raw
pages; an adaptive-LZ4 segment may contain raw and LZ4 pages, and every LZ4
descriptor must itself satisfy the authenticated minimum-saving predicate and
maximum-page limit.
An adaptive segment may be entirely raw when no page clears its threshold.

### Page descriptor codec field

The current type/layout fields, coordinates, statistics, lengths, and digest
retain their offsets. Bytes 5 through 7 become:

| Offset | Width | Field | Canonical values |
|---:|---:|---|---|
| 5 | 1 | page encoding | `0` plain Arrow-layout buffer |
| 6 | 1 | page compression | `0` none, `1` independent LZ4 block |
| 7 | 1 | reserved | `0` |

The length and digest rules are:

- compression `none`: `physical_bytes == logical_bytes`; the digest covers
  those raw stored bytes;
- compression `lz4_block`: `0 < physical_bytes < logical_bytes`; the digest
  covers the compressed stored block, not the decoded buffer;
- both lengths must convert to `usize`, fit the page and segment envelopes,
  and agree with the page-kind shape derived from row count;
- statistics always describe the logical Arrow-layout buffer; and
- unknown encoding, codec, or reserved values fail closed before page I/O.

RRFlow uses a raw independent LZ4 block, not an LZ4 frame and not a
size-prepended helper. The descriptor already carries authenticated physical
and logical lengths, so a second unauthenticated size field would create an
allocation authority.

### Logical segment envelope

`MAX_SEGMENT_BYTES` remains the 1 GiB physical file ceiling. Add an equally
strict logical envelope so compression cannot turn a small file into an
unbounded decode request.

Before any semantic page is loaded, metadata parsing must reconstruct the
hypothetical uncompressed layout in descriptor order:

1. begin at the 256-byte header;
2. align the logical cursor to 64 bytes before each page;
3. checked-add each page's `logical_bytes`;
4. align once before the index;
5. checked-add the authenticated index length and 64-byte footer; and
6. reject unless the result is at most `MAX_SEGMENT_BYTES`.

The writer performs the same calculation before publication. This preserves
the pre-compression v5 safety envelope instead of allowing compressed physical
size to hide excessive decoded state. Per-request projected-read limits remain
additional, smaller operational budgets rather than substitutes for format
safety.

## Codec dependency

Promote exact `lz4_flex = 0.14.0` from a laboratory-only optional edge to the
normal `rrd-lsm` dependency with:

```toml
lz4_flex = { version = "=0.14.0", default-features = false,
  features = ["std", "safe-encode", "safe-decode", "checked-decode"] }
```

The production path calls `lz4_flex::block::{compress_into,
decompress_into, get_maximum_output_size}` explicitly. It does not enable the
frame feature or deprecated crate-root helpers. Zstandard remains optional
behind `physical-policy-lab`. The workspace architecture oracle must prove
LZ4 is the one declared production codec for this slice and that Zstandard
does not enter the default executable graph.

## Write path

`Segment::write_from_memtable_with_cache` receives the validated compression
policy from `Database`. Both flush and every compaction output use the same
path; no compaction-only encoder is introduced.

For each logical page:

1. validate the logical page shape and account it against the logical segment
   envelope;
2. if policy is `None`, the page is empty, or its logical length exceeds the
   authenticated maximum compressible page size, append raw bytes without
   codec scratch;
3. otherwise compute `get_maximum_output_size(logical_bytes)`, reject arithmetic
   or conversion overflow, allocate that bounded scratch, and call
   `compress_into`;
4. apply the exact basis-point predicate to the returned initialized prefix;
5. append either the selected compressed prefix or the original raw bytes;
6. hash exactly the appended stored bytes;
7. record codec, physical length, logical length, and existing logical
   statistics in the descriptor; and
8. discard scratch before moving to the next page.

Compression errors fail the segment build. They never publish raw bytes as a
fallback, because that would make the same configuration and input produce
different durable identities. Publication retains the existing temporary
file, `sync_all`, content-addressed rename, manifest, and `CURRENT` ordering.

The encoder returns internal write evidence with raw/compressed page counts,
logical/stored page bytes, and compression scratch high-water bytes. The
database's existing flush/compaction trace records those low-cardinality
values; it never emits keys, values, prompts, or project labels.

## Read and ownership path

The reader must authenticate before decoding:

1. current-format probe and whole-file footer verification;
2. bounded header/index parse and logical-envelope validation;
3. selective stored-page range acquisition;
4. page SHA-256 verification over the stored physical bytes;
5. codec dispatch;
6. exact bounded decode when compressed; and
7. existing Arrow shape, order, statistics, filter, MVCC, and semantic checks.

For an LZ4 page, allocate one 64-byte-aligned `MutableBuffer` with logical
length, call `decompress_into(stored, output)`, and require the returned byte
count to equal `logical_bytes`. Short output, trailing encoded input rejected
by the checked decoder, invalid offset/match data, or any codec error becomes
`Error::InvalidSegment`; no partial buffer enters the cache.

Ownership remains explicit:

| Source and codec | Returned Arrow buffer | Borrowed bytes | One-load allocation evidence | Copied bytes | Decompressed bytes |
|---|---|---:|---:|---:|---:|
| mmap + raw | lease-backed mapped slice | logical | 0 | 0 | 0 |
| bounded/io_uring + raw | owned aligned buffer filled by I/O | 0 | returned capacity | 0 | 0 |
| snapshot bytes + raw | owned Arrow copy | 0 | returned capacity | physical | 0 |
| mmap + LZ4 | owned aligned decoded buffer | 0 | decoded capacity | 0 | logical |
| bounded/io_uring + LZ4 | owned aligned decoded buffer; stored scratch dies after decode | 0 | stored scratch capacity plus decoded capacity | 0 | logical |
| snapshot bytes + LZ4 | owned aligned decoded buffer from borrowed snapshot slice | 0 | decoded capacity | 0 | logical |

Only mmap plus raw is a zero-copy page result. A compressed page is never
described as zero-copy, even when its encoded input came from mmap. The page
cache stores decoded buffers and charges `buffer.capacity().max(1)`, so cache
capacity represents resident query-ready memory, not compressed disk bytes.

## Evidence contracts

Advance `SEGMENT_OPEN_EVIDENCE_VERSION` and add checked aggregate fields:

- `none_policy_segment_count`;
- `adaptive_lz4_policy_segment_count`;
- `raw_page_count`;
- `compressed_page_count`;
- `stored_page_bytes`; and
- `logical_page_bytes`.

Existing `PageCacheStats` and `ProjectedReadEvidence` fields remain the
operation authority for physical bytes read, logical bytes decoded, returned
bytes borrowed, allocation capacity incurred, bytes copied, and logical bytes
decompressed. Compressed loads must make `bytes_decompressed` nonzero; cache
hits must not claim a second read, allocation, copy, or decompression.

The existing `rrd_lsm::open` and `rrflow_kv.open` events add only aggregated
numeric counts/bytes. Segment-write traces add the configured policy kind,
threshold, counts, stored/logical page bytes, and scratch high-water mark.
Error messages identify format field or page ordinal/kind but never data.

DataFusion's `MemoryPool` is not credited with these bytes. Its contract does
not automatically account source batches flowing from a `DataSourceExec`.
F-04/F-05 must later compose rrflowKV source/decode/cache reservations with
DataFusion operator reservations into one request ledger.

## Corruption and failure matrix

Every malformed case must recompute the outer segment footer when the test is
intended to reach an inner check. Required cases are:

| Mutation | Required result |
|---|---|
| v1-v5 magic/version | `UnsupportedVersion` before alternate decode |
| unknown header policy, codec, threshold, page limit, or reserved byte | fail metadata parse |
| unknown descriptor encoding/codec or nonzero reserved byte | fail metadata parse |
| page codec conflicts with the authenticated header policy, saving threshold, or page limit | fail metadata parse |
| raw page with unequal lengths | fail metadata parse |
| compressed page with zero physical length or `physical >= logical` | fail metadata parse |
| checked-add/conversion/logical-envelope overflow | fail before allocation |
| page stored-byte mutation with stale page digest | fail stored-page checksum |
| page stored-byte mutation with recomputed page and file digests | safe LZ4 decode fails or deep semantic validation rejects |
| authenticated logical length too small or too large | exact decode or shape validation rejects without over-allocation |
| valid LZ4 bytes decoding fewer/more than declared logical bytes | fail exact decoded-length check |
| raw and compressed pages mixed in one row group | open and read exactly |
| codec error during write | no final segment, manifest, or `CURRENT` publication |
| crash/ENOSPC at existing segment/manifest boundaries | existing old-or-new recovery semantics remain exact |

No test may pass solely because the whole-file footer caught the mutation.
At least one checksum-rewritten test must reach every policy, descriptor,
decode-length, and deep semantic rejection family.

## Semantic and lifecycle proof

The independent projected-read model remains the oracle. Run the identical
generated eight-family operation corpus against policy `None` and default
adaptive LZ4 and require equality for:

- current and retained-snapshot point reads;
- complete and disjoint range scans;
- key-only and key/value projected batches;
- present values, tombstones, version order, and global MVCC merge;
- flush, reopen, protected compaction, pinned read, garbage collection, and
  snapshot export/install/reopen;
- every injected write, flush, compaction, and snapshot-install boundary; and
- cancellation and all existing page/output/resource denials.

The default-compressed path must prove at least one raw and one compressed page
in the same segment so both ownership branches execute. An incompressible
vector-family page must remain raw under the adaptive threshold; structured
pages must compress on the retained corpus.

## Frozen fixtures and fuzzing

Directly replace current-format artifacts:

- `fixtures/segment-v5.hex` -> `fixtures/segment-v6.hex`;
- `fuzz_targets/segment_v5_open.rs` -> `segment_v6_open.rs`;
- `fuzz/corpus/segment_v5_open/` -> `segment_v6_open/`;
- regenerate `fixtures/manifest-v3.json` because its segment descriptor pins
  v6 identity and checksum; and
- regenerate `fixtures/snapshot-bundle-v1.hex` because it contains the
  current physical segment, while retaining bundle format version 1.

The golden segment must contain both raw and LZ4 pages. The old v5 fixture is
used only as a rejection input during the direct replacement and is not kept
as a positive fixture or compatibility reader.

The current structure-aware target retains its maximum input length, timeout,
accepted-segment full-read oracle, and tracked replay seeds. Extend its bounded
mutations to cover header policy, descriptor codec, physical/logical lengths,
stored page data, and both page/file digests. Any accepted mutated segment must
complete all point/range/projected reads without panic or unchecked allocation.

## File-by-file implementation map

The production machine plan must authenticate every existing file below in
full before edits. A newly discovered path stops implementation and amends the
plan before that path changes.

| Path | Exact responsibility |
|---|---|
| `crates/persistence/rrd-lsm/Cargo.toml`, root `Cargo.lock` | promote safe checked LZ4 block support; retain Zstandard as laboratory-only |
| `rrd-lsm/src/segment/format.rs` | policy type/default/validation; v6 identity; header and descriptor bytes; adaptive selection; stored-byte digest; logical envelope |
| `rrd-lsm/src/segment.rs` | policy propagation; stored-page acquisition; checksum-before-decode; owned aligned decode; cache charge; open/write evidence |
| `rrd-lsm/src/segment/reader.rs` | retain request-before-load logical budgets; make existing decompression/allocation counters exact; no query semantic change |
| `rrd-lsm/src/database.rs` | `DatabaseOptions` policy; flush/compaction propagation; accessors and low-cardinality traces |
| `rrd-lsm/src/lib.rs` | one public policy/default/evidence export surface |
| `rrd-lsm/src/segment/physical_policy_lab.rs`, `examples/rrflowkv_physical_policy.rs` | compare real v6 none/adaptive output and publish clean integration evidence; keep Zstandard candidate-only |
| `rrd-lsm/tests/segment.rs` | exact v6 vector, threshold boundaries, mixed codecs, checksum-rewritten corruption, decode bounds, evidence |
| `rrd-lsm/tests/hybrid_segment.rs` | none/adaptive differential across semantic families, MVCC, reopen, protected compaction |
| `rrd-lsm/tests/tiered_io.rs` | raw mmap borrowing versus compressed owned decode across I/O modes |
| `rrd-lsm/tests/snapshot_memory.rs` | decoded cache residency and bounded reopen/read RSS evidence |
| `rrd-lsm/tests/manifest.rs`, `tests/snapshot_bundle.rs` | current segment identity through manifest and snapshot install; corruption and failure boundaries |
| `rrd-lsm/tests/support/projected_read_model.rs` and its stable/stress consumers | policy-parameterized independent semantic differential; preserve the restored shared oracle |
| `rrd-lsm/fuzz/Cargo.toml`, `fuzz/README.md`, target/corpus | direct v6 rename and bounded codec/length/digest mutation coverage |
| `crates/persistence/rrd-store/src/rrflow_kv.rs`, `tests/rrflow_kv_open.rs` | propagate open evidence and prove startup/query accounting remain distinct |
| `crates/authority/rrd-engine/tests/workspace_architecture.rs` | require v6 identity, one production LZ4 codec, no prior reader, no default Zstandard |
| current-format, benchmark, architecture-flow, test-plan, POA&M, roadmap-supporting records | state actual v6 behavior/evidence and remaining C-06/F/J gaps without changing canonical status early |
| new C-06i evidence JSON and evidence index | clean revision-bound none/adaptive integration observations and limitations |

No rrflowQL/DataFusion source changes in this slice. The storage result is a
prerequisite consumed by F-01, not an excuse to implement a partial provider
beside a format migration.

## Incremental edit order

1. Commit a new runtime machine plan against the clean documentation revision;
   validate it with zero post-plan paths.
2. Promote the dependency and add the policy type/configuration validation;
   run policy unit tests before format changes.
3. Advance format constants and implement header/descriptor encoding, adaptive
   write selection, and metadata/logical-envelope rejection. At this point the
   current golden tests are expected to fail because v5 is intentionally gone.
4. Implement stored-byte acquisition, checksum-before-decode, exact owned LZ4
   decoding, and cache/evidence accounting; run focused raw/compressed and
   checksum-rewritten corruption tests.
5. Thread the writer policy through flush and compaction and add the none versus
   adaptive independent-model differential.
6. Directly replace fixture, manifest/snapshot vector, fuzz target/corpus, and
   architecture identity in one reviewed current-format closure.
7. Update store trace propagation and the current documentation named by the
   machine plan; regenerate repository inventory.
8. Run focused tests, full owning-package tests, strict Clippy, architecture,
   workspace, documentation, generated-surface, version, and diff checks.
9. Reread every changed file and the complete diff; commit implementation.
10. From that exact clean commit, run the release-profile fixed-machine
    none/adaptive evidence command with one warm-up and three retained child
    processes. Commit evidence separately.
11. Resolve remote URLs and refs, push normally only to `development/main`, and
    verify the remote revision immediately. Do not touch official `origin`.

## First implementation oracles

The runtime package begins with these focused assertions before widening:

```text
adaptive_policy_rejects_invalid_threshold_and_page_limit
v6_adaptive_writer_selects_raw_and_lz4_pages_deterministically
v6_rejects_authenticated_compression_metadata_corruption
v6_rejects_authenticated_lz4_and_logical_length_corruption
v6_raw_mmap_is_borrowed_and_lz4_mmap_is_owned
none_and_adaptive_lz4_match_the_independent_mvcc_model
v6_bytes_match_the_checked_in_format_vector
```

The fixed-machine evidence passes only if:

- all child processes exit zero with identical corpus, format, configuration,
  deterministic byte, semantic, and counter identities;
- every selected page round-trips exactly;
- both raw and compressed pages occur;
- the retained corpus's aggregate stored page bytes are at least 12.5 percent
  below its logical page bytes;
- none and adaptive policies return identical semantic results through reopen
  and compaction;
- no logical-envelope, cache-capacity, snapshot, or publication invariant
  fails; and
- the artifact labels latency, CPU, RSS, and compression ratios as one-host
  integration evidence, not release or competitor proof.

## Verification ladder

```bash
python3 scripts/ci/check_change_plan.py
cargo test -p rrd-lsm --test segment <focused-v6-test> --locked -- --exact
cargo test -p rrd-lsm --test hybrid_segment \
  none_and_adaptive_lz4_match_the_independent_mvcc_model --locked -- --exact
cargo test -p rrd-lsm --test tiered_io --locked
cargo test -p rrd-lsm --test manifest --test snapshot_bundle --locked
cargo test -p rrd-store --test rrflow_kv_open --locked
cargo test -p rrd-lsm --all-targets --all-features --locked
cargo clippy -p rrd-lsm --all-targets --all-features --locked -- -D warnings
cargo clippy -p rrd-store --all-targets --locked -- -D warnings
cargo +nightly fuzz run --fuzz-dir crates/persistence/rrd-lsm/fuzz \
  segment-v6-open -- -runs=4096 -max_len=256 -timeout=10
cargo test -p rrd-engine --test workspace_architecture --locked
cargo check --workspace --all-targets --locked
python3 scripts/ci/build_execution_inventory.py --check
python3 scripts/ci/check_documentation.py
python3 scripts/ci/check_generated_surfaces.py
python3 scripts/ci/check_workflow.py
python3 scripts/check_version.py
cargo fmt --all -- --check
git diff --check
```

The retained release-profile integration command is:

```bash
cargo run --release --locked -p rrd-lsm \
  --features physical-policy-lab \
  --example rrflowkv-physical-policy -- \
  --output docs/evidence/c06i-rrflowkv-adaptive-page-compression-linux-x86_64.json
```

A dirty run is diagnostic and cannot populate the retained artifact.

## Deferred work

- Zstandard placement waits for C-07's real LSM level, background compaction,
  write-stall, and cold-data policy. It is not a hidden v6 option.
- Cache admission waits for integrated concurrent hot/scan traces, exact
  decoded-byte capacity, pinned-generation accounting, duplicate-load
  characterization, and F-05 request behavior. Moka is not inserted into the
  physical cache from a simulator result.
- Value separation remains rejected until one package owns pointer framing,
  atomic publication, snapshots, backup/restore, range-read amplification,
  crash-safe value-log GC, and corruption.
- Family clustering, BM25 postings, TurboQuant vectors, disk ANN, and graph
  adjacency remain native E-gate access paths over the same rrflowKV
  transaction and immutable-format authority.
- F-01/F-02 build the stamped DataFusion provider only after this page contract
  is stable and measured.

## Stop conditions

Stop and re-plan before continuing if:

- a required path or public symbol is absent from the runtime machine plan;
- LZ4 cannot be deterministic and safe with the pinned dependency/features;
- any length, alignment, threshold, scratch, or cursor calculation is
  unchecked;
- decode allocation can occur before current-format, whole-file, metadata, and
  logical-envelope validation or page-digest verification;
- compressed output is called zero-copy or charged only at its stored size;
- the same input/configuration can silently fall back to another codec;
- v5 remains accepted, a compatibility path appears, or a positive v5 fixture
  survives;
- none/adaptive results differ under the independent MVCC oracle;
- a checksum-rewritten malformed segment is accepted or can panic/over-allocate;
- flush, compaction, snapshot, pinned-view, or GC semantics regress;
- DataFusion, Moka, Zstandard, value separation, graph, BM25, ANN, install, or
  Connectome work leaks into the slice;
- a benchmark or one-host result is presented as release, scale, latency, or
  competitor proof; or
- any push targets official `rrflow/rrflow`, rewrites history, or is not
  verified against the exact development ref.

## Primary-source adaptation record

- The pinned [LZ4 block specification](https://github.com/lz4/lz4/blob/0774d05537f9762f838f7ab541b7765f1a729cb5/doc/lz4_Block_format.md)
  contributes independent-block semantics and the requirement for out-of-band
  compressed/decompressed sizes and safe bounded decoding. RRFlow owns its
  framing, authentication, limits, and tests.
- [`lz4_flex` 0.14.0](https://github.com/PSeitz/lz4_flex/tree/1bffdcbbf906a234b913937cb2f57c0245915038)
  contributes the safe checked block implementation. RRFlow rejects its frame
  and size-prepended convenience formats for persisted pages.
- The [Arrow columnar format](https://arrow.apache.org/docs/25.0/format/Columnar.html)
  contributes buffer separation and alignment constraints. Compression makes
  a page a decode path; RRFlow does not copy Arrow IPC compression framing.
- The [Parquet page metadata contract](https://github.com/apache/parquet-format/blob/bb22d0171b47000308e24209db79876b8dbe9566/src/main/thrift/parquet.thrift)
  supports separate compressed/uncompressed lengths and checksumming stored
  bytes. RRFlow does not copy Parquet pages, Thrift, encodings, or readers.
- [RocksDB compression guidance](https://github.com/facebook/rocksdb/wiki/Compression)
  contributes per-block raw fallback and the light-hot/heavy-cold policy
  question. RRFlow rejects missing-codec fallback and RocksDB's option model.
- [RocksDB block-cache guidance](https://github.com/facebook/rocksdb/wiki/Block-Cache)
  contributes decoded-block charging and scan/pinning concerns. It does not
  establish RRFlow's cache policy.
- DataFusion's [custom provider guide](https://datafusion.apache.org/library-user-guide/custom-table-providers.html)
  places I/O in the execution stream and requires truthful pushdown. Its
  [MemoryPool contract](https://docs.rs/datafusion/55.0.0/datafusion/execution/memory_pool/trait.MemoryPool.html)
  does not automatically account `DataSourceExec` or flowing batches, so
  rrflowKV retains explicit source/decode/cache budgets.
- [Moka](https://github.com/moka-rs/moka/tree/a616ec19e8d4ed938caf8b2c88090331d778d5da)
  contributes a future TinyLFU comparison only. Best-effort weighted capacity
  is insufficient evidence for the current physical cache.

No upstream crate tree, file format, public API, compatibility surface,
benchmark verdict, or hidden runtime is copied and renamed RRFlow.
