# POAM-005 — native graph, scalar, lexical, and vector access

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-005`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Native `RuntimeRef` accepts nonempty UTF-8 kind and ID components without NUL,
but public `DataReference` uses the narrower lowercase ASCII `CanonicalId` for
both. Public mutation and anchor operations therefore cannot represent all
native record references. Query source kinds also require an unquoted word.
Expanding those boundaries requires one traced contract and generated-client
change; changing only a formatter does not resolve the mismatch.

The current query grammar also exposes `traverse:` as an explicit graph
source. The requested relational-first experience needs relationship discovery
and traversal behind the same transaction and query authority; a separate
graph-specific query is not evidence of that experience. Its existing temporal
and adjacency behavior must be traced into a relational-facing replacement
before that syntax or its tests are removed.

The query/context result identity previously concatenated unrestricted reference
components with `:`. For example, `(a:b, c)` and `(a, b:c)` both became
`record:a:b:c`; context's candidate map could then coalesce distinct records.
Draft [PR 266](https://github.com/rrflow/rrflow-development/pull/266) repairs
the result projection and invalidates incompatible derived-index selection;
it does not establish the broader public identifier change or close this item.

Accepted C-03 proves atomic multi-family publication/recovery, and C-04's accepted evidence
proves authenticated direct semantic-version selection and exact snapshots without normal
runtime-log reconstruction. Vector search now reads its artifact and quantization catalogues
at the request stamp; maintenance replay and final closure keep C-04/[POAM-028](poam-028.md)
open. rrflowQL traversal now reads native temporal adjacency ranges,
but context graph expansion and memory-engram seat planning still materialize scope
snapshots; scalar/unique state, incremental BM25 postings, and exact/approximate vector generations
still do not form one planner-selected, stamped native access system. Elias-Fano/PFOR
posting compression, TurboQuant_prod, and LSM-VEC-style disk navigation are evidence-gated
candidates, not implemented defaults.

The installed document-and-graph journey originally found that the record
binder rejected a registered schemaless document. The
[document and graph journal](../../evidence/change-journals/gate-h/h-04-document-graph-journey.md)
preserves that observation at its original revision. The subsequent
[schemaless projection repair](../../evidence/change-journals/gate-e/e-01-schemaless-document-query.md)
admits bounded untyped projection and tests a document/relationship join and
reopen; it does not establish dynamic-property predicates or arbitrary join
keys. The typed-document example and this repair do not close POAM-005.

An installed-project Inventory exposed a traversal syntax gap: a snapshot
digest beginning with a digit could not be used as a start ID, and typed
traversal queries did not round-trip that identity. The [query repair
journal](../../evidence/change-journals/gate-e/e-01-traversal-source-identities.md)
tracks the quoted-ID repair and its candidate evidence. This does not close
the wider graph-access or project-cycle requirements.

The existing traversal also replayed its entire accumulated selected history at
every breadth-first depth. The
[incremental reduction journal](../../evidence/change-journals/gate-e/e-01-incremental-graph-reduction.md)
records the reproduced eight-hop case (45 temporal mutation visits for 17
supplied versions) and its replacement with one request-local cursor-aware
reducer. This repairs repeated temporal reduction without closing E-01:
context/seat paths, total memory budgets, eager result/path allocation and the
complete native access qualification remain open. Research-to-boundary choices
are explained by the [scale rationale](../../architecture/engine-scale-and-contracts.md#graph-vector-and-knowledge-research-mapped-to-rrflow).

## Impact

The [schemaless document query journal](../../evidence/change-journals/gate-e/e-01-schemaless-document-query.md)
records the original rejection and the subsequent merged projection repair.
Dynamic-property predicates and joins still require explicit type and
execution semantics. Properties named like built-in fields (such as `id`) are
currently shadowed in record query rows; arbitrary property namespace access
still needs a defined contract. The projection repair does not qualify those cases.
Its local results cannot close this ledger or establish general document-query
support.

Context can remain slow, and a derived index can still become stale, lose exact semantics,
or select an unbounded path until the accepted deltas have stamped materializers, exact
fallbacks, algorithm-quality evidence, and planner evidence.

## Owning gates

E-01 through E-05

Roadmap owner: [Gate E](../../roadmap/rrflow-1.0/gate-e.md).

## Closure evidence

Retain the C-03 atomic/reopen/fault and C-04 direct-read/source-closure corpora; bounded
native readers and materializers must match exact oracles across update, retire, stale,
corrupt, interrupted-build, and reopen. BM25 codec tests compare raw/bit-packed, partitioned
Elias-Fano, and PFOR by partition; vector tests measure exact oracle, recall@k, estimator
error/bias, filtered recall, RSS, update/build/compaction amplification, and exact
rerank/fallback before retaining TurboQuant_prod or disk navigation. Physical counters prove
engine-selected access.
