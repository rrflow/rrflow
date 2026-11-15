# POAM-042 — the engram has no configurable organisation layer

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-042`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

The parent ledger owns priority and lifecycle status. This detail records a
verified gap against [OBJ-15](../../objectives/rrflow-1.0-alpha.md#required-outcomes);
it cannot close a roadmap gate independently.

## Observed deficiency

Verified at development `main` `2e3a67d2`:

- The only grouping field in the schema registry is one label pair.
  [`DataCatalogueIdentity`](../../../crates/transport/rrd-contract/src/lib.rs)
  holds a `namespace` and a `database`, both `default`, once per
  `DataSchemaRegistry`. No operation, rrflowQL syntax or `SecurityAction`
  selects either value, and `rrd-query` never reads them. The
  [instance topology](../../architecture/instance-topology.md#canonical-identities-and-cardinality)
  reserves other values for a later schema contract.
- A table kind carries a logical model, a schema mode and properties
  (`DataTableSchema`). No field records the part of the project a kind belongs
  to, and no category or tag exists for kinds or records.
- Every query-family request must name the whole engram scope
  `instance:{instance_id}`; any other scope fails with `WrongScope`
  (`rrd-engine/src/engine/query.rs`). An rrflowQL query reads one kind, plus at
  most one joined kind. Neither `AssembleContext` (`memory_context.rs`) nor
  vector search can be limited to part of the engram.
- A grant binds one action to an exact resource prefix, with an optional
  `DataPolicy` of tenant and row equality predicates and an allowed-field set
  (`rrd-security/src/lib.rs`). No resource kind or action names a part of the
  engram, so a project can isolate data only through row predicates that every
  caller must already know.
- `DiagnosticSnapshot` (`diagnostic.rs`) and the storage counters report the
  whole engram. Nothing measures retrieval, query cost or storage for a part of
  the engram.
- Knowledge trees (`knowledge_tree.rs`) group vector collections and external
  knowledge sources. They do not group record, relation or event kinds.
- The [historical vocabulary note](../../history/platform-vocabulary-note.md#rejected-resource-synonyms)
  of 2026-08-27 rejects `domain` as a public resource and points to
  `namespace` for logical isolation. That note is superseded, but no current
  record replaced its rule.

## Impact

A project with several internal domains (services, packages, subject areas)
cannot organise its engram into them. It cannot isolate one, scope retrieval
to one, or measure where its reasoning is slow or working well. A client that
wants to show that structure would have to invent its own grouping, which
would be a second source of truth the engine, grants and API cannot see.

## Owner decision

On 2026-10-27 the repository owner directed that domains inside a project
engram are configurable. RRFlow offers a default setup that each project tunes
itself. The default groups kinds; categories and tags label data across it;
data can be isolated; and drill-down shows bottlenecks and also what is working
well. The design absorbs current research on what succeeds for reasoning
engines.
[ADR-0003](../../decisions/0003-configurable-engram-organisation.md) records the
decision, and the [engram organisation contract](../../reference/data/engram-organisation.md)
owns its terms.

## Progress

H-11a closed the registry part of this gap. Domain and category definitions,
the membership of every kind and vector collection, and kind tags are part of
the schema registry. They are validated at the public boundary and in the
kernel, persisted on rrflowMX and rrflowKV, and readable after reopen. The
former `namespace`/`database` label is gone, and every default-organisation
registry keeps its digest. Record tags, scoped reads, isolation, domain grants
and per-domain evidence remain open, so every read is still engram-wide. The
[H-11a journal](../../evidence/change-journals/gate-h/h-11a-estate-organisation-registry.md)
records the evidence.

## Owning gates

H-11 owns the organisation contract and its engine behaviour. H-06 projects it
in the desktop client. D-03 and D-05 supply the default organisation proposal from the
project tree. H-01 and H-05 supply the evidence it measures.

Roadmap owners: [Gate H](../../roadmap/rrflow-1.0/gate-h.md) and
[Gate D](../../roadmap/rrflow-1.0/gate-d.md).

## Closure evidence

In one installed engram, through public operations, at an exact revision:

- **Configuration:** a versioned organisation definition assigns every kind to
  exactly one domain. The bundled default is the initial revision, and an
  attunement proposal is accepted only through an authorized revision.
- **Labels:** categories and tags label kinds and records, and the engine
  validates them.
- **Scoping:** rrflowQL, context assembly and vector search can each be scoped
  to a domain or a tag, and each result names the scope it read.
- **Isolation:** an isolated domain refuses every read, retrieval and
  relation that lacks its own grant.
- **Evidence:** diagnostics and retrieval evidence report per domain.

Each of these works on rrflowMX and rrflowKV and gives the same answer after
close and reopen.
