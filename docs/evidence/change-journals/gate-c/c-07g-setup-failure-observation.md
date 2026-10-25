# C-07g: retain diagnostics at corpus failure

**Status:** active evidence record; installed observation and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-setup-failure-observation`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [99](https://github.com/rrflow/rrflow-development/pull/99)

Baseline `400c510bde969b5f95308298e067d38557ad9798`, tree
`ac574d217f7e7c2125c303a5f406832b72a23c7c`, combines fetched main with PR98.
The pre-edit brief binds a diagnostic-only change. Linux16 failed the same
hard L0 admission after the foreground repair; the setup path propagated its
error before retaining live storage diagnostics. Counting its progress events
confirmed zero resource observations. Another policy change without that
observation would not establish the cause.

## Change and review

In `rrflow-cli/tests/support/sustained.rs`, corpus failure now attempts exactly
one existing authenticated `sample` before owned cleanup. A five-second async
timeout bounds the awaited diagnostic read; it does not claim to interrupt a
blocking operating-system call. Successful observations use the existing
`ProgressEvent::Resource` and admitted progress checkpoint. The supplemental
event is not an accepted workload segment or a qualification result.

Capture, timeout and artifact failures emit only a closed status label and
cannot replace the original setup error. No retry, engine mutation, raw
credential output, new schema, authority, workload or relaxed threshold is
introduced. Read README/OBJ-02/C-07/POAM-035 and authoring procedure; complete
setup-to-driver transition, corpus seeding, resource sampling and progress
ownership call sites. Reviewed the full changed block and diff. Supporting
changes are this linked record, POAM-035 and generated navigation/inventory.

Research: the existing public diagnostics contract supplies the needed
observation; no external research or upstream adaptation. Preserve actual
timestamps and allocation ownership. Stop if diagnostics could qualify a
failed campaign, hide its rejection, exceed artifact admission or change normal
load execution.

## Local validation and limits

- `cargo check -p rrflow-cli --test installed_reopen --locked`: passes,
  41.67 seconds. This proves compilation, not observation success.
- `cargo clippy -p rrflow-cli --test installed_reopen --locked -- -D warnings`:
  passes, 21.62 seconds.
- `cargo fmt -p rrflow-cli -- --check`, documentation, navigation, execution
  inventory and diff checks pass.
- Existing resource-window tests and installed failed-setup characterization
  will run against the frozen native candidate. They have not passed for this
  source in this record. Storage tests are not repeated for unchanged engine
  code; PR98 records its 441 storage/seven admission passes and native failure.

Raw local validation logs use `target/c07g-setup-observation-*`. The smallest
installed oracle must retain a diagnostic event or explicit bounded-unavailable
outcome, preserve the setup failure and confirm owned cleanup. No successful
capture, root cause, capacity result or full-platform qualification is asserted.

## Prior native/build failures retained

Linux16 source `55886b1662bb6ebf6d8b06aa757d561c222cdcc1`, executable
`6ed9867ae1184dfee320f82fe06f103e9b652ec9cb914e48a6fde961dc64ced4`, campaign
`d7c7d5faf5d8b7de5a76947cb0838190a28db8236371bbdfc0d7bced56e895ed`, failed
after acknowledged ordinal 376 with the closed resource label
`api-resourceexhausted-maintenance-l0-segments`. Elapsed 46721 ms, zero accepted
segments, cleanup confirmed. Progress digest is
`9efeb706043195e1af3515b6e62da424eeea2e5bd43e5b9100d1e65ba7dbdf7d`;
raw receipts are `target/c07-linux-scale-evidence-16` and the full16 log.
PR98 returned to draft; local correctness passes are not native scale acceptance.

The Windows20 release build for PR98 exited 1 with no compiler/linker diagnostic
in `target/c07-windows20-build.log`. One same-source verbose retry with three
build workers was started; its result remains pending here. No cause is
attributed without evidence, and no Windows20 soak had started at this record.
Linux PR98 release build and its three existing resource-window tests passed,
but that did not prevent the installed failure above.

Checklist: pre-edit brief, owner and scope, source review, initial missing-event
oracle, compile/lint checks and linked journal are recorded. Installed capture,
causal diagnosis, candidate CI, full capacity profiles, macOS failures, 24-hour
soak and reasoning-runner budget enforcement remain open. No roadmap, objective
or POAM-035 closure.
