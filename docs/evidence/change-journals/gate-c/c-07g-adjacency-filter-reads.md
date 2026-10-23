# C-07g: addressed relation endpoint queries

**Status:** active evidence record; implementation complete, native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-adjacency-filter-reads`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [83](https://github.com/rrflow/rrflow-development/pull/83)

## Brief, baseline and traceability

Starting revision `d3aad85f1d2735670df9254a7904488352493a30` integrates
the addressed query candidate `dab8f747ca2f8b382fe6b003ce5fbea625d62d06`.
The pre-implementation PR brief binds this installed capacity prerequisite to
the existing C-04 authenticated adjacency/identity substrate and E-01 planner.
The product version stays 1.0.0; no gate changes status.

Relation endpoint equality previously read the complete relation family before
filtering. The planner now selects an outgoing or incoming adjacency range when
the endpoint ID and kind uniquely identify a node. Kind comes from an explicit
equality or a single allowed kind in the stamped relation schema. Execution
expands the named relations to their complete authenticated identity histories
before temporal materialization and residual DataFusion filtering. Every phase
shares the read stamp and aggregate physical-key budget. This closure prevents
an old endpoint from resurrecting a moved, backdated or retired relation.

Exact relation ID equality takes precedence. Ambiguous or unrestricted endpoint
kinds, non-equality filters, joins and MATCH retain their existing paths. MATCH
requires corpus-wide lexical statistics. Invalid reference strings do not
introduce a new bind error. No new operation, API, schema, index, cache, format,
runtime or authority was introduced. Whole-scope commit constraint validation
remains the separate POAM-034 E-01/E-02 problem.

Production changes are confined to `crates/compute/rrd-query/src/plan.rs`
(`bound_version_sources`, adjacency selection and path explanation) and
`src/execute.rs` (`ReadPath::Versioned` validation and loading). Existing
`rrd-store::read_versioned_expanding` authenticates each range before expansion,
deduplicates observations and enforces the shared budget. Existing
`rows_for_source` materializes history. Graph traversal and the store are
unchanged. The new `tests/query/adjacency_reads.rs` is registered by the existing
query integration target. No implementation was moved or deleted.

Read the README owner chain, C-07, POAM-035 and POAM-034, the change-authoring
procedure, query planning/binding and execution/materialization, the store
versioned-read expansion contract, graph identity expansion, core relation
schema/retirement types, query fixtures and installed `workload.rs::verify_key`.
Reviewed the complete production diff and the full new regression file.
Generated inventory retains the exact changed-path projection.

Research decision: reuse the local authenticated range contract; no upstream
algorithm or dependency is being adapted. Resource/debug decisions: preserve
physical read counters, explicit path explanation, caller budgets and existing
authentication. No payload logging or private reasoning schema. Stop conditions
are incomplete temporal closure, changed authorization, relaxed bounds, hidden
family reads, a new authority or unsupported completion claims.

## Failure and acceptance evidence

Dedicated build directory: `target/c07g-addressed-build`, reused only by the
same worktree path. Native campaigns remain separately frozen.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-query --test query endpoint_filter` before implementation | Both regressions failed: physical key 129 exceeded the 128-key budget; explicit endpoint still selected a family |
| Same focused command after implementation | Two passed; 19.76 s test time |
| `cargo test --locked -p rrd-query` | 63 passed; no failures |
| `cargo clippy --locked -p rrd-query --all-targets -- -D warnings` | Passed |
| `cargo test --locked -p rrd-engine --lib query` | Six passed; no failures |
| `cargo fmt --all` | Passed |
| Navigation, execution inventory, documentation policy and `git diff --check` | Passed |

The family-read differential oracle covers outgoing and incoming endpoints,
96 unrelated records and relations, endpoint moves, backdated updates,
retirement, historical known/valid coordinates, missing nodes, native reopen
and memory/native stores. Physical read counts remain stable under unrelated
growth. Colliding node IDs across two permitted kinds retain family scope until
an explicit kind disambiguates them; each explicit query returns the correct
relation. Exact-ID precedence, non-equality and lexical fallback are checked.
The owning suite covers graph, joins, index/lexical paths and export.

Logs: `target/c07g-adjacency-{red,green,suite,clippy,engine}.log`. No threshold
or semantic expectation was weakened. The completed engine check reports six
passes; a later attempt to poll its already-closed process ID was a tool error,
not an additional test execution or a test failure.

## Checklist and remaining acceptance

- Bound the outcome, baseline, exact scope, failure oracle, research/resource
  decisions and stop conditions in PR83 before implementation.
- Mapped family-read replacement to existing authenticated adjacency and full
  identity histories; retained temporal filtering and one budget authority.
- Recorded both initial failures and the corrected differential histories;
  reviewed production and test changes and linked this journal from POAM-035.
- Regenerated navigation and inventory; documentation and diff checks passed.
- Keep native runs at `d209218d` frozen; they do not execute this source.
  Native comparison, complete capacity profiles, 24-hour acceptance and exact
  candidate CI/merge remain required and are not inferred from these tests.

C-07, OBJ-02 and POAM-035 remain open. This package implements endpoint source
selection; it does not establish throughput, whole-scope write scalability,
competitive performance or a completed alpha.
