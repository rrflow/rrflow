# Existing-project retained admission, ported by hand

**Status:** active implementation evidence; Windows proved here, Linux and macOS open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/existing-project-retained-admission-20261115`
**Owner:** Gate D existing-project installation, [POAM-006](../../../poam/rrflow-1.0-alpha/poam-006.md)

## Change brief

- **Outcome and package:** POAM-006, existing-project installation. The owner decided on 2026-10-31 that a leftover `.rrflow` "must be adopted in place, not refused", and the first characterization ([existing-project-admission-windows](existing-project-admission-windows.md)) states the intended repair: preserve project-owned files, directories and ACLs.
- **Starting revision:** `a30f68d4e`.
- **Current behaviour before:** the installer moved every entry of a leftover `.rrflow` into `.rrflow/previous/<plan id>/` (#236). That relocation was an agent's reading of the owner's sentence, not an owner decision. The preservation oracle `existing_project_preserves_inherited_engram_and_opens_after_replay` stayed ignored because it failed on exactly that move.
- **Target behaviour:** an existing `.rrflow` and `.rrflow/rrd` are retained where they are. The plan binds their native identity and security descriptor; nothing inside is moved, deleted or re-permissioned; RRFlow creates its own private `roots` and `credentials` beside the project's entries; the locator and bootstrap are staged privately with identity-bound publication proofs.
- **Source of the design:** the archived work of draft PR #247 (`refs/archive/20261114/wip/rrflow-existing-project`, `d76fee096`, 838 commits behind), ported by hand in four commits. The archived code was never text-merged: it was read against `main`'s split modules and written into them.
- **Stop conditions:** a failing preservation oracle, any gate failure on Linux or macOS.

## What changed

1. **Contract** (`rrd-contract/src/attunement/installation.rs`): `InstallationRemovalRule::RetainExistingDirectory`, `InstallationManagedPath.{existing_directory_identity_sha256, existing_directory_security_sha256, publication_proof}`, `InstallationPublicationProof`, and the validation that ties them together. Plans without the new optional fields keep their digests (the golden is unchanged).
2. **Native security** (`rrd-engine/src/engine/token_key/private_file*`): `directory_security_sha256` reads owner, group and DACL on Windows and uid, gid, mode and POSIX ACLs on Linux, from the retained handle, twice, and refuses other platforms.
3. **Layout and ancestors** (`engram_layout.rs`): `bind_existing_ancestors`, `validate_plan_ancestors`, the retained layout (staging in `.rrflow/credentials`, proof paths), `for_plan`, and a recovery check that holds only directories RRFlow created to an exact entry list.
4. **Installation** (`installation/{planning,admission,apply,open,validation,storage,bootstrap}.rs`): planning binds the ancestors and sets the retained layout, admission validates them with the exact revision before the first write, publication goes through `publish_into_retained_parent`, and open and verify require the proofs.
5. **Removed, because the relocation it served is gone:** `InstallationPlan.adopted_engram_entries`, `MAX_ADOPTED_ENGRAM_ENTRIES`, `ADOPTED_ENGRAM_DIRECTORY`, `validate_adopted_engram_entries`, `EngramFilesystem::adopt_engram_directory`, `existing_engram_entries`, `adopt_private_directory_anchored` and the platform `adopt_directory_relative` and `DefaultOwner` code. A plan or record carrying `adopted_engram_entries` no longer decodes (pre-release: reinstall, no migration). Three root re-exports nothing outside the crate used were dropped to keep the code-shape ratchet.
6. **Deliberate differences from the archived candidate:** a pre-canonical `.rrflow/instance.toml` is left where it is and is no longer a refusal, because nothing reads it and the owner's own checkout has one; only a project with a locator is refused as an installed engram.

## Tests

Preservation oracle un-ignored and passing. Ported from the archived work: `retained_*` tests in `tests/installation/retained_engram.rs` (identical replacement refused, concurrent project-owned file preserved, unproven staging refused without removal, concurrent identical bootstrap refused, changed ancestor refused before writes and re-plannable, replaced directory refused, changed ACL refused), process death at each of the eleven install stages with the project's files and ACLs unchanged (`installation_concurrency.rs`), and in-process recovery at each stage (`installation_recovery.rs`). New: the contract test for retained plans and the directory security digest test. Rewritten to the retained behaviour: the leftover-folder and stale-plan tests.

## Commands and results

Run on Windows 11 at the commit that carries this record; see the commit and the gate for the full workspace run:

- `cargo test -p rrd-engine --lib -- retained_`: 15 passed.
- `cargo test -p rrd-engine --lib installation`: all passed, including the oracle.
- `cargo test -p rrd-contract --test attunement_contract`: passed.

## Not proved

Linux and macOS behaviour (the Linux descriptor code compiles and runs only in the gate; other Unix targets refuse by design), the crash windows inside a staging or proof write, re-admission after a changed retained descriptor after installation, native filesystem ID reuse, and a real-checkout run against the owner's own `.rrflow`. These stay in POAM-006.
