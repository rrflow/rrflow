# POAM-028 — vector catalogue history replay and C-04 source closure

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-028`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

C-04's accepted evidence records that vector-search admission has no
causal-log source. Two vector catalogues are still rebuilt by replaying the
scope's whole runtime change log from cursor 0, in
`crates/authority/rrd-engine/src/runtime/vector_catalog.rs`:

- `vector_artifact_catalog_entries` pages `changes_since` from cursor 0 at the
  current head, with no `ReadStamp`;
- `quantization_artifact_catalogue_at_read` pages `read_changes` from cursor 0
  at a freshly captured stamp, not the request's.

`reopen_vector_runtime_metadata` runs both inside every vector search
(`crates/authority/rrd-engine/src/engine/vector/search.rs`). HNSW build,
artifact publication preflight, collection deletion, and payload-index
deletion rebuild both catalogues; quantization build, list, activate, and
retire rebuild the quantization catalogue. Recursive retrieval and hybrid
search repeat the search path for every nearest leaf. The `rrd-maintenance`
crate, which has no in-tree caller, rebuilds maintenance-run state the same
way.

C-04's source-closure test
(`crates/authority/rrd-engine/tests/direct_read_source_closure.rs`) did not
search for `changes_since(` or `snapshot_changes(`, and it allowlisted the
per-search `read_changes(` call as an explicit quantization
projection-catalogue rebuild. It now enumerates every public runtime change-page reader
and lists these three replays as known deficiencies of this record; each is
tolerated only while this record is not closed.

## Impact

Vector search cost grows with the scope's whole history, not with the request.
On the Connectome demonstration estate (270 records, 986 relations, 270
vectors), 270 exact searches examined 1,320,583 keys in 46.9 s. Search
candidates come from the request's `ReadStamp` while both catalogues come from
the current head, so the two can differ when a publication commits in
between; no test covers that interleaving. The accepted C-04 claim overstated
closure, so C-04 is reopened.

## Owning gates

C-04 (reopened 2026-10-17), with E-04 for the persistent vector generations.

Roadmap owners: [Gate C](../../roadmap/rrflow-1.0/gate-c.md) and
[Gate E](../../roadmap/rrflow-1.0/gate-e.md).

## Closure evidence

Both catalogues are served by direct stamped reads at the request's
`ReadStamp`, with no change-log source on any vector path; the source-closure
test's known-deficiency list is empty; physical counters show that one vector
search's catalogue work stays constant as unrelated scope history grows, at
retained and current stamps and after rrflowKV reopen; and C-04's acceptance
evidence is recorded again at the closing revision.
