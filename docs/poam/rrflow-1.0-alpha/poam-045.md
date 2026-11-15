# POAM-045 — vector search returns a vector's old version after it moves to another collection

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-045`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Observed at development/main `81fce08e` while writing E-04b's oracle
(`rrd-engine` `engine::tests::vector_index::turboquant_overlay`):

- **The repro.** A vector identity is put in collection `documents`, then put
  again in collection `archive`, with the same valid time. An exact search of
  `documents` still returns the identity, scored with its `documents` version.
- **Why.** `RrdEngine::search_vectors_at`
  (`rrd-engine/src/engine/vector/search.rs`) keeps a canonical version only if
  that version names the requested collection (`vector_matches_collection`),
  and it does this before the exact scorer resolves each identity's latest
  visible version. The identity's newer version names `archive`, so it is
  removed first, and the superseded `documents` version is then the latest
  candidate left.
- **Point retrieval does it the other way round.**
  `RrdEngine::retrieve_vector_points` (`engine/vector/points.rs`) resolves the
  latest valid version first and then filters its address. It notes that "a
  collection/name move must not resurrect an older matching version".
- **Accelerators.** By reading the code: HNSW base candidates and the scalar,
  product, binary and TurboQuant generations are built from the old version,
  and nothing in the old collection records the move. So the same version
  returns through those paths.
- **What the store already has.** The commit path records a move as a removal
  from the old source (`VectorSourceDelta` with `before` and no `after`, in
  `rrd-store/src/access/vector.rs`).

## Impact

A vector that moved out of a collection is still found there, with content the
engram has replaced. Search and point retrieval disagree about which version
is current.

## Remediation

Pull request #211 makes one rule decide which identities a collection holds.
An identity is in a collection at a read stamp and valid time only when its
latest version there names that collection and named vector
(`rrd_vector::latest_versions`, the first step of the visibility resolver, and
the engine's `collection_versions`). These paths resolve through that rule
before they filter an address:

- exact search;
- the TurboQuant fast path;
- point scroll and point retrieval;
- the retrieval query.

Every approximate path, HNSW included, reranks the identities its generation
and overlay propose from those canonical candidates. An exact segment's hits
stand only when the canonical candidates confirm each one; otherwise the exact
scan answers. No artifact format or byte changes.

That rerank alone left the generations proposing a moved identity's replaced
version, which cost recall ([POAM-048](poam-048.md)). It also left a
collection that every identity has left refusing deletion
([POAM-047](poam-047.md)). Pull request #217 fixes both.

The oracle is `engine::tests::vector_index::collection_moves` (seven tests)
and `rrd-vector` `runtime::tests::an_accelerator_never_answers_with_an_identity_that_moved_away`.
The row is Verifying until Gate C and Gate E record that evidence at the
merged revision.

## Owning gates

C-04, E-04

C-04 owns direct current and temporal reads; vector search must resolve the
visible version before it filters an address, as point retrieval does. E-04
owns the accelerator generations and their delta overlay; a move out of a
source must remove the identity from that source's candidates.

Roadmap owners: [Gate C](../../roadmap/rrflow-1.0/gate-c.md) and
[Gate E](../../roadmap/rrflow-1.0/gate-e.md).

## Closure evidence

- **A search oracle.** After moves in both directions, at valid times before
  and after the move, exact, HNSW, quantized and TurboQuant search return an
  identity only from the collection its visible version names, and equal
  point retrieval at the same stamp.
- **Delta parity.** A source's deltas and its search candidates agree on every
  move.
