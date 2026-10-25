# C-07 execution contracts and DuckDB integration plan

**Status:** active planning evidence; runtime implementation and qualification remain open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07-duckdb-execution-plan`
**Owner:** OBJ-02 prerequisite, [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md)

## Baseline and scope

Starting revision `ecfeea332e6d7a0138b2dbb6c393e0d734d50db9`, tree
`a1901442d72742d3941f8d1e998fdde43e67af0c`; clean public development main.
[PR 57](https://github.com/rrflow/rrflow-development/pull/57) carried the
pre-implementation brief after the linked
[baseline seed](c-07-duckdb-planning-baseline.md). The branch is
`agent/c07-duckdb-plan`; exact remote/ref/revision push records are retained
locally in `target/c07-duckdb-pushes.jsonl` and the public PR history.

Current-to-target change: refine C-07c-h into explicit installed history,
resource ownership/admission, lifetime, native capacity, sustained-run and
acceptance contracts; bind the requested DuckDB capability to D-06 instead of
silently adding a second native store. Export interoperability precedes source
import and optional contained worker execution. No runtime implementation,
dependency, public schema, persisted format or version changes occur here.

## Reviewed inputs and changed paths

Complete records reviewed: AGENTS; alpha objective and release-roadmap portal;
Gate F; dependency spine; knowledge connectors; POA&M parent/governance/033/035;
change-authoring and implementation-navigation; architecture skill;
`crates/authority/rrd-engine/src/engine/query_export.rs`;
`crates/compute/rrd-query/Cargo.toml` and
`crates/adapters/rrflow-cli/Cargo.toml`. README and C-07 were read in sections;
Gate D's connector/dependency and installation-acceptance sections, the
independent-engine bootstrap requirements, CI topology, transaction intent,
definitive-rejection and receipt-replay branches, installed lifecycle/process
fixtures and storage evaluation API inventory were inspected. Truncated
combined outputs are not treated as unseen source inspection.

Created the DuckDB reference and POAM-036; updated README warp, data reference
index, knowledge-connector link, D-06 package table, C-07 implementation
contracts, dependency spine and parent POA&M. Generated POA&M/journal indexes
and execution inventory retain their mechanical role. No implementation is
moved, deleted or replaced, so replacement-characterization evidence is not
applicable. The already accepted C-07b PR/CI/merge is linked, not requalified.

## Research, trace and resource decisions

The [DuckDB reference](../../../reference/data/duckdb.md) links official Rust,
security, concurrency, limits and Parquet sources and records the version
discrepancy encountered in current documentation/search results. No library
version is guessed, dependency added, upstream source adapted or vendor
performance result transferred. An actual pinned native client/build corpus
is required in its later implementation PR.

Runtime traces and resource measurements are not applicable to this
documentation-only change. Their requirements are concrete in c-g and the
DuckDB contract: bounded test rendezvous absent from release artifacts,
independent acknowledgement history, aggregate reservation/RSS/temporary
space, native capacity error, retained buffers, cleanup and a real 24-hour run.
The plan preserves existing engine authority and generated public contracts.

## Validation and surfaced failures

- `cargo test -p rrflow-eval --test storage_contract --locked`: 11 passed,
  zero failed/ignored; baseline characterization only.
- `python -X utf8 scripts/knowledge/render_navigation.py` and
  `python -X utf8 scripts/ci/build_execution_inventory.py`: regenerate changed
  projections; both `--check` commands passed with 17 indexes, 389 nodes,
  1,706 edges and 1,370 inventory records.
- `python -X utf8 scripts/ci/check_documentation.py` initially rejected the
  missing manual data-index link, POAM title mismatch, missing canonical POAM
  sections and missing direct gate-owner links. Those were corrected; the
  final check with this journal passed: 390 statuses, 388 classified coordinates.
- `git diff --check`: passed. Changed prose and its diff were reviewed; generated
  digests/navigation are checked by their owning generators.

One exploratory PowerShell brace-expansion command and one wildcard-path
search failed and were corrected without mutation; the baseline retains this.
No source code was changed, so compilation, Clippy, full local workspace tests,
native crash/ENOSPC/soak runs and DuckDB conformance were not run for this
package. Candidate CI still gates merge and must bind the exact final revision.
Its run and merge evidence are recorded in PR 57, not predicted here.

## Checklist and remaining work

The brief binds OBJ-02/C-07/POAM-035 plus the requested D-06 interoperability
prerequisite, current/target behavior, exact documentation scope, first oracle,
research and trace/resource decisions, replacement non-applicability and stop
conditions. The baseline was synchronized; unrelated changes were absent;
there is one canonical reference per detailed subject. No implementation or
availability claim is inferred from this plan. Final projection/ownership and
whitespace checks precede commit, and exact-candidate CI precedes merge.

Status change: new POAM-036 is **Sequenced**. C-07, POAM-035 and every alpha
outcome remain open/partial. C-07c is the next implementation; c-h and DuckDB
support are not implemented by this package. No model reasoning, PostgreSQL
compatibility, cloud sync or competitive performance is claimed. Stop on a
second authority, unavailable native enforcement, runtime fetch, unreviewed
format change or a failed required oracle; do not weaken an exit to pass it.
