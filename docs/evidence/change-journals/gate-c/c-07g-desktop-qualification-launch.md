# C-07g: nominal Desktop qualification campaign launch

**Status:** active native campaign; completed qualification remains unproved
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-desktop-qualification-launch`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [111](https://github.com/rrflow/rrflow-development/pull/111)

This evidence continuation starts at
`31f95a76a90e951eac3c9fdafc4ee161363b6cc3`, tree
`c3d40262af4f4eb0ab921fa4839298cba84779e0`. PR111's extended change brief
preceded these repository edits. The [host preparation](c-07g-desktop-vm-preparation.md)
and [WSL preflight launch](c-07g-desktop-preflight-launch.md) remain immutable.
The package advances from a prepared host to actual execution of the existing
qualification driver. It changes no engine source, default configuration,
schema, workload oracle or acceptance threshold. Stop and retain a failed
point on any setup, admission, lifecycle, resource, latency or evidence error;
do not widen a running limit or automatically retry uncertain effects.

The operator installed build-essential, pkg-config and libssl-dev inside the
new disposable guest, then copied the frozen source, toolchain, dependency
cache and build artifacts into the same absolute paths. This copied no
project credentials or existing estate and changed no host package or
permission. The transfer completed in 86.726 seconds. Source remained clean
at `c206c270ee5a2d1a3e07f7ffe6e2d4ec0466be61`, tree
`e7c5503e59f385d2eda302d22b95bc3f879e9f3b`. Cargo runs offline with the
copied locked inputs. `cargo test --release --locked -p rrflow-cli --test
installed_reopen --no-run` passed; the ordinary primary remained byte-identical:
`5c460fe660bac0c4a11957e5b35bc03ad742ed7f635a932f314674f01c1d5ebc`.
The harness SHA-256 is
`62b7efd23251c364639f7468e46a2c58bfbd52b60f0bead3db5d18a15c4d9e42`.

Bootstrap used the installed primary to preview/apply a fresh estate, start
its loopback service, authenticate and collect the public storage observation.
The installed configuration remains
`2d382dbde4b923db65fb5ebbf841d12b45ec0e278c7cab41635dbff782072d55`.
Bootstrap credentials stay private. The operator helper only adjusts its
local source path and reported KVM/ext4 environment; canonical installation,
campaign validation and execution remain in repository code.

`cargo run --quiet --locked -p rrflow-eval -- storage campaign validate`
accepted purpose `Qualification` and campaign identity
`d6d3ff1fafbb4b48dad9ef0bced117bab396f87c94400b4e539cc692206af4de`.
The frozen input file SHA-256 is
`6b04f1f374f970516a272000f9c1ca609c29794f2808b83ac5ec8b4492c2ac33`.
Admission is not execution proof. The frozen nominal point has:

- four vCPUs, eight GiB provisioned host RAM and a two-GiB aggregate
  harness/engine cgroup with swap disabled;
- 100000 records, 500000 relationships, 100000 384-dimensional vectors and
  102400000 logical base-payload bytes;
- four clients at 25 requests/second, seeds 7, 13 and 29, zero allowed errors
  and the existing 500-ms point ceiling plus qualification-specific oracles;
- two-hour setup, 30-minute lifecycle and 30-second operation bounds;
- 100 forced reopens per seed, all five existing lifecycle lanes, five-minute
  warmup, 30-minute measurement, 60-second 2x burst, five-minute drain and
  24-hour soak per seed;
- one-second samples, 64-GiB disk budget and 16-GiB total evidence allowance,
  with 200 attempts/eight MiB per segment, 100000 segments and 256 samples per
  segment. These finite values were frozen before launch, not changed mid-run.

User service `rrflow-c07-desktop26.service` started in the guest at
2026-10-24 01:40:17 UTC. It runs the existing
`scripts/ci/run_sustained_capacity.py` with an explicit delegated cgroup
parent and a private estate temporary directory. Its outer limit is 307080
seconds, derived from the complete three-seed schedule plus cleanup margin;
the canonical launcher retains its shorter internal deadlines. The service
does not restart a failed campaign automatically. Guest environment: Linux
7.0.0-38-generic, ext4 `/dev/vda1`, GLIBC 2.43, rustc 1.99.1, and the reported
Intel Core Ultra 9 275HX CPU. The guest shares the physical Windows/WSL host
with Linux25; overlapping transfer, setup and workload activity is not an
isolated physical-host performance comparison.

At 01:40:49 UTC, the service was active and the first seed had 960 corpus
acknowledgements, last ordinal 959. The actual cgroup contained harness and
engine processes and read back `memory.max=2147483648`, `memory.swap.max=0`,
`cpu.max=400000 100000`; current/peak aggregate memory was
313589760/361127936 bytes at that observation. The installed driver also
verifies process membership and the allocation receipt. This is live setup
evidence, not complete corpus/readback, completed lifecycle or soak evidence.
The 24-hour phase had not begun. Linux25 separately remained in ingestion,
with 29312 acknowledged records at an earlier observation.

Private operator state is under
`/home/devcontainers/rrflow-c07-native-79663cdd/desktop-qualification-26/`
inside the VM. Its `evidence/` directory, frozen campaign, service-launch
receipt and sanitized observations are separate from bootstrap credentials.
Operator wrappers are `target/stage_c07_guest_workspace.py`,
`target/prepare_c07_guest_campaign.py`, `target/configure_c07_guest_campaign.py`,
`target/launch_c07_guest_campaign.py` and `target/inspect_c07_vm_campaign.py`.
Read their complete contents, the existing bootstrap helper, corpus seed and
independent readback, capacity-profile validator, native allocation verifier,
canonical launcher and prior journals; reviewed the sustained schedule and
identity admission, cold-start boundary, J-04 projection and C-07 owners.
No new upstream adaptation or web research was needed. Preserve existing
bounded public diagnostics and raw failures; no new engine trace was added.

Only this journal, its POAM-035 link and generated navigation/inventory are
changed. There is no implementation move, deletion or replacement. The change
checklist binds the objective, baseline, exact scope, native failure oracle,
provenance, resource observations and stop conditions. Unchanged Rust suites
are not rerun for evidence-only edits; the harness no-run command verifies
staging, not correctness. Full campaign/offline verification, remaining load
sweep, native CI and other C-07 profiles remain outstanding. No OBJ-02,
C-07 or POAM-035 closure, completed soak, alpha status or PR merge is claimed.

Full changed records and diff were reviewed. `python
scripts/knowledge/render_navigation.py` passed (17 indexes, 491 nodes, 2047
edges); `python scripts/ci/build_execution_inventory.py` and `--check` passed
(1582 records); `python scripts/ci/check_documentation.py` passed (492
statuses, 490 coordinates); `git diff --check` passed. These validate the
evidence links and inventory, not the running native campaign. A later
completed result or failure requires a new immutable journal.
