# D-01.4 shared SDK bootstrap identity correction

**Status:** active candidate correction; full candidate CI and merge pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-bootstrap-sdk-identity`
**Owner:** OBJ-07 prerequisite, D-01.4, POAM-006

Starting revision `45159b902db953c2451e369f2d047427f0e3daf5`, tree
`7f4b2c8c5b70ef5c9817ccd3740ab7819e6a8d43`, on
[PR 51](https://github.com/rrflow/rrflow-development/pull/51).
This record supplements the immutable [bootstrap implementation journal](d-01-bundled-bootstrap.md).
The PR's correction brief was published before these edits.

## Failure and bounded replacement

[Candidate run 39653830662](https://github.com/rrflow/rrflow-development/actions/runs/39653830662)
failed its supported-SDK job before readiness. The default profile changed in
D-01.4; its digest participates in the canonical installation identity. The
checked shared SDK corpus still named `instance-d021958630bf8ce9b6073a33` and
the corresponding estate, whereas the planner produced
`instance-56c5f218cd3b36c14014abec`. The live SDK server correctly rejected that
mismatch. Passing contract decoding and generated API parity did not detect
the stale installation identity.

The first local oracle, `shared_sdk_corpus_identity_matches_bundled_installation`,
failed with the same old/new instance IDs. It uses the existing fixed SDK
installation anchor, the actual canonical planner and checked corpus, and
asserts the fresh project remains empty after preview. The correction changes
only the six identity/scope occurrences in `fixtures/rrd-sdk-conformance-v1.json`.
It preserves the live server's independent guard, all thirteen semantic domains,
requests, expected results and operation digests. Identity remains owned by
the planner. This is fixture alignment within D-01.4's existing cross-client
acceptance scope, not an HTTP, SDK or lifecycle change.

## Review and decisions

Full reads: `README.md`, `docs/roadmap/rrflow-1.0-execution/change-authoring.md`,
the shared SDK corpus, `crates/transport/rrd-client/examples/sdk_conformance_server.rs`,
`scripts/ci/run_sdk_conformance.py`, and the complete engine
`src/engine/tests/installation_bootstrap.rs`. Targeted reads covered the
installation identity seed, contract corpus-validation test, Gate D acceptance
matrix, CI operations and prior bootstrap journal. The complete two-file
implementation diff was reviewed, including all six fixture replacements.
Generated journal navigation and execution inventory are the only other
changed projections. No implementation is removed, moved or consolidated.

No new external research or upstream adaptation is needed: this failure is
fully explained by local profile identity and the live harness's exact error.
Existing CI logs and the new assertion provide the necessary diagnostics;
no production tracing, credentials, network permission or resource policy is
changed. Stop conditions remain stale or divergent corpus identity, any
weakened live guard, unintended product changes, or failed candidate evidence.

## Acceptance evidence

Windows local commands and results:

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-engine --lib shared_sdk_corpus_identity_matches_bundled_installation -- --nocapture` before correction | Failed: 0 passed, 1 failed; exact candidate-CI mismatch reproduced |
| Same focused command after correction | Passed: 1 passed, 0 failed, 0 ignored |
| `cargo test --locked -p rrd-engine --all-targets` | Passed: 207 passed, 0 failed; one intentionally ignored subprocess helper is invoked explicitly by its parent tests |
| `cargo test --locked -p rrd-contract --all-targets` | Passed: 106 passed, 0 failed, 0 ignored |
| `cargo clippy --locked -p rrd-engine -p rrd-contract --all-targets --all-features -- -D warnings` | Passed in 48.81 seconds |
| `cargo fmt --all --check` and `git diff --check` | Passed |
| `python -X utf8 scripts/knowledge/render_navigation.py` | Passed: 17 indexes, 364 nodes, 1583 edges |
| `python -X utf8 scripts/ci/build_execution_inventory.py` | Passed: 1315 path records |
| `python -X utf8 scripts/ci/check_documentation.py` | Passed: 365 statuses, 363 classified coordinates |

The local logs are `target/d01-bootstrap-sdk-oracle.log`,
`target/d01-bootstrap-sdk-oracle-fixed.log`,
`target/d01-bootstrap-engine-sdk-correction.log`, and
`target/d01-bootstrap-contract-sdk-correction.log`. Their paths are local
diagnostics, not durable release evidence. Strict lint output is retained in
`target/d01-bootstrap-sdk-clippy.log`. The full candidate pipeline must still
accept the final committed source.

All six live SDKs are not rerun locally: their qualification is the full
candidate's dedicated Linux lane with the declared language toolchains. The
native CLI lifecycle was already tested in the parent implementation and is
not repeated locally for this test/fixture-only correction; CI reruns it.
No previous failed or incomplete candidate is accepted as a pass.

## Checklist and remaining work

Owner and baseline identified; existing diff inspected; bounded correction
brief published before edits; source/fixture behavior mapped; first failure
reproduced; planner retained as identity owner; focused and owning suites
executed; diagnostics and research decisions recorded; changed files and diff
reread; linked journal and generated projections included. Candidate CI and
merge evidence belong to PR 51 and are still pending at this record's commit.

There is no roadmap or POA&M status change. The stale corpus is corrected by
this candidate; acceptance still requires all six live SDKs and every other
required CI partition at the exact final head. D-01.5/.6, parent D-01, OBJ-07
and POAM-006 remain open. This correction makes no alpha, PostgreSQL,
graph/vector scale, provider reasoning or cloud-sync claim.
