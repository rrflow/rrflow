# C-07g: reduce canonical digest CPU cost

**Status:** active implementation evidence; full capacity and soak qualification remain open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-canonical-digest-throughput`
**Owner:** OBJ-02 / C-07g / J-04 / POAM-035 / PR117

## Brief, baseline and evidence-led scope

[PR117](https://github.com/rrflow/rrflow-development/pull/117) binds this change
before implementation. Starting public main is
`0273f11bc9784681f9532ca06d912b7f0e11ddb7`, tree
`aa82d8c40a28e4b9911d59117704b511f1272cd7`. Empty opening
`135b3e262d3bf562793d3afeaa868b296eba105e` creates the brief-first PR without
another planning file. Its skip-CI marker avoids an empty native run; the actual
implemented candidate requires the complete candidate CI before landing.
Product version remains 1.0.0.

An optimized installed-engine profile identified a real CPU hot path:
`rrd_core::digest::compress` accounted for 24.14% of 348 sampled `cpu-clock:u`
events, with zero lost samples. This was a diagnostic-only 1,000-record Linux
run at source `469a4a568139f1d1a17a65225e2ce2da41505dbc`; ordinary engine SHA-256
`6207b0eecb222b418e8a27844678d3a1800e75f8bff0064dcae3672d7499f50c`.
Perf 7.0.14 sampled owned ready engine processes at 49 Hz. Its process/artifacts
were bounded and outside the qualification allocation, so the run cannot
establish throughput, memory or capacity acceptance. Local capture:
`/home/devcontainers/rrflow-c07-native-79663cdd/p2-5rps-growth-1000-469a4a56/perf-2490401.data`.
Only symbol/count summaries are used as evidence; raw process capture stays local.

The preceding unprofiled 25-RPS characterization completed but rejected 207
scheduled operations (198 driver slots, nine storage conflicts). A separately
frozen larger-segment configuration rejected 111 (107 driver slots, four
conflicts). Segment sizing changes working-slot histories too, so this is not
a controlled engine speedup comparison. One measured backup exceeded its
8-second offered segment and delayed the next offers by up to 2.866 seconds;
that driver barrier does not explain every rejection. All failed points remain
retained under operator-local `target/c07-pr116-native-evidence`.

At the roadmap's separate 5-RPS rung, all 300 offered preflight operations
acknowledged with no latency overrun or unresolved effect, but the first short
endurance segment failed a 2.230-second resource-sampling interval. Counters
showed automatic flush/compaction around the interval, not proof of causation.
The profiled repeat acknowledged 600 offers with one latency overrun, then
failed restart startup before full final readback. An earlier profiler attempt
also failed startup. Neither is a passing capacity result. No 24-hour soak ran.
These findings motivate a measured primitive optimization while preserving
transaction, verification, scheduling and resource thresholds.

## Canonical implementation and traceability

`rrd-core/src/digest.rs` remains the sole kernel byte-identity interface. Replace
its scalar SHA-256 compression/padding implementation with the already-locked
pure-Rust `sha2 = 0.11.0` primitive, with default features disabled at the kernel
edge. No upstream crate tree or semantic model is copied. Cargo.lock gains only
the kernel dependency edge; no third-party version changes. The kernel still
compiles independently of every outer RRFlow layer and has no storage,
transport, platform service, tracing or tier-policy dependency.

The public `Sha256` wrapper retains const construction, update/finalize and
finalize_hex, checked input-length admission, cloning, one-shot helpers and
return types. A private optional state initializes lazily to preserve const
construction without exposing upstream types. Default CPU feature detection
selects supported instructions and retains the portable backend. Remove the
old scalar compression directly; there is no second runtime implementation,
format, hash algorithm, receipt authority or compatibility lane.

Traceability before replacement: the original state/buffering/compress helpers
map to this same wrapper over the pinned primitive. Existing published vectors
and chunk fixtures remain; independently fixed vectors cover padding lengths
0/1/55/56/63/64/65/127/128/129/4097, the million-a vector, empty updates and
cloned divergent streams. Existing authenticated-log, public-contract,
WAL/segment/manifest and evaluation goldens must remain unchanged. Stop on any
digest drift, format/API/transaction change, unsupported forced instruction or
new outward dependency. Unchanged digest bytes are necessary, not full engine
qualification.

Files read: README, AGENTS, SPEC, alpha objective, C-07/J-04, POAM-035,
change-authoring and independent-engine/source-adaptation guidance; complete
kernel digest and lib boundary, root/kernel manifests and locked dependency;
pinned sha2 0.11.0 API/backend source and manifest; affected digest fixtures,
contract/LSM/evaluation consumers and native receipts. For attribution, read
installed sustained sampling, workload execute/verify, segment scheduling,
reader/backup handling, resource validation and native compaction/page-cache
paths. Those latter paths are unchanged by this package.

Research: [RustCrypto sha2 0.11.0](https://docs.rs/sha2/0.11.0/sha2/)
defines its pure-Rust API, runtime x86/AArch64 acceleration and portable fallback.
MIT/Apache-2.0 licensing and the existing locked checksum are retained. The
useful behavior is the cryptographic primitive only, mapped to the existing
kernel digest boundary under C-07g/J-04. It supplies no engine architecture,
state, mutation authority or deployment dependency. No new runtime trace is
added; existing diagnostic captures locate the cost and unchanged receipts
check identity.

## First oracle and bounded measurement

Before replacement, the new independent fixtures passed against the original
implementation: three focused tests, one ignored manual benchmark. The exact
bounded command was
`cargo test --release --locked -p rrd-core --lib digest::tests::bounded_sha256_throughput --jobs 6 -- --exact --ignored --nocapture`.
A fixed 32-MiB 0xa5 input was hashed 16 times (536,870,912 bytes), with its digest
independently generated using Python hashlib and asserted on every iteration.
Original elapsed time: 1,269,274,600 ns. Replacement: 123,072,400 ns, about 10.3x
for this primitive measurement. Windows x86_64, Intel Core Ultra 9 275HX,
rustc 1.99.1, release profile. This single paired micro-measurement is not a
latency distribution, engine-wide speedup or capacity acceptance. Logs are
operator-local `target/c07-digest-baseline.log` and `target/c07-digest-candidate.log`.

The replacement focused tests passed four cases, including explicit input-length
overflow rejection; the benchmark stays ignored in ordinary tests. The complete
kernel suite passed 67 tests, zero failures, with one manual benchmark ignored.
A separate release build with `RUSTFLAGS='--cfg sha2_backend="soft"'` passed all
four focused digest tests, proving the portable path against the same vectors.
Its target directory is isolated; no machine-wide build flags changed.
Log: `target/c07-digest-portable.log`.

Changed paths are the kernel digest/tests, kernel dependency and boundary
comment, root dependency comment, Cargo.lock edge, this journal, the existing
C-07/POAM-035 links and generated navigation/inventory. No golden files or
storage schema are regenerated to conceal a difference. Full affected-package,
installed integration, lint, documentation and candidate-CI results are recorded
below as they complete. Full scale/soak and comparative quality/token savings
remain unrun for this candidate; C-07/POAM-035 stay open.


### Owning suites and source integration decision

`cargo test --locked -p rrd-contract -p rrd-lsm -p rrflow-eval --jobs 6`
passed 372 tests with zero failures. Two explicit native child-helper entries
remain ignored at top level and execute through their parent tests. This
includes existing contract/schema goldens, authenticated storage, native fault
and 100-cycle unit histories, and evaluation receipts. No golden changed.
`cargo clippy --locked -p rrd-core -p rrd-lsm -p rrd-contract -p rrflow-eval --all-targets --jobs 6 -- -D warnings`
passed, as did `cargo fmt --all -- --check` and whitespace validation.
Operator-local log: `target/c07-digest-validation.log`.

The pre-edit brief's declared PR116 prerequisite integration was clarified
before the merge effect: bring exact `469a4a568139f1d1a17a65225e2ce2da41505dbc`
into this candidate normally so the final installed comparison retains its
pre-soak admission and verification timing. Its native CI is still pending;
source integration does not accept that prerequisite or permit this PR to land
ahead of its required CI. No force push or history rewrite is used.

Native profiling required the Ubuntu operator packages `linux-tools-generic`
and `linux-perf` (perf reports 7.0.14). They are development observation tools,
not linked RRFlow build/runtime dependencies. No host perf permission policy
was relaxed. The first capture failed ordinary startup and is retained; the
second captured the stated samples but failed final restart. These diagnostic
failures are explicit limitations, not passing lifecycle evidence.


The ordinary installed command
`cargo test --locked -p rrflow-cli --test installed_reopen --jobs 6 -- sustained::installed_capacity_workload_smoke --exact --ignored --nocapture`
passed in 9.44 seconds on the initial digest candidate. It exercised eight
records, 24 relations, 16 vectors, all four write transitions, backup under
writes, retained-reader release, actual termination/reopen and 12 replayed
transactions. Retained projected bytes drained from 225 to zero. The receipt
states `capacity_qualification=false`; executable digest is
`58dd4b792086bb1c5ad65d7f5179cb41f695d72db4600c8086dba0cc5db328f6`.
The earlier admission candidate's identical debug smoke took 77.02 seconds;
this is an observed development-test improvement, not a release SLA comparison.

Exact prerequisite `469a4a568139f1d1a17a65225e2ce2da41505dbc` was integrated by
normal merge `6ae7ae435322f69138a9ad0ab33174b85c645b8d` after local source checks
finished. The digest edits remained unstaged and separate from that merge.
The integrated evaluation and installed smoke are rerun because the harness
changed; the already-passing storage suite is not rerun without a new storage
change. Prerequisite native CI and this candidate's own CI still gate landing.


### Integrated acceptance and change-authoring checklist

After prerequisite integration, `cargo test --locked -p rrflow-eval --jobs 6`
passed all 48 evaluation tests; the installed smoke command above passed again
in 8.88 seconds with the same ordinary executable digest and all flow assertions.
Log: `target/c07-digest-integrated.log`. This justified repeat tests the changed
harness; it is not another capacity claim.

- [x] Read owning objective, roadmap, deficiency and actual implementation before editing.
- [x] Put the bounded brief and replacement map in PR117 before implementation.
- [x] Attribute the CPU cost with a retained diagnostic profile and primary-source research.
- [x] Keep one kernel interface, unchanged identities, transaction authority and budgets.
- [x] Exercise independent vectors first, portable backend, owning suites and installed flow.
- [x] Review the complete authored source diff; lockfile changes only one dependency edge.
- [x] Preserve failed native histories, known limits and unrun qualification explicitly.
- [ ] Complete candidate CI and native sustained acceptance before claiming those results.

No full-corpus, 24-hour soak, comparative model-quality/token saving, or UI
accounting implementation is claimed. Existing C-07 and POAM-035 remain open;
the accounting plan remains owned by the existing data-flow/H-05/G-06 records.

Documentation policy, generated navigation check, generated execution inventory (1,592 records), final formatting and staged whitespace checks passed. Candidate CI and optimized Linux sustained comparison are pending at commit; no roadmap completion status changed.


### CI-exposed prerequisite cleanup integration

PR116 source 469a4a56 failed macOS job 120935676134 after five passing native
capacity cases, during owned-image cleanup (exit 16). Its repair is separately
recorded in [the cleanup journal](c-07f-owned-image-detachment.md). The PR117
brief was extended before integrating exact prerequisite repair
`88fbc5b41128371eabff21928b48d84255372e90` normally. Resolve the POAM append
conflict by retaining both observations; regenerate the mechanical inventory.
No runtime code differs from digest candidate d512b932 because of this cleanup
integration. Its ongoing optimized Linux characterization keeps its original
source identity. Both candidates still require native CI before landing.

The integrated 13-check cleanup suite passed in 0.056 seconds. Documentation caught an encoding error during conflict resolution; restored the original UTF-8 heading from Git and retained both evidence additions before regenerating indexes.
