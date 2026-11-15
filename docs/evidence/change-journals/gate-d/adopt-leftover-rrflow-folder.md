# Adopt a leftover `.rrflow` folder in place

**Status:** historical implementation evidence; the relocation it describes was replaced
**Superseded by:** [retained admission, 2026-11-15](existing-project-retained-admission-20261115.md)
**Coordinate:** `rrflow://rrflow-instance/data/change-journal/adopt-leftover-rrflow-folder`
**Owner:** [Gate D](../../../roadmap/rrflow-1.0/gate-d.md), installed lifecycle ([contract](../../../reference/deployment/installed-lifecycle.md))

## Outcome and baseline

Starting main `f256c8e1`. On 2026-10-31 the repository owner reported that
importing a project from Connectome "fails protected". Reproduced with the
engine bundled in the owner's build (`8d47500`): any project holding a
`.rrflow` folder with ordinary inherited permissions and no locator failed the
no-write preview with
`Storage("private object requires the current owner and a protected non-null DACL")`.
The owner's own checkout had such a folder from September development runs.
The owner decided the folder must be adopted in place, not refused.

## Change

- `rrd-contract`: `InstallationPlan.adopted_estate_entries` (optional, sorted,
  bounded, plain top-level names; `previous` excluded). Plans without it keep
  their established digest; plans with it bind the list into the digest.
- `rrd-engine` planning lists the leftover folder's entries instead of failing;
  admission tolerates an unadmitted non-private `.rrflow` only before a locator
  exists. An installed estate stays strict and is never adopted.
- Apply (new installs only, before the install intent) refuses changed
  entries, makes `.rrflow` private in place when the current user owns a real
  folder (Windows protected owner/SYSTEM DACL via the retained handle; Unix
  `fchmod 0700`), and moves each listed entry with no-replace renames into
  `.rrflow/previous/<plan id>/`. Recovery admits `previous`.
- The superseded test that asserted refusal of `.rrflow/instance.toml` is
  replaced by tests for the new owner-decided behavior.

## Evidence

- `cargo test -p rrd-contract --test attunement_contract`: 20 passed, including
  `adopted_estate_entries_are_bound_only_when_present`.
- `cargo test -p rrd-engine --lib`: 259 passed, 4 ignored, 0 failed, including
  `a_leftover_rrflow_folder_is_adopted_in_place_and_keeps_every_file`,
  `a_changed_leftover_rrflow_folder_refuses_the_stale_plan_without_effects` and
  `an_installed_estate_is_never_adopted`.
- Windows CLI, the owner's failing shape (`.rrflow/dev`, `.rrflow/rrd`, no
  locator): plan lists `["dev","rrd"]`, apply installs, `verify` passes,
  `serve` plus `ready` authenticate. A clean project passes the same flow.

## Not done

- Moved entries keep their original permissions; they were already readable
  as before adoption.
- The install inventory still ignores `.gitignore`; projects with large
  generated folders can exceed its file or byte bound. Separate package.

## 2026-11-09 main reconciliation

Merged `development/main` at `a9f1b0fb` (413 commits ahead of this branch's
base) by an ordinary merge. Two conflicts: the `rrd-contract` export list,
resolved by adding this package's `validate_adopted_estate_entries`,
`ADOPTED_ESTATE_DIRECTORY` and `MAX_ADOPTED_ESTATE_ENTRIES` to main's current
list, and the generated Gate D index, regenerated. On the merged tree:
`cargo test -p rrd-contract --test attunement_contract` 20 passed;
`cargo test -p rrd-engine --lib` 397 passed, 4 ignored;
`cargo clippy -p rrd-contract -p rrd-engine --all-targets -D warnings` clean;
`cargo fmt --check` clean. Not rerun here: the Windows CLI run on the owner's
failing shape recorded above; candidate CI's installed lifecycle jobs are the
acceptance. #247 characterizes the same inherited-ACL shape with a failing
preservation test and no repair; once this package lands, that test belongs
here as an additional regression or #247 closes as folded.

### Candidate CI, 2026-11-09

The GitHub Windows runner failed `a_leftover_rrflow_folder_is_adopted_in_place_and_keeps_every_file`
with "the existing .rrflow folder belongs to another account": an administrator's
token gives new objects `BUILTIN\Administrators` as owner, so the folder the
test itself created did not match the token's user SID. Adoption now accepts a
folder owned by the user or by the token's default owner (`TokenOwner`), which
is how Windows itself decides what an account owns. 64 installation tests pass
locally; the runner is the acceptance. The same run also failed
`git_inputs_detect_appearance_change_and_bound_bytes`, a main-line test this
package does not touch, which passed on the two other Windows runs today; it
is recorded as a flake to investigate, not fixed here.

2026-11-10 candidate correction after run 42750074866 (Windows engram process): on the hosted Windows runner the leftover `.rrflow` folder is owned by BUILTIN\Administrators, the elevated token's default owner, so adoption set the private DACL but the private-object check still required the user as owner. Adoption now mirrors fresh creation: it sets the protected private DACL through the WRITE_DAC handle the owner always holds, then, only when the owner differs from the user, takes ownership through a second handle opened with the WRITE_OWNER that the new DACL grants. Every failure names its step. Locally (owner already the user) the adoption test, the private-file tests and strict clippy pass; the runner is the oracle for the ownership step.
