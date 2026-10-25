# C-07g: completed integrated 10k native characterization

**Status:** active implementation evidence; full qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-admission-native-characterization`
**Owner:** OBJ-02 / C-07c,g / POAM-035 / PR121

## Source and scope

This is the native follow-through for [PR121](https://github.com/rrflow/rrflow-development/pull/121)
and its [admission diagnostics](c-07g-reopen-admission.md), including the
integrated PR117/119/120 implementation. Exact clean source:
`316569a3abe2060c324581f838e250ed4aead47b`. No additional engine mutation was
made for this run. Preserve the earlier [failed integrated runs](c-07g-integrated-native-growth.md):
this completed characterization does not explain their startup denial or
sampling gap, and it cannot erase them.

The run uses the ordinary release binary on native WSL ext4, four vCPUs and
an enforced two-GiB engine allocation. The observed host has 33352753152 bytes,
so this is not the frozen eight-GiB Desktop host. Installed page cache remains
explicitly 64 MiB for this characterization; the shipped default is unchanged.
The frozen corpus contains 10000 records, 50000 relations and 10000 vectors
with 384 dimensions and 10240000 logical source bytes. Four clients offer
5 requests/second, with a 10-request/second burst and unchanged 500-ms latency
budget. Warmup/measured/burst/drain/short-endurance durations are respectively
10/30/10/15/60 seconds. Two configured forced reopens, cold-start checks and
final restart/readback run on the same estate. This is one seed, not the full
profile/repetition or 24-hour matrix.

## Exact result

- Campaign: `b2a551a5d15b56cb4e32fdf23e33bd54f07e3b61c2abd470f15f006898fa8466`.
- Ordinary executable: `21beef8d06cf54aff53d3d73d1d8795dee314cfe1e4eff413fb6bbfbe2b813cb`.
- Harness: `c5fceef41dfd9139606e34b0c75a8b24db64a8df852385e96d09bed95de2d8b0`.
- Twelve accepted evidence segments; 600 offered operations acknowledged;
  zero failed, rejected, unresolved or over-budget operations in this run.
- All 10000 original seed records were independently verified. Both forced
  reopens preserved original replay receipt identities. The final restart
  independently verified those 10000 records and all 40 working slots.
- Backup creation took 11.216384868 seconds; archive verification took another
  1.538920624 seconds. Offers continued across the segment boundary while
  backup ran. This exercises the bounded scheduling implementation, not merely
  a standalone archive command.
- The peak sampled RSS was 394166272 bytes. Observed automatic compactions: two,
  with zero failed compactions and 8388542 peak compaction-buffer bytes. The
  drain tail has zero native transactions, active readers, pinned bytes,
  outstanding requests, queued requests and compaction debt.
- Final readback took 161.245015355 seconds. This remains a material cost;
  preserving complete readback is part of the oracle.
- Harness runtime: 779.59 seconds. Launcher exit: zero, cleanup confirmed.
  Result status is `completed`; `qualification_passed` is explicitly false.

The last segment digest is
`a622662baeb55ad06b73c10cd3556c15cf0d10892508b3f201a7d23269ff85e9`.
The retained progress stream contains 32186 records and 7245625 bytes, digest
`083d4abdaf2dd0d0f300940774820086e2a2fa049756373df4eca4d1babc6ac4`.
These identities bind the completed run; they are not substitutes for the raw
receipts and independent ledgers.

## Commands, retention and remaining work

The clean release build used `cargo test --release --locked -p rrflow-cli
--test installed_reopen --no-run --jobs 6`. The bounded launcher used
`python3 scripts/ci/run_sustained_capacity.py --campaign <frozen-campaign>
--configuration <frozen-configuration> --evidence <owned-evidence>
--cgroup-parent /sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service`.
The exact command/source/configuration/artifact identities are retained at
`/home/devcontainers/rrflow-c07-native-79663cdd/integrated64-5rps-growth-10000-316569a3/`.
Safe result, configuration, launch, timing and resource summaries are copied to
operator `target/c07-pr120-evidence/316569a3/10000/`; bootstrap credentials were
not copied or printed. An analysis invocation made before `result.json`
existed failed with FileNotFoundError; it was rerun after completion and did
not mutate the campaign or classify a partial run as passing.

The prior startup admission error did not reproduce. No canonical root-cause
repair is claimed. A separately frozen 100-reopen characterization of this
same source and 10k corpus started only after the previous launcher confirmed
cleanup. Its outcome remains pending and belongs in a later evidence record.
The new [macOS metadata-pressure package](https://github.com/rrflow/rrflow-development/pull/122)
also awaits native candidate evidence. Full profiles, repetitions and actual
24-hour soak still follow the owning C-07 plan.

Checklist: the PR's existing native-reproduction brief authorized this run;
source and thresholds were frozen; independent records/relations/vector
readback, receipt replay, backup, compaction, retention/drain and native
resource evidence were retained; no runtime algorithm or research claim was
added. Read the complete result, launch receipt, generated timing summary,
resource/backup summary and progress event identities. Source review remains
in the linked implementation record. This record and owner links are the only
new tracked effects; navigation/inventory are generated. Documentation and
generated checks precede commit. Gate C, POAM-035 and alpha remain open.
