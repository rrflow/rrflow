# Configurable inference extensions and recursive learning

**Status:** active design contract; LFG, SWAG and recursive-learning execution remain unimplemented
**Coordinate:** `rrflow://rrflow-instance/data/reference/inference-extensions`
**Owner:** the distinct LFG/SWAG roles and the recursive-learning track under one RrdEngine

The owner confirmed two configurable RRFlow extensions: LFG and SWAG. SWAG is
the System One controller and model gateway. RRFlow remains the reasoning,
authorization, transaction and context authority. RROFLOW, presented through
rroflow.ai, names the recursive-learning aspects of that same engine, not a
separate database or a replacement Connectome UI.

## Extension contracts

LFG is the optional CPU classification configuration. It supplies bounded,
attributed observations such as task complexity to SWAG. Whether a model is
needed for a particular LFG classifier remains configurable/design work; no
trained model or mock classifier is claimed as implemented.

SWAG is the optional fast controller/gateway configuration. It consumes allowed
observations and current policy, selects a permitted model/capability through
configured endpoints, and proposes or dispatches only effects authorized by
RrdEngine. It cannot choose an unauthorized backend, grant permissions, commit
canonical state directly or become another planner/persistence authority.

The existing EndpointProfile/EndpointBinding work-class contracts are the
endpoint authority. Future persisted extension settings must reference those
bindings and version their own rules, thresholds, resource limits, enabled
state and model/artifact pins. Connectome configures and inspects engine-owned
state; it does not run another routing or learning lifecycle. Disabling an
optional extension must leave core engine readiness and operator access usable.

The existing Gate G router text predates this split. Its revised acceptance
must distinguish CPU classification, fast controller decisions, engine-owned
authorization/dispatch, configuration, failure behavior and measured quality.
An interface, fixture, successful compilation or generated model answer is
not proof that either extension works.

## Recursive-learning track

Learning consumes committed outcomes, corrections, verdicts and source evidence
at a declared read coordinate. It produces a versioned candidate improvement
to rules, exemplars, claims or approved configuration. Evaluation compares that
candidate with the current version under a reproducible corpus and resource
budget. Promotion is an authorized, durable engine change with attribution,
receipt and rollback; failed evaluations cannot silently alter active policy.
No automatic editor/provider hook, vendor-owned state or model-generated text
becomes lifecycle authority.

HydraDB and Nexus capability research informs this track but cannot grant them
RRFlow authority. Any provider retrieval, ingestion, feedback or learning
capability must declare provenance, permissions, budgets, uncertainty and
recovery. Native equivalents must satisfy the same semantic acceptance rather
than merely copy a vendor's names or SDK wrappers.

The next implementation boundaries are the existing rrd-inference backend
ports, public router/reasoning/endpoint contracts and RrdEngine composition.
[Gate G](../roadmap/rrflow-1.0/gate-g.md),
[Gate I](../roadmap/rrflow-1.0/gate-i.md) and
[POAM-009](../poam/rrflow-1.0-alpha/poam-009.md) own their execution and closure.
[Knowledge trees](knowledge-trees.md) own knowledge-source configuration and
provider qualification. This record adds no shipped feature or release-gate
completion claim.
