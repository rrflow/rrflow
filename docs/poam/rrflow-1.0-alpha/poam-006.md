# POAM-006 — installed product and attunement lifecycle

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-006`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

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
The estate-controller
backup, prune, and restore paths resolve a managed instance through its installed locator
and distribution binding and act on the instance's own catalogue and staging roots; the
estate cannot yet register backups the served engine created, so a retention decision over
such a catalogue is refused.

## Impact

The [D-01.3 real-process oracle](../../evidence/change-journals/gate-d/d-01-install-exclusion-oracle.md)
reproduced concurrent recovery of a live install intent: a second process
completed the same plan while its creator was paused before storage admission.
The storage writer lock does not protect the whole installation interval.
The [D-01.3 repair candidate](../../evidence/change-journals/gate-d/d-01-exclusive-installation.md)
now retains a native project lease through acknowledgement, with competing-plan,
process-death and live-recovery tests. Its exact candidate passed the full CI
pipeline as recorded below. Broader D-01.5 native qualification remains required;
this does not close the parent gap.

RRFlow has a coherent local pre-release startup spine, but it is not yet a repeatable
turnkey distribution. It cannot discover and attune a project durably, perform the full
repair lifecycle or remove only owned integration. Installed backup resolution and staged
restore publication have [bounded D-11 evidence](../../roadmap/rrflow-1.0/gate-d-evidence.md#d-11-partial-evidence-2026-10-18-backup-completeness-and-restore-publication).
Native staged processes and isolated Linux offline execution have bounded proof;
complete native filesystem qualification and signed clean-machine bundles remain open.
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
Inspection at `1815af32` verifies that estate-directory creation still uses
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

## Private estate-directory admission (2026-10-21)

A native Windows regression demonstrated that exact installation recovery
accepted permissive permissions on `.rrflow/rrd/roots`. D-01.1 supplies private
creation and handle-based ownership/mode/DACL admission for the four
engine-owned estate directories. Apply checks existing boundaries before
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
