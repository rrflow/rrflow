# C-07f owned native capacity and explicit candidate repair

**Status:** active implementation candidate; native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07f-native-capacity`
**Owner:** OBJ-02, C-07f, POAM-035

[PR 71](https://github.com/rrflow/rrflow-development/pull/71) carries the
pre-effect brief. The branch began at synchronized development/main
`72dd666b254f3f403d11f942c1180b108cc40d21` and fast-forwarded the PR 65 prerequisite
through `beb55307a9c54df3eedec45a113084e360a4bdc1`. PR 65 must pass and land
before this dependent package merges. Neither source is promoted to the
official repository. Product version remains 1.0.0.

## Brief and implementation traceability

Current behavior had kill-only installed diagnostic pauses and no installed
explicit torn-tail repair. A partial native WAL write could remain non-ready
after space returned. The failure oracle requires actual native capacity
errors, preservation of every prior ACK and exact receipt, atomic uncertain
effects, explicit recovery where required, distinct subsequent work and reopen.
It cannot accept simulated StorageFull, permanent denial, partial success,
silent truncation, damaged-input overwrite or an unowned filesystem.

The diagnostic-primary scope now supports a single nonce/id-bound private-pipe
resume. Native create/write/sync/rename wrappers observe the original OS error
without injecting or replacing it. Bounded receipts carry PID, phase, operation,
role and native code; paths, credentials and payloads are absent. Ordinary
builds exclude both protocols. The owned process guard closes control pipes,
caps output, and kills/reaps its children.

The existing installed-crash Estate owns installation, real primary staging,
session, request identity and public-client flow. Its capacity extension uses
that owner for a separately admitted small native volume. The corpus fills
only its create-new filler file, requires the selected native error, freezes
the affected process, releases filler space, resolves the unknown request and
checks prior receipt identity and exact rows again after restart. Initial WAL
reservation is consumed by acknowledged operations, not by mutating WAL bytes.
The uncertain request is frozen before failure. Existing C-07a ledger checking
remains the independent result oracle.

ReadOnlyDatabase's existing physical/semantic inspection was factored into one
internal reader. Public ordinary open still rejects torn tails and cannot
mutate them. Its explicit native repair owner alone acquires the exclusive
lease and reads the validated prefix; recovery policy caps remain in force.
It binds the complete physical inventory, full WAL and prefix hashes, tail
extent and recovered sequence. A stale plan, complete corrupt frame or altered
prefix denies truncation. An exact already-truncated prefix permits a sync
retry. RrflowKvInspector and the repair wrapper reuse the same semantic
watermark/format lowering rather than introducing another storage model.

Installed repair is composed through RrdEngine and its locator, executable,
sealed configuration, installed record, token and credential binding. Review
against the lifecycle contract rejected an initial uncommitted in-place
wrapper. The final installed implementation copies to an absent candidate,
changes only its WAL, verifies native physical closure and installed watermarks,
retains the original under content-addressed quarantine, and publishes the
candidate under the existing active name. It reuses restore's publication lease.
A bounded private intent resumes copy/quarantine/publication, an audit in the
candidate binds before/after inventories, and the existing engine control
journal records completion. Completed retries return that receipt. General
locator replacement and full semantic/index repair remain D-11, as with the
current limited restore-publication profile.

New source boundaries are native `tail_repair`, engine `tail_repair/publication`,
installed harness `support/capacity`, and CI-owned volume/process controls.
Characterization retained: ordinary inspection's mutation-free denial,
installed open/verify damage rejection, existing domain crash schedules,
existing restore publication, and primary raw-command rejection. New regression
definitions cover native stale plans, corruption, budgets, ACK prefix and
exact retries, plus installed interruptions after copy, active quarantine,
candidate publication and control receipt. No prior implementation lane is
retained as a fallback.

## Read and effect inventory

Owner/contract reads: README.md; AGENTS.md; alpha objectives; roadmap Gate C and
C-07; execution-map procedure; POAM-035; installed-lifecycle repair, filesystem,
configuration and restore sections; CI operations and reusable workflow.
Relevant implementation reads:

- rrd-lsm database, WAL replay/repair, manifest, inspection, recovery_budget,
  filesystem and file_io, diagnostic crash/installed controls, lib/error,
  recovery_limits and existing MVCC/inspection/compaction characterizations.
- rrd-store rrflow_kv, control, lib and the native options/inspector lowering.
- rrd-engine installation, installation/lease, estate_layout, core options,
  restore_publication, token_key/private_file, clock, module/lib exports and
  installation characterizations.
- CLI command, installed adapter, installed_crash, installed_lifecycle,
  installed_reopen, support/process and response/history helpers.
- C-07a storage model/ledger; run_offline_lifecycle, run_filesystem_containment,
  Windows offline job ABI and cleanup; workflow checker; local sysinfo 0.39.6
  disk APIs used for independent mounted-volume admission.

Changed paths are mechanically recorded by the execution inventory and PR
diff. This record explains intent, replacements and evidence, not another
global path/status authority.

## Native ownership and research decisions

Linux formats only a new capped regular image and mounts it in a private mount
namespace using an autoclearing loop device. APFS uses an owned fixed image;
NTFS uses a new fixed VHD with exact image-to-disk checks before initialization,
partitioning and formatting. No supplied existing mount/disk can be selected.
Windows descendants enter a kill-on-close job while suspended; POSIX harnesses
have owned process groups. Detachment is verified before removing only known
host fixture files and the empty mount directory. Failed cleanup retains the
fixture and fails the case. Missing tools/privilege report not-run, not success.

Primary references: [Microsoft Mount-DiskImage](https://learn.microsoft.com/en-us/powershell/module/storage/mount-diskimage),
[Windows native error codes](https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-),
[DiskPart](https://learn.microsoft.com/windows-server/administration/windows-commands/diskpart),
[Apple disk images](https://support.apple.com/en-gb/guide/disk-utility/dskutl11888/mac),
[Linux ext4](https://cdn.kernel.org/doc/html/latest/admin-guide/ext4.html),
and [util-linux loop cleanup](https://man7.org/linux/man-pages/man8/umount.8.html).
No upstream implementation was copied. Native ENOSPC/EDQUOT or documented
Windows disk-full codes are distinct from application admission, process death,
OS crash and device power loss.

## Commands and actual results

- `cargo check -p rrflow-cli --features crash-test --all-targets --locked`:
  diagnostic control passed (11.70 s); installed repair passed (7.14 s);
  candidate publication passed (5.20 s). These are compilation only.
- `cargo clippy -p rrflow-cli -p rrd-lsm -p rrd-store -p rrd-engine --features
  rrflow-cli/crash-test --all-targets --locked -- -D warnings`: passed before
  candidate refinement (13.29 s), and after candidate/refinement regression
  definitions (10.37 s). Cache directory was the existing declared local
  c07d-propagation-build target.
- Intermediate compiler/lint failures surfaced: private storage_error helper
  not in scope; nonexistent ledger.validate (correct owner is reconcile);
  unused imports; Clippy is_multiple_of. Corrected at the owning call sites,
  without suppression or removal of assertions.
- `python -m py_compile scripts/ci/run_native_capacity.py
  scripts/ci/owned_windows_process.py`: passed. PowerShell parser admission of
  owned_capacity_volume.ps1: passed with no parse errors. These do not execute
  volume effects or qualify OS behavior.
- `python scripts/ci/check_workflow.py`: passed after the required capacity step
  and both ordinary-artifact diagnostic marker exclusions were added.
- Rust formatting, documentation/navigation generation, execution inventory,
  generated-surface check and staged-diff checks are performed before commit;
  exact failures, if any, remain part of the candidate record.

No local runtime test suite, native volume fill, benchmark, soak or privileged
disk operation ran. The owner requested implementation progress without
repeated local runtime campaigns. Required remote candidate CI remains the
merge gate. The existing three-platform installed-crash job now explicitly
executes the owned volume runner; selecting an ignored case or compiling its
source does not count as native evidence.

## Remaining acceptance and checklist

Pre-effect brief, grounded boundary reads, canonical destination map,
characterization preservation, resource/trace/research decisions, negative
oracles, compile/lint outcomes, and unrun qualification are recorded above.
Stop on any failed source binding, unowned device, non-capacity error, changed
ACK/receipt, partial effect, lost preserved input, unbounded work, unsafe
cleanup or failed candidate pipeline. C-07, OBJ-02 and POAM-035 stay open.

Authoring checks also caught the coordinated-record status vocabulary; the
journal now declares active status and the documentation policy passes.
Generated-surface parity passes for 42 HTTP operations, unchanged signal/OpenAPI
identities and seven signal projections. The repair plan bounds logical copy
and metadata writes; it does not claim a physical-allocation reservation.

Native capacity acceptance at this exact revision is pending. The complete
targeted-damage/backup matrix, recovery-space reservation, aggregate process
and host limits, sustained capacity profiles and actual 24-hour soak remain
open. The code does not establish PostgreSQL compatibility, retrieval quality,
provider reasoning, cloud sync, Connectome completion or alpha readiness.

## Candidate correction: public readback windows

Candidate 459e60fd, run 39889580174, Linux installed-capacity job
120205851327 failed before disk fill: the harness asked for 8 MiB query output
while the public query contract admits at most 768 KiB. The readback now
first enumerates every subject, then fetches at most 64 objects per lexical
window at the same captured cursor. Exact cardinality, no truncation, complete
identity/value equality and ACK reconciliation remain required. No public
limit is relaxed. Reads: capacity.rs, installed_crash.rs, installed_reopen.rs,
rrd-contract/src/lib.rs QueryBudget, and rrd-query/tests/syntax_parser.rs.

The same run exposed cleanup masking the primary failure: successful losetup
with no associated device can return empty output after autoclear. The
owned-device cleanup accepts that successful empty listing, while command
failure or ambiguous ownership still fails. The ownership marker read is now
bounded to 65 bytes for a 64-byte nonce. Source: the retained native CI output;
no additional research or architecture change was needed.

`cargo clippy -p rrflow-cli --test installed_crash --features crash-test
--locked -- -D warnings` passed (26.26 seconds). The intermediate check found
the missing public query scope in the new window builder; it was restored
before strict lint passed. Python syntax compilation and generated repository
navigation/inventory checks run before commit. No local native runtime suite
or disk fill was run; corrected candidate acceptance remains pending.

The Windows candidate job 120205851581 failed in the diskpart image-creation
control before installation. The old wrapper discarded its textual native
failure details; the output does not establish a specific root cause. Creation
now invokes Windows CreateVirtualDisk directly with version-one parameters,
VHD device 2, Microsoft vendor identity, 512-byte sectors, and
FULL_PHYSICAL_ALLOCATION. The exact new contained image remains the only
target; no physical parent/source is supplied. Native error codes now survive
as Win32Exception. Existing association checks precede every initialize,
partition, format and detach effect. This replaces only the script-based image
creation, not engine storage or a runtime product dependency.

Primary API references read for this correction:
[CreateVirtualDisk](https://learn.microsoft.com/en-us/windows/win32/api/virtdisk/nf-virtdisk-createvirtualdisk),
[versioned parameters](https://learn.microsoft.com/en-us/windows/win32/api/virtdisk/ns-virtdisk-create_virtual_disk_parameters),
and [fixed-allocation flag](https://learn.microsoft.com/en-us/windows/win32/api/virtdisk/ne-virtdisk-create_virtual_disk_flag).
The new native call is compiled/parsed only locally; candidate NTFS execution
must establish whether the host supports the requested owned image.

## Native candidate boundary corrections

Run 39891012307 reached the actual installed corpus on Linux and Windows, then
rejected its 150,000-key query request against the sealed 100,000-key ceiling.
The bounded windows now also respect the default 512 KiB output ceiling. Reads
included the installed TOML defaults; no engine ceiling was raised.

Windows estate integration exposed a native mandatory-lock conflict while
inspection issued an EOF read on the empty MANIFEST.LOCK through another
handle. Inspection now hashes exactly its pre-admitted extent and rechecks the
length afterward. Empty files need no read; the exclusive repair lease is
retained throughout. Short files, growth and corrupted contents still fail.
Existing native tail-repair and installed interruption/readback regressions
exercise the boundary; this is a storage fix, not a skipped platform assertion.

The APFS control failed before the corpus without retaining its native error;
that evidence is insufficient to identify a root cause. The image runner now
retains the failed native tool and bounded stderr/stdout for owned-image
commands only. Windows detach also returned an empty structured response;
its script now writes and flushes UTF-8 JSON directly instead of relying on a
pipeline followed by process exit. Empty responses still fail qualification.
No new host operation ran locally.

The exact-extent correction passed `cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings` in 6.19 seconds. PowerShell parsing and Python syntax compilation passed. Native runtime acceptance remains candidate-CI work, not a local run.


## Publication owner release after interrupted repair

Candidate `117c388385db3c2f6bc879c1b8bc4dc54192e471`, run 39893191950,
engine-authority job 120217937801, failed the installed tail-repair interruption
history at installation.rs:498: immediate resume reported another publication
owner after the preceding apply returned. Inspection found PublicationLease
relied solely on file close. A duplicated/inherited open-file description can
retain that lock past owner drop. This is a supported explanation for the
observed failure; the CI log alone does not identify an inheriting process.

The existing publication owner now explicitly unlocks on Drop, matching the
native manifest/inspection owner lifetime. It keeps the persistent inode and
nonblocking exclusive acquisition; there is no retry/sleep workaround. A
regression definition retains a duplicated descriptor, verifies denial during
ownership, drops the owner, reacquires immediately, and verifies disposal of
the old duplicate cannot release the new owner's lease. The existing installed
interruption history remains the integration oracle.

Before editing, PR 71's brief was extended with this exact owner and oracle.
Reads: restore_publication, tail_repair/publication, installation/lease,
installation interruption tests, native manifest/inspection Drop, anchored
filesystem flags and [Rust File locking semantics](https://doc.rust-lang.org/std/fs/struct.File.html#method.unlock).
`cargo clippy -p rrd-engine --all-targets --locked -- -D warnings` passed in
38.50 seconds. No local runtime test ran. The dependency PR 65 passed all
candidate checks at `beb55307a9c54df3eedec45a113084e360a4bdc1` and merged as
`6de0c5be56a6b7c888586186babb3b1648eb55ae`; this worktree merged that
unchanged-source main ancestry. Native capacity/repair acceptance still needs
the corrected candidate pipeline. C-07 remains open.


## Native image and receipt framing correction

The same candidate's Windows job 120217937576 executed the fresh WAL capacity
case in 156.84 seconds. The native write and filler both reported error 112;
the case's repair/reopen/prior-ACK ledger passed. The outer runner then failed
while decoding control output, so the suite did not pass. Rust's single-thread
libtest output placed the receipt immediately after its test-name prefix;
requiring a standalone `test ... ok` line was invalid with uncaptured output.
The exact selected test now uses quiet harness output, starts its receipt on a
new line, and requires the successful one-test summary plus the same exact
mode/point/build/ledger receipt. This changes framing, not a storage oracle.
Windows image control now emits a unique framed JSON receipt, verifies exactly
one frame and identifies attach/detach parse failures. A cleanup failure also
retains the preceding failure instead of concealing it.

The macOS job 120217937673 passed all three installed crash cases, then failed
before APFS execution with `hdiutil: create: -format requires -srcfolder or
-srcdevice`. Blank-image creation now selects `-type UDIF` instead of the
source-conversion `-format UDRW`; the new owned image's capacity, device and
filesystem checks remain mandatory. Apple?s archived [APFS tool example](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/ToolsandAPIs/ToolsandAPIs.html)
uses size/fs image creation; the actual native command error is the direct
current evidence. No claim is made that the corrected APFS case has executed.
PowerShell and Python syntax checks are authoring checks only. No local disk
image, runtime suite or exhaustion run was launched.


## Non-browsable APFS fixture admission

Candidate `151626c0ef950c47e0b2f4bdec9750d5bb321e80`, run 39895107779,
macOS job 120225089113 created/attached the capped APFS image successfully,
then failed before filling: Volume::admitted found zero sysinfo matches.
Locked sysinfo 0.39.6's Apple implementation skips volumes for which
kCFURLVolumeIsBrowsableKey is false; the fixture intentionally uses -nobrowse.
This is an admission mismatch, not native exhaustion evidence.

The macOS harness now queries statfs on the canonical fixture root, requires
that the returned mount root match exactly, verifies APFS and actual bounded
capacity, and independently checks the root's device differs from its parent's.
Image/device/nonce ownership and capped image creation are unchanged. Linux
and Windows retain their existing native disk enumeration. Added only a
macOS-specific test dependency on the already locked libc 0.2.189. No default
runtime dependency, mount visibility change or relaxed volume admission.

Read: Volume::admitted, native image controller and identity checks, locked
sysinfo Apple getfsstat/browsability filter and libc statfs declaration.
Primary references: [Apple statfs](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/statfs.2.html)
and [locked sysinfo source](https://docs.rs/sysinfo/0.39.6/src/sysinfo/unix/apple/disk.rs.html).
The owner brief was updated before implementation. Native macOS execution
remains the acceptance oracle, not Windows compilation of cfg-gated source.

The initial Windows compile found a stale shared-target rrd-contract artifact
from C-07g (a missing diagnostic admission initializer in unchanged F source).
After cleaning that package, `cargo clippy -p rrflow-cli --test installed_crash
--features crash-test --locked -- -D warnings` passed (20.13 seconds).
No local runtime test, mount or fill ran. Candidate CI and actual APFS/NTFS/ext4
capacity matrix remain required; C-07 status is unchanged.


## Native create-phase exhaustion

On the same candidate, Windows job 120225089228 passed the fresh WAL
exhaustion/recovery case (159.11 seconds). The next segment case reached its
rendezvous, but the observed error's role was WAL, not segment (58.06 seconds,
failed). The exact-role assertion correctly rejected this as segment proof.
Data-cluster exhaustion alone does not guarantee create-phase failure on NTFS,
where resident metadata may still accommodate small new files.

The owned filler now additionally consumes create_new directory entries at the
selected segment/manifest/CURRENT directory until creation actually returns a
native capacity error. The installed locator supplies that directory; canonical
containment and non-symlink checks bind it to the capped owned volume. Each
successful path is retained for exact cleanup after server termination, with
at most 8,192 files and the existing 25-second total fill deadline. Only those
owned paths and the data filler are removed. WAL still uses data-only fill.
Receipts retain metadata file count and the actual creation capacity code;
engine failure role and native error remain mandatory and unchanged. No native
error is substituted and a later-phase error still fails the selected case.

Read: diagnostic boundary/observation code, capped native filesystem wrapper,
installed locator, WAL warmup and native capacity cleanup. PR brief was updated
before implementation. `cargo clippy -p rrflow-cli --test installed_crash
--features crash-test --locked -- -D warnings` passed (15.64 seconds) after
clearing shared-target artifacts belonging to the G worktree. No local native
fill/runtime case executed. The next exact-candidate native matrix must prove
these paths; earlier WAL success does not validate the modified candidate.

## Offline timeout evidence and debug hashing cost

Continued from `ebb70266284bfb01c729229bf53d091b257a038a` in PR 71. Native
macOS estate job 120232488067 in run 39897715995 failed the unchanged
540-second isolated harness deadline. Retained offline-macos-latest artifact
reported command deadline but discarded every partial lifecycle observation.
This is a qualification failure, not evidence of a specific engine startup
error. The PR brief was extended before editing the acceptance helper.

The bounded subprocess helper now invokes an optional completion observer
before raising on timeout/output/exit failure. The offline payload retains
only allowlisted case names/status/counts, fresh/existing receipt marker
counts, readiness count and output sizes. It does not retain arbitrary
stdout/stderr, argument contents or receipt payloads. Native failure details
are carried alongside this partial receipt; full success still requires the
same complete lifecycle and independent history validators. Added negative
definitions for timeout/output observer delivery and secret/unknown-field
omission. The 540-second inner and 600-second outer deadlines are unchanged.

Inspected CLI build_identity.rs, installation executable SHA verification,
the large-debug-binary lifecycle fixture, workspace build profiles, and locked
sha2 0.10.9 src/sha256.rs dispatch. This feature set selects its software
compression on aarch64; development/test builds left that implementation
unoptimized while repeatedly hashing the actual executable. Added a dev
package optimization for sha2 only (opt-level 3). Development assertions and
overflow checks, the hashing algorithm, actual-file verification, all cases,
release settings and dependencies remain unchanged. This is a source-grounded
performance hypothesis pending the same native candidate, not a measured
speedup or a proven root cause. No digest cache or identity bypass was added.

`python -m py_compile scripts/ci/run_offline_lifecycle.py
scripts/ci/test_offline_lifecycle.py` passed. Cargo metadata --no-deps
--format-version 1 --locked passed after correcting the output path from the
worktree's absent target directory to the root target directory. No local
runtime tests were launched; native candidate execution remains required.
Reviewed the full changed helper/test/profile diff. No source move/deletion,
new dependency, relaxed capacity oracle or roadmap status change. C-07f,
C-07g, OBJ-02 and POAM-035 remain open.
Dependency verification correction before push: `cargo tree --locked -e
features -i sha2 --target aarch64-apple-darwin` resolves sha2 **0.11.0**, not
the 0.10.9 copy initially inspected in the local registry. The pinned
sha256.rs and aarch64_sha2.rs include runtime-selected hardware acceleration.
The previous software-only inference is withdrawn; no native backend absence
has been demonstrated. The dev optimization still targets the pinned actual
compression implementation without bypassing verification, but its effect on
the observed timeout remains unmeasured until candidate CI. Updated the PR
brief to the verified dependency facts. The command acquired already-locked
platform source dependencies; Cargo.lock was unchanged.

## Native capacity candidate receipts

Candidate `ebb70266284bfb01c729229bf53d091b257a038a`, PR 71, run
39897715995 passed all eight native capacity cases on each of Linux/ext4,
macOS/APFS and Windows/NTFS. Preserved all 24 full case receipts and three
suite receipts in [the candidate evidence](c-07f-native-capacity-ebb702.json),
including exact ordinary/diagnostic build identities, observed failures,
repair/readback results, expected-state digests and per-platform job links.
These were extracted from completed successful jobs, checked for exactly the
fresh/existing cross-product of four named boundaries, and checked against
passed suite receipts. The original logs remain remote CI records.

This is a native capacity result for that exact candidate, not full-candidate
acceptance: its macOS offline lifecycle deadline failed and later source
changes require new CI. No local disk-exhaustion workload ran, no 24-hour
campaign ran, and no capacity-profile or roadmap completion status changed.
The evidence export initially used Windows CRLF; the whitespace check exposed it. Normalized the JSON to LF without changing parsed receipts, then repeated the check.

## Native recovery-reserve admission

Continued from `5cc37849`; PR 71's disk-admission refinement was posted before
implementation. Added positive, sealed StorageLimits.recovery_reserve_bytes
(default 16 MiB, protocol ceiling 1 TiB), lowered into native DatabaseOptions
on create and reopen. AnchoredPath carries this policy through retained root
handles and directory rebinding. The native space helper queries POSIX
fstatvfs or Windows FileFsFullSizeInformation on those handles, without
resolving diagnostic names. Windows observes caller quota availability;
POSIX reports nonprivileged available blocks, which does not guarantee quota
admission. Unknown units, incomplete native replies and arithmetic overflow
fail explicitly.

WAL header/append/initial preallocation, encoded segment publication/import,
and manifest/CURRENT/checkpoint temporary publication check rounded proposed
bytes plus a conservative allocation-unit metadata allowance and the reserve.
Already-written immutable outputs remain charged by actual free-space
observation. Typed DiskAdmission is a pre-effect refusal, preserved through
store conversion and engine admission/absent-outcome classification; it does
not poison the WAL or manifest publication. Actual write/sync/rename errors
retain existing uncertainty and fencing. Real ENOSPC diagnostic barriers
remain after admission, so an external capacity change still exercises native
failure handling. This does not reserve physical blocks, guarantee metadata
or inode capacity, bound all estate disk usage, or admit external backup
archive/staging effects. Read-only open/inspection do not require free space
simply to observe existing acknowledged data.

Read/reviewed: new filesystem/space.rs in full; AnchoredPath root creation,
rebinding and effect methods; native handle validation; DatabaseOptions,
create/open, prepare-write, flush/compaction publication; WalWriter creation,
reopen, append and initial preallocation; segment encoding/import publication;
manifest publication/fencing; native/store error conversion; engine storage
lowering, transaction rejection/intent handling and service classification;
StorageLimits validation/defaults, installation template and TOML fixtures;
storage manifest validation; existing installation/storage projection examples,
SDK generators and reference identity checks. No upstream engine source was
adapted, no dependency or format lane was added, and no implementation moved.

Primary native API references reviewed 2026-10-23:
[fstatvfs](https://www.man7.org/linux/man-pages/man3/statvfs.3.html),
[NtQueryVolumeInformationFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntqueryvolumeinformationfile),
and [FILE_FS_FULL_SIZE_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddk/ns-ntddk-_file_fs_full_size_information).
They support handle-based observations and platform field semantics; they do
not establish crash durability or transfer third-party benchmark results.

Regression definitions cover allocation rounding/metadata/reserve overflow,
real handle observations without disk filling, WAL/flush/compaction refusal
with prior-ACK readback and later valid work, positive/bounded sealed policy,
engine lowering/error classification and declared-versus-installed reserve
mismatch. The existing projection generator now refreshes the public default
configuration fixture as well as installation fixtures. All five SDK and
signal/reference projections were regenerated; the final OpenAPI identity is
`64a5c9081928a5708a2f16b4ad08a285fe64c2473473edf13f1ea06fe8889377`.
No runtime suite or local disk effect was executed.

Surfaced authoring failures: an incorrectly quoted Python command did not
execute and made no source change; replaced it with structured patches and a
literal here-string. Strict Clippy rejected a field reassignment after Default;
used the existing initializer pattern. The worktree lacked TypeScript tools;
global pnpm 10 rejected the pnpm 12 lockfile. The declared pnpm 12.14.0 installed
44 locked packages from cache, with zero downloads, without changing the
lockfile; generation succeeded. Local Node is 24.21.0, below the package's
24.22.0 requirement, so no SDK runtime qualification is claimed. Generated
surface checks exposed stale authored digest citations; updated only current
contract/SDK references, preserving historical evidence. Final parity passed
for 42 operations and seven signal projections on this dependency branch.
Full candidate CI remains required; C-07f/g, Gate C, OBJ-02 and POAM-035 remain
open, including installed damage, reader interference, fleet/profile and
24-hour qualification. Reviewed changed source and projection diffs.
Final `cargo fmt --all` and strict Clippy across rrd-lsm, rrd-store,
rrd-engine, rrd-contract, rrflow-eval and rrflow-cli all targets passed (8.41s).
Default installation/public fixture and synthetic storage projection generation
passed; no engine workload was invoked by these generators. Whitespace and
generated-surface checks passed before staging.

### Candidate SDK identity projection correction

Starting revision: `9f0f3bf3b3e0bcd41411308a4086f268ce38be76`. Required candidate run `39902969502` exposed a stale shared corpus: optional-engine job `120248753326` and macOS estate job `120248753430` failed the existing `shared_sdk_corpus_identity_matches_bundled_installation` assertion; SDK job `120248753437` independently reported the same planned identity `304f5c8569d429363f7d65c1`. The new sealed default recovery reserve changes the deterministic installation identity. Read the complete shared corpus, its typed validation, the identity characterization and the installation projection example. Refreshed the six instance/estate/scope occurrences in `fixtures/rrd-sdk-conformance-v1.json`; all payloads, expectations and anchor bytes remain intact. PR brief was posted before this correction. No local runtime suite executed; the next candidate must repeat the unchanged identity and five-language conformance oracles. No gate/status change.


### Linux selected-create exhaustion correction, 2026-10-23

Candidate `1c4361be7ce788100a627a0e76a25286de43b38e` run 39903727016,
job 120251115161 passed seven native capacity cases; existing/current-before-create
failed waiting for the selected native error (60-second event deadline). Both
ordinary domain histories passed. No merge or eight-case acceptance is claimed.
The old data-first filler stopped on one metadata allocation error, which did
not establish that the selected different filename could not fit an ext4
indexed directory. This is a fixture inference, not a proven engine failure.

The pre-edit PR refinement binds the repair to the same native oracle. The
owned Linux image now requests 4,096 inodes and admits at most 8,192. For
create-boundary cases only, metadata is exhausted before data through the
already-open filler; fstatvfs must report zero free inodes. That observed
counter is emitted in the case receipt. WAL still exhausts data alone;
APFS/NTFS retain data-first metadata fill. Exact owned filler cleanup and
selected native ENOSPC/readback requirements remain. No timeout increased.
Source: [mke2fs inode-count option](https://man7.org/linux/man-pages/man8/mke2fs.8.html).
No upstream code imported. Read and reviewed full Filler/Drop/resume control,
owned Linux volume creation/admission and native log failure. Local runtime
or disk effects remain not run; candidate CI must prove the correction.

Authoring checks: crash-test installed_crash Clippy with warnings denied passed
(18.61s on Windows; Linux branch requires native CI). Navigation, inventory,
documentation policy and diff whitespace checks passed. Python controller
syntax compiled without executing volume operations. No native acceptance yet.

### Linux inode-oracle direct dependency correction

Candidate 5627c07e failed compilation in native Linux job 120265582476: the
new fstatvfs oracle referenced libc while its existing direct dev dependency
was macOS-only. Expanded that same target-scoped declaration to Linux, using
the existing locked version. No capacity assertion or failure deadline was
relaxed. The PR brief records this refinement before implementation. No
Linux runtime result from that candidate is claimed; new candidate CI is
required. Cargo metadata and static repository checks follow this correction.
