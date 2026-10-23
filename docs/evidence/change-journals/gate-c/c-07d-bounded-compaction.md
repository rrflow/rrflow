# C-07d bounded immutable maintenance implementation

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-bounded-compaction`
**Owner:** OBJ-02, C-07d, POAM-035

[PR 65](https://github.com/rrflow/rrflow-development/pull/65) bound this continuation
before code edits, from `4dc1b43764c4f48f35f532ca2a575811e8587cb8`, tree
`18110ec35c154a62eae3c9d48f4fa4ce220973a8`. The owner directed implementation
without more planning-only packages or repeated local runtime suites.

## Change brief and traceability

Before this change, immutable compaction bounded segment count and used a soft
output target. One hot key could accumulate its entire history without a hard
input/work cap. Failed automatic compaction after a flush emptied the memtable,
so another attempt could admit writes without retrying that maintenance until
the next flush. Existing decoded-value pruning also cloned retained values.

`rrd-lsm::CompactionLimits` now bounds physical input bytes, authenticated decoded
page bytes, input versions, output partitions, cooperative elapsed milliseconds
and L0 segment count. Metadata admission precedes cursor decoding. Every record,
partition and pre-publication boundary observes the cooperative deadline.
Pruning moves the selected version values; protected-version semantics remain
unchanged. Write admission checks saturated L0 even with an empty memtable,
and explicit flush checks the prospective L0 count before output creation.

`MaintenanceLimit` remains typed through rrd-store and the engine's existing
storage-admission response. Only an absent domain outcome plus this known
pre-append rejection permits intent cleanup; unknown I/O remains uncertain.
The required installed StorageLimits, sealed configuration, locator projection
and checked core lowering carry all six controls through the existing install,
reopen, inspection and restore composition. Product version stays 1.0.0; this
extends the still-unmerged format-3 configuration candidate.

Defaults: 1 GiB physical input, 256 MiB logical input, 4,194,304 versions,
256 output partitions, 30 seconds cooperative elapsed budget, 64 L0 segments.
Logical accounting excludes allocator overhead/RSS. Deadline checks cannot
interrupt OS I/O. Unpublished partitions remain owned by existing validated
GC. Higher-level debt, aggregate retained generations, disk growth and installed
reclamation are not proven by these controls.

Implementation review covered the complete new maintenance_limits module;
DatabaseOptions validation/construction, prepare_write, flush_memtable_inner,
compact_inner, selection, merge_compaction_candidate and partition writing;
SegmentRecordCursor and authenticated page descriptors; native/store/engine
error mappings and transaction absent-outcome handling; installed policy
validation/defaults/digest/lowering and its existing restore callers. The
existing compaction test target preserves characterizations and adds independent
hot-key history/reopen and repeated-write pressure cases. These were compiled,
not executed locally. No implementation was moved to another authority.

Owner and supporting reads: README, alpha objective OBJ-02, roadmap C-07,
POAM-035, change-authoring/traceability procedure and CI contract. Reviewed
fixture sources include attunement contract generation, installation bootstrap
identities, installation inventory/identity seed, shared SDK harness and five
language capability fixtures, evaluation storage model/validation/schema and
synthetic fixture construction. Changed-source and generated diffs were reviewed.
The execution inventory supplies exact changed paths and hashes.

Research/source adaptation: no upstream algorithm/code was imported; this
extends RRFlow's existing synchronous publication authority. Trace/debug/resource
decision: preserve counters and typed errors, expose limits through the existing
configuration capability, and make no heap or native capacity claim. Stop on
unsafe pruning, early ACK, ignored caps, a second scheduler or failed candidate CI.

## Failures repaired and commands

Candidate CI `39880104913` at the starting revision exposed stale attunement
configuration/plan/result hashes, bundled profile identities, SDK corpus identity,
and evaluation schemas/synthetic fixtures. Engine-authority, product-operations,
SDK conformance, optional-engine and macOS estate lanes failed on those assertions.
These failures were read from the original job logs, not rerun until green.

Repairs preserve the assertions. Attunement digests follow the canonical Rust
serialization grammar and the native ordered JSON captured in that failed CI.
SDK identity is derived from the fixed anchor, empty inventory, bundled profile
and effective configuration; the derivation reproduces the previous candidate's
observed identity before advancing the fixture. Java uses Map.ofEntries because
the expanded storage object exceeds Map.of's ten-entry overload. A C# fixture
syntax error introduced during editing was corrected during diff review.

The declared `rrflow-eval` example `refresh_storage_contract` renders existing
synthetic inputs and schemas with canonical serializers/digests; ledger inputs
are preserved, non-synthetic input is refused, and it runs no native workload.
The evaluation README documents the command. Current configuration digest is
`a8c50eaf57e5a749eb7a6f65bc0a61ffff21b771044f7225789bf0d6ca63324c`;
OpenAPI is `7ef6b88399359d1075138e328d910ca301e8de360cbd19cc6c5f993c3314c8f0`.

Executed commands/results:

- `cargo check -p rrd-lsm -p rrd-store -p rrd-contract -p rrd-engine -p rrflow-cli --all-targets --locked`: passed, 19.43 seconds.
- `cargo clippy -p rrd-lsm -p rrd-store -p rrd-contract -p rrd-engine -p rrflow-cli --all-targets --locked -- -D warnings`: passed, 14.77 seconds.
- Final `cargo clippy -p rrd-lsm -p rrd-store -p rrd-contract -p rrd-engine -p rrflow-cli -p rrflow-eval --all-targets --locked -- -D warnings`: passed, 20.71 seconds, including the new oracle definitions and fixture renderer.
- `cargo fmt --all`, `gofmt -w client_test.go`: completed.
- Existing five-language generators: TypeScript via Node 24.22/tsx;
  Python via `uv run --no-project --with ruff==0.17.1 python sdks/python/scripts/generate.py`;
  Java/.NET via their checked-in Python scripts; Go via `go run ./cmd/generate`.
  All completed; generated Windows newline differences were normalized to LF.
- `cargo run --quiet --locked -p rrflow-eval --example refresh_storage_contract`: rendered synthetic projections only.
- `python -X utf8 scripts/ci/check_generated_surfaces.py --write` and `--check`: passed, 42 operations and seven signal projections.
- `python -X utf8 scripts/ci/check_documentation.py`: passed before journal/index regeneration; final recorded generation/check commands accompany the commit.
- `python -X utf8 scripts/knowledge/render_navigation.py`: 17 indexes, 427 nodes, 1,853 edges.
- `python -X utf8 scripts/ci/build_execution_inventory.py`: 1,428 path records.
- Final documentation policy: passed, 428 statuses and 426 classified coordinates.
- `git diff --check` and staged diff checks: passed.

No local runtime suite, installed workload, provider evaluation, benchmark,
native filesystem exhaustion or soak was run. Compilation and synthetic rendering
are not behavioral acceptance. The current candidate must pass mandatory CI
before merge. C-07 and POAM-035 remain open; no roadmap completion status changes.

## Change checklist

The pre-effect PR brief names OBJ-02/C-07d, baseline, exact boundaries, failure
oracles and stop conditions. Existing owners retain their authority; policy
fields are required rather than silently defaulted by clients. The hot-key
oracle requires identical protected reads after rejection and reopen; the L0
oracle requires repeated rejected attempts to leave sequence/state unchanged.
Both await execution in candidate CI. Journal/index/inventory are regenerated
before commit. Failed baseline CI and unrun checks remain explicit. No promotion
to the official repository is authorized by this package.
