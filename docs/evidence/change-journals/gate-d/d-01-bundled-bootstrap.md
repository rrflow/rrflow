# D-01.4 bundled bootstrap and complete installation preview

**Status:** active implementation candidate; full candidate CI and merge pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-bundled-bootstrap`
**Owner:** OBJ-07 prerequisite, D-01.4, POAM-006

Starting main revision `6ba9dc5a299ae283f4d890392f5e0fc008609d79`, tree
`945913a343f8c84f660db2b12be99eb8f7fd2f0e`. The characterization commit is
`205eedfbc8eb19c12d41fe31a227515a55952b97`. [PR 51](https://github.com/rrflow/rrflow-development/pull/51)
contains the pre-production implementation brief and replacement map. GitHub
refused a zero-ahead PR; the first failing test preceded remote PR creation,
as explicitly recorded in the [initial oracle](d-01-bootstrap-preview-oracle.md).
Production edits followed the open draft PR. No private promotion or direct
main push is part of this package.

## Change and traceability

The old preview omitted the actual secret-bearing root intent and locator
staging path, provided no materialized generic instructions, and hashed assets
whose checkout bytes could differ between platforms. Plan contract version 4
binds a versioned bootstrap value within the existing InstallationPlan and
InstalledEstateRecord. It declares thirteen managed paths and ten ordered
actions, distinguishing secret preparation from delivery. Product version
remains `1.0.0`; older installation plans are rejected without a fallback reader.

The generic template and explicit default/Clyffy profile assets are linked
into the engine. Profile version 2 directly replaces version 1, preserving its
deployment, configuration selection, principal/grant policy, entropy size,
inactive capabilities, attunement phases and budgets. The selected specialization
owns the seat; the plan's seat is a validated projection. Clyffy is selected
only with the existing `--profile clyffy` input. No provider, language, network
fetch or companion process becomes an installation prerequisite.

The bootstrap seals template/specialization revisions and digests, exact
rendered instructions, existing configuration/phase identities, ten initial
logical record kinds, entropy sizes, owner-only directory/secret paths and
rrflowKV format coordinates. Endpoint candidates use the existing versioned
EndpointProfile contract; both shipped profiles explicitly declare absence.
Process/network invocations and activated bindings are explicitly empty.
Initial semantic payloads are capped at one MiB each and four MiB together;
these are not physical-space or scale-performance claims. Clock and entropy
continue through existing engine authorities and the private durable intent.
The installed record, checkpoint, runtime commit, security journal and result
retain actual hashes; no secret bytes appear in the public preview.

Instructions publish at `.rrflow/bootstrap.md` through the previewed staging
path. Staging and publication are observable recovery boundaries. Existing
project instructions remain byte-identical. Inspection, installed open and
replay verify the instructions and reject drift without repair. Existing
anchored filesystem, native lease and security transaction authorities remain
in place. Initial attunement remains pending; installation is not source-index,
model reasoning or provider-activation evidence.

| Prior behavior/source | Canonical replacement and evidence | Gate |
|---|---|---|
| `assets/install/default-profile-v1.json`, DefaultInstallProfile seat and phase data | Version-2 default/Clyffy assets, specialization-owned seat, retained install/lifecycle tests and explicit default/clyffy record inspection | D-01.4 |
| Nine-path installation contract and EstateLayout | Same plan/layout extended with intent, locator pending, bootstrap and bootstrap pending; independent literal path set plus observed filesystem at every stage | D-01.4 |
| No linked instruction template | Engine-owned generic Markdown, exact staged publication/readback, altered-template and altered-installed-byte rejection | D-01.4 |
| Build-dependent include_bytes input | Asset-local LF attributes, fixed literal digest assertions and forced-autocrlf checkout comparison | D-01.4 |
| Initial semantic transaction assembled by current engine/security/seat owners | Same transaction, with explicit kind/count and payload bounds before commit; actual journal and bounded stamped semantic reads prove installed state | D-01.4 |
| Planned CLI-owned template placeholders in execution inventory | Actual engine-owned profile/template paths; no second CLI template authority | D-01.4 |

## Source review and scope

Owner reads: README, alpha objective/roadmap owner chain, Gate D, POAM-006,
agent-bootstrap, installed-lifecycle, CI operations and the change-authoring
and traceability procedures. The immutable first-oracle record retains the
starting characterization and research.

Implementation reads across this D-01 sequence include the complete existing
installation, estate-layout, admission, recovery, contract attunement/export,
engine installation/recovery/concurrency tests and actual CLI lifecycle files.
This package rereviews their changed planning, validation, staging, inspection,
receipt, fixture and denial boundaries. Complete new-file reads cover engine
installation/bootstrap.rs, contract attunement/bootstrap.rs, the independent
installation_bootstrap corpus, both replacement profiles and the generic
instructions. The storage initialization-format projection is reviewed against
all of rrflow_kv.rs and its lib.rs exports; it changes no storage algorithms.
The inventory generator is read in full before replacing its planned asset
entries. Targeted owner reads cover build_memory_estate_plan,
prepare_security_initialization, EndpointProfile, engine clock/secret intent,
rrflowKV inspection and bounded versioned reads. The complete package diff is
reviewed before commit; no unrelated worktree state is included.

Changed implementation paths are the existing `rrd-contract` attunement types,
exports, fixture and tests; engine assets/install, estate_layout, installation,
installation/recovery and installation tests; the two storage format projection
paths; and the CLI lifecycle test. New helpers remain within those existing
owners. Reference/roadmap/POA&M prose, this journal, generated journal navigation
and execution inventory are supporting records. The old profile is removed
directly. No crate, dependency, executable, lifecycle, provider registry or
runtime hook is added.

## Research and diagnostics

[Git attributes](https://git-scm.com/docs/gitattributes#_eol) and
[Rust include_bytes](https://doc.rust-lang.org/std/macro.include_bytes.html)
justify pinning linked asset checkout bytes, not normalizing operator inputs.
No upstream database code or experimental algorithm is adapted. Existing
redacted install-stage diagnostics gain bootstrap staging/publication; no
private reasoning trace is collected. No throughput/latency or database-scale
benchmark is claimed. Resource decisions are explicit input/record limits;
performance qualification remains with its owning gates.

The forced `core.autocrlf=true checkout-index` comparison now matches the index
exactly for all four owned assets (zero CRLF pairs):

| Asset | Bytes | SHA-256 |
|---|---:|---|
| default-profile-v2.json | 2991 | `4cd907c211671714e33208bc13540a50c59e6b7488184d9e7378d1d5976d3f53` |
| clyffy-profile-v2.json | 3101 | `048de57d1a8f5aec86b58cfb1533996f6bad7f6927040d69876c05fc68eeab5e` |
| default-estate-configuration-v2.toml | 413 | `aed474faf82e0a185b94d972f917170f06f27181d3ab6eafa249abe6a9e85d63` |
| templates/generic-project-v1.md | 1000 | `2df8396b905776c27f3d5c0d9b178410a792671d1c2fd50eac7c1fe7c98bc6df` |

The command used the reviewed staged index and an ignored target-directory
checkout; its JSON receipt is retained at
`target/d01-bootstrap-fixed-checkout-evidence.json`.

## Failures and corrections

- The first observed intent-path oracle failed as recorded in its own journal;
  it passes after extending the sealed managed paths.
- Initial cargo check found three ContractError/ServiceError conversion errors
  and an unused seat import; explicit conversion at the engine boundary fixes
  them without weakening contract validation.
- An initial installation run passed 39 cases, failed one and ignored the
  subprocess fixture. Its existing forged-credential test needed to forge the
  new entropy projection too before testing sealed-bundle refusal. The repaired
  test still requires a structurally valid malicious plan and zero effects.
- Initial expanded bootstrap tests passed three and failed one: readback used
  real time against the test observer's future fixed clock. Readback now uses
  the same explicit test clock; the product rollback rule stays unchanged.
- The first contract suite exposed a lease fixture using last_mut after paths
  were appended. It now locates the lease by its kind and still rejects path,
  digest, retention and missing-lease alterations.
- The first full engine run passed 143 unit cases, ignored the subprocess helper,
  then rejected a new unrestricted causal-log test read. The oracle now uses
  the existing bounded stamped read API; its architectural allowlist is unchanged.
- A later engine run passed 205 cases and failed the source inventory check
  because the deleted profile remained in the unstaged Git index. Staging the
  reviewed replacement resolved that mechanical mismatch; both architecture
  targets then passed (32 cases), followed by the full suite below.
- Python edits briefly changed four test files to CRLF. The reviewed source
  bytes were restored to their existing LF convention before the final checks.

## Acceptance results

Local Windows commands and evidence:

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-engine --lib installation_preview_covers_transient_intent_before_storage_creation -- --nocapture` | 1 passed; initial repair |
| `cargo test --locked -p rrd-engine --lib installation_bootstrap -- --nocapture` | 5 passed, 0 failed/ignored |
| `cargo test --locked -p rrd-engine --all-targets` | 206 passed, 0 failed, 1 subprocess-helper ignore; bounded parent tests explicitly launch that helper |
| `cargo test --locked -p rrd-contract --all-targets` | 106 passed, 0 failed/ignored |
| `cargo clippy --locked -p rrd-engine -p rrd-contract -p rrd-store -p rrflow-cli --all-targets --all-features -- -D warnings` | Passed |
| `cargo test --locked -p rrflow-cli --test installed_lifecycle -- --nocapture` | 5 passed, 0 failed/ignored; 214.41 seconds; actual staged Windows binary 156115456 bytes |
| `python -X utf8 scripts/ci/check_generated_surfaces.py --check` | Passed: 42 HTTP operations, 7 signal projections; public HTTP/SDK shape unchanged |
| `python -X utf8 scripts/ci/check_documentation.py` | Passed after correcting the new record status and regenerating navigation/inventory |

The golden installation fixture is generated through its existing test with
`RRFLOW_UPDATE_GOLDEN=1`; the environment variable is removed and the ordinary
contract suite verifies it again. Generated schema remains closed. Local
logs are retained under `target/d01-bootstrap-*.log`. The observed-effect
corpus checks literal file paths, exact instructions, both selected seats,
all eight control journal actions, two stamped semantic mutations, absent
provider activation, pending job state and the actual storage format. The
recovery/concurrency corpus includes both new staging/publication boundaries,
including real-process termination and retry with the original prepared secrets.

Final `cargo fmt --all --check`, documentation policy, navigation check,
execution-inventory check and both staged/unstaged `git diff --check` pass.
Navigation contains 17 generated indexes, 363 nodes and 1581 edges; execution
inventory contains 1314 records. The first final documentation pass correctly
rejected this new journal's missing active/historical status prefix. Correcting
the prefix and regenerating inventory resolves it without relaxing policy.

## Checklist and remaining acceptance

- [x] Bound OBJ-07 prerequisite, D-01.4, POAM-006 and exact stop conditions.
- [x] Attached the implementation brief before production edits; recorded the
  initial failing-test/remote-PR sequencing exception.
- [x] Mapped replacement behavior and preserved its characterization tests.
- [x] Used current canonical composition, source assets and generated projections.
- [x] Recorded research, limits, redacted trace decisions and surfaced failures.
- [x] Ran the first oracle, affected package suites and strict lint.
- [x] Record the completed primary-executable lifecycle and final mechanical checks.
- [ ] Complete exact-candidate full CI and merge through PR 51.

This candidate does not close D-01, OBJ-07 or POAM-006. D-01.5 still owns
native privileged filesystem/ACL and offline qualification; D-01.6 owns
complete candidate reconciliation. Local Windows tests are not Linux/macOS
or offline isolation evidence. Signed acquisition, source attunement execution,
provider integration/activation, graph/vector scale, PostgreSQL compatibility,
cloud sync and full alpha outcomes remain open. No performance or reasoning
completion is inferred from the bootstrap tests.

The candidate CI run and merge revision will be attached to PR 51 after this
record's commit. This immutable journal is not rewritten to turn pending CI
into a pass; later corrections require a new linked receipt.
