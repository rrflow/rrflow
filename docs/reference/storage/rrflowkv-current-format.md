# rrflowKV current physical format

**Status:** active implementation reference for the single current typed application-key and accepted C-06 hybrid immutable-segment formats
**Coordinate:** `rrflow://rrflow-instance/data/reference/storage/rrflowkv-current-format`
**Owner:** physical bytes, limits, recovery rules, and pre-release format debt implemented by `rrd-store` and `rrd-lsm`

This record describes the bytes the current checkout can create or read. It is
not the accepted RRFlow 1.0 target. The target architecture is owned by the
[engine data-flow record](../../architecture/engine-data-flow.md); this file
does not own release status, and makes no performance or competitive claim.
Current comparison mechanics and evidence limitations are owned by the
[benchmark-harness reference](rrflowkv-benchmark-harness.md).
The [roadmap](../../roadmap/rrflow-1.0.md) owns the replacement and removal
work. Closed [POAM-002](../../poam/rrflow-1.0-alpha.md) records C-06 physical-
layout acceptance; closed POAM-003 records the accepted single-node reader and
whole-executable pre-release-shape convergence. C-07 and F remain separate
open qualification boundaries.

## Current and target boundary

The current application-key format is the C-01 `RRKV0001` typed ordered tuple
grammar. All live `RrflowKvStore` key construction and decoding uses it, and a
store whose manifest does not authenticate that application identity is
rejected. The mutable path remains a write-optimized WAL plus MVCC memtable.
Flush now transforms that sorted mutable state into segment v6: an ordered
key/version spine, six Arrow-layout buffers, one authenticated membership
filter per row group, and an authenticated none/adaptive-LZ4 writer policy.
Each page remains raw unless its independent LZ4 block meets the configured
saving threshold. Point, range, snapshot, CAS, recovery, and compaction reads
use those objects directly and do not invoke DataFusion.

This is the accepted C-06 physical implementation, not completion of the
persistent-engine alpha and not a claim that query output is zero-copy. The
segment owns Arrow-compatible buffer layout
and safe mapped-buffer ownership. C-06g exposes a synchronous selective
projected stream to the physical storage boundary, but Gate F must still adapt
that stream through a stamped `TableProvider`/`ExecutionPlan` and prove actual
`RecordBatch` borrowing, asynchronous backpressure, pushdown, cancellation,
and cross-operator resource bounds. Current semantic values can still be JSON
bytes inside the value-data page, and current rrflowQL adapters still decode
values and allocate result arrays.

The active physical readers accept exactly mutation batch v2, manifest v3, and
segment v6. Batch v1, manifest v1, and segment v1/v2/v3/v4/v5 bytes are negative
inputs that return an explicit unsupported-format error before an alternate
decoder can run. There is no row-segment compatibility reader, Fjall store,
backend selector, or migration executor. The disabled optional OpenRaft
adapter remains tracked by POAM-023 and excluded from the single-node alpha.
This record does not turn any rejected format or post-alpha implementation into
a product requirement.

## Implemented object set

All integers in binary objects are unsigned big-endian unless stated
otherwise.

| Object | New-write format | Current role | Implementation |
|---|---:|---|---|
| Application keys | `RRKV0001` | Typed ordered family/address/tuple spine for every `RrflowKvStore` key | [`key_codec.rs`](../../../crates/persistence/rrd-store/src/key_codec.rs) and [`keyspaces.rs`](../../../crates/persistence/rrd-store/src/keyspaces.rs) |
| WAL | 1 | Ordered atomic-batch frames and durability boundary | [`wal.rs`](../../../crates/persistence/rrd-lsm/src/wal.rs) |
| Mutation batch | 2 | Canonical put/delete payload inside one WAL frame | [`batch.rs`](../../../crates/persistence/rrd-lsm/src/batch.rs) |
| Manifest | 3 | Immutable reachable-state inventory, sequence boundary, and authenticated segment schema/codec/page-format identities | [`manifest.rs`](../../../crates/persistence/rrd-lsm/src/manifest.rs) |
| `CURRENT` and checkpoint controls | 1 | Authenticated manifest and retention pointers | [`manifest.rs`](../../../crates/persistence/rrd-lsm/src/manifest.rs) |
| Immutable segment | 6 | Ordered key/version spine plus raw or adaptive-LZ4 aligned Arrow-layout column pages and authenticated row-group membership filters | [`segment/mod.rs`](../../../crates/persistence/rrd-lsm/src/segment/mod.rs) and [`segment/format.rs`](../../../crates/persistence/rrd-lsm/src/segment/format.rs) |
| Physical snapshot bundle | 1 | Authenticated flush-bounded manifest closure | [`snapshot_bundle.rs`](../../../crates/persistence/rrd-lsm/src/snapshot_bundle.rs) |

Current default and hard bounds are implementation policy, not values for
callers to duplicate:

| Bound | Current value | Source |
|---|---:|---|
| Mutable encoded WAL payload before maintenance | 64 MiB | `DEFAULT_WAL_PAYLOAD_MAX_BYTES` |
| Mutable MVCC versions before maintenance | 524,288 | `DEFAULT_MEMTABLE_MAX_VERSIONS` |
| Individual WAL frame payload | 16 MiB | `WAL_MAX_PAYLOAD_BYTES` |
| Mutation operations per batch | 1,000,000 | `MAX_OPERATIONS` |
| Key/value bytes | 1 MiB / 8 MiB | batch and segment validators |
| Segment bytes / segment-index bytes | 1 GiB / 64 MiB | segment validator |
| Row-group target / maximum rows | 64 KiB / 2,048 | configurable `SegmentRowGroupBudget` and database defaults |
| Immutable page-cache capacity / default policy / protected target | 4 MiB / scope-aware scan-resistant LRU / 8,000 basis points | configurable `DatabaseOptions`; process-local and absent from durable identities |
| Adaptive-LZ4 minimum saving / maximum attempted logical page | 1,250 basis points / 16 MiB | configurable `SegmentCompressionPolicy`; hard bounds are `1..=9,999` basis points and `1 byte..=1 GiB` |
| Reconstructed logical bytes per segment | 1 GiB | segment metadata validator before page allocation |
| Active projected read views / ranges / selected runs | 1,024 / 1,024 / 4,096 | `ProjectedReadBudget` defaults |
| Pinned projected bytes / versions examined | 4 GiB / 16,000,000 | `ProjectedReadBudget` defaults |
| Projected page requests / logical page bytes | 4,000,000 / 8 GiB | `ProjectedReadBudget` defaults |
| Projected output rows / output buffer bytes | 1,000,000 / 1 GiB | `ProjectedReadBudget` defaults |
| Projected batch rows / allocated bytes | 8,192 / 64 MiB | `ProjectedReadBudget` defaults |
| Snapshot bundle bytes / segments | 1 GiB / 1,000,000 | snapshot-bundle validator |

Before a new batch crosses either mutable threshold, the single writer runs
the ordinary crash-ordered flush path. It never splits one accepted atomic
batch. An individually oversized batch remains one frame and is reported in
maintenance evidence.

## Typed application keys (`RRKV0001`)

Every application key is one self-describing tuple:

```text
format-tag / RRKV0001 / optional tenant / optional scope / family-tag / family / typed fields
```

The format and family tags are `0xf0` and `0xf1`. An absent address component
is `0x00`; present tenant, scope, and other text values use the text tag
`0x21`. Variable-width byte and text fields are split into eight-byte groups
with a canonical padding marker, so embedded NUL, slash, `0xff`, and every
other byte are data rather than delimiters. Fixed numeric fields are
big-endian; signed values flip the sign bit; descending version fields invert
the encoded `u64`, making newer versions sort first. The decoder rejects
unknown types, invalid UTF-8 text, non-canonical Boolean values or padding,
truncation, and trailing partial fields.

The frozen top-level family bytes are:

| Byte | Family | Byte | Family |
|---:|---|---:|---|
| `0x10` | current | `0x19` | vector |
| `0x11` | temporal | `0x1a` | projection delta |
| `0x12` | outgoing edge | `0x1b` | catalogue |
| `0x13` | incoming edge | `0x1c` | runtime commit |
| `0x14` | scalar | `0x1d` | outbox |
| `0x15` | unique | `0x1e` | audit |
| `0x16` | term dictionary | `0x1f` | engine event |
| `0x17` | term statistic | `0x20` | system |
| `0x18` | term posting |  |  |

The C-03 semantic writer currently assigns these implemented subspaces. The
tuple fields shown are logical typed components encoded by `KeyCodec`; they are
not delimiter-concatenated strings.

| Family | Subspace | Current key purpose |
|---|---:|---|
| catalogue | `0x20` / `0x21` | current runtime schema / retained runtime snapshot |
| catalogue | `0x30` / `0x31` / `0x32` | current schema-bound index commit binding / immutable binding revision / scope-level binding-set integrity head |
| scalar | `0x01` | current non-unique scalar entry keyed by scope, index, typed values, record, and descending valid-from |
| unique | `0x01` | current unique-window entry with the same typed value and record coordinates |
| vector | `0x01` / `0x02` | current vector head with commit coordinate / immutable temporal vector change |
| projection delta | `0x02` | generic projection work identity committed with its source mutation |
| projection delta | `0x03` | schema/catalogue/digest-bound scalar or BM25 old/new record fields |
| projection delta | `0x04` | vector-source old/new immutable-version pointers; no HNSW bytes |

An rrflowQL index-catalogue replacement and its commit bindings are written by
one control transaction. Installing or changing a scalar binding performs a
bounded current-record backfill; a unique binding rejects overlapping
non-null value windows before either the catalogue or entries publish. Later
record replacement or retirement removes the exact old scalar/unique key,
writes the exact new key when present, and records one materializer delta in
the semantic transaction. The binding-set head pins the complete projected
definition digest to the exact canonical catalogue bytes. A missing, partial,
or generically replaced projection therefore fails the next semantic commit
closed instead of silently bypassing an index constraint. The transaction also
rewrites the catalogue watermark value it observed, so a concurrent catalogue
transition conflicts at physical commit rather than publishing data under an
unobserved definition.

Current vector values are stored as heads containing the exact source commit
coordinate. Every vector insert, replacement, or retirement also writes a
temporal version plus source-specific pointer delta in the same semantic
batch. BM25 term dictionaries/postings, exact-vector access, TurboQuant, and
HNSW generations are not constructed here; Gates E-03 and E-04 consume the
committed deltas and must prove their independent publication and exact
fallback behavior. These semantic values remain encoded value bytes within
the v6 Arrow-layout buffers. A columnar physical envelope does not by itself
qualify their native access paths, candidate generation, recall, or exact
fallback behavior.

Catalogue subfamilies separately address function artifacts, definitions,
transaction bindings, membership revisions, the compare-and-swap head, and
invocation receipts. Artifact bytes remain values addressed by digest; they
are never copied into a key or repeated in a head record. Current per-project
semantic constructors use the global optional address and encode any existing
scope identity as a typed tuple field. The grammar nevertheless freezes and
tests distinct tenant and scope address components; changing their use is a
reviewed direct format convergence, not string concatenation or a compatibility
reader.

`prefix_end` computes the exact exclusive upper bound by carrying through
trailing `0xff` bytes. Range construction therefore uses a complete typed
component prefix and never a conceptual `*`, `~`, `+`, slash, or NUL delimiter.
The conceptual family symbols remain documentation shorthand only.

## WAL and acknowledgment

One accepted mutation batch is one WAL frame. `Authoritative` acknowledgment
follows `sync_data`; `Buffered` acknowledges a written frame without claiming
durability. A failed write or synchronization poisons that writer instance, so
the caller must reopen and recover rather than append after an unknown partial
write.

The deterministic write-fault surface names four ordered boundaries:
`write.prepared` after validation/admission and before append;
`write.wal_appended` after one complete frame is written but before the
authoritative sync; `write.wal_synced` after synchronization but before the
memtable becomes visible; and `write.visible` after the complete batch is
visible but before its receipt returns. Failures after append fence the writer
until reopen. Recovery may retain or discard an unacknowledged complete frame
at the unsynced boundary, but it may never expose only some operations from
that frame; synchronized and visible frames recover completely.

WAL file header, 16 bytes:

| Offset | Bytes | Meaning |
|---:|---:|---|
| 0 | 8 | ASCII `RRDWAL01` |
| 8 | 2 | format version `1` |
| 10 | 2 | header length `16` |
| 12 | 4 | CRC32C over bytes `0..12` |

Frame header, 32 bytes:

| Offset | Bytes | Meaning |
|---:|---:|---|
| 0 | 4 | ASCII `RRD1` |
| 4 | 2 | format version `1` |
| 6 | 1 | record kind `1` for an atomic batch |
| 7 | 1 | zero flags; unknown flags fail closed |
| 8 | 4 | payload length, at most 16 MiB |
| 12 | 8 | first MVCC sequence |
| 20 | 8 | last MVCC sequence |
| 28 | 4 | CRC32C over header bytes `4..28` and payload |

Sequence ranges must be contiguous and non-zero. Recovery replays the longest
valid prefix. An incomplete file header is corruption; an incomplete final
frame header or payload is a reported torn tail. Only
`repair_torn_tail` truncates that tail and synchronizes the file. Complete bad
magic, version, kind, flags, length, sequence, or checksum is corruption and is
never silently repaired.

## Atomic mutation batch v2

The 16-byte header is `RRDBAT02`, a `u16` version `2`, zero `u16` flags, and a
`u32` operation count. Each operation has a `u32` key length and a `u32` tagged
value length followed by key/value bytes. Bit 31 marks delete and requires the
remaining length bits to be zero; a clear bit is a put and permits an empty
value.

Empty batches or keys, mismatched magic/version pairs, unknown flags,
delete-with-length, trailing bytes, and out-of-contract lengths fail closed.
One MVCC sequence is allocated per operation while the complete payload stays
inside one atomic WAL frame.

The checked-in v1 bytes are a negative fixture. `RRDBAT01`/version 1 returns
`UnsupportedVersion` before any operation header is decoded; no v1 reader is
present. Writers and readers use only v2.

## Manifest, publication, and checkpoints

A manifest has a monotonically increasing generation, its parent digest after
generation 1, durable and WAL sequence boundaries, and every reachable segment
descriptor. Segment order is canonicalized before the manifest SHA-256 digest
is calculated. Each descriptor authenticates content identity, key and
sequence ranges, entry count, and byte count. L0 may overlap; higher levels
must contain ordered, non-overlapping key ranges.

Manifest v3 may carry one non-zero opaque `application_format`. `rrd-store`
binds the eight ASCII bytes `RRKV0001` as a big-endian `u64`. Every reachable
segment descriptor additionally authenticates segment format v6 plus the
current schema, key-codec, and page-format SHA-256 identities. Unknown, absent,
or different identities fail closed, and physical snapshot installation
requires an identical source and target identity. Cross-format movement is not
an alpha requirement; the final 1.0 tests create the accepted format directly.

Publication holds the operating-system writer lock, validates the expected
`CURRENT`, generation, and parent, writes and synchronizes immutable manifest
bytes, atomically renames the separately checksummed `CURRENT` pointer, and
synchronizes the directory. Named checkpoints are authenticated immutable
pointers that pin a manifest generation; rebinding a name fails, release is
explicit, and garbage collection uses the checkpoint inventory rather than
filename inference.

The manifest-v1 JSON is a negative fixture and returns `UnsupportedVersion`;
no v1/v2 manifest state reader is present. No superseded
application-key reader remains after C-01. These removals contribute C-05
closure but are not, by themselves, the evidence that accepted C-06.

## Hybrid immutable segment v6

A v6 segment contains a fixed 256-byte `RRDSEG06` header, 64-byte-aligned page
payloads, a bounded `RRDIX006` row-group index, and a 64-byte lowercase ASCII
SHA-256 footer over every preceding physical byte. Segment and page integers
are little-endian because the page values are Arrow-compatible native buffers;
the `RRKV0001` ordered key grammar above remains byte-order-preserving and is
not re-encoded by this layer.

Flush emits rows in `(key ascending, sequence ascending)` order and never
splits the complete version chain for one key across row groups.
`DatabaseOptions::segment_row_group_budget` configures the soft byte and row
targets used by both flush and compaction; the defaults are 64 KiB and 2,048
rows. Both values must be non-zero, the byte target cannot exceed the 1 GiB
segment limit, and the row target must fit `u32`; invalid configuration fails
before creating storage. Each segment authenticates its selected targets in
the v6 header, so historical segments remain self-describing if a later open
selects different targets for future writes. One key's chain may exceed either
target within the 1 MiB key and 8 MiB value bounds. Each row group owns exactly
six plain buffers, each stored raw or as one independent LZ4 block:

| Page | Arrow-compatible representation | Point-read role |
|---|---|---|
| key offsets | `(rows + 1)` little-endian signed 64-bit offsets | key-spine binary search |
| key data | concatenated non-empty key bytes | key-spine comparison |
| sequences | `rows` little-endian unsigned 64-bit values | MVCC visibility |
| value validity | Arrow least-significant-bit-first validity bitmap | tombstone detection |
| value offsets | `(rows + 1)` little-endian signed 64-bit offsets | selected value range |
| value data | concatenated non-null value bytes | selected value payload |

Every 96-byte page descriptor pins page/column/buffer/logical/physical type,
plain encoding, raw/LZ4 codec identity, row interval, null count, aligned
offset, stored and logical byte lengths, minimum/maximum statistics, and a
SHA-256 digest over the stored bytes. The v6 header authenticates the writer
policy: `none`, or adaptive LZ4 with a nonzero `1..=9,999` basis-point threshold
and a bounded maximum logical page size. The default is 1,250 basis points and
16 MiB. The row-group descriptor pins its strict key range. The
32-byte row-group-index header fixes filter format `1`, ten bits per unique
key, seven probes, and eleven zero reserved bytes. Each 32-byte row-group
header fixes its nonzero unique-key count and exact derived filter-word count;
the little-endian `u64` words follow the key bounds and six page descriptors.
The outer segment SHA-256 authenticates this complete index.

Standalone and newly written segment admission, plus snapshot-byte validation,
validate the outer digest, framing, schema/codec/page identities, every page
digest, Arrow offset/validity shape, key/sequence ordering, statistics, segment
counts, and a byte-identical filter reconstructed from decoded unique keys.
Normal manifest-based reopen authenticates the complete file digest and
metadata against the manifest, parses the bounded persisted filters, and reads
zero semantic pages. Runtime page loads authenticate and validate each selected
page, so tampering after open fails closed. Metadata admission also sums every
logical page with checked arithmetic and rejects a reconstructed segment above
1 GiB before decode allocation. `SegmentOpenEvidence` schema 3 records
format-probe, whole-file checksum, metadata, none/adaptive policy counts,
raw/compressed page counts, stored/logical bytes, persisted-filter count/bytes,
and semantic-page phases without changing the segment or manifest.
`RrflowKvOpenEvidence` freezes that validation record and checked page-cache/
I/O deltas for higher-layer checkpoint reconciliation before the store is
published. Later projected reads own their own meters, so segment admission,
startup reconciliation, and query work are not inferred from one concurrent
process-counter delta.

Point lookup first selects a row group by authenticated key bounds and its
persisted Bloom filter, then reads only key offsets/data/sequences and, for a
live match, value validity/offsets/data. Range and compaction scans read the
required complete row groups and never use probabilistic membership to suppress
a range. The filter is authenticated acceleration metadata rather than
semantic presence: a negative can skip point page I/O, while a positive still
executes exact key and MVCC comparison.

`Database::begin_projected_read` validates nonempty sorted disjoint half-open
ranges, the requested snapshot, and every nonzero budget before returning a
stream. It captures the sequence, manifest, an `Arc`-owned memtable generation,
and only range/sequence-eligible `Arc` segments. An active-view lease retains
the complete manifest closure for garbage collection. A write that encounters
a shared memtable uses copy-on-write; later flush or compaction publication
cannot change the captured generation.

The stream has one monotonic cursor per captured run and a minimum-key heap.
For each key it selects the sole greatest sequence visible at the snapshot;
equal key/sequence entries in separate live runs are corruption and fail
closed. Only the winning run reads validity, so a winning tombstone suppresses
older values. Keys-only output never requests value offsets or data. Key-value
output reads the winner's offsets, applies output/batch bounds, and copies the
selected value into a bounded output buffer. Each batch carries little-endian
signed 64-bit offsets plus key data and, when selected, value offsets/data as
64-byte-aligned `arrow_buffer::Buffer` owners. These are Arrow-compatible
buffers, not a DataFusion `RecordBatch`.

`ProjectedReadEvidence` reports manifest and snapshot identity, projection,
ranges/runs/row groups, pinned bytes, examined versions, page-family requests,
cache hits/misses/loads, actual mmap/io_uring/bounded reads, physical/logical
bytes, borrowed/decoded/decompressed/allocated/copied bytes, output and batch
peaks, and `creating`, `running`, `completed`, `cancelled`, or `failed` outcome.
A typed ceiling failure is not successful truncation. Completion, explicit
cancellation, failure, and drop fuse the stream and release its lease once.

All segments in one database share one exact-byte immutable page cache. Its
closed `PageCachePolicy` supports selectable exact LRU and default
`ScanResistantLru`; the latter admits into probationary residency, promotes
only reuse across logical engine operations, demotes least-recent protected
pages above the configured target, and evicts probationary pages first. One
checked, opaque, nonzero scope is generated per projected stream and shared
only by its internal cursors. Repeated touches from that same scope refresh a
probationary page and increment `same_scope_hits` without promoting it. The
scope is never public input, persistent state, authorization/query identity, or
trace dimension. Ordinary operations and later projected scopes may promote.

Cache evidence reports policy, total/probationary/protected resident bytes and
entries, hits, misses, loads, admissions, rejected admissions, promotions,
same-scope suppressions, demotions, evictions, and duplicate completed loads.
It also separates `bytes_read`, `bytes_decoded`, `bytes_borrowed`,
`bytes_allocated`, `bytes_copied`, and `bytes_decompressed`, plus filter
counters. A zero-capacity or oversize page remains readable but is rejected
from admission. Total and region accounting remain exact; I/O and decode occur
outside the cache mutex, and eviction cannot invalidate an `Arc<LoadedPage>`
already owned by a reader. In explicit mmap mode, an aligned raw page is exposed as an
`arrow_buffer::Buffer` borrowing the mapping; its allocation owner holds the
`Arc<Mmap>` for the buffer lifetime. A compressed page is authenticated in
stored form, decoded through the checked LZ4 block API into an exact-length
64-byte-aligned owned Arrow buffer, and charged by stored read, logical decode,
decompression, allocation, and cache-resident bytes. Bounded and io_uring raw
paths allocate an aligned Arrow buffer and read into it. Snapshot-byte
validation copies or decodes into an Arrow-owned buffer because the envelope
does not promise alignment or a stable mapped owner. Returned point values are
still copied into owned `Vec<u8>`.
Therefore only the page-buffer load is currently borrow-eligible; end-to-end
zero-copy query output is not claimed.

Encoding remains `plain`; codec identity is exactly `none` or `lz4` and must
agree with the authenticated segment policy. The default adaptive writer uses
checked worst-case scratch space and selects LZ4 only when its stored block
saves at least the configured threshold; it never falls back to an undeclared
codec. `none` remains an explicit valid policy. Persisted row-group filters,
adaptive LZ4, and scope-aware scan-resistant cache admission are integrated.
The cache policy is process-local, so reopening the same bytes under exact or
scan-resistant LRU changes no manifest, snapshot, sequence, or value. Key/value
separation and semantic-family cache partitions are rejected for this accepted
format: the former lacks atomic pointer publication, snapshot, recovery,
corruption, range-read, and value-log garbage-collection proof; the latter
duplicates family knowledge below the ordered typed-key boundary and can
strand capacity.

The filter distinction is important. `Segment::open` is a standalone deep-open
path: it reads and validates all six pages in every row group, reconstructs the
canonical filter from unique decoded keys, and rejects a mismatch. Normal
`Database::open` authenticates the manifest-owned segment and parses the same
filter from its bounded index without semantic page reads. An in-range point
miss can therefore record a filter negative and return before loading the key
spine after reopen. A maybe-present answer cannot establish presence and always
falls through to exact MVCC lookup.

The default-disabled `physical-policy-lab` feature mounts a measurement child
inside the canonical segment module so it can compare real v6 `none` and
adaptive-LZ4 output, inspect private parsed descriptors without copying the
format parser, and run the same persisted eight-family corpus through both
production page-cache policies. Its LZ4, filter, and cache observations consume
the sole production implementations. Zstandard and value placement remain
laboratory-only; the cache simulator remains screening history and cannot
substitute for the real-reader comparison.
Production `lz4_flex` is pinned with safe encode/decode and checked-decode
features; Zstandard remains an optional lab dependency. Any other retained
policy requires a separately planned explicit format revision and complete
recovery/snapshot/garbage-collection regression proof.

The clean C-06i candidate screen at revision `f7257fa` confirmed the v4 normal
reopen gap: 16,234 in-range filter checks produced zero filter negatives and
426 page loads. The serialized-filter candidate had zero member false
negatives, 0.634% observed false positives, and 13,304 bytes across 139 real
row groups, so authenticated persisted filters are the first retained-policy
implementation slice. Segment v5 implemented that slice with one canonical
authenticated filter and no v4 compatibility reader. Segment v6 directly
replaces v5 and integrates adaptive LZ4 with no v5 compatibility reader. The
clean v6 integration artifact at revision `eb7445e` records 50 raw and 784
compressed reopened pages, 1,418,038 stored versus 8,988,877 logical page
bytes, 336,041 query-decompressed bytes, and exact isolated-child identities.
The clean C-06j integration artifact at revision `5b1c31d` runs three isolated
release-profile children over that same persisted state. Exact LRU performs 48
post-scan hot-page loads; the integrated scope-aware scan-resistant policy
performs zero, records 18,200 same-scope suppressions, 48 promotions, and 48
protected entries, and remains within the exact 1 MiB experimental capacity.
Both policies produce identical manifest, semantic, and projected-row digests.
Zstandard remains laboratory-only. Value separation, family partitioning,
Moka, TinyLFU, and caller-controlled bypass are rejected for the accepted C-06
format. The prior filter/compression artifacts remain historical proof for
their slices; the C-06j artifact owns current cache-integration facts.

Segment v1/v2/v3/v4/v5 inputs return `UnsupportedVersion` at every file-open and
snapshot-validation boundary. No reader or migration path for them remains.

## Flush, compaction, snapshots, and garbage collection

Flush order is: synchronize the active WAL; write, synchronize, and rename the
content-addressed segment; create and synchronize the successor WAL; persist
the next manifest; publish `CURRENT`. Before `CURRENT`, recovery sees the old
state. After it, the new segment and successor WAL must both exist.

Compaction selects bounded source-level input and overlapping target-level
segments, performs a forward k-way merge, and publishes all output partitions
with one manifest compare-and-swap. The current default output target is
8 MiB, split only at key boundaries. Explicit compaction retains versions
visible to protected sequences; cooperative automatic maintenance retains all
versions because the copyable low-level snapshot is not a lifetime-tracked
reclamation lease. The C-06g projected stream is lifetime tracked: garbage
collection treats each active stream manifest as another root. It deletes only
objects unreachable from `CURRENT`, named checkpoints, and active projected
views after validating the complete root inventory.

A snapshot bundle begins with `RRDSNP01`, version `1`, zero flags, manifest
length, and segment count. It carries the authenticated manifest, each
descriptor and exact segment byte string, then a 64-byte lowercase ASCII
SHA-256 digest over the preceding envelope. Export first reaches a
flush-bounded manifest whose successor WAL is empty. Install validates the
complete closure before publishing a new local manifest; it never adopts the
source manifest history.

## Concrete fixture and failure examples

These are checked-in, executable examples rather than illustrative pseudocode:

1. [`batch-v2.hex`](../../../crates/persistence/rrd-lsm/fixtures/batch-v2.hex)
   encodes exactly three operations: put `alpha=one`, put `beta=two`, then
   delete `alpha`. The frozen-codec test decodes the bytes, rejects every
   truncation and a trailing byte, checks malformed delete length and version
   cases, and compares a fresh encoding byte-for-byte with the fixture.
2. `torn_tail_is_reported_and_only_explicit_repair_truncates_it` writes a valid
   prefix and torn tail, proves ordinary recovery does not mutate the file,
   repairs at the reported boundary, then reopens the accepted prefix.
3. `current_publication_is_ordered_content_addressed_and_compare_and_swap`
   proves stale publication cannot move `CURRENT` and accepted publication
   names the authenticated immutable manifest.
4. `v6_rejects_authenticated_length_flags_and_page_corruption` corrupts v6
   header/index/page fields and proves the reader fails closed.
5. `physical_snapshot_bundle_round_trips_installs_atomically_and_continues_writes`
   exports, installs, reopens, verifies the same state, and continues the
   sequence after the imported boundary.
6. `mmap_and_bounded_reads_are_identical_and_measure_page_ownership` proves
   identical results while mmap reports borrowed bytes and bounded/io_uring
   reports allocated bytes. It does not prove zero-copy DataFusion output.
7. `versions_remain_exact_when_one_key_exceeds_the_row_group_target` proves the
   writer does not split a key's MVCC chain at a row-group target.
8. `v6_bytes_match_the_checked_in_format_vector` freezes the complete current
   segment bytes independently of the snapshot envelope.
9. `projected_stream_matches_model_across_ranges_snapshots_and_projections`
   compares full, bounded, disjoint, empty, keys-only, and key-value streams at
   retained snapshots against the independent MVCC model.
10. `projected_stream_enforces_bounds_and_reports_selected_pages` proves every
    request/operation/batch ceiling, range/request validation, page-family
    selection, cancellation/drop, and Arrow-buffer alignment.
11. `projected_stream_rejects_equal_key_sequence_across_live_runs` proves that
    an ambiguous MVCC winner fails closed instead of depending on run order.
12. `pinned_projected_stream_survives_flush_compaction_and_gc_until_drop`
    proves an old generation remains readable and its files become reclaimable
    only after its stream releases the manifest lease.
13. `rrflow_kv_open_separates_segment_validation_and_reconciliation_io` proves
    startup phases are immutable and distinct from later query work.
14. `projected_read_adversarial` compares deterministic mixed-family operation
    histories with an independent MVCC oracle, drives every injected write,
    flush, and compaction boundary, and verifies cancellation, limits, reopen,
    compaction, and garbage-collection lifetime.
15. `rrflowkv_stress` runs the same oracle with explicit seed, case, and
    operation counts and reports stable replay/resource counters.
16. The nested cargo-fuzz targets drive the state-machine oracle and apply blind
    plus checksum-aware codec, length, page, and footer mutations to the frozen
    authenticated segment-v6 fixture under AddressSanitizer. Generated
    corpus/artifacts are ignored while reviewed seeds and the nested lockfile
    remain tracked.
17. `rrflowkv-physical-policy` runs isolated release-profile trials over a
    deterministic eight-family corpus, real v6 none/adaptive output, the
    create/flush/reopen miss workload, and the production exact versus scope-
    aware scan-resistant cache differential. It verifies integrated filter,
    LZ4, and cache behavior; authenticates semantic/durable identity and exact
    capacity; and keeps Zstandard/value-placement conclusions scoped.

## Frozen vectors

- [`rrflow-kv-key-codec-v1.hex`](../../../crates/persistence/rrd-store/fixtures/rrflow-kv-key-codec-v1.hex)
- [`rrflow-kv-store-layout-v1.hex`](../../../crates/persistence/rrd-store/fixtures/rrflow-kv-store-layout-v1.hex)
- [`wal-v1.hex`](../../../crates/persistence/rrd-lsm/fixtures/wal-v1.hex)
- [`batch-v1.hex`](../../../crates/persistence/rrd-lsm/fixtures/batch-v1.hex) — rejection input only
- [`batch-v2.hex`](../../../crates/persistence/rrd-lsm/fixtures/batch-v2.hex)
- [`manifest-v1.json`](../../../crates/persistence/rrd-lsm/fixtures/manifest-v1.json) — rejection input only
- [`manifest-v3.json`](../../../crates/persistence/rrd-lsm/fixtures/manifest-v3.json)
- [`segment-v6.hex`](../../../crates/persistence/rrd-lsm/fixtures/segment-v6.hex)
- [`snapshot-bundle-v1.hex`](../../../crates/persistence/rrd-lsm/fixtures/snapshot-bundle-v1.hex)

Changing current pre-alpha bytes requires changing the explicit version and
updating the relevant vector through the test-controlled
`RRFLOW_UPDATE_GOLDENS` path. Readers never infer a version.

## Executable proof

Run focused contract evidence before the package suite:

```bash
cargo test -p rrd-store --test key_codec --locked
cargo test -p rrd-store --test native_index_commit --locked
cargo test -p rrd-store --lib rrflow_kv::tests::rrflow_kv_multi_family_transaction_recovers_all_or_none_at_every_wal_boundary --locked -- --exact
cargo test -p rrd-store --locked
cargo test -p rrd-lsm --test failure_matrix --locked
cargo test -p rrd-lsm --test mvcc batch_codec_is_canonical_strict_and_frozen -- --exact
cargo test -p rrd-lsm --test wal torn_tail_is_reported_and_only_explicit_repair_truncates_it -- --exact
cargo test -p rrd-lsm --test manifest current_publication_is_ordered_content_addressed_and_compare_and_swap -- --exact
cargo test -p rrd-lsm --test segment v6_bytes_match_the_checked_in_format_vector -- --exact
cargo test -p rrd-lsm --test segment v6_rejects_authenticated_compression_metadata_and_block_corruption -- --exact
cargo test -p rrd-lsm --test segment v6_rejects_authenticated_length_flags_and_page_corruption -- --exact
cargo test -p rrd-lsm --test segment persisted_row_group_filters_prune_reopened_point_misses --locked -- --exact --nocapture
cargo test -p rrd-lsm --test segment v6_rejects_authenticated_filter_corruption --locked -- --exact --nocapture
cargo test -p rrd-lsm --test hybrid_segment --locked
cargo test -p rrd-lsm --test projected_read_adversarial --locked -- --nocapture
cargo run -p rrd-lsm --example rrflowkv_stress --locked -- --seed 14592251008053203194 --cases 16 --operations 96
cargo run --release --locked -p rrd-lsm --features physical-policy-lab --example rrflowkv-physical-policy -- --seed 14592251008053203194 --trials 3 --records-per-family 1024 --versions-per-key 2 --value-bytes 512 --misses 16384 --cache-bytes 1048576 --output docs/evidence/c06j-rrflowkv-scan-resistant-cache-linux-x86_64.json
cargo +nightly fuzz check --fuzz-dir crates/persistence/rrd-lsm/fuzz
cargo +nightly fuzz run --fuzz-dir crates/persistence/rrd-lsm/fuzz segment-v6-open -- -runs=4096 -max_len=256 -timeout=10
cargo test -p rrd-lsm --test tiered_io mmap_and_bounded_reads_are_identical_and_measure_page_ownership -- --exact
cargo test -p rrd-lsm --test snapshot_bundle physical_snapshot_bundle_round_trips_installs_atomically_and_continues_writes -- --exact
cargo test -p rrd-lsm
```

The key-codec tests prove C-01's ordered application-key contract, frozen bytes,
strict malformed-key rejection, real-store persistence, and close/reopen
readback. The native-index and complete semantic-fault tests contribute C-03's
insert, replacement, retirement, unique-conflict, catalogue-drift rejection,
source-delta, rrflowMX/rrflowKV differential, exact-key failure-boundary, and
rrflowKV-reopen evidence. C-03 is accepted by the canonical roadmap; that does
not qualify Gate E's bounded read paths/materializers or Gate C-06's physical
format. The remaining lower-level tests prove only the present object format.
C-05's lower source, dependency, opener, physical-reader, and upper-shape
closure evidence is recorded in its execution journals; C-05d records the
audit correction that forced C-05e through C-05h before the gate could close.
The optional post-alpha cluster implementation remains unqualified under
POAM-023. The v6 segment vector, fixed and generated mixed-family MVCC
comparisons, configurable authenticated row-group targets, malformed-byte
denial, ownership counters, bounded projected reads, pinned-generation GC, and
separated segment-open/startup-reconciliation/query evidence and authenticated
persisted-filter corruption/reopen-I/O proofs are now concrete C-06 evidence.
C-06h's finite stable/stress/sanitizer runs add reproducible adversarial
qualification. The adaptive-LZ4 slice adds exact none/adaptive reopen and
compaction differential, raw/owned-decode I/O evidence, checksum-aware
corruption denial, and current-format fuzzing. The C-06j slice adds a real
persisted mixed-family exact-versus-scan-resistant cache differential, exact
region/capacity and scope-suppression evidence, durable/semantic identity, and
explicit rejection of value separation and family partitioning. The canonical
roadmap accepts C-06 from the complete retained corpus. Gate C-07 separately
requires installed-path recovery, maintenance, concurrency, storage-full, and
reader-lifetime proof; Gate F separately requires a stamped streamed
DataFusion provider with projection/predicate/budget evidence. Passing this
suite cannot close those gates by itself.
