# C-07g: addressed record and relation queries

**Status:** active evidence record; implementation complete, native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-addressed-query-reads`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [82](https://github.com/rrflow/rrflow-development/pull/82)

## Brief, baseline and traceability

Starting revision `2ff045edd209728a8c15b69ea4824e2f3767b14e` integrates
the frozen combined candidate `d209218da735b26197ec9feeb3c1873eb480383b`.
The pre-implementation PR brief bound the C-07g scale prerequisite to existing
C-04 identity authentication and E-01 query planning. Version stays 1.0.0.

Single-source record/relation queries with exact ID equality previously loaded
their entire typed family before filtering. The installed corpus verification
performs such a record query for every item. Binding now selects the existing
`RuntimeVersionedSource::Identity` using the stamped schema's logical model and
the resolved equality value. The original predicate and every other predicate
remain in DataFusion. Existing authenticated history, retirement, known/valid
time, key budgets and row materialization remain authoritative. The existing
identity path explanation also covers record/relation IDs and takes precedence
over broader index candidates.

`MATCH` retains family scope because BM25 statistics depend on the corpus.
Invalid runtime-ID strings retain their previous comparison semantics rather
than becoming binding errors. Joins, non-equality filters, claims, traversals,
event cursor behavior and relation endpoint queries keep their existing paths.
No new operator, format, index, cache, schema or API was introduced. POAM-034's
separate whole-scope commit constraint cost remains E-01/E-02 work.

Source replacement is confined to `rrd-query/src/plan.rs` binding and path
selection. `execute.rs::ReadPath::Versioned` and `rows_for_source` continue
execution and temporal materialization; `rrd-store` owns authenticated ranges.
The characterization lives in `tests/query/identity_reads.rs`, registered by
the existing query integration target. No implementation was moved/deleted.

Read README owners, C-07/POAM-035 and POAM-034, the change-authoring routine,
query `plan.rs` binding/planning/source and field routines, `execute.rs` path
validation/loading/row materialization and BM25 assembly, `catalog.rs` source
coverage, `syntax.rs` source/comparison forms, core logical-model/retirement
types, existing query identity/temporal fixtures and the installed corpus
`workload.rs::verify_key`. Reviewed the complete changed plan and new test file.
The repository inventory owns the exact file projection.

Research decision: use the existing local authenticated identity contract; no
new upstream algorithm or dependency. Resource/debug decisions: retain the
caller budget, physical read-evidence counters and explicit chosen-path
explanation. No payload logging or private reasoning schema. Stop conditions:
temporal/authentication regression, new public model, weakened bounds, hidden
constraint-validation cost or unsupported completion.

## Failure and acceptance evidence

Dedicated existing build directory: `target/c07g-addressed-build`, used in
the same worktree path as the prerequisite candidate.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-query --test query addressed_id_queries` before implementation | Failed at physical key 129 under a 128-key budget after unrelated growth |
| Same focused command after implementation | Passed; 7.86 s |
| `cargo test --locked -p rrd-query` with final additional regressions | 61 passed; 49.19 s including build |
| `cargo clippy --locked -p rrd-query --all-targets -- -D warnings` | Passed; 32.25 s |
| `cargo test --locked -p rrd-engine --lib query` | Six passed; 24.30 s including build |
| `cargo fmt --all` | Passed |
| Navigation renderer, execution inventory, documentation policy and `git diff --check` | Passed |

The differential oracle compares exact selected rows with a family-read
reference across 96 unrelated records and 96 unrelated relations, updates,
endpoint changes, retirement, historical known/valid coordinates, missing IDs,
additional predicates and native reopen. Vector-family changes are outside
this package. Record-family examined/decoded counts stay unchanged as unrelated
objects grow. Parameterized equality selects the same identity path; exhausted
budgets remain explicit. Empty-ID and non-equality cases preserve behavior;
`MATCH` is checked to retain family scope. The owning suite also covers joins,
graph traversal, lexical/index paths, protocol equivalence and export.

The test helper's key-budget type was corrected to the actual `u64` contract
before its first compile completed. No test threshold or expected semantic
result was weakened. Logs are `target/c07g-query-red.log`,
`c07g-query-green.log` and `c07g-query-suite.log`.
Integration/lint logs: `target/c07g-query-engine.log` and
`target/c07g-query-clippy.log`. No generated public contract changed.

## Checklist and remaining acceptance

- Bound outcome, source baseline, exact scope, failure oracle and stop
  conditions in the PR before edits.
- Mapped replaced family reads to canonical authenticated identities and
  preserved the full temporal predicate evaluation.
- Recorded the actual budget failure, correction, differential history,
  resource/debug/research decisions and full changed-file review.
- Linked this journal from the POAM and regenerated navigation/inventory;
  strict query lint, documentation and diff checks passed.
- Keep the prior Linux larger-corpus and Windows 24-hour runs frozen at
  `d209218d`; their evidence does not claim this source was executed. Native
  comparison, complete profiles, soak and exact candidate CI remain required.

C-07, OBJ-02 and POAM-035 remain open. No throughput, competitive or alpha
completion is inferred from component correctness. Relation endpoint reads and
whole-scope commit constraints are not solved by exact ID source selection.
