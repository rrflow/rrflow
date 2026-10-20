# POAM-028 — vector catalogue history replay and C-04 source closure

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-028`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

The following observation describes the 2026-10-17 reopening; repair evidence
appears below.

C-04's accepted evidence records that vector-search admission has no
causal-log source. Two vector catalogues were rebuilt by replaying the
scope's whole runtime change log from cursor 0, in
`crates/authority/rrd-engine/src/runtime/vector_catalog.rs`:

- `vector_artifact_catalog_entries` pages `changes_since` from cursor 0 at the
  current head, with no `ReadStamp`;
- `quantization_artifact_catalogue_at_read` pages `read_changes` from cursor 0
  at a freshly captured stamp, not the request's.

`reopen_vector_runtime_metadata` ran both inside every vector search
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
projection-catalogue rebuild. The correction enumerated every public runtime
change-page reader and listed these three replays as known deficiencies; each
was tolerated only while this record was not closed.

## Impact

At reopening:

Vector search cost grows with the scope's whole history, not with the request.
On the Connectome demonstration estate (270 records, 986 relations, 270
vectors), 270 exact searches examined 1,320,583 keys in 46.9 s. Search
candidates come from the request's `ReadStamp` while both catalogues come from
the current head, so the two can differ when a publication commits in
between; no test covers that interleaving. The accepted C-04 claim overstated
closure, so C-04 is reopened.

## Repair evidence (2026-10-20)

[PR 39](https://github.com/rrflow/rrflow-development/pull/39) replaces both
vector catalogue log scans with authenticated typed version reads. Search
reads candidates, the three catalogue record types, and exact referenced
object identities in one storage transaction at the request's `ReadStamp`
and under its `max_storage_keys`. Existing atomic publication, canonical JSON,
revision-gap and quantization lifecycle checks still validate the result.
Other catalogue inspection/publication helpers use direct reads with an
explicit 100,000-key ceiling; they do not obtain an unbounded history allowance.

The retained-stamp test first failed by selecting generation 2 for a request
bound to generation 1. The repair preserves the expected generation through
publication and close/reopen, and similarly preserves ready/active/retired
quantization state. A 1,024-unrelated-record fixture keeps semantic reads at
four keys and 5,467 decoded bytes. Total reads increase from 28 to 65 keys
because accumulator authentication grows; a retained read examines 37 keys.
These numbers are fixture evidence, not a general constant-time claim.
The [repair journal](../../evidence/change-journals/gate-c/c-04e-stamped-vector-catalogues.md)
records exact commands and limitations.

The two repaired source-closure exceptions are removed. The independent
`rrd-maintenance` log replay remains the one known exception, so POAM-028
and C-04 remain open. Catalogue generation/lifecycle history is still read
within its typed ranges; persistent incremental vector serving, large-estate
resource qualification and current-head generation lookup remain E-04 work.
This package does not implement codebase inventory or establish that a
stored source digest still matches an externally modified working tree.

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
