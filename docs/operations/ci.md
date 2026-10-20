# RRFlow CI and runner operations

**Status:** active operational contract; current checks are not RRFlow alpha or release evidence
**Coordinate:** `rrflow://rrflow-instance/data/operations/ci`
**Owner:** candidate CI topology, workflow supply-chain policy, diagnostic workflow boundary, and private runner procedure; indexed by `docs/operations/README.md`

This record describes checked-in automation and the operator steps around it.
The [release roadmap](../roadmap/rrflow-1.0.md) alone owns gate status, the
[repository run checklist](../roadmap/rrflow-1.0-execution-map.md#repository-wide-run-checklist)
defines risk-based package widening, and the [POA&M](../poam/rrflow-1.0-alpha.md)
owns deficiencies. A green workflow proves only the commands and assertions it
actually executes.

## Candidate chain

RRFlow has one candidate CI chain. The thin caller is
`.github/workflows/ci.yml`; `.github/workflows/ci-reusable.yml` owns every
check. A branch push does not start a second copy of the same work.

The caller accepts pull requests, merge-queue candidates, and explicit manual
dispatches. Its concurrency key cancels stale executions for the same pull
request or ref. The reusable workflow reduces every partition into one stable
`ci-gate` check for branch protection:

- supervised RRD daemon smoke on the standard Linux runner;
- repository policy, formatting, architecture, evaluation, dependency, and
  binary-budget gates on a heavy Linux runner;
- five default-feature engine suites on isolated heavy Linux runners:
  `kernel-storage`, `query-retrieval`, `engine-authority`, `service-protocol`,
  and `product-operations`;
- one current shared SDK harness, isolated optional-feature checks, and a real
  PostgreSQL/pgvector integration;
- daemon portability on GitHub-hosted Windows and macOS runners; and
- one always-evaluated gate which fails unless every partition succeeded.

The topology job starts one local RRD daemon and checks readiness and
capabilities. It does not start Connectome or prove installation, rrflowMX and
rrflowKV semantic equivalence, native graph/BM25/vector projections, streamed
Arrow/DataFusion execution, persistent reasoning/context, or restart recovery.
The shared SDK harness is also current characterization: the deficiencies and
real-process corpus required for H-04 remain open. Neither job may be cited as
complete-engine evidence.

The repository-policy partition also verifies the generated
`docs/roadmap/rrflow-1.0-file-plan.jsonl`. That inventory must match every
tracked or non-ignored path and every long-lived future path registered by its
generator. Immediate absent paths are authorized by the earlier active change
package and become current inventory records when created. Regenerate the
inventory with `python3 scripts/ci/build_execution_inventory.py` after an
accepted path or content change.

The Connectome UI is optional for engine operation, while candidate CI follows
repository policy and recursively materializes every declared submodule. The
Connectome development repository is public and can be acquired anonymously.
The [publication receipt](../evidence/change-journals/gate-h/h-06-connectome-public-association.md)
records successful recursive checkout after the visibility change; the full
candidate pipeline remains required before merge.
The architecture suite checks the exact Connectome source identity and denies
engine dependencies or targets inside it, alongside the general pinned-source
validator. The inventory records the parent gitlink without traversing child
files. `python3 scripts/ci/test_execution_inventory.py` proves parity across
absent/populated/edited child files, pin-sensitive identity, and conflict
rejection. UI builds and real-engine conformance remain separate H-06 evidence.

Marked documentation indexes are derived from their immediate coordinated
children. Run `python3 scripts/knowledge/render_navigation.py` after adding,
moving, or removing a child record; `--check` is non-mutating and fails on
drift. The same tool derives the coordinate/link relationship graph used for
future knowledge import and retrieval. Generated listings and edges are
navigation, not documentation or lifecycle authority.

The same partition enforces the documentation bootstrap package. The
documentation policy builds and validates two in-memory packages through the
single exporter, while the workflow exports twice at the checked-out revision,
compares the exact JSONL bytes, and runs the focused drift corpus. Package
outputs remain temporary and are never editable documentation authority.

The five engine suites cover every workspace package exactly once, but package
names do not define product ownership. The partitioning avoids one
workspace-wide test/link target graph on the hosted fallback without turning
all 20 implementation packages into separate product checks.

## Presubmit and change-evidence boundary

Repository presubmit is not an RRFlow runtime hook. It cannot create an engine
event, trigger a persisted routine, resolve a skill, mutate an estate, or claim
that an effect completed. Gate I owns those future runtime capabilities.

The portable development control has three layers:

1. the checked-in commands and policy scripts are the reproducible presubmit;
2. candidate CI runs those exact controls and reduces them to
   `pipeline / ci-gate`; and
3. repository branch policy makes the pull request and stable check
   non-bypassable when the GitHub account tier supports that control.

Every code-bearing or structural package lands as one pull request against
`main`. Its description carries the change brief defined by the
[change-authoring procedure](../roadmap/rrflow-1.0-execution/change-authoring.md#required-change-checklist).
Candidate CI runs on that pull request and must pass before merge; a direct
push to `main` is not a landing path. The pull request description begins at
the repository-effect boundary. It is not a workflow for exploratory reading or
private AI reasoning and must not contain hidden chain-of-thought.

The planning-only commit, `docs/roadmap/rrflow-1.0-active-change.json`, and
`scripts/ci/check_change_plan.py` were retired on 2026-10-16. Between
2026-08-20 and 2026-09-15, 442 commits reached `main` without a pull request;
candidate CI triggers only on `pull_request`, `merge_group`, and
`workflow_dispatch`, so none of them ran it automatically, and the per-change
validator passed at `cfa0e80` while the generated-surface, backup-fixture, and
edge binary-ceiling checks failed there. The active-change file stays unchanged
as historical evidence linked from earlier journals; nothing reads or updates
it.

Candidate CI proves the checked-in controls, not algorithm correctness or alpha
readiness. Only the separately evidenced server-side protection below can make
the pull request and required candidate check non-bypassable.

A developer may run the
[repository checklist](../roadmap/rrflow-1.0-execution-map.md#repository-wide-run-checklist)
locally before publishing, but the local run is convenience and early feedback,
not authority. RRFlow deliberately has no installed Git/editor/provider hook:
client Git hooks are not distributed by clone, can be skipped, and would create
the exact provider-owned lifecycle behavior prohibited by `AGENTS.md`.

The documentation policy machine-checks the owner chain, required
change-authoring procedure/evidence fields, generated-index freshness,
active-record classification, knowledge-package determinism, and local links.
The execution inventory checks every current/generated/registered-planned
path. Each package records its result in a linked record under
`docs/evidence/change-journals/`; the execution portal contains no journal
bodies. These checked-in controls remain bypassable outside candidate CI;
POAM-027 stays open for eligible server-side review/check/force-push/deletion
enforcement.

## Local development container

`.devcontainer/` reproduces the candidate-CI toolchain on any host that runs
rootless Podman (or Docker) on amd64 Linux, including Podman machine on
Windows. It is local reproduction and early feedback only: a container run is
not candidate-CI, J-02, POAM-027, alpha, or release evidence, it is not a
hook, and it declares no editor lifecycle command.

| Target | Contents | Use |
|---|---|---|
| `rust` | snapshot-pinned Ubuntu 24.04 build dependencies, rustup 1.29.1, Rust from `rust-toolchain.toml`, uv, and the `actions/python-versions` CPython build that `setup-python` installs as `python3` | engine work, policy scripts, focused tests; the VS Code default |
| `full` | `rust` plus Node, pnpm, Go, Temurin, Maven, and the .NET SDK | TypeScript/SDK work and `scripts/ci/run_sdk_conformance.py` |

Every base image is digest pinned, every apt package resolves from one fixed
Ubuntu snapshot, and every downloaded toolchain is verified with
`sha256sum`/`sha512sum --check --strict` against a checksum in the
Containerfile. Certificate bootstrap extracts trust roots from an Apt-authenticated
package without installing a moving OpenSSL dependency; installed packages
then resolve from the fixed snapshot. The Temurin download tag is recorded
separately from the equivalent SemVer selector used by `setup-java`.
`scripts/ci/check_workflow.py` fails when a container version,
checksum, or image digest diverges from `rust-toolchain.toml` or
`.github/workflows/ci-reusable.yml`, or when `devcontainer.json` declares a
lifecycle command. Build steps are POSIX `sh` because Podman's default OCI
image format ignores `SHELL`.

CI and the container track the latest stable release of every toolchain,
using the long-term-support line where a project publishes one (Node.js 24,
Java 25, .NET 10) and the Ubuntu release that GitHub's `ubuntu-latest` runner
uses (24.04). A toolchain upgrade changes `.github/workflows/ci-reusable.yml`,
`rust-toolchain.toml` where applicable, and the Containerfile in the same pull
request; the parity check rejects one without the other.

Keep the source on a Linux filesystem; a Windows bind mount is slow and
reintroduces executable-bit and line-ending drift. Two entry points share the
`rrflow-cache` volume (Cargo registry, target directory, and language caches):

- **VS Code:** set `dev.containers.dockerPath` to `podman`, then run
  *Dev Containers: Clone Repository in Named Container Volume* for the
  development repository. VS Code builds the `rust` target.
- **Podman CLI:** from an existing checkout, with a compose provider such as
  `podman-compose` on `PATH`:

```bash
podman compose -f .devcontainer/compose.yaml build
podman compose -f .devcontainer/compose.yaml run --rm workspace \
  /seed/.devcontainer/seed-workspace.sh
podman compose -f .devcontainer/compose.yaml run --rm workspace
```

The seed script clones committed history from the read-only `/seed` mount of
the invoking checkout into the `rrflow-src` volume, restores its remote names,
URLs, and upstream, and then initializes every declared submodule (such as
`connectome`) at its pinned gitlink revision, matching a recursive CI checkout.
It copies no credentials, hooks, local Git
configuration, or uncommitted files, and it refuses a non-empty volume. Set
`RRFLOW_DEV_TARGET=full` to build and enter the SDK image.

The optional pgvector service uses the candidate-CI image digest, database,
and credentials on host networking, so the CI test URL works unchanged:

```bash
podman compose -f .devcontainer/compose.yaml --profile pgvector up -d pgvector
RRFLOW_PGVECTOR_TEST_URL='postgresql://rrflow_test:rrflow_test_password@127.0.0.1:5432/rrd_operator_knowledge_test?sslmode=disable' \
  RRFLOW_PGVECTOR_TEST_DISPOSABLE=1 \
  cargo test -p rrd-operator-knowledge --all-targets --all-features --locked
```

Toolchain downloads are development build inputs, like the CI setup actions;
they are never RRFlow install-time or runtime fetches. Only amd64 is
supported; the checked-in digests and checksums name amd64 artifacts.

## Repository workflow policy

`scripts/ci/check_workflow.py` discovers every checked-in `.yml` and `.yaml`
under `.github/workflows/`; a newly added workflow cannot escape its
supply-chain checks. It requires:

- every external action to use a full 40-character commit SHA;
- every workflow service image to use a SHA-256 digest;
- every checkout step to set `persist-credentials: false`, and to set
  `submodules: recursive` whenever `.gitmodules` exists;
- top-level read-only contents permission and no `pull_request_target` in any
  workflow; and
- complete candidate-suite/default-feature/optional-feature coverage, bounded
  candidate jobs, safe fork routing, and exact final-gate reduction; and
- development-container toolchain, checksum, image, and pgvector parity with
  candidate CI, strict download verification, and no editor lifecycle command.

A Git submodule is a pinned source input, not an install-time fetch. The
workspace architecture test accepts a gitlink only when `.gitmodules` declares
it exactly once at a normalized repository-relative path with a
credential-free `https://` URL, so an anonymous clean clone and candidate CI
reproduce the same tree. Cargo workspace members remain tracked in this
repository, and the signed default distribution must still contain every byte
it needs after acquisition.

Workflow ownership is directory-wide in `.github/CODEOWNERS`, so adding a new
workflow does not bypass the existing workflow review boundary. Repository or
organization settings should additionally require full-SHA actions; that
external GitHub setting cannot be proven by this checkout.

## Scheduled storage diagnostics

`.github/workflows/rrd-lsm-benchmark.yml` is separate from candidate CI. It
runs manually and weekly on `ubuntu-latest`, uses the same immutable checkout,
toolchain, and cache action revisions, and uploads raw semantic-storage and
AI-access results with a pinned artifact action. It is intentionally not a
required `ci-gate` dependency.

The [rrflowKV benchmark reference](../reference/storage/rrflowkv-benchmark-harness.md)
owns those executable shapes and their limitations. Hosted runner output lacks
the fixed hardware, complete provenance, full governed engine flow, and
quality/failure corpus required by J-04; scheduling and artifact upload cannot
turn it into performance or competitive evidence.

## Diagnostic, performance, and fault lanes

The
[engine observability contract](../architecture/engine-data-flow.md#runtime-modes-build-profiles-and-build-identity)
defines one semantic engine with release, optimized-diagnostic,
runtime-analysis, sanitizer, and benchmark modes. None of those modes is
currently a qualified RRFlow binary merely because Cargo can compile the
workspace.

Candidate CI stays bounded to deterministic correctness and repository policy.
Additional lanes enter the required gate only with an owning roadmap package,
declared target/support matrix, time/resource budget, retained failure
artifacts, and a false-negative analysis:

| Lane | Required use | Gate/evidence boundary |
|---|---|---|
| optimized diagnostic parity | Prove release and diagnostic profiles have identical feature closure, formats, operations, results, receipts, and resource limits; symbols/configuration may differ. | H-05, J-03, J-05 |
| deterministic fault/reopen | Inject WAL/manifest/segment/page, sync, acknowledgement, process-kill, ENOSPC, and corruption boundaries; verify exact state after reopen from a recorded seed/schedule. | C-07, D-11, J-02 |
| Loom | Explore bounded concurrent schedules around transaction, cache, publication, and cancellation primitives. | owning C-through-I package, J-02 |
| Miri/sanitizers | Detect supported undefined-behavior, address/leak, and race classes on deliberately bounded targets. | owning package, J-02 |
| Tokio Console/profiler | Diagnose scheduler, resource, lock, allocation, CPU, and I/O stalls only after ordinary correlated evidence identifies a reproducible case. | runtime-analysis; never conformance by itself |
| fixed-hardware benchmark | Retain raw histograms, failed samples, quality/correctness results, resource use, build identity, workload, and machine/filesystem/device provenance. | J-04 only |

Hosted runner benchmarks and ad hoc `cargo bench` output remain diagnostics.
They cannot support a latency, scalability, zero-copy, resource, or competitor
claim. The J-04 harness must use the canonical latency boundary and diagnostic
capture bundle, separate success/error and cold/warm behavior, and preserve
p50/p95/p99/p99.9 distributions rather than averages.

## Runner trust boundary

The repository variables below contain JSON `runs-on` values:

| Variable | Physical pool value | Safe default |
| --- | --- | --- |
| `CI_SELF_HOSTED_ENABLED` | `true` | disabled |
| `CI_LINUX_STANDARD_RUNNER` | `["rrflow-standard"]` | `["ubuntu-latest"]` |
| `CI_LINUX_HEAVY_RUNNER` | `["rrflow-heavy"]` | `["ubuntu-latest"]` |

A pull request whose head repository differs from the base repository is
forced onto GitHub-hosted Linux even when the variables exist. Missing or
disabled `CI_SELF_HOSTED_ENABLED` also forces hosted Linux. The workflow does
not use `pull_request_target`. Same-repository branches, merge groups, and
manual proofs may use the private physical pool.

This routing expression is defense in depth, not permission to connect a
self-hosted pool to a public repository: a fork can propose a change to the
workflow itself. The installer queries GitHub and fails closed unless the
repository visibility is `PRIVATE`. Do not set the physical routing variables
or register the scale sets while the repository is public. The repository
owner confirmed public visibility on 2026-10-16, so candidate CI uses
GitHub-hosted runners and the routing variables stay unset.

GitHub recommends ephemeral runners for autoscaling and identifies Actions
Runner Controller (ARC) as its reference Kubernetes implementation. The
checked-in deployment is configured as follows:

- ARC chart version: `0.14.2`; controller, runner, and Docker images are
  digest pinned;
- `rrflow-heavy`: scale 0..1, 48 CPUs and 64 GiB total, isolated Docker daemon;
- `rrflow-standard`: scale 0..2, 16 CPUs and 20 GiB each; and
- maximum runner allocation: 80 CPUs and 104 GiB, leaving 8 CPUs and 24 GiB
  for the 88-CPU/128-GiB host, k3s, ARC, and filesystem cache.

Each job receives a fresh pod and an empty work volume. No runner worktree is
mounted from the host. The runner service account has no Kubernetes API
permissions, and only the heavy class has a privileged Docker sidecar.

References:

- [GitHub: self-hosted runners](https://docs.github.com/en/actions/reference/runners/self-hosted-runners)
- [GitHub: deploy ARC runner scale sets](https://docs.github.com/en/actions/how-tos/manage-runners/use-actions-runner-controller/deploy-runner-scale-sets)
- [GitHub: secure use of self-hosted runners](https://docs.github.com/en/actions/reference/security/secure-use)

## Physical-host bootstrap

Install a pinned single-node k3s `v1.36.3+k3s1` cluster on the physical host
and make its kubeconfig available to the administration machine. Do not share
this cluster with production workloads. Install Helm 4 and authenticate `gh`
as a repository administrator. The checked-in script does not install or
attest those prerequisites. Run it from the target checkout with an explicit
repository identity, or let `gh repo view` resolve the current checkout:

```bash
GITHUB_REPOSITORY=OWNER/REPOSITORY \
  KUBECONFIG=/path/to/physical-host.kubeconfig \
  scripts/ci/install-arc.sh
```

The installer validates an `OWNER/REPOSITORY` coordinate and denies
public-repository registration before creating separate controller and runner
namespaces. It streams the existing GitHub token directly into a Kubernetes
Secret, installs the version-selected OCI charts with digest-pinned images,
waits for the controller, installs both scale sets, and sets the repository
routing variables. It does not write the token to argv or a values file.

The chart fetch, GitHub API calls, and Kubernetes operations require network
and administrator access. This is CI-infrastructure bootstrap, not the RRFlow
offline distribution or per-project installation path, and no checked-in
record currently proves that the physical pool is installed.

Prove physical execution after installation:

```bash
gh workflow run ci.yml --repo OWNER/REPOSITORY \
  --ref <candidate-branch>
gh run watch --repo OWNER/REPOSITORY --exit-status
```

The standard and heavy jobs must report `rrflow-standard` and `rrflow-heavy`
runner identities. Windows, macOS, the final gate, and all fork pull requests
must remain GitHub-hosted.

## Repository enforcement

Public visibility exposes this control on the current hosting tier. Protect
`main` with these invariants:

- pull requests and the stable `pipeline / ci-gate` check are required;
- merge queue or an up-to-date branch is required;
- force pushes and deletion are denied; and
- Actions must require immutable full-SHA references.

This is an external control and needs external evidence. On 2026-09-11,
read-only GitHub API requests for both repository rulesets and `main` branch
protection on the then-private `rrflow/rrflow-development` returned HTTP 403
with the message that GitHub Pro or public visibility is required. The
repository owner confirmed public visibility on 2026-10-16, so rulesets are
available; no ruleset is recorded here yet. Therefore the checkout does
**not** currently claim that `pipeline / ci-gate`, pull requests, reviews,
deletion denial, or force-push denial is enforced. Do not compensate with a
local hook or push to the official promotion repository. Create the `main`
ruleset, then record the exact repository, rule identity, required check name,
protected ref, API response, and verification attempt in this record and one
linked package journal.

The repository CI-policy check rejects candidate trigger duplication, unsafe
fork routing, unpinned actions, service or runner images, persisted checkout
credentials, over-broad workflow permissions, `pull_request_target`, missing
candidate timeouts, incomplete suite or feature coverage, missing gate
reduction, or a runner layout which exceeds the documented host allocation.
The Rust workspace architecture test remains focused on engine ownership,
dependency direction, and storage-opening authority rather than GitHub Actions
implementation details.
