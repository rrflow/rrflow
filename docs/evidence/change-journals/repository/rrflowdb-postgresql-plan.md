# rrflowDB PostgreSQL and independent-release planning receipt

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/rrflowdb-postgresql-plan`
**Owner:** OBJ-09/H-08 design, research and release-prerequisite planning evidence

## Change brief and authority

On 2026-10-19 the owner asked for online research and a concrete rrflowDB plan
to leave Turso, own/manage the database and officially release it. The earlier
identity-only PR #24 did not define the implementation or compatibility scope.
This expanded package starts from development main
`9bcc6154e9ee48f3ad8d28786a6abe75689e371d`, carries forward that identity
correction as `d1602b1118e7bea597e949cf5b8d728469358ddf`, and is reviewed in
[PR #25](https://github.com/rrflow/rrflow-development/pull/25). The PR brief was
published before authoring this plan. The original identity receipt remains
unchanged historical evidence.

Current to target: existing native storage and narrow rrflowQL had no bounded
PostgreSQL implementation/qualification plan. Define one core profile with
source ownership, semantic mapping, dependencies, negative oracles, data
portability and release proof. Add H-08 as eight open packages under the
existing roadmap, extend OBJ-09 and Gate J evidence, and record the remaining
implementation gap in POAM-011. No runtime, version or official release changes.

Scope: README; system overview; alpha objectives; root roadmap, Gate H/J,
dependency spine and current-alpha H count; POAM-011/013; new
`docs/reference/protocol/postgresql.md` and its index; this journal and index;
one explicit contract-to-H-08 assignment in the existing inventory generator;
generated inventory. Stop on runtime implementation, new dependency activation,
live data movement, invented compatibility/performance evidence or promotion.

## Code grounding and research decisions

Read the relevant owning records and changed sections: README, AGENTS, system
overview, objectives, Gates C/F/H/J, dependency spine and execution checklist,
POAM-011/013, physical-format reference, release version policy, protocol index,
and repository change-authoring instructions. Read complete manifests for
`rrd-server` and `rrd-query`, query module exports, and the affected inventory
path assignment and validation flow. Inspect the source definitions of
`RuntimeValue`, `RuntimeValueType`, schema records, `Query`/temporal selectors,
the `StorageTransaction` trait, `RrdEngine::begin_transaction`, HTTP dispatch,
and the outbound PostgreSQL adapter's imports/deployment model. These are
boundary inspections, not a claim of exhaustive source audit.

The code confirms snapshot-isolated native byte transactions and an existing
governed commit path; it does not prove PostgreSQL READ COMMITTED, savepoints,
SQL types, relational DDL/DML or server wire compatibility. Search of manifests,
lockfile and source identified no Turso/libSQL/SQLite database dependency; the
PostgreSQL dependency is an optional outbound operator adapter. The tracked
tree at baseline has no LICENSE/LICENCE/NOTICE-named files despite the workspace
licence expression; POAM-013 now records that specific packaging gap.

Used the engineering system-design skill to organize functional/nonfunctional
requirements, architecture, mapping, trade-offs and qualification. Primary
online inputs are linked beside their use in the
[single compatibility contract](../../../reference/protocol/postgresql.md):
PostgreSQL 18 protocol/flow, SASL, types, isolation, savepoints, catalogues,
SQLSTATE, COPY and regression docs; Apache DataFusion/sqlparser; pgwire;
pgvector; Turso compatibility/export context; and primary driver release pages.
The PostgreSQL 18.6 and seven-client fixture selections are versioned research
inputs, not acquired or tested artifacts. H-08a locks their full provenance.

No upstream implementation is copied. The research selects reusable codec and
parser candidates, maps them to RRFlow-owned boundaries, and leaves dependency
admission to a traced implementation package. Runtime instrumentation/resource
measurements are specified as later acceptance, not performed for a plan.
No assertion about a vendor acquisition or comparative performance is made.

## Failure oracles and verification

First characterization oracle: RRD's current source has HTTP/WebSocket and an
outbound PostgreSQL client, but no inbound PostgreSQL profile. The plan makes
that gap explicit rather than interpreting a dependency or build as support.

Initial documentation validation exposed two mechanical omissions:

- inventory generation rejected H-08 because it had no assigned file;
- POAM-011's expanded gate-family list lacked C and F owner links.

After updating the PR brief, register the new contract in the existing explicit
path map and add the missing owner links. No generator algorithm changes.
Subsequent navigation, inventory and documentation checks passed. A guessed
PostgreSQL SASL URL returned 404; research used the verified
`sasl-authentication.html` primary page instead. One patch context mismatch
made no change and was corrected before verification.

Acceptance results: `python scripts/knowledge/render_navigation.py` and
`--check` passed (14 indexes, 294 nodes, 1278 edges);
`python scripts/ci/build_execution_inventory.py` and `--check` passed
(1192 records); `python scripts/ci/check_documentation.py` passed
(295 statuses, 293 coordinates); `python scripts/knowledge/test_navigation.py`
passed 6 tests and `python scripts/knowledge/test_export.py` passed 11;
`git diff --check` passed. Review covers the full new contract,
package table, complete authored diff and consistency of owner links, required
profile scope, exclusions, dependency order and honest implementation status.
Review moved concurrency-specific constraint oracles into H-08c so H-08b's
schema/value package does not depend on its successor's transaction work.

Checks not run: PostgreSQL reference/driver corpus, Rust runtime, cloud/UI,
data migration, performance, signed artifacts and official promotion. Those
are implementation/release work; no executable product behaviour changed.
Candidate CI remains required before this PR can merge.

Checklist/disposition: source-grounded architecture; primary sources linked;
finite required profile and explicit non-goals; one native semantic/storage
authority; source-exit and ownable release mapped; H-08 registration and owner
links verified; generated projections refreshed; historical evidence preserved.
H-08 and its eight packages are open, H's total becomes 0/8, OBJ-09 remains
partial and POAM-011/013 remain open. No existing gate is marked complete.
