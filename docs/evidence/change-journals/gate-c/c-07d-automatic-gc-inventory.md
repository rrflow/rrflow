# C-07d automatic-GC snapshot inventory alignment

**Status:** active candidate correction; native CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-automatic-gc-inventory`
**Owner:** OBJ-02, C-07d/e, POAM-035

Baseline `387605791a2da6d5943f4ea612cc33065ba45461`.
[PR 65](https://github.com/rrflow/rrflow-development/pull/65) was updated before
this correction. CI run 39886297557, kernel-storage job 120195457301, failed
`flush_rotates_wal_publishes_manifest_and_preserves_old_snapshots` at mvcc.rs:693:
its final assertion required unreferenced WAL 1 to remain after automatic GC.
The same test had already verified exact old/new snapshot values after reopen.

The replacement assertion requires WAL 1 to be absent, while preserving WAL 2
(checkpoint), WAL 3 (current), every manifest/sequence assertion and both
snapshot readbacks. This aligns the physical retirement oracle with automatic
maintenance; it does not weaken the acknowledged/snapshot value oracle. Reads:
that complete test and surrounding flush/read APIs, compaction/inspection GC
characterizations, current CI subsystem/optional commands and the owning C-07
plan. No runtime source or effect authority changes.

The two broad Cargo test commands use --no-fail-fast to collect later target
failures in one candidate run. Exit failure, job deadlines and ci-gate remain
unchanged; no test is skipped or marked passing. Research/resource/debug: no
external adaptation; preserve full failure output, avoid repeated one-failure
feedback cycles, and retain native candidate CI as acceptance authority.

Validation: changed assertions and commands reviewed; Rust formatting and
staged diff checks pass; navigation, execution inventory and documentation
checks run before commit. No local runtime suite ran, as directed. Native
acceptance is pending. Checklist: pre-effect scope, source replacement,
independent value/physical oracles, failure evidence and not-run checks bound;
no roadmap/POAM closure. C-07f continues separately in dependent draft PR 71.
