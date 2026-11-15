# POAM-017 — engram-administration authority

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-017`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

Local engram administration and recovery use `rrd-engram::LocalOperatorPolicy`, a second
file-backed identity/permission authority. Static engine methods accept caller-selected
database, authority-instance, policy, key, state-root, and authorization-time coordinates,
open a new engine after the package-local check, mutate `EngramRepository` directly, and
append broad `EngramAdmin` audit records without evaluating canonical `SecurityState` or
`RrdOperation::EngramAdmin`. The existing “one authority” test succeeds even though its
canonical principal has no engram-admin grant.

## Impact

Local and remote callers can receive different authorization decisions; a caller controls
the credential-validity timestamp and physical target; policy changes have no canonical
revision/stamp; the seven real least-privilege distinctions collapse in audit; engram state
and audit remain split; and the operation cannot satisfy install resolution or
rrflowMX/rrflowKV semantic equivalence.

## Owning gates

A-07, C-02, C-03, D-01, D-06, H-04, H-05, J-01 through J-03, J-05

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Absorb the seven implemented engram permission/effect distinctions into canonical security;
resolve an installed engine and credential reference; use engine-observed time and one
policy/transaction stamp; remove the local policy/permission/authorization types and
arbitrary-path entry points; preserve denial before unauthorized creation through the
install trust boundary; then pass local/remote and MX/KV differential, denial, crash/reopen,
secret, audit, and clean-install tests.
