# C-07h: retained native failures and combined component checks

**Status:** active evidence record; failed characterizations retained, qualification open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07h-retained-native-failures`
**Owner:** [C-07h](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [76](https://github.com/rrflow/rrflow-development/pull/76)

This immutable follow-up starts at acceptance-branch revision
`43448e0850ee55e0a1dd068092e66ec084a3842b`. Its PR brief was updated before
edits. It preserves the [earlier native record](c-07h-native-flow-characterization.md)
and changes no runtime code, threshold, release version or gate status.

## Completed failed campaigns

| Campaign | Exact tested source | Actual failure |
|---|---|---|
| Windows13: NTFS, 4 CPU/4 GiB, 32 records/96 edges/64 vectors at 8 dimensions | `d2882fb31625366fe115b1f4f1937ffce5422f1d` | 255 completed segments; resource coverage failure after 2,571,563 ms |
| Linux04: WSL ext4, 4 CPU/2 GiB, 1,000 records/5,000 edges/1,000 vectors at 512 dimensions | `79663cdd4d162379c82a2fab0ff3f87d5cbf907e` | 1,000 writes acknowledged, 936 setup records verified; 30-minute setup deadline failed |
| Linux05: same frozen corpus/allocation/deadline | `d209218da735b26197ec9feeb3c1873eb480383b` | 1,000 writes acknowledged, 950 setup records verified; same deadline failed |

All three launcher receipts report nonzero execution and confirmed cleanup.
Neither Linux campaign reached a measured segment. Their launcher elapsed
times are 1,802,380 and 1,802,894 ms. The 14 additional verifications do not
establish attribution or a supported rate. Both harness logs explicitly report
corpus setup exceeding its frozen deadline.

| Campaign | Campaign SHA-256 | Executable SHA-256 |
|---|---|---|
| Windows13 | `06ff105f0d3c280aed7801319abc2e27c0f5cc4ff132fa9be4ceb9aa2ff1c4e8` | `a43a6fde4afdecade3468f932c861e6d110783919bd2dcf2c07f593edde32840` |
| Linux04 | `704aee55ebcc0bedd94affb2abc899adb3cc14b9c1a3861732c0f29cfcd27b81` | `28f4fdc3a7d63bb0dad0d377e5d525196a95b8f1cd2d6f1f9b3d01ab4a892aed` |
| Linux05 | `c38bd351562a0086b6e24b73bd272b34bcbe34174f03274b88fbcf8ed17e7e04` | `f0c2e964ee5171c42fe12e5e016c35aee37609b5fcf576fc93655ec720f79ba4` |

Windows13 failed-segment digest:
`b1e5e6b39eea4d1adc2a4091b8d42e4e8b8a708fec8b11dcabf2ea11b63f6544`.
The validator reported a resource-series gap or invalid process identity.
Actual samples in the preceding late segment fell inside the next scheduled
window but were omitted there. [PR81](https://github.com/rrflow/rrflow-development/pull/81)
preserves those actual observations without rewriting time or inventing samples.
The failed campaign also retains real latency violations; the coverage repair
does not erase them.

## Completed combined checks

At frozen source `d209218da735b26197ec9feeb3c1873eb480383b`, direct subprocess
execution reported actual exit zero for:

- `cargo test --release --locked -p rrflow-cli --test installed_reopen resource_window --jobs 6`: two passed on Windows and two on Linux.
- `cargo clippy --release --locked -p rrflow-cli --all-targets --jobs 6 -- -D warnings`: passed on Windows.
- `cargo test --release --locked -p rrflow-eval --jobs 6`: 47 passed on Windows.

Windows executable digest:
`81b953e476b41018b5620d1e4fb884c7919ca1e20bdffe7c3d6e006b8a7c8522`;
Windows harness:
`0bb009f8ec2c940305e7bf40b746fcc5914ebb3d86c0ab489a95dac73afa589c`;
Linux harness:
`7cde66091c2ace55ebf083d8479d91d5ac5e4158448c1fe72f53b329ef4d9d4d`.
These checks do not override Linux05's native timeout. An earlier PowerShell
wrapper returned exit1 around successful Cargo output; the direct subprocess
checks retain Cargo's actual exit and the wrapper discrepancy.

Windows14 is a separate requested 24-hour characterization at that source.
Partial observations cannot count as a completed soak. The later addressed
read and complete pipeline repairs are separate candidates;
[PR85](https://github.com/rrflow/rrflow-development/pull/85) owns their joined
oracle and exact-source native comparison. Do not combine unlike revisions
into a nominally passing operating point.

## Provenance and checklist

Raw roots relative to `D:/localdev/devlabs/rrflow/target`:
`c07-windows-characterization-13/evidence`, `c07-linux-scale-evidence-04`,
`c07-linux-scale-evidence-05`. Linux originals remain under
`/home/devcontainers/rrflow-c07-native-79663cdd/scale-characterization-{04,05}/evidence`.
The operator projection `c07h-retained-failures-proof.json` derives source,
tree, binary, configuration, progress counts and cleanup from actual
campaign/result/launch/progress files. It is discovery, not acceptance authority.
Component logs: `c07g-overlap-{focused-combined,lint-combined,eval-combined}.log`
and `c07-linux-overlap-build.log`.

Read result/launch/campaign records, progress counts and terminal failures,
relevant resource-window observations, actual Cargo summaries, the C-07/POAM
owners and preceding journals. Reviewed this complete record and its diff.
The scoped pre-edit brief, baseline, resource/research decision and evidence
paths are recorded. No upstream adaptation or external research is needed for
receipt reconciliation. An initial documentation patch used a nonexistent
POAM context and made no changes; it was corrected against the actual file.
Navigation, inventory, documentation and diff checks precede commit; runtime
tests are not rerun for this prose-only package.

Full profiles, 24-hour completion, exact candidate CI, ordinary PR landing and
C-07h reconciliation remain required. C-07, OBJ-02 and POAM-035 remain open.
No alpha, reasoning, competitive or promotion claim is made.
