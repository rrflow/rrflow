# Authored Connectome optional source integration

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/authored-connectome-optional-source-journal`
**Owner:** evidence for the optional authored client-source boundary; no engine or client qualification

## Package and baseline

This package advances an OBJ-09 / H-06 source-integration prerequisite while
preserving OBJ-07 / D-01 standalone engine operation. It completes no alpha
outcome or release-roadmap gate.

The starting parent revision is
`470594178d25f1bf12d3ec6d9041488817a9e931` on
`agent/connectome-subrepo`. The existing index already selected the owner's
authored Connectome UI at gitlink
`abc549cf7bb61f2ed4a20353ccb2e3621da515c9`, from
`https://github.com/rrflow/connectome-development.git`, branch
`agent/layman-lenses-wip`. This package does not change that gitlink or
`.gitmodules`, rewrite client history, or modify the separate original client
checkout. The child source-correction package has its own review boundary.

The prepared change brief lives in the local task's ignored tooling directory
for transfer to the parent pull request. No pull request, candidate-CI run,
commit, push, or merge is claimed by this record. The repository's normal
pull-request and passing candidate-CI landing rule remains required.

## Change brief and traceability

Previously, inventory treated the tracked client gitlink as ordinary file
bytes, so an absent or directory client checkout could not be inventoried.
The architecture source-closure guard rejected every gitlink. The owner's
explicit instruction requires the authored client to remain a separate
repository while developing it beside RRFlow.

The restored bounded patch makes one exception for the exact root Connectome
path, repository, and authored branch. Inventory records its indexed commit
without reading client contents. Engine Cargo sources, workspace membership,
path dependencies, and tracked source or asset symlinks cannot import that
optional checkout. Other gitlinks, custom update commands, duplicated config
entries, missing declarations, unmerged pins, and another branch fail closed.

| Current boundary | Canonical destination and preserved behavior | Replacement evidence |
|---|---|---|
| `git_files`, `current_record`, `records`, `validate` | Existing execution-inventory generator; ordinary files retain their byte/digest records, the optional client contributes exactly one gitlink record. | Six Python regressions against real temporary Git indexes and declaration files; real parent inventory validation. |
| `workspace_source_inputs_are_repository_closed`, `git_index_entries` | Existing engine architecture suite; only the canonical optional client pin is allowed, with no Cargo or tracked-symlink source coupling. | Both new policy regressions plus all 29 existing architecture cases pass in the standalone source harness. |
| Product/source/version documentation and CI | Existing owners retain engine independence, exact client branch, separate release/version control, and the future browser-delivery boundary. | Workflow, documentation, version, and Rust formatting checks pass. |

Exact authored scope: `.github/workflows/ci-reusable.yml`, `AGENTS.md`,
`README.md`,
`crates/authority/rrd-engine/tests/workspace_architecture.rs`,
`docs/architecture/system-overview.md`,
`docs/reference/client/connectome.md`,
`docs/reference/release/version-policy.md`, and
`scripts/ci/build_execution_inventory.py`; create
`scripts/ci/test_execution_inventory.py` and this record. Generated navigation
and execution-inventory publication is coordinated by the parent task after
the final authored file set is stable.

No engine runtime, Cargo dependency, public operation, frozen product version,
budget, default distribution, client pin, or roadmap status changes. Stop
conditions remain any second engine authority, required client checkout,
automatic fetch, additional submodule/repository/path/branch, or claim that a
desktop build proves authenticated client/cloud conformance.

## Review and research decisions

Reviewed owning records: `README.md`, `AGENTS.md`, the alpha objective's
OBJ-07/OBJ-09, Gate H's H-06, the change-authoring and structural-traceability
procedures, the independent-engine/project-integration reference, system
overview, Connectome contract, and version policy. Reviewed the saved
`undone-connectome-integration.patch`, existing CI workflow, inventory
generator's complete changed functions, architecture guard's complete changed
functions and policy tests, new Python regression file, and complete package
diff. No broad unrelated working-tree restoration was performed.

Research uses those repository owners, the prior bounded patch, and real Git
index/config behavior. No external source implementation or new dependency
is adapted. Existing diagnostics are preserved; runtime traces and resource
benchmarks are not applicable to this metadata/test/documentation package.
Existing changed files retain their CRLF line endings.

## Commands and results

- `python scripts/ci/test_execution_inventory.py`: **6 passed**. Tests cover an
  engine-only checkout; absent versus populated client checkout with identical
  rendered evidence and prohibited child reads; pin-evidence tampering;
  additional and unmerged gitlinks; exact repository/path/authored-branch and
  update-policy rejection; missing and orphan declarations.
- Real parent `records()` followed by `validate(rows)`: **passed**, 1,184
  records before this journal was added, with exactly the indexed
  `abc549cf7bb61f2ed4a20353ccb2e3621da515c9` optional-client record and no child
  source enumeration. Final generated inventory is a later coordinated step.
- `cargo fmt --all -- --check`: **passed**.
- `python scripts/ci/check_workflow.py`: **passed**; three workflows, seven
  substantive jobs, five cohesive suites, twenty default-feature packages,
  six optional-feature packages.
- `python scripts/ci/check_documentation.py`: **passed before this journal was
  added**; 288 document statuses and 286 classified coordinates. Rerun after
  generated navigation includes this record.
- Bundled Python running `scripts/check_version.py`: **passed**, frozen
  release train `1.0.0`. The shell-default Conda Python first failed importing
  its broken `pyexpat` DLL; this was an interpreter environment failure.
- `cargo test -p rrd-engine --test workspace_architecture --locked`: **not
  completed**. The sandbox initially denied the configured compiler wrapper;
  an elevated retry began a cold Arrow/DataFusion build and was interrupted
  to preserve disk space. No Cargo-suite pass is claimed.
- Focused source-only alternative: set `CARGO_MANIFEST_DIR` to the existing
  `crates/authority/rrd-engine` directory; run `rustc --edition=2024 --test
  crates/authority/rrd-engine/tests/workspace_architecture.rs --extern
  serde_json=connectome/src-tauri/target/debug/deps/libserde_json-c57e1f8763efd716.rlib
  -L dependency=connectome/src-tauri/target/debug/deps -C debuginfo=0 -o
  .tools/architecture_check.exe`, then execute that binary: **31 passed, zero
  failed**. This compiles the actual unchanged test source against an already
  cached library and exercises the real parent Git index and Cargo metadata;
  the cache location is local verification tooling, never an engine build or
  runtime dependency.

The two new architecture cases prove canonical gitlink policy and deny parent
file/directory symlink imports of client schemas, templates, configuration,
or assets while retaining internal repository symlinks and rejecting escapes.
The existing source-closure case also passes against the actual metadata.

## Integrated parent-task smoke evidence

The coordinating parent task subsequently exercised the actual corrected
desktop UI based on the owner's `abc549cf` source, with the RRFlow engine built
at `4705941`. This is a local working-tree smoke result, not a clean released
artifact or complete H-06 conformance result:

- Installation applied the sealed engine-issued plan, and serving started at
  `http://127.0.0.1:60410` as
  `instance-3a9cd51750f9564607e67699`.
- The engine answered authenticated readiness. The authored Connectome UI
  opened a session as `local-operator`, rendered **Records & relations** at
  cursor 2, and showed all three engine-identity cards as **VERIFIED**.
- The estate was stopped through the UI, the desktop application closed, and
  development process 31300 exited with status 0.

No legacy form, unavailable engine operation, full multi-model corpus, remote
transport, or cloud deployment is inferred from that successful local path.
The parent task owns the child implementation/build/test evidence and its
final source/artifact digests.

## Remaining evidence and status

### Owner-directed domain and learning identity clarification

During this package the owner clarified control of both `rrflow.ai` and
`rroflow.ai`: RRFlow is the reasoning engine; RROFLOW names its
recursive-learning aspects. The README now states that identity and links to
the existing system-overview owner, whose canonical terminology distinguishes
the two domain presentations under one `RrdEngine` authority. Connectome is
the shared UI; RRO retains its existing Reason Ready Object meaning.

The system overview records the owner's intended Raindrop.ai study as
research direction for effortless RRFlow configuration and extension. No
third-party service implementation was inspected or imported in this
documentation-only subtask, and no Raindrop feature, new engine, learning
workflow, or cloud deployment is claimed as implemented. Runtime source,
contracts, budgets, versions, and roadmap status remain unchanged.

After this clarification, `python scripts/ci/check_documentation.py` passed
with 289 document statuses and 287 classified coordinates; scoped
`git diff --check` passed. No runtime test is claimed for a prose-only
identity clarification.

The full engine suite, clean candidate CI, installed client conformance,
browser transport/authentication, cloud deployment, and release assembly are
not run or qualified by this package. H-06 and all existing POA&M outcomes
remain unchanged; no new product deficiency is inferred from the local Python
or disk environment. Parent tooling regeneration, final changed-file review,
and candidate-CI evidence remain the landing prerequisites.

The interrupted cold parent Cargo build's generated `target/debug` directory
was removed only after verifying its exact containment under the parent
`target` and observing no remaining Cargo or Rust compiler process. The
separate child target, saved engine, standalone architecture harness, and
other local tooling were preserved. This local cleanup is not a product or
source change.

The change checklist has an owning outcome, bounded effect scope, source and
test map, real failure oracle, preserved authority/budget/version decisions,
research/trace decisions, surfaced failures, exact results, and explicit
unrun checks. It is not a roadmap-completion receipt.

## Links

- [Connectome contract](../../../reference/client/connectome.md)
- [Gate H](../../../roadmap/rrflow-1.0/gate-h.md)
- [Change-authoring procedure](../../../roadmap/rrflow-1.0-execution/change-authoring.md)
- [Implementation traceability](../../../roadmap/rrflow-1.0-execution/implementation-navigation.md#traceability-before-structural-change)
