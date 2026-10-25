# Gate C — make rrflowKV the only local persistent substrate

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-c`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [x] | C-01 | Freeze one ordered binary key codec for current records, temporal versions, outgoing/incoming edges, scalar values, term postings, vectors, projection deltas, catalogue state, and runtime commits. | `rrd-core`, `rrd-store` | Ordering/golden tests prove prefix boundaries, round trips, tenant separation, and malformed-key rejection. |
| [x] | C-02 | Expose the minimal snapshot transaction primitives required by the semantic store: point read, bounded range scan, put, delete, commit, rollback, and conflict. | `rrd-lsm`, `rrd-store` | rrflowKV and rrflowMX conformance suites agree on read-your-writes, repeatable reads, range ordering, and write conflicts. |
| [x] | C-03 | Commit canonical record, relation, both adjacency directions, synchronous index changes, runtime log entry, durable projection deltas, function invocation receipt and derived proposal, effect-complete audit, and outbox entry as one write batch. Replace the private monolithic function-catalogue control record with typed definitions, bindings, content-addressed artifacts, immutable membership revisions, and one compare-and-swap head under the same transaction authority. | `rrd-store`, `rrd-engine` | The shared rrflowMX/rrflowKV corpus plus failure injection at every prepare/WAL/batch/acknowledgement boundary proves all-or-nothing behavior; an allowed function audit cannot survive a failed domain commit, advertised catalogue limits fit physical limits, and rrflowKV reopens without re-executing a prepared function under another runtime build. |
| [ ] | C-04 | Serve current and temporal reads from direct versioned keys at one `ReadStamp`; remove normal-path whole-log reconstruction. | `rrd-store` | Physical counters and plan evidence show bounded point/range reads while exact snapshot comparisons remain equal. |
| [x] | C-05 | Keep Fjall selection, migration-only runtime paths, and alternate stores absent; remove every pre-1.0 reader and alternate format branch from the 1.0 executable. | `rrd-store`, workspace | Fresh rrflowKV database and format-rejection tests pass; repository search and dependency metadata contain one rrflowKV opener and one accepted physical-format reader. |
| [x] | C-06 | Replace row-record immutable segments with the hybrid rrflowKV layout: an ordered key/version spine plus Arrow-compatible column pages, explicit encoding/compression metadata, authenticated persisted membership filters, safe buffer lifetimes, and one exact-byte scan-resistant physical page cache. Keep point/range/CAS reads independent of DataFusion. Segment v6 retains canonical row-group filters and authenticated none/adaptive-LZ4 page policy; the family-neutral cache defaults to scope-aware probationary/protected LRU while exact LRU remains a selectable oracle/operator policy. Current evidence rejects key/value separation, semantic-family partitioning, Moka, TinyLFU, and caller-controlled cache bypass rather than assuming them as architecture. | `rrd-lsm`, `rrd-store` | Frozen format vectors, property/fuzz tests, exact differential reads, authenticated filter/codec/length-corruption and reopen-I/O tests, selective projection/scan counters, mixed-family interference tests, and comparative production-reader measurements prove the layout; eligible raw/aligned pages borrow buffers while compressed pages decode into exact bounded owners and all filter, stored-read, logical, decoded, decompressed, copied, allocated, cached, admission, promotion, suppression, demotion, eviction, and open-validation bytes/events are reported. Stationary pages remain the accepted placement because the value-separation model lacks atomic publication, snapshot, recovery, corruption, range-read, and garbage-collection proof. |
| [ ] | C-07 | Prove WAL recovery, manifest recovery, bounded maintenance and write backpressure, pinned-snapshot compaction, Arrow-page lifetime safety, checksums, storage-full behavior, and acknowledged-write durability. | `rrd-lsm` | Crash matrix, reader/compaction concurrency, sustained-write/maintenance/RSS runs, and repeated reopen suite pass with no lost acknowledged write, unbounded write-buffer growth, dangling mapped buffer, or exposed partial batch. |


## [Accepted evidence](gate-c-evidence.md)

## C-07 implementation packages

The [installed durability and bounded-growth plan](gate-c-07.md) owns C-07's
ordered packages, failure/resource oracles, native qualification and handoff
to the existing retrieval, connector, UI and competitive-evaluation gates.
The plan does not complete C-07 or change the accepted C-06/D-01 evidence.

## Exit condition

Gate C exits only when rrflowKV is the sole local persistent implementation and
its correctness is demonstrated below the semantic engine.
