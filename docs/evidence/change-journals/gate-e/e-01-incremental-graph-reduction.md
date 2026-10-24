# E-01 prerequisite: incremental temporal graph reduction

**Status:** active evidence for an existing-path repair; Gate E remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-e/e-01-incremental-graph-reduction`
**Owner:** [Gate E](../../../roadmap/rrflow-1.0/gate-e.md), OBJ-04 prerequisite and [POAM-005](../../../poam/rrflow-1.0-alpha/poam-005.md)
**Pull request:** [42](https://github.com/rrflow/rrflow-development/pull/42)

## Baseline and change brief

Synchronized clean development/main at
`3faa6752c9ff84e785254fd8112a77a81c5b0894`, tree
`25c49dbb08343eccba5ccdd65b046a5a5600741a`. Empty anchor
`8c72a6ebd57d5a1a70811e9c016bfe6fa162a3ef` established the PR description
before source edits. This bounded repair removes repeated work in an existing
query path; D-01 remains the active dependency-spine item.

Previously, every breadth-first advance replayed and cloned all accumulated
authenticated graph history and cloned the entire visited-node set. Replace
that with one temporary temporal reducer per request and insert only newly
reached nodes into the visited set. Expansion and final row projection share
the reducer; no second storage or semantic authority is introduced.

Preserve authenticated step reads, aggregate storage-key budget, scope,
known/valid time, full relation-identity history, deterministic traversal
ordering, joins, persisted formats, schemas and public APIs. Stop on semantic
drift, a new authority, an undeclared dependency, changed authorization or
failed candidate CI. No version, dependency, runtime fetch or vendor engine
was added.

## Implementation traceability and exact scope

| Existing behavior/source | Canonical destination | Replacement evidence |
|---|---|---|
| `physical/graph.rs::AdjacencyExpansion` retained a duplicate change map and replayed it at every depth | Request-local `physical/graph/state.rs::GraphState`; apply only supplied changes and look up frontier relations directly | Deep-chain mutation-visit regression and independent graph integration oracle |
| `execute.rs::active_graph_values` reduced ordered history | Same shared private reducer, consumed into record/relation maps | Chronological independent replay across 594 temporal/order combinations and complete query suite |
| Expired versions or retirements suppress earlier state; later reads may deliver smaller cursors | Highest eligible cursor plus optional live value per identity; expired/retired identities retain their cursor | Backwards valid-time changes, moved endpoints, future/expired versions, retirement/re-put, duplicate versions, other scope and known-time filters |
| Each advance cloned all previously visited nodes | Existing walk, with a set containing only newly reached nodes, then `extend` | Existing directed/typed/depth/loop graph oracle; traversal order unchanged |
| Durable graph queries had in-memory/durable parity but this target lacked explicit close/reopen repetition | Existing `tests/graph_traversal.rs`, one additional reopen corpus | Eighteen temporal/directional cases checked against independent whole-log oracle before and after rrflowKV close/reopen |

Created `src/physical/graph/state.rs` and `src/physical/graph/tests.rs` under
`crates/compute/rrd-query`; changed its `src/physical/graph.rs`, `src/execute.rs`
and `tests/graph_traversal.rs`. No production file was deleted or moved.
The chronological test oracle deliberately retains independent simple replay;
it is test evidence, not a production fallback.

Supporting changes: extend `docs/architecture/engine-scale-and-contracts.md`
with primary-source graph/vector/knowledge mapping and a relationship diagram;
update POAM-005's observed repair and its stale vector-catalogue sentence;
create this journal; regenerate nearest navigation/relationship projections
and the execution file inventory. No objective, roadmap status or planning
order changed.

## Research, trace, resource and debugging decisions

The [scale rationale](../../../architecture/engine-scale-and-contracts.md#graph-vector-and-knowledge-research-mapped-to-rrflow)
records current primary links and maps them to existing owners. HydraDB's
published pinned-snapshot/request-local topology reuse supports disposable
acceleration with exact visibility. SurrealDB's HNSW versus disk-index and
memory accounting descriptions inform E-04/F-04 comparison requirements.
Pinecone's slab article informs generation/delta and metadata-selectivity
experiments; Nexus's separate knowledge product informs incremental cited
knowledge under D-05/G. These are design inputs, not independent performance
proof. No upstream implementation was copied, no vendor default selected, and
no product-level competitive claim established.

The attempted HydraDB Cortex paper URL returned 404 and the SurrealDB
whitepaper URL led to a gated landing page. Neither is cited as a paper read.
The current public architecture and index documentation supplied the usable
technical sources. HydraDB's public AI page links its open-source engine and
separately describes the hosted context API; they were not treated as one
interchangeable connector contract.

Test-only mutation-visit instrumentation measures the demonstrated reduction
cost. Existing public storage access evidence and authentication remain
unchanged; no new signal schema or prompt-content trace is needed. The reader
still accumulates selected authenticated history, final projection still
materializes rows, and path construction remains depth-sensitive. This is
neither a whole-query linear-time/RSS guarantee nor a fixed-hardware benchmark.
Equal-cursor versions are already authenticated and conflict-checked by the
store before reduction; the reducer does not replace that integrity check.

## Verification and surfaced failures

- Before the production replacement, `cargo test -p rrd-query --lib deep_expansion_reduces_each_supplied_change_once --locked -- --nocapture` failed as intended: depth 8 required 45 temporal mutation visits for 17 supplied versions. No compiler failure was substituted for the oracle.
- `cargo test -p rrd-query --lib physical::graph::tests --locked -- --nocapture`: 2 passed. Depths 8/32/64 reduced 17/65/129 supplied versions with exactly 17/65/129 mutation visits; temporal comparison covers 9 valid times, 6 known cursors and 11 reordered/duplicated histories.
- `cargo test -p rrd-query --test graph_traversal --locked`: 4 passed, including seeded independent traversal, joins, physical storage evidence and durable reopen. Existing physical-key counts remain independent of 400 unrelated graph entries.
- `cargo test -p rrd-query --all-targets --locked`: 60 passed, zero failed, on the final source. `cargo clippy -p rrd-query --all-targets --locked -- -D warnings`: passed. The initial package run also passed; final source was rerun after the visited-set adjustment.
- `python scripts/knowledge/render_navigation.py --check`, `python scripts/ci/build_execution_inventory.py --check`, `python scripts/ci/check_documentation.py`, `python scripts/ci/check_workflow.py`, `cargo fmt --all -- --check`, and staged diff whitespace checks passed. `python scripts/ci/test_execution_inventory.py`: 4 passed; `python -m unittest discover -s scripts/knowledge -p test_*.py`: 17 passed. Navigation projects 17 indexes, 348 nodes and 1,532 edges; inventory projects 1,285 paths.

The first source-only push resolved the feature branch's tracked destination
to `refs/heads/main` and was rejected by GH013 (PR and required CI rules).
Nothing landed on main. The local receipt was corrected to record the actual
rejected destination; subsequent pushes use an exact SHA and fully qualified
feature-branch destination, after resolving the development URL. The official
repository was never a destination. Empty-anchor CI run `39517986903` was
cancelled; it is not acceptance evidence.

Review caught avoidable whole-set merging via `BTreeSet::append`; it was
replaced with insertion of newly reached nodes via `extend`. The final suite
was rerun after that change. Temporary Windows CRLF churn was normalized back
to tracked LF before final review. Exploratory reads of nonexistent physical
module-root files were corrected by inspecting the inline module in `lib.rs`.

## Files read and final review

Read in full: `AGENTS.md`; change-authoring and implementation-navigation
procedures; Gate E; POAM-005; scale rationale; query `Cargo.toml` and `src/lib.rs`;
original and final `src/physical/graph.rs`, `tests/graph_traversal.rs`;
final `src/execute.rs`, `src/physical/graph/state.rs`, and
`src/physical/graph/tests.rs`; this journal. Relevant sections additionally
read: README identity/warps; alpha outcome table; dependency spine; Gate F;
CI policy; `catalog.rs` source normalization; store
`access/versioned_read.rs::read_versioned_expanding`; core
`runtime.rs::RuntimeChange`; prior embedding journal as evidence format.
The complete final diff and generated projections are reviewed before commit.

## Change checklist and remaining work

- Owner/outcome, exact scope, baseline, invariants and stop conditions bound in PR before implementation.
- Existing behavior mapped to one canonical replacement and independent oracles.
- Primary research, source-adaptation exclusion, work counter and trace decisions recorded.
- Reproduced failure retained; focused, package and policy results reported separately.
- New files, supporting records and generated projections reconciled; no unrelated changes.
- Final full-file/diff review completed before commit; exact candidate CI required before merge.

No local full-workspace, native macOS/Linux, process-kill/fault, sanitizers,
fixed-hardware latency/RSS/recall or competitor trials were run. Cross-workspace
and platform candidate lanes are delegated to repository CI, not presumed
passed. This package adds no vector algorithm, persistent compiled graph,
new planner, model controller or PostgreSQL support. Context/seat snapshot
materialization, complete memory accounting, native index convergence and
competitive qualification remain owned by POAM-005 and E/F/J. D-01 and all
alpha outcomes keep their existing status.

## Pull request, CI and merge evidence

The immutable journal records local evidence at authoring time. PR 42 carries
the exact final candidate SHA, candidate workflow/check results, and merge
receipt after CI. All required candidate checks must pass before merge into
public development main. No official promotion or release is authorized here.
