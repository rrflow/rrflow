# C-07g: resource sampling on scheduled deadlines

**Status:** active evidence record; unchanged native comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-sample-cadence`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [92](https://github.com/rrflow/rrflow-development/pull/92)

Starting revision `4c33186b1c983254af321180f67c293abc5e50ca` integrates PR91
from fetched main. The pre-edit PR brief binds the replacement, oracle,
scope, research/resource decisions and stop conditions. The sustained native
harness previously waited a fresh sampling interval after each collection.
Collection time therefore accumulated into the gaps. It now advances from the
previous scheduled deadline. An overrun permits one immediate acquisition and
resets the cadence without replaying missed ticks. Already-due offered work
precedes sampling in the biased scheduling select; ready client completions
retain priority. No additional tasks or queues are created.

The replacement covers drain, scheduled workload, client/backup completion
waits and offered-window tails in
`crates/adapters/rrflow-cli/tests/support/sustained.rs`. The private helper and
its regression belong to `support/sustained/resource_window.rs`. Initial
sampling resumes from the actual last observation, including time spent
opening a held reader. Real observation timestamps, overlap retention, PID
validation, sample caps, segment deadlines and the independent maximum-gap
validator remain unchanged. Slow acquisition can still fail coverage; this
does not claim to repair every host stall.

## Inspection, oracle and checks

Read the README/objective/C-07/POAM owners and authoring/traceability procedure;
the complete changed sampler, drain and segment functions, overlap helper,
native memory acquisition, independent resource-series validation and native
failure receipts. Rechecked both changed source files and the complete diff.
No external implementation was adapted; the local failure and existing timer
contract supply the research basis. Preserve true observations and failure
identities; do not change interval, thresholds, corpus, allocation or engine
behavior. Stop on fabricated samples, unbounded catch-up, offered-work
starvation or broader authority changes.

The deterministic regression ran first against completion-relative scheduling
and failed: an 89 ms collection delayed the next deadline by 89 ms. After the
repair, 100 such collections stay on the declared grid; one overrun resets
without an accumulated backlog. Zero periods and deadline overflow fail.
Existing overlap/PID/sample-cap and genuinely missing-observation tests remain.

Commands and results:

- `cargo test --locked -p rrflow-cli --test installed_reopen resource_window -- --nocapture`:
  red 2 passed/1 failed, then green 3 passed.
- `cargo test --locked -p rrflow-cli --test installed_reopen -- --nocapture`:
  5 passed, 7 explicitly ignored native campaigns. Those ignored histories are
  not claimed as executed by this command.
- `cargo test --locked -p rrflow-eval storage::sustained`: 15 passed.
- `cargo clippy --locked -p rrflow-cli --tests -- -D warnings`: passed.
- Formatting, documentation, generated navigation/inventory and diff checks
  passed before commit; exact candidate CI and new native soak remain
  separate acceptance work. Raw local logs: `target/c07g-sample-cadence-*`.

## Retained native outcomes

Windows15 source `fcbd769049f49e1eb219e084ba3084bd571c0ce0`, binary
`d7221d55e83e5bba1c321290e9d0830724ebe5c82eeaacc1db9133791a532a7b`, campaign
`dc8a17dcda2a2bcac14c9123f9221f86075a30d6596834a2e7e6e8abd2b21783`, harness
`e86f045a1ad2f12272c8ed81e2298609f143fe99d52c6622d4af698ddf31ecd0`
failed after 259 segments and 2611000 ms with a real 2089433500 ns sample gap
under the unchanged 2000000000 ns limit. The samples belong to PID 54292.
Cleanup is confirmed. Failed segment digest:
`8f406bbbe4f07b6242905b337339f80f26572f66a39222813089f763d8439a39`.
This is failed soak evidence, not a 24-hour pass.

Linux10 source `84a283819a60433c76340c90f1985e47fce48db6`, campaign
`76a90a9c33507a3a4c7c610e5e667ee8ae848803d6b967ce04783b78e6b0bfa7`, harness
`a715a26a42e4c614e866dd5e77ae4826f9eeb947f88955fedbc29e37b0b9785c`
completed 1,000 corpus acknowledgements/readbacks and two reopens. Backup
creation plus verification completed in 29.617 seconds within its unchanged
30-second deadline; application backup digest
`7b5e6f7a6cc3a6bf6bc8b301a1c73cbf22b3ea51d2a199b6be8049ba2e084182`.
The whole campaign still failed after ten segments/487634 ms because drain
did not recover maintenance debt: baseline 42, final 43, required maximum zero.
All final transaction/view/pin/queue/active/fence owners were zero. Measured
load also had 13 unsuccessful attempts and 25 latency violations. Cleanup is
confirmed. This establishes further C-07d/g work, not capacity qualification.
Receipts remain in `target/c07-linux-scale-evidence-10`; Windows receipts in
`target/c07-windows-characterization-15/evidence`.

Checklist: pre-edit brief, mapped replacement, failure oracle, implementation,
focused/owning checks, failure retention, diff reread, journal and projections
are recorded. Full profiles, 24-hour soak, macOS missing native-I/O receipt,
maintenance progress and exact candidate CI remain open. No alpha, roadmap
completion, POAM closure or promotion status changes.
