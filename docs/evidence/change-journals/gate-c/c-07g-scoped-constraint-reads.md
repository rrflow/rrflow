# C-07g: scope commit constraint inputs

**Status:** active evidence record; native capacity and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-scoped-constraint-reads`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), E-01/E-02, OBJ-02 and POAM-034/035
**Pull request:** [88](https://github.com/rrflow/rrflow-development/pull/88)

## Brief, baseline and canonical replacement

Starting revision `ce3a2395e2268752e35e68ead24c0ae4533bba65` integrates PR87
from fetched main. The pre-edit brief binds the existing whole-scope cost as
a C-07 capacity prerequisite, with E-01/E-02 retaining constraint ownership.

`validate_schema_and_objects` previously loaded all current records/relations
when any schema declared uniqueness or degree constraints. Stable-schema
commits now acquire only mutated unique record kinds and the current adjacency
groups at proposed relation endpoints. Pair-only uniqueness selects the exact
typed pair, even when the same endpoint has many other relations. Incoming and
outgoing degree checks read their respective groups. Repeated ranges and
relation identities are deduplicated; disagreeing copies or misplaced keys
fail closed. Existing byte/work admission still bounds these reads.

`RuntimeSchemaRegistry::validate_objects` remains the sole verdict authority.
The helper only replaces input selection. Schema changes retain the existing
whole-scope path. Moves remove occupancy from old groups and validate all new
groups; same-batch updates/retirements keep the existing kernel semantics.
Record uniqueness still scans the affected kind. Value-specific persisted
unique indexes remain E-02 work, so POAM-034 is not closed.

## Scope, traceability and decisions

Changed `rrd-store/src/access/semantic_commit.rs`, factored acquisition into
`semantic_commit/constraints.rs`, extended canonical current-kind/adjacency
prefix support in `keyspaces.rs`, and added `semantic_commit/constraint_tests.rs`.
No storage format, schema, public operation, index or dependency changed.
No implementation was deleted or moved outside its canonical boundary.
The old whole-scope input oracle remains in schema-change production behavior
and differential tests, not as an alternate runtime fallback.

Read README and the objective/C-07/E/POA&M owner chain, authoring/traceability
procedures, semantic commit validation and current adjacency maintenance,
canonical keyspace grammar and AccessRead helpers, kernel uniqueness/cardinality
and temporal overlap behavior, existing public runtime tests and installed
capacity schema. Complete changed functions, new helper/tests and diff were
reread; generated inventory retains exact file identities.

Research/source adaptation: none, because current adjacency and the existing
kernel validator define this repair. Resource evidence counts actual keys
returned by the storage scan during constraint acquisition; these counts are
not total page-I/O, elapsed-time or full native capacity benchmarks. Preserve
all physical limits. Stop on different constraint verdicts, missing touched
groups, changed durable/temporal behavior or unsupported completion claims.

## Failures and acceptance

Before implementation, the fixed-edge regression grew from one scanned
relation to 97 after 96 unrelated relations. Afterward it reads the same two
endpoint entries before/after growth and native reopen; unconstrained record
updates scan no constraint inputs. Pair-only uniqueness reads one pair entry
even after 96 additional pairs at the same source. Unique-record acquisition
reads one affected-kind row despite 96 unrelated-kind records.

The full-state differential oracle covers all eight combinations of pair,
incoming and outgoing constraints through actual memory/native commits:
insert, update, move, adjacent/overlapping validity, retirement, recreation,
multi-relation batches, stricter schema replacement, stale-cursor rejection,
and reopen. Rejections preserve the cursor and have no commit outcome.
Additional tests cover unique-property replacement and collisions, misplaced
adjacency and disagreeing incoming/outgoing copies.

An initial retirement fixture used time 30 for a relation valid only until
20. Existing retirement admission correctly rejected it. The fixture now
retires at valid time 15; no production retirement behavior changed.

| Command | Actual result |
|---|---|
| `cargo test --locked -p rrd-store --lib constraint_reads_stay_scoped -- --nocapture` before repair | Failed: 97 scanned relations versus baseline one |
| Same focused command after repair | Passed |
| `cargo test --locked -p rrd-store --lib constraint_tests -- --nocapture` | Five passed |
| `cargo test --locked -p rrd-store` | 226 passed |
| `cargo test --locked -p rrd-engine --lib` | 165 passed, two existing ignored |
| `cargo clippy --locked -p rrd-store --all-targets -- -D warnings` | Passed |
| `cargo fmt --all` | Passed |

Local command receipts remain under `target/c07g-constraints-*`. Navigation,
inventory, documentation and diff checks accompany commit. Checklist: brief,
canonical replacement map, red oracle, implementation, differential/focused/
owning tests, resource decisions, full diff review and linked evidence recorded.
Exact candidate CI must pass before merge. Native capacity, complete profiles
and a successful 24-hour soak remain unproven. C-07, E-01/E-02, OBJ-02 and both
POA&M items remain open; no product-version or alpha-status change.
