# D-01.5 required native lifecycle corpus oracle

**Status:** active failing characterization; native qualification implementation pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-qualification-oracle`
**Owner:** OBJ-07 prerequisite, D-01.5, POAM-006

Starting main revision `7aa10e2312d0f5ed85beee9945e302b188fd4dc4`, tree
`eb523d8772eeb297aea9c7b7538b33002a77d5e6`, after synchronizing the public
development remote. [PR 51](https://github.com/rrflow/rrflow-development/pull/51)
merged at this revision after candidate `9cfeeca40d95abed954e50df2a38e3e99861656d`
passed all seventeen jobs in [run 39654924753](https://github.com/rrflow/rrflow-development/actions/runs/39654924753).
That includes all six live SDKs, native Windows/macOS lifecycle tests and the
Linux ordinary and isolated five-case corpora. Linux isolated execution took
398.27 seconds and bound executable SHA-256
`652060742c0c5ca47c05c299ed8d15e285f3e8467494248ff9bbb0b23c93ad68`.
These results accept the bounded bootstrap implementation, not parent D-01.

## Pre-edit brief and exact characterization scope

The D-01.5 brief was prepared and a new branch published at the unchanged main
revision. GitHub refused the pre-edit draft PR with `No commits between main
and agent/d01-native-qualification`. This first failing test and linked record
therefore precede remote PR creation; runner/product implementation follows
the open draft PR and its brief. No planning-only commit or retired active-change
record is introduced.

Current `require_lifecycle_tests` requires only the fresh/existing pair, although
the actual primary harness has five required cases. The new independent set in
`scripts/ci/test_offline_lifecycle.py::test_every_primary_lifecycle_case_is_required`
removes each real case in turn and expects rejection. The runner incorrectly
accepts omission of all three other cases: primary help, rejected development
commands, and exact-plan/read-only verification shape. This is a demonstrated
qualification gap, not evidence that those existing product cases fail.

Only that test, this linked journal, Gate D/POAM-006 evidence and generated
navigation/inventory change in this first commit. The canonical runner and all
product implementations remain unchanged. The roadmap advances its active
subdivision from accepted D-01.4 to D-01.5 using the exact PR 51 evidence;
parent D-01, OBJ-07 and POAM-006 remain open.

The implementation brief binds the next changes to this runner and its native
helpers, CLI process qualification, native permission/filesystem tests and the
existing candidate CI lanes. It requires explicit platform enforcement probes,
complete executed case accounting, finite deadlines/output, exact artifact
identity and sanitized failure evidence. Unsupported enforcement, missing
privileges, required skipped cases, unrelated-process effects, secret disclosure
or a cross-owner product defect stop the package. No engine, provider, storage,
transaction or lifecycle authority changes are authorized by this test.

## Source and research review

Full implementation reads before the first oracle: `run_offline_lifecycle.py`,
`test_offline_lifecycle.py`, `run_filesystem_containment.py`, the primary CLI
`tests/installed_lifecycle.rs`, both engine native private-file implementations
and their tests, LSM `tests/filesystem.rs`, and the candidate caller/reusable
workflows. Relevant owner reads cover README, Gate D and its acceptance matrix,
POAM-006, CI operations, change authoring and the prior bootstrap journals.
The exact first-test diff and changed owner prose are reviewed before commit.

The corpus failure itself needs no outside research. Native implementation
research includes [Microsoft WFP dynamic sessions](https://learn.microsoft.com/en-us/windows/win32/api/fwpmu/nf-fwpmu-fwpmengineopen0),
[filter conditions](https://learn.microsoft.com/en-us/windows/win32/fwp/filtering-condition-identifiers-),
[Apple sandbox configuration](https://developer.apple.com/documentation/xcode/configuring-the-macos-app-sandbox),
and [Apple-distributed OpenSSH's sandbox entry](https://github.com/apple-oss-distributions/OpenSSH/blob/main/openssh/sandbox-darwin.c).
These references do not qualify an RRFlow runner. Actual native denial and
cleanup must pass on each claimed OS/filesystem profile.

[Rust issue 114554](https://github.com/rust-lang/rust/issues/114554) supplies a
specific mechanism to investigate for the earlier Linux executable-busy failure:
a concurrent fork can retain the writable descriptor of a staged executable
until exec. That is a characterization lead, not yet a proven cause or repair
of the earlier candidate failure. No upstream implementation is copied and no
host policy or account is changed during this preparation.

Diagnostics use the first failing unittest and candidate logs. Production
tracing is unchanged. Static inspection also identifies unbounded command
capture and service output in the current CLI harness; D-01.5 must supply
bounded replacement evidence. Existing native staging is not renamed offline
qualification.

## Commands and observed results

`python -X utf8 scripts/ci/test_offline_lifecycle.py
OfflineLifecyclePolicyTests.test_every_primary_lifecycle_case_is_required`
ran one test and failed three subcases with `RuntimeError not raised`, exactly
the three omissions above. The local log is
`target/d01-native-corpus-oracle.log`; it is diagnostic output, not release
evidence. The full Python suite is intentionally not claimed green before the
runner is repaired. No Rust implementation changes occur here, so Rust suites
are not repeated for this first-test commit. PR 51's accepted native results
belong to its exact candidate, not to unfinished D-01.5.

`python -X utf8 scripts/knowledge/render_navigation.py` passed with 17 indexes,
365 nodes and 1588 edges; `python -X utf8 scripts/ci/build_execution_inventory.py`
rendered 1316 records; `python -X utf8 scripts/ci/check_documentation.py` passed
366 statuses and 364 classified coordinates. `git diff --check` passed. The
complete changed source/owner diff is reviewed before commit; generated facts
come from those repository tools. The draft PR and candidate run
will bind this failing source; this commit cannot satisfy acceptance or merge.

## Auditable checklist

Owner/baseline and current diff inspected; source and first failure identified;
pre-edit remote PR attempted and its zero-ahead limitation recorded; bounded
test-only change authored; source-adaptation/trace/resource decisions explicit;
failure observed without weakening acceptance; linked evidence and POA&M entry
created; owner status reconciled only for accepted PR 51. Full native execution,
repair, package suites, final candidate CI and merge remain pending. No claim
of alpha, PostgreSQL compatibility, graph/vector scale, model reasoning or
cloud synchronization is made.
