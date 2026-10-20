# POAM-009 — RouterBackend and LFG execution

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-009`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Reasoning-tree and three-proposal router contracts now include a closed provider-neutral
model manifest, independent runtime handshake, and byte-verified pre-load admission. There
is still no executable `RouterBackend` dispatch, LFG adapter/conformance, installed model
selection, persisted tree execution, or route packet binding authenticated provider
representation and durable seat attribution to the authorization stamp.

Inspection at `554ce4ccf1cb25aceba036dafaf4c8df79c42701` also confirms a
bounded scale risk in the existing embedding path:
`RrdEngine::generate_embeddings_at` holds `embedding_backends`' mutex through
`EmbeddingBackendRegistry::embed_batch` and synchronous adapter execution.
Other backend work and catalogue access on that registry share the lock.
Input/output size validation exists; this inspection is not a measured
throughput regression or proof of a deadlock. The
[G-01a package](../../roadmap/rrflow-1.0/gate-g.md#bounded-reasoning-implementation-packages)
must characterize blocked-backend interference, preserve provenance checks,
and prove bounded dispatch/cancellation before increasing concurrency.

## Impact

Inspection at `3a8828b42a11899d03fd45db0b2d2fd694dc8ce9` found that Gate G
still described LFG as the model router although the canonical terminology
already separated optional CPU analysis (LFG) from System One selection and
dispatch (SWAG). The [LFG/SWAG contract](../../reference/inference/lfg-swag.md)
and revised Gate G correct that planning mismatch. No executable analysis,
SWAG policy, governed runtime endpoint activation or scoped inference-cache
adapter is supplied by the correction. Existing `EndpointProfile` declaration
and validation are not runtime enforcement.

A local model cannot yet safely steer the fast and analytical paths, and routed state cannot
prove which durable seat was authorized to cause it.

Inspection at `8e07399843d4dd55e847dc191e7bda59bc05614a` found no
first-party controller-training pipeline, held-out controller evaluation or
governed model-promotion implementation, and the canonical RROFLOW term had no
owning roadmap package. The [learning contract](../../reference/inference/learning/controller-training.md)
and G-06/I-03/I-07 package refinements assign those missing responsibilities.
Existing model-manifest validation admits the three route-proposal grammar,
not an arbitrary endpoint-selector model or unbound LoRA sidecar. No training,
Spark hardware, runtime admission extension or promotion is qualified here.

## Owning gates

D-05, G-01 through G-06; I-03/I-07 for governed improvement

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate G](../../roadmap/rrflow-1.0/gate-g.md), and
[Gate I](../../roadmap/rrflow-1.0/gate-i.md).

## Closure evidence

Invalid-output and identity-denial corpora, same-stamp principal/representation/seat/policy
route packets, attributed CAS/reopen tests, and separated model/storage measurements pass.

The [research rationale](../../architecture/engine-scale-and-contracts.md)
and Gate G package table define the repair sequence and semantic input/output
checks. Their existence does not close this item.
