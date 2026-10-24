# C-07g: Desktop corpus preflight launch

**Status:** active native run; no completed capacity or soak result
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-desktop-preflight-launch`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [111](https://github.com/rrflow/rrflow-development/pull/111)

Evidence-package baseline `b3688cebb0d86ef95bad6629aeb7c0a63e8562f6`, tree
`e7c5503e59f385d2eda302d22b95bc3f879e9f3b`, joins fetched public main with
PR110's exact `c206c270ee5a2d1a3e07f7ffe6e2d4ec0466be61` candidate. The trees
are identical. The draft PR brief precedes this journal and POAM/index edits.
No engine source, policy, workload oracle or acceptance threshold changes.
The preceding [short installed result](c-07g-session-retention-final-native.md)
does not supply full-size or 24-hour evidence.

PR107's candidate `8d2755a451c9523072bf1e34a7c95e8ea697c98f` passed all 23
checks, including `pipeline / ci-gate`, in
[run 40029932444](https://github.com/rrflow/rrflow-development/actions/runs/40029932444).
It merged through its PR as `335c9b67d8913f548de119378929a7aba8062a5a` at
2026-10-24 01:01:22 UTC. The exact repository/ref/revision/CI binding was
recorded before merge. PR106 was included by ancestry. PR110 still awaits
macOS CI; no pending candidate was merged or pushed directly to main.

Linux25 freezes PR110's clean candidate and ordinary primary
`5c460fe660bac0c4a11957e5b35bc03ad742ed7f635a932f314674f01c1d5ebc`.
The unchanged installed configuration digest is
`2d382dbde4b923db65fb5ebbf841d12b45ec0e278c7cab41635dbff782072d55`.
The Desktop corpus comes from the existing J-04 projection: 100000 records,
500000 relationships, 100000 vectors with 384 dimensions, and 102400000
logical base-payload bytes. The owned cgroup limits the aggregate harness and
engine to four vCPUs/two GiB with swap disabled. The observed WSL host has
33352753152 bytes, exceeding Desktop's fixed eight-GiB host. That difference
is retained explicitly; this is characterization, not Desktop qualification.

After complete ingestion and independently regenerated readback, the frozen
point offers 25 requests/second across four clients. Setup is bounded to two
hours; operations retain a 30-second deadline. Schedule: 60-second warmup,
five-minute measurement, 30-second 2x burst, 60-second drain and ten-minute
short soak. It retains all existing lifecycle lanes and two forced reopens.
There is one seed (7), a two-GiB evidence quota and a 64-GiB declared disk
budget; preparation observed over 80 GiB free. No limit can change mid-run.
The canonical campaign identity is
`907ddbf92cd5c23870094e7a72ee7d4df17639e05ce4208132943a6ac922cd2e`;
the input file SHA-256 is
`c34a988630a0fb9fba7fb76af3ae76102ecea16c6e19950250b992dce6382666`.

Ordinary release build and installed harness build passed; preparation took
9.59 seconds. `cargo run --quiet --locked -p rrflow-eval -- storage campaign validate`
accepted the frozen manifest with purpose Characterization. The existing
`scripts/ci/run_sustained_capacity.py` runs under user systemd unit
`rrflow-c07-desktop25.service`, launched at 2026-10-24 01:00:26 UTC, with a
15000-second outer service limit. The launcher retains its own shorter,
manifest-derived deadlines and owned cgroup cleanup. At 01:03:32 UTC the
service was active and progress contained 7360 acknowledgements (last ordinal
7359), with ingestion still running. There was no complete readback, final
result, cleanup receipt or qualifying soak result at that observation.

Operator-local launch inputs, intent, stdout/stderr and eventual evidence are
under `/home/devcontainers/rrflow-c07-native-79663cdd/desktop-preflight-25/`.
Bootstrap records there are private and must not be copied into evidence.
The safe evidence subdirectory contains the canonical campaign/progress and,
only after completion, result/launch receipts. Preparation/launch wrappers
are `target/prepare_c07_desktop25.sh`, `target/run_c07_desktop25.sh` and
`target/c07-desktop25-prepare.log`; they configure this explicit experiment
and invoke existing repository tooling, not a new product dependency.

The PR108 macOS crash job also exposes a retained qualification failure:
[job 120691007445](https://github.com/rrflow/rrflow-development/actions/runs/40034335616/job/120691007445)
panicked at `capacity.rs:264`, `bounded metadata fill`, in the owned-volume
filler. This is failure to complete the native exhaustion fixture, not proof
of a lost acknowledged engine write. Its raw log is retained at operator-local
`target/c07-pr108-job-120691007445.log`, SHA-256
`41c74eebeb4eced7d55c7ea26d45379cdadc708deb688b79ddc35a50f064ffdb`.
No deadline was enlarged and no failed job rerun to replace this observation.
This repeats the metadata-fill deficiency already tracked by POAM-035.

Read README, objective OBJ-02, C-07/J-04 workload and duration owners, POAM-035,
authoring/CI procedures, capacity projection, complete corpus seeding/readback,
campaign/profile validation, allocation verification, sustained launcher,
operator preparation/retention scripts, prior session evidence and complete
filler implementation. Preserve existing public diagnostics and resource
samples. No source adaptation was needed. A preliminary check of the official
[Ubuntu cloud image catalogue](https://cloud-images.ubuntu.com/noble/current/)
did not select or provision a VM. Local Hyper-V inventory lacks permission;
WSL has accessible KVM but no QEMU binary and interactive sudo is required.
The current primary also requires GLIBC_2.43, so it must not be assumed to run
on an older image. No privileged host or VM configuration was changed.

The next outcome is the canonical completed result or retained first failure,
then `storage campaign verify` over the exact artifacts. Repair any verified
defect in its existing owner before advancing. Qualification still requires
the chosen profile's actual allocation, five-minute warmup, 30-minute measured
interval, three seeded repetitions, 100 forced reopens, all lanes and 24 hours,
plus candidate CI and the broader native negative matrix. This record neither
starts nor claims that qualification. No C-07, objective or POAM closure.

Only this journal, the POAM link and generated navigation/inventory change.
The journal, changed POAM passage and complete diff were reviewed. Navigation
generation, execution inventory (1580 records), documentation policy (490
statuses, 488 coordinates) and `git diff --check` passed. Unchanged Rust suites
are not rerun for this evidence-only package. The draft remains subject to
exact candidate CI and later completed native evidence; no PR111 merge is claimed.
