# C-07g: authenticated identity intervals

**Status:** active evidence record; component implementation complete, pipeline and native qualification open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-identity-range-reads`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [84](https://github.com/rrflow/rrflow-development/pull/84)

## Change brief and traceability

Baseline `df324644e6f6102687906da6a9d725f8e7bd415f` integrates PR83.
The pre-edit PR brief binds the C-07g range workload to the existing C-04
authenticated storage and E-01 planner. Version remains 1.0.0.

Record and relation ID inequalities previously scanned their whole typed
family. `RuntimeVersionedSource::IdentityRange` now selects complete histories
inside an inclusive/exclusive lexical interval. It uses the existing ordered
reference keys, one stamped read, key authentication and physical-key budget.
The planner intersects repeated valid bounds, preserves all residual filters,
and reports `versioned_identity_range_read`. Contradictory or invalid-ID
comparisons preserve the family fallback; equal endpoints with a strict side
select an empty range. Joins and MATCH preserve their existing corpus paths.

The canonical store source admission now compares actual half-open physical
intervals, admitting adjacent intervals, deduplicating identical sources and
rejecting overlap in either order. Returned identities must match the logical
family, kind and bounds as well as their exact encoded key and accumulator
proof. No new disk format, index, cache, operation or runtime authority exists.
Internal source serialization adds the typed variant; public query syntax and
operation models are unchanged. No implementation was deleted or moved.

Production scope: `rrd-store/src/access/versioned_read.rs` source, interval
admission/scan/validation; query `plan.rs` binding/explanation, `execute.rs`
path validation and `catalog.rs` source coverage. New tests:
`rrd-store/src/access/versioned_identity_ranges.rs` and
`rrd-query/tests/query/identity_ranges.rs`, registered by their existing owners.
This journal, POAM-035 and generated navigation/inventory carry the evidence.

Read the README/owner chain, C-07 and POAM-035/034, change-authoring procedure;
store key codec/reference-prefix grammar, source range admission, metered
scan/authentication and entry validation; core RuntimeId validation; query
binding, source coverage, physical path validation and temporal materialization;
installed workload RangeRead and the existing identity/history tests. Reviewed
the complete changed source diff and both complete new test files.

Research decision: reuse the repository's memcomparable string grammar and
authenticated source contracts; no upstream code or new dependency. Keep
existing physical read counters and plan explanations, caller limits and
payload-free diagnostics. Stop on ordering mismatch, incomplete history,
broadened authority, changed lexical semantics, relaxed bounds or unsupported
qualification. POAM-034's write constraints remain a separate open problem.

## Executed acceptance

Build directory `target/c07g-addressed-build` stays with the same worktree path.

| Command | Actual result |
|---|---|
| `cargo test --locked -p rrd-query --test query identity_range_read_cost` before implementation | Failed: key 129 exceeded the fixed 128-key budget after unrelated growth |
| Same command after implementation | One passed |
| `cargo test --locked -p rrd-query --test query identity_ranges` | Three passed; 1.73 s test time |
| `cargo test --locked -p rrd-store --lib identity_range_tests` | Two passed |
| `cargo test --locked -p rrd-store -p rrd-query` | 284 passed across both owners; 94.58 s including build |
| `cargo clippy --locked -p rrd-store -p rrd-query --all-targets -- -D warnings` | Passed; 7.44 s |
| `cargo test --locked -p rrd-engine --lib query` | Six passed; 31.19 s including build |
| `cargo fmt --all` | Passed |

The differential histories cover records and relations, updates, retirement,
known/valid coordinates, memory/native stores and native reopen. Bound tests
cover strict/inclusive/one-sided/equal endpoints, repeated predicates,
parameters, missing/contradictory predicates and budget denial. Physical-key
tests enumerate prefix strings, eight-byte tuple boundaries, quotes, backslash
and Unicode, reject out-of-range entries and check source serialization.
All existing query/index/graph/store suites remain green. Logs are
`target/c07g-range-{red,green,focused,store-focused,suites,clippy,engine}.log`.

## Discovered pipeline gap and remaining acceptance

While reviewing the public query route, inspection found that
`StampedQueryPipeline::bind` first calls `Catalog::capture_for_query_at`, which
loads the entire requested source family to derive watermarks. The bounded
execution fixes therefore do not yet make the public pipeline bounded by the
addressed identities. This is a separate binding/capture package; changing its
watermark/index and aggregate-budget semantics is outside this interval brief.
The next package must reproduce the gap through the actual pipeline and remove
that preflight scan without weakening exact-index or historical semantics.

Linux campaign05 at `d209218d` committed 1,000 corpus items but verified only
950 before the frozen 30-minute setup deadline. It failed and cleaned up;
raw evidence is retained at `target/c07-linux-scale-evidence-05`. A Linux PR83
release build and two resource-window tests completed, but a new 30-minute
campaign is intentionally not spent on that known-incomplete pipeline. Native
comparison follows the pipeline repair. The separate Windows campaign14 stays
frozen; an in-progress requested 24 hours is not a completed soak.

Checklist: pre-edit brief, baseline/scope/traceability, red oracle, focused and
owning checks, source/test review, resource/research decisions and known gaps
are recorded. Navigation/inventory and documentation checks precede commit.
Exact candidate CI/merge, complete profiles, soak and C-07h reconciliation remain
required. C-07, OBJ-02, POAM-035 and alpha status are unchanged and open.
