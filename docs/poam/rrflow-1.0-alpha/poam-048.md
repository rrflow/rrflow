# POAM-048 — an approximate generation keeps proposing a vector that moved to another collection

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-048`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Observed while authoring [POAM-045](poam-045.md) (pull request #211), on the
E-04 stack at E-04d `f34451ee`, with POAM-045's fix applied:

- **The repro.** Use the engram of
  `engine::tests::vector_index::collection_moves`:
  - `a` moves from `documents` to `archive`, and `c` to `mirror`;
  - `b` moves to `archive` from valid time 50;
  - `d` moves out and back;
  - `e` moves in from `archive`.

  Build each accelerator after the moves. Search `documents` at valid time 60
  near `a`'s value with `top_k` 3 and
  `RequireApproximate { exact_rerank: 3, ef_search: 32 }`. Exact search
  returns `e`, `d` and `f`. TurboQuant, scalar quantization and HNSW each
  return only `e`. This was a temporary engine test, run once and not
  committed.
- **Why.** A generation is built from every version that names its collection
  and named vector, and it keeps them all. That covers `ensure_vector_index`,
  `build_vector_quantization_artifact` and the E-04c extend. Nothing in a
  generation records that an identity's later version names another
  collection, so it proposes the replaced versions of `a`, `b` and `c`.
  POAM-045's rerank resolves each proposal against the canonical candidates
  and discards those. The rerank width is spent, and fewer hits remain.
- **HNSW never recovers.** HNSW's incremental advance only inserts versions.
  A graph built before a move keeps proposing the moved identity until a full
  rebuild.
- **Why the build cannot simply record the move.**
  - A version in the old collection that marks the move and is never visible
    needs an empty validity window when the move takes effect at the replaced
    version's own valid time. That is the common case, and
    `RuntimeVector::validate` refuses `valid_to <= valid_from`.
  - Dropping, at build time, every version that a version in another
    collection supersedes at all valid times handles the common case. But
    HNSW maintenance would then have to remove nodes. A move that takes
    effect later in valid time would still be proposed after it takes effect.

### Empty foreign generation on the HNSW read-path candidate

In the unmerged current-main integration of [PR #278](https://github.com/rrflow/rrflow-development/pull/278), a real engine regression moved every vector out of `documents`, then built its empty HNSW generation with a source cursor covering those moves. A different `mirror` collection shared the vector field and contained a record written before that cursor. Exact search returned that record; required-approximate search selected the empty foreign generation and returned `[]`. The generation contains no node from which to infer its source, and the post-generation delta cannot recover the older mirror record. The [integration journal](../../evidence/change-journals/gate-e/e-04-hnsw-read-main-integration-20261107.md) retains the test and baseline. This is a verified candidate defect, not an assertion that this exact path is already on main. The candidate is draft; a source-binding repair and real-engine/reopen/authorization/budget checks are required before it can land.

The same unmerged integration also reproduced a projection-ID collision: separate source addresses `(a-b, c)` and `(a, b-c)` both received `hnsw-a-b-c` from successful index operations. The linked integration journal records the exact test and source identity. A successor source-binding repair must prevent this collision and bind ownership to the durable generation; no separate mainline execution is claimed.

## Impact

An approximate search returns fewer hits than it asked for whenever its
nearest proposals have moved away. It raises no error and gives no evidence
that recall was lost. The loss grows with moves and persists across
generations.

## Remediation

Pull request #217 gives every generation build its collection's view of each
identity's complete history (`rrd_vector::source_history`). That covers HNSW
full and advance, the scalar, product and binary codecs, and TurboQuant full
and extend.

An exit is a version outside the collection that directly follows one in it:
the move the store records as a removal delta. At an exit:

- every held version that applies only from the exit's valid time on is
  dropped;
- the version the collection resolves to just before the exit takes effect
  is copied, with the exit's cursor, ending at the exit's valid time.

That window is never empty, so no format changes. The view's source cursor
covers every version ever in the collection and every exit, held or dropped.
Every build takes it, so a generation's coverage never moves backward, even
when the newest version it held moves out. An engram without moves builds the
same bytes.

A graph cannot remove a node. So when the view no longer holds a version the
active HNSW graph stores, `ensure_vector_index` builds the next generation in
full instead of advancing or replaying it.

A version outside the collection that follows another outside it is not an
exit. The store records no change to the collection for it, so an extended
TurboQuant generation still equals a full build. In that rare case the
generation can still propose the version the collection held before, and the
rerank discards it.

The tests:

- **The #211 recall probe is now permanent.** Five engine tests in
  `engine::tests::vector_index::collection_moves` show that, near a moved
  identity's value at valid times 10 and 60, every generation built or
  maintained after the moves fills its rerank width. That covers HNSW built
  after the moves and HNSW advanced from a graph built before them, scalar
  quantization, and TurboQuant built after the moves and extended from a
  generation built before them.
- **A property test** in `rrd-vector` (`source::tests`) runs 20,000 randomized
  histories across three collections. At every valid time, a collection's own
  resolution of its view finds the identity live with the same value exactly
  when canonical resolution does, apart from that one allowed case.
- **An E-04c test** shows an extended generation equals a full build after
  such a change.

The row is Verifying until Gate E records that evidence at the merged
revision.

## Owning gates

E-04

E-04 owns the accelerator generations, their maintenance and their recall.

Roadmap owner: [Gate E](../../roadmap/rrflow-1.0/gate-e.md).

## Closure evidence

- **A recall oracle.** After moves in both directions and a move that takes
  effect later in valid time, an approximate search near a moved identity's
  replaced version fills its rerank width with identities the collection
  holds. This holds for every generation built or maintained after the moves:
  HNSW (built after them, and advanced from a graph built before them),
  scalar quantization, and TurboQuant (full, and extended from a generation
  built before them). A generation built before a move and not maintained
  since keeps proposing the identity until its next generation, as it does
  for a retirement.
- **Generation contents.** No generation built or maintained after a move
  proposes the replaced version at a valid time where the move applies. The
  one exception is a version outside the collection that follows another
  outside it, which the store does not record as a change to the collection.
