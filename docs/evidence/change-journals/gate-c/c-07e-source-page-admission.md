# C-07e: admission for retained source pages

**Status:** active evidence record; implementation complete, exact-candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07e-source-page-admission`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [79](https://github.com/rrflow/rrflow-development/pull/79)

## Baseline and change brief

Branched from development/main `c148465068f675ed70068b4b0aea60725e3718dc`,
with PR 72 dependency `dce14e51a224c891fe1568e5a226ef51bf6ef3db`.
The PR description preceded source edits and records the subsequent typed-error
boundary refinements before those edits. Version remains 1.0.0.

Cache residency did not bound allocations retained by readers after eviction,
or concurrent misses and uncached maintenance reads. The same `read_page` and
`finish_page` paths now reserve a per-open source allocation pool before raw
allocation/copy/decompression. `source_page_bytes` is required sealed storage
configuration (256 MiB default), lowered through the existing engine composition
boundary. Cache eviction can free cache owners; it cannot free a reader's
charge. Input and output are both charged during decompression. Immutable
Arrow owners retain charges through clones, slices, eviction and database
close. Raw mmap pages remain borrowed. Snapshot-input backing bytes, allocator
overhead and RSS are outside this payload-allocation gauge.

Current/peak/denied allocation diagnostics flow through the existing native
store evidence and public storage admission snapshot. Campaign retained-owner
sums include source pages. Native/store/query error types preserve admission
provenance. Known read-only query, hybrid, vector and Parquet paths can return
resource exhaustion. Generic durable error conversion remains uncertain:
source pressure may obstruct lookup of an already committed outcome. Only a
successfully observed absent outcome allows commit rejection and intent
clearance. Runtime traces classify this pressure with a bounded static label.

No segment format, checksum, snapshot retention, write acknowledgement,
projection authority, provider integration or lifecycle owner was replaced.
No source tree was moved/deleted. Stop conditions were format/retention changes,
an unbounded fallback, a second authority or unsupported completion claims.

## Traceability, reads and research

Existing `PageCache::complete_load`, `Segment::read_row_group_uncached`,
`read_page`, `finish_page`, output allocation ownership, database create/open,
read-only inspection, store error conversion, installed storage lowering and
public diagnostics supplied the behavior map. They remain their canonical
owners; source reservation extends acquisition rather than adding a fallback.
Characterization includes the existing cache, tiered-I/O, projected lifetime,
compaction/fault and acknowledged-history suites.

Read the README owner map, OBJ-02, Gate C/C-07, POAM-035 and change-authoring
procedure; examined `segment/mod.rs`, `segment/output.rs`, `reader.rs`,
`database.rs`, `inspection.rs`, `error.rs`, `lib.rs`, `rrflow_kv.rs`, store
`engine.rs`/`error.rs`, contract `deployment.rs`/`diagnostic.rs`, engine
`core.rs`/`diagnostic.rs`/`error.rs`/`transaction.rs`, query/retrieval/vector
adapters, `runtime/query.rs`, and the corresponding lifetime/admission tests.
Read the complete new `source_memory.rs`, `source_memory_tests.rs` and
`source_page_limits.rs`; reviewed changed source and generated-surface diffs.
The repository-generated inventory records exact changed paths; no parallel
implementation inventory is introduced here.

The locked Arrow 59.2.0 `MutableBuffer::from_len_zeroed` implementation supplies
exact allocation capacity; `Buffer::from_custom_allocation` preserves the
immutable owner. The existing projected-output owner supplied the local
lifetime pattern. No upstream database code or new dependency was imported.
This repair needs ownership/error evidence rather than external research.
Resource observations are allocation charges, not RSS. Debug capture uses
bounded counters and typed/static failure classes, never page payload traces.

## Failure and acceptance evidence

The first four focused native ownership tests passed. Two additional native
cases cover pressure eviction/malformed decode and zero-capacity rejection
before filesystem effects. The acknowledged-write fixture initially failed
at flush (128-byte source cap versus 556 bytes), exposing that the uncached
maintenance validator also acquires pages. The completed oracle now proves
that this denial leaves the prior manifest/value intact, and a sufficient
explicit reopen budget permits flush and exact readback.

A first compile exposed an overbroad error-match edit in `Error::source`; it
was corrected. An engine test incorrectly referenced the non-direct native
crate; it now tests through the store composition boundary. Frozen fixture
checks exposed the intentional configuration/OpenAPI/installation identity
changes. Their digests and shared SDK identity were updated from executed
owners, and evaluation/attunement goldens were regenerated with their existing
explicit update modes. No failing check or threshold was disabled.

Commands used the dedicated `target/c07e-source-page-build` build directory.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-lsm` | 191 passed, one existing child entry ignored; 67.23 s |
| `cargo test --locked -p rrd-engine --lib` | 158 passed, two existing ignored; 38.75 s |
| `cargo test --locked -p rrd-query` | 60 passed |
| `cargo test --locked -p rrd-contract` | 110 passed; 5.67 s |
| `cargo test --locked -p rrflow-eval` | 30 passed; 2.25 s |
| `cargo clippy --locked -p rrd-lsm -p rrd-store -p rrd-query -p rrd-engine --all-targets -- -D warnings` | passed; 39.52 s |
| `cargo fmt --all` | passed |
| `python -X utf8 scripts/ci/check_generated_surfaces.py --check` | passed; 43 HTTP operations |
| TypeScript `tsc --noEmit` and `tsx --test tests/client.test.ts` | passed; five tests |
| Go `gofmt -w client_test.go` and `go test ./...` | passed |
| `python -X utf8 scripts/knowledge/render_navigation.py`, `scripts/ci/build_execution_inventory.py`, `scripts/ci/check_documentation.py`, `git diff --check` | passed |

OpenAPI: `737574186cd6443f5aa108bf429c7a4e8b078d5cf48406472e4bd6038604b6de`.
Configuration: `d4f28eec17effe5d50947d8b6ca25c967e8c7f6cf31af6d284c2031de73444da`.
The installed bootstrap oracle verifies the fixed SDK anchor's new
instance/estate suffix `83fd04e75ef1faaa29bf3054`.

The first full engine suite had 157 passes and one stale SDK-anchor failure;
the corrected focused and full suites passed. Evaluation fixture failures
identified the sealed configuration digest, field ordering and progress-schema
changes. Existing `RRFLOW_UPDATE_STORAGE_GOLDENS` and `RRFLOW_UPDATE_GOLDEN`
owner generators regenerated evaluation and attunement fixtures, followed by
normal full-suite checks without update flags. No threshold was weakened.

Python 3.15 SDK execution was unavailable through this host's interpreter
manager; its constraint was not lowered. Pinned cached Ruff 0.17.1 rendered
the Python projection. The pnpm wrapper failed; its existing pinned 12.14.0
Node entry installed the frozen dependency lock from cache. Java/.NET SDK
execution and installed native pressure/capacity/soak for this candidate were
not run locally. They remain CI/profile requirements. Generated SDK output
was normalized to repository LF after Windows generator execution.

## Checklist and limits

- Owning outcome, exact baseline, scope, oracle and stop conditions were bound
  in the PR before edits; typed error propagation refinements were recorded.
- Source ownership and replacement behavior were mapped at existing boundaries.
- Allocation charges and read-only versus uncertain-effect classification are
  explicit; no private model reasoning or payload logging is recorded.
- Tests include concurrent owners, retained slices, file decode overlap,
  borrowed mmap pages, malformed decode, no-effect admission and exact reopen.
- Generated OpenAPI, SDK and evaluation projections remain derived from owners.
- A linked journal, POAM follow-up and regenerated navigation/inventory accompany
  this package. Candidate CI and the main-branch PR gate remain required.

No C-07, alpha or profile status changes. This package does not claim native
installed pressure qualification on all platforms, aggregate RSS proof, full
capacity profiles, competitive results or a completed 24-hour soak. Existing
scale/soak failures remain open under C-07g/J-04 and POAM-035; the earlier
candidate's native run is not acceptance evidence for this source change.
