# C-07g: installed failure observation result

**Status:** active evidence record; diagnostic oracle passed, capacity campaign failed
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-setup-failure-observation-results`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [99](https://github.com/rrflow/rrflow-development/pull/99)

This supplements the immutable [implementation journal](c-07g-setup-failure-observation.md).
Starting revision `2de723012bca56b12c932aca9885f723cbc03310`, tree
`525fa87c4b603f3ca95f260552dfe14af1dbb7d0`. The updated PR brief binds this
evidence-only record and generated navigation/inventory. No engine behavior,
limits, acceptance thresholds or previous journal changes.

## Installed observation

Linux17 ran the same bounded characterization against that exact source.
Executable SHA-256:
`6ed9867ae1184dfee320f82fe06f103e9b652ec9cb914e48a6fde961dc64ced4`.
Campaign SHA-256:
`a912dbc33fcf2ce7b119d095f61ae50c4e75b30345f755a7660328d44ad7a094`.
The campaign failed after 54,210 ms, with zero accepted segments and confirmed
owned cleanup. Last acknowledged corpus ordinal was 376. The original
`api-resourceexhausted-maintenance-l0-segments` failure remains the outcome.

Exactly one supplemental resource event was retained at elapsed
53,541,891,306 ns. Progress has 764 records, 169,577 bytes and SHA-256
`1a763e12d0d00c7aace5d68a7dc6fd18067bf32c97559959ca86d42175f1ce5f`.
The event contains public storage diagnostics rather than a fabricated workload
segment. It did not change `qualification_passed: false`.

| Observed field | Value |
|---|---:|
| L0 segments / configured hard cap | 57 / 64 |
| Compaction debt / all segments | 50 / 180 |
| Automatic flushes / failed flushes | 24 / 2 |
| Automatic compactions / failed compactions | 56 / 0 |
| WAL payload bytes / configured maximum | 65,400,672 / 67,108,864 |
| Mutable approximate bytes | 65,413,136 |
| Active native transactions / read views / outstanding requests | 0 / 0 / 0 |
| Resident bytes | 306,159,616 |

This shows that maintenance executed while L0 capacity prevented the next
partitioned flush. It does not establish the complete cause of debt growth or
justify raising the hard cap. Per-level selection and write amplification need
further evidence. The diagnostic oracle passed; the workload did not.

## Commands and scope of evidence

- Linux release `cargo test --release --locked -p rrflow-cli --test
  installed_reopen resource_window`: passed, three tests, build 1m34s.
- `wsl -d Ubuntu -- python3
  /mnt/d/localdev/devlabs/rrflow/target/collect_c07_linux_run.py 17`: exit 0;
  collected failed campaign receipts only after asserting cleanup. This
  collector success is not a campaign pass.
- PowerShell parsed every progress JSON record and selected the resource
  event. Raw artifacts remain in `target/c07-linux-scale-evidence-17` and the
  corresponding full17 log; the launch receipt records cgroup v2 enforcement
  of four CPUs and 2 GiB.
- Windows PR98 revision `55886b1662bb6ebf6d8b06aa757d561c222cdcc1` retry:
  `cargo test --release --locked -p rrflow-cli --test installed_reopen
  resource_window --jobs 3 --verbose`, exit 0, 787.59 seconds, three tests
  passed. Original unexplained build failure remains recorded in the prior
  journal. No Windows20 campaign or 24-hour soak was started.

Reviewed README, AGENTS, authoring procedure, inference contract, Gate G,
previous journal and PR brief; inspected the native launch/progress/result
artifacts and build log. Read the complete changed record and generated diff.
Research/source adaptation: none, since this package reports existing native
measurements. Trace decision: retain actual timestamps, closed failure and
allocation identity. No additional tests for an evidence-only change;
navigation, inventory, documentation and diff checks pass. The initial
documentation check rejected the missing active/historical status marker;
the marker was corrected before rerunning the check.

Checklist: pre-edit PR scope, starting identity, exact source/campaign,
falsifying outcome, cleanup, unchanged thresholds, prior failure preservation,
linked evidence and generated navigation are recorded. Candidate CI, macOS
failures, full capacity profiles, 24-hour soak and alpha qualification remain
open. No objective, roadmap or POAM-035 closure.

## Reasoning budget limitation

The requested user controls remain governed by the [effort and run budget
contract](../../../reference/inference/lfg-swag.md#user-selected-effort-and-run-budgets).
Installation validates reasoning limits, but no governed reasoning runner
consumes them yet. The existing read-only preview at
`target/reasoning-budget-preview/receipt.json` accepts 120,000 ms total,
16 steps and 15,000 ms per step; it explicitly records `applied: false` and
`runner_enforcement: false`. Neither the preview nor this storage diagnostic
implements cancellation, repetition limits, runtime budget enforcement or a
Connectome effort control. Those requirements remain open under Gate G/D-06/H-06.
