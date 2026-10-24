# D-01.1 private estate-directory admission

**Status:** active bounded implementation evidence; D-01 remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-private-estate-directories`
**Owner:** OBJ-07 prerequisite, D-01.1, POAM-006

## Baseline and brief

[PR 48](https://github.com/rrflow/rrflow-development/pull/48) carried the
change brief before source edits. The clean synchronized starting `main` was
`1815af32a53e161bc5f3d7f0f7b4365462cd9f7a`, tree
`5599452feb6898cff316e0644672bf76ee804e97`; empty anchor
`a94f424852fed4e8f34c80a8dc6b64345d347ee7`. The six-package planning PR 47
was in candidate CI when this independent first implementation began.
It subsequently merged as `ff028bc8a92f0e6d83cef0b0bafdc7dc5231fd1d` after
run `39617883599` and `pipeline / ci-gate` concluded success. The Windows job
summary lagged behind its successful completed steps; no check was bypassed.
That plan was merged into this branch before the implementation candidate.
The author temporarily stashed only this package's three changed documentation
projections/ledger; restoring them required preserving both appended POAM-006
sections and regenerating the execution inventory. No source conflict or user
change was discarded.

Replace default Windows directory creation and name/type-only recovery checks
with private native creation and opened-object security admission for exactly
`.rrflow`, `.rrflow/rrd`, `.rrflow/rrd/roots` and `.rrflow/credentials`.
Preserve project-parent permissions, installation identity, sealed plans,
security transactions, credential formats and eight-stage recovery. Refuse
incompatible existing permissions without repair or migration. Product version
remains `1.0.0`; no public schema or storage format changes.

Stop conditions: additional directory/storage scope, insecure fallback,
duplicate private-file policy, changed lifecycle authority or identity,
automatic permission repair, unavailable enforcement treated as success,
secret-bearing diagnostics, or a failed acceptance case. Ancestor retention,
mount races and installation-wide exclusivity remain subsequent packages.

## Reads and replacement traceability

Owners reviewed: README/AGENTS, OBJ-07, D-01 closure plan, POAM-006/-013,
change-authoring and implementation-traceability procedures, bootstrap,
installed-lifecycle and native CI. Complete source reads covered estate layout,
private-file facade and both native implementations, token-key export boundary
and installation recovery. Installation plan/apply, locator/open/inspection,
publication/cleanup and storage-opening handoff were reviewed by function;
recovery test stages and native CI selection were inspected.

| Previous behavior | Canonical replacement | Replacement evidence |
|---|---|---|
| `estate_layout.rs::owner_only_directory_builder` requests Unix mode or Windows defaults | Native create/validate helpers at the existing private-object boundary; original helper removed directly | Creation under a permissive parent stays private, exclusive, and leaves the parent permissive |
| Recovery checks directory names/types | `validate_estate_directories` admits each existing private boundary before apply; recovery reuses it | Actual permissive-directory recovery regression, unchanged intent, absent store/credential, successful exact retry after explicit fixture correction |
| Installed locator callers can traverse permissive estate directories | `read_locator` requires all four directory boundaries before installed access | Locator, credential, verify, open and replay denial with identical file bytes; exact replay succeeds after fixture correction |
| Windows secret-specific security inspection | One shared owner/protected-DACL validator with separate file/directory shape and inheritance rules | Existing secret tests retained; directory permissive/null/unprotected/missing-inheritance ACL tests |

Changed implementation: `engine/estate_layout.rs`, `installation.rs`,
`token_key.rs`, `token_key/private_file.rs`, `private_file/unix.rs` and
`private_file/windows.rs`. New integration fixtures are inside the existing
private-file test module, so the native `engine::token_key` CI invocation runs
them without adding a workflow step. The ordinary engine suite retains all
installation/recovery tests. No crate, source file or runtime dependency was
added, moved or removed. Docs update the installed-lifecycle contract and
POAM-006; this journal and its generated navigation/inventory supply evidence.

## Research and diagnostics

[CreateDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectoryw)
supports a creation-time security descriptor on capable filesystems;
[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
provides directory/reparse handles and sharing semantics. Windows requests a
protected owner/LocalSystem DACL with object/container inheritance and validates
the opened native directory. Unix requests `0700`, opens with directory,
no-follow, nonblocking and close-on-exec flags, and validates effective owner
and mode. Unsupported security enforcement is an error. The attempted Open
Group mkdir reference returned HTTP 403 and is not claimed as read evidence.
No upstream implementation was adapted.

The fixed four-directory admission has no recursive scan or unbounded retry.
Existing redacted diagnostics and lifecycle traces remain; no paths, content
or secrets are added to traces. Tests use deterministic permission changes,
actual filesystem objects and byte inventories. No benchmark, atomic snapshot,
all-races or hidden chain-of-thought claim is made.

## First failure and verification

Before production changes, `cargo test --locked -p rrd-engine --lib
permissive_estate_directory_cannot_advance_recovery -- --nocapture` failed:
0 passed, 1 failed, at `permissive estate directory must deny recovery:
.rrflow/rrd/roots`. The fixture interrupts after estate-directory creation,
changes a real Windows DACL, then tries exact recovery.

The corrected production path denies all four directory substitutions before
creating the store or delivering credentials. The expanded fixture also
checks installed access denial, unchanged file bytes, no implicit repair,
preserved first-apply time and exact replay after test-owned correction.

Local Windows results:

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-engine --lib engine::token_key -- --nocapture` | 9 passed |
| `cargo test --locked -p rrd-engine --lib engine::tests::installation` | 26 passed |
| `cargo test --locked -p rrd-engine --all-targets` | 192 passed across unit, integration and architecture targets |
| `cargo clippy --locked -p rrd-engine -p rrflow-cli --all-targets -- -D warnings` | Passed |
| `cargo test --locked -p rrflow-cli --test installed_lifecycle -- --nocapture` | 5 passed in 271.70 seconds; actual staged Windows executable 155,400,192 bytes; fresh/existing restart and restore retained |
| `cargo fmt --all --check` and `git diff --check` | Passed |
| `python scripts/ci/check_documentation.py` | Passed; owner chain and local links |
| `python scripts/knowledge/render_navigation.py --check` | Passed; generated indexes fresh |
| `python scripts/ci/build_execution_inventory.py --check` | Passed; generated path inventory fresh |
| `python scripts/ci/test_execution_inventory.py` | 4 passed |
| `python scripts/ci/check_generated_surfaces.py` | Passed; 42 HTTP operations and 7 signal projections unchanged |
| `python scripts/check_version.py` | Passed; 1.0.0 |

An intermediate fixture failed twice with Windows access denied: removing
inheritable owner grants from the changed parent also removed child access,
masking the intended byte-preservation assertion. The fixture now preserves
inheritable owner access while granting Everyone read; the ACL-only child
fixture uses an explicitly protected leaf. Production admission was not
weakened. The first baseline failure log remains separate from later results.
Raw local logs are under ignored `target/d01-directory-*.log`.

## Limitations and checklist

Unix fixtures require native candidate CI; cross-compilation is not their
proof. Privileged foreign-owner tests, hostile ancestor/mount races, full
store-descendant confinement, process-wide install exclusion and signed
clean-machine bundles are not qualified here. The next packages own those
requirements. No D-01, OBJ-07, alpha or parent POA&M completion is claimed.

Checklist: pre-edit owner/scope/stop decisions recorded; actual baseline
failure; direct replacement at the existing private-object authority; native
research and resource/debug decisions; smallest test followed by owning suite;
fixture failures disclosed; changed source, docs and the complete diff reviewed;
generated projections refreshed and checked before commit. Exact final
candidate CI and merge evidence are attached to
PR 48. Empty-anchor CI `39619853148` was cancelled as superseded, not counted
as qualification. Every push records public development destination/ref/SHA;
no direct `main` or private official-repository push is permitted.
