# Gate D accepted evidence

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-d-evidence`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

## D-01 accepted installed foundation (2026-10-21)

D-01's eight required rows are accepted at candidate
`9b6c357186c495acd807b7efced61234fcdb324a`, tree
`e1a600d45b65da3c1e83e986963ef30f8f18ca39`.
[PR 52](https://github.com/rrflow/rrflow-development/pull/52) merged as
`fe0028c6e0fd616e38e68b59e90ff3ea9420de38` with that same tree.
[Run 39674145683](https://github.com/rrflow/rrflow-development/actions/runs/39674145683)
has 17 successful required jobs. Attempt 1 completed fifteen jobs, but macOS
never acquired a hosted runner and was cancelled with an ARM64 capacity
annotation. Attempt 2 reran macOS and the dependent aggregate gate successfully
at the same revision. This was an infrastructure retry, not a waived test.

The retained [qualification observations](../../evidence/d01-native-qualification.json)
contain all job conclusions, exact artifact provenance, receipt SHA-256 values,
native fixture results, executed installation test names and process-death
stages. Raw uploaded receipt bytes are retained separately:

| Native profile | Actual executable SHA-256 | Retained receipt and result |
|---|---|---|
| Linux x86_64, kernel 6.17.0-1022-azure, ext4 | `652060742c0c5ca47c05c299ed8d15e285f3e8467494248ff9bbb0b23c93ad68` | [Linux](../../evidence/d01-native-installed-linux-x86_64.json): five passed, zero failed/ignored; private mount/network/PID namespaces and absent source roots |
| macOS arm64, Darwin 25.6.0, APFS | `5c15ddf6e8cb87b91e1379bc92c1c7baafd7df817be2243dc1f992514bacab5b` | [macOS](../../evidence/d01-native-installed-macos-arm64.json): five passed, zero failed/ignored; inherited source/network denial, including descendants |
| Windows AMD64, Server 2025, NTFS | `52369c0bd77cd42ad18829009b40dbdb209aa0475f09b0e7d9e96784a1f51ff8` | [Windows](../../evidence/d01-native-installed-windows-amd64.json): five passed, zero failed/ignored; disposable principal, complete source-object ACL denial, WFP and owned job |

Every receipt includes fresh and existing modes, build/catalogue/plan/backup/
read-manifest hashes, commit sequence 1 and exact installed/reopened/restored
runtime cursors 2/4/3. Source-root and IPv4/IPv6 denial are asserted before the
actual cases. Windows additionally verifies 8,051, 499 and 25,517 source objects,
rejects a protected-child ACL bypass, and verifies exact ACL restoration.
Cleanup failure overrides success. These platform profiles are finite native
development-runner evidence; OS libraries and the qualification harness remain
available. They do not qualify a signed clean-machine distribution or arbitrary
hostile-code containment. The [CI operations owner](../../operations/ci.md#candidate-chain)
defines the enforcement and resource limits.

| D-01 requirement | Observed acceptance at the candidate |
|---|---|
| Primary executable and offline independence | All five [actual CLI lifecycle cases](../../../crates/adapters/rrflow-cli/tests/installed_lifecycle.rs) execute on every isolated native profile. The fresh/existing journeys perform version, deterministic plan/apply, authenticated readiness, commit/query, termination/reopen, backup, restore publication and mutation-free quick verify. No companion executable or checkout provides readiness. |
| Pure complete preview | [Bootstrap tests](../../../crates/authority/rrd-engine/src/engine/tests/installation_bootstrap.rs), especially `bundled_bootstrap_closes_observed_files_records_and_inactive_specializations`, assert thirteen paths, ten initial records (eight control entries and two runtime mutations), linked template/specialization identities and empty external process/network/activation lists. `planning_is_byte_identical_and_has_no_project_effect` and actual binary preview preserve project bytes. |
| Identity, mode and exact authorization | [Installation tests](../../../crates/authority/rrd-engine/src/engine/tests/installation.rs) prove independent identical-content identities, root/mode/executable/digest binding, sealed configuration without reread, profile forgery denial and authorized relocation without foreign-state pairing. Bootstrap drift and installed-instruction mutation deny without repair. All corresponding names have passing native results in the retained observations. |
| Native filesystem and credential boundary | Native file/directory owner/mode/DACL, hard-link and symbolic/reparse cases pass. `foreign_owner_is_denied_without_repair` explicitly runs once with zero skips on each OS. [Filesystem cases](../../../crates/persistence/rrd-lsm/tests/filesystem.rs) include Windows junctions, Unix symlinks, explicit Linux bind-mount and macOS mounted-APFS denial. `anchored_installation_preserves_outside_state_at_durable_boundaries` preserves outside sentinels through recovery effects. |
| Interrupted and concurrent apply | [Recovery](../../../crates/authority/rrd-engine/src/engine/tests/installation_recovery.rs) and [concurrency](../../../crates/authority/rrd-engine/src/engine/tests/installation_concurrency.rs) prove competing/live-recovery exclusion, same-plan admission, retained secret identity and one committed outcome across eleven boundaries: lease admission plus ten durable-effect stages. Actual child-process death is tested at every boundary. Component tests use the documented inert executable-identity fixture; the actual primary executable proof is the first row. |
| Installed authority and normal open | Partial/foreign recovery state, missing committed secrets, aliases and incompatible pre-canonical state deny. `installed_open_and_verify_never_repair_a_torn_or_missing_active_wal` rejects both damage variants with the entire project byte inventory unchanged. Lower [inspection](../../../crates/persistence/rrd-lsm/tests/inspection.rs) and [store lifecycle](../../../crates/persistence/rrd-store/tests/installed_lifecycle.rs) tests retain explicit create/open/inspect separation; no successful initializer or create-on-open path substitutes for the installed binding. |
| Persistence and preservation | `exact_apply_reopens_and_quick_verification_changes_no_bytes` and both actual lifecycle modes preserve project-owned content and installed identity. Every platform receipt retains asserted cursor progression 2/4/3, backup/restore identity and restored read manifest. Pending attunement remains pending; this does not claim attunement execution. |
| Evidence and resource limits | Five process-helper parent cases exercise deadlines, output flood, readiness bounds, finite capture and concurrent copy/spawn. Thirteen runner-policy tests enforce exact case/result admission, closed receipt fields and platform probe rules. Commands, readiness, services, streams, aggregate output and cleanup have explicit bounds; receipts contain structural outcomes/digests, not credentials or private chain of thought. |

Native Windows installation has 46 passing cases; macOS has 48 because two
additional Unix symlink cases compile there. Each suite's ignored child fixture
is explicitly invoked by its passing process parents. The separately ignored
privileged ownership and mount fixtures are explicitly executed and required to
pass; they are not silently counted as acceptance. The Linux engine library has
148 passing cases; its ownership fixture is also run separately. Exact test
names and per-platform observations are retained, rather than treating equal
counts as equivalent platform coverage.

[PR 53](https://github.com/rrflow/rrflow-development/pull/53) performs D-01.6
owner reconciliation. The [receipt-retention journal](../../evidence/change-journals/gate-d/d-01-native-acceptance-receipts.md)
and [reconciliation journal](../../evidence/change-journals/gate-d/d-01-acceptance-reconciliation.md)
record review commands, surfaced failures, mechanical checks and limits.
D-01.1 through .5 implementation remains linked from the [package table](gate-d.md#d-01-remaining-closure-packages).
C-07 is next. OBJ-07 remains partial, POAM-006 remains open, and RRFlow remains
pre-alpha. Wider storage recovery/lifetime, native graph/vector access, streamed
analytics, durable reasoning, PostgreSQL compatibility and release qualification
are not implied by this acceptance.

## D-01 bounded baseline (2026-10-21)

The original [D-01 closure packages](gate-d.md#d-01-remaining-closure-packages)
started from development `main` at
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
D-01 candidate. At this historical baseline, directory security, filesystem
containment, install-wide exclusivity, complete bootstrap and native qualification
remained open. The later accepted matrix above supersedes that gap assessment;
the baseline evidence and its original limits remain unchanged.

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
