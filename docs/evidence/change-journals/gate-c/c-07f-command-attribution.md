# C-07f: identify installed capacity recovery command failures

**Status:** active implementation evidence; native timeout causes remain open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-command-attribution`
**Owner:** OBJ-02 / C-07c,f / POAM-035 / PR124

## Baseline and bounded change brief

[PR124](https://github.com/rrflow/rrflow-development/pull/124) contains the brief
before implementation. Public-main baseline is
`94d0567fb865fb49a90aaeeef4b5861707c9f8a7`; empty opening commit is
`42015bab5ff3c4b6c0f4471f1cc88f7b57040a33`. No version, release, transaction,
storage format or ordinary-runtime behavior changes. This package does not
supersede PR123's integrated growth implementation or rebind its running VM
campaign. The ordinary workload can continue independently.

The waiting operator service for the three-seed, multi-day qualification was
stopped before launch, with `operator-hold.json` retained in the owned VM. The
full Desktop preflight continues on its unchanged source as characterization.
The next actual soak must bind the final integrated candidate; no prior or
currently running actual 24-hour soak is claimed.

Current behavior loses the CLI command identity when the child monitor reports
a deadline. Target behavior preserves static command phase, elapsed time and
the original deadline on start and terminal outcomes. The only source changes
are CLI `tests/installed_crash.rs` and `tests/support/capacity.rs`. The common
`installed_command` wrapper uses the existing `Process::start/finish`, preserving
120-second limits, capture sizes, process cleanup and nonzero-exit handling.
Only a closed enum enters phase records; argument vectors, paths, digests,
credentials and captured stdout are excluded. Repair-plan success/denial stays
separate from explicitly authorized repair application.

Traceability: `Estate::invoke` keeps all existing callers and assertions;
`capacity_case` keeps the same repair plan/apply, independent history/readback
and retry-receipt proof. The common wrapper replaces only duplicated process
invocation. No runtime or recovery policy moves. Stop on widened deadlines,
weakened native error selection, retries, unbounded capture or a runtime repair
without attribution.

## Verified original failures

Read original GitHub job logs and retained their bytes under operator
`target/c07-timeout-investigation/` before editing:

The macOS log SHA-256 is
`24f53633852f62ab3bcbcbc0d35225d10fe676e00a440a7b4027a71a27473dec`;
the Ubuntu log SHA-256 is
`85bf1e85de3bfef9a57c664c3e5d8ad30b6e61e1bb9019c0ab290c38010ff612`.
The capacity source blob in both failed branch heads (`530694c7`, `38e7cc64`)
and baseline main is `97d3be89c91063f5048d6ae34a44bda7b49b7cf9`, confirming
the call-site trace against the source used by both reports.

- [PR118, run 40121463373, macOS job 120953725578](https://github.com/rrflow/rrflow-development/actions/runs/40121463373/job/120953725578):
  the fresh WAL case reached `wal_before_write`, reporting payload 2660375
  bytes. Filler wrote 254607360 bytes and obtained native code 28. The client
  returned `api-ResourceExhausted`; no selected native-I/O receipt arrived in
  60 seconds. Controller teardown killed the process. This does not establish
  whether the selected write succeeded, a later admission failed, or observation
  was missing. In particular, the selected pause was reached. PR122's metadata
  refill is conditional on a metadata directory; WAL passes `None`, so that
  repair is not evidence that this failure is fixed.
- [PR23, run 40121518000](https://github.com/rrflow/rrflow-development/actions/runs/40121518000):
  the fresh ext4 WAL case filled 242651136 bytes, obtained native code 28 and
  returned `api-Internal`. The subsequent `Estate::invoke` exceeded 120 seconds.
  The stack names `capacity_case`'s async closure; its sole `Estate::invoke`
  call applies the repair plan. Thus command attribution is `repair apply`,
  while the expensive/stalled subphase and cause are unknown. Hashing, copy,
  replay and publication remain possibilities, not measured diagnoses.

Neither run reports a completed successful recovery oracle. Lack of a reported
readback mismatch is not evidence of correctness. API error codes are not
syscall receipts and are not changed by this package. Expected panics in the
filler's passing negative tests were distinguished from actual failed tests.

## Inspection and research decisions

Owners read: README, OBJ-02, C-07, POAM-035, CI operations, change-authoring and
implementation-navigation procedures. Source inspected: installed crash and
capacity harnesses; bounded process capture; native capacity launcher; native
diagnostic control/error observation; filesystem I/O wrappers; WAL append;
installed tail-repair binding and publication/copy. These original logs and
local call paths are sufficient for this diagnostic change. No external
algorithm, dependency or source adaptation is used.

Trace decision: add phase/timing at the existing test invocation boundary;
retain error and output limits. Resource decision: no additional runtime
sampler, policy or host allocation. The focused regression launches the actual
test executable as an explicitly ignored, bounded sleeping helper, then proves
the existing monitor still fails and reports `RepairApply` without stdout or
argument exposure. This is failure attribution, not storage-full qualification.

## Validation and remaining work

Windows validation used the existing explicit target directory with
`CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`:

- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked command_timeout_identifies -- --nocapture`:
  one passed in 2.02 seconds (11.015 seconds including rebuild). The child
  emitted and flushed its private marker before the ready-file handshake;
  the monitor failed it at 2015 ms under the test's 2000-ms deadline. The
  failure named `RepairApply` and omitted stdout/arguments. Production fixture
  command deadlines remain 120 seconds.
- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked`:
  11 passed, zero failed, two explicit helpers ignored; test time 215.47 seconds
  (216.110 seconds including Cargo). Both fresh/existing domain and
  publication/reclamation histories passed. The ignored entries are the
  separately invoked real owned-volume capacity corpus and sleeping helper.

The initial documentation check rejected a missing `active` status word;
the next rejected its stale generated index. Both were corrected at the
owning record and generator. A patch-context mismatch applied no changes and
was corrected after rereading the affected text. No test failure or deadline
was waived. Initial exploration also encountered invalid local path globs and
a PowerShell inline-Python quoting failure; subsequent commands used exact
paths and file-backed scripts, with no repository mutation from those errors.

Native reproduction and causal repair of both intermittent failures remain required; actual
ext4/APFS/NTFS outcomes and full candidate CI cannot be inferred from the local
regression. POAM-035 gains the two exact failure instances; C-07 remains open.

Changed records: this linked journal, POAM-035, the C-07 owner link and generated
navigation/file inventory. Existing immutable journals are unchanged. The PR
brief and this record carry scope, traceability, research/debug decisions,
failure oracle, remaining errors and stop conditions. No alpha status changes.


Final local checks: strict `cargo clippy -p rrflow-cli --test installed_crash
--features crash-test --locked -- -D warnings` passed in 34.485 seconds.
Documentation, generated navigation, generated execution inventory, formatting
and `git diff --check` passed. The first inventory check detected drift because
the journal changed during generation; the final sequence regenerated after
the evidence update and passed. Exact command/result logs are retained under
operator `target/c07-timeout-investigation/validation/`.

Review and checklist: reviewed the complete source diff, changed diagnostic
functions/call sites, surrounding crash/recovery oracles and the changed owner
records. Scope, static-label privacy, original deadlines, no runtime changes,
first failing oracle, owning suite, research/trace decision, retained failures,
generated indexes, remaining errors and stop conditions are recorded here and
in the PR brief. New native volume runs were not executed locally: the local
suite's owned-volume case is explicitly ignored, with native qualification
delegated to the required candidate CI. No APFS capability exists on this
Windows host. Candidate CI and merge are pending; no acceptance is inferred.
