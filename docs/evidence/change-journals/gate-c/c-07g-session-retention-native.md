# C-07g: installed session-retention comparison

**Status:** active short native characterization; full qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-session-retention-native`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [110](https://github.com/rrflow/rrflow-development/pull/110)

This supplements the immutable
[implementation evidence](c-07g-session-transaction-retention.md). The tested
source is `994d278cf0b95c13161b7db72e88098cb31d9480`, tree
`8d3f71c4ee6860425ce29c9bfdc22aee7b68e2a0`. Its ordinary release primary is
SHA-256 `a6e336d47402a1358b0e55bc350197027ee90ec5fabf43b0cd0cfa455b9244c5`.
This follow-up changes only evidence, the POAM link and generated discovery.
It does not change source, installed configuration, workload, limits or gate
status. No external implementation/research or additional runtime trace was
needed; retain the existing bounded public receipts and resource samples.

Linux23 uses the existing WSL/ext4 native launcher and cgroup-v2 enforcement:
four vCPUs, a two-GiB engine cap, 1000 records, 5000 stored edges, 1000 vectors
of 512 dimensions and four clients at one request/second before the 2x burst.
The logical corpus definition, installed configuration, seed, schedule, required
lanes, budgets and thresholds follow the frozen preceding characterization.
Execution timestamps and build identities differ between runs.
This is not the 100000-record Desktop qualification profile or a 24-hour soak.

The release build and three diagnostic classifier tests passed in 224.59
seconds including build. The canonical installed driver then completed all
22 segments, with zero failures, unresolved outcomes, missing metrics or
latency-budget violations in warmup, measured, burst, drain and two-minute
soak phases. It acknowledged and independently verified all 1000 corpus
records, survived two forced reopens and completed reader-pin interference
and backup-under-write. The driver took 322.86 seconds; the allocation launch
receipt records 323761 ms and confirmed cleanup. `qualification_passed` remains
false by design: this small short run cannot qualify the required matrix.

Backup creation took 4.744 seconds and creation plus catalogue verification
took 5.179 seconds, within the unchanged 30-second deadline that Linux22
failed. The measured phase's maximum scheduled latency was 121.794 ms; the
short soak's maximum was 306.163 ms. Maximum phase resident memory was
298975232 bytes. The drain met its frozen criteria, with observed compaction
debt zero. The warm storage snapshot had 16 immutable segments/34593904 bytes;
Linux22's preceding snapshot had 531/630421168. These are raw shared-host
observations, not a controlled performance ratio: Windows compilation
overlapped Linux22. Both runs' independent corpus readback digest is
`d6660f60f85694bb6d98eaca518642d74bfbd9399b29c05b46bd4ebb1dc0b300`.
Stored graph/vector data is not evidence of ANN quality or complete reasoning.

Linux23 identities:

- Campaign: `59f4c7ac28863595957218d7dd44f0ce9ff605287b5add2aae879153f3d74e82`.
- Progress: `7d49c4330f280e081da31e9b484fb5f28563fe46dfb00dc1cfff5d3c262c8aae`
  (1796673 bytes, 3998 records).
- Result file: `756a91887ee15d08a067477e6d318fd07798b63ceda60c62b2d73c25e6afe3fa`.

Safe raw artifacts, all 22 segment records and individual file digests are
retained at operator-local `target/c07-linux-scale-evidence-23/`; the preceding
failed run remains at `target/c07-linux-scale-evidence-22/`. The launch uses
`scripts/ci/run_sustained_capacity.py` and the existing
`installed_reopen::sustained::installed_sustained_capacity` oracle; operator
wrappers only select the declared manifest, retain output and inspect backup
file sizes. The actual commands/logs are `target/c07-session-linux-build.log`
and `target/c07-session-profile23.log`. Launch/result summaries, selected public
resource/backup events, the canonical authoring and C-07 owners, and the
implementation journal were reviewed. Generated navigation/inventory and the
final diff are checked before commit.

On the same source, `cargo test --release --locked -p rrflow-cli --test installed_crash --features crash-test fresh_installed_domain_boundaries --jobs 6 -- --exact --nocapture`
passed in 13.86 seconds (241.84 seconds for the wrapper including diagnostic
build and ordinary-primary restoration). The diagnostic primary SHA-256 is
`699629e90c7d1213dc4981cf03c42f548ce1c169a6b470ce7d49cb63db9dbc0f`.
It hit all five domain boundaries (`wal_before_write`, `wal_appended`,
`wal_synced`, `visible`, `receipt_persisted`) and five compound recovery-read
kills. The oracle reconciled six logical calls/21 attempts, five uncertain
outcomes, six present effects, zero unresolved outcomes and zero violations.
Each readback had one application. The ordinary primary was checked to exclude
both diagnostic markers before the test, then rebuilt through Cargo and
verified byte-identical afterward. Its distribution check is retained in
`target/c07-session-ordinary-distribution.json`; the raw diagnostic log is
`target/c07-session-linux-crash.log`. Host stdout/stderr interleaving affects
that log's printed receipt framing; the test's in-process reconciliation and
exit status, rather than a parsed log receipt, supply this bounded result.

These results precede the reader-format fence identified in review and cannot
be attributed to that follow-up revision. The subsequent journal binds its
regression and final-candidate evidence. Other installed crash modes,
publication/reclamation, macOS/Windows native matrix, complete capacity
profiles and the 24-hour soak are not supplied by this record. POAM-035 and
C-07 remain open; no alpha or merge claim follows.
