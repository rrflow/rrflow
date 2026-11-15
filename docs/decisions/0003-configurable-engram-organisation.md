# ADR-0003: Configurable engram organisation

**Status:** active accepted architecture decision
**Coordinate:** `rrflow://rrflow-instance/data/decision/0003-configurable-engram-organisation`
**Owner:** how one project engram is organised into domains, categories and tags, isolated where required, and measured per domain
**Decision date:** 2026-10-27

## Context

One project has one engram, its logical rrflowDB
([instance topology](../architecture/instance-topology.md)). A real project
is rarely one subject. It has services, packages, product areas and kinds of
knowledge that behave differently. Today the engram has no layer that
represents those parts ([POAM-042](../poam/rrflow-1.0-alpha/poam-042.md)):

- every kind sits in one flat schema registry;
- the registry's `namespace`/`database` pair is one inert label;
- every read is engram-wide;
- grants cannot name a part of the engram;
- evidence is reported only for the whole engram.

A client that wants to show the project's structure would have to invent one.

On 2026-10-27 the repository owner directed that domains inside a project
engram are configurable. RRFlow offers a default setup that each project
tunes itself. It groups kinds, lets projects create categories and tags, can
isolate data, and drills down to find bottlenecks and also what is working
well. The design must absorb what current research shows succeeds for
reasoning engines.

A [historical vocabulary note](../history/platform-vocabulary-note.md#rejected-resource-synonyms)
of 2026-08-27 rejected `domain` as a public resource and pointed to
`namespace` for logical isolation. That note is superseded. This decision
replaces its rule.

## Research basis

A survey of primary sources up to 2026-10-27 informed the decision. The
choices below rest on the parts with direct evidence.

- **Filtering must happen inside the index.**
  - pgvector documents that "with approximate indexes, filtering is applied
    after the index is scanned". A condition matching 10% of rows returns about
    4 rows at the default `hnsw.ef_search` of 40
    ([pgvector](https://github.com/pgvector/pgvector)).
  - LanceDB documents the same short-result failure for post-filtering
    ([LanceDB filtering](https://docs.lancedb.com/search/filtering)).
  - Predicate-aware graph search reports 2–1,000× higher throughput at a fixed
    recall ([ACORN, arXiv 2403.04871](https://arxiv.org/abs/2403.04871)).
- **Isolation buys authorisation, retention and noisy-neighbour control, not
  answer quality.**
  - The vector-store documentation covers what isolation costs. Shared
    approximate indexes let one tenant affect another's recall (pgvector).
    Physical partitions cost resources and slow global queries
    ([Qdrant multitenancy](https://qdrant.tech/documentation/guides/multitenancy/),
    [Pinecone](https://docs.pinecone.io/guides/index-data/implement-multitenancy),
    [Weaviate](https://docs.weaviate.io/weaviate/manage-collections/multi-tenancy)).
  - Cognee states the trade-off directly: "node sets are tags, datasets are
    walls", and walled datasets allow "no cross-dataset reasoning"
    ([Cognee](https://docs.cognee.ai/examples/organizing-your-data)).
  - No source measured what reasoning quality is lost by splitting knowledge
    across partitions.
- **Averages hide opposite effects, so measurement must be per part.**
  - On LongMemEval with gpt-4o, Zep improved single-session-preference
    questions by 184% and temporal reasoning by 38.4%. It also lost 17.7% on
    single-session-assistant questions
    ([Zep, arXiv 2501.13956](https://arxiv.org/html/2501.13956v1)).
  - Independent benchmarks find graph retrieval helps multi-hop and reasoning
    questions and often loses to plain retrieval on single-fact questions
    ([GraphRAG-Bench, arXiv 2506.05690](https://arxiv.org/abs/2506.05690);
    [RAG vs. GraphRAG, arXiv 2502.11371](https://arxiv.org/abs/2502.11371)).
- **Hierarchy and community summaries depend on the question.**
  - They help broad sensemaking questions
    ([GraphRAG, arXiv 2404.16130](https://arxiv.org/html/2404.16130v2)).
  - Independent evaluations find them weaker on single-hop and unanswerable
    questions, at a much higher token cost (arXiv 2502.11371).
- **Free-form model-assigned tags have no measured benefit.**
  - A-MEM, Mem0 categories and Cognee node sets publish no ablation that
    isolates tags.
  - Mem0 does not re-tag existing memories when its category list changes
    ([Mem0 categories](https://docs.mem0.ai/platform/features/custom-categories)).
- **Tuning is credible only as measured comparison.** Reported self-tuning
  gains are proxies or uneven across benchmarks, and vendor memory
  leaderboards contradict each other. A configuration change therefore has to
  be compared against held-out evaluation before it is promoted.

## Decision

RRFlow adopts **configurable engram organisation**. It is part of the
engram's schema registry, is enforced by `RrdEngine`, and is measured per
domain. The [engram organisation contract](../reference/data/engram-organisation.md)
owns the exact terms, limits and build order.

1. **Domains group kinds.**
   - A domain is a named part of one engram's catalogue, such as a service, a
     package or a subject area. Domains may nest to a bounded depth.
   - Every record, relation, event, series, geo, object and vector-collection
     kind belongs to exactly one domain.
   - A kind's identity is independent of its domain, so moving a kind is a
     schema revision that rewrites no record. Old read stamps still see the
     old organisation.
   - The engram is the one database. A domain is its catalogue namespace and
     becomes the SQL schema in the PostgreSQL projection. The inert
     `namespace`/`database` registry label is retired.
2. **Categories and tags label across domains.**
   - A category is a governed vocabulary of tags. It declares whether it
     applies to kinds, records or both, and whether a value takes one tag or
     several.
   - Kind tags live in the registry.
   - Record tags are bi-temporal relations from the record to a tag node in a
     reserved family. Native adjacency then gives every tag a candidate set,
     each assignment carries provenance, and a removed tag is retired rather
     than deleted.
   - A model may propose a tag. A proposal is applied only through an
     authorised write.
3. **A scope is applied before candidates are generated.**
   - rrflowQL, context assembly, vector search, catalogue reads and
     diagnostics accept a scope of domains and tags.
   - The engine applies it as a pre-filter through native indexes or
     adjacency. It never filters after a bounded search.
   - Every result names the scope it read and reports truncation explicitly.
4. **Isolation is optional, per domain, and fails closed.**
   - Shared domains are the default. They behave as one engram, with atomic
     cross-domain commits and reasoning across domains.
   - An isolated domain is reachable only through grants that name it. Grants
     for the whole engram do not reach into it.
   - A relation may cross an isolated boundary only when the domain allows
     that relation kind and the writer holds both grants.
   - An isolated domain keeps its own lexical statistics, and its vectors
     never share an approximate index with another domain.
   - Isolation is chosen for authorisation, retention, forgetting or
     contamination control. Nobody has measured it as a quality improvement.
5. **Evidence is reported per domain.**
   - Diagnostics and retrieval evidence report, for each domain:
     - kinds, records, internal and crossing relations, vectors and storage;
     - query and retrieval counts, latency by stage, and work done;
     - returned, delivered and used context;
     - denials and truncations.
   - Where H-02 records verified outcomes, evaluation results are also split
     by question type.
   - An unknown value is shown as unknown.
   - This is how an operator finds both bottlenecks and what is working.
6. **The default works with no setup, and tuning is measured.**
   - The bundled initial revision has one project domain, `default`, plus the
     reserved `rrflow` domain for engine-owned families. It has no categories.
   - Attunement may propose domains from the committed project tree, such as
     workspace members, packages and service roots. Community detection over
     the relation graph may also propose them. A proposal becomes the
     organisation only through an authorised revision.
   - A proposed reorganisation can be run as an experiment against held-out,
     per-domain evaluation before it is promoted. The experiment uses the
     machinery of the
     [prompt graph families](../reference/data/rrflowdb-capability.md#prompt-graph-families).
7. **Derived structures attach to domains as projections.**
   - Community or tree summaries and code graphs are optional derived
     projections of a domain's canonical data. A code graph uses one hop by
     default.
   - Retrieval routing uses summaries only for broad sensemaking questions.
   - These projections never become canonical truth or bypass the domain's
     scope and grants.

## Consequences

- A project can see and work in its own structure. Connectome and SQL clients
  show the same domains at the same read stamp, and no client invents a
  grouping.
- Grants, budgets and evidence gain a stable unit smaller than the engram, so
  a bottleneck or a success can be traced to one domain.
- Reorganising stays cheap and reversible: membership is a revision, not a
  data migration.
- Scoped retrieval needs native pre-filtering. Vector search must support
  filter-aware candidate generation over domain and tag candidate sets, and
  the E-04 vector serving work inherits that requirement.
- Isolation adds enforcement points to every read, traversal, retrieval,
  changefeed and relation write. The whole engram stays one transaction
  authority and one backup unit.

## Rejected alternatives

- **Inherited namespace/database containers.** Each container would be an
  isolated sub-catalogue with no relations or transactions across it. That
  makes isolation the default, although the evidence supports it only as an
  exception, and it brings back the containers the topology record rejects.
- **A grouping kept only in the client.** It would be a second source of truth
  that grants, scopes, evidence and other clients cannot see.
- **One engram per domain.** It would split one project's knowledge,
  transactions and reasoning across instances. Project links already cover
  knowledge that is genuinely separate.
- **Free-form model-assigned tags applied automatically.** Nothing measures a
  benefit, and the vocabulary would drift.
- **Kind names qualified by their domain.** Moving a kind would change its
  identity and every reference to it.
- **Filtering after the index search.** It returns silently short results.

## Links

- [Engram organisation contract](../reference/data/engram-organisation.md)
- [Instance topology](../architecture/instance-topology.md)
- [Schema catalogue](../reference/data/schema-catalogue.md)
- [rrflowDB capability record](../reference/data/rrflowdb-capability.md)
- [Security authority](../reference/security/authority.md)
- [Connectome client contract](../reference/client/connectome.md)
- [ADR-0001 single-engine authority](0001-single-engine-authority.md)
- [ADR-0002 adaptive reasoning with governed effects](0002-adaptive-reasoning-governed-effects.md)
