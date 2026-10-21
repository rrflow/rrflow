# D-01.5 native coverage completion

**Status:** active implementation candidate; native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-coverage-completion`
**Owner:** OBJ-07 prerequisite, D-01.5, POAM-006

[PR 52](https://github.com/rrflow/rrflow-development/pull/52) was updated before
these edits. Starting revision is `4e6042da68bd360620cfd5f81afd906c3f46f4a2`,
tree `c85cf6ba1f3609701562b1cde3e85267cb879801`. This follows the immutable
[qualification candidate](d-01-native-qualification-candidate.md).

## Scope and traceability

The process-death recovery test covered seven of eleven typed boundaries.
It now includes EstateDirectoriesReady, StorageReady, TokenReady and
CredentialReady, retaining the existing credential-digest and recovery oracles.
It prints each passed stage. These are component process cases using the
existing small executable-identity fixture, distinct from the five-case actual
primary-binary corpus. Ten stages are durable effects; lease admission is the
eleventh boundary. Neither fixture is relabeled as the other.

The LSM filesystem test previously qualified macOS symlinks but had no mounted
volume adversary. Its new explicit native case opens an anchor before attaching
a test-owned sparse APFS image, proves a changed device identity, then requires
read/truncate/create/publish/remove denial and unchanged sentinel/pending bytes.
The existing filesystem runner now selects the native Linux or macOS case,
requires exact listing/result accounting, bounds capture/process lifetime, and
detaches only the image with its exact created path. Failed detach prevents
recursive fixture removal. Linux preserves its private mount namespace.
Windows junction behavior is unchanged. This is qualification infrastructure,
not a runtime dependency or a new filesystem implementation.

Full reads/review include README, Gate D, POAM-006, CI operations, change-authoring
routine, installation_concurrency.rs, installation/recovery.rs, filesystem.rs,
run_filesystem_containment.py, run_offline_lifecycle.py, check_workflow.py and
ci-reusable.yml, plus the complete installation_bootstrap.rs and fixture attributes. The complete changed diff is reviewed. Changed implementation
paths are the concurrency and filesystem tests, filesystem runner, workflow
and workflow checker, SDK anchor attributes and bootstrap hash assertion; supporting changes are this journal, CI operations,
POAM-006 and generated navigation/inventory. No paths are moved/deleted, and
no production behavior, API, storage format, dependency or authority changes.

Research uses Apple's [APFS disk image tools](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/ToolsandAPIs/ToolsandAPIs.html)
as a historical CLI reference; actual native CI must prove current behavior.
The attempted archived hdiutil manual URL returned 404 and is not evidence.
No upstream implementation is adapted. Diagnostics are stage names and bounded
fixture results; production tracing is unchanged. Image creation is bounded at
60 seconds, the macOS child group at 90 seconds, and detach at 30 seconds.
Unsupported enforcement, missing required cases, escaped fixture paths,
unrelated device effects or a production defect stop acceptance.

## Observed validation

At the starting candidate, `cargo test -p rrd-engine --all-targets --locked`
passed 207 tests with zero failures and two explicit child/privileged fixture
entries ignored by ordinary invocation. The library was 145 passed; integration
suites totaled 62. The privileged case is required separately by native CI.

After expansion, `cargo test -p rrd-engine --lib
process_death_releases_admission_without_regenerating_prepared_secrets --locked
-- --nocapture` passed one case in 1.37 seconds and reported all eleven stages.
The owning `cargo test -p rrd-engine --lib installation --locked -- --nocapture`
passed 45 cases in 25.89 seconds, zero failures, one parent-driven child entry
ignored and 101 unrelated cases filtered.

`cargo test -p rrd-lsm --test filesystem --test manifest --locked` passed six
Windows filesystem and nine manifest cases. The PowerShell stderr wrapper
initially reported a command status inconsistent with the all-passing summaries;
an explicit `exit $LASTEXITCODE` rerun confirmed exit zero. Strict filesystem
clippy and strict engine-test clippy passed. Ten offline-runner policy tests, workflow policy and Python
compilation passed. Native macOS volume execution is not runnable on this
Windows workstation and remains required in CI. Formatting passed.

The starting candidate run 39662303299 passed verify but failed the expanded
Windows installation suite: shared_sdk_corpus_identity_matches_bundled_installation
observed instance-d8c45f6ee961032cb51a01dd instead of the corpus's
instance-56c5f218cd3b36c14014abec (44 passed, one failed). Git's actual smudge
filter with core.autocrlf=true converted the inert anchor from 134 bytes,
SHA-256 d42417daff2c8032efa0c090f1f7d6fb18525f3c2813aa8e1730ff935dd01ee7,
to 136 bytes, SHA-256 c040d717c0a777f087097ce8821e57edd2838f69beff67fbac1acdeb9984ebc7.
After a pre-edit PR brief amendment, fixtures/.gitattributes pins only that
anchor as binary; installation_bootstrap.rs asserts its canonical hash before
planning. Both autocrlf=true and false now produce the same 134 canonical bytes.
The focused shared-corpus test passes one case in 0.11 seconds; the owning
installation_bootstrap filter passes all six cases in 0.26 seconds. Production
identity hashing and corpus identities are unchanged. This is not a passing
native CI record. No D-01 row, alpha
outcome or POA&M status is closed. Final candidate CI and D-01.6 reconciliation
remain required.

## Change checklist

Owners/baseline/diff reconciled; pre-edit brief bound; missing cases identified;
canonical test replacement mapped; research and resource decisions explicit;
focused and owning tests executed; documentation policy passed 368 statuses
and 366 coordinates; navigation passed 17 indexes/367 nodes/1595 edges;
execution inventory passed 1323 records; native checks still pending are named;
full-file/diff review performed; linked journal and mechanical projections
accompany the candidate. No acceptance claim follows from compilation or local
results alone.
