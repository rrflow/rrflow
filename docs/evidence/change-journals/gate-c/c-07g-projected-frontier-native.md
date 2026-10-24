# C-07g: native result after deferred projected reads

**Status:** active immutable native evidence; setup failed
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-projected-frontier-native`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [108](https://github.com/rrflow/rrflow-development/pull/108)

This supplements the [implementation receipt](c-07g-projected-frontier.md).
Candidate `7a8bd51564b7fb103776253a85daa75be43d153c`, tree
`1f6458080b35ffb5f31ae6b3ebba9b14437abef2`, built executable
`2d9a7332abf56120e79a5fc2f4243b5f6a55273c891c304fc83cb9505e27fb3a`.
The Linux release build/resource-window command recorded in the earlier native
comparison passed all three checks; build time was 3m 34s. The owning Windows
suite's 212 passes remain in the implementation receipt, not inferred here.

Operator script `target/profile_c07_backup20.py` prepared Linux20 using the
same configuration-equality and frozen-workload copy procedure as Linux18.
It invoked `run_c07_linux_scale20.sh`, which executes the canonical
`scripts/ci/run_sustained_capacity.py` through an owned systemd scope and
cgroup-v2 allocation. Corpus, scheduling, four CPUs, 2 GiB ceiling, deadlines
and acceptance thresholds were unchanged. A separate once-per-second observer
records only backup artifact filenames/sizes. It read no payload or credential.
The campaign source was clean. No source or policy mutation accompanied this run.

The installed test failed after 97.43 seconds with acknowledged corpus ordinal
608, matching the preceding diagnostic repeat's stopping ordinal. Its label
was `rejected-api-internal`, receipt digest
`c8a189d3b9d774b1e9e3d930041478dc0694dc5e75fd10d9a791acddebb2946d`.
The label includes successful transaction-abort evidence, not an assumed
rejection based only on the API code. The original server cause is not exposed
by this closed label. At failure, RSS was 354,230,272 bytes, L0 count 13, debt
6, with zero failed flushes/compactions. These observations do not identify the
cause or prove a memory/performance improvement over a different run.

No workload segments were sealed and backup was never reached, so the file-size
trace is empty. This run does not prove a backup improvement. Forced reopens,
measured load, drain, short soak and final readback were not reached. Owned
cleanup was confirmed after 97,804 ms; the native process returned 101.

Campaign `5e943d097f8d08ebbc34a9b52170467b4171481c08ef3f167afd99044951de45`
retained 1,252 progress records (277,125 bytes), SHA-256
`6d5f5ffa5a06765ed09aeac230f1d2327b97d03ff31c6c1bfaa63f08baa77246`.
The raw campaign/launch/result/progress and digest manifest remain in operator
workspace `target/c07-linux-scale-evidence-20/`; command output is
`target/c07-frontier-profile20.log`. Bootstrap files are excluded.

Read the full run result/launch, selected progress observations, safe file-size
observer and preceding implementation receipt. Investigation followed client
classification/abort, server response conversion and engine/store errors:
several typed storage read limits deliberately map to Internal until durable
outcome reconciliation, so Internal alone cannot diagnose the failing owner.
No raw-message log or runtime error-policy change follows from this evidence.
Further attribution remains required before a causal repair.

Only this linked record, POAM-035 and generated navigation/inventory change.
The complete record/diff were reviewed. Navigation, execution inventory,
documentation (483 statuses, 481 coordinates) and diff checks passed.
No unaffected storage retest was run for evidence-only changes.
Native repeatability, backup timing, full profiles, 24-hour soak and candidate
CI remain open. No gate, objective, version or maturity status changes.
The pre-edit PR's unchanged-campaign oracle failed; no acceptance claim replaces it.
