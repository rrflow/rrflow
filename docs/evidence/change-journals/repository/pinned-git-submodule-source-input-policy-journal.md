# Pinned Git submodule source-input policy journal

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/pinned-git-submodule-source-input-policy-journal`
**Owner:** evidence for the repository-owner decision that permits declared, pinned Git submodules

This record reports one repository policy package. It changes which source
inputs the repository accepts; it does not add a submodule, move Connectome,
change the signed-distribution contract, or advance any roadmap item.

## Evidence record

```text
gate/package and alpha outcome or prerequisite advanced: repository policy / A-07 source-closure rule and J-05 distribution prerequisite — the repository owner states that Git submodules are an accepted practice and directs that Connectome may become a submodule; the blanket submodule ban becomes a pinned-source-input rule
starting revision/tree and pull request: development/main 0bdc400a (Merge pull request #15) on branch policy/pinned-git-submodules; pull request recorded below
change brief (current -> target behavior, owner, exact scope, unchanged behavior, stop conditions): AGENTS.md (introduced by 14f6eb26, 2026-09-06), the system-overview source boundary, and workspace_source_inputs_are_repository_closed (A-07.1h) rejected every tracked gitlink -> a submodule is accepted when .gitmodules declares it exactly once at a normalized repository-relative path with a credential-free https:// URL that an anonymous clean clone and candidate CI can fetch, its gitlink pins the revision, and every CI checkout sets `submodules: recursive` once .gitmodules exists. Owner: AGENTS.md and docs/operations/ci.md. Unchanged: Cargo Git dependencies and transitive Git sources stay rejected; Cargo workspace members stay tracked in this repository; escaping paths and symlinks stay rejected; the signed default distribution must still contain every byte it needs after acquisition; Connectome remains optional for default readiness. Stop if the change would allow a credentialed, local-path, SSH, or undeclared submodule, weaken the Cargo source closure, or require editing historical evidence.
files and symbols read in full: AGENTS.md; crates/authority/rrd-engine/tests/workspace_architecture.rs (workspace_source_inputs_are_repository_closed, every_workspace_target_source_is_tracked, GitIndexEntry, git_index_entries); scripts/ci/check_workflow.py (verify_workflow_security, checkout_steps); docs/operations/ci.md repository workflow policy; docs/architecture/system-overview.md source and distribution boundary; docs/roadmap/rrflow-1.0-execution/change-authoring.md journal template. The A-07.1h journal and gate-a evidence that record the original ban were read and intentionally left unchanged as historical evidence.
implementation/evidence paths created, changed, moved, or deleted: changed AGENTS.md, docs/architecture/system-overview.md, docs/operations/ci.md, scripts/ci/check_workflow.py, and crates/authority/rrd-engine/tests/workspace_architecture.rs (declared_submodules, submodule_violations, new test submodule_policy_accepts_only_declared_pinned_https_submodules); created this journal; regenerated docs/evidence/change-journals/repository/README.md and docs/roadmap/rrflow-1.0-file-plan.jsonl.
research decision and primary-source/adaptation record: not required beyond Git's own behavior: `git config --file .gitmodules --null --get-regexp` parses declarations with Git's configuration grammar, and gitlinks (index mode 160000) pin an exact commit. Submodules are a long-standing native Git feature supported by hosting and CI checkout tooling; no upstream code was adapted.
trace/resource/debug decision (add/preserve/not applicable, with reason): runtime trace and resource evidence not applicable (no RRFlow runtime change); debugging evidence added through violation messages that name the submodule, path, and failed condition.
first-failure or characterization oracle and result: baseline workspace_source_inputs_are_repository_closed rejects any gitlink unconditionally. The new synthetic test accepts a declared gitlink-pinned https submodule and the empty case, and rejects an undeclared gitlink, a declaration without a gitlink, an SSH URL, a relative local URL, a URL with embedded credentials, a missing URL, an escaping path, and a duplicate path. check_workflow.py on a scratch copy passes without .gitmodules, fails with `checkout must fetch declared submodules recursively: .github/workflows/ci-reusable.yml:28` once .gitmodules exists, and passes again after every checkout gains `submodules: recursive`.
acceptance commands and exact results: in the Linux development container with Rust 1.99.1, `cargo test -p rrd-engine --test workspace_architecture --locked` — 30 passed; `cargo clippy -p rrd-engine --test workspace_architecture --locked -- -D warnings` — exit 0; `cargo fmt --all -- --check` — exit 0; on the host `python scripts/ci/check_workflow.py` — OK; `python scripts/ci/check_documentation.py` — OK; navigation, inventory, version, and diff checks are recorded at commit time.
surfaced failures and corrections: the first container run could not start because the Podman machine was stopped; the second could not run git because a worktree's .git file holds a Windows path, so the container received GIT_DIR/GIT_WORK_TREE for the worktree metadata; rustfmt rewrapped the new code and Clippy rejected one redundant match guard (`Some(authority) if authority.is_empty()` became `Some("")`). The local development image predated the 1.99.1 toolchain, so rustup auto-installed it inside the container.
checks not run and reason: no submodule was added, so no recursive clone or CI run with a real submodule was exercised; the engine suites beyond the architecture test, SDK conformance, and platform portability jobs were not run because only a test, a policy script, and documentation changed. Candidate CI on the pull request is the authoritative run.
remaining known errors and owner: adding Connectome as a submodule is a separate package that must also add `submodules: recursive` to every workflow checkout; it requires the Connectome repository to be cloneable anonymously, and rrflow/connectome-development was private on 2026-10-19 pending the owner's review of one token-like value in its history. The development container's seed script does not yet initialize submodules.
full-file reread and diff review: complete — every changed file and the full diff were reread.
change checklist: complete — owner decision recorded, all three rule locations updated, historical evidence preserved, policy enforced by test and workflow check with negative cases, Cargo source closure unchanged, journal and generated indexes updated
roadmap/POA&M status change, or explicit none: none
pull request, CI run, and merge evidence: recorded on the pull request for branch policy/pinned-git-submodules
```

## Links

- [Repository instructions](../../../../AGENTS.md)
- [CI operations](../../../operations/ci.md#repository-workflow-policy)
- [Source and distribution boundary](../../../architecture/system-overview.md#source-and-distribution-boundary)
