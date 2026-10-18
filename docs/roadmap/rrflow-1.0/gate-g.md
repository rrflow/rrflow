# Gate G — configure LFG, SWAG and governed learning within RRFlow

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-g`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | G-01 | Add a provider-neutral `RouterBackend` capability separate from `EmbeddingBackend`; compose SWAG as the optional System One controller/gateway and LFG as its optional CPU classification input through explicit configuration. | `rrd-inference`, `rrd-engine` | Classifier and controller adapters pass their own descriptor, bounds, timeout, invalid-output and digest checks; disabled optional extensions preserve core engine readiness. |
| [ ] | G-02 | Build a bounded route packet from one stamped tree, eligible recipes, verified observations, and allowed query fields. Bind the authenticated principal, current provider-representation edge, resolved durable seat, policy revision, and authorization digest at that same read coordinate; a caller-supplied actor label is never identity evidence. | `rrd-engine` | Golden packet and denial corpus prove representation establishes attribution rather than permission, stale/revoked/foreign representations fail closed, and the model sees no raw KV keys, secrets, hidden reasoning, unauthorized fields, or unbounded workspace content. |
| [ ] | G-03 | Grammar-constrain model-backed SWAG controller output to the three routing decisions and validate again after decoding; LFG observations remain typed classification inputs. | SWAG controller adapter | Corpus includes valid, malformed, unknown-recipe, unauthorized-query, stale-cursor, and prompt-injection cases; invalid decisions produce no mutation. |
| [ ] | G-04 | Execute recipe selection and branch navigation on the fast path without rrflowQL/DataFusion; keep deterministic predicates and CAS mutation in `RrdEngine`, and commit the resolved seat attribution with the resulting tree state, audit, and causal trace. | `rrd-engine`, `rrd-store` | Trace and physical-plan evidence show bounded rrflowKV operations, no DataFusion plan, conflict denial, no actor-string impersonation, and correct attributed state after reopen. |
| [ ] | G-05 | Lower `request_context` into semantic rrflowQL/context intent while leaving physical access selection to the engine. | `rrd-engine`, `rrd-query` | Neither extension can select an unauthorized physical index/backend; resulting plan is authorized, stamped, budgeted, and equivalent to a typed SDK request. |
| [ ] | G-06 | Publish model and storage latency separately with task-success, classification-accuracy, controller routing-accuracy, invalid-decision, learning-regression and escalation metrics. | evaluation harness | Reproducible hardware/model manifest and raw samples support every reported latency or quality claim. |
| [ ] | G-07 | Persist versioned, independently enabled LFG and SWAG settings through RrdEngine, referencing the existing endpoint profiles, allowed model/artifact pins, rules, thresholds and resource limits. Connectome reads and configures the same state. | `rrd-contract`, `rrd-inference`, `rrd-engine`, optional Connectome | Authorized configure/read/disable, stale-revision denial, credential-reference secrecy, restart, unavailable-binding and core-readiness cases pass; no client-owned lifecycle or hidden compatibility adapter. |
| [ ] | G-08 | Implement the RROFLOW recursive-learning track from committed outcomes, verdicts and corrections to versioned candidate rules, exemplars, claims or configuration. Keep evaluation, authorized promotion and rollback explicit. | `rrd-engine` over existing reasoning/feedback and Gate I contracts | Reproducible baseline/candidate evaluations bind source/read identity and budgets; regression or denied promotion changes no active policy; accepted promotion and rollback retain attribution, receipts and restart evidence. |

The [inference-extension reference](../../reference/inference-extensions.md)
owns the owner's LFG/SWAG role split and recursive-learning intent. This
revision corrects the earlier LFG-only router wording; it adds no completed
implementation or trained-model qualification. [Knowledge trees and provider
paths](../../reference/knowledge-trees.md) separately track native/adapter
capabilities; an external service is never another RRFlow authority.

## Exit condition

Gate G exits only when the configured classification and SWAG controller
implementations pass their conformance corpus, steer a persisted tree through
engine-owned authorization/dispatch, and the learning promotion/rollback track
passes its acceptance evidence. No extension gains direct storage or planner
authority; a trained LFG model is not presumed where a classifier requires none.
