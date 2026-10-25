# C-07g: preserve failed startup state and identify file admission

**Status:** active implementation evidence; native root-cause repair pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-reopen-admission`
**Owner:** OBJ-02 / C-07c,g / POAM-035 / PR121

## Brief and baseline

[PR121](https://github.com/rrflow/rrflow-development/pull/121) was opened from
public main `94d0567fb865fb49a90aaeeef4b5861707c9f8a7` with opening commit
`d9813b23a3d1e1bbdad4157739b4b1d76326017b` before implementation. An ordinary
merge incorporated exact integrated source `037635afbf202fe8379ebec647f51bd57b410a79`;
its retained failed-growth evidence was carried as `c69348bc`. Prerequisite
PR117/119/120 acceptance remains separate. This package cannot land ahead of
its prerequisite candidates.

The [integrated failure record](c-07g-integrated-native-growth.md) is the first
native oracle: after all 10k writes and independent readbacks, restart exits
with `multiply linked file denied`, then TempDir unwinding removes the failed
estate. A link-count check of `!= 1` also rejects zero links, so the old message
does not establish a hard-link attack or publication bug. Native publication
already uses exclusive rename APIs; no speculative publication rewrite follows.

Target: retain original owned startup-failure state after child cleanup, and
identify denied file admission with bounded relative context and actual link
count. Preserve the exact admission rule and all existing caps/deadlines.
Stop before changing another authority or accepting aliased/untrusted files.
The actual startup cause remains unproved until exact-source reproduction.

## Implementation and traceability

`filesystem/native.rs::validate_file` reports expected and observed link counts
while continuing to require exactly one. `AnchoredPath::validate_file` adds a
relative label limited to 256 characters, escaped for diagnostic output, only
to repository-created PermissionDenied errors. Native OS error codes remain
intact. Both ordinary options and callback-based private opens use this owner.
Mount, nonregular-object, symlink and pre-truncation validation remain intact;
retained-handle reopen retains its original strict validation.

`tests/support/owned_directory.rs` wraps the existing uniquely owned TempDir.
Normal completion drops it normally. Explicit retention changes only cleanup
ownership; it neither copies files nor chooses a new path. The installed
startup failure branch terminates the child, collects/reaps its bounded output,
then retains that original estate and prints a local private-state location.
It does not print credentials or automatically upload those bytes. Retention
uses the existing admitted workload allocation; it does not grow another copy.
Unrelated state and successful-run cleanup remain covered.

Changed: native and anchored admission diagnostics, filesystem regression tests,
installed startup capture. Created: private test-directory owner and its tests,
this evidence record. Navigation/inventory are generated. No format, source
selection, engine authority, deadline, qualification profile or security-policy
change is made. No runtime dependency, provider path or fallback was added.

## Reads, research and diagnostics

Read complete changed native admission and anchored open/callback/reopen
functions, TempDir/startup capture and Process finish/drop paths, complete new
directory owner/tests, the native failure log and progress, snapshot-file
reopen callers, private-file enforcement and installation-lease ownership.
The lease implementation intentionally retains its file permanently. The
complete changed diff was reviewed. Earlier source reads are recorded in the
linked integrated-growth record.

This uses existing Rust/native file metadata and TempDir ownership, with no
external algorithm adaptation. No new research claim is made. The previously
reviewed PSI source supports only the diagnostic stall interpretation. Bounded
file context and private original-state retention are necessary because the
first failed estate no longer exists; neither is itself a startup repair.

## Verification and limitations

Windows commands use the shared target and dev/test debug symbols disabled.
Logs and exact command/duration receipts are under operator
`target/c07-pr121-evidence/`.

- Windows `cargo test -p rrd-lsm --test filesystem --locked`: six passed.
- Linux same target on the authored source: seven passed, one explicitly
  privileged bind-mount helper ignored. The unlinked-handle fixture reports
  zero links and still denies admission; hard-linked files report two and
  preserve outside bytes. This is component evidence, not capacity.
- `cargo test -p rrflow-cli --test installed_reopen --locked`: twelve passed,
  seven explicit native helpers ignored. Directory tests prove ordinary cleanup
  and retention of one original tree through panic unwinding.
- Separate installed capacity smoke passed in 8.04 seconds: actual records,
  relationships, vectors, backup verification, restart and exact replay.
- Full LSM suite: 215 passed, two intentional child helpers ignored. Strict
  affected Clippy passed. Formatting, documentation policy, generated
  navigation/inventory and diff checks passed before candidate publication. Linux crash/mount privilege and the
  full native candidate matrix remain candidate CI requirements.

A separate prerequisite CI failure is now retained at run `40124044612`, job
`120962408662`: macOS native capacity metadata fill reached its 8192-file bound
without an OS capacity error. This is a C-07f fixture deficiency, not a passing
native exhaustion case. PR117 cannot merge with that failed check. Its repair
belongs to its bounded native-capacity package, not this admission change.

Checklist: owning outcome/brief before code; native first-failure evidence;
canonical admission and test ownership extended directly; bounded diagnostic
and unchanged safety decisions; focused then owning checks; full changed-file
reread/diff review; missing checks and unrelated failure recorded. No roadmap,
POAM-035 or alpha completion change. Native startup reproduction, canonical
root-cause repair, successful profiles/repetitions and actual 24-hour soak
remain required.
