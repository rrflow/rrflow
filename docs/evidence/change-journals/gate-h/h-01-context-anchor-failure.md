# H-01 temporal context-anchor failure characterization

**Status:** active immutable failure evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-01-context-anchor-failure`
**Owner:** OBJ-06/H-01 recall correctness prerequisite; POAM-010

Starting revision: `ccad7869c9d2fc2a277e7c723e17428e79d2dab7`.
Current behavior: context seed candidates require temporal visibility, but
snapshot graph expansion reconstructs roots from all requested seeds. A visible
relation can therefore recall a neighbor through an expired root. Native
rrflowQL traversal already rejects an invisible starting record.
Target: one visible, deduplicated root set shared by graph stage selection and
execution, preserving historical reads, rank ordering and traversal bounds.
This is repair of existing recall behavior, not a new model execution path or
completion of governed reasoning. DevPulse and cloud accounts are irrelevant
to this native correction.

The local change brief preceded edits. GitHub requires a nonempty branch diff;
this initial failing regression/evidence commit opens the package PR before
production implementation. No retired planning-only file is created. Stop if
the oracle disproves the issue or the fix requires changing relation/path
semantics, a public contract or engine authority.

Files read in full for this boundary: engine/context.rs and tests/context.rs;
ADR-0002; change-authoring procedure; implementation navigation; POAM-010.
Additional inspected sections: current README/AGENTS/objective/dependency spine,
engine-data-flow context contract, memory_context request/packet validators,
rrd-query physical/graph.rs root predicate and traversal, engine tests.rs
session helpers, core.rs rrflowMX constructor, public retirement mutation.
Broader discovery inspected flight preparation and router admission contracts;
these are not represented as executed reasoning. Some exploratory reads used
nonexistent guessed paths or PowerShell wildcard arguments; corrected by
repository path discovery, with no repository effects.

Changed implementation evidence: only
`crates/authority/rrd-engine/src/engine/tests/context.rs` at this stage. The test
uses public engine transactions and context assembly with expired, future,
retired and missing roots, mixed live roots, historical queries, rrflowMX and
persistent reopen. It is an expected red test until the repair lands. Created
this journal; updated POAM-010 and generated index/inventory. No production
code, dependency, version, provider or external application change.

First failure: `cargo test --locked -p rrd-engine --lib context_graph_anchors -- --nocapture`
with `CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target` on native Windows,
Rust 1.99.1. Compilation succeeded; 0 passed, 1 failed, 103 filtered out.
At valid time 1300, expired anchor returned `record:note:expired-leaf` when
expected identities were empty. This proves the actual defect on rrflowKV;
later matrix cases did not execute after this first assertion. Reopen and
rrflowMX are not yet reported passed.

Research/adaptation: none needed; canonical native behavior supplies the oracle.
Trace/resource/debug: retain packet and per-stage evidence; test exact returned
identities and selected/skipped graph state without a new logging authority.
The replacement remains within the existing snapshot/edge/item/byte budgets.
Traceability: root filtering stays in RrdEngine; preserve the current graph
walker and its relation semantics. Final replacement and owning-suite evidence
will be a separate linked immutable receipt. Full runtime suites, benchmarks,
model execution and clean installation are not run for this red baseline.

Checklist: baseline, owner, brief, scope, inspected sources, first failure,
research/trace decisions, replacement map, non-runs and stop conditions are
recorded. POAM-010 gains the verified defect; no roadmap gate changes status.
