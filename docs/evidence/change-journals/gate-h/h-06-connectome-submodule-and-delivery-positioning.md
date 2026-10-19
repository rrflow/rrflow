# H-06 Connectome submodule and delivery positioning journal

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-06-connectome-submodule-and-delivery-positioning`
**Owner:** evidence for the optional UI source prerequisite; H-06 remains open

## Change brief and baseline

Gate/package and alpha outcome or prerequisite advanced: OBJ-09, H-06/H-07
source integration and delivery positioning; POAM-011 remains open.

Starting revision/tree: development `main` at
`0bdc400a291574e8762b840ec23db56189bcd608`, tree
`c2c47ddfff7ecab7295969781e2a7b63fea94d93`. Work is isolated on
`agent/connectome-product-integration`. The original local integration branch
and the separately edited sibling Connectome checkout were preserved.

Change brief (current -> target behavior, owner, exact scope, unchanged
behavior, stop conditions): replace an unregistered separate UI checkout with
the optional `connectome/` Git submodule, pinned to published development
revision `ce08e2f6e1efd60ea7762598c23dc4659d0df1a3`. Align README identity and
the architecture/client owners with the owner's shippin.cloud private-devspace
and DevPulse local-engine offering. Those experiences remain delivery targets.
Do not change engine runtime, UI implementation, source visibility, product
version, or acceptance status. Stop if UI source becomes an engine input, the
pin is unavailable, unrelated implementation changes are needed, or a hosted
execution/synchronization capability would be claimed without evidence.

The owner instructed `execute` after reviewing the locally prepared brief and
the narrow exception to opening a PR before the first implementation commit.
GitHub requires a commit difference to open that PR. The original brief was
saved before implementation; the resulting PR description carries its scope,
oracles, decisions, and publication controls.

## Reviewed owners and implementation

Owner records reviewed: `README.md`, `AGENTS.md`, the alpha objectives and
roadmap portals, Gate H, POAM-011, system overview, Connectome client contract,
agent-bootstrap independent-engine boundary, CI operations, execution map,
change-authoring procedure, and the Gate H journal index.

Files and symbols read in full at the affected implementation boundary:
`workspace_source_inputs_are_repository_closed`, `git_index_entries`, the new
registration validator and negative cases in
`crates/authority/rrd-engine/tests/workspace_architecture.rs`;
`git_files`, `planned_gates`, `current_record`, `records`, `validate`, `render`,
and `main` in `scripts/ci/build_execution_inventory.py`; the complete new
`scripts/ci/test_execution_inventory.py`; `.gitmodules`; and the candidate
inventory-check step. Supporting reads covered metadata assembly, existing
source/identity checks, knowledge navigation/export, and documentation policy.

Implementation-requirements traceability:

| Current behavior and source | Canonical destination and gate | Replacement evidence |
|---|---|---|
| Architecture test rejects every mode-160000 Git entry. | Same source-closure test, H-06 prerequisite; permit only the exact optional UI registration and keep engine targets, workspace members, and path dependencies outside it. | Positive registration and negative URL, path, extra setting, duplicate setting, missing registration, symlink, and additional-submodule cases; complete architecture suite. |
| Inventory reads each Git path as a file, failing on submodule directories. | Same inventory generator; represent the parent gitlink identity without adopting child files. | Temporary Git repositories prove absent/populated/edited-child parity, pin-sensitive identity, ordinary file hashing, and conflict rejection. |
| Product portal omits the cloud/desktop delivery relationship. | README identity, architecture delivery boundaries, client checkout contract. | Documentation authority/link and knowledge checks; no runtime qualification claim. |

No runtime behavior was moved or deleted. Existing containment and source
closure remain in their canonical test; no fallback source lane was added.

Implementation/evidence paths created, changed, moved, or deleted:

- Created `.gitmodules`, the `connectome` gitlink, the focused inventory test,
  and this journal.
- Changed `README.md`, `AGENTS.md`, `docs/architecture/system-overview.md`,
  `docs/reference/client/connectome.md`, `docs/roadmap/rrflow-1.0/gate-h.md`,
  `docs/poam/rrflow-1.0-alpha/poam-011.md`, `docs/operations/ci.md`, the existing
  architecture test, inventory generator, and CI inventory step.
- Regenerated the Gate H journal index and execution inventory using their
  owning scripts. No authored source was moved or deleted.

Research decision and primary-source/adaptation record: local owner records,
Git remote/index inspection, `git ls-remote`, and GitHub repository metadata
were sufficient. The acquired UI commit matches the published development
main revision. No external algorithm or source adaptation was performed.

Trace/resource/debug decision: not applicable to this source-registration and
documentation package. No runtime trace, profiler, benchmark, or performance
claim was introduced.

## Oracles, acceptance, and failures

First-failure or characterization oracle and result: after adding the gitlink,
the unchanged inventory failed with `PermissionError` while reading the
`connectome` directory. The unchanged source-closure test failed with
`tracked Git submodule supplies repository input at connectome`. Both failures
were corrected at their existing owners rather than disabling the checks.

Acceptance commands and exact results on Windows:

| Command | Result |
|---|---|
| `git submodule add --depth 1 https://github.com/rrflow/connectome-development.git connectome` | Acquired published commit `ce08e2f6e1efd60ea7762598c23dc4659d0df1a3`; no sibling-source dependency. |
| `git ls-files --stage -- connectome` and `git submodule status -- connectome` | Parent mode `160000` and initialized child agree on the exact pin. |
| `python scripts/ci/test_execution_inventory.py` | 4 tests passed. |
| `cargo test -p rrd-engine --test workspace_architecture workspace_source_inputs_are_repository_closed --locked` | Passed after the narrow UI exception and containment checks. |
| `cargo test -p rrd-engine --test workspace_architecture --locked` | 30 tests passed. |
| `python scripts/ci/check_workflow.py` | Passed: 3 workflows, 7 substantive jobs, 5 engine suites, 20 default-feature packages, 6 optional-feature packages. |
| `python scripts/knowledge/test_navigation.py` | 6 tests passed. |
| `python scripts/knowledge/test_export.py` | 11 tests passed. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy -p rrd-engine --test workspace_architecture --locked -- -D warnings` | Passed with warnings denied. |
| `python scripts/knowledge/render_navigation.py` then `--check` | Passed: 14 indexes, 289 nodes, 1250 edges; one index regenerated, then zero drift. |
| `python scripts/ci/build_execution_inventory.py` then `--check` | Passed: 1186 current/generated/planned records. |
| `python scripts/ci/check_documentation.py` | Passed: 290 statuses, 288 classified coordinates, owner chain, indexes, and local links. |
| `git diff --check` | Passed. |

Surfaced failures and corrections: initial sandbox access was denied in the
original session; owner-authorized access and the later unrestricted session
allowed work to proceed. The two expected source-policy/inventory failures
above drove the implementation. A patch used an outdated CI-document anchor
and was reapplied against the actual baseline. A test invocation before its
file existed failed, then passed after creation. The first documentation check
reported this not-yet-created journal link. Windows text writing initially
changed all Gate H line endings; LF was restored so only the intended rows
change. An unquoted PowerShell Git tree selector was rejected; the quoted
selector supplied the baseline tree above. Attempted submodule deinitialization
was refused because the new gitlink was staged; no forced removal was used and
that attempt supplies no absent-checkout evidence. Temporary-repository tests
provide the absent/populated/edited inventory evidence.

Checks not run and reason: no UI build/browser suite, full runtime workspace
suite, cloud launch, Rust/Python/Go project execution, DevPulse integration,
account creation, synchronization, signing, or release qualification. Those
implementations and acceptance journeys were not changed. Candidate CI is a
separate PR result, not implied by these focused local passes.

Remaining known errors and owner: Connectome H-06 real-engine conformance and
H-07 remote endpoint qualification remain open under POAM-011. The private UI
repository requires access when explicitly acquired. Hosted project execution,
preview/deployment, accounts, and synchronization still need their own bounded
contracts, implementations, and acceptance evidence.

## Review and publication

Full-file reread and diff review: the affected implementation symbols and
authored additions were reread with their surrounding boundaries; the complete
changed diff was reviewed. Generated inventory is regenerated, not hand-edited.

Change checklist:

- Owner chain, baseline, exact scope, source pin, and stop conditions bound.
- First failure reproduced and replacement behavior tested at the same owner.
- Research, adaptation, trace, resource, and debugging decisions recorded.
- Unrelated local work preserved; no official repository push or visibility
  change; no version, runtime, or acceptance-status change.
- Linked evidence and generated navigation/inventory accompany the package.
- Publication uses the development PR and candidate CI; no direct main push.

Roadmap/POA&M status change, or explicit none: no completion or lifecycle status
change. H-06 names the pinned source boundary; H-07 includes shippin.cloud
endpoint candidates. POAM-011 records source integration and the unqualified
delivery targets without treating registration as conformance.

Pull request, CI run, and merge evidence: the PR from
`agent/connectome-product-integration` to development `main` carries the brief,
candidate checks, and any later merge receipt. At journal creation it has not
been merged. Before publication, the exact resolved development URL, branch
ref, and commit are recorded; the official promotion repository is not a push
destination.

## Links

- [Gate H](../../../roadmap/rrflow-1.0/gate-h.md)
- [Connectome checkout contract](../../../reference/client/connectome.md#repository-checkout)
- [Delivery architecture](../../../architecture/system-overview.md#delivery-experiences)
- [POAM-011](../../../poam/rrflow-1.0-alpha/poam-011.md)
