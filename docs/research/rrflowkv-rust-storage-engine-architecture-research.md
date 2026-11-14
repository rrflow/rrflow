# rrflowKV Rust storage-engine architecture research

**Status:** active draft supporting research for owner review; not an ADR, roadmap,
completion claim, or authorization to rewrite engine code
**Coordinate:** `rrflow://rrflow-instance/data/research/rrflowkv-rust-storage-engine`
**Owner:** primary-source evidence for C-06, C-07, F-01 through F-05, and the
later governed configuration workflow
**Audience:** RRFlow owner and engineers implementing the 1.0 pre-release
**Researched:** 2026-09-12 through 2026-09-13
**Latest reviewed repository baseline:** `5b1c31de73cabf79fe3353112635a09f6a12e364`
**Implementation update:** C-06g adds the bounded pinned projected reader;
C-06h adds finite adversarial qualification; segment v6 integrates the C-06i
authenticated persisted row-group filters and adaptive-LZ4 page policy. The
clean integration artifacts verify the normal writer/reopen/read path. C-06j
adds the production scope-aware scan-resistant page cache and real persisted
mixed-family comparison. The canonical roadmap accepts C-06; C-07 and F retain
installed-path lifetime/recovery/concurrency and streamed DataFusion work.
**Assumptions:** RRFlow remains one independently installable Rust product;
`RrdEngine` remains the sole authorization and transaction authority;
rrflowKV is the persistent substrate; rrflowMX is the volatile conformance
profile; rrflowQL owns native and DataFusion query execution; external
databases and providers remain optional adapters

## Direct finding

Yes: RRFlow can build a Rust-native storage engine that reaches RocksDB-class
quality for its declared workload. The current repository is not starting from
zero. It already contains a checksummed WAL, atomic batches, MVCC sequence
visibility, write-conflict detection, manifest/CURRENT publication,
checksummed immutable segment-v6 files, raw/adaptive-LZ4 Arrow-compatible
column pages, authenticated persisted row-group filters,
snapshots, a bounded pinned projected stream, compaction, garbage collection,
snapshot bundles, page caching, multiple I/O modes, phase-scoped physical
counters, deterministic differential tests, and real process-kill recovery
tests.

It is not yet defensible to call the implementation equivalent to RocksDB.
RocksDB's production standard includes much more than an LSM data structure:
continuous randomized crash testing, fuzzing, inspection and repair tools,
background flush and compaction scheduling, explicit write stalls and pending
compaction accounting, pinned live-file generations, extensive per-operation
statistics, and reproducible workload benchmarks.[^1] [^2] [^3] The current
`rrd-lsm` implementation lacks several of those operational systems and the
sustained evidence needed to qualify them.

The correct engineering decision is therefore:

1. preserve and harden the current RRFlow-native WAL/MVCC/manifest/segment
   primitives;
2. retain C-06g's ordered projected reader and owned read generations, then
   attack them with C-06h adversarial/property/fuzz/fault qualification before
   connecting DataFusion to storage;
3. establish a walking `rrflow`/`rrflow.exe` install/start/persist/reopen/verify
   product immediately after C-06, as the canonical roadmap requires;
4. qualify maintenance, recovery, storage-full behavior, and lifetime safety
   through that installed path in C-07;
5. add graph, BM25, exact-vector, HNSW/TurboQuant, and RRF as native,
   transactionally maintained access paths over the same canonical data;
6. let DataFusion consume bounded Arrow batches from rrflowKV, never own a WAL,
   transaction, authorization decision, or commit; and
7. build discussion-to-configuration as a governed proposal/preview/apply
   path through `RrdEngine`, not as an editor hook, model-authored command, or
   parallel workflow runtime.

This is feasible, but “better than RocksDB” must mean better on a declared
RRFlow workload after correctness parity—not better at every possible KV
workload by assertion.

## One engine, five responsibilities

The names describe responsibilities inside one product. They are not five
databases and should not become five independently authoritative runtimes.

```text
HTTP / WebSocket / SDK / CLI / MCP
                |
                v
          RrdEngine
  auth -> policy -> read stamp -> plan -> commit receipt
       /              |                 \
      v               v                  v
 rrflowMX         rrflowQL          native access paths
 volatile         planner +         graph / scalar / BM25 /
 profile          DataFusion        exact + ANN / RRF
      \               |                  /
       \              v                 /
        +-------- bounded Arrow batches-+
                       |
                       v
                   rrflowKV
       WAL -> MVCC memtables -> immutable hybrid tables
             -> manifest/version set -> compaction
```

| Boundary | Owns | Must not own |
|---|---|---|
| `RrdEngine` | authentication, authorization, semantic validation, read stamps, mutation plans, compare-and-swap, commit acknowledgement, reasoning/routine state transitions | raw file layouts, DataFusion operator internals, provider-specific lifecycle |
| rrflowKV / `rrd-lsm` | ordered bytes, WAL, MVCC, immutable tables, versions, snapshots, compaction, recovery, checksums, physical budgets and evidence | documents, graph semantics, model policy, HTTP, MCP, DataFusion |
| `rrd-store` | typed RRFlow key families, transactional semantic repositories, projection source truth, storage-profile conformance | independent WAL, alternate persistent backend, query planning |
| rrflowMX | non-durable implementation of the same semantic storage behavior used for fast active state and conformance | canonical durable truth after restart |
| rrflowQL / `rrd-query` | parsing, binding, logical/physical planning, native operators, DataFusion execution, pushdown contracts, query budgets | direct durable mutation or authorization |

“DataFusion as a subprocess” should mean a subordinate execution subsystem
with its own bounded task and blocking-I/O pools, not a separate OS daemon. A
separate process would add IPC and buffer-lifetime complexity and would work
against the local zero-copy goal. DataFusion explicitly separates lightweight
planning, lightweight execution construction, and work performed while a
`SendableRecordBatchStream` is polled.[^4] RRFlow should conform to that model
inside the primary process while retaining an explicit scheduler boundary.

## What RocksDB-class quality actually requires

The comparison target is a qualification envelope, not a feature-count table.
Every row below requires reproducible evidence against a named revision,
filesystem, device, operating system, configuration, workload, and seed.

| Qualification dimension | Minimum RRFlow proof | Current state |
|---|---|---|
| acknowledged-write durability | no acknowledged mutation is lost after process kill, power-loss model, partial/torn write, directory-sync failure, or reopen | useful fixed failure matrices and SIGKILL tests exist; continuous randomized fault coverage does not |
| atomicity and isolation | exact independent oracle across batches, conflicts, snapshots, deletes, flush, compaction, reopen, and cancellation | strong targeted MVCC/batch tests exist; concurrency state space and sustained randomized histories remain incomplete |
| metadata recovery | CURRENT, manifest, WAL inventory, immutable files, and orphan cleanup form one validated closure; create, open, inspect, verify, repair, and salvage have distinct authority | manifest/CURRENT and checkpoints exist; complete WAL/version inventory, mutation-free inspection, repair/salvage, and storage-full closure remain incomplete |
| reader lifetime | each reader owns one immutable physical generation until release on Linux, Windows, and macOS; GC cannot unlink its closure early | C-06g owns sequence/manifest/memtable/segment generations and a GC lease, with one flush/compaction/GC test; cross-platform installed-path lifetime and fault qualification remain C-07 work |
| sustained maintenance | background flush and compaction keep up or apply explicit measured backpressure; memory and pending work stay bounded | compaction is synchronous and policy-light; no background scheduler, debt model, or admission controller exists |
| streaming reads | ordered k-way merge applies snapshot visibility and tombstones without materializing the result; projection avoids unneeded pages | C-06g supplies a synchronous bounded projected k-way stream and omits value pages for keys-only reads; asynchronous DataFusion adaptation and remaining broad callers are open |
| read/write/space amplification | physical reads, writes, compaction bytes, cache work, and live/dead space are attributed per operation and over time | C-06g scopes projected query work and separates segment-open/startup reconciliation; write/compaction/live-dead amplification ledgers remain incomplete |
| cache behavior | capacity, admission, pinning, scan resistance, hit/miss/load, duplicate-load, and eviction behavior are measured | accepted C-06j provides exact and default scope-aware scan-resistant policies, strict byte/region accounting, a real mixed-family pollution differential, and duplicate-load counters; lock contention, coalescing, and sustained concurrency remain C-07 work |
| observability | stable low-cardinality metrics, scoped traces, status/inspection output, stall causes, compaction debt, and redacted capture bundles | useful tracing and counters exist across crates; no complete storage status surface or end-to-end causal proof exists |
| test maturity | unit/property/model tests plus continuous crash stress, fuzz, deterministic fault injection, sanitizer/Miri lanes, and fixed-hardware regression | targeted tests, the C-06g projected MVCC/bounds/generation cases, and C-06h's bounded model/segment fuzz targets are substantial; no continuous `db_stress`-class fault matrix or cross-platform sanitizer qualification exists |
| operator tooling | offline/online verify, manifest/table/WAL inspection, backup/restore, safe repair policy, and reproducible benchmark binaries | snapshot/archive pieces and diagnostic examples exist; primary-binary inspection/verify/repair is not complete |

RocksDB documents explicit write slowdown and stop conditions based on immutable
memtables, L0 file count, and pending compaction bytes because an engine that
accepts writes faster than maintenance can absorb them will increase read and
space amplification until it fails.[^3] RRFlow needs the same *behavioral
property* with RRFlow-owned types and metrics; copying RocksDB's option surface
would be the wrong goal.

RocksDB also distinguishes ordinary unit tests from `db_stress`: its crash
tests continuously randomize operations and options, kill and reopen the
database, validate against independent state, and run under multiple sanitizer
builds.[^2] FoundationDB demonstrates the stronger testing principle: seeded,
deterministic simulation of failures must be complemented by live performance
and real hardware failure testing.[^5] RRFlow should adopt that testing ladder
without pretending a Rust port of either test harness already exists. Its
Rust-native lanes should combine coverage-guided fuzzing, small-state
concurrency permutation testing, and deterministic fail points where each tool
fits; `cargo-fuzz`, Loom, and `fail-rs` are reference mechanisms rather than a
preselected implementation stack.[^21] [^22]

## Current implementation audit

This audit began against the complete baseline `rrd-lsm` source and its owning
tests, plus the rrflowKV store bridge and current rrflowQL Arrow/DataFusion
path. The limitation table below is updated for the C-06g candidate; remaining
code is implementation inventory until its owning roadmap evidence passes.

### Sound primitives to retain

| Current path | Useful behavior that must survive convergence |
|---|---|
| `crates/persistence/rrd-lsm/src/wal.rs` | framed WAL records with checksums, exact batch recovery, durability modes, poisoned-writer behavior after uncertain append |
| `crates/persistence/rrd-lsm/src/batch.rs` | atomic ordered write-batch encoding and frozen format fixtures |
| `crates/persistence/rrd-lsm/src/memtable.rs` | sequence-version chains, tombstones, visible-version selection, sorted keys |
| `crates/persistence/rrd-lsm/src/transaction.rs` | snapshot sequence capture and write/write conflict validation |
| `crates/persistence/rrd-lsm/src/manifest.rs` | content-addressed manifests, checksummed CURRENT pointer, publication ordering, checkpoints and reachability |
| `crates/persistence/rrd-lsm/src/segment/format.rs` | authenticated segment-v6 metadata, row groups, persisted membership filters, none/adaptive-LZ4 policy, raw/LZ4 page descriptors, 64-byte alignment, and format/schema/key/page identities |
| `crates/persistence/rrd-lsm/src/segment.rs` | mmap/bounded/io_uring page sources, checksum-before-decode page acquisition, bounded aligned LZ4 reconstruction, decoded-byte page cache, table reads, evidence, and frozen bytes |
| `crates/persistence/rrd-lsm/src/database.rs` | single-writer composition, write preparation, flush/publication ordering, compaction, snapshot install, GC |
| `crates/persistence/rrd-lsm/src/snapshot_bundle.rs` | portable checksummed snapshot closure and restore validation |
| `crates/persistence/rrd-store/src/rrflow_kv.rs` | RRFlow key-format binding, startup checkpoint reconciliation, storage-profile integration |
| `crates/persistence/rrd-store/tests/rrflow_kv_model_soak.rs` | deterministic exact-model comparison across substantial mutation histories |
| `crates/persistence/rrd-store/tests/durability.rs` | real child-process termination and reopen evidence |

### Gaps that prevent a production claim

| Current code | Observed limitation | Canonical correction |
|---|---|---|
| `Database::{scan,scan_ranges,scan_each}` | older semantic callers can still assemble complete maps/vectors; C-06g adds `begin_projected_read` without silently changing those call sites | migrate only F-01 and later bounded native callers to the accepted projected stream, then remove broad normal-execution use under their owning tests |
| `Segment::{visible_from,visible_ranges}` and `SegmentRecordCursor` | pre-projected broad internal readers still load complete value columns; C-06g's crate-private projected cursor is key-spine-first and late-loads winning validity/value pages | preserve projected semantics through C-06h; remove remaining broad normal-execution use directly when F/E consumers migrate, with no compatibility lane |
| `Snapshot { sequence }` | the public logical token alone owns no physical files; C-06g pairs it with a crate-private `ReadView` that owns the captured manifest, `Arc` memtable/segments, and GC lease | carry that owned view through F-01's provider and C-07's installed cross-platform lifetime qualification |
| `Database::prepare_write` | flush and maintenance can run synchronously on the writer | C-07 background flush/compaction scheduler, immutable memtable queue, explicit admission/backpressure |
| `Database::compact_inner` | segment-count selection, whole-job merging, no debt/rate/subcompaction policy | score/overlap/byte-aware picker, bounded jobs, cancellation, rate limits, metrics; specialize only from benchmarks |
| `PageCache` | one global mutex, selectable exact LRU, default scope-aware probationary/protected LRU, exact capacity/region counters, and classified duplicate completed loads; no in-flight coalescing | C-07 measures contention and proves any sharding/coalescing change; preserve the accepted scan-resistance oracle and exact accounting |
| `IoContext` | one mutex around one io_uring instance and synchronous calls; C-06g reports the backend that actually served a read after fallback | later compare bounded per-device queues/pools and cancellation based on measurements |
| `ManifestStore::load` | full immutable-file digest verification on normal open can scale with total bytes; WAL closure is simpler than mature version tracking | separate fast authenticated metadata open from full verify; track complete version/WAL closure and retain full verification as an explicit operation |
| WAL lifecycle | one straightforward log path; no group commit, fragmentation policy, log inventory/recycling, or retained-log accounting | preserve correctness first; add version-bound WAL inventory and benchmarked group commit only after installed lifecycle works |
| immutable table policy | persisted row-group filters, adaptive LZ4, and scope-aware scan-resistant cache admission are integrated; stationary values and family-neutral capacity are the accepted C-06 choices | retain value separation and semantic-family partitioning as rejected for this format unless a new package supplies complete recovery/GC/snapshot and comparative evidence |
| operator surface | no primary-binary table/WAL/manifest inspect and full verify/repair flow | D-01 creates read-only inspect/verify; C-07/D-11 close full verification and recovery policy |

The current `Database` file is too broad for long-term ownership, but splitting
it immediately would hide behavioral changes in file motion. First establish
the owned reader and failure tests against the current symbols. C-07 can then
separate version management, maintenance, and recovery in direct moves while
the tests keep behavior visible.

## Rust-native reference systems

The references below are inputs, not dependencies or architectural authority.

| Reference | What it proves or teaches | What RRFlow must not infer |
|---|---|---|
| RocksDB | mature LSM lifecycle: WAL/version metadata, live-file pinning, iterators, compaction, backpressure, checksums, diagnostics, stress, fuzz, benchmarks and tools[^1] [^2] [^3] [^6] | that C++ APIs/options/formats should be copied, or that feature presence proves RRFlow correctness |
| Fjall | a serious safe-Rust embedded LSM can provide ordered forward/reverse scans, keyspaces, cross-keyspace atomicity, compression, optional value separation, automatic maintenance, snapshots and serializable transaction modes[^7] | that embedding Fjall would make rrflowKV RRFlow-owned, or that Fjall's defaults match RRFlow's graph/index/Arrow workloads |
| `lsm-tree` | block tables, filters, caching, MVCC, leveled/FIFO compaction and optional value separation can be cleanly implemented in stable Rust; it explicitly is only an LSM primitive and lacks a WAL[^8] | that an LSM crate alone is a database or recovery system |
| SurrealKV | a Rust product-specific LSM can be developed to remove RocksDB dependency and can combine MVCC, durability modes, time travel, checkpoint/restore, leveled compaction and optional value logging[^9] | that its ACID/performance claims are RRFlow evidence, or that its public/storage model should be renamed into RRFlow |
| SlateDB | a Rust LSM can place immutable data and manifests on object storage, trading local latency/API cost for durability and scale while using caches, filters and compression[^10] | that remote object storage belongs in the alpha hot path; it is a later cold-tier reference |
| Lance | page-level columnar layout, independent column reads, random access, explicit metadata and aligned buffers are useful immutable-format precedents[^11] | that Lance supplies RRFlow's WAL, MVCC, semantic transaction authority, graph indexes, or reasoning lifecycle |
| Arrow | a standardized columnar memory layout supports locality, SIMD and eligible zero-copy sharing when layout and lifetime constraints are satisfied[^12] | that every mmap is an Arrow array, that variable-length/decompressed data is zero-copy, or that Arrow is persistence |
| DataFusion | extensible logical/physical planning, projection/filter/limit pushdown, polled batch streams, metrics and bounded execution[^4] | that DataFusion should own storage writes or that wrapping an eager `Vec<QueryRow>` is streaming |
| Moka | a concurrent, best-effort bounded in-memory hash cache with TinyLFU/LRU-style admission/eviction and request coalescing[^13] | that DataFusion requires Moka or that an eventually consistent cache policy may own storage lifetime/correctness |

### Moka decision

Moka is not required by Arrow or DataFusion, and it remains absent from the
workspace. C-06j implements a purpose-built exact-byte, scope-aware scan-
resistant cache because storage pages require deterministic charging,
generation ownership, physical-read attribution, and scan behavior. The real
reader improved the declared post-scan load oracle without Moka, so no Moka
prototype or dependency is justified for the physical cache. A future higher-
level immutable plan/context cache may reconsider it only under its own stamp,
capacity, staleness, latency, and operation-accounting evidence.

## Target rrflowKV physical design

### Mutable path

```text
authorized semantic mutation plan
  -> ordered physical WriteBatch
  -> writer admission and sequence reservation
  -> append/checksum WAL frame
  -> requested durability barrier
  -> publish into mutable MVCC memtable
  -> commit receipt / uncertain outcome handling
```

The alpha may retain a serialized writer because correctness and a usable
binary are more important than premature multi-writer machinery. Group commit
is a later measurable optimization. The contract must already distinguish
buffered acceptance, authoritative durability, visible state, acknowledgement,
and uncertain failure.

### Immutable path

```text
mutable memtable reaches policy limit
  -> rotate to immutable memtable
  -> background flush job
  -> sorted key/version spine
  -> row groups with independent Arrow-layout pages
  -> per-page checksums + format/schema/statistics metadata
  -> fsync file and directory as required
  -> publish version edit / CURRENT
  -> retire WAL only after the new closure is durable
```

The ordered spine remains the source for point, prefix, range, graph adjacency,
and index-key navigation. Arrow-layout pages are immutable scan projections,
not a replacement for the ordering structure. This hybrid is where RRFlow can
differentiate: the same durable table can serve low-latency ordered access and
columnar analytical projection without exporting the estate to another
database.

### Read path

```text
RrdEngine binds ReadStamp
  -> rrflowKV captures ReadView
       sequence + version/manifest + memtable generations + table handles
  -> source-level pruning by typed key range and authenticated statistics
  -> one cursor per eligible run
  -> heap merge by key, newest visible sequence wins
  -> tombstone suppression
  -> requested pages only
  -> bounded Arrow-compatible batches
  -> native operator or rrflowQL/DataFusion consumer
  -> release ReadView and make retired files GC-eligible
```

For C-06g, the stream may own/copy merged output buffers. “Zero-copy” is only
reported for a buffer whose Arrow array can retain the exact mapped owner and
whose encoding needs no decode, decompression, offset rewrite, or merge copy.
The evidence must report borrowed, decoded, decompressed, allocated, and copied
bytes separately.

### Version and maintenance path

The C-07 target should introduce internal responsibilities equivalent to these
industry terms, while retaining RRFlow types and public boundaries:

- `VersionSet`: current and retained immutable file closures plus WAL inventory;
- `ReadView`: one logical stamp and owned physical generation;
- `MemtableSet`: mutable and immutable flush queue with byte accounting;
- `FlushScheduler`: bounded jobs that turn immutable memtables into tables;
- `CompactionPicker`: score, overlap, live/dead bytes, family, and device-aware
  selection;
- `CompactionJob`: cancellable, rate-limited merge with checksum handoff;
- `ObsoleteFileCollector`: deletes only files absent from current, checkpoint,
  snapshot, backup, and active-read closures;
- `StorageStatus`: low-cardinality health, debt, stalls, amplification, cache,
  I/O, and error state; and
- `Verifier`: mutation-free quick/full inspection with an explicit separate
  repair/salvage authorization path.

These are internal responsibilities, not an instruction to create nine crates
or move code before the behavior is characterized.

## Native graph, lexical, vector, and DataFusion placement

All derived access paths must be reconstructible from canonical source records,
but their accepted generation heads and deltas are committed atomically with
the semantic mutation that invalidates or advances them.

| Access path | Canonical physical shape | Query behavior |
|---|---|---|
| record/document | typed ordered current/history keys plus immutable projected columns | point/range natively; broad projection may stream to DataFusion |
| temporal graph | direction-specific typed adjacency keys plus edge record/history | prefix/range traversal at one stamp; no graph-wide reconstruction |
| scalar/unique | value-ordered typed keys with record identity and generation | exact/range candidates with uniqueness checked in the transaction |
| BM25 | versioned analyzer identity, term dictionary, compressed posting blocks, deletion generation and statistics | native lexical scorer; exact corpus oracle; optional Arrow analytics over results |
| exact vector | canonical full-precision vectors and payload/filter truth | correctness oracle and rerank source |
| ANN/quantized | replaceable immutable HNSW/TurboQuant generation plus delta/deletion state | filtered candidates, exact rerank, recall evidence, stale/corrupt generation denial |
| RRF | pure configured fusion over stamped ranked lists | no independent persisted truth; feedback may propose a later authorized policy revision |

WiscKey demonstrates that separating large values from the sorted LSM can
reduce compaction I/O, but shifts cost into indirection and value-log garbage
collection.[^14] It is therefore a measured option for large raw text/vector
families, not an up-front universal rule. Likewise, LSM-VEC and TurboQuant are
promising experimental references for disk-resident ANN and extreme vector
compression, but they require independent reproduction and exact recall/error
oracles before RRFlow adopts their claims.[^15] [^16]

## Governed natural discussion to configuration

Natural language is evidence and intent, never executable configuration. A
model/provider adapter may translate a discussion into a proposal, but the
proposal cannot authorize itself, invent a storage key, execute a command, or
schedule a routine.

The required chain is:

```text
natural discussion
  -> provider-neutral ConfigurationProposal
  -> schema + semantic + capability validation
  -> deterministic ConfigurationPlan and exact diff
  -> preview of records, files, permissions, budgets and effects
  -> operator authorizes the exact plan digest
  -> RrdEngine compare-and-swap commit
  -> canonical configuration-changed EngineEvent
  -> bounded persisted Trigger evaluation
  -> idempotent RoutineActivation proposal
  -> leased/checkpointed RoutineRun
  -> optional skill/context resolution
  -> authorized operation or fenced external activity
  -> receipt, verification, trace and terminal status
```

This follows three sound industry patterns without importing their authority:

1. desired configuration and observed status have separate generations, so a
   stale observation cannot be mistaken for application of the current
   configuration; Kubernetes uses `generation`/`observedGeneration` for this
   distinction.[^17]
2. rules are schema-checked before activation; Cedar documents why validation
   against typed entity/action schemas prevents broad classes of runtime
   policy errors.[^18]
3. trigger predicates are deterministic, side-effect free, bounded, and
   non-Turing-complete; CEL demonstrates that such expressions can be typed and
   evaluated with predictable cost.[^19]

RRFlow should adapt those properties into a closed RRFlow-owned predicate AST
and binary/wire contract. It should not accept arbitrary shell, JavaScript,
SQL, provider prompt text, or dynamically fetched policy code as a trigger.
CloudEvents provides a useful interoperable event-envelope vocabulary—unique
identity, source, type, subject, schema and occurrence context—but RRFlow's
persisted event remains its own typed engine record tied to estate, stamp,
authorization, provenance and idempotency coordinates.[^20]

### Required typed records

| Record | Purpose | Authority |
|---|---|---|
| `ConfigurationRevision` | immutable desired state, base revision, schema identity, content digest, estate scope | persisted through `RrdEngine` |
| `ConfigurationProposal` | untrusted requested changes, rationale, evidence links, author/provider identity | cannot mutate |
| `ConfigurationPlan` | deterministic normalized diff, prerequisites, affected records/files, permissions, budgets, estimates, rollback/removal plan | preview only until exact digest authorization |
| `ConfigurationDecision` | operator/policy allow or deny bound to proposal/plan/revision digests | required input to apply |
| `ConfigurationStatus` | observed revision, conditions, last applied receipt and verification | derived from committed outcomes, never from a client event |
| `EngineEvent` | immutable committed occurrence with source, action, target, scope, stamp, idempotency and provenance | emitted within/after authoritative commit as defined by I-01 |
| `TriggerDefinition` | versioned bounded predicate over committed events plus an operation/routine-start proposal | no direct commit or external effect |
| `RoutineDefinition` | immutable resumable graph of authorized operations/activities with budgets and compensation | engine-owned definition |
| `RoutineRun` | durable activation, lease, step/checkpoint, attempts, receipts, cancellation and terminal state | advanced only by `RrdEngine` |
| `SkillPackage` | immutable instruction/resources with identity, digest, applicability and context budget | content resolved by a routine/request, never executable lifecycle code |
| `AdapterBinding` | explicit endpoint/tool/command capability, credentials reference, permissions, budgets and retirement/removal policy | inactive until exact authorization |

### Concrete first workflow example

Discussion:

> When the same Rust compiler diagnostic appears three times on the same
> project snapshot, propose the bounded debugging routine using the approved
> Rust-analysis skill. Do not use the network and stop after ten steps or sixty
> seconds.

The adapter should propose—not apply—a typed diff equivalent to:

```text
trigger diagnostic-repeat-v1
  event_type       = project.diagnostic.committed
  predicate        = same(snapshot_digest, diagnostic_fingerprint)
                     && count(window = 30m) >= 3
  max_evaluations  = 1_000
  proposes         = routine debugging-v1

routine debugging-v1
  skill            = rust-analysis@<digest>
  max_steps        = 10
  wall_time         = 60s
  network           = deny
  mutation          = separate approval
```

Preview must show the exact trigger/routine/skill revisions, event fields,
window semantics, resource budgets, permissions, records, installation-owned
files, and removal plan. Apply must reject changed base revision, changed
proposal/plan digest, missing skill, unsupported event field, unbounded window,
ambiguous identity, unauthorized capability, or a second activation with the
same idempotency key. This proves that conversation can become durable
configuration without letting the model become the workflow authority.

### Canonical code placement when its roadmap wave begins

This is target ownership, not authorization to create the files during C-06g:

| Boundary | Planned responsibility |
|---|---|
| `crates/transport/rrd-contract/src/configuration.rs` | closed provider-neutral proposal/plan/decision/status DTOs and canonical digest rules |
| `crates/transport/rrd-contract/src/automation.rs` | public event, trigger, routine, skill and adapter-binding DTOs after I-01 begins |
| `crates/persistence/rrd-store/src/keyspaces.rs` | typed ordered keys for revisions, heads, events, cursors, activations, runs, receipts and skill manifests |
| `crates/persistence/rrd-store/src/repository/configuration.rs` | transactional configuration repositories; no policy evaluation |
| `crates/persistence/rrd-store/src/repository/automation.rs` | transactional event/routine repositories; no scheduler or adapter calls |
| `crates/authority/rrd-engine/src/engine/configuration/` | validate, plan, preview, authorize, CAS apply, inspect and retire operations |
| `crates/authority/rrd-engine/src/engine/automation/` | event commit, bounded trigger evaluation, activation, leases, routine advancement and receipt acceptance |
| `crates/adapters/*` | stateless natural-language/provider/host translations and fenced external activities only |
| bundle-resident project templates | generic inactive configurations, rules, routines and skills installed through D-01/I-06 preview/apply; no runtime fetch |

The exact file plan must be regenerated from the codebase when those gates
start. Creating these modules now would produce another contract island before
storage, the primary binary, persisted attunement jobs, and canonical events
exist.

## C-06h observed qualification

C-06h implements the testing behaviors selected above without importing an
upstream storage or testing surface. One shared test-only state machine owns an
independent MVCC version history and decodes bounded operations across audit,
incoming/outgoing edge, record, runtime, scalar, term, and vector families.
Stable tests, a configurable stress executable, and a coverage-guided target
all consume that same oracle. A second fuzz target mutates the frozen
authenticated segment-v4 fixture and, when `Segment::open` accepts bytes,
requires a complete visible-version read to remain valid.

The model deliberately combines semantic-family batches, retained snapshots,
projected keys/key-value streams, reopen, flush, protected compaction, garbage
collection, pinned-view lifetime, cancellation, output/page limits, and all
ten injected write/flush/compaction boundary classes. Implementation output
never updates expected state. Every unexpected result panics with the seed,
operation, input digest, and exact input bytes.

The bounded Linux run completed 1,536 stress operations including 283 injected
failures and 382 reopens; 512 state-machine sanitizer executions; and 4,096
segment-v4 sanitizer executions. The complete 94-test `rrd-lsm` suite and
strict all-target Clippy passed. The repository version policy now recognizes
the nested fuzz project only through explicit cargo-fuzz metadata and requires
an unpublished `0.0.0` one-member workspace directly below a declared product
crate; seven direct denial probes passed without weakening product workspace
or frozen-version parity. These results qualify a reproducible finite candidate
corpus only. They do not cover real child-process termination, device failure,
ENOSPC, cross-platform mapped-buffer lifetime, continuous stress, or
fixed-hardware performance; C-07 and Gate J retain those claims.
Loom remains excluded because it cannot model the filesystem/mmap/io_uring
boundaries under test, and no Miri claim is made for code it did not execute.

## C-06i physical-policy candidate screen

The code audit established a sharper filter gap than the earlier shorthand
"no persisted filters." `Segment::open` already builds a correct process-local
ten-bit/seven-hash Bloom filter, but only after deep-reading every semantic
page. The normal manifest-owned `Database::open` path correctly avoids that
startup read and installs conservative `allow_all` filters. Point misses inside
a row-group key range therefore perform filter checks but cannot be rejected
after normal reopen. The C-06i baseline must measure those resulting key-spine
loads rather than crediting the standalone-open filter to the database.

The first C-06i slice is a candidate screen, not a format patch. A
default-disabled module beside the canonical segment encoder builds one
deterministic eight-family MVCC corpus, calls the real v4 encoder, and consumes
its parsed page descriptors. This avoids both an invented benchmark layout and
a second parser. Its isolated release-profile children compare:

- raw pages with adaptive LZ4 and Zstandard level 1, counting per-page framing,
  exact decode, selected pages, stored bytes, and encode/decode CPU;
- a serialized version of the current Bloom behavior, with exact member and
  absent-key probes per real row group;
- the current exact-byte LRU behavior with a purpose-built exact-byte
  segmented-LRU trace candidate under repeated-hot, scan, repeated-hot access;
- an explicitly non-retainable value-separation byte model; and
- real rrflowKV create, flush, process-state drop, normal reopen, missing-key,
  present-key, filter, cache, and physical-I/O counters.

The original codec screen kept both codecs behind the disabled
`physical-policy-lab` feature. Current segment v6 promotes only pinned safe,
checked `lz4_flex` into production; Zstandard remains lab-only. Moka is not
added: this slice can answer whether scan resistance is useful without
weakening exact accounting, and its simulator cannot claim integrated
concurrent-cache latency. Value separation remains rejected for
production regardless of modeled savings because it lacks authenticated
pointer publication, read-view/snapshot closure, crash recovery, corruption,
range-read, and value-log garbage-collection evidence.

One warm-up and every retained trial run in separate child processes. The
evidence binds seed/corpus, raw trials, clean revision/tree, Cargo.lock and
executable digests, compiler/target/profile/command, CPU/memory/kernel,
filesystem/mount/device, frequency policy, and current load. OS/device cache
and competing load are disclosed as uncontrolled. This is enough to select a
separately planned production experiment on the measured host; it is not
installed-product, release, cross-platform, or competitor evidence.

The clean `f7257fa` run resolved the screen without promoting a simulation.
Across 139 real row groups, the serialized Bloom candidate used 13,304 bytes,
had zero false negatives and 0.634% observed false positives, while normal
manifest reopen produced zero filter negatives and loaded 426 pages for the
same bounded miss workload. Persisted authenticated row-group filters are
therefore the first production experiment. Adaptive LZ4 and Zstandard both
cleared the byte threshold but remain placement experiments because their CPU
costs and hot/cold roles differ. Segmented LRU improved the modeled post-scan
hot set but still lacks integrated concurrency, generation, and lifetime
evidence. Value separation remains rejected because a byte model cannot answer
its recovery and garbage-collection obligations.

### Persisted-filter adaptation

The retained behavior was first adapted into RRFlow segment v5 rather than
copied from an upstream format and is retained in the sole current segment v6
reader and writer. One implementation in `segment/format.rs` owns the ten-bit/
seven-probe hash policy, sizing, encoding, parsing, and membership operation.
Each authenticated row-group index entry stores the unique-key count and exact
derived little-endian filter words. Normal manifest reopen parses those bounded
metadata bytes without reading semantic pages. Standalone admission and
snapshot validation rebuild the words from decoded unique keys and reject any
mismatch even when a mutated file has a recomputed outer checksum. Versions 1
through 5 remain direct rejection inputs; there is no compatibility reader or
migration lane.

The original fixed integration workload measured the v5 encoder and normal
rrflowKV reopen; the current adaptive-compression artifact repeats those filter
checks through v6. Both require nonzero persisted filter count/bytes, zero
semantic-page work during open, zero member false negatives, and fewer miss-
path page loads than filter checks while exact present, tombstone, snapshot,
flush, compaction, and reopen semantics remain unchanged. The production
filter does not participate in range exclusion or establish semantic presence.
Zstandard and value placement remain outside the integrated production policy;
the historical candidate result did not silently activate them. C-06j later
qualified the RRFlow-owned scope-aware scan-resistant cache through the real
reader rather than promoting this simulator.

### Adaptive page-compression decision

The selected production experiment was per-page adaptive LZ4 inside rrflowKV,
not compression in DataFusion and not a second columnar store. The implemented
decision is grounded in three different kinds of evidence:

1. RRFlow's fixed candidate corpus shows that both LZ4 and Zstandard produce
   substantial byte reduction and exact round trips, but LZ4 has the lower
   measured encode/decode CPU for the intended common path.
2. The LZ4 block specification requires the caller to supply compressed and
   decompressed sizes and a bounded destination.[^23] RRFlow's current 96-byte
   page descriptor already authenticates physical length, logical length, and
   a stored-page SHA-256 digest, so raw independent blocks fit without adopting
   another frame or size authority.
3. Arrow's physical format permits independent buffer compression and raw
   fallback, but a compressed buffer must be decoded before consumers can use
   the Arrow layout.[^12] Therefore only an eligible raw mmap page is a
   zero-copy result; a compressed page is explicitly an owned aligned decode.

The implemented target is a direct segment-v6 replacement. An authenticated
header record carries the writer policy, and each page descriptor carries
`none` or `lz4_block`. The default policy selects a block only when checked
integer arithmetic proves at least 1,250 basis points of saving; otherwise it
writes the original page. An authenticated configurable page limit prevents
the codec's required worst-case output scratch from growing implicitly; the
16 MiB default covers the existing maximum individual value with headroom, and
larger pages remain raw. The page digest authenticates stored bytes before
decode, while the existing whole-file footer authenticates the complete
framing. This follows the useful separation in Parquet between compressed and
uncompressed page sizes and a checksum over serialized stored page bytes,[^24]
without copying Parquet's encoding, metadata system, or reader.

Compression must not weaken rrflowKV's resource boundary. Before loading any
semantic page, the v6 parser reconstructs and bounds the entire hypothetical
uncompressed segment layout to the existing 1 GiB segment envelope. A selected
page is then read by physical length, authenticated, decoded into an aligned
buffer sized from the already-bounded logical length, and accepted only when
the decoder returns that exact length. The decoded buffer—not the smaller
encoded block—is charged to the page cache. RocksDB's documentation supports
lightweight per-block compression, raw fallback for poor ratios, decoded block
caching, and separate consideration of heavier bottom-level codecs,[^25] but
RRFlow rejects its missing-codec fallback and option topology.

Zstandard remains a laboratory candidate until C-07 has real level and cold
placement semantics. The segmented-LRU simulation alone did not justify a
change; C-06j's later persisted production-reader differential does. Moka's
weighted capacity is documented as best-effort and its weight does not control
victim selection,[^26] which is not enough for RRFlow's exact decoded-byte,
pinned-generation, scan, and physical-evidence contract. Value separation
remains rejected for lack of crash-safe publication, snapshot, range-read, and
value-log garbage-collection proof.

This physical boundary deliberately prepares rather than impersonates F-01.
The future DataFusion provider performs I/O in its polled execution stream and
claims pushdown only when rrflowKV actually avoids work.[^4] DataFusion's
memory-pool contract does not automatically track `DataSourceExec` or flowing
`RecordBatch` memory,[^27] so rrflowKV source, decode, and cache accounting must
remain explicit and later compose with operator reservations under F-04/F-05.
The complete byte contract, source map, corruption matrix, and execution order
are owned by the
[adaptive page-compression engineering plan](../roadmap/c06i-rrflowkv-adaptive-page-compression-engineering-plan.md).

### Adaptive page-compression integration result

Segment v6 now carries the authenticated writer policy and per-page raw/LZ4
identity described above. The default writer attempts a checked LZ4 block for
each page up to the authenticated 16 MiB attempt cap and retains it only at the
1,250-basis-point threshold. The normal reader authenticates stored bytes
before exact-length decode, bounds the reconstructed logical segment before
allocation, borrows only raw mmap pages, and charges compressed pages as owned
aligned decoded cache entries. Flush and compaction use the configured policy;
`none` remains an explicit conformance option. Versions 1 through 5 are rejected
with no compatibility reader.

The exact none/adaptive independent-model corpus passes through reopen and
protected compaction. The corruption corpus covers authenticated policy and
codec discriminants, reserved bytes, raw and compressed length invariants,
page caps, the aggregate logical envelope, stale and recomputed checksums, and
too-short/too-long LZ4 output. A structure-aware current-format fuzz target
combines blind mutations with checksum-rewritten header, descriptor, stored
page, and footer mutations. On clean revision `eb7445e`, one warm-up and three
retained isolated children exited zero and recorded 50 raw plus 784 compressed
reopened pages, 1,418,038 stored versus 8,988,877 logical page bytes, and
336,041 query-decompressed bytes. This is production-path integration evidence
on one host, not release, all-workload, cross-platform, or competitor proof.

### Scan-resistant page-cache integration result

C-06j retains one cache implementation and two closed process-local policies:
exact LRU as an oracle/operator choice and a default probationary/protected
scan-resistant LRU with an 8,000-basis-point protected target. The first real-
reader attempt exposed why unconditional second-hit promotion was insufficient:
one projected stream generated 601 repeated page hits and both policies then
reloaded 45 hot pages. RRFlow preserved the workload and instead allocated one
opaque scope per stream. A probationary hit from that same scope now refreshes
recency and counts suppression; reuse by a later operation may promote. Scope
identity is internal scheduling metadata, never client input, durable state,
authorization, query semantics, or trace identity.

Clean revision `5b1c31d` runs the same persisted eight-family corpus under
both policies in three isolated release-profile children. Exact LRU records 48
post-scan hot-page loads; scope-aware scan-resistant LRU records zero, 18,200
same-scope suppressions, 48 promotions, and 48 protected entries. Both remain
within the 1,048,576-byte capacity and produce identical manifest, semantic,
and projected-row digests. Artifact SHA-256 is
`8a008ee33bb50ca197783227cfcfbb4d58945dd2f26dca8cdf12e1804106b9ad`.
This qualifies C-06's mixed-workload/cache choice only. C-07 retains global-
mutex contention, duplicate-load coalescing, sustained concurrency, recovery,
and cross-platform reader lifetime; F retains DataFusion provider and query-
cache budgets.

## Execution order

The canonical roadmap order is the authority:

1. C-06g: pinned selective projected rrflowKV stream and phase-scoped evidence
   (implemented candidate; recorded in the execution journal);
2. C-06h: property/fuzz/adversarial and mixed-family correctness (implemented
   candidate with stable, stress, and bounded sanitizer evidence);
3. C-06i: persisted filter and adaptive LZ4 integrated;
4. C-06j: scope-aware scan-resistant cache integrated through the real mixed-
   family reader; stationary values and family-neutral capacity retained, and
   C-06 accepted by the canonical roadmap;
5. D-01: real `rrflow`/`rrflow.exe` install plan/apply, create/open/inspect,
   serve, authenticated ready, commit, close/reopen and baseline verify;
6. C-07: recovery, background maintenance, backpressure, ENOSPC and sustained
   lifetime through the installed product;
7. D-02 through D-04: durable attunement jobs, deterministic project tree and
   incremental parse;
8. E: persistent native graph, scalar, BM25, exact-vector and ANN generations;
9. F: stamped streaming Arrow/DataFusion and one cross-operator resource
   ledger;
10. D-05 onward, then G/H: complete attunement, routing, persisted reasoning,
   dynamic recall, public surfaces and Connectome; and
11. I: canonical engine events, triggers, routines, skills, adapters and the
    discussion-to-configuration/install workflows described above.

Before this research batch, the supporting execution map placed C-07 before
D-01 in one forward table. That contradicted the canonical roadmap, which
intentionally puts the walking product before installed-path qualification.
The supporting table is corrected with this record; it cannot override the
roadmap.

## Acceptance ladder and claim policy

### Alpha-integrity threshold

- one native primary executable and offline bundle-resident install path;
- explicit create/open/inspect and authenticated readiness;
- no lost acknowledged write in the required crash/failure matrix;
- exact MX/KV semantic differential and close/reopen proof;
- bounded streaming reads with owned generations;
- background maintenance and explicit backpressure;
- persistent native graph/BM25/vector paths with exact oracles;
- stamped Arrow/DataFusion stream and truthful pushdown;
- deterministic project attunement and persisted reasoning/routine state; and
- complete verify/backup/restore/repair rehearsal for the declared scope.

### RocksDB-class production qualification

- continuous seeded crash/stress and fuzz lanes over mixed operations/options;
- Linux, Windows and macOS filesystem/lifetime coverage;
- sustained load until steady state, with bounded RSS, queue depth and debt;
- read, write and space amplification ledgers;
- fixed-hardware latency/throughput histograms with coordinated-omission-safe
  load generation;
- inspection, status, quick/full verify, backup/restore and safe repair tools;
- release/diagnostic parity and sanitized failure bundles; and
- a published matrix of unsupported semantics and known limits.

### RRFlow differentiation threshold

Only after both layers above pass may RRFlow claim an advantage from its hybrid
tables, native graph/index families, TurboQuant, LSM-vector layout, or
DataFusion integration. Each claim must name the workload where it wins and
the correctness/recall tradeoff. A faster result with a weaker durability,
snapshot, filter, or recall contract is not a win.

## Research limitations

- No benchmark was run in this research package, so it makes no latency,
  throughput, amplification, recall, deployment-size, or superiority claim.
- External project documentation describes those projects' intended behavior;
  it is not independent proof of every implementation claim.
- The repository owner's reported source-use waivers are outside this technical
  review. Any adapted implementation still requires a provenance record.
- TurboQuant and LSM-VEC remain research inputs until the complete algorithms
  are reproduced and compared against exact oracles in RRFlow.
- The discussion-to-configuration design is a draft boundary for review. It
  does not change Gate I status and does not authorize its code ahead of the
  roadmap.

## Sources

[^1]: [RocksDB overview](https://github.com/facebook/rocksdb/wiki/RocksDB-Overview), especially the production tools, `db_stress`, fuzzing and `db_bench` inventory (accessed 2026-09-12).
[^2]: [RocksDB stress test](https://github.com/facebook/rocksdb/wiki/Stress-test), randomized continuous black-box/white-box crash, reopen, validation and sanitizer practice (accessed 2026-09-12).
[^3]: [RocksDB write stalls](https://github.com/facebook/rocksdb/wiki/Write-Stalls), memtable/L0/pending-compaction backpressure behavior (accessed 2026-09-12).
[^4]: [Apache DataFusion custom table provider](https://datafusion.apache.org/library-user-guide/custom-table-providers.html), planning, execution-plan and polled-stream responsibility plus pushdown guidance (accessed 2026-09-12).
[^5]: [FoundationDB simulation and testing](https://apple.github.io/foundationdb/testing.html) and [client testing](https://apple.github.io/foundationdb/client-testing.html), deterministic seeded simulation, injected failures, live performance and hardware testing (accessed 2026-09-12).
[^6]: RocksDB [live SST tracking](https://github.com/facebook/rocksdb/wiki/How-we-keep-track-of-live-SST-files), [WAL tracking in MANIFEST](https://github.com/facebook/rocksdb/wiki/Track-WAL-in-MANIFEST), [checksum handoff](https://github.com/facebook/rocksdb/wiki/Full-File-Checksum-and-Checksum-Handoff), [iterator implementation](https://github.com/facebook/rocksdb/wiki/Iterator-Implementation), and [per-operation I/O statistics](https://github.com/facebook/rocksdb/wiki/Perf-Context-and-IO-Stats-Context) (accessed 2026-09-12).
[^7]: [Fjall repository](https://github.com/fjall-rs/fjall), safe-Rust embedded LSM features, durability modes, snapshots and transactional modes (accessed 2026-09-12).
[^8]: [`lsm-tree` repository](https://github.com/fjall-rs/lsm-tree), LSM primitive scope and block/filter/cache/compaction/value-separation features (accessed 2026-09-12).
[^9]: [SurrealKV repository](https://github.com/surrealdb/surrealkv) and [architecture](https://github.com/surrealdb/surrealkv/blob/main/docs/ARCHITECTURE.md), product-specific Rust LSM motivation and current declared capabilities (accessed 2026-09-12).
[^10]: [SlateDB repository](https://github.com/slatedb/slatedb), object-store LSM architecture and latency/cost tradeoff (accessed 2026-09-12).
[^11]: [Lance file format](https://github.com/lance-format/lance/blob/main/docs/src/format/file/index.md), column/page metadata and random-access layout (accessed 2026-09-12).
[^12]: [Apache Arrow columnar format](https://arrow.apache.org/docs/format/Columnar.html), physical buffer layout, alignment and zero-copy constraints (accessed 2026-09-12).
[^13]: [Moka documentation](https://docs.rs/moka/latest/moka/) and [repository](https://github.com/moka-rs/moka), concurrency and best-effort bounded admission/eviction semantics (accessed 2026-09-12).
[^14]: [WiscKey: Separating Keys from Values in SSD-conscious Storage](https://www.usenix.org/system/files/conference/fast16/fast16-papers-lu.pdf), FAST 2016.
[^15]: [LSM-VEC](https://arxiv.org/abs/2505.17152), disk-oriented vector indexing over an LSM design; research reference, not accepted RRFlow evidence.
[^16]: [TurboQuant](https://arxiv.org/abs/2504.19874), quantized vector search estimator; research reference, not accepted RRFlow evidence.
[^17]: [Kubernetes API conventions](https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md), desired spec, observed status and generation conventions (accessed 2026-09-12).
[^18]: [Cedar schema-based policy validation](https://docs.cedarpolicy.com/policies/validation.html), typed validation before policy use (accessed 2026-09-12).
[^19]: [Common Expression Language specification](https://github.com/cel-expr/cel-spec), mutation-free, non-Turing-complete typed expression model (accessed 2026-09-12).
[^20]: [CloudEvents specification](https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md), vendor-neutral occurrence/event identity and context envelope (accessed 2026-09-12).
[^21]: [Rust Fuzz Book](https://rust-fuzz.github.io/book/), Rust-native fuzzing practice, and [`loom`](https://docs.rs/loom/latest/loom/), deterministic concurrency permutation testing (accessed 2026-09-12).
[^22]: [`fail-rs`](https://github.com/tikv/fail-rs), runtime-configurable Rust fail points for deterministic and probabilistic failure injection (accessed 2026-09-12).
[^23]: [LZ4 block format at pinned revision `0774d055`](https://github.com/lz4/lz4/blob/0774d05537f9762f838f7ab541b7765f1a729cb5/doc/lz4_Block_format.md), independent-block framing limits and bounded decode requirements (accessed 2026-09-13).
[^24]: [Apache Parquet page metadata at pinned revision `bb22d017`](https://github.com/apache/parquet-format/blob/bb22d0171b47000308e24209db79876b8dbe9566/src/main/thrift/parquet.thrift), separate compressed/uncompressed sizes and checksum placement (accessed 2026-09-13).
[^25]: RocksDB [compression](https://github.com/facebook/rocksdb/wiki/Compression) and [block-cache](https://github.com/facebook/rocksdb/wiki/Block-Cache) guidance, per-block policy, decoded caching, pinning, and scan concerns (accessed 2026-09-13).
[^26]: [Moka at pinned revision `a616ec19`](https://github.com/moka-rs/moka/tree/a616ec19e8d4ed938caf8b2c88090331d778d5da), best-effort capacity and weighted-eviction semantics (accessed 2026-09-13).
[^27]: [DataFusion 55 `MemoryPool`](https://docs.rs/datafusion/55.0.0/datafusion/execution/memory_pool/trait.MemoryPool.html), including memory that is not automatically tracked by the pool (accessed 2026-09-13).
