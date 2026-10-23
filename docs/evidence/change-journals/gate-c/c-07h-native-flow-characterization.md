# C-07h combined native flow and fleet characterization

**Status:** active bounded native evidence; qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07h-native-flow-characterization`
**Owner:** [C-07h](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 / POAM-035

Evidence-record baseline e674d06c0c7711dd134d2f754f0168748c0aa274, tree 5869d4b8684c185f016162117340d1d618b6e763. The PR76 description bound this
documentation-only reconciliation before editing. This record reports completed
native executions and retained failures. It does not alter workload acceptance,
engine behavior, profile minima or gate status.

## Exact execution identities

| Run | Candidate | Ordinary executable SHA-256 | Campaign SHA-256 |
|---|---|---|---|
| Windows high load | 79663cdd4d162379c82a2fab0ff3f87d5cbf907e | a43a6fde4afdecade3468f932c861e6d110783919bd2dcf2c07f593edde32840 | cf74425d3e10b26517f35f65fd3720325dd51d23aecd11d0ef6f691c52a415fe |
| Windows two-estate fleet | d2882fb31625366fe115b1f4f1937ffce5422f1d | a43a6fde4afdecade3468f932c861e6d110783919bd2dcf2c07f593edde32840 | 8d3500b5d6175aba4edb2dd86ddb08ac8194a5c99903d4cc50df4132b704fbe0 |
| Linux WSL ext4 cold flow | 79663cdd4d162379c82a2fab0ff3f87d5cbf907e | 28f4fdc3a7d63bb0dad0d377e5d525196a95b8f1cd2d6f1f9b3d01ab4a892aed | 5084b098fa831a0fb980dd882cb0fa48f64b40548f8723cf1a73fccc71466f5e |

Each run uses a real installed optimized engine, four CPU capacity units and a
4 GiB kernel-enforced process-tree allocation. The fleet shares one allocation
across both estates. The Linux environment is Ubuntu under WSL2 kernel
6.18.33.2-microsoft-standard-WSL2, not a bare-metal Linux performance claim.
Its isolated Rust toolchain is the repository-pinned 1.99.1. Candidate checkout
fetches the pinned Connectome submodule; default readiness does not use it.

## What completed and what did not pass

The Windows high-load execution completed in 298016ms with 28 linked segments,
backup-under-write, retained readers, two pre-load forced reopens, final restart
and exact readback. It performed repeated automatic compactions without the
previous maintenance-admission/Internal failures. The last segment digest is
df16c16229c2d3539b36c4bbf1e3ee20a9b70cfbd4b2a47d8d1939fc7ed4290e.
However, its 5400 offered operations include 3996 rejected operations and 470
latency-budget violations; unresolved operations are zero. Twenty requests per
second is NOT a supported operating point. A completed characterization and a
successful evidence verifier do not mean that its performance thresholds passed.

The repaired fleet execution completed in 328140ms: quiet, equal and abusive
trials, each with two independent installed estates and four clients per estate.
The ordinary rate is one request/second per estate; the abusive member doubles
that rate. All six child campaigns completed: 520 offered operations across 50
segments, zero rejected/uncertain operations and zero latency-budget violations.
Forty-two phase/reopen boundary observations record explicit foreign-session
denials and kernel allocation membership. Independent current/final readback,
retained cursors, backup overlap and restart receipts reconcile. The aggregate
reports `isolated_latency_passed=true`, `qualification_passed=false`.
The three trial digests are:

- f43456b0836f09e5dc11beb6f9c80941ddbdadbd0e6d5ea7de3242606a6843cc
- 12c010aea3c3140895951a801f018dd3f68e2c0745f62682a210d72eaea55495
- 530666806b4589931d0647c56c2b699328b8b06fa86eb80feca59d4e727ea4f2

The Linux cold execution completed in 144042ms: 130 offered operations across
13 linked segments, zero rejections, unresolved outcomes or latency-budget
violations, with cold/warm, backup, retained-reader and forced-reopen lanes.
Before restart it synchronized and advised eviction of five owned regular
files, 17649791 bytes and 4312 pages; mincore observed zero resident pages.
Readback after restart was exact. The last segment digest is
9ec94d481779737da4faa9d63f5ec0016374a62cef37d74183478bfc6d0ca30f.
All three outer launch receipts confirm cleanup and exit zero. All three
qualification flags remain false because these are bounded characterizations,
not the owning full corpus/rate/seed/duration profiles.

## Failures retained and placement decisions

The first fleet campaign failed `invalid client timing boundary`: an idle
neighbour spent the whole segment budget before its real verification probe.
The [idle reservation repair](c-07g-idle-segment-deadlines.md) reserves probe time
and enforces the outer deadline without changing the ledger. The completed
fleet above runs that repair. Its owning evaluation suite passed 40 tests and
strict CLI/evaluation lint before the native rerun.

The first Linux bootstrap copy did not preserve executable permissions; the
operator preparation helper now preserves file mode. The next Linux cold
attempt correctly failed `cache advice left resident pages; cold state unproven`.
Inspection showed Ubuntu's default /tmp is tmpfs. The successful run explicitly
sets TMPDIR to an owned directory on the declared ext4 filesystem; it does not
relax zero-residency acceptance or change host-wide cache settings. The
[Linux fadvise contract](https://man7.org/linux/man-pages/man2/posix_fadvise.2.html)
explains why advice alone is not proof: only actual observed residency is used.
Windows process command inspection confirmed the earlier default temporary
estates were already on D: NTFS. No Windows volume defect is claimed.

The earlier 24-hour attempt was intentionally replaced after 116 segments by
candidate d2882fb3 with the repaired harness, canonical owner-projection digest
and explicit owned D: temporary placement. Its named Windows Job, memory limit
and native member PID were verified before termination; the failed/interrupted
receipt and operator-stop record are retained. The replacement campaign13 is
in progress, NOT a 24-hour result. No elapsed partial run is counted as a pass.

## Evidence locations and acceptance checklist

Operator-local roots relative to `D:/localdev/devlabs/rrflow/target`:

- `c07-windows-characterization-11/evidence` (high load; nonpassing thresholds).
- `c07-windows-fleet-characterization-01/evidence` (retained failed quiet trial).
- `c07-windows-fleet-characterization-02/evidence` (completed repaired fleet).
- `c07-linux-cold-evidence-03` (bounded copy of complete Linux raw evidence).
- Linux original: `/home/devcontainers/rrflow-c07-native-79663cdd/cold-characterization-03/evidence`.
- `c07-windows-characterization-12` (interrupted, cleanup recorded) and
  `c07-windows-characterization-13` (new explicit-placement long run).

Execution uses `python scripts/ci/run_sustained_capacity.py --campaign <frozen>
--configuration <sealed> --evidence <new>`; Linux additionally supplies the
writable delegated user cgroup parent and runs inside a user systemd scope.
No host-wide cache, cgroup or firewall configuration changes are made.
The launcher compiles/selects the ordinary optimized installed_reopen harness,
executes its explicit installed_sustained_capacity case, enforces native
allocation, then verifies the complete raw campaign. A separate rerun of
`rrflow-eval storage campaign verify <evidence>` passed for all three completed
roots above using the current d2882fb3 verifier. This checks their hash links,
ledgers and aggregates; it does not promote characterization into qualification.

Reviewed records: C-07/objective/POAM owners, candidate change journals and PR
briefs; complete result/launch/campaign receipts, bounded member aggregates and
phase-boundary records, cold-file progress, process membership and the actual
source methods for idle timing, mincore/fadvise and owned-process cleanup.
No runtime code or research implementation was adapted for this record. New
paths are this journal; POAM-035 and generated navigation/inventory are updated.
Full changed-document/diff review and documentation checks precede commit.
Exact current candidate CI, the complete declared capacity profiles, full
24-hour acceptance and final C-07h reconciliation remain uncompleted. These
local artifacts are not GitHub candidate-CI or release-promotion evidence.
C-07, OBJ-02 and POAM-035 stay open; no merge or full-alpha claim is made.
