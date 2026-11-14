# C-06j rrflowKV scan-resistant cache engineering plan

**Status:** active supporting work-package evidence; runtime implementation and clean artifact complete
**Coordinate:** `rrflow://rrflow-instance/data/work-package/c-06j-rrflowkv-scan-resistant-cache`
**Owner:** subordinate C-06 implementation and evidence record
**Baseline revision:** `9592b886f2c6f2f716933fcf46e8c0595de3c315`
**Baseline tree:** `bc177bbdb4af4adb1e30ae0cfe8f6ae46d2fd949`
**Research:** [`rrflowkv-page-cache-and-mixed-workload-research.md`](../research/rrflowkv-page-cache-and-mixed-workload-research.md)

## Outcome

Replace rrflowKV's sole admit-every-miss immutable-page LRU with one
configurable, exact-byte cache that supports both the existing exact LRU oracle
and a scan-resistant probationary/protected policy. Make that policy
scope-aware: repeated page requests inside one projected stream remain
probationary, while a later engine operation may promote them. Make it the
process-local default only after a real persisted eight-family
workload proves semantic identity, exact capacity, and fewer post-scan hot-page
loads. Record but do not solve concurrent duplicate loads. If all prior C-06
evidence remains green and the remaining value-placement/family-grouping
choices are explicitly rejected from measured evidence, close C-06 and make
D-01 the next dependency-ordered package.

This package changes no segment, WAL, manifest, snapshot, application-key, or
public query encoding. It adds no database daemon, cache service, DataFusion
object, semantic-family quota, hook, routine, skill, adapter, or compatibility
path.

## Why this is the next package

The canonical roadmap records three remaining C-06 decisions after segment v6:

1. mixed-family interference;
2. integrated page-cache policy; and
3. whether key/value separation or explicit family grouping is justified.

The existing candidate screen already rejects value separation as a byte-only
model and advances a segmented cache in simulation. The missing proof is the
real v6 reader under one mixed persisted corpus. Finishing that proof is the
smallest coherent step before C-07 recovery/concurrency and D-01 installation.

## Baseline behavior and ownership

| Path and symbols | Current behavior | Required change |
|---|---|---|
| `crates/persistence/rrd-lsm/src/segment.rs` — `PageCacheStats`, `PageCache`, `CacheEntry`, `new_page_cache`, `load_page_with_evidence`, `impl PageCache` | One `HashMap`, one lazy LRU heap, one global mutex, admit every fitting miss; duplicate losing loads are unclassified. | Add the closed policy type, exact/probationary/protected regions, validated basis-point target, exact state transitions, strict region/total accounting, admissions/promotions/demotions/rejections/duplicate-load evidence, and policy-aware lookup/insertion helpers. |
| `crates/persistence/rrd-lsm/src/segment/reader.rs` — `ProjectedReadStream`, `SegmentProjectedCursor`, `load_projected_page` | One projected stream can request the same row-group page repeatedly while preparing and copying rows; no logical cache-reuse scope exists. | Allocate one checked opaque scope per stream and pass it through every segment page request so same-stream hits refresh without promoting; later streams get distinct scopes. |
| `crates/persistence/rrd-lsm/src/database.rs` — `DatabaseOptions`, create/open cache construction, open trace | Configures bytes only and always constructs exact LRU. | Add non-durable `page_cache_policy`, validate before filesystem mutation, pass it to the sole cache constructor, expose current policy, and trace low-cardinality kind/ratio on open. |
| `crates/persistence/rrd-lsm/src/lib.rs` — database/segment exports | Exposes bytes and cumulative stats but no policy. | Export the closed policy and default protected-target constant; expose no cache internals. |
| `crates/persistence/rrd-lsm/src/segment/physical_policy_lab.rs` — cache simulator, reopened integration, decisions | Compares simulated exact/segmented LRU; real reopen uses the implicit default and does not compare policies. | Keep the simulator as a deterministic screen, add a real same-database exact-versus-scan-resistant mixed-family workload, bind semantic digests and production counters, and call the selected policy integrated only when the real invariant passes. |
| `crates/persistence/rrd-lsm/examples/rrflowkv_physical_policy.rs` — child identity, aggregate evidence, artifact envelope | Emits C-06i codec/filter evidence and simulated cache hit counts. | Rev the evidence envelope, authenticate the integrated per-policy observations across children, aggregate post-scan loads/hits, emit C-06j scope/limitations, and reject a false or vacuous cache result. |
| `crates/persistence/rrd-lsm/tests/page_cache.rs` | Absent. | Add black-box configuration, exact differential, mixed-family pollution, reopening, and capacity/evidence tests through public `Database` and projected-read APIs. |

No production cache module is moved in this package. Moving the current cache
out of `segment/mod.rs` would mix a structural rewrite with the policy proof;
that can be planned after behavior is frozen. No existing test file is deleted
or weakened.

## Exact public type changes

Add beside `PageCacheStats`:

```rust
pub const DEFAULT_PAGE_CACHE_PROTECTED_CAPACITY_BASIS_POINTS: u16 = 8_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageCachePolicy {
    ExactLru,
    #[default]
    ScanResistantLru {
        protected_capacity_basis_points: u16,
    },
}
```

If Rust's derived enum default cannot express the configured field cleanly,
implement `Default` manually. The serialized form must remain closed and typed;
do not use a stringly typed policy map.

Required methods:

```text
validate() -> Result<Self>
kind() -> &'static str
protected_capacity_basis_points() -> u16
```

Validation:

- `ExactLru` always validates;
- `ScanResistantLru` requires `1..=9_999` basis points;
- invalid policy fails `DatabaseOptions::validate` before creating a path;
- a zero-byte cache is allowed as an explicit no-residency configuration and
  still reports loads/rejected admissions truthfully.

Add `page_cache_policy: PageCachePolicy` to `DatabaseOptions`; default it to
`PageCachePolicy::ScanResistantLru` with the repository's measured 8,000 basis
point target. Add `Database::page_cache_policy()` or return the policy in
`PageCacheStats`; do not persist it into the manifest because changing a
process-local replacement policy does not change durable semantics.

Extend `PageCacheStats` with exact cumulative/current fields:

```text
policy
probationary_resident_bytes
protected_resident_bytes
probationary_entries
protected_entries
admissions
admission_rejections
promotions
demotions
same_scope_hits
duplicate_loads
```

Existing hit/miss/eviction/load and byte ownership counters retain their
meaning. `loads` counts each actual `read_page` completion, including a
duplicate losing load. `duplicate_loads` counts only a completed load whose
key was inserted by another reader before the completion lock was reacquired.
`same_scope_hits` counts probationary hits intentionally not promoted because
they came from the same projected stream that last touched the entry.
An oversize/no-capacity page increments `admission_rejections` but is still
returned to the requesting read.

## Internal cache state

Replace the single-region entry with:

```text
CacheRegion = Exact | Probationary | Protected
CacheAccess = OrdinaryOperation | ProjectedScope(non-zero u64)
CacheEntry = { value, last_used, region, last_access }
```

`PageCache` retains the one immutable-value map and adds one lazy-invalidated
min-heap per reachable region. It tracks total, probationary, and protected
resident bytes. A helper must rebuild stale heaps when heap metadata grows
beyond the existing bounded ratio. Clock wrap must renumber all entries while
preserving their relative recency and region.

Do not put `LoadedPage` bytes in a second map. Do not charge stored compressed
bytes; charge `LoadedPage::resident_bytes()` exactly as today. Do not retain a
request, key payload, prompt, tenant label, or semantic family in cache
metadata.

## State-transition algorithm

### Lookup

1. Lock the shared cache.
2. If absent, increment `misses` and return miss.
3. If exact LRU, refresh the exact-region stamp.
4. If scan-resistant and protected, refresh the protected stamp.
5. If scan-resistant and probationary:
   - if the current projected scope equals the entry's last scope, refresh it
     in probationary and increment `same_scope_hits`;
   - otherwise move its charged bytes from probationary to protected,
     increment `promotions`, refresh its stamp in the protected heap, and
     demote least-recent protected entries until protected bytes are at or
     below the configured target.
6. Increment `hits`, rebuild stale heap metadata if necessary, and clone the
   immutable `Arc`.

Lookup performs no I/O under the cache mutex.

Every `ProjectedReadStream` receives one checked non-zero scope generated
inside rrflowKV and shared by all of its segment cursors. A later stream gets a
different scope. Point operations use the ordinary-operation access class and
retain second-access promotion. Scope exhaustion fails stream creation; scope
values are never public input, persistent state, authorization, query
semantics, or trace dimensions.

### Completed load and admission

1. Read/authenticate/decode outside the mutex exactly as the v6 reader does
   now.
2. Reacquire the cache and update load and physical/decoded ownership
   counters.
3. If the key now exists, increment `duplicate_loads`, apply ordinary hitless
   recency preservation to the existing value, and return that immutable
   value. Do not replace it or double-charge residency.
4. If the loaded charge is larger than total capacity, or capacity is zero,
   increment `admission_rejections` and return it uncached.
5. Exact LRU: evict least-recent exact entries until the candidate fits, then
   insert into `Exact`.
6. Scan-resistant LRU: evict least-recent probationary entries until the
   candidate fits; only if no probationary victim exists may it evict the
   least-recent protected entry. Insert the candidate into `Probationary`.
7. Increment `admissions`; update total and region bytes; assert the exact
   total/region invariants in tests and return the loaded value.

### Promotion and demotion

The protected target is
`capacity_bytes * protected_capacity_basis_points / 10_000`, using checked
arithmetic. While protected residency exceeds that target, demote its
least-recent valid entry into probationary without changing total residency or
counting an eviction. A variable-sized page larger than the target can remain
cached but cannot monopolize protected capacity: it is demoted. Capacity
pressure subsequently chooses from probationary pages first.

Every heap pop must reject stale `(stamp, key)` records until the map entry has
the same region and stamp. An empty victim set while bytes exceed capacity is
an internal invariant failure, not permission to exceed the configured bound.

## Failure-first tests

Create `tests/page_cache.rs` before production edits. Its initial compile must
fail because `PageCachePolicy` and the new stats do not exist. Do not weaken
the test to make the old LRU pass.

### Configuration denial

- zero and 10,000 protected basis points fail before the database directory is
  created;
- exact LRU, default scan-resistant LRU, a valid non-default ratio, and zero
  capacity are accepted;
- reopening identical durable state with another cache policy changes no
  manifest, sequence, or values.

### Integrated mixed-family differential

Build one deterministic persisted v6 corpus containing audit, incoming edge,
outgoing edge, record, runtime, scalar, term, and vector-shaped prefixes. Use
multiple row groups and values large enough that a complete key/value
projected stream exceeds cache capacity. For each policy, from a fresh reopen:

1. read one present key per family twice;
2. capture the warm stats;
3. consume the complete bounded `ProjectedReadStream` and digest its exact
   rows;
4. read the same hot keys again; and
5. capture post-scan loads/hits, values, residency, and region state.

Assertions:

- both row/value digests equal an independent expected digest;
- both policies stay within capacity;
- exact LRU demonstrates at least one post-scan reload on this corpus;
- scan-resistant LRU records promotions and protected entries;
- the projected scan records same-scope suppressed promotions and cannot
  promote its own newly admitted probationary pages;
- scan-resistant post-scan loads are strictly fewer than exact LRU and are zero
  if the declared hot set fits the protected target;
- scan-resistant region bytes sum to total residency;
- exact LRU reports no probationary/protected region state; and
- neither policy changes snapshot, manifest digest, ordering, tombstones,
  compression evidence, or projected-read outcome.

### Internal variable-size and stale-heap cases

Unit cases in `segment/mod.rs` may call private cache methods with synthetic
`LoadedPage` buffers only if needed to prove:

- a page larger than the protected target but smaller than total capacity is
  served/cached without violating bounds;
- promotion followed by demotion preserves the same `Arc`;
- stale heap records cannot evict the wrong entry;
- clock renumber preserves region and recency; and
- a page larger than total capacity is served but rejected from admission.

Prefer the public integrated test for the retained policy verdict. Private
unit tests are supporting invariant evidence, not the decision oracle.

## Physical-policy laboratory and artifact

Increment `PHYSICAL_POLICY_EVIDENCE_VERSION` and the example envelope version.
Add a serialized `IntegratedCachePolicyObservation` containing:

```text
policy kind and protected ratio
snapshot/manifest/semantic digest
hot family count and hot request count
projected row count and row digest
capacity/current region bytes and entries
warm, scan, and post-scan hit/miss/load deltas
admissions/rejections/promotions/same-scope hits/demotions/evictions/duplicate loads
physical/decoded/decompressed byte deltas
semantic_identity_exact and exact_capacity_respected
```

The laboratory must create/flush the corpus once and independently reopen it
under exact and scan-resistant policies. Each run starts with an empty
process-local cache. Timing fields are distributions; deterministic identity
comparison excludes elapsed time and host RSS but includes all semantic,
configuration, byte, and counter results that should be stable.

The selected decision becomes `integrated` only when:

```text
both semantic identities are exact
both capacity invariants hold
same snapshot and projected row digest
scan-resistant promotions > 0
scan-resistant post-scan hot loads < exact-LRU post-scan hot loads
```

The clean artifact path is
`docs/evidence/c06j-rrflowkv-scan-resistant-cache-linux-x86_64.json`. It binds
the exact clean revision/tree, executable/Cargo.lock digests, command, compiler,
target, CPU, memory, filesystem, device, frequency policy, load, all raw child
trials, and limitations. The prior C-06i artifact remains immutable historical
evidence for segment v6 compression.

## Trace, resource, and debugging requirements

### Runtime trace

Add only low-cardinality configuration to the existing `rrd_lsm::open` event:

```text
page_cache_policy
page_cache_protected_capacity_basis_points
page_cache_capacity_bytes
```

Do not emit cache keys, page ordinals, family names, paths beyond the existing
operator path, values, prompts, or tenant identifiers. Cumulative per-read
counters remain accessible through `PageCacheStats` and existing storage/query
evidence; do not create an event stream per cache access.

### Resource evidence

The exact invariant is:

```text
resident_bytes <= capacity_bytes
scan-resistant resident_bytes
  == probationary_resident_bytes + protected_resident_bytes
```

Loaded immutable `Arc`s held by active readers may outlive cache eviction; this
is existing read ownership and must not be falsely counted as cache residency.
This package makes no process-RSS equality claim.

### Debugging evidence

Policy, ratios, admissions, promotions, same-scope suppressed promotions,
demotions, evictions, rejected admissions, duplicate loads, physical bytes,
decoded bytes, and post-scan loads must be sufficient to distinguish policy,
scope, disk/decode, or semantic failure. No debug-only alternative execution
path is allowed.

## Documentation and status edits after runtime proof

Update only current owners/supporting records:

- `README.md`: current status and next dependency, not algorithm duplication;
- `docs/objectives/rrflow-1.0-alpha.md`: truthful C-06 prerequisite status;
- `docs/roadmap/rrflow-1.0.md`: close C-06 only if every named acceptance
  condition is already backed by prior plus current evidence;
- `docs/poam/rrflow-1.0-alpha.md`: narrow POAM-002 to C-07 or close only the
  C-06 physical-layout component;
- `docs/architecture/engine-data-flow.md`: cache location below projected
  batches and future DataFusion;
- `docs/reference/storage/rrflowkv-current-format.md`: process-local policy and
  counters; fix any stale v5 wording encountered in the owned section;
- `docs/reference/storage/rrflowkv-benchmark-harness.md`: C-06j reproduction
  and limitations;
- `docs/research/rrflowkv-rust-storage-engine-architecture-research.md`: change
  only candidate/future statements now proven or rejected;
- `docs/evidence/test-plans/persistence-scenario-matrix.md`: mixed-family cache
  row and remaining C-07/F/J gaps;
- evidence/roadmap indexes, this implementation record, and execution journal;
  and
- regenerate the current file inventory deterministically under `target/` using
  `scripts/ci/build_execution_inventory.py`; the former tracked
  `docs/roadmap/rrflow-1.0-file-plan.jsonl` is a stable descriptor.

If any C-06 acceptance item remains unproved, leave C-06 open and name the
exact missing evidence. Never close it because the package compiles.

## Execution order

1. Commit the machine plan alone as the direct child of the clean baseline.
2. Run `check_change_plan.py`; only declared, still-uncommitted planning and
   research records may differ, and no runtime path may differ.
3. Commit this file, the research file/indexes, and the pre-edit traceability
   journal as a separate documentation-only child before runtime edits.
4. Add `tests/page_cache.rs`; record the intended missing-type compile failure.
5. Add and validate `PageCachePolicy` and `DatabaseOptions` propagation.
6. Run the unchanged real-reader workload. Preserve its first failure—45
   post-scan loads for both policies because 601 same-stream hits promoted scan
   pages—as the reason for the successor machine plan.
7. Add the engine-owned projected reuse scope and same-scope suppression.
8. Implement policy-aware lookup, promotion/demotion, admission, eviction, and
   counters without I/O under the mutex.
9. Make all focused black-box and internal invariants pass.
10. Extend the feature-gated laboratory and evidence example; run dirty smoke
   trials only.
11. Run the owning package, strict Clippy, architecture, and workspace check.
12. Commit runtime source and tests.
13. From the clean runtime revision, run release-profile evidence into the new
    artifact and validate deterministic child identity.
14. Update the declared source-of-truth/supporting records, journal exact
    commands/results, regenerate inventory, reread every changed file and the
    full diff, then commit closeout.
15. Resolve both remote URLs and refs; push normally only to
    `development/main`; verify its exact revision and verify official main did
    not move.

## Acceptance commands

```text
python3 scripts/ci/check_change_plan.py
cargo test -p rrd-lsm --test page_cache --locked
cargo test -p rrd-lsm --test segment database_page_cache_is_shared_bounded_and_observable --locked -- --exact
cargo test -p rrd-lsm --features physical-policy-lab cache_candidates_preserve_identity_and_exact_capacity --locked
cargo test -p rrd-lsm --all-targets --all-features --locked
cargo clippy -p rrd-lsm --all-targets --all-features --locked -- -D warnings
cargo test -p rrd-store --locked
cargo test -p rrd-engine --test workspace_architecture --locked
cargo check --workspace --all-targets --locked
cargo run --release --locked -p rrd-lsm --features physical-policy-lab --example rrflowkv-physical-policy -- --seed 14592251008053203194 --trials 3 --records-per-family 1024 --versions-per-key 2 --value-bytes 512 --misses 16384 --cache-bytes 1048576 --output docs/evidence/c06j-rrflowkv-scan-resistant-cache-linux-x86_64.json
python3 scripts/ci/build_execution_inventory.py --check
python3 scripts/ci/check_documentation.py
python3 scripts/ci/check_generated_surfaces.py
python3 scripts/ci/check_workflow.py
python3 scripts/check_version.py
cargo fmt --all -- --check
git diff --check
```

The release-profile evidence command runs only from a clean runtime revision.
Use `--allow-dirty` solely for discarded smoke output.

## Stop conditions

Stop and amend the committed plan before continuing if:

- exact and scan-resistant policies return different rows, values, ordering,
  snapshots, or manifests;
- cache or region residency exceeds capacity;
- a caller, family prefix, DataFusion node, provider, hook, or skill gains
  physical cache authority;
- a projected reuse scope is externally supplied, persisted, traced as an
  identifier, reused across streams, or changes authorization/query results;
- Moka, another cache crate, a frequency sketch, a ghost cache, or time-based
  promotion becomes necessary;
- policy is persisted into segment/manifest bytes or creates a compatibility
  reader;
- I/O occurs while holding the page-cache mutex;
- cache eviction invalidates an active `Arc<LoadedPage>`;
- the mixed corpus does not force exact-LRU reloads or the selected policy does
  not improve the declared post-scan result;
- same-stream projected hits promote probationary pages instead of incrementing
  the suppression counter;
- duplicate-load evidence cannot be defined without claiming coalescing;
- a new path was not declared in the machine plan;
- any existing storage/recovery/projected-read test regresses;
- C-06 closure would rely on the simulator, compilation, or one metric rather
  than all named evidence; or
- a push would target the official repository, rewrite history, or publish a
  release/tag/artifact.

## Completion record

Planning commits `d3a36b0` and `982b7a2` preceded their respective runtime
edits. Runtime commit `5b1c31de73cabf79fe3353112635a09f6a12e364`
implements the closed cache policy, exact accounting, scope-aware projected
reuse, production-reader comparison, and evidence validation. Its clean source
tree is `d99c09eadf567f55aaf3f41ea1614c16a697c1d4`; Cargo.lock remains
`316533e7512dbb9029e494386503ca8430b0c69853b3b78ef02319dcbab93a27`.

The retained Linux x86_64 artifact is
`docs/evidence/c06j-rrflowkv-scan-resistant-cache-linux-x86_64.json`, SHA-256
`8a008ee33bb50ca197783227cfcfbb4d58945dd2f26dca8cdf12e1804106b9ad`.
Three isolated release-profile children exit zero. Exact LRU performs 48 post-
scan hot-page loads; scope-aware scan-resistant LRU performs zero and records
18,200 same-scope suppressions, 48 promotions, and 48 protected entries. Both
policies remain within 1,048,576 bytes and return identical manifest,
semantic, hot-value, and projected-row identities.

All 114 `rrd-lsm` all-target/all-feature tests, strict Clippy, the complete
locked `rrd-store` suite, all 28 workspace-architecture tests, and the locked
workspace all-target check pass. The first 45-versus-45 real-reader failure,
the resulting scope correction, the initially untracked-target architecture
failure, and the corrected example target name are retained in the execution
journal. Documentation closeout and the normal development-only push are
recorded in that journal; no official repository, tag, binary, artifact
promotion, or release is authorized by this package.
