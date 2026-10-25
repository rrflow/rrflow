# D-01.5 native qualification candidate

**Status:** active implementation candidate; native candidate CI acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-qualification-candidate`
**Owner:** OBJ-07 prerequisite, D-01.5, POAM-006

[PR 52](https://github.com/rrflow/rrflow-development/pull/52) carries the pre-edit
brief. Main baseline is `7aa10e2312d0f5ed85beee9945e302b188fd4dc4`, tree
`eb523d8772eeb297aea9c7b7538b33002a77d5e6`. Implementation begins after the
[first failing oracle](d-01-native-qualification-oracle.md), revision
`4d727a6721a0ab1fea6c4c69c51b8d46553734ea`, tree
`252da2874b3e187ec1ddc3ffbf7190cf60a18515`. Its candidate
[run 39659420825](https://github.com/rrflow/rrflow-development/actions/runs/39659420825)
reproduced all three missing-case failures in verify. The still-running
superseded run was cancelled after that failure was inspected; it is not an
accepted candidate.

## Bounded change and replacement map

The existing five-case primary-process corpus remains the behavioral owner.
`run_offline_lifecycle.py` now rejects omitted, duplicate or unknown case
listings and requires exact successful result accounting. Two closed receipts
bind fresh/existing plans, backup, executable/build/public-contract/signal
hashes, observed durable cursors and bounded output. No raw estate or credential
is retained in an uploaded artifact. Receipt omission or unknown fields fail.

`rrflow-cli/tests/support/process.rs` replaces unbounded command/service capture
in `installed_lifecycle.rs`. Every process has a deadline and stream ceiling;
readiness lines and aggregate lifecycle output also have ceilings. Drop kills
and reaps only owned children. A short copy/spawn admission section covers the
writable executable descriptor and all child launches, including Unix SIGINT;
it does not serialize whole lifecycle tests. Existing command, exact-plan,
permission, authentication, Parquet, query, restart, backup/restore and content
preservation assertions remain. `lifecycle_process.rs` exercises real hanging,
flooding, finite, oversized-line and concurrent-staging processes. Its ignored
child entry is explicitly invoked by parents, not a skipped required outcome.

The native private-file modules gain test-only foreign-owner cases. Unix uses
retained handles with different numeric ownership. Windows temporarily enables
SeRestorePrivilege and changes only fixture ownership to LocalSystem while
retaining the current-user DACL. Repeated denial must preserve bytes and foreign
ownership. Restoration guards preserve original ownership and token state.
`run_private_owner.py` requires exactly one explicit privileged invocation;
unavailable privilege is failure. Production permission policy is unchanged.

Linux keeps mount/network/PID isolation and adds known-file absence, IPv6,
finite capture and exact receipt admission. macOS adds inherited sandbox-exec
source and network denial with direct and descendant probes. Windows uses the
new CI-only `windows_offline.py`: disposable standard-user principal, removed
traversal bypass, source-root deny ACLs, scoped dynamic WFP filters, suspended
launch into an owned job and bounded descendant cleanup. Private fixture
home/temp cleanup runs as that same principal before restoring source ACLs and
removing the account. No account, ACL or network policy was changed on the
local developer workstation. These runners are candidates until native CI
proves their actual enforcement; no mocked result substitutes for that proof.

The reusable workflow, its policy checker and CI operations owner require the
new cases and sanitized artifacts. Native installation filters now execute
bootstrap, ancestor, concurrency and recovery together. Gate D/POAM-006 link
this candidate without closing status. Generated navigation and file inventory
remain replaceable mechanical projections. No product version, dependency,
engine authority, transaction, provider, storage format or public API changes.

## Review and research

Full source review covers the primary lifecycle and new process helper/tests,
Unix and Windows private-file modules, existing LSM filesystem/containment
fixtures, offline runner and policy tests, new ownership/Windows helpers,
caller/reusable workflows and complete workflow policy checker. Owner review
covers README, Gate D acceptance matrix, POAM-006, CI operations and the
change-authoring/traceability routine. The complete changed diff is reviewed
before commit; existing implementation behavior is retained at its canonical
boundary rather than moved to another implementation lane.

Primary references include [Rust issue 114554](https://github.com/rust-lang/rust/issues/114554),
[WFP filter layout](https://learn.microsoft.com/en-us/windows/win32/api/fwpmtypes/ns-fwpmtypes-fwpm_filter0),
[restricted tokens](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-createrestrictedtoken),
[token-owned process creation](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createprocesswithtokenw),
and [native macOS profile usage](https://github.com/anthropics/sandbox-runtime/blob/main/src/sandbox/macos-sandbox-utils.ts).
Windows ABI definitions were checked against the locked windows-sys 0.61.2
bindings: local read-only layout checks observe filter 200, session 72,
condition 40 and extended job limit 144 bytes. That verifies layout sizes only,
not policy behavior. No upstream algorithm or runtime is copied. Concurrent
staging now has a real-process case, but the earlier CI ETXTBSY root cause is
not claimed proven by a passing retry or by this mitigation alone.

Diagnostics retain case names, bounded structural failures, byte counts and
hashes. Production tracing is unchanged. Resource bounds are 120-second normal
commands, 480-second service lifetime, 60-second/64-KiB readiness, 16 MiB per
stream and 128 MiB per lifecycle. The Python runner bounds capture and enforces
a 540-second corpus budget plus native outer cleanup bounds.

## Failures and local evidence

The initial 4-MiB stream budget rejected both long Windows lifecycle cases
(three other cases passed). A focused rerun identified authenticated `ready` as
the overflowing command. The measured legitimate catalogue is 5,598,305 bytes,
with empty stderr. After setting the explicit 16-MiB ceiling, all five cases
passed in 228.46 seconds with no ignored/filtered cases. This is a harness
budget correction, not a product failure or a PostgreSQL/graph/vector result.

The first oversized-announcement oracle incorrectly assumed libtest's first
line was nonempty; its leading newline defeated that fixture. A fixed literal
native cmd/sh output fixture now tests the intended boundary. The resulting
helper suite passes five parent tests in 2.02 seconds; its one ignored child
entry is invoked explicitly by those parent tests.

`python -X utf8 scripts/ci/test_offline_lifecycle.py` passes ten tests, including
real child deadline/output failures, missing/duplicate results and tampered or
unknown receipt fields. `python -X utf8 scripts/ci/check_workflow.py` passes
three workflows, seven substantive job families, five engine suites, twenty
default packages, six optional packages and nineteen container pins.

`cargo test -p rrd-engine --lib engine::token_key --locked` passes ten ordinary
Windows cases; the new privileged case is not locally executed. `whoami /priv`
confirms this developer token lacks SeRestorePrivilege. Native CI is required.
`cargo test -p rrd-engine --lib installation --locked -- --nocapture` passes
45 cases in 30.71 seconds; one bounded child fixture is explicitly parent-driven.
Strict clippy for both CLI integration targets and engine tests passes.
The read-only filesystem helper identifies the developer workspace as NTFS.

The final receipt-bearing `cargo test -p rrflow-cli --test installed_lifecycle
--locked -- --nocapture` passed all five cases in 229.59 seconds, with no ignored
or filtered cases. Both receipts bind executable SHA-256
`c61ac26495279b4f15a7356f3bd8affc326b9ea85ec999a5cec895934da4d60a`,
observed runtime cursors 2/4/3 and committed claim sequence 1. Existing/fresh
aggregate capture is 16,853,698/16,853,692 bytes respectively. These are local
unqualified development-build results, not signed-distribution provenance.

The first documentation check rejected a journal status without the required
active/historical classification. After correcting that classification, the
index was regenerated as required; no document gate was weakened. Final
mechanical checks passed: navigation rendered 17 indexes, 366 nodes and 1592
edges; execution inventory rendered 1321 records; documentation validated 367
statuses and 365 classified coordinates. Python runner tests, workflow policy,
workspace formatting, strict CLI lint and `git diff --check` passed. No native
candidate CI result is claimed by this local record. Cross-platform
native qualification, required privileged invocations, full candidate pipeline,
merge and D-01.6 reconciliation remain pending. No alpha or signed distribution
claim follows from this candidate.

## Auditable change checklist

Baseline/owners/diff and full source were reconciled; pre-edit PR brief exists;
first failure is retained; canonical behavioral replacement is mapped above;
research, trace, resources and diagnostics are explicit; local failures and
corrections are recorded; native limits are fail-closed; linked journal,
navigation and inventory accompany the candidate. Unsupported enforcement,
missing privileges/cases, escaped paths, unrelated process effects, secrets in
artifacts or a cross-owner production defect stop acceptance. D-01.5, parent
D-01, OBJ-07 and POAM-006 remain open; D-01.6 is not started.
