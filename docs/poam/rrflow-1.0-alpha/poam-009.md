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

Inspection at `554ce4ccf1cb25aceba036dafaf4c8df79c42701` identified a
bounded scale risk in the existing embedding path:
`RrdEngine::generate_embeddings_at` holds `embedding_backends`' mutex through
`EmbeddingBackendRegistry::embed_batch` and synchronous adapter execution.
Other backend work and catalogue access on that registry share the lock.
Input/output size validation exists; this inspection is not a measured
throughput regression or proof of a deadlock. The
[G-01a package](../../roadmap/rrflow-1.0/gate-g.md#bounded-reasoning-implementation-packages)
must characterize blocked-backend interference, preserve provenance checks,
and prove bounded dispatch/cancellation before increasing concurrency.

[PR 41](https://github.com/rrflow/rrflow-development/pull/41) repairs the
existing catalogue/execution coupling without increasing that concurrency.
`EmbeddingBackendRegistry::try_acquire` returns one owned execution lease
with the immutable installed descriptor and selection-time registry revision.
`RrdEngine` releases the registry lock before input preparation and backend
execution. The existing single-batch ceiling becomes explicit fail-fast
admission: excess calls report retryable `ResourceExhausted` and do not call a
backend or copy its inputs. Listing and installation remain available during
execution. Dropping, rejecting, returning or unwinding the lease releases its
slot; a recognized backend mutex poison is confined to that backend and maps
to `Internal`. Descriptor drift rejects output and later execution while the
catalogue retains its admitted identity. The
[admission repair journal](../../evidence/change-journals/gate-g/g-01a-embedding-admission-repair.md)
records the failing oracle, replacement tests and limits.

This is a repair of the existing embedding surface, not G-01a closure or a
change to the D-01-first dependency spine. One blocked adapter still occupies
the sole execution slot. There is no waiting queue, deadline enforcement,
cooperative cancellation, hard worker termination, credential revocation
fence, configurable parallelism, durable inference lease or RouterBackend
execution. Higher concurrency remains blocked on the named G-01a evidence.
A retryable busy response is not automatic retry authorization or proof that a
provider effect can be repeated safely.

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

Inspection at `ff11edf6ec8da7461955cb4eb349fb81828a2f09` confirms that
`ReasoningLimits` is validated/sealed by installation and exposed by server
capabilities, but no governed reasoning runner consumes its aggregate elapsed,
step and per-step ceilings. Defaults therefore cannot prove prevention of
hours-long work or repeated retries. The
[effort and run-budget contract](../../reference/inference/lfg-swag.md#user-selected-effort-and-run-budgets)
and Gate G acceptance now explicitly require finite user-selected effort,
aggregate accounting, no-progress stopping and non-resetting cancellation/
resume. Connectome must not advertise those controls as operational while
the capability is unavailable. This remains an implementation deficiency;
the clarification does not close this item.

## Owning gates

D-05, G-01 through G-06; I-03/I-07 for governed improvement

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate G](../../roadmap/rrflow-1.0/gate-g.md), and
[Gate I](../../roadmap/rrflow-1.0/gate-i.md).

## Closure evidence

[PR 100](https://github.com/rrflow/rrflow-development/pull/100) adds a configured
deadline to the existing embedding execution path. Late output is rejected and
the admitted worker keeps its slot until actual return. This addresses caller
waiting, not forced adapter cancellation or aggregate reasoning-run budgets.
The [budget-control sequence](../../roadmap/rrflow-1.0/gate-g.md#budget-control-delivery-sequence)
binds those remaining requirements to code owners and falsifying histories.
POAM-009 remains open; controlled adapter tests do not qualify a real router.

Invalid-output and identity-denial corpora, same-stamp principal/representation/seat/policy
route packets, attributed CAS/reopen tests, and separated model/storage measurements pass.

The [research rationale](../../architecture/engine-scale-and-contracts.md)
and Gate G package table define the repair sequence and semantic input/output
checks. Their existence does not close this item.
