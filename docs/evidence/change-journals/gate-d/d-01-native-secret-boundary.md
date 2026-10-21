# D-01 native secret leaf-file boundary

**Status:** active bounded implementation evidence; D-01 and alpha remain open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-secret-boundary`
**Owner:** OBJ-07 prerequisite, D-01, POAM-006 and POAM-013

## Baseline and change brief

[PR 45](https://github.com/rrflow/rrflow-development/pull/45) was opened with
the change brief before source edits. Synchronized starting revision:
`6a4f4c24140ed8f50fe1437b66ed6039d8a4f637`, tree
`065f1ada3be35022732500c04ed6800ec9253b65`. The empty PR anchor was
`1f2f377d787abe0058a55b31bb22fb10d0ad8d4a`. Version remains `1.0.0`.

Previously, secret readers checked a path and then reopened it, accepted
multiple hard links, and inherited Windows creation ACLs without inspecting
them. Consolidate secret leaf-file I/O at the existing engine authority:
exclusive protected creation, one bounded validated read handle, effective
Unix owner/private mode and Windows owner/protected-DACL validation. Route
token keys, operator credentials, installation intents and publication intents
through it. Preserve entropy, encodings, digest checks, replay, transaction
authority, installation identity and every existing successful lifecycle.

Stop on changed authority or recovery semantics, silent permission repair,
unbounded reads, unsupported enforcement succeeding, leaked secret diagnostics,
undeclared scope or failed acceptance. Ancestor containment, mount races,
general hostile-filesystem confinement and complete D-01 closure are outside
this bounded package.

## Read and replacement map

Owners reviewed: `README.md`, `AGENTS.md`, alpha objectives, dependency spine,
Gate D, POAM-006/-013, ADR-0002, execution change-authoring and implementation
navigation, installed-lifecycle reference and CI operations. Implementation
review covered the engine manifest/composition, estate layout, installation
callers, every secret-reader call site, installation and restore recovery,
their fixtures and the actual primary-process lifecycle. Complete new
private-file modules, token-key, recovery, publication and modified test files,
manifest and changed diff were reread, including the retained end-to-end
lifecycle assertions. Lockfile and generated projection changes were reviewed
as mechanical diffs and verified by their owning tools.

| Current behavior and source | Canonical replacement | Characterization/replacement oracle |
|---|---|---|
| `engine/token_key.rs` path check, reopen and private creation | `engine/token_key/private_file.rs`, `private_file/unix.rs`, `private_file/windows.rs`; token helpers delegate | actual hard-link regression, byte bounds, exclusive creation, native permissions, retained key |
| `engine/installation/recovery.rs` separate intent reader | shared private reader; existing canonical intent comparison retained | interrupted intent denial leaves no estate and unchanged bytes; retry retains timestamp and cursors |
| `engine/restore_publication.rs` check then ordinary intent read | shared private reader and token reader | retained interrupted publication, backup/restore and restored readback corpus |
| installed open/inspect/exact replay | same engine operations with shared reader | external aliases deny all three without changing project bytes or cursors |
| primary CLI quick verification | unchanged adapter using hardened engine | real executable rejects linked credential, then completes normal lifecycle after fixture alias removal |

Created paths are the three private-file source modules and this journal.
Changed paths are the engine manifest, lockfile, token-key/recovery/publication
modules, installation/recovery tests, CLI installed-lifecycle test, native CI
step, installed-lifecycle reference, POAM-006 and generated navigation/inventory.
No crate, runtime helper, public operation, format or authority was added.
Optional native dependencies use already locked `libc`/`windows-sys` versions
and enter only the full engine feature. No source files were moved or deleted;
duplicated helper bodies were replaced directly, with no fallback lane.

The pre-edit PR brief was extended before changing the existing native CI
lane: Windows/macOS process tests did not run the platform-specific engine
unit fixtures. The lane now invokes `engine::token_key` explicitly. Linux's
existing full engine suite covers its native implementation. Job topology,
deadlines and required checks are unchanged.

## Research, trace, resource and debugging decisions

Primary API references informed implementation at this one engine boundary:
[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew),
[GetSecurityInfo](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo),
[security descriptor strings](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-string-format),
[handle information](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/ns-fileapi-by_handle_file_information),
[Rust Windows open options](https://doc.rust-lang.org/stable/std/os/windows/fs/trait.OpenOptionsExt.html)
and [Unix open options](https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html).
The two attempted Open Group `open` pages returned HTTP 403; they are not
claimed as read evidence. Exact local `windows-sys` bindings were inspected.
No upstream database code or architecture was adapted.

Reads retain caller-specific byte ceilings, with a checked maximum-plus-one
read and before/after handle validation. Windows API allocations and handles
have explicit owners; creation supplies security before content is written.
No shell ACL utility, secret-bearing trace or permission repair is introduced.
Existing redacted diagnostics and lifecycle receipts remain. No performance
claim requires a benchmark here; fixed hardware and broad resource
qualification remain their owning gates. Tests observe authorized effects,
denials, receipts, byte stability and durable recovery, never private model
chain of thought or a persisted schema of reasoning.

## Failure oracle and acceptance

Before production edits, `cargo test --locked -p rrd-engine --lib
secret_hardlinks_are_rejected_without_changing_either_path -- --nocapture`
failed: zero passed, one failed, with `multiply linked secrets must be denied`.
The fixture creates a real secret and hard link and verifies both contents.
The repaired test rejects both paths and reopens successfully after removal
of only the test-owned alias.

Local Windows results:

| Command | Observed result |
|---|---|
| `cargo check --offline -p rrd-engine --lib` | Passed; one unused import surfaced and was removed |
| `cargo test --locked -p rrd-engine --lib engine::token_key -- --nocapture` | 6 passed; protected creation, permissive/null/unprotected ACL denial, hard links, device rejection, byte bounds and open-handle sharing |
| `cargo test --locked -p rrd-engine --lib engine::tests::installation` | 26 passed, including every existing eight-stage retry and publication fixture |
| `cargo test --locked -p rrd-engine --all-targets` | 189 passed across unit, integration, causal-trace and architecture suites |
| `cargo tree --locked -p rrd-engine --no-default-features --edges normal --depth 1` | Only the engine root; neither new native dependency enabled |
| Same dependency command with `--features edge` | Only the existing five direct edge dependencies; neither new native dependency enabled |
| `cargo test --locked -p rrflow-cli --test installed_lifecycle -- --nocapture` | 5 passed in 217.20 seconds; staged actual executable 155,394,560 bytes; fresh/existing restart, Parquet and restore readback retained |
| `cargo clippy --locked -p rrd-engine -p rrflow-cli --all-targets -- -D warnings` | Passed |
| `cargo test --locked -p rrd-engine --lib malformed_or_foreign_recovery_state_is_preserved_and_rejected` | 1 passed after fixture correction |
| `cargo clippy --locked -p rrd-engine --lib --tests -- -D warnings` | Passed after fixture correction |
| `cargo fmt --all --check`, `git diff --check` | Passed |
| `python scripts/ci/check_documentation.py` | Passed: 356 document statuses, 354 classified coordinates |
| `python scripts/ci/check_workflow.py` | Passed: existing 3 workflows and 7 substantive jobs |
| `python scripts/knowledge/render_navigation.py --check` | Passed: 17 indexes, 355 nodes, 1550 edges |
| `python scripts/ci/build_execution_inventory.py --check` | Passed: 1297 path records |
| `python scripts/ci/test_execution_inventory.py` | 4 passed |
| `python scripts/ci/check_generated_surfaces.py` | Passed: 42 HTTP operations and 7 signal projections unchanged |
| `python scripts/check_version.py` | Passed: release train 1.0.0 |

The malformed-intent fixture was changed to create a properly protected file,
so its invalid-content oracle is not masked by earlier Windows ACL denial.
Native hard-link aliases live outside project roots, preventing inventory
drift from substituting for the intended secret-boundary failure.

Raw local logs are retained under ignored `target/native-secret-*.log`;
the PR candidate check logs are the remotely accessible exact-revision record.

## Limitations and change checklist

Not locally run: Unix fixtures, privileged foreign-owner changes, Windows
reparse adversarial fixtures, complete ancestor/mount race matrix, Miri,
sanitizers, signed clean-machine bundles or model reasoning. Native Unix
fixtures are required in candidate CI; the other omissions remain explicit
D-01/J work. One-handle leaf reads do not establish a complete atomic snapshot
or prevent all same-owner changes. This package adds no upgrade/migration
promise for older pre-alpha estates.

Checklist: owning prerequisite and stop conditions bound before edits; actual
first failing oracle retained; replacement behavior mapped to existing
authority; primary-source/native API decisions recorded; smallest tests then
owning suite; failures and omitted checks disclosed; native CI coverage made
explicit; diff reviewed; journal linked through generated navigation and
inventory before commit. POAM-006 records the verified deficiency and bounded
remediation. No objective, roadmap gate or parent POA&M status changes.

Anchor CI `39534147475` was cancelled as superseded and is not qualification.
All 17 checks for the final exact PR candidate must succeed before merge.
The PR, candidate CI run and merge commit bind landing evidence. Every push
records the resolved public development URL, feature ref and exact revision;
the private promotion repository and `main` receive no direct push.
