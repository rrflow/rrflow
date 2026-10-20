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

## Remediation

Pull request #10 (`5d1f87d`) made the backup catalogue format version 2: an application-complete
backup carries the control-journal entries of the engine-declared families (installed record and
checkpoint, configuration, security policy, audit records and head, attunement, estate documents)
as a content-addressed control-journal archive, and a restore replays them in order, each
transition committed with a replay-progress marker so an interruption resumes without
duplication. The token key and operator credential are files outside the store and are never
carried; session leases, subscriptions, and operation receipts stay out of the scope. Version 1
catalogues are refused rather than read as complete.

Restore publication (`rrflow restore plan|apply`) then promotes the verified staging root into
the storage root's own name: the plan binds both roots and the backup by digest and refuses a
served estate; apply carries the token key into the candidate, quarantines the active root under
`restores/<instance>/`, renames the candidate into place, opens it through `open_installed`,
records the publication receipt, and passes quick verification, resuming from a durable intent
after an interruption at any stage.

## Owning gates

D-11, J-02, J-03

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md) and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Present, recorded in [Gate D accepted evidence](../../roadmap/rrflow-1.0/gate-d-evidence.md):
a backup of an installed estate includes the installed record, install checkpoint, active
configuration, security state, and attunement state, with the token key and every other secret
excluded; `application_complete` is true only when that closure is present; a restored root
opens through `open_installed` after publication and reaches authenticated readiness at the
backup's runtime cursor (`a_staged_restore_is_published_in_place_and_the_estate_opens_from_the_backup`,
`an_interrupted_publication_resumes_from_its_recorded_stage`, and the real-process installed
lifecycle test); and the characterization test is replaced by
`a_restored_root_carries_the_installed_control_state` and the open-installed assertions.
