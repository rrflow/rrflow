# RRFlow CI and runner operations

**Status:** active operational contract; current checks are not RRFlow alpha or release evidence
**Coordinate:** `rrflow://rrflow-instance/data/operations/ci`
**Owner:** candidate CI topology, workflow supply-chain policy, diagnostic workflow boundary, and private runner procedure; indexed by `docs/operations/README.md`

This record describes checked-in automation and the operator steps around it.
The [release roadmap](../roadmap/rrflow-1.0.md) alone owns gate status, the
[repository run checklist](../roadmap/rrflow-1.0-execution-map.md#repository-wide-run-checklist)
defines risk-based package widening, and the [POA&M](../poam/rrflow-1.0-alpha.md)
owns deficiencies. A green workflow proves only the commands and assertions it
actually executes.

## C-07 installed campaign progress

The existing `scripts/ci/run_sustained_capacity.py` launcher accepts the opt-in
`--observe-progress`, `--progress-interval-ms` (1,000–60,000; default 15,000)
and `--progress-max-bytes` (4,096–16,777,216; default 1,048,576) flags. It
exclusively creates `<evidence>.observations.jsonl` alongside the campaign
directory and emits the same bounded, cumulative event-kind counts to stderr.
No event payload, record ID, prompt or credential is copied. A source-available
flag distinguishes a not-yet-created stream from zero observed events. Linux
observations include available owned-cgroup counters; Windows reports these as
unavailable. The observer stops on malformed, replaced or truncated input or
when its output limit is reached and reports an error to stderr; the campaign
continues. It starts after the Linux cgroup is created or, on Windows, after
the harness image is pinned and before the owned job is launched. Observations
are best-effort, buffered diagnostics, not durable audit, proof of completed
records, the campaign verifier, or a qualification result. Observer startup
requires a fresh sidecar path. Its thread is joined on exit; a blocked read is
reported after a bounded stop wait and may finish after native cleanup. The
observer runs in the launcher outside the owned native job/cgroup, and its
reads and sidecar writes add overhead; compare performance only with the same
observation settings and report them.

## C-07 storage-diagnostic attribution

To characterize a slow installed `rrflow` storage-diagnostic request, start the
primary executable with `RRFLOW_TRACE=rrd_server::diagnostic=debug,rrd_store::diagnostic=debug`
and `RRFLOW_TRACE_FORMAT=json`; its existing tracing implementation emits JSON
to stderr. Capture stderr under an explicit size and retention budget; an
unconsumed pipe or slow sink can itself affect execution. Trace activation
does not change the resource-sampling requirements or authorize collection of
other data. Use the same trace settings for the actual process under test, not
just the harness process.

The server event reports `service_elapsed_ns` around the engine service call
and a success boolean. For a successful rrflowKV physical snapshot, a store
event reports `mutex_wait_ns`, `mutex_held_assembly_ns`,
`memtable_profile_ns`, key/version counts and segment count. The lock-wait
interval includes scheduling and the lock call; the held-assembly interval ends
immediately before dropping the guard, and includes profile time. Emission
happens after unlock. Service time also includes authorization and validation;
it does not include network, HTTP scheduling before the call, or the full
response. These observations do not independently isolate scheduler, I/O,
authorization or underlying causes. An error before the physical snapshot
has no store event. Clock resolution, trace overhead, filtering, loss and
preemption affect interpretation.

The trace span contains SHA-256 of the client request ID rather than its raw
value. The digest can still be sensitive or guessable; treat traces as scoped
diagnostic data. The C-07 sampler uses a checked process-local request counter
and prints the matching digest for completed slow samples within its existing
64-record bound; include the harness process and campaign identity when joining
across restarts. Failed or canceled calls need not produce a completed-sample
record, and records beyond that bound are not retained. Other callers need
their own unique request IDs and matching digest to join records. Missing or
truncated events are missing evidence, not zero cost. Exact-field or
source-location subscriber filters can affect the `tracing` enabled check; use
the stated target filter and validate capture on the installed candidate.

The [C-07 diagnostic journal](../evidence/change-journals/gate-c/c-07g-diagnostic-lock-attribution.md)
records the bounded controlled test and its limits. The original failed run
used an earlier binary and cannot retroactively acquire these events.
The [integrated trace journal](../evidence/change-journals/gate-c/c-07g-installed-trace-correlation.md)
records the small installed mixed-family and restart proof; it is not capacity
or soak qualification.

For transaction-begin attribution, add
`rrd_engine::transaction_diagnostic=debug` to the process's `RRFLOW_TRACE`
filter. Each enabled event reports the SHA-256 of the request ID, the outcome
(`created`, `replayed`, or `error`), the last phase reached, total time and
elapsed nanoseconds for validation, admission, authentication and policy,
replay lookup, snapshot preparation, and control-state publication. A zero
phase may mean it was not reached, not that it was free. Snapshot preparation
also includes lease checks and transaction-record construction. Time in the
client and HTTP transport is outside these measurements. Emission occurs after
the admission permit is released; tracing can nevertheless affect the caller's
latency. Use a bounded sink and treat the digest as potentially guessable. These
optional events are not durable audit records, may be filtered or lost, and
cannot establish a percentile or a capacity qualification by themselves.

## Candidate chain

RRFlow has one candidate CI chain. The thin caller is
`.github/workflows/ci.yml`; `.github/workflows/ci-reusable.yml` owns every
check. A branch push does not start a second copy of the same work.

The caller accepts ready pull requests, merge-queue candidates, and explicit
manual dispatches. Draft PRs remain reviewable packages without automatically
launching the full pipeline. `opened`, `synchronize`, `reopened`,
`ready_for_review`, and `converted_to_draft` are explicit PR activities.
Becoming ready starts candidate CI even without a new commit. Draft events use
a separate concurrency group from ready candidates, so a delayed draft event
cannot cancel a ready run. Returning to draft skips the reusable caller but does
not cancel a ready run already in progress; a subsequent ready candidate for
the same PR supersedes that run. Other PRs and non-PR refs remain isolated.
A skipped draft run is not candidate acceptance. GitHub does not guarantee
concurrency start order, and a local grouping test cannot prove live delivery
order. The required gate fails unless every partition the change selected
succeeded and every partition it did not select was skipped by its lane
condition. Manual dispatch remains available for an explicitly requested full
run on a draft branch.

This uses GitHub's documented [pull-request activity types](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request);
the default activity list does not include readiness transitions. The local
`test_candidate_admission.py` corpus verifies the checked-in admission shape
and rejects missing transitions, filters and lost merge/manual coverage; it
does not emulate GitHub execution. Merge-group candidates select lanes from
their own diff against the queue base, exactly as pull requests do. The caller
has no path filter; lane selection lives inside the reusable workflow, where
`ci-gate` can see it.

## Development gate and qualification matrix

Owner decision, 2026-11-14: during pre-release, candidates run a development
gate, and release qualification runs apart from it ("dev build, normal build
later"). This replaces the 2026-11-09 proportional-lanes decision, under which
most engine changes still selected the full matrix.

**One branch.** Owner's rule, 2026-11-14 (`AGENTS.md`): pre-release work is
committed and pushed directly to `main`. The `ci` workflow therefore also runs
on every push to `main`, selecting lanes from that push's own changes. The
ruleset on `main` forbids only deletion and force pushes; the pull-request
requirement, the required check and the merge queue were removed that day. A
failed push run opens `Main is red: ci`.

**Development gate (every push and every candidate).** The first job, `changes`, diffs the
candidate against its base and runs `scripts/ci/select_ci_lanes.py`, which maps
every changed path to light, Linux-only lanes. No changed path selects the full
matrix.

| Lane | Selected by | Partitions |
|---|---|---|
| `docs` | `docs/**`, Markdown anywhere, issue and PR templates, `LICENSE` | `docs-policy`: documentation authority and local links, generated navigation, execution inventory, capacity projection rendering, whitespace |
| `scripts` | `scripts/**`, `eval/**`, `.github/workflows/**`, `.devcontainer/**`, `deploy/**`, `.gitmodules`, `.gitignore`, `.gitattributes`, `VERSION`, the Connectome gitlink | `scripts-policy`: the repository's Python test files that need no build, plus the CI policy tests and `check_workflow.py` |
| `engine` | `crates/**`; also `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` and `.cargo/**` | `engine-fast` on Linux: formatting, generated-surface parity, `cargo test` and clippy for the changed crates, `cargo check --all-targets` for every workspace dependent. A workspace input selects no single crate, so the whole workspace is compiled |
| `sdk` | `sdks/**`, `fixtures/**`, `crates/transport/**` | `sdk-conformance`; transport crate edits also run `engine-fast` |

A path the classifier does not recognize runs every light lane, compiles the
whole workspace, and is named in the job log.

Compiling a dependent cannot see a golden file in it going stale. A crate
whose tests pin another crate's public identity is therefore tested, not only
compiled, when that crate changes. `PINNED_BY` in
`scripts/ci/select_ci_lanes.py` owns the list: today a change to
`rrd-contract` also tests `rrflow-qualification-storage`, whose fixture
records the public catalogue's digest. The cause and the fix are in the
[journal](../evidence/change-journals/repository/ci-pinned-identity-20261114.md).
Any other behaviour of a dependent that a change breaks is found by the
nightly full matrix, not by the gate.

A test that runs another package's binary needs it built first, because
`cargo test -p <crate>` builds only that crate's own binaries. `TEST_BINARIES`
in the same script names them, and `engine-fast` builds them before testing:
today `rrd-server`'s tests run `rrflow-cli`'s `rrd-engram-controller`. A test
in `scripts/ci/test_select_ci_lanes.py` fails when a crate calls
`workspace_binary("…")` without being declared
([journal](../evidence/change-journals/repository/ci-fast-lane-test-binaries-20261114.md)).

**Qualification matrix (`full`).** The full matrix covers the native installed
lifecycle on Linux, macOS, Windows and Linux ARM64, crash injection, the
100-cycle reopen, native capacity, the partitioned engine suites, optional
features and SDK conformance. It runs:

- nightly against `main` (`.github/workflows/ci-nightly.yml`, `scope: full`);
- on a manual dispatch of `ci` or the nightly workflow; and
- before a release decision, as Gate J requires.

A failed nightly or manual qualification run of `main` opens the
`Main is red: <workflow>` alarm issue, and a later passing run closes it
(`merge-alarm.yml` watches both workflows). A red qualification run is repaired
before new work, like a blocked merge, but it does not hold a candidate that
passed the development gate.

What the development gate does not prove is exactly that matrix: behavior on
macOS, Windows and ARM64, crash recovery, and capacity. Evidence for a gate
that needs those comes from a qualification run, cited by its run id.

Alongside whichever light lanes a change selects, `repository-invariants` runs
the engine's repository-wide architecture tests
(`cargo test -p rrd-engine --test workspace_architecture`) on every candidate
the full matrix does not cover. Those tests scan every tracked file for the
retired identity, trailing whitespace, untracked target sources and physical
imports, so a documentation or data change can break them: #308 landed
evaluation inputs through the docs and scripts lanes on 2026-11-09 and left
main failing two of them for every engine-lane candidate. The job costs one
cached `rrd-engine` test build; the full matrix already runs the same tests
inside `engine-suites`, so it skips there.

A `full` run absorbs the light lanes, so their jobs skip and the full
partitions carry their checks.

`scripts/ci/check_workflow.py` requires every partition to declare exactly its
lane condition and the gate to check each result by lane;
`test_select_ci_lanes.py` and `test_ci_lanes_policy.py` are its regressions and
run in the `changes` job and the verify policy step.

## Pull request limits

After lane selection, the `changes` job runs `scripts/ci/check_pr_governance.py`
and fails a pull request that breaks a work-in-progress limit; size alone only warns. The limits
exist because on 2026-11-09 the repository held 27 open pull requests, 20 of
them drafts, most 130 to 420 commits behind `main`, with two merged into
`main` carrying no file changes; prose in `AGENTS.md` had not prevented any
of it.

| Limit | Value | Lifted by the `integration` label | Evidence |
|---|---|---|---|
| changed files | at least one | never | #298 and #299 merged with zero changes |
| base branch | `main` | never | eight stacked lab pull requests could not enter the queue in turn |
| merge-base age | 3 days | never | DORA's trunk-based capability: merge to trunk at least daily, three or fewer active branches |
| hand-written changed lines | 400, advisory only | not needed | The cited studies (Sadowski et al., ICSE-SEIP 2018; Cisco/SmartBear, 2,500 reviews) measure human line-by-line review. The owner lands complete packages as one pull request, generated projections are not reviewed line by line (GitHub collapses `linguist-generated` files), and size labels in practice (Kubernetes `size/*`) inform rather than block. Above 400 hand-written lines the job prints a warning that points the reviewer to the change brief and traceability map; generated SDK, OpenAPI, signal, schema-golden and lockfile lines are excluded from the count. Changed 2026-11-13 at the owner's direction |
| open pull requests | 5 | yes | teams running parallel agents size concurrency by pull requests awaiting review; conflicts grow faster than linearly with parallel agents |
| open per author | 3 | yes | open-source projects admitting agents cap contributors at about three open pull requests |

Pull requests opened before 2026-11-10 are the backlog being absorbed: the
empty-diff, base-branch and merge-base rules apply to them, the two counts
and the size limit do not, and they are not counted against newer ones.
`scripts/ci/test_pr_governance.py` is the regression corpus. The evidence is
correlational and partly vendor-reported; the numbers are starting values
to be changed from measurement, in this record, not from taste.

Sources: [DORA, trunk-based development](https://dora.dev/capabilities/trunk-based-development/);
[Sadowski et al., Modern Code Review: A Case Study at Google, ICSE-SEIP 2018](https://sback.it/publications/icse2018seip.pdf);
[SmartBear/Cisco peer review study](https://smartbear.com/resources/white-papers/11-best-practices-for-peer-code-review/);
[Osmani, Your AI coding agents need a manager](https://addyosmani.com/blog/coding-agents-manager/);
[CircleCI, parallel coding agents without piling up PRs](https://circleci.com/blog/parallel-coding-agents/);
[Wessel et al., AI policies in popular open source projects](https://arxiv.org/html/2609.07542);
[AgenticFlict, merge conflicts in AI-generated contributions](https://arxiv.org/pdf/2601.20109).

`scripts/ci/check_session_end.py` is the agent-side guard: it fails while the
worktree holds uncommitted or unpushed work, so a session cannot end with
hundreds of staged files that no pull request carries, as five did on
2026-11-09.

Rust cache steps still restore compatible entries on every candidate, but do
not save new entries from `merge_group` runs. Queue-ref entries are scoped to
the temporary ref and cannot warm the next candidate's base; disabling those
saves reduces one source of eviction pressure. Pull-request saves and the
explicit main-only macOS and Windows warmers remain available. Other language
caches are unchanged.
This does not create a main cache or guarantee a hit, speedup, or reduced bill.
GitHub documents [cache scope and eviction](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching);
the pinned [rust-cache action](https://github.com/Swatinem/rust-cache/blob/6323deb102c322ba6fcbdcafc7e3dddab59af2b6/action.yml)
documents that `save-if` controls saving, not restore. Inspect later run logs
and cache usage rather than inferring impact from the workflow text.

The Windows warmer is `.github/workflows/ci-windows-cache-warm.yml`; after
that file reaches public `main`, an operator can dispatch the main revision:

```sh
gh workflow run ci-windows-cache-warm.yml --repo rrflow/rrflow-development --ref main
```

Inspect the resulting run's exact head and cache-save log, then check the next
candidate's restore log and timing. The manual job uses the candidate's fixed
Windows runner, toolchain, two Cargo workers, cache key and locked release
build; its separate 120-minute ceiling does not extend the 90-minute
candidate job or run the tests. It never qualifies the candidate. Cache
scopes, eviction, dependency-only caching and runner variability can still
produce a miss or leave workspace compilation slow. Standard hosted runners
for this public repository are documented as free by
[GitHub](https://docs.github.com/en/actions/reference/runners/github-hosted-runners);
cache storage beyond the included allowance may incur charges depending on
account settings. Do not add untrusted public jobs to a private self-hosted
runner to populate this cache.

The reusable workflow reduces every partition into one stable `ci-gate`
check for branch protection:

- installed primary-executable lifecycle on the standard Linux runner;
- repository policy, formatting, architecture, evaluation, dependency, and
  binary-budget gates on a heavy Linux runner;
- five default-feature engine suites on isolated heavy Linux runners:
  `kernel-storage`, `query-retrieval`, `engine-authority`, `service-protocol`,
  and `product-operations`;
- one current shared SDK harness, isolated optional-feature checks, and a real
  PostgreSQL/pgvector integration;
- a synthetic storage-contract preflight on the selected standard Linux runner;
  the ordinary 100-cycle reopen matrix starts only after this candidate's
  catalogue and synthetic fixture projections pass. The job first verifies that
  the named test exists, so an empty test selection cannot pass;
- installed lifecycle and daemon portability on GitHub-hosted Windows and
  macOS runners;
- separately identified installed domain-crash diagnostics and ordinary-build
  exclusion on GitHub-hosted Linux, macOS and Windows;
- ordinary-primary fresh/existing 100-cycle acknowledged kill/reopen histories
  on those same three native platforms; and
- the three light lanes, `docs-policy`, `scripts-policy` and `engine-fast`,
  described under proportional lanes, and the `repository-invariants` job
  that runs on every candidate the full matrix does not cover; and
- one always-evaluated gate which fails unless every selected partition
  succeeded and every unselected partition was skipped.

The preflight responds to a candidate whose changed operation catalogue left a
stale synthetic fixture: the native reopen smoke rejected that identity only
after its release build. On a valid candidate the added dependency can increase
wall-clock time; measure the actual preflight and matrix durations before
claiming a speedup. It does not replace a native smoke, reopen, crash, capacity
or soak test. The aggregate gate explicitly rejects a skipped, cancelled or
failed preflight and the same outcomes from every other required partition.

### Inspect candidate failures while jobs are running

Use the existing authenticated GitHub CLI to examine one explicit Actions run
or compare two attempts' current snapshots:

```sh
python3 scripts/ci/inspect_candidate_runs.py 40991825021 40996295111 --pr 208
```

When a candidate or protected merge group fails, inspect its exact run and, if
available, the preceding candidate before deciding whether to rerun or repair.
Record the failed command and native platform, run and attempt IDs, tested
source/tree, job link and any retained artifact in the owning package journal.
Use the result to choose a focused reproduction; do not infer a root cause from
a duration or replace the original failure with a later unrelated pass.

The report identifies the event head, tested reusable-workflow revision and
tree, attempt, incomplete or failed jobs and steps, matching named failure
durations, and up to ten longest completed steps with valid timestamps. The
timing list includes successful steps and is a snapshot, not a diagnosis. It
compares event heads with the current PR head, so a merge-group
head will normally differ. A shared tree does not establish identical execution
environment or PR association. Inspect the run and job links and logs before
diagnosis; timestamps and unfinished job inventories can be incomplete. The
tool refuses inconsistent source/attempt metadata and incomplete paginated
inventories. Its JSON is an informational snapshot, not a CI receipt, acceptance
decision, root-cause diagnosis, or authority to skip a gate. The existing
`rrflow-qualification-storage acceptance ci-receipt` remains the evidence boundary.

The Linux lifecycle and Windows/macOS portability lanes also build the native
`rrflow` primary and Rust `rrflowdb` example, stage separate digest-checked
copies, and run the installed example over authenticated loopback HTTP. The
runner checks the exact synthetic task and owning relationship at one read
stamp, changes the task, rejects the old expectation, verifies the stopped
engram and reads the same state after reopening. It bounds child output and
execution, and terminates only its own processes. This is a small fresh-project
integration corpus, not the complete H-04 SDK matrix or C-07 qualification.

The Linux topology and Windows/macOS engram partitions also require
`projected_lifetime` and `reader::lifetime_tests`: overlapping empty-reader
admission, mixed generations and terminal source-owner release, with copied
output surviving database/stream disposal. Weak-owner and registry checks
cover memtable, raw mmap, raw bounded and adaptive-LZ4 paths. These are native
ownership regressions, not aggregate RSS, installed reclamation or soak proof.

The native partitions also require `mutable_admission`: configured writer and
read-only reopen denial preserves typed byte/version errors and existing file
bytes; admitted inspection recovers exact semantic and control identities. The
existing semantic and engine admission suites include both native mutable
resources through prior-ACK/rejection/reopen/replay/abort/next-work histories.
These component tests do not qualify installed configuration.

The native partitions also require the `configured_` histories in
`logical_archive_resilience` and `backup_catalogue`: one native policy follows
logical/application restore, partial-prefix resume, catalogue/control replay
and verification reopens. Rejected work stays private; an adequate policy
resumes exact runtime/control/object evidence without duplicate effects.
Denied replay preserves file bytes. These are component restore limits, not
installed configuration binding, aggregate recovery memory or capacity proof.

Those same partitions separately require `projected_output_limits` and
`segment::output::tests`. These qualify output-byte admission before allocation,
last-owner release through buffer clones/slices, concurrent readers, failed
batch construction and retained output through reclamation/reopen. The native
pool is scoped to one database-open instance and excludes source pages, caller
copies and allocator metadata; these checks do not establish total-memory or
installed capacity limits.

The same native partitions require the `interleaved` filter in
`projected_read_adversarial`, with a five-minute step limit. Nine fixed histories
combine two reader stamps, a pinned transaction, mixed-family writes/deletes,
checkpoint release, compaction, actual GC and reopen. A joined reader thread
uses capacity-one channels and 30-second receive deadlines. Independent model
rows and physical-file checks require retention before terminal release and
reclamation afterward; copied Arrow batches and individual buffer clones
survive database close and disposable-directory removal. Mapped raw, bounded
raw and bounded adaptive-LZ4 paths must report that they were exercised. This
is native component lifetime evidence, not installed reclamation or memory
capacity qualification.

The same three native partitions require `mutable_limits`: hard active-state
byte/version admission, pre-effect rejection, hot-key pressure with held
history, and writer/read-only WAL replay denial followed by exact recovery.
This accounts active key/value/version charges; it does not establish aggregate
heap/RSS, installed configuration binding or whole-startup work limits.

The same three native partitions require `write_diagnostics` and
`rrflow_kv_operator`. Uncontended timing may be zero; measured phases/waits
must fit their independent outer duration and supplied lock context survives
exactly, including zero and saturating values. Bounded sample/drop accounting,
maintenance counters and durable readback remain required. Positive timing is
not evidence of physical synchronization; the native durability/I/O suites
retain that responsibility.

The topology and native portability jobs run `installed_lifecycle` against the
actual primary executable, without a surrogate executable-identity fixture.
Those jobs omit dev/test debug symbols and log executable size; executable
identity still binds the actual tested bytes. Startup retains a 60-second
bound. The backup/restore lifecycle client uses an explicit 60-second request
timeout; this does not qualify the default short-request timeout for heavy
physical verification or generic uncertain-outcome recovery.
Fresh and existing projects cover no-write deterministic plan, exact apply,
authenticated discovery, commit/query, backup, termination, restarted query,
offline verification, and restore publication/readback. All native platforms terminate the initial owned child after acknowledged work;
subsequent Unix shutdowns check graceful SIGINT. Restart discovery verifies the
installed instance and closed authentication session; restored readback compares
query, scope, schema, read-manifest and row identity with the original result.
This bounded corpus does not prove graceful Windows shutdown, a complete
crash matrix, clean-machine distribution, rrflowMX/rrflowKV equivalence, native
graph/BM25/vector projections, streamed Arrow/DataFusion, or model reasoning.
The shared SDK harness is also current characterization: the deficiencies and
real-process corpus required for H-04 remain open. Neither job may be cited as
complete-engine evidence.

The native lifecycle harness stages a copy of the actual primary executable
in a temporary directory containing spaces and gives each product child an
empty working directory, isolated home/temp directories and an empty executable
search directory. Only Windows OS-discovery variables survive from the parent;
provider credentials, proxy settings and Cargo/developer configuration do not.
Its child guards retain the fixture until the process is reaped.

All three native lanes require `python scripts/ci/run_offline_lifecycle.py
--receipt <runner-temp>/rrflow-offline.json` (Linux uses `python3`). The runner
requires exactly the cases in its `REQUIRED_CASES` set, each passing once with
no ignored or filtered cases. Fresh/existing receipts bind the actual executable,
build metadata, public-contract and signal digests, installation/restore plans,
backup, read manifest and asserted durable cursors. Unknown receipt fields are
rejected. Retained artifacts contain these bounded, non-secret receipts and
sanitized failure status; raw engrams and credential-bearing output are never
uploaded. The [D-01 acceptance record](../roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21)
binds the first accepted three-platform result to its exact candidate. Later
changes still require their own candidate CI; this result cannot qualify a
different revision or a signed platform bundle.

The `native-arm64` candidate job additionally requires an actual Linux ARM64
runner and an `aarch64-unknown-linux-gnu` Rust host before executing the native
storage/recovery, source-admission and primary-executable installed-lifecycle
tests. Its result is part of the same aggregate gate for ready PRs and merge
groups. The job log identifies its candidate and host; a cross-compile or a
skipped job is not native evidence. This bounded lane does not run the full
offline-denial, 100-cycle, native-capacity matrix or 24-hour soak on ARM64 and
does not claim those qualifications. Those remain with their existing owners.

The C-07c client-history extension runs after each fresh/existing restore
fixture. A bounded test-owned relay forwards one commit and suppresses its
entire response. The public client records uncertainty; three forced native
kill/reopens require stable exact retries, six predeclared claim values, no
rejected effects, quota recovery and an unchanged runtime head. Each mode
retains four logical commit identities and 13 attempts (11 acknowledged,
one rejected, one initially uncertain) checked by the existing `rrflow-qualification-storage`
ledger. The runner requires both closed, build-bound `storage_histories`
receipts and rejects omitted schedules or unknown nested fields. Production
engine bytes have no fault switch. These three-cycle response-boundary cases
do not replace C-07c's internal WAL/publication pauses or dedicated 100-cycle
profile corpus, and do not qualify power loss or real capacity exhaustion.

The separate `installed-reopen` job explicitly selects both ignored long tests
in `installed_reopen` with default features and the existing optimized
`--release` Cargo profile. This is a source-built ordinary artifact, not a
signed distribution or an official release. Each test freezes 100 independent
two-claim requests before execution. Every ACK is followed by an owned process
kill and reopen, exact full-history readback before retry, replay of every
previous transaction with stable receipts, and another cursor/readback check.
Each mode reconciles 5,150 attempts and records 100 kill/reap/restart chains
plus final cleanup. Startup and request deadlines remain 60 seconds; the
corpus has a one-hour deadline, each child a three-minute deadline, and bounded
capture. The job's 90-minute budget includes compilation and qualification;
it does not change the existing lifecycle or diagnostic job budgets.

The actual ordinary artifact must exclude the diagnostic protocol marker.
Build, installation-plan and effective configuration identities accompany the
sanitized receipts. CI requires both complete receipts and retains them as
artifacts; default-suite ignored results are not qualification. This is a
small fixed correctness workload under default installed settings. It does
not qualify J-04 constrained allocations, publication/reclamation boundaries,
power loss, hard resource limits, disk exhaustion or a 24-hour soak.
The existing `lifecycle_process` target also qualifies the relay with forced
nonblocking accepted sockets, fragmented header/body input, interrupted I/O,
native read expiry, bounded malformed-input denial, redacted errors and idle
guard cleanup. These helper cases preserve the required installed lifecycle corpus.

Preflight failures retain a bounded failed receipt even before build hashes are
available; cleanup failure always overrides any prior passed receipt.

Linux copies the binary, test harness and isolation helper into `/tmp`.
Noninteractive sudo creates private mount, network and PID namespaces, masks
the checkout parent (including siblings) and Cargo/Rustup roots, and enables
only loopback. Namespace separation, empty masked directories, known source
file absence, absent default routes and IPv4/IPv6 `ENETUNREACH` are required.
The actual product runs as the original non-root user. Privileged timeout and
PID-namespace teardown cover descendants. util-linux, iproute2, Python and sudo
are qualification infrastructure, not product dependencies.

macOS requires an inherited `sandbox-exec` profile. Directory listing and
known-source read/write probes must deny; direct and descendant IPv4/IPv6
connections must return permission denial while loopback works. The fixed
profile receives source roots as parameters. Missing enforcement fails; it
cannot downgrade to an ordinary staged test. An owned process group bounds
cleanup. This deprecated OS utility is a test prerequisite, not an RRFlow
installation dependency.

The filesystem runner also requires a macOS APFS image mounted below a retained
project anchor. It asserts a changed device identity and denial of read,
truncate, create, publication and removal. The runner detaches only its exact
owned image before removing the fixture, including after bounded child failure.
Linux keeps the private-namespace bind-mount adversary; Windows keeps the native
junction corpus. These cases require their respective native CI results.

Windows enforcement runs only on ephemeral GitHub-hosted runners. A disposable
principal uses an owned noninteractive window station/desktop with explicit
access; token-owned runtime-file probes precede launch. Those objects are
qualification infrastructure, not product UI or installation dependencies.
Inherited deny ACLs cover only that standard principal in declared source trees.
A no-reparse inventory bounds each tree to 200,000 objects and 120 seconds;
native access checks verify denial on every object before launch. Native fixtures
require inherited denial, protected-child bypass rejection and exact ACL
restoration. A complete post-cleanup inventory verifies original ACL digests.
Traversal bypass is retained for Windows process startup; source isolation
depends on verified object ACLs, never a root-only deny. A dynamic WFP session denies
its non-loopback IPv4/IPv6 connections. The Windows IPv6 probe uses a preferred
link-local address scope from a connected native interface, discovered read-only,
so missing global IPv6 routing cannot substitute for a permission-denial result.
No route or address is changed. The child starts suspended and enters a
kill-on-close job before execution. After all owned descendants terminate, the
same principal removes its private test home/temp trees; exact source ACLs,
WFP objects and account are cleaned up. Missing privilege, blocked cleanup or
failed direct/descendant probes fail qualification. No policy/account changes
run on the developer workstation.

The CLI qualification helper bounds ordinary commands to 120 seconds, service
lifetime to 480 seconds, readiness to 60 seconds/64 KiB, each output stream to
16 MiB and each lifecycle's collected output to 128 MiB. The discovery catalogue
currently exceeds 4 MiB; the measured Windows response is 5,598,305 bytes.
Real helper cases exercise timeout, output flood, oversized readiness, finite
capture and concurrent copy/launch. One short admission lock covers executable
copy and every child spawn, while lifecycle tests continue concurrently. This
addresses the documented inherited writable-descriptor mechanism without a
retry sleep; it does not by itself prove the cause of an earlier CI failure.

`python scripts/ci/run_private_owner.py` explicitly executes the privileged
foreign-owner file/directory case on each OS. Unix requires root; Windows
requires `SeRestorePrivilege` and restores the previous token state. Missing
privilege fails, and the ordinary suite's ignored marker is never acceptance.
Native installation suites include bootstrap, concurrency and recovery together.
OS libraries remain available. These checks do not prove hostile-code
containment, a signed clean-machine bundle, graph/vector scale or alpha.

The Linux installed-lifecycle job also runs the
[primary build-provenance helper](../reference/release/build-identity.md) on
clean candidate source. It observes Cargo's actual first-party compilation
units, embeds their declared provenance, rebuilds, and rejects changed source,
feature/semantic-profile closure or executable identity. Repository verification
runs the helper's dirty-source, drift, environment-digest and diagnostic-profile
policy tests. These are build-provenance checks; actual release/diagnostic
semantic parity remains open under H-05a.

Native filesystem containment runs the LSM filesystem and manifest fixtures
on Linux, Windows and macOS, plus installation ancestor-replacement cases.
Linux additionally runs `python3 scripts/ci/run_filesystem_containment.py`:
Cargo identifies the exact test executable, and noninteractive sudo launches
the required bind-mount adversary in a private mount namespace with private
propagation. Its 60-second timeout and 5-second kill bound prevent a stalled
fixture from holding the runner indefinitely. The test asserts namespace
separation and unchanged outside bytes after denied read, truncate, create,
publication and cleanup. Missing privileges or namespace support fails the
candidate. The ordinary suite's ignored privileged case is not acceptance;
the explicit isolated invocation must pass. This bounded fixture does not
replace D-01.5's wider native filesystem and offline qualification.

The repository-policy partition generates and checks the complete Git-backed
inventory in ignored `target/rrflow-1.0-file-plan.jsonl`, then retains it and
the exact checked-out Git revision as a CI artifact. The generator must cover
every tracked or non-ignored path and every long-lived future path registered by
its generator; a missing or stale local output fails `--check`. CI rejects dirty
or untracked non-ignored source before generating the artifact. The stable
`docs/roadmap/rrflow-1.0-file-plan.jsonl` link is a retirement descriptor, not
current inventory. Generate a current local view using
`python3 scripts/ci/build_execution_inventory.py` and check it with the same
command plus `--check`. Newly created paths become current inventory records;
the pull request description still binds their intended scope. This removes a
shared generated merge-conflict point without removing any candidate CI test.

For a local conflict preview, run
`python3 scripts/ci/inspect_merge_conflicts.py BASE_REVISION HEAD_REVISION`.
The report binds both revisions to commit IDs and marks only paths in the
existing generated-projection registry with their generator. Every other path
requires review, including authored digest citations. Git may report a conflict
without individual paths; that is still a conflict. The command does not modify
the index, working files, or refs, but Git may write unreachable merge-tree
objects. It does not resolve anything or authorize a merge. The existing
candidate and protected merge-queue checks remain required. The command refuses
configured external merge drivers and is intended for trusted checkouts; it is
not a security sandbox. `python3 scripts/ci/test_inspect_merge_conflicts.py`
in the verify job covers these behaviors with temporary Git repositories.

The Connectome UI is optional for engine operation, while candidate CI follows
repository policy and recursively materializes every declared submodule. The
Connectome development repository is public and can be acquired anonymously.
The [publication receipt](../evidence/change-journals/gate-h/h-06-connectome-public-association.md)
records successful recursive checkout after the visibility change; the full
candidate pipeline remains required before merge.
The architecture suite checks the exact Connectome source identity and denies
engine dependencies or targets inside it, alongside the general pinned-source
validator. The inventory records the parent gitlink without traversing child
files. `python3 scripts/ci/test_execution_inventory.py` proves parity across
absent/populated/edited child files, pin-sensitive identity, and conflict
rejection. UI builds and real-engine conformance remain separate H-06 evidence.

Marked documentation indexes are derived from their immediate coordinated
children. Run `python3 scripts/knowledge/render_navigation.py` after adding,
moving, or removing a child record; `--check` is non-mutating and fails on
drift. The same tool derives the coordinate/link relationship graph used for
future knowledge import and retrieval. Generated listings and edges are
navigation, not documentation or lifecycle authority.

The same partition enforces the documentation bootstrap package. The
documentation policy builds and validates two in-memory packages through the
single exporter, while the workflow exports twice at the checked-out revision,
compares the exact JSONL bytes, and runs the focused drift corpus. Package
outputs remain temporary and are never editable documentation authority.

The five engine suites cover every workspace package exactly once, but package
names do not define product ownership. The partitioning avoids one
workspace-wide test/link target graph on the hosted fallback without turning
each implementation package into a separate product check.

The installed-reopen release build is bounded to two Cargo workers on
`windows-latest` and one on Linux/macOS. In PR150 candidate run 40503118473
attempt 2, its Windows build took 63m24s and the 100-cycle history step took
4m06s; this is an observed baseline, not proof of the bottleneck's cause. The
two-worker candidate must pass the complete native pipeline and be timed before
any speed or memory-safety claim. Commands, platform matrix and gates are
unchanged.

## Presubmit and change-evidence boundary

Repository presubmit is not an RRFlow runtime hook. It cannot create an engine
event, trigger a persisted routine, resolve a skill, mutate an engram, or claim
that an effect completed. Gate I owns those future runtime capabilities.

The portable development control has three layers:

1. the checked-in commands and policy scripts are the reproducible presubmit;
2. candidate CI runs those exact controls and reduces them to
   `pipeline / ci-gate`; and
3. repository branch policy makes the pull request and stable check
   non-bypassable when the GitHub account tier supports that control.

Every code-bearing or structural package lands as one pull request against
`main`. Its description carries the change brief defined by the
[change-authoring procedure](../roadmap/rrflow-1.0-execution/change-authoring.md#required-change-checklist).
Candidate CI runs on that pull request and must pass before merge; a direct
push to `main` is not a landing path. The pull request description begins at
the repository-effect boundary. It is not a workflow for exploratory reading or
private AI reasoning and must not contain hidden chain-of-thought.

The planning-only commit, `docs/roadmap/rrflow-1.0-active-change.json`, and
`scripts/ci/check_change_plan.py` were retired on 2026-10-16. Between
2026-08-20 and 2026-09-15, 442 commits reached `main` without a pull request;
candidate CI triggers only on `pull_request`, `merge_group`, and
`workflow_dispatch`, so none of them ran it automatically, and the per-change
validator passed at `cfa0e80` while the generated-surface, backup-fixture, and
edge binary-ceiling checks failed there. The active-change file stays unchanged
as historical evidence linked from earlier journals; nothing reads or updates
it.

Candidate CI proves the checked-in controls, not algorithm correctness or alpha
readiness. Only the separately evidenced server-side protection below can make
the pull request and required candidate check non-bypassable.

A developer may run the
[repository checklist](../roadmap/rrflow-1.0-execution-map.md#repository-wide-run-checklist)
locally before publishing, but the local run is convenience and early feedback,
not authority. The repository installs no Git hook: client hooks are not
distributed by clone and can be skipped, so no hook can carry this gate.

## Local development container

`.devcontainer/` reproduces the candidate-CI toolchain on any host that runs
rootless Podman (or Docker) on amd64 Linux, including Podman machine on
Windows. It is local reproduction and early feedback only: a container run is
not candidate-CI, J-02, POAM-027, alpha, or release evidence, it is not a
hook, and it declares no editor lifecycle command.

| Target | Contents | Use |
|---|---|---|
| `rust` | snapshot-pinned Ubuntu 24.04 build dependencies, rustup 1.29.1, Rust from `rust-toolchain.toml`, uv, and the `actions/python-versions` CPython build that `setup-python` installs as `python3` | engine work, policy scripts, focused tests; the VS Code default |
| `full` | `rust` plus Node, pnpm, Go, Temurin, Maven, and the .NET SDK | TypeScript/SDK work and `scripts/ci/run_sdk_conformance.py` |
| `lab` | `rust` plus the Docker CLI, copied from Docker's digest-pinned image | the comparison lab's runner, started by `eval/lab/pod.sh`; diagnostic evidence only |

Every base image is digest pinned, every apt package resolves from one fixed
Ubuntu snapshot, and every downloaded toolchain is verified with
`sha256sum`/`sha512sum --check --strict` against a checksum in the
Containerfile. Certificate bootstrap extracts trust roots from an Apt-authenticated
package without installing a moving OpenSSL dependency; installed packages
then resolve from the fixed snapshot. The Temurin download tag is recorded
separately from the equivalent SemVer selector used by `setup-java`.
`scripts/ci/check_workflow.py` fails when a container version,
checksum, or image digest diverges from `rust-toolchain.toml` or
`.github/workflows/ci-reusable.yml`, or when `devcontainer.json` declares a
lifecycle command. Build steps are POSIX `sh` because Podman's default OCI
image format ignores `SHELL`.

CI and the container track the latest stable release of every toolchain,
using the long-term-support line where a project publishes one (Node.js 24,
Java 25, .NET 10) and the Ubuntu release that GitHub's `ubuntu-latest` runner
uses (24.04). A toolchain upgrade changes `.github/workflows/ci-reusable.yml`,
`rust-toolchain.toml` where applicable, and the Containerfile in the same pull
request; the parity check rejects one without the other.

Keep the source on a Linux filesystem; a Windows bind mount is slow and
reintroduces executable-bit and line-ending drift. Two entry points share the
`rrflow-cache` volume (Cargo registry, target directory, and language caches):

- **VS Code:** set `dev.containers.dockerPath` to `podman`, then run
  *Dev Containers: Clone Repository in Named Container Volume* for the
  development repository. VS Code builds the `rust` target.
- **Podman CLI:** from an existing checkout, with a compose provider such as
  `podman-compose` on `PATH`:

```bash
podman compose -f .devcontainer/compose.yaml build
podman compose -f .devcontainer/compose.yaml run --rm workspace \
  /seed/.devcontainer/seed-workspace.sh
podman compose -f .devcontainer/compose.yaml run --rm workspace
```

The seed script clones committed history from the read-only `/seed` mount of
the invoking checkout into the `rrflow-src` volume, restores its remote names,
URLs, and upstream, and then initializes every declared submodule (such as
`connectome`) at its pinned gitlink revision, matching a recursive CI checkout.
It copies no credentials, hooks, local Git
configuration, or uncommitted files, and it refuses a non-empty volume. Set
`RRFLOW_DEV_TARGET=full` to build and enter the SDK image.

The optional pgvector service uses the candidate-CI image digest, database,
and credentials on host networking, so the CI test URL works unchanged:

```bash
podman compose -f .devcontainer/compose.yaml --profile pgvector up -d pgvector
RRFLOW_PGVECTOR_TEST_URL='postgresql://rrflow_test:rrflow_test_password@127.0.0.1:5432/rrd_operator_knowledge_test?sslmode=disable' \
  RRFLOW_PGVECTOR_TEST_DISPOSABLE=1 \
  cargo test -p rrd-operator-knowledge --all-targets --all-features --locked
```

Toolchain downloads are development build inputs, like the CI setup actions;
they are never RRFlow install-time or runtime fetches. Only amd64 is
supported; the checked-in digests and checksums name amd64 artifacts.

The documentation policy machine-checks the owner chain, required
change-authoring procedure/evidence fields, generated-index freshness,
active-record classification, knowledge-package determinism, and local links.
The execution inventory checks every current/generated/registered-planned
path. Each package records its result in a linked record under
`docs/evidence/change-journals/`; the execution portal contains no journal
bodies. These checked-in controls remain bypassable outside candidate CI;
POAM-027 stays open for eligible server-side review/check/force-push/deletion
enforcement.

## Repository workflow policy

`scripts/ci/check_workflow.py` discovers every checked-in `.yml` and `.yaml`
under `.github/workflows/`; a newly added workflow cannot escape its
supply-chain checks. It requires:

- every external action to use a full 40-character commit SHA;
- every workflow service image to use a SHA-256 digest;
- every checkout step to set `persist-credentials: false`, and to set
  `submodules: recursive` whenever `.gitmodules` exists;
- top-level read-only contents permission and no `pull_request_target` in any
  workflow; and
- complete candidate-suite/default-feature/optional-feature coverage, bounded
  candidate jobs, safe fork routing, and exact final-gate reduction; and
- development-container toolchain, checksum, image, and pgvector parity with
  candidate CI, strict download verification, and no editor lifecycle command.

A Git submodule is a pinned source input, not an install-time fetch. The
workspace architecture test accepts a gitlink only when `.gitmodules` declares
it exactly once at a normalized repository-relative path with a
credential-free `https://` URL, so an anonymous clean clone and candidate CI
reproduce the same tree. Cargo workspace members remain tracked in this
repository, and the signed default distribution must still contain every byte
it needs after acquisition.

Workflow ownership is directory-wide in `.github/CODEOWNERS`, so adding a new
workflow does not bypass the existing workflow review boundary. Repository or
organization settings should additionally require full-SHA actions; that
external GitHub setting cannot be proven by this checkout.

## Scheduled storage diagnostics

`.github/workflows/rrd-lsm-benchmark.yml` is separate from candidate CI. It
runs manually and weekly on `ubuntu-latest`, uses the same immutable checkout,
toolchain, and cache action revisions, and uploads raw semantic-storage and
AI-access results with a pinned artifact action. It is intentionally not a
required `ci-gate` dependency.

The [rrflowKV benchmark reference](../reference/storage/rrflowkv-benchmark-harness.md)
owns those executable shapes and their limitations. Hosted runner output lacks
the fixed hardware, complete provenance, full governed engine flow, and
quality/failure corpus required by J-04; scheduling and artifact upload cannot
turn it into performance or competitive evidence.

## Diagnostic, performance, and fault lanes

The Linux installed lane and Windows/macOS portability lanes run
`cargo test -p rrd-lsm --lib fault_io --locked -- --nocapture` with a five-minute
step limit. This C-07b unit corpus wraps native file calls with deterministic
faults, then reconstructs a temporary engram from an independent stable-byte
and directory-entry model. Logs retain call/case counts and failing ordinals.
The model has fixed byte/file/event bounds and exercises compound recovery
failures. It is I/O-model evidence on each host, not native process-death,
ENOSPC, OS-crash, device-cache or physical power-loss qualification. The fault
controller is compiled only into `rrd-lsm` unit tests; no product feature or
runtime setting enables it.

The same filter also executes C-07c's `fault_io::crash` native process corpus.
Its owned unit-test child pauses at a nonce/PID-bound native boundary, then
the parent kills and reaps that exact handle. Forty borrowed/owned,
WAL-only/published-segment cases cover write, complete append, sync,
visibility, flush publication and reclamation; eighty additional kills interrupt
WAL replay. Two separate 100-cycle histories prove exact acknowledged values
and sequences through repeated reopen. Logs retain the test executable digest,
native OS, boundary, run identity and hit/reap timings. The one ignored child
fixture is explicitly invoked by these parent tests; no required parent is
ignored. The child accepts at most 4 KiB through its parent-owned stdin, emits
bounded identity-only receipts, and has a 30-second pause deadline. The parent
has a 20-second rendezvous deadline and 8 KiB per-stream capture bounds.

This corpus is instrumented native **storage-component** process-death proof.
It is not the installed primary binary, a loss of OS/device caches, native
ENOSPC, resource-envelope or sustained-load proof. The independent installed
client histories and the remaining C-07c composition/profile requirements stay
distinct. The I/O model remains entirely under `cfg(test)`. The owned process
rendezvous source is also reused by the explicit diagnostic primary below;
ordinary dependency builds exclude it.

The required `installed-crash` matrix runs on Linux, macOS and Windows. It
first builds the **ordinary** primary and streams its bytes to verify absence
of the crash protocol marker, retaining its SHA-256 and byte count in the log.
It then builds `rrflow-cli --features crash-test` and runs:

```text
cargo test -p rrflow-cli --test installed_crash --features crash-test --locked -- --nocapture --test-threads=2
cargo test -p rrflow-cli --test lifecycle_process --features crash-test --locked
cargo clippy -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --locked -- -D warnings
```

This separately identified diagnostic artifact requires a bounded private stdin
schedule (or explicit null for an unarmed run). The root comes from the installed
locator, and only the exact authorized operation digest arms the domain scope
after intent persistence. Five points cover pre-WAL, complete append, native
sync, visibility and persisted receipt before response. Each fresh/existing
case also kills reopen before the first WAL frame is applied. Independent rows,
cursors, prior ACK receipts and exact retries reconcile uncertainty without
double application. Negative controls reject inadmissible schedules and prove
an unrelated operation cannot trigger the selected boundary.

The two engram histories retain six logical calls and 21 attempts each,
including five uncertain client results; ten point/recovery hits per engram
bind PID, nonce, point and build identity. The third parent test qualifies
schedule admission. No parent is ignored. Native stdout/stderr capture is
bounded to 1 MiB per stream, input to 4 KiB, observed event lines to 64 KiB and
the event queue to four entries. Parent readiness/event waits are 60 seconds,
child lifetimes 180 seconds, diagnostic admission 180 seconds and the armed
native pause 30 seconds. The job allows 45 minutes including builds; the warm
test step allows 20 minutes and two test threads. Existing ordinary lane
limits are unchanged. `ci-gate` requires all three diagnostic jobs.

The same required native job now invokes
`python3 scripts/ci/run_native_capacity.py` for fresh/existing engrams at WAL,
segment, manifest and CURRENT-write boundaries. It creates a capped ext4 loop
image in a private mount namespace, fixed APFS image, or NTFS VHD. The harness
requires a separately enumerated small volume, an exact owner marker, a private
one-use pause/resume, and an actual native capacity error at the selected
phase. It preserves every acknowledged value/receipt, resolves the uncertain
call, explicitly publishes a repaired candidate when a final WAL frame is
incomplete, admits distinct new work, and reopens again. These are actual
filesystem-failure checks; they do not establish OS power-cut safety or a
24-hour operating envelope. Missing facilities report not-run with nonzero
exit; any failed cleanup fails qualification. The normal binary must contain
neither diagnostic marker. The job's 75-minute bound includes the existing
corpus and a 30-minute native capacity step. Local execution remains explicit:
this runner must never be pointed at an existing disk or mount.

The nondefault four-crate `crash-test` composition is owned by this dedicated
job; other optional features retain their existing matrix. It must never be
used to assemble the normal distribution. These tests do not close installed
publication/GC composition, 100-cycle operating profiles, resource limits,
retained buffers, real exhaustion, sustained capacity or the 24-hour soak.

The installed domain-crash job also runs the
[publication and reclamation corpus](../evidence/change-journals/gate-c/c-07c-installed-publication.md)
separately for fresh/existing engrams. It retains two bounded native receipts,
requires all six maintenance boundaries and their compound manifest-recovery
kills, and reconciles prior ACKs before retrying each uncertain target. The
ordinary domain step excludes these cases to avoid duplicate execution. Native
acceptance requires the explicit execution and receipt-validation steps;
component histories and a successful older job cannot replace them.

The
[engine observability contract](../architecture/engine-data-flow.md#runtime-modes-build-profiles-and-build-identity)
defines one semantic engine with release, optimized-diagnostic,
runtime-analysis, sanitizer, and benchmark modes. None of those modes is
currently a qualified RRFlow binary merely because Cargo can compile the
workspace.

Candidate CI stays bounded to deterministic correctness and repository policy.
Additional lanes enter the required gate only with an owning roadmap package,
declared target/support matrix, time/resource budget, retained failure
artifacts, and a false-negative analysis:

| Lane | Required use | Gate/evidence boundary |
|---|---|---|
| optimized diagnostic parity | Prove release and diagnostic profiles have identical feature closure, formats, operations, results, receipts, and resource limits; symbols/configuration may differ. | H-05, J-03, J-05 |
| deterministic fault/reopen | Inject WAL/manifest/segment/page, sync, acknowledgement, process-kill, ENOSPC, and corruption boundaries; verify exact state after reopen from a recorded seed/schedule. | C-07, D-11, J-02 |
| Loom | Explore bounded concurrent schedules around transaction, cache, publication, and cancellation primitives. | owning C-through-I package, J-02 |
| Miri/sanitizers | Detect supported undefined-behavior, address/leak, and race classes on deliberately bounded targets. | owning package, J-02 |
| Tokio Console/profiler | Diagnose scheduler, resource, lock, allocation, CPU, and I/O stalls only after ordinary correlated evidence identifies a reproducible case. | runtime-analysis; never conformance by itself |
| fixed-hardware benchmark | Retain raw histograms, failed samples, quality/correctness results, resource use, build identity, workload, and machine/filesystem/device provenance. | J-04 only |

Hosted runner benchmarks and ad hoc `cargo bench` output remain diagnostics.
They cannot support a latency, scalability, zero-copy, resource, or competitor
claim. The J-04 harness must use the canonical latency boundary and diagnostic
capture bundle, separate success/error and cold/warm behavior, and preserve
p50/p95/p99/p99.9 distributions rather than averages.

## Runner trust boundary

The repository variables below contain JSON `runs-on` values:

| Variable | Physical pool value | Safe default |
| --- | --- | --- |
| `CI_SELF_HOSTED_ENABLED` | `true` | disabled |
| `CI_LINUX_STANDARD_RUNNER` | `["rrflow-standard"]` | `["ubuntu-latest"]` |
| `CI_LINUX_HEAVY_RUNNER` | `["rrflow-heavy"]` | `["ubuntu-latest"]` |
| `CI_LINUX_ARM64_RUNNER` | operator-provisioned Linux ARM64 labels | `["ubuntu-24.04-arm"]` |

A pull request whose head repository differs from the base repository is
forced onto GitHub-hosted Linux even when the variables exist. Missing or
disabled `CI_SELF_HOSTED_ENABLED` also forces hosted Linux. The workflow does
not use `pull_request_target`. Same-repository branches, merge groups, and
manual proofs may use the private physical pool.

This routing expression is defense in depth, not permission to connect a
self-hosted pool to a public repository: a fork can propose a change to the
workflow itself. The installer queries GitHub and fails closed unless the
repository visibility is `PRIVATE`. Do not set the physical routing variables
or register the scale sets while the repository is public. The repository
owner confirmed public visibility on 2026-10-16, so candidate CI uses
GitHub-hosted runners and the routing variables stay unset.

GitHub recommends ephemeral runners for autoscaling and identifies Actions
Runner Controller (ARC) as its reference Kubernetes implementation. The
checked-in deployment is configured as follows:

- ARC chart version: `0.14.2`; controller, runner, and Docker images are
  digest pinned;
- `rrflow-heavy`: scale 0..1, 48 CPUs and 64 GiB total, isolated Docker daemon;
- `rrflow-standard`: scale 0..2, 16 CPUs and 20 GiB each; and
- maximum runner allocation: 80 CPUs and 104 GiB, leaving 8 CPUs and 24 GiB
  for the 88-CPU/128-GiB host, k3s, ARC, and filesystem cache.

Each job receives a fresh pod and an empty work volume. No runner worktree is
mounted from the host. The runner service account has no Kubernetes API
permissions, and only the heavy class has a privileged Docker sidecar.

References:

- [GitHub: self-hosted runners](https://docs.github.com/en/actions/reference/runners/self-hosted-runners)
- [GitHub: deploy ARC runner scale sets](https://docs.github.com/en/actions/how-tos/manage-runners/use-actions-runner-controller/deploy-runner-scale-sets)
- [GitHub: secure use of self-hosted runners](https://docs.github.com/en/actions/reference/security/secure-use)

## Physical-host bootstrap

Install a pinned single-node k3s `v1.36.3+k3s1` cluster on the physical host
and make its kubeconfig available to the administration machine. Do not share
this cluster with production workloads. Install Helm 4 and authenticate `gh`
as a repository administrator. The checked-in script does not install or
attest those prerequisites. Run it from the target checkout with an explicit
repository identity, or let `gh repo view` resolve the current checkout:

```bash
GITHUB_REPOSITORY=OWNER/REPOSITORY \
  KUBECONFIG=/path/to/physical-host.kubeconfig \
  scripts/ci/install-arc.sh
```

The installer validates an `OWNER/REPOSITORY` coordinate and denies
public-repository registration before creating separate controller and runner
namespaces. It streams the existing GitHub token directly into a Kubernetes
Secret, installs the version-selected OCI charts with digest-pinned images,
waits for the controller, installs both scale sets, and sets the repository
routing variables. It does not write the token to argv or a values file.

The chart fetch, GitHub API calls, and Kubernetes operations require network
and administrator access. This is CI-infrastructure bootstrap, not the RRFlow
offline distribution or per-project installation path, and no checked-in
record currently proves that the physical pool is installed.

Prove physical execution after installation:

```bash
gh workflow run ci.yml --repo OWNER/REPOSITORY \
  --ref <candidate-branch>
gh run watch --repo OWNER/REPOSITORY --exit-status
```

The standard and heavy jobs must report `rrflow-standard` and `rrflow-heavy`
runner identities. Windows, macOS, the final gate, and all fork pull requests
must remain GitHub-hosted.

## Merge alarms

`.github/workflows/merge-alarm.yml` runs `scripts/ci/merge_alarm.py` when `ci`
completes and when a pull request leaves the merge queue or closes. A failed
candidate, a removal from the queue, or a red `main` opens one issue labelled
`merge-blocked`, comments on the pull request and mentions its author. A later
failure adds to the same issue, which closes when the pull request merges or
closes. A red-`main` issue closes when `main` next passes.

Cancelled runs, drafts and failures on a superseded commit stay quiet. An open
`merge-blocked` issue comes before new work, for a person or an AI session.

## Repository enforcement

Public visibility exposes this control on the current hosting tier. `main`
is protected with these invariants:

- pull requests and the stable `pipeline / ci-gate` check are required;
- the merge queue is required;
- force pushes and deletion are denied; and
- Actions must require immutable full-SHA references (enforced in this
  checkout by `scripts/ci/check_workflow.py`; the hosted repository setting is
  still open, see below).

This is an external control and needs external evidence. On 2026-09-11,
read-only GitHub API requests for both repository rulesets and `main` branch
protection on the then-private `rrflow/rrflow-development` returned HTTP 403
with the message that GitHub Pro or public visibility is required. After the
repository became public, ruleset `25439032` ("main: pull requests with passing
candidate CI") was created on 2026-10-19 and is `active` on the default branch
`main`:

| Rule | Configuration |
|---|---|
| `pull_request` | required; 0 approving reviews (a sole maintainer cannot approve their own pull request); stale approvals dismissed; review threads must be resolved; merge, squash, and rebase allowed |
| `required_status_checks` | `pipeline / ci-gate` from GitHub Actions (integration `15368`); since 2026-10-24 the branch need not be up to date, because the merge queue tests the merged result |
| `merge_queue` | added 2026-10-24; merge commits; a queued group merges only when all its entries pass (`ALLGREEN`); up to 5 entries are built and merged at once; at least 1 entry, with a 5-minute wait; 180-minute check timeout, above the observed 64–92-minute candidate runs |
| `non_fast_forward` | force pushes denied |
| `deletion` | branch deletion denied |
| bypass | repository administrators, `pull_request` mode only: an administrator may override a rule while merging a pull request, but cannot push directly to `main` |

`GET /repos/rrflow/rrflow-development/rules/branches/main` reports all five
rules from that ruleset, and the GraphQL `mergeQueue(branch: "main")` query
returns the active queue with that configuration. A merge-queue candidate runs
the same caller through its `merge_group` trigger on a
`gh-readonly-queue/main/...` ref, which has its own concurrency group, and
reports the same `pipeline / ci-gate` check. A direct push of an empty probe commit to `main` with
administrator credentials was rejected with "Changes must be made through a
pull request" and "Required status check `pipeline / ci-gate` is expected",
and `main` was unchanged. Switching the bypass to `always` is one ruleset
update, but it would also let any tool holding administrator credentials push
directly to `main`.

The repository-level Actions setting that requires full-SHA action references
(`sha_pinning_required`) is still `false`. Every workflow already pins full
SHAs and `scripts/ci/check_workflow.py` enforces that in this checkout; enabling
the hosted setting remains open. Do not compensate with a local hook or push to
the official promotion repository.

The repository CI-policy check rejects candidate trigger duplication, unsafe
fork routing, unpinned actions, service or runner images, persisted checkout
credentials, over-broad workflow permissions, `pull_request_target`, missing
candidate timeouts, incomplete suite or feature coverage, missing gate
reduction, or a runner layout which exceeds the documented host allocation.
The Rust workspace architecture test remains focused on engine ownership,
dependency direction, and storage-opening authority rather than GitHub Actions
implementation details.

The macOS installed-crash lane additionally runs three planned fresh/existing
WAL capacity pairs, each on a newly owned APFS image. These regress the
intermittent selected-WAL error gap; the first failure stops the run. They do
not replace the full native capacity matrix or permit retry-until-green.

An owned APFS image creation failure records bounded `hdiutil create -verbose`
output, the failing mode and point, harness digest and CI run coordinates in a
dedicated runner-temp JSON file. A non-zero exit is recorded this way even when
a descendant keeps the command's output open; the record then names which stream
stayed open. The macOS job attempts to upload only those
diagnostic JSON files, including after a failed capacity step. No disk image or
product output is uploaded. A lost runner or failed artifact upload can still
prevent retention. The record does not classify the cause, retry the case, or
turn a failed native test into acceptance; see the
[C-07 diagnostic journal](../evidence/change-journals/gate-c/c-07-native-create-diagnostics.md).

The separate manual `c07-native-preflight.yml` workflow defaults to the same
three fresh/existing APFS pairs and controlled failure diagnostics on one macOS
runner. First it deliberately supplies an invalid filesystem to the native
image-create command and verifies that the runner retains the resulting error;
that injected receipt is uploaded separately from any real capacity failure.
The injected failure is a diagnostic test, never a capacity pass. Once that
workflow exists on default `main`, dispatch an exact branch
with `gh workflow run c07-native-preflight.yml --ref <branch>` and verify the
run's recorded head and tree. To investigate one failure without the other
cases, for example, add `-f point=wal_before_write -f mode=fresh -f repetitions=1`.
The wrapper validates these selections against the native runner's point list
and preserves its failure and cleanup behavior. This is a focused diagnostic,
not candidate CI, the required merge group, the full capacity matrix, or a soak. A passing run
does not erase a prior failure or prove its cause. Normal PR and merge-group
events continue to run the unchanged complete pipeline.
