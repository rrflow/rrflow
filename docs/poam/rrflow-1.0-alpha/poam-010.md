# POAM-010 — context routing, feedback, and retention

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-010`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Context routing uses useful components but lacks native incremental BM25/HNSW integration,
adaptive access planning, and verified feedback-policy evolution. Proposed
working/semantic/episodic memory tiers are still policy labels rather than independent
databases, and no authorized retention routine yet converts evidence, access, outcome,
legal/hold, provenance, or project policy into a versioned retirement proposal. Compaction
cannot be allowed to infer semantic deletion from age, access count, an Ebbinghaus score, or
a prompt boundary.

## Impact

Retrieval remains expensive and cannot prove that dynamic routing improves quality;
premature autonomous decay could erase governed evidence while prompt-triggered
flush/reindex work could amplify writes and latency.

## Owning gates

E, H-01, H-02, I-01 through I-03

Roadmap owners: [Gate E](../../roadmap/rrflow-1.0/gate-e.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate I](../../roadmap/rrflow-1.0/gate-i.md).

## Closure evidence

Exact and approximate quality corpus, plan evidence, feedback replay, rollback, and
regression gates pass. A deterministic policy fixture proves tier/priority changes and
retirement proposals are authorized, reviewable, hold-aware, restart-safe, and reversible
until an engine commit; compaction only reclaims versions already made unreachable by
accepted retention state. No request forces a durability flush, full statistics rebuild,
vector retrain, or graph rewrite.


## Temporal context-anchor correctness

The [H-01 regression](../../evidence/change-journals/gate-h/h-01-context-anchor-failure.md)
proved that an expired record could remain a graph anchor during context
assembly and recall its visible neighbor. Requested seeds must be filtered at
the same captured valid-time snapshot used for seed candidates. Graph stage
selection and execution must share that filtered root set; historical requests
must retain their valid neighbors. This bounded correction does not close
native-index, adaptive-routing or feedback qualification.

The [bounded repair](../../evidence/change-journals/gate-h/h-01-context-anchor-repair.md)
uses one visible root set for selection and traversal. The expired/future/
retired/missing, mixed-root and historical corpus passes on rrflowMX and
rrflowKV, with identical persistent packets after reopen. This is context
correctness evidence only; the broader item remains open.
