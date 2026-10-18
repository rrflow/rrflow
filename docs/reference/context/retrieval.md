# RRFlow recursive retrieval algebra

**Status:** active implemented engine algebra; persistent native recall execution remains incomplete
**Coordinate:** `rrflow://rrflow-instance/data/reference/context/retrieval`
**Owner:** bounded recursive keyword/vector retrieval, fusion, reranking, result shaping, and stage evidence through `RrdEngine`

`RrdEngine::execute_retrieval_query` is one authenticated semantic operation
for composing named-vector and keyword recall. It prevents applications from
building their own vector, keyword, recommendation, and reranking coordinator.
The operation is real and replays after reopening persistent engine state, but
its current physical execution is materially short of the Gate E/F/H target.

This record owns the recursive retrieval algebra. It does not redefine vector
search, rrflowQL, graph traversal, context-packet assembly, or reasoning-tree
execution.

## Authority and snapshot

The engine requires both `query_execute` and `vector_search`, captures one
`ReadStamp`, resolves one vector-collection definition, and evaluates every
branch against that logical stamp. Stored examples identify an exact vector
record, so a subject with several named representations is not ambiguous.
Conflicting active vectors or conflicting payload values fail closed.

Every requested filter, boost property, group key, and facet key must have an
active typed payload-index definition in the collection catalogue. Today that
check governs allowable fields; it does not prove a native physical payload
index was used. Filtering, property merging, grouping, and faceting currently
operate over materialized `RuntimeProperties`.

## Current physical flow

The operation first reads every vector version in the scope at its read stamp
(`read_vector_versions`, a whole-family read rather than a cursor-zero replay)
and builds an in-memory vector candidate set. Physical work is then repeated by
some leaves:

```text
ExecuteRetrievalQuery
        |
        v
RrdEngine authorization + one ReadStamp + collection catalogue
        |
        v
whole vector-version read -> in-memory collection vectors
        |
        +--> nearest ------> search_vectors_at -> another whole vector-version
        |                    read plus two catalogue replays from cursor zero
        |                    (POAM-028)
        |
        +--> keyword ------> stamped rrflowQL pipeline -> materialized batches
        |
        +--> recommend/discover/context -> in-memory exact scoring
        |
        v
Rust RRF -> ordered in-memory rerank -> bounded result shaping
        |
        v
result + plan digest + ordered stage evidence
```

Multiple leaves still share the same validated stamp, but the old claim that
canonical changes were reduced only once was incorrect. Each nearest branch
invokes the current vector search path, which reads every vector version again
and replays both vector catalogues from cursor zero
([POAM-028](../../poam/rrflow-1.0-alpha/poam-028.md)); the keyword branch
performs its own stamped query. Structural request bounds do not yet
constitute one cross-operator physical I/O budget.

The keyword branch delegates to `StampedQueryPipeline` and consumes its
materialized result batches. The enclosing recursive program, RRF, reranking,
grouping, faceting, and matrix calculation are Rust collection operations,
not a DataFusion physical plan. Graph traversal is not a `RetrievalQuery` leaf.
F-03 must compose graph, BM25, vector, and RRF operators over stamped Arrow
batches; H-01 must make the engine select eligible recall avenues from context
intent and budgets.

## Recursive query contract

The public Rust contract supports:

- `nearest` over dense, sparse, or multi-dense named vectors, with current
  exact or eligible approximate vector paths;
- `keyword` through the stamped rrflowQL match query;
- `recommend` with average-vector or best-score positive/negative examples;
- `discover` with a target and ordered positive/negative context pairs;
- `context` with context pairs and no target;
- recursive prefetch plus weighted reciprocal-rank fusion; and
- ordered score-boost, exact, model-bound, and maximal-marginal-relevance
  reranking.

The schema rejects unknown fields and bounds depth by 8, nodes by 128,
prefetch branches by 16, examples by 256, rerank stages by 16, result and
candidate limits by the vector-search ceiling, and matrix samples by 1,024.
Weights and boosts use bounded fixed-point integers. Invalid shapes,
non-finite vectors, dimensional drift, model mismatch, empty inputs, and
amplification beyond the declared bounds fail validation.

Sparse and multi-dense values are first-class exact representations in this
algebra. A named vector called `image` demonstrates collection composition but
does not prove image ingestion or an image-embedding backend. Sparse exact
scoring does not establish a persistent sparse inverted index.

## Fusion, reranking, and shapes

Weighted reciprocal-rank fusion merges by subject. Each contribution retains
its stage ordinal, rank, and score; ties resolve by canonical subject identity.
Exact and model rerank use the selected named vector, model rerank enforces the
exact embedding binding, multi-dense scoring uses directed MaxSim, and MMR is
a deterministic greedy relevance/diversity pass.

Result shaping supports points, bounded groups, bounded facets, and a bounded
directed similarity matrix. Groups and facets summarize only the already
bounded candidate universe; they are not whole-estate OLAP aggregations.

The result carries the read manifest, cursor, full query-plan digest, ordered
stage evidence, and per-hit contributions. That evidence is returned to the
caller; this operation does not itself persist the query, result, feedback,
context packet, or reasoning-tree transition.

## Evidence and remaining work

`rrd-contract/tests/public_contract.rs` proves the strict recursive schema and
bounds. `engine::tests::vector_index::unified_retrieval_algebra_executes_multimodal_late_interaction_and_analytics`
proves keyword+dense+sparse RRF, exact and model-bound reranking, MaxSim,
payload boosting, MMR, recommendation, discovery, context, groups, facets,
matrix output, and equal results after rrflowKV reopen. Permission tests prove
that either missing permission denies execution.

Only the embedded Rust engine capability is implemented; the current product
capability catalogue marks outward HTTP, WebSocket, SDK, CLI, and MCP surfaces
as planned.

C-04 must remove the remaining whole-log reads, the two catalogue replays of
POAM-028. E-01 through E-05 must provide native graph,
payload, BM25, vector, and cost-selected paths. F-01 through F-04 must stream
and budget one Arrow/DataFusion physical plan. G-05 and H-01/H-02 must lower
model context intent into that plan, keep query-time RRF pure, persist verified
feedback only for later stamps, and connect the result to engine-owned context
and reasoning state. Until those gates pass, this is a useful semantic oracle,
not the complete persistent multimodal reasoning-and-recall engine.
