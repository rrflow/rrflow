# C-07g: resource observations across overlapping offered windows

**Status:** active evidence record; implementation under qualification
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-overlapping-resource-windows`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [81](https://github.com/rrflow/rrflow-development/pull/81)

## Baseline and change brief

Starting revision `b0e5c1e57e798b5757d30d4bd3cd7460f11b6b43` integrates
PR 78 and its dependencies over development/main. The PR brief preceded source
edits and declares integration of PR 80 before native reruns. Version remains
1.0.0. No gate or objective status changes.

The installed driver partitions offered requests into bounded ledgers. A late
preceding segment can continue executing and sampling after the next segment's
scheduled start. Previously, the next ledger discarded those actual overlapping
observations. It now carries only observations inside its offered window, with
the same owned PID and unchanged bytes/timestamps. Existing progress records
attribute those observations to the overlapping segment as well. One previous
admitted segment is retained, with room reserved for new first/final samples
inside the frozen count cap. The independent validator still enforces gaps,
allocation identity, process identity and all latency/error thresholds.

No clock origin, request timing, throughput, outcome, sample acquisition,
storage format or engine behavior changes. No background sampler or unbounded
history is introduced. A real unsampled interval remains a failure.

## Actual failure and traceability

Windows campaign13 at `d2882fb31625366fe115b1f4f1937ffce5422f1d` failed
after 255 completed segments and approximately 42.86 minutes. Cleanup was
confirmed. The campaign requested 24 hours but did not complete them.
It used four CPUs, 4 GiB, four clients, one request/second and the small
32-record/96-edge/64-vector characterization corpus.

| Observation | Campaign-relative nanoseconds |
|---|---:|
| Segment 255 scheduled start | 2560409933400 |
| Prior segment actual sample | 2561830172500 |
| Prior segment actual sample | 2562966803500 |
| Prior segment actual finish | 2562967502500 |
| Segment 255 first newly acquired sample | 2563149587600 |

Omitting the prior samples manufactured a 2739.6542 ms initial gap against the
frozen 2000 ms bound. The same evidence also records late and rejected work;
this repair does not make that operating point sustainable or rewrite the
failed result. Retained campaign identity:
`06ff105f0d3c280aed7801319abc2e27c0f5cc4ff132fa9be4ceb9aa2ff1c4e8`.
Failed segment digest:
`b1e5e6b39eea4d1adc2a4091b8d42e4e8b8a708fec8b11dcabf2ea11b63f6544`.
Local artifacts remain under `target/c07-windows-characterization-13/evidence`;
the failure summary is `target/c07-failed-native-campaigns-20261023.json`.

Read README and relevant C-07/POAM-035 owners, change-authoring routine,
`support/sustained.rs` sample/segment/phase orchestration, `sustained/native.rs`,
evaluation `storage/sustained.rs` admission/append/validation, and
`storage/progress.rs` recording/verification. Inspected actual adjacent segment
timestamps and attempts, without dumping credentials or user payloads. Read
the entire new `sustained/resource_window.rs` helper and regressions and reviewed
the driver diff. The helper remains inside installed qualification tooling;
`RrdEngine` and the evidence validator retain their existing authorities.

Research decision: source-grounded window ownership repair; no new external
algorithm, dependency or research claim. Debug evidence retains bounded sample
values, timestamps and process identities. Stop conditions: fabricated samples,
relaxed bounds, reset clocks, concealed overload, or unsupported completion.

## Verification and remaining acceptance

The regression uses the exact retained timestamp boundary above and verifies
byte-identical carry, PID mismatch denial, frozen sample admission and absence
of invented observations for a genuinely uncovered interval. It does not
substitute synthetic resource values for native qualification.

`cargo test --release --locked -p rrflow-cli --test installed_reopen resource_window --jobs 6`
built the native release harness in 7m13s and reported both regressions passed.
Log: `target/c07g-overlap-focused.log`, dedicated build directory
`target/c07g-overlap-build`. The PowerShell redirection wrapper returned 1
despite the Cargo pass summary; the combined rerun will capture the subprocess
exit directly. Format/diff and generated navigation/inventory/documentation
checks pass. Combined prerequisite integration, CLI lint and native execution
are still pending at this implementation commit.

Candidate CI, a fresh native installed run, full corpus profiles and a completed
24-hour soak remain required. Prior failed artifacts are immutable. C-07,
OBJ-02 and POAM-035 remain open.

## Change checklist

- Bound baseline, outcome, exact scope, oracle and stop conditions in the PR
  before implementation.
- Mapped existing sample acquisition, offered windows and independent
  validation; replaced only lost association across an overlapping boundary.
- Kept retention/count/time and allocation contracts explicit, without a new
  engine authority or external prerequisite.
- Recorded the real failure, trace/resource/research decisions, file review and
  unrun acceptance. Linked the POAM follow-up and regenerate nearest navigation
  and execution inventory before commit.
- Ordinary exact-candidate CI and PR merge remain mandatory; no promotion or
  alpha completion is implied.
