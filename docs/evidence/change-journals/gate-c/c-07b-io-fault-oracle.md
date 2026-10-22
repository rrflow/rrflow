# C-07b native-call fault and stable-byte oracle

**Status:** active implementation evidence; exact-candidate CI required for acceptance
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07b-io-fault-oracle`
**Owner:** OBJ-02 prerequisite, [C-07b](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Starting revision and change brief

Public development main `fca1a980245a9ff8634fa70fe21ee79a27eb2047`, tree
`7678086381f4a9112d4dcf0d7acdb2463b16e494`, was fetched into a clean Windows
checkout. [PR 56](https://github.com/rrflow/rrflow-development/pull/56) received
the change brief before implementation, following the
[baseline seed](c-07b-io-baseline.md). The branch is
`agent/c07b-storage-io-faults`; all pushes resolve only the public development
remote, ref and exact revision. No official promotion is requested or performed.

The package replaces coverage limited to logical stop milestones with actual
native-call faults and an independent stable-byte/name model at the existing
WAL/filesystem/manifest owners. It repairs demonstrated uncertain-I/O handling;
it does not introduce another persistence backend or transaction authority.
Formats, dependencies, policy defaults, public schemas, product version and
provider behavior remain unchanged. Stop conditions are lost acknowledged
values, partial atomic batches, stale authority, undeclared scope/format changes,
unbounded fixtures or unsupported completion claims.

## Implementation and traceability

| Existing owner and changed path | Final behavior and replacement evidence |
|---|---|
| `crates/persistence/rrd-lsm/src/wal.rs` | Native file calls pass through private helpers. Zero-byte vectored append returns WriteZero and poisons; standalone sync failure poisons. New WAL names receive a parent barrier; a reused empty successor receives file and parent barriers before publication. Existing framing/goldens remain. |
| `src/filesystem.rs`, new `src/filesystem/io.rs` | Native anchored handles still perform every real effect. Test-only observation/faults wrap open/read/write/data-sync/file-sync/directory-sync/rename/remove. Short and interrupted writes exercise the real loops; before/after-effect errors remain distinct. No production fault controller exists. |
| `src/manifest.rs` | Uncertain publication/checkpoint effects fence further publication. Reopen stabilizes selected CURRENT/checkpoint names before dependent effects. Deduplicated manifest/checkpoint publication establishes the missing parent barrier. |
| `src/database.rs`, `src/error.rs` | Writer readiness checks manifest uncertainty; GC and both snapshot-install paths deny stale effects. Recovery-required/poison messages describe unresolved state without incorrectly attributing every error to append. |
| `src/segment/mod.rs` | Immutable writes/syncs use the private call helpers. Reusing an existing segment stabilizes its directory entry before a manifest can select it; v6 bytes and read/cache algorithms remain. |
| `src/lib.rs`, new `src/fault_io.rs`, `src/fault_io/model.rs`, `src/fault_io/tests.rs` | Unit-test-only thread-local scheduling, RAII cleanup, independent byte/namespace model, requested-value oracle, exhaustive bounded call sweep, partial-frame and retry/reopen regressions. No Cargo feature or environment variable enables faults in a product build. |
| `.github/workflows/ci-reusable.yml`, `docs/operations/ci.md` | Linux installed and Windows/macOS portability lanes run the focused oracle with a five-minute step bound. Existing kernel/optional-feature suites also cover unit tests. |

No implementation is moved or deleted. Existing WAL/manifest/segment goldens,
containment, MVCC, compaction, snapshot, transaction and projected-read tests
remain the characterization corpus. The only structural additions are private
helpers and test code within the canonical LSM crate. The roadmap chapter,
dependency spine and POAM-035 link this evidence; generated navigation and file
inventory are regenerated rather than maintained as a second authority.

## Oracle, bounds and limitations

The fixed requested history contains two acknowledged seed pairs, two further
two-key authoritative writes, flushes, compaction, a retained checkpoint,
checkpoint release and reclamation. Expected keys/values and acknowledged or
attempted IDs come from the submitted history, never engine readback. Every
acknowledged pair must survive; an uncertain pair may be wholly present or
absent; an unattempted pair must be absent. Each recovered estate must accept
another valid write and survive GC/reopen.

The device model is seeded from fixture files before fault scheduling, records
write-input bytes, and separates live/stable contents from live/stable directory
entries. File sync alone cannot publish a name; directory sync alone cannot
persist contents. Rename retains object identity. Loss reconstructs only the
stable image after all engine handles and mappings drop. Separate model tests
prove buffered-byte loss and unsynchronized rename/unlink behavior.

On Windows the bounded history exposed **127 intercepted calls**, swept through
**314 fault/loss cases**. Each failure image then exercised every intercepted
recovery occurrence before and after effect: **7,452 compound recovery cases**.
Every failed ordinary reopen preserves file bytes, and a subsequent clean reopen
checks the independently requested values. Counts can differ by platform because
native vectored-write completion differs; the test discovers and sweeps that
host's complete bounded history instead of hardcoding a successful count.

Additional regressions cover sync poisoning, zero writes, newly created WAL
names, stale CURRENT continuation/GC/snapshot install, 18 combinations of
partial header/payload writes followed by errors, segment/manifest/CURRENT
retry/reopen, uncertain checkpoint unlink, and reused successor synchronization.
Ordinary torn-tail open preserves bytes and denies access; only the explicitly
invoked repair test truncates a classified incomplete tail.

Scheduling is bounded to 4,096 events and at most two scheduled faults per
session; the model limits objects to 128 and combined live/stable bytes to 8 MiB.
Temporary roots are test-owned; no disk-filling, host policy, external service or
project state is touched. Windows lock-file inventory checks its zero length
without reading through the exclusive lock. Traces contain operation kinds,
fixture paths and ordinals, with no production payload telemetry.

This is **I/O-model evidence**, including real native-call error propagation.
It does not model arbitrary torn sectors, controller caches, kernel crashes or
all filesystem operations. The bounded corpus covers an already-created estate;
fresh-install namespace creation, arbitrary persistent damage, process-death
ACK receipts, native ENOSPC, aggregate hard resource caps, lifetime concurrency
and sustained scale retain their owning later packages. It does not claim every
possible program history or native power-loss safety.

## First failures and corrections

The first four focused regressions failed as expected for zero-write handling,
standalone sync poisoning and a missing new-WAL name barrier. The initial
publication test first exposed a harness issue: Windows denied reading the
exclusively locked empty lock file. After checking zero length instead, the
regression reproduced another acknowledged write through the obsolete WAL
after CURRENT rename failed after effect. The four regressions then passed.

The first broad sweep passed 314 cases and 6,196 compound recovery cases. A
separate continuation fixture then failed with a missing selected segment after
retrying its uncertain rename. Repairs added reuse barriers and stabilization
of observed CURRENT/checkpoint names on reopen; the expanded recovery sweep
contains the additional directory calls and 7,452 compound cases. Successor
reuse also explicitly synchronizes its validated header before becoming reachable.
Snapshot installation is fenced before creating imported objects, not merely
when it eventually attempts manifest publication.

Intermediate compiler feedback found a private test enum import and an unused
Write import; both were corrected. Strict Clippy found one needless borrowed
snapshot path in the new test; it was corrected without changing behavior.
An attempted relink while the earlier Windows test executable was running
failed with LNK1104; waiting for that owned test to finish and rerunning succeeded.
No test process was killed or failure hidden. These are retained development
failures, not accepted candidate results.

## Acceptance commands and results

Executed on the Windows checkout with the repository's locked Rust toolchain:

- `cargo test -p rrd-lsm --test wal --test failure_matrix --test manifest --test compaction`: baseline **30 passed**.
- `cargo test -p rrd-lsm --lib fault_io -- --nocapture`: focused fault corpus passed; the nine-test run recorded the 314/7,452 sweep before adding the focused successor test, which separately passed.
- `cargo test -p rrd-lsm --lib fault_io --locked -- --nocapture`: final **10 passed**, 127 calls, 314 fault/loss cases and 7,452 compound recovery cases; 41.47 seconds on this local run, without a performance claim.
- `cargo test -p rrd-lsm --locked`: **127 passed**, including 22 unit tests; the Linux-only snapshot-memory target and doc tests executed zero tests on Windows. This run preceded the final two snapshot-entry guards.
- `cargo test -p rrd-lsm --lib uncertain_current_publication --locked`: final snapshot-entry guard regression **1 passed**; denied bundle/file installation preserves target files.
- `cargo test -p rrd-lsm --test snapshot_bundle --locked`: after those guards, **6 passed**.
- `cargo test -p rrd-store --test durability --test rrflow_kv_open --test semantic_commit_atomicity --locked`: **13 passed** across the storage adapter boundary.
- `cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings` and `cargo fmt --all --check`: passed after the corrections above.
- `python -X utf8 scripts/ci/check_workflow.py`: passed; three workflows, seven substantive jobs, five engine suites, twenty default-feature packages and six optional-feature packages.

- `python -X utf8 scripts/ci/check_documentation.py`: passed, 386 statuses and 384 classified coordinates.
- `python -X utf8 scripts/knowledge/render_navigation.py --check`: passed, 17 indexes, 385 nodes and 1,679 edges.
- `python -X utf8 scripts/ci/build_execution_inventory.py --check`: passed, 1,366 records.
- `git diff --check`: passed. Generated changes were reconciled to the declared paths.

Exact candidate
CI, native host results and merge identity are bound by PR 56 and its required
`pipeline / ci-gate`; this immutable journal does not predict their outcome.
All candidate partitions must pass before merge.

## Research, review and change checklist

[SQLite anomaly/compound-failure testing](https://www.sqlite.org/testing.html)
informed finite fault enumeration and subsequent recovery failure;
[PostgreSQL 18 reliability](https://www.postgresql.org/docs/18/wal-reliability.html)
informed separation of synchronization from device-cache guarantees. Sources
were revisited 2026-10-21; no code was copied or upstream architecture adopted.
Bounded test traces and exact value assertions suffice for this correctness
package. Production tracing, profiling and resource benchmarking are unchanged.

Read and reviewed: README, AGENTS, OBJ-02 and dependency spine; C-07 and
POAM-035; authoring/navigation and CI owners; LSM WAL, filesystem/native,
manifest, database, segment, lib and error sources; WAL and failure-matrix
tests; every new helper/model/test file and the complete diff. The source review
followed callers through ordinary writes, transactions, flush/compaction,
checkpoint/GC and both snapshot-install entry points. Full changed-file reread
and generated-diff review were completed before commit. No unrelated changes are included.

Change checklist: owner and baseline bound; PR brief predates implementation;
current/target behavior and exact paths declared; traceability retains all
formats/tests/authority; smallest failing oracles recorded; research and
trace/resource decisions explicit; owning suite and adapter checks executed;
failures and omissions retained; generated projections refreshed; candidate CI
required; public-development-only landing; no product version change.

Remaining errors and status: no known failing local acceptance test after the
listed corrections. POAM-035 remains open; C-07a/b now have bounded implementation
evidence, with C-07c next only after exact-candidate acceptance. OBJ-02, Gate C,
alpha and release remain incomplete. Native exhaustion/power loss, installed
crash histories, sustained throughput/RSS and competitive/full-product behavior
were not run or claimed by this package.
