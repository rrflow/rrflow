# POAM-016 — security initialization and effect-complete authorization

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-016`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

The [selected-source worker regression](../../evidence/change-journals/gate-g/task-worker-selected-source.md)
exposed a restart limitation on the installed bootstrap principal. The authorized
security-authority replacement accepts adding `ProjectSourceRead` to that
principal, but `installation.rs::with_installed_binding` subsequently compares
its entire grant-action list with immutable `initial_grants` and returns
`ProjectBindingMismatch` when opening the installed engram. Source edits and
replanning are not the identified cause. The separate explicitly granted
principal used in the original PR289 worker test was a test workaround for this
limitation, not the only supported new-engram provisioning route. PR291 adds a
[default-off source-read choice to a new installation's sealed initial plan](../../evidence/change-journals/gate-d/d-01-explicit-source-read-install-choice.md)
and records the controlled installed worker, typed denial, serve/reopen, and
fresh-auth readback proof. It does not change grants after installation or
repair the existing bootstrap-principal mutation/restart mismatch. That
post-install limitation remains open: reconcile governed security changes with
installation binding, or reject unsupported changes before commit, and prove
the resulting public restart behavior. This package does not redesign
initialization or claim that the original-principal post-install mutation path
works.

Security policy, sessions, and audit have useful validation and denial behavior, but
`rrd-security` still exposes a direct `StorageEngine` repository; policy and runtime data
use separate observations; transaction authorization is operation-wide rather than
effect-complete; missing policy enables anonymous loopback application access; and
authorized/domain/completed audit writes are split commits. Initial local security now
enters through the exact sealed installation plan: engine-generated token and operator
credential bytes are recovered or created once, verified on readback, and the installed
record, policy, checkpoint, audit, and outbox are committed as one semantic outcome before
locator publication. The standalone bootstrap helper/binary and its CLI, supervisor, and
Kubernetes callers are removed. Capability-scoped external secret providers, non-Unix ACL
qualification, trusted installation time, rotation/delivery receipts, and effect-complete
authorization remain open.

## Impact

Code can bypass `RrdEngine`, a concurrent policy change can race a data read, a permitted
transaction can contain unauthorized graph/index/vector or field effects, an uninitialized
component fixture can grant loopback application access, and a domain mutation can commit
without final audit/outbox evidence. Platform or provider credential handling remains
unqualified even though the removed product bootstrap paths can no longer initialize a
different engram or grant set.

## Owning gates

A-07, C-01 through C-04, D-01, D-02, D-06, F-01, F-05, H-04, H-05, H-07, J-01 through J-03,
J-05

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate F](../../roadmap/rrflow-1.0/gate-f.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Make security a pure vocabulary and decision dependency of `RrdEngine`; transact policy,
sessions, state, indexes, audit, outbox, and cursors through one accepted rrflowMX/rrflowKV
boundary; bind authorization to the data stamp; authorize every semantic effect before
commit; and replace absence-driven access with D-01's local-only, fresh-target, exact-plan
installation action. Retain the now-implemented one-time engine secret generation and
atomic installed-binding/policy/checkpoint/audit outcome; add capability-scoped/versioned
secret and clock adapters, typed verifiers, and prepared/effect receipts for credential
delivery; then pass MX/KV differential,
conflict, effect-boundary crash/reopen, path/link/ACL/provider-drift, denial, redaction,
cross-surface, secret-accounting, and clean-deployment tests.
