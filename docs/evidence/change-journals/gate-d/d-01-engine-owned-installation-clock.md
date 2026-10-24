# D-01 engine-owned installation clock evidence journal

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-engine-owned-installation-clock`
**Owner:** package receipt for `D01-05-engine-owned-installation-clock-v1`; the
roadmap retains completion authority

This record reports one bounded installed-lifecycle prerequisite. RRFlow now
owns the first installation-time observation inside `RrdEngine`, persists its
source and trust classification, preserves it through interrupted recovery,
and diagnoses or rejects relative rollback. It does not mark D-01, D-02,
D-11, OBJ-07, POAM-006, Gate J, the alpha objective, release readiness, or
promotion complete. In particular, an unverified host clock is not described
as trusted UTC.

## Bound authority, baseline, and planning chain

- Alpha prerequisite advanced: remove caller-selected installation time and
  establish the first versioned clock observation/rollback substrate at the
  governed durable installation boundary.
- Source baseline revision: `75a1415806613c80ea06f1931121e388f6aeea6c`;
  tree: `c41e7bb21aaeac14c6002dc518e5569f3220a3f8`.
- Planning-only commit: `fdf013dc3341070bd10d4bec0af1611dd7c66d72`;
  tree: `4ed13ec4504e089ff85d68656b2071adcb08144d`; sole parent:
  `75a1415806613c80ea06f1931121e388f6aeea6c`. Only
  `docs/roadmap/rrflow-1.0-active-change.json` changed.
- Corrective planning-only commit:
  `e28c16d1ad3231ee785b54359226a78a39c997d4`; tree:
  `16a07658ab9c60133d9495b91570a3437d29f784`; sole parent:
  `fdf013dc3341070bd10d4bec0af1611dd7c66d72`. It updated only the active
  change record so the declared baseline is its direct parent and removed the
  Rust signal projection from changed evidence after generation proved that
  canonical kernel-owned file remained byte-identical.
- Shell-quoting planning correction:
  `95db006cc6e26f0252c7bbfad47c47f26818fc90`; tree:
  `16d0489e0cf8d5e55351cf17c6d48d020d8195b3`; sole parent:
  `e28c16d1ad3231ee785b54359226a78a39c997d4`. It changed only the
  active record, made the edit-sequence instruction baseline-relative, and
  quoted the .NET test filter so its `|` is an argument rather than a shell
  pipeline.
- Final xUnit-runner planning correction:
  `c6f262e8b8f5622ad2a91909970409abf0030937`; tree:
  `932bc81b6fae082061dd2e247abac7fdbeeb19eb`; sole parent:
  `95db006cc6e26f0252c7bbfad47c47f26818fc90`. Complete review of the
  repository's .NET reference, SDK pin, common build policy, and executable
  xUnit v3 project proved that `dotnet test` may exit zero without discovery.
  Only the active record changed; it now invokes the established test
  executable with class filters from the pinned SDK directory.
- `python3 scripts/ci/check_change_plan.py` accepted the planning commit before
  implementation and accepted all corrections; the final plan at
  `c6f262e8b8f5622ad2a91909970409abf0030937` had zero post-plan paths.
- Branch: `agent/connectome-temporal-runtime-visualizer`.
- `Cargo.lock` remains at SHA-256
  `316533e7512dbb9029e494386503ca8430b0c69853b3b78ef02319dcbab93a27`.
- `development` resolves to the private incremental repository
  `https://github.com/rrflow/rrflow-development.git`. `origin` resolves to the
  promotion-only `https://github.com/rrflow/rrflow.git`. No official-origin
  push, tag, artifact, release, signature, or promotion is part of this package.

## Change brief

At baseline, public `RrdEngine::apply_installation` required every adapter and
test to provide `at_unix_ms`. The primary CLI read `SystemTime` before command
dispatch, panicked before the Unix epoch, and supplied the number as engine
authority even for commands that did not need time. Recovery froze only that
bare integer. Installed records, open, quick inspection, and reports carried no
source, trust, observation failure, rollback policy, or assessment. A host
clock behind installation did not prevent installed open. The primary process
test was Unix-only and sent SIGINT, proving graceful signal handling rather
than abrupt portable process loss.

The bounded target is one clock boundary, not an all-purpose time service:

- format-2 `EstateConfiguration` owns bounded
  `clock.maximum_rollback_ms`, with zero as the strict default and one hour as
  the compiled maximum;
- production installation obtains a fallible `SystemTime` observation inside
  `RrdEngine`; public callers cannot inject a source or timestamp;
- the observation records format, `host_system_time` source, `unverified`
  trust, and Unix milliseconds, and becomes the first owner-only intent anchor;
- exact recovery never substitutes the anchor; the installed record digest
  commits to it and the v2 checkpoint must name the same anchor coordinate;
- recovery, exact replay, and open fail closed when observation is unavailable
  or rollback exceeds policy;
- read-only inspection completes its structural checks and reports `current`,
  `rollback_within_tolerance`, `rollback_exceeded`, or `unavailable`; the last
  two make overall verification fail without changing estate bytes;
- CLI version and plan are clock-independent, quick verify returns failure for
  a failed report, and the primary lifecycle uses bounded `Child::kill`; and
- OpenAPI, language SDK endpoints/types, fixtures, and signal identities are
  regenerated from their owners.

The product remains `1.0.0`. This package does not set the OS clock, contact a
time service, add a dependency, authenticate NTP/NTS, establish a TPM-backed
clock, solve forward clock error, define runtime-wide timestamp high-water
rules, alter semantic valid time, or schema private reasoning. It does not
implement portable user/machine identity, attunement execution, external
capability grants, backup/restore, destructive deletion, ACL qualification,
Kubernetes qualification, or signed offline distributions.

## Implementation traceability and result

| Baseline behavior | Canonical destination and result | Proof | Owner |
|---|---|---|---|
| Public apply accepted caller time | private `ClockSource`, production `HostSystemClock`, and public diagnostic values inside `rrd-engine`; only private tests use `FixedClock` | all outward callers compile without time; engine test rejects unavailable apply before project writes | D-01/D-02 |
| Configuration v1 had no rollback policy | sole bundled format-2 configuration and v2 domain-separated digest with strict bounded `ClockPolicy` | five configuration tests plus refreshed public/attunement fixtures and generated OpenAPI/SDK projections | D-01 |
| Recovery intent and installed record stored a bare time | version-2 intent/record bind the complete first `ClockObservation`; the v2 checkpoint is cross-checked against its coordinate; retry reuses its exact bytes and derived semantic time | eight-stage recovery corpus plus explicit recovery rollback/no-write test | D-01/POAM-006 |
| Replay and open ignored host rollback | one assessment compares observation, anchor, and sealed policy before accepting recovery/replay/open | strict one-millisecond replay/open denial and configured 250-ms acceptance | D-01/D-02 |
| Verification could only pass and carried no time evidence | typed failed status, clock assessment, clock check digest, CLI trace/rendering, and nonzero failure exit | unavailable/exceeded read-only reports retain byte-identical estate inventory | D-01/D-11 |
| Primary lifecycle used external Unix SIGINT | standard-library bounded abrupt termination and reopen, with the exact command scheduled in Linux and Windows/macOS jobs | four-test Linux primary lifecycle finishes in 1.87 seconds; workflow checker requires both job placements | D-01/J |
| Clients repeated the old effective configuration | contract-owned OpenAPI plus generated endpoint/signal projections and five language fixtures carry format 2, policy, and digest | generator parity and TypeScript/Python/Go/Java SDK tests | H-04/J |

No implementation was moved. The superseded pre-release
`default-estate-configuration-v1.toml` was removed directly; there is no
successful format-1 fallback or migration lane.

## Complete file review

Fifty-nine baseline files were read in full before their dependent edits. The
planning record binds each exact SHA-256, line count, complete line coverage,
and relevant symbols. The review comprised:

- complete repository instructions, README, objective, roadmap, Gate D,
  POA&M/POAM-006, ADR-0002, provider-neutral bootstrap, execution portal and
  change-authoring procedure;
- complete canonical configuration research, installed lifecycle, security
  authority, and public-contract references;
- the complete 6,378-line contract owner, configuration module/tests and both
  complete fixtures;
- complete profile/configuration assets, engine errors/composition/exports,
  1,898-line installation coordinator, recovery intent, both installation test
  modules, and the complete engine-authority journey;
- complete primary CLI dispatch/installed adapter/lifecycle test, both MCP
  process tests, 712-line local driver test, 3,109-line HTTP process test,
  SDK-conformance server, and 1,497-line Rust client real-server test;
- complete reusable workflow and workflow-policy checker;
- all five complete SDK generators and client tests, the complete generated
  surface checker, navigation renderer, and execution-inventory generator; and
- the complete .NET SDK reference, pinned SDK selection, common build policy,
  and executable xUnit v3 project used to correct the acceptance runner.

After implementation, hand-authored changed files and the complete diff were
reread. Generated files were compared byte-for-byte with their declared
generators, and fixtures were decoded/validated by their owning Rust tests.

## Implementation and evidence paths

Created implementation paths:

- `crates/authority/rrd-engine/src/engine/clock.rs`; and
- `crates/authority/rrd-engine/assets/install/default-estate-configuration-v2.toml`.

Deleted implementation path:

- `crates/authority/rrd-engine/assets/install/default-estate-configuration-v1.toml`.

Changed implementation paths are the declared contract configuration/export/
tests; install profile; engine error/composition/installation/recovery/export/
tests; CLI dispatch/installed/lifecycle; MCP, server, and Rust-client callers;
reusable workflow/policy; and five SDK client fixtures. Changed evidence paths
are README, configuration research, installed lifecycle, security authority,
public contract, POAM-006, both contract fixtures, generated OpenAPI/endpoint/
language-SDK signal projections, this journal, its generated Gate D index, and
the generated Git-backed file inventory. The kernel-owned Rust signal
catalogue remained byte-identical and is verified, not listed as changed
evidence. `Cargo.lock` did not change.

## Research and adaptation record

Research was required because wall/monotonic time, authenticated
synchronization, rollback/freeze defense, audit timestamps, and leeway are
security and distributed-systems boundaries:

- Rust [`SystemTime`](https://doc.rust-lang.org/stable/std/time/struct.SystemTime.html)
  established fallible, non-monotonic wall observation; Rust
  [`Instant`](https://doc.rust-lang.org/stable/std/time/struct.Instant.html)
  remains only process-local elapsed measurement.
- [RFC 8633](https://www.rfc-editor.org/rfc/rfc8633.html) retained explicit
  source/correction/monitoring policy; [RFC 8915](https://www.rfc-editor.org/rfc/rfc8915.html)
  retained authenticated exchange and replay protection as a future trust
  upgrade. Neither was represented as present runtime behavior.
- [NIST SP 800-53 AU-8](https://csrc.nist.gov/CSRC/media/Projects/risk-management/800-53%20Downloads/800-53r5/SP_800-53_v5_1-derived-OSCAL.pdf)
  informed named source and consistent audit coordinates.
- [The Update Framework](https://theupdateframework.github.io/specification/latest/)
  retained signed expiring metadata as later distribution rollback/freeze
  defense, not installed lifecycle state or clock correction.
- Microsoft and Apple platform time guidance retained OS diagnosis/correction
  as qualified operational effects. [RFC 7519](https://www.rfc-editor.org/rfc/rfc7519.html)
  informed only small, explicit, bounded leeway—not unlimited rollback or proof
  of correct time.

No upstream crate tree, time protocol implementation, public compatibility
surface, network client, platform command, or hidden runtime was copied.

## Trace, resource, and debugging decisions

`rrflow::clock` traces add bounded boundary, source/trust, status, observation,
anchor, rollback, tolerance, and failure fields. Installation traces retain
their existing monotonic elapsed measurement. No credential, project content,
prompt, arbitrary path, or private model reasoning is logged.

Resource evidence is unchanged project/file inventory around failed apply,
recovery, replay, open, and inspection. The package adds no network access,
durable side store, dependency, background task, OS mutation, or storage hot
path and makes no throughput claim.

Deterministic debugging uses private `FixedClock::at` and
`FixedClock::unavailable` values through production assessment code. There is
no public source injection, environment switch, runtime failpoint, provider
hook, editor hook, or second lifecycle authority.

## First failures and corrections

- The first engine-authority run failed authorization because historical
  fixture timestamps were now behind the real installation anchor. The test
  derives governed effect times from the engine-returned installation time;
  semantic valid-time fixtures remain unchanged.
- Final diff review found that enforcement existed for interrupted recovery and
  exact replay but explicit no-write tests covered only open/inspection. Tests
  were added for both branches before acceptance.
- The same full review found two internal consistency problems before final
  acceptance: installed open assessed time only after constructing the
  persistent engine, and the v2 checkpoint timestamp was not compared with the
  canonical installed anchor. Open now requires the read-only inspection
  preflight before persistent construction, revalidates the same plan/instance/
  anchor afterward, and verification rejects a checkpoint anchor mismatch.
- `cargo fmt --all -- --check` reported only standard Rust layout differences;
  `cargo fmt --all` corrected them and the final check passes.
- The repository-pinned .NET command cannot start on this host: `global.json`
  requires SDK `10.0.111` with roll-forward disabled, while only `10.0.112` is
  installed. The pin was not weakened. As supplemental evidence, the xUnit v3
  executable was run from outside global-file discovery with SDK 10.0.112; all
  four selected client/signal tests passed.
- An earlier run of the existing four-test controller fault-injection suite
  completed in 229.48 seconds and the final run completed in 228.94 seconds. It
  is retained and labeled component fault-injection evidence, not the product
  lifecycle; the bounded primary product test completed in 1.87 seconds.
- The first execution-inventory render failed because the unstaged index still
  named the deleted format-1 asset. The declared delete/create candidate was
  staged, the Git-backed inventory was regenerated from that exact index, and
  its check then passed with 1,156 records. No missing file was recreated and
  no compatibility path was added.
- Final change-plan validation first failed because the plan overdeclared the
  kernel-owned Rust signal catalogue as changed even though configuration only
  changed OpenAPI identity and the signal generator correctly left that file
  byte-identical. No artificial generated diff was introduced. Implementation
  was preserved, the 23-path evidence declaration was corrected in a second
  planning-only commit, and the clean corrective plan passed before the
  implementation was restored.
- Literal command review then found that the .NET filter's `|` was unquoted and
  would be parsed by a shell as a pipeline. The implementation was preserved a
  second time, that command was made literally executable in a third
  planning-only commit, and its clean change-plan gate passed before restore.
- Running that corrected `dotnet test` command from repository root returned
  exit zero after restore but reported no discovered or executed test. The
  complete .NET owner record and project showed why: the tests are an xUnit v3
  executable and the SDK pin is below the project path. A fourth planning-only
  correction replaced the false-pass command with the repository-established
  `dotnet run ... -- -class ...` invocation from `sdks/dotnet`; its clean plan
  gate passed before implementation was restored. The zero-test exit is not
  counted as evidence.

## Acceptance commands and exact results

| Command | Result |
|---|---|
| `python3 scripts/ci/check_change_plan.py` (planning chain) | passed before implementation for the original plan and all three corrective plans; final package accepted as planning-only with zero post-plan paths |
| `cargo test -p rrd-contract --test deployment_configuration_contract --locked` | passed, 5/5 |
| `cargo test -p rrd-contract --test public_contract --test attunement_contract --locked` | passed, 33/33 and 8/8 |
| `cargo test -p rrd-engine installation --locked` | passed, 18/18 installation tests; 68 unrelated library tests filtered |
| `cargo test -p rrd-engine --test engine_authority --locked` | passed, 3/3 |
| `cargo test -p rrflow-cli --test installed_lifecycle --locked` | passed, 4/4; test runtime 1.87 seconds |
| `cargo test -p rrflow-mcp --test stdio --test stdio_daemon --locked` | passed, 3/3 and 2/2 |
| `cargo test -p rrd-server --test local_estate_driver --test http_process --locked` | passed, HTTP 17/17 in 1.58 seconds and controller 4/4 in 228.94 seconds |
| `cargo test -p rrd-client --test real_server --locked` | passed, 3/3 |
| `cargo check -p rrd-client --example sdk_conformance_server --locked` | passed |
| `python3 scripts/ci/check_workflow.py` | passed; 3 workflows, 7 substantive jobs, 5 engine suites, 20 default and 6 optional packages |
| five declared endpoint generators in check mode | passed |
| `python3 scripts/ci/check_generated_surfaces.py --check` | passed; 33 operations, signal SHA-256 `239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07`, OpenAPI SHA-256 `96a232bda6cfb3135e75df86e8b97a77b8a5f098545ce9fe8dc0d3e7a6d6ad26`, seven projections |
| TypeScript, Python, Go, and Java selected SDK suites | passed, 5, 5, all Go packages, and selected Maven tests respectively |
| pinned executable xUnit v3 acceptance command | host-blocked as expected; exited 155 before build/discovery because exact SDK 10.0.111 is absent |
| supplemental xUnit v3 run under installed SDK 10.0.112 | passed, 4/4 selected client/signal tests |
| `cargo fmt --all -- --check` | passed after mechanical formatting |
| `python3 scripts/ci/check_documentation.py` | passed; 270 document statuses, 268 classified coordinates, parent indexes, and local links |
| `python3 scripts/knowledge/render_navigation.py --check` | passed; 14 indexes, 269 nodes, 1,107 edges, zero stale indexes |
| `python3 -m unittest scripts.knowledge.test_navigation` | passed, 6/6 |
| `python3 scripts/ci/build_execution_inventory.py --check` | passed, 1,156 records |
| `python3 scripts/check_version.py` | passed; release train remains 1.0.0 |

## Checks not run

- GitHub-hosted macOS and Windows jobs were not executed locally. Their exact
  primary lifecycle command is now required by workflow policy, but native
  platform execution remains pending CI evidence.
- No OS clock was read through platform synchronization APIs, changed, slewed,
  or stepped; no NTP/NTS server or hostile time environment was used. Those
  require a separately authorized diagnosis/correction package.
- No trusted clean machine, native Windows ACL, macOS Developer ID/notarization,
  Windows Authenticode, signed archive, installer, SBOM attestation, or offline
  acquisition test ran.
- No real Kubernetes API server/cluster, provider, generator, external command,
  DataFusion subprocess, repair, restore, backup publication, uninstall, or
  destructive deletion operation ran.
- Full workspace tests and storage benchmarks were not run because this package
  changes the installed lifecycle/contract boundary, not storage algorithms or
  promotion thresholds. All affected boundary-owning suites were run.

## Remaining known errors and owners

- POAM-006/D-01/D-02/J own trusted platform time diagnosis, explicit
  correction plan/apply and receipts, forward-error/drift evidence,
  runtime-wide high-water semantics, and native cross-platform qualification.
- D-02/D-06/POAM-016 own one portable user identity, a distinct per-machine
  identity, rotation/revocation and capability-scoped secrets across a user's
  Linux, macOS, and Windows project installations.
- D-02 through D-10 own durable attunement execution, explicit authorization
  for commands, network, generators, providers, and DataFusion, and generated
  output re-entry through project inventory/mutation authority.
- D-11/POAM-006 own canonical installed backup resolution, portable encrypted
  backup, repair, restore, ownership-safe uninstall, and a separately gated
  permanent purge requiring typed acknowledgement.
- Gate J owns native filesystem/ACL and real OS-kill qualification, Linux/
  macOS/Windows installers, Developer ID plus notarization, Windows code
  signing, signed offline distributions, clean-machine proof, and exact
  promotion. Kubernetes follows qualified local operation.
- Adaptive reasoning/recall execution and Arrow/DataFusion escalation remain
  with their roadmap gates; the clock configuration is not evidence that those
  engines are complete.

## Full-file reread, checklist, and status

- [x] One measurable D-01 prerequisite, POAM-006 gap, exact source baseline,
  and the four-entry planning-only chain are bound.
- [x] All 59 baseline files have complete machine-bound read coverage.
- [x] Public caller-time injection is removed; private deterministic clocks do
  not escape the engine test boundary.
- [x] Intent, record, configuration, recovery, replay, open, inspection, CLI,
  workflow, fixtures, OpenAPI, and SDK projections converge on one owner.
- [x] Unknown/unavailable/out-of-policy time fails governed effects/open while
  read-only diagnosis remains available and non-mutating.
- [x] Research, trace, resource, debugging, first-failure, non-run, and
  remaining-gap decisions are explicit without a schema of thought.
- [x] The old format-1 asset is removed directly; no fallback lane exists.
- [x] Product version, protocol version, Cargo dependencies/lock, roadmap
  checkboxes, POA&M lifecycle state, release state, and official promotion are
  unchanged.

Roadmap/POA&M status change: **none**. D-01 and OBJ-07 remain partial;
POAM-006, trusted/native time, attunement, maintenance, identity, external
capabilities, Kubernetes, Gate J, release, and promotion remain open.

At journal authoring time, implementation and evidence are a reviewed worktree
delta after final planning commit
`c6f262e8b8f5622ad2a91909970409abf0030937`.
No implementation commit or push has occurred. The exact result revision and
private-development push receipt are reported in the handoff; neither Git nor
this journal can change roadmap status, and official promotion still requires
every Gate J prerequisite plus explicit authorization of that exact revision.
