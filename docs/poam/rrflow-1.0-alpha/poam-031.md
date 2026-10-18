# POAM-031 — installed-estate backups omit the installed control state

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-031`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

As observed at `726117b`, a backup of an installed estate carried its claims, runtime
commits, immutable objects, and the vector-collection and index catalogue control families,
and reported `application_complete` as true. It omitted the installed estate record
(`server/state/install/record`), the install checkpoint, the active estate configuration, the
security policy and audit journal, and the attunement plan and job state. The logical archive
format carried only claim and runtime-commit actions. A restored root therefore held the data
but not the estate: `open_installed` rejected it with "installed estate record is missing", no
principal or credential existed in it, and no publication could promote it into `roots/` without
re-installation. Characterized by `a_restored_root_lacks_the_installed_record_and_security_state`
in the engine's installation tests, which passed while the state was absent.

## Impact

Recovery from a backup recovered records, relations, vectors, and catalogues, but lost the
installation identity, configuration revision, credentials, policy, and audit chain. The
`application_complete` flag overstated what a backup covered, restore publication (D-11) was
blocked, and a restore rehearsal could not reach authenticated readiness.

## Remediation in progress

The backup catalogue is format version 2. An application-complete backup carries the
control-journal entries of the engine-declared families (installed record and checkpoint,
configuration, security policy, audit records and head, attunement, estate documents) as a
content-addressed control-journal archive, and a restore replays them in order, each transition
committed with a replay-progress marker so an interruption resumes without duplication. The
token key and operator credential are files outside the store and are never carried; session
leases, subscriptions, and operation receipts stay out of the scope. Version 1 catalogues are
refused rather than read as complete. `a_restored_root_carries_the_installed_control_state`
replaces the characterization: the restored root holds the record, checkpoint, configuration,
and security policy byte-identically, every in-scope journal entry up to the backup watermark in
order, no transient family, and an audit chain that reads back and closes on its head.

## Owning gates

D-11, J-02, J-03

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md) and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

A backup of an installed estate includes, or binds through an authenticated manifest, the
installed record, install checkpoint, active configuration, security state, and attunement
state, with the token key and every other secret excluded (present); `application_complete` is
true only when that closure is present (present); a restored root opens through
`open_installed` after publication and reaches authenticated readiness (open: restore
publication, D-11); and the characterization test is replaced by the open-installed assertion
(the restored-state assertion is in place; the open-installed assertion lands with
publication).
