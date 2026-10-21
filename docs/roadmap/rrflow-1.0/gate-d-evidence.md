# Gate D accepted evidence

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-d-evidence`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

## D-01 bounded baseline (2026-10-21)

The [remaining D-01 packages](gate-d.md#d-01-remaining-closure-packages)
start from development `main` at
`1815af32a53e161bc5f3d7f0f7b4365462cd9f7a`. These merged packages supply
bounded evidence to preserve, not complete D-01 acceptance:

| Package | Exact candidate and candidate CI | Accepted scope and limit |
|---|---|---|
| [PR 43: standalone offline qualification](https://github.com/rrflow/rrflow-development/pull/43) | `f0312a59a1403379a4e94b7ecacc994fc3c3901a`; [run 39529214804](https://github.com/rrflow/rrflow-development/actions/runs/39529214804), 17 successful checks | Actual staged executable and scrubbed environment; Linux checkout/toolchain masks, private namespaces and network denial with asserted hashes. Native Windows/macOS staging is not offline enforcement proof. |
| [PR 44: independent installation identities](https://github.com/rrflow/rrflow-development/pull/44) | `27eaff4a7b59f0a385ca4800521ec979d52ece4e`; [run 39530221595](https://github.com/rrflow/rrflow-development/actions/runs/39530221595), 17 successful checks | Explicit identity and initial-root binding in plan contract v2; distinct identical-content projects, foreign-plan denial, exact retry and installed relocation. No plan-v1 migration claim. |
| [PR 45: native secret leaf-file boundary](https://github.com/rrflow/rrflow-development/pull/45) | `7267553ff8c315b22a056b836dd4225156562cf6`; [run 39535526639](https://github.com/rrflow/rrflow-development/actions/runs/39535526639), 17 successful checks | Same-handle bounded secret reads, Unix owner/private-mode and Windows protected-DACL handling, hard-link/nonregular-file denial and retained recovery. No complete parent, mount, race or foreign-owner qualification. |

The linked immutable journals record
[offline qualification](../../evidence/change-journals/gate-d/d-01-offline-installed-qualification.md),
[identity implementation](../../evidence/change-journals/gate-d/d-01-independent-installation-identities.md)
and [secret-file implementation](../../evidence/change-journals/gate-d/d-01-native-secret-boundary.md).
Historical passing runs cannot substitute for the complete matrix on the final
D-01 candidate. Directory security, filesystem containment, install-wide
exclusivity, complete bootstrap and native qualification remain open. Neither
D-01, OBJ-07 nor alpha status changes with this evidence reconciliation.

## D-11 partial evidence (2026-10-18): backup completeness and restore publication

- An application-complete backup of an installed estate carries the estate's
  control state as an ordered, content-addressed control-journal archive
  (installed record and checkpoint, active configuration, security policy and
  audit chain, attunement plan and job state, estate documents). The backup
  catalogue is format version 2; version 1 catalogues are refused rather than
  read as complete. Restore replays the archive one transition per entry, each
  committed with a replay-progress marker, resumes after an interruption, and
  fails closed on a target whose control state diverges. The token key and
  operator credential are files outside every family and are never carried.
  (`rrflow/rrflow-development` pull request #10, merged as `5d1f87d`.)
- `rrflow restore plan` and `rrflow restore apply` publish one staged restore
  offline. The plan is deterministic and digest-bound: it names the candidate,
  the active root it replaces, the backup that produced the candidate, and
  read-only evidence of both roots, and it refuses a served estate (the
  read-only inspector cannot take its shared lock under a running writer).
  Apply accepts only the exact digest, holds an exclusive publication lease,
  re-verifies both roots against the plan before any mutation, carries the
  token key into the candidate, renames the active root into quarantine under
  `restores/<instance>/`, renames the candidate into the storage root's own
  name, opens the result through `open_installed`, records the publication
  receipt in its control journal, and passes quick verification. Every stage
  is durable in a private intent beside the candidate; a rerun after a crash
  at any stage completes the same publication once, and a rerun after
  completion returns the recorded receipt as a replay. The previous root is
  never overwritten.
- Evidence: `a_restored_root_carries_the_installed_control_state`,
  `a_staged_restore_is_published_in_place_and_the_estate_opens_from_the_backup`,
  and `an_interrupted_publication_resumes_from_its_recorded_stage` (engine
  installation tests); the rrd-store control-journal archive, replay, resume,
  and divergence tests; and the real-process
  `installed_engine_plans_applies_serves_discovers_authenticates_and_reopens`,
  which installs, serves, commits, backs up, commits again, stages the restore,
  stops, plans and applies the publication through the primary executable, and
  verifies the estate at the backup's runtime cursor with the old root
  quarantined.
- This evidence closes POAM-031. It does not check D-11: full verification,
  repair plan/apply, salvage, uninstall, the estate-controller backup driver
  (POAM-006), the remaining restore and salvage items (POAM-025), and locator
  replacement for a candidate whose configuration revision differs from the
  active locator remain open.
