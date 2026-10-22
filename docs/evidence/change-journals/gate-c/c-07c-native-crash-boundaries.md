# C-07c deterministic native process boundaries

**Status:** active bounded implementation evidence; exact native candidate CI required
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07c-native-crash-boundaries`
**Owner:** OBJ-02 prerequisite, [C-07c](../../../roadmap/rrflow-1.0/gate-c-07.md)

## Brief and baseline

[PR 59](https://github.com/rrflow/rrflow-development/pull/59) carries the
pre-implementation brief. Public main started at
`51c5f7a6a20e1751383b19dc1af545e34bdabf94`, tree
`216b00db20010fc5f28d6700169dd405e6e0291b`; the
[baseline journal](c-07c-native-crash-baseline.md) opened review before code.
The seed's CI was canceled because it contained no implementation; it is not
acceptance. There were no unrelated worktree changes.

Current-to-target: injected native-call errors and the stable-byte model did
not prove a real child was killed at an exact internal boundary. Extend that
existing unit-test boundary with an owned one-use rendezvous, native death,
compound reopen and repeated exact-history checks. Production formats,
runtime/API/default policy, version and source authority remain unchanged.
No code is deleted or moved. Stop on unreached schedules, lost acknowledged
values, partial pairs, wrong sequences, leaked children, unbounded capture or
required relaxed limits. Full installed qualification is not inferred.

## Implementation and review

The existing `fault_io::Ticket` now observes before/after native calls for the
new `fault_io::crash` controller. `database.rs` adds one `cfg(test)` visibility
observation after the memtable's atomic write. The entire controller remains
inside the already unit-test-only module: no Cargo feature, environment,
prompt, public header or product configuration enables it.

The parent creates a unique fixture beside its test artifact, freezes requested
pair values, and owns each child handle and private pipes. A closed, bounded
stdin schedule binds canonical fixture root, random 256-bit nonce, operation
identity and exact point. The child emits one bounded nonce/PID/point receipt
and pauses under a deadline. The parent validates identity before kill/reap.
Missing, changed or unreached schedules cannot produce accepted kill evidence.
Guard cleanup kills/reaps only that child and joins both bounded capture
workers on failure. No process-name lookup or external endpoint is involved.

WAL append is observed immediately before the durability call, after the
complete record write; it is not mislabeled from a potentially partial native
write. CURRENT rename and reclamation are observed before and after their
native effect, before the following directory barrier. Recovery termination
occurs after the first WAL payload read, before its frame is applied.
Two additional recovery kills per boundary establish repeatable interrupted
reopen. The model's volatile-byte loss remains a separate proof.

Forty cases cover ten points, borrowed/owned writes and WAL-only versus
published-segment fixtures. Eighty compound recovery kills preserve those
histories. Both fixture layouts also execute 100 acknowledged write/kill/reopen
cycles, checking every predeclared value and the exact sequence after every
cycle. The pre-WAL case requires absence; all completed-frame native process
cases require exact full presence. The OS remains running, so this does not
promise that unsynchronized bytes survive power loss. Test receipts identify
the unit-test executable SHA-256, OS, boundaries, nonce, PID and hit/reap time.

Read the README, alpha objective, C-07, POAM-035, dependency spine, authoring
routine, installed lifecycle/history and native CI binding. Reviewed
`fault_io.rs`, its existing model/test use, native I/O wrappers and filesystem
effect ordering, `database.rs`, and both new crash files. The production file's
only edit is the test-only visibility observation. The full changed-file/diff
review and generated inventory reconcile these paths; no parallel catalogue,
benchmark authority or compatibility implementation is added.

Research: existing C-07 process-versus-power-loss semantics and native code
inspection suffice; no upstream implementation is adapted. Earlier vendor
knowledge-path research did not justify this storage change. Diagnostics are
bounded test receipts; production tracing and allocator policy are unchanged.

## Commands, results and limitations

- Unchanged baseline: `cargo test -p rrd-lsm --lib fault_io --locked --
  --nocapture` passed ten tests in 63.19 seconds. Windows exercised 127
  modeled calls, 314 fault/loss cases and 7,452 compound recovery cases.
- First attempted exact filter omitted the qualified module name and ran
  zero cases. This is not acceptance. With the full test name,
  `cargo test -p rrd-lsm --lib
  fault_io::crash::tests::native_pre_wal_kill_preserves_ack_and_denies_unwritten_pair
  --locked -- --exact` failed before wiring: `native boundary was not reached`,
  zero passed/one failed in 0.02 seconds. Unused-controller warnings in that
  deliberately unwired intermediate build disappeared after wiring.
- First focused wired corpus: four passed, one explicitly invoked child
  fixture ignored by the ordinary parent runner, 7.22 seconds.
- Expanded full `cargo test -p rrd-lsm --locked`: 131 tests passed, no failures,
  one explicitly invoked child fixture. The unit suite passed 26 cases in
  42.32 seconds. Windows's Linux-only `snapshot_memory` target ran zero cases;
  this does not establish native memory bounds.
- `cargo clippy -p rrd-lsm --all-targets --locked -- -D warnings` passed.
- Final focused `cargo test -p rrd-lsm --lib fault_io::crash --locked --
  --nocapture` passed four parent tests in 4.11 seconds and retained the three
  native receipts, including both 100-cycle layouts. The child fixture is
  invoked 322 times across positive and negative parent cases, not skipped
  coverage. Logs are `target/c07c-native-final.log` and
  `target/c07c-native-full.log` locally; candidate CI retains native output.

- `cargo build -p rrflow-cli --bin rrflow --locked` passed in 37.08 seconds.
  A bounded streaming scan confirmed the native pause protocol marker is absent
  from the 156,127,744-byte ordinary executable; its SHA-256 was
  `453c98adf3a79b2c2d0ed35f2daf4a8666ad0161d07b9b460f704144690a08c5`.
  The instrumented test executable was
  `ce354e7a62f535f0d11e7155289a1ab49b644e1a551a7f2535752519d7769374`.
  This byte observation supports the `cfg(test)` exclusion; it is not signed
  release-build qualification.
- The retained three receipts independently validated 320 unique run nonces,
  the 40/80 boundary/recovery counts, two distinct 100-cycle layouts, exact
  sequences and monotonic hit/reap times within 30 seconds. The two additional
  positive/negative harness child invocations are not counted as corpus hits.
- `cargo fmt --all --check`, `git diff --check`, documentation ownership,
  navigation regeneration/check and execution inventory regeneration/check
  passed. Counts: 395 document statuses, 393 classified coordinates, 17
  indexes, 394 nodes, 1,722 edges and 1,379 inventory records. Projections are
  regenerated after this final journal edit before commit.

Exact-candidate CI must pass on Linux, macOS and Windows; local Windows
results alone do not qualify those hosts. The PR carries the candidate run
and landing evidence rather than rewriting this immutable journal later.

Not run or implemented here: joined installed internal-boundary controller,
installed 100-cycle operating profiles, hard resource/queue admission,
snapshot/retention accounting, real native capacity exhaustion, larger-than-
memory profiles and 24-hour soak. Existing installed response-loss proof is
retained, but cannot substitute for these cases. No new production deficiency
was reproduced. POAM-035, C-07c-h and alpha remain open. DuckDB execution and
knowledge-path activation stay at their existing dependent gates.
