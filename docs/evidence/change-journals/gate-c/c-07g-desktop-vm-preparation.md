# C-07g: isolated Desktop host preparation

**Status:** active prerequisite evidence; no guest workload or soak result
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-desktop-vm-preparation`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [111](https://github.com/rrflow/rrflow-development/pull/111)

Continuation starts at `c7959ca5989b6bd6ef3255c4c10e41cdd348e24c`, tree
`a70fd8ed8e764478c08daa4652a3182cd435d518`. PR111's extended brief preceded
these repository evidence edits. Its original [launch receipt](c-07g-desktop-preflight-launch.md)
remains unchanged. This record prepares the fixed Desktop host prerequisite;
it changes no engine source, workload policy, threshold, lifecycle or default
dependency. Stop on image verification, KVM, guest compatibility or allocation
failure rather than relabeling the existing oversized WSL host.

The WSL host exposes accessible `/dev/kvm`; its user lacks noninteractive sudo
and Windows Hyper-V inventory permission. No host permission or installation
was changed. `apt-get --simulate --no-install-recommends install
qemu-system-x86 qemu-utils cloud-image-utils` resolved 29 missing packages.
`apt download` fetched their exact versions from configured Ubuntu archives;
`dpkg-deb -x` extracted them into an operator-owned directory. QEMU reports
`10.2.1 (Debian 1:10.2.1+ds-1ubuntu3.3)`. Package names, versions and SHA-256
digests are retained in `desktop-vm-tools/package-receipt.json` beneath
`/home/devcontainers/rrflow-c07-native-79663cdd/`. This is laboratory host
preparation, not an RRFlow install-time fetch or distribution dependency.

The official [Ubuntu 26.04 cloud-image catalogue](https://cloud-images.ubuntu.com/releases/resolute/release/)
supplied `ubuntu-26.04-server-cloudimg-amd64.img`. `gpgv --keyring
/usr/share/keyrings/ubuntu-cloudimage-keyring.gpg SHA256SUMS.gpg SHA256SUMS`
verified the checksum manifest against the installed Ubuntu cloud-image
keyring. The image's 859090944 bytes hash to
`5645d0318ad21f643ebb1ac4567514620e9893380b07b0201fda163baa5b7eb5`, matching
that signed manifest. No upstream engine implementation was adapted. The
chosen guest supplies GLIBC 2.43, required by the frozen ordinary primary.

Owned user service `rrflow-c07-desktop-vm.service` runs KVM with four vCPUs,
8192 MiB provisioned guest RAM, a 96 GiB copy-on-write guest disk, virtio
devices and SSH forwarded only on `127.0.0.1:22225`. The outer service caps
QEMU at ten GiB with no swap and 400% CPU; that allowance includes emulator
overhead and does not replace the guest's eight-GiB limit. Its finite runtime
is seven days. SSH uses a private operator key and guest passwords/root SSH
are disabled. Private keys, cloud-init seed and bootstrap material must not
enter retained public evidence. The guest is disposable; it contains no user
project and shares no writable project mount.

The first SSH attempt timed out during initial boot. After cloud-init finished,
SSH and `cloud-init status` passed. An initial `--version` invocation was
rejected by the existing command parser; the documented `version` subcommand
then succeeded. No parser compatibility alias was added. The frozen PR110
primary copied into the guest retains SHA-256
`5c460fe660bac0c4a11957e5b35bc03ad742ed7f635a932f314674f01c1d5ebc` and reports
`RRFlow 1.0.0 (pre-release; rrd protocol v1)`. Help exposes the installed
product lifecycle. These prove executable compatibility, not installation or
durable workload correctness in this guest.

The guest receipt records four CPUs, 8321495040 bytes of OS-visible memory
after kernel reservations and 96551845888 bytes available on its root
filesystem. An empty, uniquely owned child of the delegated user cgroup
accepted and read back `memory.max=2147483648`, `memory.swap.max=0`,
`memory.oom.group=1`, `pids.max=512`, `cpu.max=400000 100000`; `cgroup.kill`
exists. The empty probe was removed. No process was placed in that probe,
so this checks launcher prerequisites, not workload enforcement or OOM behavior.

Operator scripts are `target/prepare_c07_vm_tools.py`,
`target/prepare_c07_vm_image.py`, `target/start_c07_desktop_vm.py` and
`target/inspect_c07_guest.py`. Their complete contents were reviewed. Private
operator state remains in `desktop-vm/`; safe image, launch and guest receipts
are separate from credentials. Preserve the existing native launcher and
bounded public resource observations; no new engine tracing is needed for
this host-only operation. Owner review follows README, OBJ-02, C-07's workload
requirements, J-04's allocation/duration table, POAM-035, change-authoring and
CI procedures, the preceding launch journal and canonical
`scripts/ci/run_sustained_capacity.py` allocation/launch path.

Linux25 remains frozen on PR110 in its original WSL service and source/build
directories. A later observation contained 24448 corpus acknowledgements
(last ordinal 24447); ingestion was still running. Guest boot/copy overlaps
that shared-host characterization and must not be described as an isolated
performance comparison. There is no completed Linux25 result, full readback
or guest qualification in this record. The guest's actual installation,
frozen campaign, full corpus, three seeds, 100 reopens, profile sweep and
24-hour results remain required. C-07, OBJ-02 and POAM-035 stay open.

Only this new journal, the POAM-035 link and generated navigation/inventory
change. No implementation is deleted, moved or consolidated. The checklist
binds the existing outcome, exact evidence scope, native prerequisite oracle,
research provenance, resource measurements, failures and stop conditions.
Unchanged Rust suites are not rerun for evidence-only edits. Full changed
records and diff were reviewed. `python scripts/knowledge/render_navigation.py`
passed (17 indexes, 490 nodes, 2043 edges),
`python scripts/ci/build_execution_inventory.py` and its `--check` passed
(1581 records), `python scripts/ci/check_documentation.py` passed (491
statuses, 489 coordinates), and `git diff --check` passed. Exact candidate CI
remains required before merge. No PR111 merge is claimed.
