# rrflowDB database identity and PostgreSQL compatibility target

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/rrflowdb-postgresql-identity`
**Owner:** OBJ-01/A-01 terminology correction and POAM-011 compatibility-gap receipt

## Brief and baseline

Starting revision: `9bcc6154e9ee48f3ad8d28786a6abe75689e371d` on development
main. The owner clarified that rrflowDB is intended to be the pure Rust,
PostgreSQL-inspired database dedicated to AI and written for AI while
compatible with PostgreSQL. The existing description as a persistent logical
estate understated that identity. This documentation-only package records the
target in README and its existing architecture owner; it adds the unqualified
compatibility gap to POAM-011 under OBJ-09/H-04 without selecting a protocol
version, limiting the owner's intended compatibility scope, or claiming
implementation completion.

The change brief was prepared before edits for the draft PR description using
the session's previously authorized local-brief-first bootstrap sequence.
Scope: README, the canonical rrflowDB row and persistence/memory section in
system overview, POAM-011, this journal, its generated index, and generated
file inventory. No implementation is moved, removed, or rewritten. No runtime,
source pin, dependency, product version, or roadmap completion status changes.
Stop conditions are implementation expansion, invented compatibility guarantees,
or choosing unapproved support/version and release commitments.

## Grounding and research

Owners inspected: README, AGENTS, the complete system overview, alpha objective
and outcome set, Gate H's H-04 requirement, POAM-011, and the change-authoring
procedure/template. Implementation inspection: complete `rrd-server` and
`rrd-query` Cargo manifests; HTTP dispatch catalogue in
`crates/transport/rrd-server/src/http/router.rs`; imports and deployment model
in `crates/operations/rrd-operator-knowledge/src/pgvector_postgres.rs`; and
workspace searches for PostgreSQL/pgwire compatibility and dependencies.

Characterization: source searches found the outbound PostgreSQL/pgvector
adapter and its tests, but no PostgreSQL server wire surface. Current owners
describe RRD HTTP/WebSocket and lack a PostgreSQL support matrix. This is a
verified specification/evidence gap, not a claim of exhaustive protocol audit.

Primary references: PostgreSQL's [frontend/backend protocol](https://www.postgresql.org/docs/18/protocol.html)
and [SQL conformance](https://www.postgresql.org/docs/18/features.html).
These identify distinct qualification areas; no upstream implementation was
copied. Runtime tracing, debugging and resource measurement are not applicable
to this identity correction. Exact compatibility coverage remains open for
the owner's scope clarification and a subsequent bounded implementation plan.

## Verification and disposition

Acceptance: `python scripts/knowledge/render_navigation.py` and `--check`
passed (14 indexes, 292 nodes, 1257 edges);
`python scripts/ci/build_execution_inventory.py` and `--check` passed
(1190 records); `python scripts/ci/check_documentation.py` passed
(293 statuses, 291 coordinates); `git diff --check` passed. No validation
failure surfaced. An exploratory unquoted `git rev-parse HEAD^{tree}` command
was misparsed by PowerShell; the exact starting commit above is the baseline,
and no tree-ID evidence is claimed.
The complete authored diff is reviewed for consistency with engine authority,
independent native storage, pre-alpha maturity, and honest target/status wording.
No runtime or compatibility test is claimed; no executable behavior changed.

Checklist: identity corrected at its owner; PostgreSQL compatibility retained
as a requirement; outbound-adapter evidence distinguished from server
compatibility; missing support matrix recorded in POAM-011; existing boundaries
preserved; journal linked by generated navigation; inventory regenerated;
candidate CI required before PR landing. No gate or POA&M lifecycle status
changes, and no main merge or official promotion is claimed.
