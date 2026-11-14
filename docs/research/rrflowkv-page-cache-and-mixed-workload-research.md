# rrflowKV page-cache and mixed-workload research

**Status:** active supporting research; implementation handoff complete and canonical acceptance remains owned by the roadmap
**Coordinate:** `rrflow://rrflow-instance/data/research/rrflowkv-page-cache-mixed-workload`
**Owner:** source-backed input to C-06; not architecture, roadmap, or completion authority
**Reviewed:** 2026-09-13
**Baseline:** `9592b886f2c6f2f716933fcf46e8c0595de3c315`

## Decision in one paragraph

RRFlow should retain an RRFlow-owned, exact-byte cache below `RrdEngine` and
below the future rrflowQL/DataFusion provider. The production default should
change from admit-every-miss exact LRU to a configurable scan-resistant LRU:
new pages enter a probationary region, reuse by a later engine operation
promotes them into a protected region, and eviction drains probationary pages
before protected pages. Repeated touches inside one projected stream refresh
probationary recency but do not prove reuse. The stream receives an opaque,
process-local scope generated inside the engine; clients neither provide nor
observe it. Exact LRU remains selectable only as a differential and operator
policy. The implementation must expose exact residency, admission, promotion,
same-scope suppression, demotion, eviction, load, and duplicate-load counters.
It must not partition capacity by semantic family, disable caching for all
projected scans, import Moka, add TinyLFU frequency state, or claim concurrent
scalability in this package.

This is a physical read policy, not a second reasoning or query engine. A
future rrflowQL `TableProvider` will request bounded, read-stamped projections
from rrflowKV; the page cache remains underneath that source and cannot own
query results, authorization, reasoning state, or persistence.

## Question and acceptance boundary

The narrow question is: how should the current shared immutable-page cache
preserve repeatedly used graph, lexical, control, and vector metadata pages
when a broad Arrow-compatible scan touches a larger one-use working set?

The answer is acceptable only if one real rrflowKV corpus proves all of the
following:

1. exact LRU and scan-resistant LRU return byte-identical values and projected
   rows at one `Snapshot`;
2. both policies remain at or below the configured byte capacity;
3. a repeated mixed-family hot set is promoted and survives a larger scan
   under the selected policy;
4. the selected policy performs fewer post-scan physical page loads than exact
   LRU on the declared corpus;
5. every counter is derived from the production reader, not a trace-only
   simulator; and
6. the artifact discloses that one host and one corpus are not release,
   concurrency, scale, or competitor evidence.

## Current implementation audit

At the baseline, `crates/persistence/rrd-lsm/src/segment.rs` owns one
process-local cache shared by all segments in a `Database`:

- the key is `(segment_generation_id, page_ordinal)`;
- the resident value is one `Arc<LoadedPage>` containing a raw borrowed Arrow
  buffer or an owned decoded Arrow buffer;
- charge is `Buffer::capacity().max(1)`, not logical row count or stored
  compressed length;
- one `Mutex<PageCache>` protects a `HashMap` and a lazy-invalidated min-heap;
- every miss loads outside the mutex, then every load that fits is admitted;
- eviction is strict least-recently-used across all pages;
- concurrent misses can perform duplicate reads before the second insertion
  observes the first; and
- cumulative evidence reports hits, misses, evictions, loads, physical reads,
  decoded/borrowed/allocated/copied/decompressed bytes, and filter probes.

The byte accounting and immutable ownership are sound inputs. Admit-every-miss
LRU is the specific weakness: a sequential projected scan whose working set is
larger than the cache can evict pages that point and graph navigation repeatedly
reuse. The existing feature-gated laboratory models a fixed 80/20 segmented
policy and records a better post-scan hot hit count, but that simulator neither
executes the real reader nor proves configuration, variable-sized pages,
concurrent behavior, or end-to-end semantic identity.

The first real-reader C-06j run proved that unconditional second-hit promotion
was still wrong. One key/value projected stream generated 601 cache hits while
walking its row groups because validity, offset, and data pages are requested
again as rows are prepared and copied. Those same-stream hits promoted scan
pages and both exact LRU and the naive two-region implementation performed 45
post-scan hot loads. The workload and strict comparison are retained. RRFlow
must distinguish reuse across engine operations from repeated physical access
inside one operation; reducing row-group size or weakening the assertion would
only hide the defect.

The current layout already sorts the canonical application key families.
Therefore a family-specific cache quota would duplicate semantic knowledge in
the physical layer and can strand capacity. The experiment must mix audit,
incoming/outgoing edge, record, runtime, scalar, term, and vector-shaped keys,
but the replacement policy must remain oblivious to those names.

## Primary-source findings

### Database block caches distinguish scan traffic from reusable traffic

RocksDB documents a shared uncompressed block cache, explicit capacity and
hit/miss/add evidence, and sharding to reduce lock contention. It also warns
that pinned blocks and non-strict insertion can exceed nominal capacity. RRFlow
retains shared immutable decoded pages and strict byte accounting, but does not
adopt RocksDB's C++ cache API, sharding defaults, compressed secondary cache,
or relaxed capacity behavior. [RocksDB block-cache documentation](https://github.com/facebook/rocksdb/wiki/Block-Cache)

RocksDB explicitly supports `fill_cache = false` for bulk iterators so a scan
does not displace cached contents. RRFlow adapts the underlying distinction
without disabling admission: pages touched by one projected scope remain
probationary, while a later scope can promote and reuse them. Its key-layout
guidance also recommends
placing metadata and bulky content in different key regions when their access
patterns differ. RRFlow retains the principle that scan traffic must not erase
the hot set and relies on its canonical ordered family prefixes for locality.
It does not expose a caller-controlled bypass in C-06j: projected pages may be
reused by later analytical queries, and an ingress client must not select
physical cache authority. [RocksDB basic operations](https://github.com/facebook/rocksdb/wiki/Basic-Operations/8b0db11192422ae154253ae6e76123f28b09488a)

RocksDB's trace simulator records caller, column family, hit/miss, block size,
and whether a miss may insert. It includes midpoint LRU and a ghost admission
policy that admits on second access. RRFlow retains trace-replay comparison and
the second-access principle, but requires the decision to pass through the real
v6 reader before adoption. It does not copy a RocksDB trace format or ghost
cache. [RocksDB cache analysis and simulation](https://github.com/facebook/rocksdb/wiki/Block-cache-analysis-and-simulation-tools)

InnoDB uses a two-region LRU so newly read pages start in an old region and
pages reused after admission become young. Its time threshold prevents repeated
touches caused by one scan from immediately promoting a page. RRFlow retains
that semantic distinction but replaces wall-clock delay with an exact logical
projected-stream scope, making the verdict deterministic. Its documentation
specifically targets mixed OLTP plus batch-scan workloads and requires workload
benchmarking before changing policy. RRFlow does not import MySQL's time
window, page-size assumptions, global configuration, or fixed 3/8 default.
[InnoDB scan-resistant buffer pool](https://dev.mysql.com/doc/refman/8.0/en/innodb-performance-midpoint_insertion.html)

### TinyLFU is relevant, but it is not justified by the current evidence

TinyLFU compares a candidate's estimated recent frequency with an eviction
candidate using compact approximate history. W-TinyLFU combines admission with
a recency window and reports strong results across several trace classes, but
its own evaluation shows workload-dependent window sizing and a weak result on
one OLTP trace. RRFlow retains TinyLFU as a later measured candidate if real
traces show two-region LRU is insufficient; C-06j does not introduce a count-min
sketch, aging schedule, or approximate victim comparison. [TinyLFU paper](https://arxiv.org/abs/1512.00727)

Moka offers concurrent Rust caches with TinyLFU admission and weighted
capacity, but documents best-effort bounding and eventually consistent policy
metadata. Those are reasonable application-cache tradeoffs, not drop-in proof
for RRFlow's exact physical byte ledger and phase evidence. Adding Moka would
also introduce a second policy implementation before RRFlow has measured its
own real page trace. No Moka dependency is selected. [Moka crate documentation](https://docs.rs/moka/latest/moka/)

### DataFusion's cache is a different boundary

DataFusion's `TableProvider` supplies Arrow `RecordBatch` sources and receives
projection, filter, and limit pushdowns; its returned execution plan is
responsible for streaming and parallel scan execution. RRFlow therefore needs
rrflowKV page reuse below the provider rather than an eager `MemTable` copy or
a DataFusion-owned persistence cache. [DataFusion `TableProvider`](https://docs.rs/datafusion/latest/datafusion/datasource/trait.TableProvider.html)

DataFusion's `RuntimeEnv` separately owns query memory, temporary disk, and
session cache management. Those budgets will be composed at F-01/F-05. They do
not replace or absorb rrflowKV's immutable decoded-page cache, and C-06j must
not add a DataFusion object to the persistence crate. [DataFusion `RuntimeEnv`](https://docs.rs/datafusion/latest/datafusion/execution/runtime_env/struct.RuntimeEnv.html)

## Alternatives considered

| Alternative | Decision | Reason |
|---|---|---|
| Keep admit-every-miss exact LRU as the only policy | Reject as default; retain as oracle | It has exact accounting but is vulnerable to one-pass scan pollution. |
| Promote on every probationary hit | Reject after real-reader failure | One projected stream legitimately rereads pages while preparing and copying rows; 601 same-stream hits polluted protection and produced the same 45 hot reloads as exact LRU. |
| Disable admission for every projected scan | Reject as default | It protects point-read pages but prevents repeated analytical projections from earning residency and lets a caller select physical behavior. |
| Partition cache capacity by record/edge/term/vector family | Reject | It embeds semantic family knowledge below the typed key boundary and can strand capacity; mixed-family evidence should drive a family-neutral policy. |
| Import Moka/TinyLFU now | Reject for C-06j | Approximate frequency, best-effort capacity, and eventually consistent policy evidence add unproved state and weaken exact accounting. |
| Admit only after a ghost-key second hit | Defer | It is scan resistant but deliberately performs a second physical load and needs separately budgeted ghost metadata. |
| RRFlow exact-byte probationary/protected LRU | Select | It directly adapts the already advanced laboratory candidate, keeps exact ownership, lets reuse earn protection, and can be compared against the existing policy through the real reader. |
| Shard locks and coalesce concurrent loads | Defer to C-07 | The current package can count duplicate loads, but concurrency, pinned owners, cancellation, and fairness need the C-07 reader/maintenance stress matrix. |

## Selected RRFlow policy

`PageCachePolicy` has two closed variants:

- `ExactLru`; and
- `ScanResistantLru { protected_capacity_basis_points }`.

The default protected target is 8,000 basis points because that is the exact
candidate already screened by the repository laboratory. It is configuration,
not durable segment metadata, and must validate strictly between zero and the
full cache capacity. A later ratio change requires new evidence; it is not a
version marker.

The state transitions are:

```text
miss -> load -> admission with the current internal reuse scope
same projected-scope probationary hit -> probationary recency refresh
later projected-scope or ordinary-operation probationary hit -> protected promotion
protected hit -> protected recency refresh
protected bytes above target -> least-recent protected demotion
capacity pressure -> probationary eviction, then protected eviction only if needed
oversize page -> serve without admission
concurrent losing load -> serve the already admitted immutable page and count duplicate load
```

One projected stream receives one non-zero scope allocated by the engine and
shared across its segment cursors. Scope exhaustion fails creation rather than
reusing an identifier. The scope is never serialized, accepted over a public
request, used for authorization, or emitted as a trace identifier.

All arithmetic remains checked or saturating evidence arithmetic as already
specified by the cache contract. Residency never exceeds `capacity_bytes`.
The cache never persists data, changes a page's digest identity, changes MVCC
visibility, or keeps a segment alive outside existing `Arc` ownership.

## Real mixed-family workload

The integrated comparison uses one persisted v6 corpus with the eight current
physical families. It reopens the same immutable state independently under
both policies, then performs:

1. one present-key warm pass across every family;
2. a second logical operation that proves cross-operation reuse and earns
   protection;
3. one complete bounded key/value projected stream larger than the configured
   cache;
4. the identical present-key pass after the scan; and
5. an exact digest comparison over returned rows and hot values.

The retained artifact must report per policy: capacity and region residency,
entries, hits, misses, admissions, rejected admissions, promotions, demotions,
evictions, loads, duplicate loads, bytes read/decoded/decompressed, projected
row digest, and post-scan hot loads. The selected policy advances only if its
post-scan hot loads are lower than exact LRU while all identities and bounds
are equal.

## Relationship to context flow

This package improves the storage half of the future context path without
pretending the complete path exists:

```text
authorized request
  -> RrdEngine read stamp and bounded physical request
  -> rrflowKV membership/range pruning and immutable page cache
  -> Arrow-compatible projected batches
  -> future rrflowQL/DataFusion scan, filter, rank, and RRF
  -> RrdEngine evidence/context packet
```

The cache knows page identity and reuse only. It does not know prompts,
providers, skills, reasoning-tree decisions, BM25 scores, vector scores, RRF
weights, or Connectome views. That separation is what lets the same cache serve
fast graph navigation and analytical Arrow scans without creating another
engine.

## Remaining risk after C-06j

- The global mutex is not a scalability claim; C-07 must measure lock
  contention and decide sharding.
- Duplicate concurrent loads are counted, not coalesced; C-07 must prove a
  safe in-flight owner before changing this.
- Logical reuse scope prevents one stream from promoting its own scan pages;
  C-07 must still prove scope allocation, cancellation, and concurrent-stream
  behavior under sustained stress.
- The Linux page cache may duplicate stored compressed bytes when buffered I/O
  is selected; direct-I/O and tier decisions remain separately measured.
- A one-host deterministic corpus cannot establish universal hit ratio,
  latency, scale, or superiority.
- DataFusion memory-pool, spill, query-cache, and provider behavior remain F-01
  through F-05 work.
- Value separation remains rejected from the current byte-only model because
  it lacks atomic pointer publication, snapshot, recovery, corruption,
  garbage-collection, and range-read proof.

## Observed integration result

The production implementation is bound to clean revision
`5b1c31de73cabf79fe3353112635a09f6a12e364` and source tree
`d99c09eadf567f55aaf3f41ea1614c16a697c1d4`. The retained artifact is
[`c06j-rrflowkv-scan-resistant-cache-linux-x86_64.json`](../evidence/c06j-rrflowkv-scan-resistant-cache-linux-x86_64.json)
with SHA-256
`8a008ee33bb50ca197783227cfcfbb4d58945dd2f26dca8cdf12e1804106b9ad`.
Across three isolated release-profile children:

- exact LRU performed 48 post-scan hot-page loads;
- scope-aware scan-resistant LRU performed zero;
- the selected policy recorded 18,200 same-scope suppressions, 48 promotions,
  and 48 protected entries;
- both policies stayed within the 1,048,576-byte capacity; and
- manifest, semantic, hot-value, and projected-row identities remained exact.

All 114 all-target/all-feature `rrd-lsm` tests, strict affected-package Clippy,
the complete `rrd-store` suite, 28 workspace-architecture guards, and the
locked workspace all-target check passed for the runtime change. The artifact
is one-host physical-policy integration evidence; it is not release,
concurrency, scale, latency, DataFusion, reasoning/recall, or competitor proof.

## Implementation handoff

The completed source/test/evidence edits are recorded by
[`c06j-rrflowkv-scan-resistant-cache-engineering-plan.md`](../roadmap/c06j-rrflowkv-scan-resistant-cache-engineering-plan.md).
The canonical [RRFlow 1.0 roadmap](../roadmap/rrflow-1.0.md) accepts C-06 from
the complete C-06a-through-C-06j evidence corpus; this supporting record does
not change that status independently.
