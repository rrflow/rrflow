# C-07d: discover admitted compaction past a blocked source prefix

**Status:** active local implementation evidence; native comparison pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07d-compaction-candidate-progress`
**Owner:** OBJ-02, C-07d/g and POAM-035
**Pull request:** [107](https://github.com/rrflow/rrflow-development/pull/107)

Baseline `ea8f3071dcde8c91652b69a265f4fe7a9357a74e`, tree
`22632b127dda4cdefff431decb64a5beeec8adef`, combines fetched development/main
with PR106 `0ba414b057ddee96638d3f43f6334158f453d666`. The draft pull request
recorded scope, ownership, oracle and stop conditions before implementation.

## Observed gap and replacement

Linux17 retained the setup failure after acknowledged ordinal 376. Its new
resource observation reports 57 L0 segments, debt 50, 56 automatic compactions,
two failed flushes and resident bytes 306,159,616 under the 2 GiB cgroup cap.
There were no accepted workload segments. Campaign
`a912dbc33fcf2ce7b119d095f61ae50c4e75b30345f755a7660328d44ad7a094` and progress
`1a763e12d0d00c7aace5d68a7dc6fd18067bf32c97559959ca86d42175f1ce5f` remain failed;
cleanup was confirmed. This points to maintenance rather than proving an RSS
limit failure. It does not itself establish the complete cause of scale denial.

The existing selector tried only shrinking prefixes of the ordered source
list. A blocked oldest source could hide a later independent range that fits
the same input budget. The regression reproduces this: the first source needs
8,234 logical bytes against a 4,096-byte limit, while the later two sources fit.
The old selector returns `MaintenanceLimit` without considering them.

`Database::select_compaction_candidate` now examines deterministic contiguous
windows in the existing source order, preferring the oldest/key-first window
and its largest admissible size. Every candidate still includes its complete
target-level overlap closure and passes the existing physical/logical/version
admission. Enumeration checks the existing elapsed budget. At most one merge
is returned per offer. No cap, queue, worker, format or snapshot rule changes.
Count-blocked overlap discovery also carries the blocking state to lower-level
selection, allowing the existing bounded relief policy to see that blockage.
An inadmissible quantum still fails; no repeated maintenance loop is added.

The regression requires the two later inputs to compact, the large first input
to remain, the logical snapshot to stay fixed, every earlier snapshot to read
exact values, and an additional acknowledged WAL-tail write plus all earlier
data to reopen exactly. Existing budget-denial, overlap-relief, tombstone,
retention, publication and partitioned-flush tests remain intact.

## Authoring and traceability

Read README/AGENTS, OBJ-02, C-07 ordered packages/policy, POAM-035,
change-authoring and traceability, prior idle/foreground maintenance evidence,
and Linux17 launch/result/progress. Reviewed complete affected preparation,
selection, overlap, admission and merge/publication functions; maintenance
limits/policy; relevant compaction, maintenance and partitioned-flush fixtures.
Reviewed the full changed functions, new regression and diff. The canonical
destination is the existing database selector, shared by explicit, foreground
and idle maintenance. No parallel path or implementation move is introduced.

Changed `rrd-lsm/src/database.rs`, `rrd-lsm/tests/compaction.rs`, POAM-035,
this linked journal and generated navigation/inventory. Local evidence and
source establish the selection defect; no upstream algorithm or external
research was needed. Preserve measured resource counters and explicit denials;
no prompt, data-value or credential logging was added. Native comparison must
reuse the failed corpus, allocation, policy and thresholds, with a new exact
source/build identity. Component passes cannot qualify native capacity or soak.

## Local validation

| Command | Actual result |
|---|---|
| `cargo test --locked -p rrd-lsm --test compaction blocked_oldest_source -- --nocapture` before implementation | Failed as intended: `InputLogicalBytes`, limit 4,096, observed 8,234; 10.95 s including build. |
| Same focused test after implementation | Passed, 3.16 s. |
| Initial focused package command with `--test flush_partitions` | Cargo rejected the nonexistent target; corrected to the existing `partitioned_flush`. No test result was inferred. |
| `cargo test --locked -p rrd-lsm --test compaction --test maintenance --test partitioned_flush` | 31 passed, no failures/ignored; 8.00 s. |
| `cargo clippy --locked -p rrd-lsm --all-targets -- -D warnings` | Passed, 5.30 s. |
| `cargo test --locked -p rrd-lsm -- --test-threads=4` | 210 passed, 0 failed, 2 existing ignored child helpers, across 29 targets including doc tests; 80.31 s. |

Formatting, generated navigation/inventory, documentation and diff checks
passed. Local logs use `target/c07d-progress-*.log`. Native source/build comparison and
candidate CI remain pending at this implementation record. The Windows/macOS
full native matrix, full profile ladder, 24-hour soak and alpha acceptance are
not established by these tests. No objective, roadmap completion, version,
maturity, UI or official promotion status changes. POAM-035 stays open.

Checklist: pre-edit PR/baseline, canonical owner and behavior map, falsifying
oracle, bounded replacement, source/diff review, trace/resource/research
decisions and linked evidence are recorded. Native results will be retained
separately against their actual candidate, including any continued failure.
