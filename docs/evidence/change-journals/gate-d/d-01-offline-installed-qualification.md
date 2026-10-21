# D-01 standalone and offline installed qualification

**Status:** active bounded implementation and qualification evidence; alpha remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-offline-installed-qualification`
**Owner:** OBJ-07, D-01, POAM-006 and POAM-013

## Baseline and change brief

Baseline `79cced3c311b631ca58b5aff33beba0393f9d802`, tree
`9d1a008c7747e25ec543c748b9856f00bd16cbf0`, clean public development main.
[PR 43](https://github.com/rrflow/rrflow-development/pull/43) carried the brief
before repository edits. Empty anchor
`63a3796114c9e10e167c358c2ccb8a0dc85d9c5b` opened the PR; its superseded
workflow `39526682496` was cancelled, not counted as qualification.

Current behavior executed the actual primary binary directly from Cargo's
target tree with an inherited developer environment. Target behavior stages
the same executable outside the checkout, preserves the full existing
fresh/existing installed-process corpus, and explicitly qualifies Linux without
checkout/sibling/toolchain source trees or outbound network. The owning D-01
chapter now enumerates remaining acceptance families; it does not create a
second alpha plan or advance the dependency spine.

## Traceability and implementation

`crates/adapters/rrflow-cli/tests/installed_lifecycle.rs` replaces only the
test's executable selection and child command construction with
`StandaloneDistribution`, `binary` and `product_command`. One thread-owned
temporary distribution retains the copied binary until child handles are
reaped. Each product command has an empty working directory and PATH,
isolated home/temp directories and only necessary Windows OS-discovery
variables inherited. All existing commit/query, budget/authentication denial,
restart, Parquet, backup, restore and secret-redaction assertions remain.

New `scripts/ci/run_offline_lifecycle.py` builds before isolation, selects the
exact Cargo executable artifacts, verifies copied hashes, and runs the same
harness inside private Linux mount/network/PID namespaces. It masks source
roots only inside the private namespace, validates loopback-only interfaces,
absent default routes and outbound `ENETUNREACH`, drops privileges before
product execution and requires both real-process cases to exist. Missing
isolation fails. Privileged timeout/PID-namespace teardown bounds orphaned
children. Ordinary operating-system libraries remain available; this is not
a hostile-program sandbox or an installer.

New `scripts/ci/test_offline_lifecycle.py` rejects missing/ambiguous artifacts,
empty/partial test corpora, unsafe or payload-overlapping mask roots and
unchanged namespaces before mount operations. It checks nested-mask reduction
and payload-hash sensitivity. `.github/workflows/ci-reusable.yml` adds the
Linux qualification and Python tests; `scripts/ci/check_workflow.py` requires
them. `docs/operations/ci.md`, Gate D and POAM-006 describe the exact boundary.
POAM-020 records the independently reproduced installation identity collision.
Generated journal navigation, relationship projection and execution inventory
are regenerated. No runtime source, public contract, format, version,
dependency, submodule or existing test assertion is removed or replaced.

## Inspection and engineering decisions

Reviewed README/AGENTS, alpha objective, dependency spine, Gate D/evidence,
POAM-006/-020, installed-lifecycle and independent-engine bootstrap contracts,
change-authoring/implementation-navigation procedures and prior native
lifecycle evidence. Reviewed the lifecycle test, CLI installed command and
build-identity composition, workflow and workflow-policy sources. Inspected
engine installation identity derivation, precondition inventory, layout,
credential and recovery boundaries to distinguish outstanding runtime gaps
from a missing test. New and modified implementation sections and the complete
diff were reread before submission.

Primary references: [util-linux unshare](https://man7.org/linux/man-pages/man1/unshare.1.html),
[Linux mount namespaces](https://man7.org/linux/man-pages/man7/mount_namespaces.7.html),
[Python process execution](https://docs.python.org/3/library/subprocess.html)
and [OS identity operations](https://docs.python.org/3.11/library/os.html).
They establish private mount propagation, namespace teardown and privilege
handling; no upstream code is adapted. Existing engine receipts and exact
query/restore oracles remain authoritative. Harness isolation facts and hashes
are qualification evidence, not a new engine trace or lifecycle authority.
No latency, throughput, RSS or competitive-performance claim is made.

Stop conditions: failed isolation, unexpected source/runtime dependency,
artifact mismatch, weakened corpus, unrelated production change, unexpected
project mutation or failed candidate CI. No condition may be converted into
a successful skip.

## Local verification and observed failures

Windows, pinned Rust 1.99.1. Focused first check:
`cargo test --locked -p rrflow-cli --test installed_lifecycle
primary_help_exposes_only_the_installed_product_lifecycle -- --nocapture`
passed one test, four filtered, using a copied binary and clean environment.
`cargo clippy --locked -p rrflow-cli --test installed_lifecycle -- -D warnings`
passed. `python scripts/ci/test_offline_lifecycle.py` passed six tests.
`cargo test --locked -p rrflow-cli --test installed_lifecycle -- --nocapture`
passed all five tests (zero failed/ignored/filtered) in 206.24 seconds. The
staged executable was 155,393,024 bytes. `cargo test --locked -p rrflow-cli` passed 16 tests
across 9 result groups, zero failures/ignored tests; its lifecycle cases
passed in 206.13 seconds. Logs are retained locally under
`target/offline-alpha-native.log` and `target/offline-alpha-package.log`.

Workflow policy accepted the current workflow and rejected in-memory variants
removing either new offline command. Four execution-inventory tests and
17 knowledge/navigation tests passed. Formatting, generated navigation and
inventory, documentation and workflow checks passed after regeneration.

The identity observation ran the already available native executable against
two newly created empty temporary directories. Both previews returned the
same target and plan; executable SHA-256
`6853715de7b816f7e9c1e921ceb670f6350bc2fb0d088e0d5cb30df6fb7d0623`.
No installation was applied. POAM-020 and the D-01 closure package record the
defect; this change does not repair it. An initial exploratory source-path
guess was absent and was resolved with the repository file list. PowerShell
rendered Cargo stderr as `NativeCommandError` in the redirected local log;
actual test/command exit codes, not that wrapper label, determine results.
The first documentation check rejected this record because its status omitted
the required active/historical classification; the status was corrected.
The subsequent check correctly rejected the stale generated journal index;
regenerating navigation and inventory restored documentation-policy success.

## Checklist, limits and landing evidence

Change brief and owner: recorded before implementation. Scope/replacement map:
above. Research, trace/resource decisions and stop conditions: above.
Verification starts with the copied primary executable and widens to its
package and candidate CI. Read/diff review, generated evidence, failed checks
and known gaps are recorded here. Before each push the development URL, exact
source revision and destination feature ref are recorded; no official-repo or
direct-main push is permitted.

Linux namespace execution and macOS staging cannot be claimed from the Windows
local run; they require the exact candidate's CI. Offline Windows/macOS,
hostile filesystem/ACL cases, independent project identities, complete
bootstrap preview, C-07 storage lifetime, attunement execution, model routing,
durable reasoning, PostgreSQL compatibility and signed release remain open.
No objective, gate, POA&M closure, version or maturity status changes. Candidate
CI and merge evidence attach to PR 43 after all required partitions pass.
