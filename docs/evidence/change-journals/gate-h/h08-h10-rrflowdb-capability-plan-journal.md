# H-08 to H-10 rrflowDB capability plan journal

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h08-h10-rrflowdb-capability-plan`
**Owner:** rrflowDB capability planning package receipt; not runtime or roadmap authority

This record reports a planning package. It writes the
[rrflowDB capability record](../../../reference/data/rrflowdb-capability.md)
and adds Gate H items H-08 (prompt graph families and the memory contract),
H-09 (Postgres-wire SQL face), and H-10 (opt-in project links). It changes no
code and completes no roadmap item.

## Evidence record

```text
gate/package and alpha outcome or prerequisite advanced: planning for OBJ-06 (dynamic governed reasoning and recall) and OBJ-09 (provider-neutral client operation): the owner's 2026-10-18 Decision 12 ("the pure Rust Postgres DB for AI" is rrflowDB with a Postgres-wire SQL face) required a capability record before code; the owner's 2026-10-19 statements add configurable prompt graph families, knowledge = memory, and opt-in read-only links between projects.
starting revision/tree and pull request: main at b298fa968c3fcd69616b456abe394c2dbdc88094 (tree d6998a8df6155c8700317d1cce8ee0ad0e0bea41); branch agent/rrflowdb-capability-plan; pull request #22 in rrflow/rrflow-development.
change brief (current -> target behavior, owner, exact scope, unchanged behavior, stop conditions): current — rrflowDB is defined as the per-project estate, but no record defines its Postgres-like SQL face, the graphs a prompt forms, the memory contract, or links between projects, and no roadmap item owns them. Target — docs/reference/data/rrflowdb-capability.md owns that target contract with a build order and proof standards; Gate H gains H-08, H-09, and H-10 with owning boundaries and acceptance evidence; the roadmap checklist and executable dependency spine (wave 10) include them; the system-overview rrflowDB row, the instance-topology workspace row, and the data reference index link the record; the execution inventory assigns planned paths to each new item. Unchanged: every implementation, contract, wire shape, OpenAPI digest, objective, POA&M record, and completion status. Stop if a planned capability would need a second persistence, query, or mutation authority, or if a record claim could not be traced to the repository or the owner's notes.
files and symbols read in full: AGENTS.md; docs/objectives/rrflow-1.0-alpha.md; docs/roadmap/rrflow-1.0/executable-dependency-spine.md (table and active-item text); docs/reference/data/README.md; the owner's LFG_NOTES_v3 (2026-09-26), Notes v2 and v2.1 (2026-09-25), and Session Notes (2026-09-24) in Google Drive; the Notion RRO, LFG, "RRO → LFG routing pipeline", and WF-01 records. Read in part: docs/architecture/system-overview.md (terminology), instance-topology.md (identities), engine-data-flow.md (automation and graph sections); docs/roadmap/rrflow-1.0.md checklist; gate-d, gate-f, gate-g, gate-h, gate-i item rows; rrd-core claim.rs (Tier, PromotionState); rrd-contract lib.rs (ClaimChangeSnapshot, TransactionMutation::AssertClaim); scripts/ci/build_execution_inventory.py (PLANNED_PATHS, validation) and check_documentation.py (classification and roadmap rules); the project's definitions-and-schema audit and build-order records.
implementation/evidence paths created, changed, moved, or deleted: created docs/reference/data/rrflowdb-capability.md and this journal; changed docs/roadmap/rrflow-1.0/gate-h.md, docs/roadmap/rrflow-1.0.md, docs/roadmap/rrflow-1.0/executable-dependency-spine.md, docs/reference/data/README.md, docs/architecture/system-overview.md, docs/architecture/instance-topology.md, and scripts/ci/build_execution_inventory.py; regenerated docs/evidence/change-journals/gate-h/README.md and docs/roadmap/rrflow-1.0-file-plan.jsonl. Nothing moved or deleted.
research decision and primary-source/adaptation record: the owner's notes are the specification for the prompt graph families (the owner named lexicon, evidence, source, knowledge, and governance; classification, reasoning, and plan/task were confirmed from the same notes; knowledge includes memory). External research was limited to the SQL face's protocol layer: pgwire 0.41.1 (MIT or Apache-2.0, 2026-09-29) supports protocol 3.0 and 3.2, extended query, COPY, TLS, SCRAM, cancel, and notifications; datafusion-postgres 0.18.0 (Apache-2.0, 2026-07-19) composes pgwire with DataFusion and a pg_catalog emulation but targets DataFusion 54 while RRFlow pins 55, so it is recorded as a reference input, not a dependency; GreptimeDB's 2026-04-01 report lists the tool, extended-query, and row-conversion problems the record's tool matrix makes explicit. No code was copied.
trace/resource/debug decision (add/preserve/not applicable, with reason): not applicable — no runtime behavior changes.
first-failure or characterization oracle and result: the documentation policy, navigation, and execution-inventory checks; the inventory check fails if a new unchecked roadmap item has no planned file, which is why each of H-08 to H-10 has planned paths.
acceptance commands and exact results: python3 scripts/ci/check_documentation.py: OK, 292 document statuses, 290 classified coordinates, parent indexes, and local links. python3 scripts/knowledge/render_navigation.py --check: OK. python3 scripts/ci/build_execution_inventory.py --check: OK. python3 scripts/ci/check_workflow.py: OK.
surfaced failures and corrections: the first draft described the native traversal reader as implemented; it is in review (pull request #21) and the record says so.
checks not run and reason: Rust builds and tests — no code changed; candidate CI runs the full pipeline on the pull request.
remaining known errors and owner: every capability in the record is planned or partial as its tables state; the open decisions (link acceptance by one owner, runtime-defined families, per-family retention) block M2 and L1 only.
full-file reread and diff review: complete for every created and changed file before commit.
change checklist: complete — owner chain, roadmap items and checklist, dependency spine, reference index, terminology and topology links, inventory assignments, journal index.
roadmap/POA&M status change, or explicit none: added unchecked roadmap items H-08, H-09, and H-10; no status change and no POA&M change.
pull request, CI run, and merge evidence: the pull request carries the candidate CI runs and the merge; this record does not restate them.
```

## Links

- [rrflowDB capability record](../../../reference/data/rrflowdb-capability.md)
- [Gate H](../../../roadmap/rrflow-1.0/gate-h.md)
- [Executable dependency spine](../../../roadmap/rrflow-1.0/executable-dependency-spine.md)
