# D-06 endpoint configuration: accepted-main refresh

**Status:** active integration evidence; shared native capacity dependency pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/endpoint-profile-accepted-main`
**Owner:** D-06 prerequisite / POAM-006 and POAM-017 / PR113

Starting head `b94c6af0ce01fd1a1127aadd1c71460f672cbdcb` integrates public main
`f3c04e955fd8851c0fc466c45f008975e9d4c1a1` ordinarily after the PR brief declared
scope and the PR129 native-capacity dependency. The merge had no conflicts.
The [original configuration record](endpoint-profile-configuration.md) retains
its original revision, source map, failure oracle and qualification limits.

Reviewed endpoint publication/read/preview and its request/plan validation,
relevant engine tests, original integration record, and complete merge source
delta. Revision exhaustion remains rejected by validated requests/plans before
arithmetic; no speculative runtime change was made. Profile state still uses
the existing control CAS and project binding. No provider is activated, no
source is moved/deleted and no alternate configuration authority is introduced.
The canonical source, tests and generated SDKs preserve their original owner.

This integration adds this journal and regenerated navigation/inventory.
Before local checks, the shared Cargo target's contract artifact was cleared
with `cargo clean -p rrd-contract` to prevent reuse of another worktree's
public operation surface. This changes derived local build output only.

Validation after source integration:

- `cargo test --locked -p rrd-engine --lib endpoint_configuration`: four passed.
- `cargo test --locked -p rrd-server --test http_process installed_endpoint_configuration_http_flow -- --nocapture`: installed authenticated configuration/restart/revocation flow passed.
- `cargo test --locked -p rrd-engine --lib`: 176 passed, two existing child/privileged fixtures ignored.
- `cargo clippy --locked -p rrd-engine -p rrd-server -p rrd-client --all-targets -- -D warnings`: passed.
- Documentation, generated navigation/inventory, formatting and whitespace checks passed before commit.

The previous native macOS failure remains retained at job 120897885715,
log SHA256 `e8b9274a530db8e2fb2f4a1485775d8e7d38044bfa23615fd67219e1b0d8fee6`.
No new external research, runtime trace, resource limit or performance claim
is needed for a conflict-free source integration. Local tests do not prove
external-provider operation, six-language daemon conformance, native capacity
or soak acceptance. Final candidate CI and the merge queue remain required.

Checklist: existing outcome/brief, source identities, complete changed diff,
preserved source/test mapping, focused flow then owning suite, generated
projections and honest failures/omissions. C-07, full D-06, alpha and POAM
statuses remain open. The branch stays draft until the shared repair lands.
