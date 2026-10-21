# POAM-020 — canonical project, estate, and instance topology

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-020`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

The installed product now has one active project/estate/instance representation:
`.rrflow/config.toml` locates the engine-owned installed record and every product caller
opens it read-only. `.rrflow/instance.toml`, `InstanceManifest`,
`ProjectAuthorityBinding`, their raw-root openers, and all product creation callers are
removed. Two semantic conflicts remain: public `PLATFORM_TERMS`/`ResourcePath` mirrors a
superseded historical organization/estate/project hierarchy while accepting arbitrary
resource-kind order, and `EstateAuthorityState` defines another
organization/account/project/environment/instance/node/shard hierarchy. Environment is
absent from the public resource vocabulary, cluster identities use another loose string
family, and the remaining shapes are not yet proven equivalent across rrflowMX and
rrflowKV.

## Impact

Product startup can no longer manufacture its own installation authority, but resource
authorization can still depend on historical word order, an estate can be mistaken for a
multi-project fleet, physical placement can diverge from semantic scope, and unfinished
estate/cluster consumers can resolve a different hierarchy from the installed identity.

## Owning gates

A-07, B-04, C-02, C-03, D-01 through D-03, D-06, H-04, H-07, J-01, J-03, J-05

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate B](../../roadmap/rrflow-1.0/gate-b.md),
[Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

The [independent installation identity package](../../evidence/change-journals/gate-d/d-01-independent-installation-identities.md)
adds an explicit OS-random identity input, deterministic version-2 plans and
an initial-root digest checked before new apply or interrupted recovery.
The same content no longer implicitly allocates one estate for distinct
projects. Its tests cover foreign-plan/credential rejection, independent
installed identities, relocation and eight-stage recovery. Caller reuse of an
identity input is not authorized project sharing; a new independent project
must allocate a new value. This repair does not close the topology vocabulary,
fleet, cross-surface or synchronization deficiencies above.

Freeze one project ↔ estate ↔ instance relationship graph and operation-specific resource
grammar; retain only the implemented D-01 locator and engine-persisted installed-estate
binding plus their strict identity, containment, digest, relocation, and foreign-state
checks; remove historical-table conformance, arbitrary path ordering, and the duplicate
estate topology; then pass fresh/existing preview/apply, MX/KV
semantic, unauthorized/nested/neighbor/escape/foreign/move, cross-surface identity,
clustered-placement, restart, and offline release tests.
