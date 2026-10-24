# C-07d: installed comparison after candidate-discovery repair

**Status:** active immutable native characterization evidence; campaign failed
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07d-compaction-native-comparison`
**Owner:** OBJ-02, C-07d/g and POAM-035
**Pull request:** [107](https://github.com/rrflow/rrflow-development/pull/107)

This follows the [implementation receipt](c-07d-compaction-candidate-progress.md)
without rewriting it. Starting candidate is
`245220065260b6e15869233569e453a8988daede`, tree
`1e69526b22aee735375c841520f238a80d0851ce`. Its installed Linux executable is
`26ec0178d5b9944418e2f03991449ab7f26daef4d884adbf81f364027326c63b`.

## Scope and method

Run the previously failed Linux17 characterization against the repaired selector.
The operator-local preparation asserts identical installed configuration, then
copies the previous allocation, corpus, schedule, workload, thresholds, seeds,
required lanes and storage budgets into a new campaign with actual candidate,
build and environment identities. This retains 1,000 records, 5,000 edges,
1,000 vectors of dimension 512, four clients at one request/second, a 2 GiB
cgroup-v2 ceiling, four CPU allocation and a 30-second operation deadline.
The native source is clean, on WSL ext4. No source, policy, timeout, corpus or
acceptance threshold was changed for the comparison.

Read the authoring procedure, C-07 owner, POAM-035, initial implementation
receipt, complete affected selector/overlap/admission functions, installed
launcher, operator-local preparation scripts, retained campaign/result/launch
and progress observations. Reviewed the backup driver and engine backup entry
point to locate the newly reached failure boundary. Local measured evidence is
the research input; no upstream code or external research is required to report
this comparison. Preserve actual resource/time observations and failure labels;
do not infer a backup cause from the deadline alone.

## Executed commands and results

- `cargo test --release --locked -p rrflow-cli --test installed_reopen resource_window --jobs 6`
  built the Linux binary in 4m 26s and passed all three resource-window checks.
- `bash /mnt/d/localdev/devlabs/rrflow/target/prepare_c07_linux_scale18.sh`, which
  invokes the operator-local `prepare_c07_linux_characterization.py`, followed
  by the operator-local equality/copy procedure above, prepared Linux18.
- `systemd-run --user --scope --unit=rrflow-c07-scale18 python3 scripts/ci/run_sustained_capacity.py --campaign <stage>/campaign.json --configuration <stage>/configuration.toml --evidence <stage>/evidence --cgroup-parent /sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service`
  ran the real installed harness and failed after 307.20 seconds. The launcher
  retained exit 101 and confirmed owned-process/cgroup cleanup.

Linux17 stopped at acknowledged ordinal 376, with 57 L0 segments and debt 50.
Linux18 acknowledged and independently verified all 1,000 corpus records,
completed both forced reopens and verified the seeded corpus after reopening.
The warmup ownership baseline had zero compaction debt. Warmup completed 20
attempts with zero failures/unresolved outcomes or latency violations; the first
10 measured attempts also had zero violations. Forty load outcomes/readbacks
were retained before failure. Observed resource samples peaked at 263,634,944
resident bytes; the final sample had seven L0 segments and zero debt.
These are observations of the sampled phases, not a setup-memory peak claim.

The campaign then failed with `backup interference deadline`. Three segments
were sealed, but the backup did not return its complete create-and-verify
evidence inside the unchanged 30-second deadline. The deadline alone does not
identify whether creation or verification dominates. Burst, drain, short soak,
final readback, full capacity profiles and the 24-hour soak were not completed.
This remains a failed campaign; clearing its earlier setup blocker does not
qualify the operating point or C-07.

## Retained identity and remaining work

Campaign identity:
`9288c5f1a9e1e74c09f4da83a77c7985b5f37f6b08153ac9f74bf832b2a88c05`.
Progress: 3,851 records, 1,034,027 bytes, SHA-256
`4b481ff7f956ba1cf5193a82213ddc93551b1a2e3558de4da9ce8238d7be025e`.
The exact campaign, launch/result, progress and all three segment files are
retained in operator workspace `target/c07-linux-scale-evidence-18/` with a
per-file digest manifest; the complete command log is
`target/c07d-progress-linux-scale18.log`. The source WSL stage is
`/home/devcontainers/rrflow-c07-native-79663cdd/scale-characterization-18/`.
Credential/bootstrap files are excluded from the copied evidence. Linux17 is
retained separately and is not reclassified.

This package changes only this linked evidence record, POAM-035 and generated
navigation/inventory after the already-tested implementation commit. The first
documentation check rejected the new record's missing `active` status marker;
the record was corrected. The complete new record and diff were reviewed.
`render_navigation.py`, `build_execution_inventory.py`,
`python -X utf8 scripts/ci/check_documentation.py` and `git diff --check` passed
(481 document statuses, 479 classified coordinates, 1,568 inventory records).
Unaffected storage suites were not rerun for prose alone.
The initial receipt owns those 210 passing storage tests and strict lint.
Candidate CI is pending and no merge is claimed. Native macOS/Windows matrix,
full profiles and 24-hour qualification remain unproved. Backup timing remains
C-07g/POAM-035 work at the existing application-backup owner. No roadmap,
objective, product maturity or version completion status changes.

Checklist: pre-edit PR scope and baseline, unchanged-workload oracle, actual
failure and cleanup, source/build/evidence identities, resource/research
decisions, excluded checks and remaining owner are recorded. No threshold
relaxation or new status authority is introduced.
