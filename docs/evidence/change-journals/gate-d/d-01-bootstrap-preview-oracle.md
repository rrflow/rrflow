# D-01.4 bootstrap preview failure oracles

**Status:** active failing characterization; complete bootstrap implementation pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-bootstrap-preview-oracle`
**Owner:** OBJ-07 prerequisite, D-01.4, POAM-006

Starting revision `6ba9dc5a299ae283f4d890392f5e0fc008609d79`, tree
`945913a343f8c84f660db2b12be99eb8f7fd2f0e`, was synchronized with
development/main with a clean worktree. D-01.3 passed all 17 jobs in
[run 39647536747](https://github.com/rrflow/rrflow-development/actions/runs/39647536747)
at candidate `3817cb5ae6d9d100cb6a9a5768c67608cbaeef6a` and merged through
[PR 50](https://github.com/rrflow/rrflow-development/pull/50). This package
preserves its native admission, directory capabilities and storage authority.

## Brief and sequencing

The next D-01 package must bind actual bundled template/specialization assets
and the complete previewed effect set through the current installation plan
and installed record. Initial seat/security/configuration/checkpoint/job state
remains in the existing engine transaction; discovery does not activate a
provider. Generic bootstrap is provider-neutral and Clyffy is explicitly
selected. Project-owned root instructions remain unchanged by default.

The pre-edit brief is prepared in `target/d01-bundled-bootstrap-brief.md`.
GitHub rejected draft PR creation with no commits ahead of main. This initial
commit therefore contains a failing executable regression, not a planning-only
commit. Attach the brief to the package PR before production edits. This is an
explicit sequencing exception, not a claim that the remote PR existed before
the first test edit.

Stop on unknown/unpreviewed effects, undeclared asset fetching, credential
exposure, provider activation by discovery, duplicate lifecycle authority,
weakened filesystem admission, damaged project content or failed replacement
evidence. D-01.5/.6 still own full native qualification and closure.

## Observed effects oracle

`cargo test --locked -p rrd-engine --lib
installation_preview_covers_transient_intent_before_storage_creation -- --nocapture`
failed on Windows: 0 passed, 1 failed, 0 ignored; the tool returned exit 1.
At the real `IntentPublished` observer, actual root entries include
`.rrflow-install-install-9210f5e2db2fc97b7da8db51.json`, absent from the accepted
managed-path preview. The fixture observes filesystem entries independently
of the intent filename builder and never reads secret payloads. It retains a
project-owned README and stops before store creation. No production failpoint
or private reasoning trace is introduced.

Created `crates/authority/rrd-engine/src/engine/tests/installation_bootstrap.rs`
and registered it in `engine/tests.rs`. Both files were read in full and their
complete diff reviewed. No production behavior has been moved or deleted in
this characterization commit. The final implementation must expand this
first oracle to the full file/record/invocation closure and every new effect
boundary; this single failing test is not complete D-01.4 coverage.

## Bundle byte identity oracle

At the same baseline, run `git -c core.autocrlf=true checkout-index --force
--prefix=<workspace>/target/d01-bootstrap-checkout/ --
crates/authority/rrd-engine/assets/install/default-profile-v1.json
crates/authority/rrd-engine/assets/install/default-estate-configuration-v2.toml`.
Compare those bytes with `git show HEAD:<path>` using SHA-256. Both differ
solely by LF-to-CRLF conversion:

| Asset | Git blob bytes / SHA-256 | Converted checkout bytes / SHA-256 |
|---|---|---|
| `default-profile-v1.json` | 2211 / `100b5a7a7ea03c37b00ad7334787a49038675b8cf6bf2489d541db53be5aff3c` | 2253 / `d725a0c034d004c3954ee45214af1a92fa1548ec0f62270a20f4a3ee4a361918` |
| `default-estate-configuration-v2.toml` | 413 / `aed474faf82e0a185b94d972f917170f06f27181d3ab6eafa249abe6a9e85d63` | 437 / `10bcb2cacfefc46fc87a08109892c8f4ce20e863d92f5a7d084df91edf421319` |

The experiment exited 0 and retained its JSON under ignored
`target/d01-bootstrap-checkout-evidence.json`. `DefaultInstallProfile::load`
hashes its `include_bytes!` input, so this is a verified cross-checkout bundle
identity gap, not a claim that operator-supplied files should be normalized.
Pin canonical owned asset bytes and retain exact digest assertions.

[Git's attributes reference](https://git-scm.com/docs/gitattributes#_eol)
defines explicit checkout line-ending control; [Rust include_bytes](https://doc.rust-lang.org/std/macro.include_bytes.html)
embeds file contents at build time. These primary sources support the byte
provenance repair. No upstream database implementation is adapted.

## Review, checklist and status

Owner reads: README, Gate D, agent-bootstrap, installed-lifecycle, POAM-006,
the change-authoring procedure and CI contract. Implementation reads relevant
to the brief include installation planning/apply/inspection and admission,
estate layout, recovery intent publication, current profile/configuration
assets, installation contract and fixtures, and CLI dispatch. Targeted reads
of memory-seat planning and security genesis identify existing transaction
effects; the final implementation journal must enumerate full changed-file
review and replacement evidence.

- [x] Bound OBJ-07/D-01.4/POAM-006, exact first oracle and stop conditions.
- [x] Prepared the brief before edits and recorded the PR-creation exception.
- [x] Added a real failing observation without changing production behavior.
- [x] Recorded primary research, byte-identity evidence and redacted diagnostics.
- [x] Reviewed both changed source files and the complete characterization diff.
- [ ] Complete implementation, expanded acceptance corpus and candidate CI.

Navigation/inventory are regenerated for this addition. Full engine, contract,
CLI, native/offline and strict implementation lint are not claimed passing on
this intentionally failing characterization. No resource/scale benchmark is
needed for the first oracle. Production replacement mapping, generated
surfaces and complete acceptance remain the implementing part of this PR.
Mechanical characterization checks passed with exit 0:
`cargo fmt --all --check`, `python scripts/ci/check_documentation.py`,
`python scripts/knowledge/render_navigation.py --check` (17 indexes, 362 nodes,
1576 edges), `python scripts/ci/build_execution_inventory.py --check` (1313
records), and `git diff --check`. These checks do not turn the failing runtime
oracle into an accepted package.

Gate D reconciles D-01.3 to its exact green candidate/run/merge and marks
D-01.4 active. POAM-006 gains the observed transient-path omission and bundle
checkout gap. D-01, OBJ-07 and alpha remain open. No graph/vector scale,
PostgreSQL compatibility, model routing or cloud-sync completion is claimed.
