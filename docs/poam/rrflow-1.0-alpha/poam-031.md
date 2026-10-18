# POAM-031 — installed-estate backups omit the installed control state

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-031`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

A backup of an installed estate carries its claims, runtime commits, immutable objects, and
the vector-collection and index catalogue control families, and reports `application_complete`
as true. It omits the installed estate record (`server/state/install/record`), the install
checkpoint, the active estate configuration, the security policy and audit journal, and the
attunement plan and job state. The logical archive format carries only claim and runtime-commit
actions. A restored root therefore holds the data but not the estate: `open_installed` rejects
it with "installed estate record is missing", no principal or credential exists in it, and no
publication can promote it into `roots/` without re-installation. Characterized by
`a_restored_root_lacks_the_installed_record_and_security_state` in the engine's installation
tests, which passes while the state is absent and must fail once a restore carries it.

## Impact

Recovery from a backup recovers records, relations, vectors, and catalogues, but loses the
installation identity, configuration revision, credentials, policy, and audit chain. The
`application_complete` flag overstates what a backup covers, restore publication (D-11) is
blocked, and a restore rehearsal cannot reach authenticated readiness.

## Owning gates

D-11, J-02, J-03

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md) and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

A backup of an installed estate includes, or binds through an authenticated manifest, the
installed record, install checkpoint, active configuration, security state, and attunement
state, with the token key and every other secret excluded; a restored root opens through
`open_installed` after publication and reaches authenticated readiness; `application_complete`
is true only when that closure is present; and the characterization test is replaced by the
open-installed assertion.
