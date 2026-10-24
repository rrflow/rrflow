# C-07g: final session-retention native evidence

**Status:** active bounded evidence; complete C-07 qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-session-retention-final-native`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [110](https://github.com/rrflow/rrflow-development/pull/110)

Starting and tested source `0f45b449ed3295e74d4ee0fd6d4492152394b5ec`, tree
`e0e18854b53b08be51a03c4fd6d0e71a0ca2bfd4`, includes the
[atomic session-retention repair](c-07g-session-transaction-retention.md) and
[reader-format fence](c-07g-session-retention-fence.md). This evidence-only
follow-up changes this journal, its POAM link and generated navigation/inventory.
The existing PR brief authorizes this final native comparison. No workload,
configuration, threshold, source or roadmap status changes.

Read the complete authoring procedure, prior implementation/fence/native
journals, session-retention module and changed session/transaction functions,
POAM-035, native launch/result records, selected resource/backup events, and
operator retention/crash scripts. Local source and observed failures suffice;
no upstream adaptation or additional runtime trace is needed. Preserve bounded
public receipts and resource observations; never retain bootstrap credentials.
Stop for failed replay, partial publication, lost effects or changed limits.

The ordinary release primary SHA-256 is
`5c460fe660bac0c4a11957e5b35bc03ad742ed7f635a932f314674f01c1d5ebc`
(135659480 bytes). Release compilation and three diagnostic classifier tests
passed in 191.48 seconds. The canonical installed sustained-capacity test
passed in 309.73 seconds; allocation launch took 310432 ms and confirmed cleanup.
The enclosing preparation/observation wrapper took 313.41 seconds.

Linux24 repeats the frozen short WSL/ext4, cgroup-v2 characterization: four
vCPUs, two-GiB engine cap, 1000 records, 5000 stored edges, 1000 512-dimensional
vectors, four clients at one request/second, and the unchanged 30-second
operation deadline. All 1000 corpus records were acknowledged and independently
verified. All 22 segments completed across warmup, measured, burst, drain and
two-minute soak, including two forced reopens, reader-pin interference and
backup under writes. Every phase reports zero failures, unresolved outcomes,
missing metrics and latency-budget violations. Backup creation took 4.495
seconds; creation plus catalogue verification took 4.910 seconds. Maximum
scheduled latency was 78.675 ms measured and 59.643 ms during the short soak;
maximum phase resident memory was 297619456 bytes. Observed compaction debt
was zero; the warm storage snapshot had 16 segments/34605512 bytes.

Campaign identity is
`73338eb253dafc809a5638ed2bab888c0327a73b1d478d69c0e0b784842d8888`.
Progress SHA-256 is
`d9489498aba8a95dc8c218be6f2622206643bb607e0b285d5829a1d434c551e6`
(1796693 bytes, 3998 records); result-file SHA-256 is
`133f316346d5573c4df91b7cd5b33f76a737bd67f99863fc72a4a82b52588de1`.
The corpus readback digest is unchanged from Linux22/23:
`d6660f60f85694bb6d98eaca518642d74bfbd9399b29c05b46bd4ebb1dc0b300`.
Safe raw artifacts, all segment records and per-file digests are retained at
operator-local `target/c07-linux-scale-evidence-24/`. Commands and logs are in
`target/c07-session-final-linux-build.log` and
`target/c07-session-final-profile24.log`; the launcher remains
`scripts/ci/run_sustained_capacity.py` and the existing installed test oracle.

On the same source,
`cargo test --release --locked -p rrflow-cli --test installed_crash --features crash-test fresh_installed_domain_boundaries --jobs 6 -- --exact --nocapture`
passed in 13.69 seconds (195.41 seconds including diagnostic build and ordinary
binary restoration). Diagnostic primary SHA-256:
`899e61c99e540b23aa39419747e249f46895488d1c30591d65403d9c3fad1665`.
All five domain boundaries (`wal_before_write`, `wal_appended`, `wal_synced`,
`visible`, `receipt_persisted`) and five compound recovery-read kills were hit.
Six logical calls/21 attempts reconciled: 16 acknowledgements, five uncertain
outcomes, six present effects, zero unresolved outcomes and zero violations.
The ordinary primary excluded diagnostic markers before testing and was
restored through Cargo with its exact SHA-256 afterward (0.15-second build).
Native stdout and stderr were retained separately to avoid the earlier host
log interleaving. The parsed receipt SHA-256 is
`b51e4a9932ce1020722611e6897a58eb68d3e0b5c5a52c0b19c6e0de718f816b`,
at `target/c07-session-final-crash-receipt.json`; raw logs use
`target/c07-session-final-crash-*.log`, with the command wrapper at
`target/c07-session-final-linux-crash.log` and ordinary distribution check at
`target/c07-session-final-ordinary.json`. No failure surfaced in these final
native runs; prior failing regressions and comparison remain linked above.

Generated navigation/inventory, documentation policy (489 statuses, 487
coordinates), `cargo fmt --all -- --check` and `git diff --check` pass.
The final journal, POAM change and full diff were reread. This completes the
package checklist's comparison, resource, receipt and failure accounting;
source tests/strict clippy remain bound to the unchanged tested source in the
fence record, rather than rerun for this evidence-only follow-up. Push only
the recorded exact branch revision to public development. CI remains pending;
this record supplies no merge receipt.

This short point has `qualification_passed: false` by design. It does not
qualify the full Desktop/Server profiles, three seeds, 24-hour soak, other
native platforms or complete crash/publication/reclamation matrix. Stored
graph/vector data is not evidence of ANN quality or complete reasoning.
The preceding backup deadline failure remains documented; this comparison
does not establish the cause of earlier generic substrate errors. C-07 and
POAM-035 remain open. Exact-candidate CI and merge remain required.
