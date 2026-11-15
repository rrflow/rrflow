# Engram organisation contract

**Status:** active target contract; H-11a is implemented (domain and category definitions, kind and collection membership, and kind tags in the schema registry); record tags, scoped reads, isolation and per-domain evidence are not ([POAM-042](../../poam/rrflow-1.0-alpha/poam-042.md))
**Coordinate:** `rrflow://rrflow-instance/data/reference/data/engram-organisation`
**Owner:** domains, categories and tags inside one engram: membership, scoping, isolation, per-domain evidence, defaults and tuning

[ADR-0003](../../decisions/0003-configurable-engram-organisation.md) records
why RRFlow organises an engram this way and the research behind it. This
record fixes the terms. The [roadmap](../../roadmap/rrflow-1.0.md) owns order
and completion: item H-11 in [Gate H](../../roadmap/rrflow-1.0/gate-h.md)
carries the work, and [OBJ-15](../../objectives/rrflow-1.0-alpha.md#required-outcomes)
measures it.

## Owner decisions this record carries

| Date | Decision | Effect here |
|---|---|---|
| 2026-10-17 | One project ↔ one engram (rrflowDB) ↔ one RRD instance. | Domains divide one engram. They never span engrams or create instances. |
| 2026-10-18 | rrflowDB is "the pure Rust Postgres DB for AI". | A domain is the engram's catalogue namespace, and the PostgreSQL projection shows it as a SQL schema. |
| 2026-10-27 | Domains are configurable, with a default setup each project tunes; kinds are grouped; categories and tags can be created; data can be isolated; drill-down finds bottlenecks and what works; the design absorbs current research. | This record. |

## Terms

| Term | Meaning | Not allowed to mean |
|---|---|---|
| **organisation** | The domain, category and tag definitions in one schema-registry revision. | A client preference, a second catalogue or a configuration file outside the registry. |
| **domain** | A named part of one engram's catalogue, such as a service, package or subject area. It has an identity, a title, a description, an optional parent domain and an isolation mode. | An engram, instance, database, storage root, tenant, deployment or authorisation principal. |
| **membership** | The one domain a kind belongs to. Records, relations, events, series values, geo values, objects and vector points inherit their kind's domain. | Part of the kind's identity. |
| **category** | A governed vocabulary of tags. It declares its targets (kinds, records or both), its cardinality (one tag or several per target) and whether writers with the grant may add tags. | A domain, a free-text property or a model-only label. |
| **tag** | One value in a category, such as `lifecycle:accepted` or `component:storage`. | A record kind or an access grant. |
| **scope** | A selector of domains (optionally with their descendants) and tags (any or all) that a read applies before generating candidates. | A filter applied after a bounded search. |

## Domains and membership

- **Definition.** A domain definition is part of the schema registry. It holds
  a `CanonicalId`, a title, a description, and an optional parent. H-11d adds
  its isolation mode (`shared` or `isolated`) together with the enforcement,
  so no registry can declare an isolation the engine does not yet enforce.
  Domains form a tree with no cycles, at most 8 levels deep.
- **Built-in domains.** `default` and `rrflow` always exist and cannot be
  redefined. A declared domain's parent is `default` or another declared
  domain, never `rrflow`.
- **Membership.** Every table kind belongs to exactly one domain, and so does
  every vector collection. A kind or collection with no explicit domain
  belongs to `default`, or to `rrflow` when it is engine-owned. A registry
  that names an unknown domain or parent, forms a cycle, exceeds the depth or
  repeats a domain is rejected before the cursor advances.
- **Identity.** Kind names stay unique across the engram, so the domain is not
  part of a kind's identity. Moving a kind to another domain is one schema
  revision. It rewrites no record. An old read stamp reads the old membership.
- **Reserved domain.** Kinds and collections whose identity starts with
  `rrflow-` are engine-owned, such as `rrflow-seat` or
  `rrflow-reasoning-budget-event`. They always belong to the `rrflow` domain,
  and no other kind or collection may be placed there. Future engine-owned
  families, such as prompt flights, tasks and the tag family below, follow the
  same rule.
- **Unchanged digests.** A registry whose organisation is all defaults
  encodes, and therefore digests, exactly as it did before domains existed.
  Organisation data adds a separately versioned section to the canonical
  encoding.
- **Retired label.** H-11a removes the registry's `DataCatalogueIdentity`
  (`namespace`/`database`). The engram is the one database, and a domain is
  its catalogue namespace. H-11d replaces the `Namespace` and `Database`
  resource kinds with `Domain`.

## Categories and tags

- **Definition.** A category lives in the schema registry with its tags, its
  targets, its cardinality and whether it is open.
  - **Closed.** Only a schema revision adds tags.
  - **Open.** Writers with the grant may add tags, within the limits below.
- **Tags on kinds** are part of the registry and are revisioned with it.
- **Tags on records** are relations of the reserved kind `rrflow.tagged`, from
  the record to a tag node of the reserved kind `rrflow.tag`.
  - They use native adjacency in both directions, so each tag has an indexed
    candidate set.
  - Each assignment is bi-temporal: removing a tag retires the relation and
    never deletes it.
  - Each assignment carries provenance: the operator, rule or model that made
    it, with revision and confidence.
  - The engine validates every assignment against its category's targets,
    cardinality and vocabulary in the same commit.
- **Proposals.** A model or rule may propose tags. A proposal is inert data
  until an authorised write applies it.
- **Vocabulary changes are explicit.** Retiring or renaming a tag is a schema
  revision, and its existing assignments must be retired or remapped first.
  The engine refuses a revision that would leave assignments to a retired tag.

## Scoped reads

Each of these reads accepts a scope:

- rrflowQL queries;
- `context-assemble`;
- `vector-search`;
- `data-schema-read`;
- `diagnostics-read`;
- changefeed reads.

A scope applies as follows:

- **Pre-filter.** The engine applies the scope before candidate generation,
  through kind membership, native adjacency of tag nodes, scalar or tag
  indexes, or filter-aware vector traversal. A path that cannot pre-filter a
  scope refuses it. It never runs a bounded search and filters afterwards.
- **Disclosure.** Every result names the scope it read, the organisation
  revision, and any truncation or budget exhaustion.
- **rrflowQL.** A query already names one kind, so its domain is implied. The
  language gains a tag predicate. Its exact grammar is fixed with golden
  vectors in the H-11c package and added to `fixtures/query-vectors.json`.
- **No scope.** A read without a scope behaves exactly as it does today. It
  reads every shared domain plus each isolated domain the caller holds a grant
  for.

## Isolation

| Mode | Reads and retrieval | Relations | Grants | Physical rules |
|---|---|---|---|---|
| `shared` (default) | Part of every unscoped read; context expansion may cross into it | May relate to any shared domain; cross-domain commits are atomic | Grants for the whole engram apply | Shares lexical statistics and indexes with other shared domains |
| `isolated` | Read, retrieved, traversed or streamed only by a caller whose grant names this domain | Crosses the boundary only for relation kinds the domain lists, with both grants held | Grants for the whole engram do not reach it; a grant names `domain:<id>` in its resource prefix | Keeps its own lexical statistics; its vectors never share an approximate index with another domain |

- **Resource paths.** These gain a `domain` segment, for example
  `instance:{id}/domain:{domain}/…`. `ResourceKind::Domain` replaces
  `Namespace` and `Database`.
- **Changing mode.** Making a domain isolated is refused while relations cross
  its boundary that it does not allow. Those relations must first be retired
  or the relation kind allowed. Making it shared again is an ordinary
  revision.
- **What stays whole.** Isolation does not split the engram's transactions,
  cursor, backup or restore. An isolated domain is still committed, backed up
  and restored with its engram.

## Per-domain evidence

`diagnostics-read` and the retrieval evidence of H-01 and H-05 report the
following per domain. A value the engine has not measured is reported as
unknown.

| Measure | Purpose |
|---|---|
| Kinds, records, relations inside the domain and crossing out of it, vector points, storage keys and estimated bytes | Size and shape |
| Queries, retrievals and denials; latency by stage; keys, pages, rows and candidates examined | Bottlenecks |
| Context returned, delivered and used or cited, and each avenue's contribution (OBJ-11 view 1) | What is working |
| Verified outcome results per question type, where H-02 records them | Effects that averages hide |
| Truncations, budget exhaustion and stale reads | Gaps |

Retrieval spans name the domain as their data source, following the
OpenTelemetry GenAI retrieval-span convention (`gen_ai.data_source.id`).

## Default setup and tuning

- **Bundled initial revision.** One project domain, `default`, and the
  reserved `rrflow` domain. Both are shared, and there are no categories. A
  project that never configures anything works exactly as an unorganised
  engram.
- **Attunement proposal.** After D-03, attunement may propose domains from the
  committed project tree: workspace members, packages and service roots, with
  each proposed kind assigned to where its source lives. Community detection
  over the relation graph may also propose domains or tags. Every proposal is
  inert until an authorised revision accepts it. The engine never reorganises
  on its own.
- **Measured tuning.** A proposed organisation can run as an experiment arm
  against held-out per-domain evaluation, with assignment, exposure and
  outcome events, using the
  [prompt graph family](rrflowdb-capability.md#configuration-and-experiments)
  experiment machinery. It is promoted only when both its grade and its sample
  meet their thresholds.

## Derived structures per domain

Community or tree summaries, code graphs and other derived projections may be
built for a domain. Each binds its source cursor and the organisation revision
it was built from. Retrieval routing (H-01) uses summaries only for broad
sensemaking questions, and a code graph's default expansion is one hop. A
derived structure never becomes canonical, and it obeys its domain's scope and
grants.

## Projections

- **Connectome** shows Project ▸ Engram ▸ Domain. It reads domains, categories
  and per-domain evidence from public operations and never keeps its own
  grouping ([Connectome contract](../client/connectome.md)).
- **PostgreSQL** (H-08) shows each domain as a SQL schema (`pg_namespace`) and
  each kind as a table in it. Kind names are unique across the engram, so a
  qualified `schema.table` and a bare `table` resolve to the same kind.
- **SDKs and OpenAPI** are generated from the contract types. No surface keeps
  a handwritten copy.

## Limits

| Limit | Value |
|---|---|
| Domains per engram | 256 |
| Domain depth | 8 |
| Categories per engram | 64 |
| Tags per category | 4,096 |
| Tags per record, across categories | 64 |
| Domains or tags in one scope | 32 |

These are contract ceilings. Installed engram limits may be lower.

## Build order

Each step is one pull request with its own oracle.

| Step | Work | Depends on | Proof |
|---|---|---|---|
| H-11a | Domain and category definitions, kind and collection membership, and tags on kinds in `rrd-contract` and the `rrd-core` registry; retire `DataCatalogueIdentity`; generated OpenAPI and SDK models | — | Golden registry fixtures; rejection of unknown domains and parents, cycles, excess depth, redefined built-in domains and reserved-domain violations; a default organisation keeps every existing registry digest; membership moves keep record identity; old stamps read old membership; rrflowMX and rrflowKV agree after reopen |
| H-11b | Record tags as the reserved `rrflow.tag`/`rrflow.tagged` family, validated in the same commit | H-11a | Cardinality, vocabulary and target violations fail; retirement keeps history; adjacency returns each tag's candidates |
| H-11c | Scoped reads in rrflowQL (tag predicate), `context-assemble`, `vector-search`, `data-schema-read`, `diagnostics-read` and changefeed, applied as pre-filters | H-11a, H-11b | Golden query vectors; scoped results equal an independent full-replay oracle; a path that cannot pre-filter refuses; results name scope and revision |
| H-11d | Isolation and domain grants: the domain isolation mode, `ResourceKind::Domain` replacing `Namespace` and `Database`, isolated read, traversal, retrieval, changefeed and relation-write enforcement, and isolated lexical statistics | H-11c | Every surface is denied without the domain grant and allowed with it; crossing relations are refused unless allowed; an engram-wide grant does not reach an isolated domain |
| H-11e | Per-domain evidence in diagnostics and retrieval evidence | H-11c, H-01, H-05 | Fixture counts reconcile with an independent recount; unknown values stay unknown |
| H-11f | Attunement proposal from the project tree, and organisation experiments | D-03, H-09 M5 | A proposal changes nothing until accepted; an experiment promotes only past both thresholds |
| H-11g | Connectome projection (H-06) and the PostgreSQL schema projection (H-08) | H-11a–e, H-06, H-08 | Both show the same organisation and values at the same read stamp |

## Proof standards

- Every organisation read gives the same answer at the same read stamp after
  close and reopen, on rrflowMX and rrflowKV.
- A scoped read examines no more work than its scope requires, and its
  physical evidence shows that.
- No claim that organisation improves answer quality is made until a measured
  experiment shows it for a named domain and question type.

## Open decisions

| Decision | Options | Blocks |
|---|---|---|
| Whether isolated domains may later use separate storage roots or encryption keys | Logical isolation only (this contract); or a physical isolation profile | Beyond H-11d |
| Bundled category templates offered at setup, such as lifecycle, component or quality | None; or optional templates shown as proposals | H-11f |
| Whether an open category's new tags need review before use | Immediate within the grant; or proposal then approval | H-11b |
