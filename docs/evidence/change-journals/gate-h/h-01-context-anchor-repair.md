# H-01 temporal context-anchor repair

**Status:** active immutable implementation evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-01-context-anchor-repair`
**Owner:** OBJ-06/H-01 recall correctness prerequisite; POAM-010

PR: <https://github.com/rrflow/rrflow-development/pull/31>.
Starting implementation revision: `4fb47dd5baa1b941c3d1b2c7259618117dc4dafb`.
The [red regression receipt](h-01-context-anchor-failure.md) records the original
`ccad7869c9d2fc2a277e7c723e17428e79d2dab7` baseline, scope, inspection and failure.
The PR brief existed before production edits. This package repairs existing
native recall independently of any DevPulse checkout or cloud service.

## Implementation and traceability

`crates/authority/rrd-engine/src/engine/context.rs`: `assemble_context_at` now
collects visible seed records while resolving seed evidence, appends visible
retrieval roots once in existing order, and gives that same root set to graph
selection, explanation and `add_graph_context`. The graph walker no longer
reconstructs roots from unfiltered caller seeds. This replaces the incorrect
root boundary directly; the snapshot, relation-path semantics, evidence/rank
order, reciprocal-rank fusion and edge/item/byte ceilings remain canonical.
No alternate query executor, lifecycle or fallback is introduced.

`tests/context.rs` proves expired, future, retired and missing anchors cannot
recall their neighbors at a time when the root is invisible. It also proves
mixed live roots, historical visibility, identical packets after persistent
reopen, rrflowMX behavior and deduplication when seed/lexical retrieval share a
root. The context-flow owner states that boundary. POAM-010 links the verified
failure and this remediation without claiming gate closure.

Full-file reread: engine/context.rs and tests/context.rs, memory_context.rs
contract validators, both package journals and POAM-010. Read sections of the
context architecture owner, native query traversal, engine session helpers,
retirement contract, rrflowMX constructor and installed lifecycle test/example.
The complete authored and generated diffs were reviewed. Initial exploration
and source mapping are retained in the red receipt. No upstream source or new
research dependency was needed. Existing per-stage plan/packet evidence reports
selected/skipped roots; no new runtime tracing authority or performance claim.

## Executed verification

Native Windows, pinned Rust 1.99.1; Cargo commands use
`CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target`.

- Before repair: `cargo test --locked -p rrd-engine --lib context_graph_anchors -- --nocapture`
  failed: expired seed at time 1300 returned its neighbor. After repair, the
  same command passed 1 test, with 103 filtered out.
- `cargo test --locked -p rrd-engine --lib engine::tests::context`: 5 passed.
- `cargo test --locked -p rrd-engine --lib`: 104 passed.
- After the overlapping-root assertion, `cargo test --locked -p rrd-engine`:
  166 passed, 0 failed, 0 ignored across 13 test/doc-test results, including
  architecture, direct-read and trace checks. Raw log is retained locally in
  `target/rrflow-reasoning-engine-tests.log`.
- `cargo clippy --locked -p rrd-engine --lib --tests -- -D warnings`: passed.
- `cargo test --locked -p rrflow-cli --test installed_lifecycle`: 3 passed.
  The full process case is `cfg(unix)` and did not run on Windows. Ten existing
  unused import/helper warnings result from that gating; these are not hidden
  as clean Windows lifecycle proof or repaired by this package.
- `cargo build --locked -p rrd-client --example rrflowdb`: passed.
- `python ../rrflow-reasoning-installed-smoke.py`: a fresh Windows estate used
  the rebuilt real `rrflow.exe` and native HTTP example client. Exact installation,
  authenticated readiness, related-record creation, commit/update, transaction
  abort, process termination, offline quick verification and reopen passed.
  Task status stayed `done`, one relationship survived and cursor stayed 7.
  The probe terminates only its own process handles. Both processes are stopped.
  This is a bounded termination/reopen probe, not a full crash matrix or a
  substitute for the unit test of temporal graph anchors.

The executable is 152262144 bytes with SHA-256
`6e6aa7e51dd8f95fe8def773698674aee82dca2dd06456269946594b0f28dfbe`.
Its local development artifact and verification JSON are located in the
PR 31 artifact receipt. This locator has a [recorded correction](h-01-context-ci-correction.md).
Source is the implementation in this
package; artifact provenance binds the eventual committed source revision and
file digests outside the repository. No official artifact is published.

Local acceptance passed: `python scripts/knowledge/render_navigation.py`
(316 nodes, 1381 edges), `python scripts/ci/build_execution_inventory.py`
(1230 records), `python scripts/ci/check_documentation.py` (317 statuses,
315 coordinates), `python scripts/ci/check_workflow.py`,
`cargo fmt --all -- --check`, and `git diff --check`. Candidate CI is required on the final PR head.

## Limits and checklist

No complete reasoning model/router, persisted governed tree execution, trained
LFG, full storage crash/soak suite, competitive benchmark, cloud sync, external
application, clean-machine signed release or promotion was run or completed.
The native binary is a development build. Existing reasoning and release gates
remain open; this receipt does not convert prompt preparation into reasoning.

Checklist: owner, baseline, PR-before-implementation, exact bounded replacement,
source inspection, red/green oracle, wider tests, trace/resource/research choices,
failures/warnings/non-runs, full diff review and linked evidence are recorded.
There is no objective/roadmap status change. POAM-010 records this bounded
remediation; its broader open deficiencies remain. PR 31 holds exact push,
candidate CI and merge evidence. No unrelated worktree was edited.
