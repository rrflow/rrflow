# POAM-034 — commit-time schema constraints load the whole scope

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-034`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

`validate_schema_and_objects` in
`crates/persistence/rrd-store/src/access/semantic_commit.rs` validates schema
constraints against everything stored in the commit's scope:

- when any record schema in the effective registry declares
  `unique_properties`, every commit in the scope loads every record in the
  scope (`values_for_scope::<RuntimeRecord>` over `RUNTIME_RECORDS`); and
- when any relation schema declares `unique_pair`, `max_outgoing`, or
  `max_incoming`, every commit in the scope loads every relation in the scope
  (`values_for_scope::<RuntimeRelation>` over `RUNTIME_RELATIONS`).

`RuntimeSchemaRegistry::validate_objects` then checks the constraints over the
loaded set. The load is triggered by the registry rather than by the commit's
own mutations, so a commit that writes no constrained table pays it too. The
memory-estate seat-representation relation in `rrd-engine` declares
`unique_pair` and `max_outgoing`. Found while designing the H-06c task records;
H-06b dropped `unique_pair` from the prompt-flight use edge, which the engine
constructs unique, so prompt flights do not add this cost to a scope.

## Impact

Write cost grows with the number of records or relations in a scope, not with
the size of the commit. An estate that declares any pair, degree, or unique
property constraint slows every write as it grows, and the memory used by
validation grows with it.

## Owning gates

E-01, E-02

E-01 owns the adjacency prefixes that answer pair and degree checks for the
endpoints a commit touches; E-02 owns the persisted unique index that answers
unique-property checks for the values a commit writes.

Roadmap owner: [Gate E](../../roadmap/rrflow-1.0/gate-e.md).

## Closure evidence

Commit-time constraint validation reads only the adjacency or index entries of
the endpoints and values the commit touches. A differential test proves the
same accept/reject outcome as the whole-scope oracle across insert, update,
retire, conflict, and reopen. Physical counters show a constrained commit's
read cost unchanged as unrelated records and relations are added to the scope.

## Bounded remediation

The [scoped constraint-input repair](../../evidence/change-journals/gate-c/c-07g-scoped-constraint-reads.md)
uses current endpoint/pair adjacency for stable-schema relation constraints
and skips unrelated record kinds. Schema changes preserve full-scope checks.
Actual selected-key counts stay fixed as unrelated data grows; differential
memory/native/reopen histories cover every relation-rule combination.
All 226 store tests and 165 engine tests pass (two engine tests remain ignored).
Unique properties still scan the affected kind rather than a persisted value
index. Exact candidate CI, native capacity and E-02's value-index requirement
remain open; this is partial remediation, not closure.
