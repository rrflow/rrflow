# POAM-006 — installed product and attunement lifecycle

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-006`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

At main `2cadd3a9045e3c1a2d9f873fe4215343680ab18b`, both bundled install
profiles supply the same fixed eleven-phase attunement sequence when no
proposal is provided. Callers may propose a different ordered sequence, but
the phase enum and checkpoint progression remain closed and linear. The
[adaptive developer-build design](../../roadmap/rrflow-1.0/adaptive-developer-build-design.md)
maps this verified gap, the existing preservation obligations and the required
project-to-handoff oracle. Neither that design nor this entry implements
dynamic selection or changes POAM-006's open status.
At main `81d2166311476ef51703532573908024a61b725a`, the current replan
implementation clones the predecessor's phase list while changing its source
identity, and the public replan validator rejects a differently selected plan.
Consequently it cannot reselect available capabilities after an ordinary edit;
the [D-02 review journal](../../evidence/change-journals/gate-d/d-02-adaptive-attunement-20261107.md)
records the source boundaries and the required successor, replay and
authorization evidence. POAM-006 remains open.
The original Connectome pin and its successor on main
`c1cbff203ecee89013878609d11aed04e89d0e71` both contain eleven static phase
declarations and a fixed duration estimate. A Git search at both revisions
found no references to those constants outside their definitions; visible use
is unproven. The supporting design records the source and reconciliation.

The immediate remediation is D-02's durable attunement executor, then the
D-03/D-04/D-05 real-project workflow and consumed E/F prerequisites in the
[current dependency spine](../../roadmap/rrflow-1.0/executable-dependency-spine.md).
`engine/installation.rs` persists a pending job; its `installation/inventory.rs`
provides installation preconditions, not the committed source inventory.
`EngramConfiguration` is sealed installed configuration, not proof of live global
preference inheritance. Connectome's phase descriptions are `contract-pending`;
its real data lenses do not establish source-generated graph/vector content.
These are verified implementation gaps, not requests for another storage soak.

Close this part only with installed execution, effective-setting origins,
source-to-query evidence, edit/delete invalidation, restart and two-project
isolation. The [Gate D requirements](../../roadmap/rrflow-1.0/gate-d.md#real-project-implementation-and-acceptance)
define the flow. Existing API contracts stay unchanged; an operation that cannot
express a required action is a specific blocker to resolve, not an implied
new endpoint. POAM-006 remains open.

## Interrupted Inventory refresh can mix snapshot generations

On public main `3a72a6b4adc43c842f67a59f4fc91d725eac7580`, a Windows
real-engine regression published a complete 2,048-file Inventory, edited the
sources, accepted a successor plan, and interrupted a later multi-batch
refresh. The completed control head remained unchanged. Its graph still
reached the old root identity, but a `KNOWN HEAD` query for that identity
returned the partially refreshed root digest, including after reopen. The same
query pinned to the completed checkpoint cursor returned the original digest.
[Draft PR #252](https://github.com/rrflow/rrflow-development/pull/252) and its
[evidence record](../../evidence/change-journals/gate-d/d-03-interrupted-refresh-visibility.md)
retain the failing oracle. D-03 owns the repair; D-04 must not treat an
unqualified head query as a complete source snapshot. This does not establish
that a checkpoint-pinned read is broken or close any roadmap item.

The same real-engine interrupted-refresh fixture, reconciled with public main
`d21a1556657fb17e783a4396368250459e03a2e8`, now also demonstrates that the
public all-model `read_data_snapshot` returns the partially refreshed root
before and after reopen while the completed head remains unchanged. An
unrelated newer claim remained visible through rrflowQL. The [cross-consumer
record](../../evidence/change-journals/gate-d/d-03-snapshot-consumer-20261107.md)
retains the failing test and precise limits. A query-only filter would not fix
this second read path; a global rollback would lose the newer unrelated claim.

The [selection-index fault characterization](../../evidence/change-journals/gate-d/d-03-interrupted-refresh-visibility.md#selection-index-omission-fault-injection-2026-11-03)
deliberately removed a newer selection-like event's semantic index entry while
leaving its log and accumulator intact. The current stamped reader returned an
older included event without detecting the omission, including after reopen.
This injected fault is not an observed consequence of normal interruption.
It shows that selection must not rely on returned-event inclusion proofs alone
to establish a bounded, complete and fresh generation choice.

D-01's bounded installed foundation is now [accepted](../../roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21).
The native proof covers Linux, macOS and Windows source/outbound isolation,
owner/permission/path/mount denial, exclusive install recovery and both actual
primary-binary lifecycle modes. POAM-006 remains **Open** for the later D/J
requirements below. Dated entries retain the gaps and results observed at their
recorded revisions; they do not override this current assessment.
The [reference-alignment journal](../../evidence/change-journals/gate-d/d-01-acceptance-reference-alignment.md)
records correction of stale D-01 qualification/specialization statements and
duplicated roadmap counts; those documentation defects do not reopen the
accepted native implementation or close the broader lifecycle gaps.

The source-built primary `rrflow` binary now owns deterministic install plan/apply,
installed serve, authenticated ready, read-only quick verify, and version. Exact-plan
retry recovers across typed durable stages, including bootstrap publication, without duplicating installation
authority, and the pre-canonical manifest, raw product CLI, development supervisor,
standalone security initializer, and server initializer are removed. The lifecycle is
still missing execution of its persisted pending attunement job, a committed deterministic
project-tree inventory, project-integration scaffolding and activated adapter bindings, real attunement phase
executors, configuration apply, full verify, repair, ownership-safe uninstall,
service installation, and a signed offline
platform distribution. Public apply no longer accepts installation time: the engine
seals one versioned `host_system_time` / `unverified` observation, exact retry preserves
it, recovery/replay/open enforce the format-2 rollback policy, and read-only verification
reports typed current/tolerated/exceeded/unavailable evidence. Authenticated time, OS synchronization diagnosis and
authorized correction and runtime-wide temporal high-water rules remain open.
The engram-controller
backup, prune, and restore paths resolve a managed instance through its installed locator
and distribution binding and act on the instance's own catalogue and staging roots; the
engram cannot yet register backups the served engine created, so a retention decision over
such a catalogue is refused.

## Nested repository setup gap

At development source `5b4039fe930332895c1b24138891d39f600030c7`, a
source-built Windows `rrflow` install preview against the actual pinned
`connectome/` checkout failed with `inventory Git input: external or
non-directory Git metadata requires admission`. Its `.git` is a Gitfile naming
the superproject's module directory. The parent inventory identifies its
tracked gitlink but records that entry as excluded and does not descend. There
is no implemented setup choice to exclude it, install a separate instance, or
include it as a source in one instance.

A separate read-only preview on the existing RRFlow checkout failed the private
owner/protected-DACL check. The precise rejected object was not identified; do
not infer that changing the `.rrflow` directory alone is a repair. The
[diagnostic journal](../../evidence/change-journals/gate-d/nested-repository-boundary-decision.md)
records the binary and observed errors. No project state or ACL was changed.

At main `e6f3632b32f1c9be33c1951886a9d9a2be648ddf`, a separate native
Windows fixture with inherited `.rrflow` and `.rrflow/rrd` directories and
existing files reproduces the same protected-DACL error at installation preview.
This identifies a reproducible class of failure, not the exact object responsible
in the real checkout. The [repair journal](../../evidence/change-journals/gate-d/existing-project-admission-windows.md)
retains the failing oracle and limits. The installer repair and real-project
acceptance remain open.

D-01/D-03 and H-06 own remediation and the real-checkout acceptance in
[Gate D](../../roadmap/rrflow-1.0/gate-d.md#nested-repository-setup). It includes
bounded, explicitly authorized metadata roots and source identity, user choice,
exact-plan revalidation, both engram layouts, restart and isolation. No new
gate or completed status follows from this decision.

## Build-plan admission gaps

Verified at `337d136210ff59b2359f898e717166c087dd5bfc`:

- `AttunementStatus`, `ResumeAttunement` and `CancelAttunement` have public
  contract validators/tests, but source search found no engine/HTTP handler
  referencing them. `endpoint_catalogue()` does not advertise those operations.
  `engine/task.rs` explicitly says a task orchestrates nothing. Reusing task
  open/close as ingestion would be an undocumented semantic API change.
- Installation's action enum includes job creation, not authorization for all
  later source/provider effects. Automatic work at server readiness cannot be
  assumed. D-02a must bind activation, authorization and dispatch explicitly.
- Phase plan fields are estimates and configuration digests. They do not prove
  a worker enforces time, memory or retry limits. D-02c must map actual enforced
  budgets, setting ownership and revision behavior before unattended work.
- The global-default/per-project settings UI needs an admitted configuration
  operation; sealed `EngramConfiguration` and endpoint configuration alone do
  not establish it. Public action/field changes remain disallowed pending the
  owner's clarification of whether missing planned operations may be added.

Remediation is the existing [D-02a/D-02c build package](../../roadmap/rrflow-1.0/gate-d.md#project-workflow-build-packages),
not a new lifecycle or a lowered acceptance threshold. Until the public-action
constraint is resolved, installed/UI job control and global configuration are
explicitly **blocked for implementation admission**, not merely unscheduled.
The owner's later implementation instruction lifts the planning hold for the
bounded embedded D-02 package. `read_attunement`/`control_attunement` now operate
on the installed job under existing EngramRead/EngramAdmin grants, with durable
CAS/receipts, lease fencing, cancellation/recovery and a real Connect checkpoint.
No HTTP operation or UI control is added. Inventory fails explicitly as
unavailable, so source ingestion, complete budgets/settings and turnkey dispatch
remain open. [The package journal](../../evidence/change-journals/gate-d/d-02-durable-attunement.md)
records the evidence; this does not close POAM-006 or D-02.
On 2026-10-26 the owner resolved the job-control part of that constraint
([OBJ-14](../../objectives/rrflow-1.0-alpha.md#required-outcomes)): the missing
planned job list, read, status, resume, cancel, receipt-read and job-transition
journal operations may be added to the operation catalogue under existing
grants, so installed/UI job control is no longer blocked for implementation
admission. Global configuration operations remain blocked pending a separate
owner decision. [POAM-040](poam-040.md) tracks the observability gap; no POA&M
status changes.
Other future algorithm choices remain owned experiments at E/F/G/J; they must
not be described as validated decisions before their named evidence exists.

## Impact

### Protected replan receipt-test chronology

The selected-plan dependency's protected run `41156697702` failed its
optional-engine profile: the replan receipt test supplied fixed timestamps
that could precede the previous effect's actual recorded completion. The engine
correctly refused the non-advancing transition. The
[candidate integration record](../../evidence/change-journals/gate-d/d-03-candidate-integration-clock.md)
binds the exact job log, isolated build and correction to the test's chronology.
Production time checks and all receipt/replay assertions remain intact; a local
default-profile pass does not clear the protected failure. Native candidate and
merge-group checks remain required, and POAM-006 stays open.

The broader local profile also exposed four undeclared bounded change-page
reads in the new Inventory tests. The same record classifies those exact
test-only oracles in the existing source-closure allowlist; it does not permit
runtime query/retrieval log replay or weaken the scanner's exact counts.

The subsequent candidate at `61bfe5a6` failed a different test in Linux
engine-authority job `124415867194`: four Inventory-following fixtures assumed
completion within 20 ms. The
[completion-relative test correction](../../evidence/change-journals/gate-d/d-03-completion-relative-tests.md)
binds subsequent effects and read/reopen times to returned job completion while
preserving every original assertion and production enforcement. The native
failure remains retained; local passes do not substitute for corrected
candidate and protected qualification. No POA&M status changes.

The same defect then failed protected run `41258683845` (PR190's queue entry,
job `124707222954`) in `replan_rechecks_inputs_and_fences_the_old_worker`; a later
queue run for that pull request, `41263871154`, passed. Earlier repairs had
corrected one test at a time. The
[file-wide chronology repair](../../evidence/change-journals/gate-d/attunement-test-time-chronology.md)
audits every supplied time in the attunement tests and derives each one that
follows a committed transition from the completion it returned, preserving every
assertion, the call order and production enforcement. A temporary delay in every
transition path now fails none of the 34 tests, where the prior tests failed 16;
at 3 s two tests still fail on real lease and session budgets, recorded there.
Protected qualification remains required, and no POA&M status changes.

### Whole-repository Inventory publication and binary replacement

PR226's [incremental publication record](../../evidence/change-journals/gate-d/d-03-incremental-publication.md)
retains an installed unchanged-input failure: every entry/edge was republished.
Current authenticated fact comparison now permits bounded write reuse, and
private progress binds its exact publication work set before reusing an offset.
Focused and interruption/reopen checks pass; full candidate and real-repository
qualification remain pending. This does not establish zero-content-read
freshness, reduced RSS, safe upgrades or completed D-03 acceptance.

On 2026-11-12 an installed engram made by one build of `rrflow` refused to serve under a rebuilt binary from the same source (`ProjectBindingMismatch`), because the installation binds `executable_sha256`; the [code-knowledge real run](../../evidence/change-journals/gate-d/d-02-code-knowledge-slice-20261112.md) reinstalled for every build. Upgrading an installed engine in place remains unsupported.

The [installed incremental trial](../../evidence/change-journals/gate-d/d-03-incremental-installed-trial.md)
records an initial fresh-engram `ProjectBindingMismatch` before server readiness.
Offline inspection and a separate restart subsequently passed without modifying
the binding; the cause remains unresolved and must be diagnosed before native
qualification. Continuing that preserved engram proved 4,279 initial mutations,
three on unchanged input, six after an ordinary README edit and successor plan,
then three on another unchanged run, with exact reopen and offline verification.
This does not erase the initial failure or establish a repeatable turnkey start.

PR227's [startup binding separation](../../evidence/change-journals/gate-d/d-03-startup-binding-inspection.md)
characterizes an independent lockout: invalid project Git input prevents opening
otherwise valid installed engine state because startup unnecessarily computes
the full source-inspection report. The focused correction preserves all engram
binding checks and keeps source admission on dependent operations; startup and
job readback pass, invalid-input Start denies, repaired input starts, and bootstrap
tampering still denies startup. All 342 engine tests and strict lint pass locally.
The [combined installed trial](../../evidence/change-journals/gate-d/d-03-startup-installed-trial.md)
at `39a7b644` now passes unavailable-input startup/readback, unchanged job after
denied Start, repaired-input admission, and the actual repository's 4,279/3/6/3
Inventory/restart/edit/successor history. Seven installed CLI lifecycle cases pass.
Quick verification passes its 13 checks but explicitly reports current content
different from the original installation precondition; the scheduled Verify phase
did not run. Candidate/protected, platform and upgrade qualification remain open.
The denial is currently surfaced as HTTP 500/internal, and the original
intermittent rejection's exact cause remains unproven.

At `2cfc0bd9`, an installed 1,870-path public RRFlow repository rejected
Inventory at the native 16 MiB physical batch ceiling. Reopen preserved the
Connect checkpoint and cursor; no Inventory completion was published. The
[bounded publication repair](../../evidence/change-journals/gate-d/d-03-bounded-inventory-publication.md)
uses engine-owned observation groups and private resumable progress, with the
complete snapshot/head/checkpoint/receipt published only in the final atomic
transaction. The same source revision in a separately installed candidate engram
passes Inventory, restart, an ordinary-edit successor and second Inventory.
Interruption and edits during partial publication cannot certify completion;
the edited job can accept a successor. This is draft Windows evidence, not
full D-03 acceptance. Final manifests/retirements still have a physical batch
bound, and retention/scale/native-race qualification remain open.

The rebuilt executable cannot reopen the original installed engram because its
hash differs from the sealed binding. That denial is preserved; no locator or
stored binding was rewritten. An authorized, recoverable binary replacement
path is still required before public deployment can promise safe upgrades.
The successful same-revision trial is a separate installation, not an upgrade
or recovery of the old engram. D/J and POAM-006 remain open.

The [D-01.3 real-process oracle](../../evidence/change-journals/gate-d/d-01-install-exclusion-oracle.md)
reproduced concurrent recovery of a live install intent: a second process
completed the same plan while its creator was paused before storage admission.
The storage writer lock does not protect the whole installation interval.
The [D-01.3 repair candidate](../../evidence/change-journals/gate-d/d-01-exclusive-installation.md)
now retains a native project lease through acknowledgement, with competing-plan,
process-death and live-recovery tests. Its exact candidate passed the full CI
pipeline as recorded below. D-01.5 now supplies accepted native qualification;
this does not close the parent gap.

RRFlow has a coherent local pre-release startup spine, but it is not yet a repeatable
turnkey distribution. It cannot discover and attune a project durably, perform the full
repair lifecycle or remove only owned integration. Installed backup resolution and staged
restore publication have [bounded D-11 evidence](../../roadmap/rrflow-1.0/gate-d-evidence.md#d-11-partial-evidence-2026-10-18-backup-completeness-and-restore-publication).
The bounded D-01 native filesystem and isolated-process matrix now passes on
all three development platforms. Wider C-07 recovery/lifetime and signed
clean-machine bundles remain open.
Unqualified process and Kubernetes controllers can still overstate readiness
or maintain lifecycle state outside the finished engine contract.

## Owning gates

D-01 through D-11, J-01 through J-05

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md) and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

The D-01 single-primary-executable fresh/existing lifecycle passes deterministic plan,
no-write preview, apply, authenticated ready, commit, close/reopen, and baseline read-only
verification without checkout companions; old initializer/supervisor/doctor/create-on-open
success paths are absent. Engine-owned anchor/rollback tests and the primary Linux
abrupt-kill/reopen journey are a bounded prerequisite, not closure. D-02 through D-05
prove durable attunement and inventory/index work; D-11 proves safe
repair/restore/uninstall; native clock, filesystem, packaging, and signed release
qualification pass in J.

## Native installed-process coverage

The [native lifecycle qualification receipt](../../evidence/change-journals/gate-d/d-01-native-installed-lifecycle.md)
records a verified test gap: the primary installed-process case was compiled
only on Unix and substituted a manifest file for the distribution executable.
Windows ran only three CLI argument tests. The replacement corpus binds the
actual executable and tests fresh and existing project installation, durable
commit, authenticated readback from a restarted process, and published-backup
readback. It preserves deterministic preview, configured ceilings, exact-plan
denials, live-writer exclusion and credential-redaction assertions.
Platform CI now runs that corpus explicitly. Windows owned-process
termination is not graceful shutdown qualification; signed offline bundles,
hostile-path/ACL proof and the broader D-01/C-07 matrix remain open.

The [standalone offline qualification package](../../evidence/change-journals/gate-d/d-01-offline-installed-qualification.md)
replaces execution directly from Cargo's target directory with staged binary
copies and a scrubbed product environment. Its Linux runner additionally
requires private mount/network/PID namespaces, masked checkout and toolchain
source paths, loopback-only networking and verified artifact hashes. The
candidate result is attached to that package PR; native staging alone is not
proof of offline isolation. The [D-01 closure table](../../roadmap/rrflow-1.0/gate-d.md#d-01-remaining-closure-packages)
keeps directory/filesystem/ACL, install-wide exclusivity and full preview
qualification explicit before the dependency spine advances.

The first native candidate exposed an overlapping-invocation defect: rejected
admission replaced the active thread owner, and completion identified work only
by caller coordinates. A stale completion could therefore clear newer work.
The [invocation correction receipt](../../evidence/change-journals/gate-d/d-01-native-invocation-correction.md)
records deterministic failures and their replacement evidence. Admission now
preserves the occupied reservation, reports a retryable in-progress conflict
with a completed audit, and binds completion to a private per-admission token.
This does not close durable reasoning, generic uncertain-outcome recovery,
or the wider installed-product qualification item.

## Installation inventory resource admission (2026-10-20)

Inspection found that the installation precondition walker collected the whole
project tree before checking `inventory_max_files`, and the content hasher read
to EOF after accounting only the file's earlier metadata length. A large tree
could therefore exhaust memory before refusal; a growing file could consume
more input than the admitted byte count.

[PR 40](https://github.com/rrflow/rrflow-development/pull/40) moves this one
precondition implementation into `engine/installation/inventory.rs`. It streams
directory entries, admits each retained entry before queuing more work, uses an
iterative walk, and sorts only the bounded output. File hashing reads at most
the admitted length and rejects short reads or observed length/mtime drift
between path and open-handle metadata observations. Stable inputs retain the
version-one digest grammar, exclusions, default profile and no-write preview.
The [bounded inventory journal](../../evidence/change-journals/gate-d/d-01-bounded-installation-inventory.md)
records characterization, tests and remaining limits.

This is not an atomic filesystem snapshot, complete link/mount/race proof,
Git-aware incremental attunement, secret classification, a watcher, or a
million-file benchmark. It does not close D-01, D-03 or POAM-006. Default
installation still rejects inputs beyond its explicit bundle profile instead
of silently ignoring source or claiming a complete index.

## Native secret leaf-file boundary (2026-10-20)

A real-file regression demonstrated that the previous token reader accepted
a hard-linked secret. Inspection also found path validation followed by a
separate open, and Windows secret creation inherited its parent ACL without
validating the opened object's security descriptor. These are verified gaps
within the existing D-01 native filesystem/credential item.

[PR 45](https://github.com/rrflow/rrflow-development/pull/45) consolidates
credential and secret-bearing recovery-intent I/O behind one bounded,
handle-validated engine boundary. Native Windows protected ACL creation and
permissive/null/unprotected ACL denial, hard-link rejection, installation
open/verify/replay refusal, and interrupted-intent recovery have concrete
failure oracles. The [package journal](../../evidence/change-journals/gate-d/d-01-native-secret-boundary.md)
binds results and cross-platform candidate checks.

Ancestor-directory replacement, mount containment, complete native ACL and
hostile-race qualification, signed packaging and the wider D-01 matrix remain
open. No alpha outcome or parent POA&M item closes on this bounded repair.

## Remaining D-01 implementation order (2026-10-21)

The [D-01 closure packages](../../roadmap/rrflow-1.0/gate-d.md#d-01-remaining-closure-packages)
now bind the next work to the current implementation and native failure oracles.
Inspection at `1815af32` verifies that engram-directory creation still uses
default Windows permissions, managed-path helpers return names after checking
them, and publication/cleanup and non-secret reads use later path operations.
The installation contract validates eight fixed managed paths, with no complete
bundle-template/specialization effect description. These are remaining gaps
within this item, not additional lifecycle authorities.

Install-wide lease/concurrency coverage is also unqualified: create-new intent
publication and the later storage writer lock do not establish the required
whole-operation proof. This planning review did not execute a competing-process
regression and does not claim a demonstrated concurrency exploit.

Implement directory security first, then anchored filesystem operations,
exclusive installation/recovery, complete bootstrap, native qualification and
exact-candidate reconciliation. Retain PR 43/44/45's bounded successes. The
roadmap owns the detailed order and acceptance; this ledger remains open for
the unresolved D-01 requirements and the later D/J lifecycle work.

## Private engram-directory admission (2026-10-21)

A native Windows regression demonstrated that exact installation recovery
accepted permissive permissions on `.rrflow/rrd/roots`. D-01.1 supplies private
creation and handle-based ownership/mode/DACL admission for the four
engine-owned engram directories. Apply checks existing boundaries before
recovery secrets/store effects; installed locator, credential, open and
inspection paths require all four. Permission drift is refused without repair
or changes to project-owned parents. The existing private-file implementation
owns the native policy; no separate credential or lifecycle authority is added.

[PR 48](https://github.com/rrflow/rrflow-development/pull/48) and its
[journal](../../evidence/change-journals/gate-d/d-01-private-estate-directories.md)
bind the failing oracle, replacement evidence and candidate CI. Retained
ancestor handles, mount/path races, store-owned descendants and complete
privileged foreign-owner qualification remain later D-01 work. This bounded
repair does not close D-01 or POAM-006.

## Filesystem authority handoffs (2026-10-21)

A real manifest regression redirected publication into a substituted Windows
junction after the original root had been opened. D-01.2 replaces subsequent
pathname authority with retained directory capabilities through installation,
rrflowKV/LSM, object/vector and backup/snapshot consumers. Verified payloads
travel as file handles. Private directory policy remains owned by the engine's
existing native helpers. [PR 49](https://github.com/rrflow/rrflow-development/pull/49)
and its [journal](../../evidence/change-journals/gate-d/d-01-anchored-filesystem.md)
bind the failing oracle and exact candidate evidence; the parent item stays open.

D-01.1's implementation candidate `096272357c02371c8f8bc912017d9af9dbe63d90`
passed candidate run `39621697700`, attempt 2, and merged in PR 48 as
`2d7167c9`. Attempt 1 exposed Linux `ETXTBUSY` during staged executable launch;
the failed jobs passed on retry. D-01.5 must qualify concurrent staging and
launch rather than treating retry success as proof that this intermittent
failure is resolved. Install-wide exclusion, bootstrap completeness, native
offline/foreign-owner/filesystem qualification and exact-candidate closure
remain open in the established order.

The first D-01.2 candidate also exposed Windows checkout conversion of the
canonical manifest golden to CRLF. Its byte-exact identity assertion failed
after all six filesystem fixtures passed. The
[checkout correction receipt](../../evidence/change-journals/gate-d/d-01-canonical-manifest-checkout.md)
pins that fixture to LF without weakening the serializer oracle. A new full
candidate is required; no D-01 acceptance is inferred from local tests.


## Complete bootstrap preview (2026-10-21)

D-01.3 candidate `3817cb5ae6d9d100cb6a9a5768c67608cbaeef6a` passed all
17 jobs in run `39647536747`, including native Windows/macOS, Linux offline,
engine suites, SDK conformance and the final gate; PR 50 merged as `6ba9dc5a`.
The broader D-01 matrix remains open.

The [D-01.4 observed-effect oracle](../../evidence/change-journals/gate-d/d-01-bootstrap-preview-oracle.md)
then demonstrated that intent publication creates a path missing from the
accepted installation preview. A separate current-baseline Git checkout
experiment reproduced LF/CRLF conversion of both embedded installation
assets, changing the raw profile digest at the same source revision. D-01.4
must supply actual bundled templates/specialization manifests, close the
file/record/effect preview and pin owned asset bytes. The regression is
intentionally failing until that implementation; no parent status closes.


The D-01.4 [implementation journal](../../evidence/change-journals/gate-d/d-01-bundled-bootstrap.md)
records plan version 4, actual linked generic/Clyffy assets, complete managed
transient paths, bounded initial record declarations and verified instruction
publication/recovery. Default provider bindings remain inactive and project-owned
instructions remain unchanged. Native D-01.5 qualification, exact-candidate
D-01.6 reconciliation and the later attunement/adapter lifecycle remain open.
PR 51 candidate `9cfeeca4` passed all 17 jobs in run `39654924753` and merged
as `7aa10e23`. The [SDK identity correction](../../evidence/change-journals/gate-d/d-01-bootstrap-sdk-identity.md)
records the stale checked corpus and focused repair without weakening the live
harness. POAM-006 stays open.


## Native qualification corpus admission (2026-10-21)

The [D-01.5 first oracle](../../evidence/change-journals/gate-d/d-01-native-qualification-oracle.md)
reproduces incomplete qualification admission: the runner accepts a corpus
missing the help, rejected-development-command or exact-plan/read-only-shape
case, provided the fresh/existing pair remains. The expanded native runner
must require every declared case and its executed result. Windows/macOS
outbound/source isolation, privileged foreign-owner cases, bounded capture and
concurrent executable staging remain within D-01.5; no parent status closes.

The [D-01.5 qualification candidate](../../evidence/change-journals/gate-d/d-01-native-qualification-candidate.md)
repairs corpus admission and adds bounded process capture, closed durable
receipts, explicit privileged ownership fixtures and native isolation runners.
Local Windows lifecycle and helper results are evidence for their named tests;
Linux/macOS enforcement and Windows hosted-principal isolation still require
candidate CI. No D-01 acceptance row or parent alpha outcome is closed here.

The [native coverage completion record](../../evidence/change-journals/gate-d/d-01-native-coverage-completion.md)
records two additional verified qualification gaps: process-death recovery
omitted four of eleven typed boundaries, and macOS lacked an explicit mounted
volume adversary. The candidate expands those cases without changing production
behavior. Native acceptance remains open; local Windows results cannot close
the macOS mounted-volume requirement.

The same native run exposed checkout newline conversion of the inert SDK
identity anchor on Windows. The linked coverage record retains the failing
identity and Git filter reproduction. The fixture is now byte-preserved and
asserts its canonical hash; native candidate requalification is still required.

The [Windows mask-policy correction](../../evidence/change-journals/gate-d/d-01-native-windows-mask-policy.md)
records a second native qualification defect: resolved/unresolved root comparison
rejected the hosted Windows checkout before isolation setup. A current-drive
regression now passes with platform-specific protected roots. Preflight failure
also retains a bounded receipt. Actual Windows principal/WFP enforcement and
the final candidate pipeline remain required; no status is closed.

The [Windows process-context candidate](../../evidence/change-journals/gate-d/d-01-native-windows-process-context.md)
records interpreter initialization failure 0xC0000142 after policy setup. The
runner now supplies an owned noninteractive desktop and probes restricted-token
access to its runtime dependencies. This requires native requalification;
policy setup alone is not proof of isolation or a running product.

That candidate still exits before Python with 0xC0000142. The
[startup matrix](../../evidence/change-journals/gate-d/d-01-native-windows-startup-matrix.md)
separates normal/restricted token and console-creation behavior using bounded
no-op interpreters. Actual qualification retains the restricted token and all
denial requirements; these diagnostics cannot close the native gap by themselves.


The matrix passes the ordinary temporary user's interpreter and fails every
traversal-stripped variant. The [source-object ACL replacement](../../evidence/change-journals/gate-d/d-01-native-windows-source-acl.md)
therefore retains standard Windows startup privileges and requires inherited
denial verified for every bounded inventoried source object, protected-child
bypass rejection and full ACL restoration. This changes the qualification
mechanism, not the required source/network denial. Hosted acceptance remains
pending and this deficiency stays open.


The [IPv6 probe correction](../../evidence/change-journals/gate-d/d-01-native-windows-ipv6-probe.md)
records successful Windows startup, complete 34,064-object source denial and
ACL restoration, followed by an IPv6 network-unreachable result. The probe now
uses a configured non-loopback link-local scope without changing host routes;
explicit IPv6 permission denial remains mandatory. Product execution and final
candidate acceptance remain pending.


The [installed WAL denial characterization](../../evidence/change-journals/gate-d/d-01-installed-wal-denial.md)
adds direct installed-open and verification evidence for torn/missing active
WAL preservation. Both cases pass locally without production changes; the final
native candidate must execute the added test before D-01 acceptance closes.

## Standalone database example

The documented source-built local database path had a reproducible Windows
installation failure: Cargo's primary executable and its `deps` entry were
hard links to the same file, while installed executable admission requires a
single link. A clean source build succeeded, the original path was denied,
and a separately copied executable with the same SHA-256 completed an actual
installed database lifecycle. The security boundary remains intact. The
[standalone database journey journal](../../evidence/change-journals/gate-h/h-04-standalone-rrflowdb-journey.md)
records the guide correction and test; Linux/macOS hosted results, candidate
CI and merge qualification remain pending. This does not close D-01 or H-04.
The D-04 developer journey independently reproduced the same Windows Cargo
hard-link denial; its retained fixture uses a separate executable copy. That
workaround does not establish a fixed direct source-build workflow.

## Endpoint and task contract integration

The [combined integration record](../../evidence/change-journals/gate-d/endpoint-profile-task-control-integration.md)
retains a verified evaluation-projection defect: unified Cargo dependencies
changed JSON object ordering and failed an exact schema golden despite equal
parsed values. Explicit sorted rendering in the existing test/refresher fixes
that representation ambiguity without changing runtime hashing or weakening
byte assertions. Both public feature flows remain at the existing engine
boundary; combined candidate qualification and the wider lifecycle remain open.

## Draft Parse memory admission

The [D-04 parsing draft](../../evidence/change-journals/gate-d/d-04-source-parsing-in-progress.md)
identifies a resource-admission gap before engine activation. Its pinned
Tree-sitter 0.24.7 default allocator aborts the hosting process on allocation
failure and its allocator hooks are process-wide. Input, output, node and time
limits alone do not prove a per-project native-parser memory ceiling. The
draft is not invoked by main's Parse phase. Keep that phase unavailable until
an authorized, isolated or otherwise demonstrably bounded execution path and
failure/cleanup behavior are tested on the supported native platforms; neither
the policy field nor a passing syntax fixture is that proof.
The draft Parse input also reuses a reader that reconstructs the complete
Inventory proposal and bounds it by one transaction's admission size. It must
not be treated as a scalable multi-group snapshot reader: implement and measure
bounded validation/readback for larger snapshots before claiming that path
works at the selected project scale. No 100,000-file Parse result is implied.
Before this draft, the installed-lifecycle reference still said only Connect
executed after bounded Inventory had landed in PR #224; the draft corrects that
sentence while retaining the unimplemented Parse and indexing limitations.

## Linux ARM64 installed evidence gap

At public main `3a72a6b4adc43c842f67a59f4fc91d725eac7580`, the candidate CI
has Linux x86-64, Windows and macOS jobs but no Linux ARM64 native installed
lane. The [ARM64 qualification journal](../../evidence/change-journals/gate-d/d-01-linux-arm64-candidate-ci.md)
records the proposed required lane and its actual results. A checked-in job,
cross-compilation or another platform's pass does not qualify Linux ARM64;
native candidate/merge-group execution and the separate capacity, recovery,
offline and soak evidence remain open.
