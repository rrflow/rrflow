# C-07g: completed integrated 10k corpus and 100 native restarts

**Status:** active implementation evidence; full profile and 24-hour acceptance open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-integrated-hundred-reopens`
**Owner:** OBJ-02 / C-07c,g / POAM-035 / PR123

## Frozen source and oracle

This follow-through to [PR121](https://github.com/rrflow/rrflow-development/pull/121)
ran exact clean source `316569a3abe2060c324581f838e250ed4aead47b` on native WSL
ext4. Its evidence is retained in the [integrated landing package](c-07-integrated-growth-candidate.md);
it is not rebound to that package's later revision. No engine mutation was
made for this run. The prior [completed two-reopen characterization](c-07g-admission-native-characterization.md)
and [failed integrated runs](c-07g-integrated-native-growth.md) remain intact.

The frozen allocation was four vCPUs, a two-GiB engine cgroup and an observed
33352753152-byte host. Configuration explicitly selected a 64-MiB page cache;
the default remains unchanged. The corpus contains 10000 records, 50000
relations, 10000 vectors of 384 dimensions and 10240000 logical source bytes.
One seed, four clients, 5 RPS and a 10-RPS burst use the unchanged 500-ms latency
limit and zero-error budget. Warmup/measured/burst/drain/short-endurance phases
are 10/30/10/15/60 seconds. The separate manifest specifies 100 forced reopens.
This is characterization, not the eight-GiB Desktop profile or actual soak.

## Completed result

- Campaign: `b9714f6600d75170567848af5bfe5a64da2f8d88e21d72d71f9ca6830671d9cc`.
- Ordinary binary: `21beef8d06cf54aff53d3d73d1d8795dee314cfe1e4eff413fb6bbfbe2b813cb`.
- Harness: `c5fceef41dfd9139606e34b0c75a8b24db64a8df852385e96d09bed95de2d8b0`.
- All 10000 corpus effects were acknowledged and independently verified.
  Each of 100 forced process restarts checked every prior lifecycle effect
  before replay, then required its original receipt identity. The last cycle
  checked 100 operations; original and replay receipt-chain digests both equal
  `f6fa4f7c70e50d55bde6b5b66e90a8b314f254a435762c2710ad54987e65c31a`.
- Twelve accepted segments contain 600 acknowledged offered operations, zero
  rejected/failed/unresolved operations, zero latency overruns and no missing
  required metrics. Measured maximum scheduled latency was 78.107625 ms;
  burst maximum 165.299003 ms; short endurance maximum 258.099320 ms.
- Backup creation took 14.330316030 seconds and verification another
  2.009916624 seconds while offered operations continued. The complete archive
  contains 10112 runtime commits and 70774 runtime mutations.
- Peak sampled RSS was 443793408 bytes. Observed automatic maintenance after
  the measured-process restart included one flush and two compactions, with
  zero failed flushes or compactions. Compaction input/output was
  21009434/19351778 bytes; peak compaction-buffer charge was 8388285 bytes.
  One write-stall increment was observed. These counters do not count
  maintenance before earlier restarts. Drain ended with zero active read views,
  native transactions, pinned/projected bytes, outstanding/queued requests and
  compaction debt; accounted mutable bytes were 49227447.
- Final restart independently checked all 10000 seeds and 40 working slots.
  It took 261.535272639 seconds, preserving the complete readback oracle.
  Final states digest:
  `99b8b432b053266e6b4a15b315501bfb350cf8a99fb033c97da1651e0b37f652`.
- Launcher completed with exit zero in 1310.700 seconds and confirmed owned
  process/cgroup cleanup. Result status is `completed`;
  `qualification_passed` is explicitly false.

The final segment digest is
`e64846cb7a1785083a3b3f8a4f72fb2d264149a466a600b001f7907a5e5c7b11`.
The sealed progress stream has 32509 records, 7349480 bytes and digest
`969b9b14681b61e007f552dbc35dd2d8bf9fc686dcdaa0d782586d41cb3fb671`.
Raw evidence remains at
`/home/devcontainers/rrflow-c07-native-79663cdd/integrated64-reopen100-growth-10000-316569a3/`.
Safe summaries are retained at operator
`target/c07-pr120-evidence/316569a3/reopen100/10000/`; no bootstrap credential or
private estate content was copied into the repository.

## Commands, review and limits

The existing launcher ran `python3 scripts/ci/run_sustained_capacity.py
--campaign <frozen-campaign> --configuration <frozen-configuration> --evidence
<owned-evidence> --cgroup-parent
/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service`. It canonically
validated the frozen source/build/configuration, enforced the native limit,
ran the ordinary installed harness and verified the completed evidence stream.
Post-run analysis separated response, dispatch and readback timing. Review
included the complete result and launch receipts, all segment outcome totals,
resource/backup summaries, final readback and last restart receipt identities.
The existing lifecycle/workload verification source was reread; independent
vector values, record payloads and exact relation sets share stamped cursors.

No new algorithm or research claim is introduced. The PR brief authorized the
run before execution; prior cleanup preceded launch and subsequent compilation
waited for this run's cleanup. Documentation/navigation/inventory checks are
recorded by the integrated package before publication. The observed host does
not satisfy the fixed Desktop allocation; this run lacks the full corpus,
profile matrix, three seeded repetitions and 24-hour duration.

The earlier startup link-admission failure did not reproduce across these
100 cycles. Its cause remains unknown; neither diagnostic code nor this
passing run is claimed to repair it. The full Desktop-corpus preflight follows
on the combined candidate with its own identity. C-07 and POAM-035 remain open.
