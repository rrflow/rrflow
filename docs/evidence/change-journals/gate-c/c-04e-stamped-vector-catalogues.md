# C-04e stamped vector catalogue reads

**Status:** active change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-04e-stamped-vector-catalogues`
**Owner:** vector catalogue repair receipt; Gate C retains completion authority

## Package and change brief

OBJ-03 equivalent stamped semantics and OBJ-10 measured-work prerequisites;
reopened C-04 / POAM-028. Baseline
`b5772374c1ea8ab388fa0c4ac13558d2e3125775`, tree
`0f3c28c6ab8d10ba174cfd737cb3d38e058b1fdc`.
[PR 39](https://github.com/rrflow/rrflow-development/pull/39) contained the
change brief before source edits. Opening commit
`c7de019eb3daea203833ebd27a87fd96ff39b84a` is only the review anchor, not
implementation evidence. Development main was fetched and the checkout was
clean. The owner requested accurate, scalable AI retrieval; D-01 remains the
active release spine and this bounded repair does not reorder indexing gates.

Current-to-target: every search formerly replayed both vector catalogues from
the whole scope log, independently of the candidate stamp and key budget.
`read_vector_catalogue_versions` now selects the three catalogue record types
and expands to exact referenced object identities using the existing
`RuntimeRepository::read_versioned_expanding`. Search adds its vector source
to that same physical transaction, stamp validation and authentication budget.
It builds metadata from those returned versions without another storage read.
Control-plane catalogue helpers use the same direct substrate with a finite
100,000-key ceiling. No format, dependency, public endpoint/schema, external
service, durable lifecycle, or write authority was added.

Stop conditions were loss of corruption/atomicity checks, a second authority,
unbounded replacement work, unexplained regressions, or missing retained-stamp
proof. None was accepted as a shortcut. This does not claim PostgreSQL parity,
working-tree indexing, SWAG execution, migration, or massive-codebase readiness.

## Implementation traceability and review

Removed behavior: the `changes_since` and `read_changes` catalogue loops in
`runtime/vector_catalog.rs`. Canonical replacement: typed version ranges in
that same module over the existing store read substrate. The catalogue
reducers retain digest, canonical JSON, scope/identity, atomic record/object
commit, revision continuity and quantization lifecycle checks. Existing
explicit diagnostic change-set reducers remain; no archive, diagnostic, or
maintenance history path is disguised as normal search.

Changed source paths are `crates/authority/rrd-engine/src/runtime/vector_catalog.rs`,
`src/engine/vector.rs`, `src/engine/vector/search.rs`, the test-module registration
in `src/engine/tests/vector_index.rs`, and `tests/direct_read_source_closure.rs`.
The new focused module is `src/engine/tests/vector_index/catalogue_reads.rs`.
No implementation file was moved or deleted. Existing HNSW, quantization,
corrupt-object, revision conflict and rrflowMX/rrflowKV catalogue tests remain.
The two repaired source-closure exceptions are removed; the independent
maintenance exception remains visibly open.

Owner/procedure reads: AGENTS, README, alpha objectives, release roadmap,
dependency spine, Gate C/D, POAM-028, agent-bootstrap independent-engine rules,
change-authoring procedure and CI candidate contract. Implementation review
covered vector catalogue publication, reconstruction, schema and validation
functions; vector search/candidate access; store runtime read APIs, expanding
read implementation and typed selectors; vector and catalogue fixtures,
source-closure exceptions, error/evidence projection, and object staging.
The new regression module and small modified composition modules were read in
full; existing large files were reviewed through relevant functions, preserved
validation paths and complete diffs. Generated navigation/inventory are
mechanically validated rather than treated as another authority.

## Research, trace and resources

[Turso's reliability report](https://turso.tech/blog/the-final-boss-of-reliability)
reinforces deterministic counterexamples and persistence failure oracles.
No upstream code or engine was copied, and no formal-verification claim is
made. The stamped-read implementation is already owned by RRFlow.
[ClickHouse Parquet](https://clickhouse.com/docs/guides/clickhouse/data-formats/parquet)
and [PostgreSQL COPY](https://www.postgresql.org/docs/18/sql-copy.html)
were reviewed as future interoperability boundaries. Existing bounded Parquet
export is not complete ClickHouse integration or PostgreSQL migration; schema,
NULL/type, source/evidence and relationship readback must be qualified separately.

Search exposes all candidate/catalogue/object read work through existing
`ReadEvidence`, including its caller's aggregate `max_storage_keys`.
No new trace vocabulary or private debug lifecycle was introduced. The fixture
prints counters and read coordinates, not source payloads or credentials.
Catalogue generation history still contributes work inside its typed ranges;
this repair removes unrelated scope history, not the need for E-04's persistent
incremental index work or large-estate evaluation.

## Validation and observed limits

The first regression failed on the old implementation: after publishing
index generation 2, a request captured at generation 1 selected `Some(2)`
instead of `Some(1)`. It passed after the replacement, including close/reopen.

- `cargo test --locked -p rrd-engine --lib catalogue_reads -- --nocapture`:
  3 passed. Covers retained HNSW generation, retained quantization
  ready/active/retired state, restart, unrelated history and shared budget.
- `cargo test --locked -p rrd-engine --all-targets --no-fail-fast`:
  173 passed across 12 targets, zero failed/ignored. Includes the six existing
  catalogue tests, source closure and 31 architecture checks.
- `cargo test --locked -p rrd-store --test direct_read_paths`: 1 passed.
- `cargo clippy --locked -p rrd-engine --all-targets -- -D warnings`: passed.

With two vector versions, one HNSW catalogue record and one object binding,
adding 1,024 unrelated records advances the cursor from 11 to 1,035 while
semantic reads remain four keys, four values and 5,467 decoded bytes.
Total reads rise from 28 to 65 keys because accumulator authentication grows;
reading the retained stamp examines 37 keys. The exact measured request budget
succeeds; a vectors-only budget or one fewer key fails. The same semantic
counts and results survive rrflowKV reopen. These are deterministic fixture
measurements, not production throughput, latency, RSS or million-file evidence.

The initial failed assertion is retained in the local first-oracle log;
subsequent source tests and strict lint passed without suppressions. The first
documentation check rejected renamed mandatory POA&M headings; restoring
`Observed deficiency` and `Impact` and putting historical context in prose
corrected the schema mismatch. Research
and repository inspection surfaced no additional verified runtime deficiency
outside the already open POAM-028 scope. Diagnostic shell path-glob errors
were corrected to directory searches and were not counted as test evidence.

Not run locally: whole-workspace optional-feature/SDK/native platform matrices,
new process-kill or storage-full campaigns, million-file indexing benchmarks,
model/quality evaluation, ClickHouse/PostgreSQL migration or compatibility
qualification. The existing candidate CI chain must pass at the exact final
head before merge; green component tests are not release evidence.

Final repository checks: `python scripts/ci/check_documentation.py`,
`python scripts/ci/check_workflow.py`,
`python scripts/knowledge/render_navigation.py --check`,
`python scripts/ci/build_execution_inventory.py --check`,
`python scripts/ci/test_execution_inventory.py`,
`python -m unittest scripts.knowledge.test_navigation scripts.knowledge.test_export`,
`cargo fmt --all -- --check`, and `git diff --check` all passed after the
heading correction. Navigation has 17 generated indexes, 344 nodes and
1,514 edges; execution inventory has 1,275 current/generated/planned paths.

## Checklist and remaining work

- [x] Baseline/owner/diff inspection and pre-implementation PR brief.
- [x] Replacement map and preserved validation/characterization.
- [x] First failing oracle, focused tests, package suite and strict lint.
- [x] Research/source-adaptation and resource/trace decisions recorded.
- [x] Changed behavior and complete diff reviewed; new source read in full.
- [x] Final documentation/workflow/navigation/inventory/format/diff checks passed;
  4 inventory tests and 17 knowledge tests passed.
- [ ] Exact-candidate CI and merge receipt recorded on PR 39 before landing.

README, OBJ-03, the dependency spine and POAM-028 now describe the repair
without closing C-04. The reopened evidence is preserved chronologically and
this journal is linked from the new C-04e evidence section. POAM-028 remains
**Open** because maintenance replay and the complete closure corpus remain.
Build/preflight operations still capture their own direct-read stamps; the
single request-wide stamp/budget guarantee established here is for vector
search. D-03/D-04 still own filesystem inventory/parsing freshness; a database
snapshot alone cannot prove an externally modified working tree is current.
Gate G still owns SWAG and user-preference-driven model selection through
engine-enforced policy. No roadmap checkbox or release status advances.

Final candidate CI and merge receipts belong to
[PR 39](https://github.com/rrflow/rrflow-development/pull/39); this source record
cannot self-certify the commit that contains it. No official-repository
promotion is authorized or performed.
