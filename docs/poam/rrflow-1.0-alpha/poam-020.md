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

### Distinct empty projects currently collide at installation

The D-01 offline-qualification inspection reproduced two different empty
project directories receiving exactly the same installation plan digest and
project/estate/instance IDs from `rrflow install plan --mode fresh`. The
observed executable SHA-256 was
`6853715de7b816f7e9c1e921ceb670f6350bc2fb0d088e0d5cb30df6fb7d0623`.
`installation_identity_seed` binds mode, inventory, profile, executable and
configuration digests, but no independently selected installation identity.
Identical existing-project content has the same structural risk. Separate
credential generation does not correct the duplicated logical identity.

D-01 must bind an independent, previewed installation identity while retaining
deterministic planning, exact apply/recovery and portable installed locators.
Changing a live root path cannot redefine an installed estate. Qualification
must include two identical-content projects, foreign-plan/locator/credential
denials and unchanged identity through relocation and reopen. This is an open
deficiency, not repaired by the offline harness.

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

Freeze one project ↔ estate ↔ instance relationship graph and operation-specific resource
grammar; retain only the implemented D-01 locator and engine-persisted installed-estate
binding plus their strict identity, containment, digest, relocation, and foreign-state
checks; remove historical-table conformance, arbitrary path ordering, and the duplicate
estate topology; then pass fresh/existing preview/apply, MX/KV
semantic, unauthorized/nested/neighbor/escape/foreign/move, cross-surface identity,
clustered-placement, restart, and offline release tests.
