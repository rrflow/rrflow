# D-01 bounded installation inventory

**Status:** active change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-bounded-installation-inventory`
**Owner:** installation resource-admission repair receipt; Gate D retains completion authority

## Package and change brief

OBJ-07 independent installed-engine prerequisite, D-01 and POAM-006.
Starting revision `b5772374c1ea8ab388fa0c4ac13558d2e3125775`, tree
`0f3c28c6ab8d10ba174cfd737cb3d38e058b1fdc`.
[PR 40](https://github.com/rrflow/rrflow-development/pull/40) recorded the
change brief before source implementation. Empty opening commit
`ae821a5aaceda814963a2b3b28426e4647172545` is only a review anchor.
An isolated worktree preserves PR 39's independent vector repair.

Previously, recursive traversal materialized all entries before checking the
configured count, and file hashing could read beyond its admitted metadata
length if the file grew. The replacement enforces entry admission while
streaming directory iteration, uses an iterative pending-directory list and
sorts only admitted output. Hashing reads no more than the admitted file length
and rejects short input and observed type/length/mtime drift. Default profile,
version-one digest grammar, exclusions and exact-plan authority are unchanged.
No public schema, format, dependency, effect or runtime authority was added.

Stop conditions: changed stable digest, silent truncation of an accepted file,
unbounded replacement traversal/hash, expanded ignore/authorization policy,
failed lifecycle tests, or unrelated scope expansion.

## Traceability and review

The existing `project_inventory_sha256`, `project_inventory_sha256_excluding`
and recursive `collect_project_entries` are removed from
`crates/authority/rrd-engine/src/engine/installation.rs` and directly replaced by
one private `installation/inventory.rs` module. Its parent imports the same
precondition entry points for preview, apply/recovery and read-only verify.
The shared `sha256_file` used for executable/managed-file identity is unchanged.
No fallback, parallel inventory or persisted cache is introduced.

Characterization first ran against the original implementation: an independent
SHA-256 grammar fixture over README, empty file and nested source, excluding
`.git` and `.rrflow`, produced
`a1a2f8baba5c66694e08b3263eb9e4df589451ef7d65d00891a26bd201937514`.
That same test moves into the canonical module and must still pass. Existing
installation preview/apply/recovery/verify tests and native primary-executable
lifecycle are the composition oracles.

Owners reviewed: AGENTS, README, alpha objectives, dependency spine, D-01,
POAM-006, independent-engine bootstrap rules, change-authoring routine and
candidate CI contract. Source reviewed: default profile and its admission,
installation precondition collection/hash and preview/apply/recovery/verify
call sites; installed lifecycle and no-write/drift tests. The new module is
read in full and the complete deletion/addition diff is checked against the
traceability map. Supporting edits are limited to POAM-006, this journal,
generated navigation and execution inventory.

## Research, resources and diagnostics

No upstream source, dependency or new algorithm is adopted. Existing Rust
filesystem/read APIs and RRFlow SHA-256 support the required bounded walk.
The earlier [Turso reliability research](https://turso.tech/blog/the-final-boss-of-reliability)
informed explicit failure/limit oracles, not an implementation or verification
claim. Build-cache sharing between worktrees is only local compilation
acceleration; no product source or runtime depends on a sibling checkout.

Retained entries and pending directories are limited by the configured count.
The content buffer is 64 KiB and admitted content reads do not exceed the
configured byte sum. Interrupted reads retry; short reads and I/O errors do not
produce a successful digest. Existing exclusion semantics are preserved.
There is no new trace vocabulary or private lifecycle state. Test diagnostics
contain counts and deterministic fixture digests, not user source or secrets.

Length/mtime observations do not prove an atomic filesystem snapshot, deny all
path/mount replacement races, or detect adversarial timestamp restoration.
D-03/D-04 still own source inventory, ignore/secret policy, incremental parsing
and freshness; C-07/J retain durability/platform qualification. No per-file
wall-time or byte-size RAM accounting for path strings is claimed by this repair.

## Validation

- Baseline `cargo test --locked -p rrd-engine --lib project_inventory_preserves_the_version_one_digest_grammar`:
  1 passed on the original implementation.
- `cargo test --locked -p rrd-engine --lib engine::installation::inventory::tests`:
  5 passed after replacement. Covers digest equality, entry limits 0/1/7/24/25,
  nested directories, Unicode/empty input, creation order and exclusions,
  configured byte refusal, a never-ending reader capped at 0/1/65,537 bytes,
  interrupted/short reads and observed file growth/shrinkage/timestamp changes.

The first replacement compile failed because test branches used different
fixed-size byte arrays; using string slices corrected the fixture type without
changing runtime behavior. Subsequent focused compilation and tests passed.
Further validation on the bounded-inventory candidate:

- `cargo test --locked -p rrd-engine --all-targets --no-fail-fast`:
  175 passed across 12 targets, zero failures/ignored.
- `cargo clippy --locked -p rrd-engine --all-targets -- -D warnings`: passed.
- `cargo test --locked -p rrflow-cli --test installed_lifecycle`:
  5 passed against the real Windows executable, including install, authenticated
  readback, restart, backup and restore.
- `python scripts/ci/check_documentation.py`,
  `python scripts/ci/check_workflow.py`,
  `python scripts/ci/test_execution_inventory.py` (4 tests),
  `python -m unittest scripts.knowledge.test_navigation scripts.knowledge.test_export`
  (17 tests), `cargo fmt --all -- --check`, and `git diff --check`: passed.
- Navigation and execution inventory were regenerated with
  `python scripts/knowledge/render_navigation.py` and
  `python scripts/ci/build_execution_inventory.py`; their final `--check`
  commands pass before commit.

Not run locally: full workspace optional-feature/SDK matrices, other native
platforms, million-file/RSS benchmarks, complete hostile-filesystem race or
process-kill campaigns, incremental attunement and model-quality evaluation.
The exact-candidate CI chain and merge remain required on PR 40.

## Checklist and remaining work

- [x] Baseline, owners, current implementation and diff inspected.
- [x] PR brief and implementation traceability map recorded before edits.
- [x] Existing digest characterized before moving/replacing code.
- [x] New module and focused bounded/negative tests reviewed.
- [x] Research, trace/resource choices and surfaced compile failure recorded.
- [x] Owning package, lint, Windows installed lifecycle and final policy results recorded.
- [ ] Exact-candidate CI and merge receipt recorded on PR 40.

POAM-006 records the verified resource-admission gap and its bounded repair;
its status and all D-01/D-03 release checkboxes remain open. No signed release,
full project indexing, PostgreSQL/ClickHouse migration, SWAG controller, or
production scale claim is made. Final candidate/merge evidence is attached to
[PR 40](https://github.com/rrflow/rrflow-development/pull/40); no official
repository promotion is authorized or performed.
