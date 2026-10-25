# C-07g: backup page admission

**Status:** active evidence record; native comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-backup-page-admission`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [89](https://github.com/rrflow/rrflow-development/pull/89)

## Brief and replacement

Starting revision `d00f67d9a0d43534d75da37446d05f44a9d2ebe0` integrates
PR88 from fetched main. The pre-edit PR description binds the change brief.
Three backup consumers requested fixed 1,024-entry control-journal pages even
when that page exceeded configured native byte admission. `ControlJournalPages`
now halves the acquisition count only on typed output-buffer or batch-allocation
byte denial. At most ten reductions reach one entry; an indivisible entry
still fails. Each reader retains its learned count across subsequent pages.
Other pressure, corruption and I/O errors return immediately. No durable effect
is retried, no native ceiling is raised and explicit public journal requests
retain their denial semantics.

The existing immutable `SnapshotStore` retains the backup cut. Catalogue capture,
control archive capture and restored-journal verification retain their existing
watermark, continuity, hash-chain, identity and publication checks. Archive
formats, public contracts, dependencies and engine authority are unchanged.

## Scope, review and decisions

Changed `crates/persistence/rrd-store/src/backup.rs` at the three journal consumers
and the private page helper; added two public native integration histories in
`tests/backup_catalogue.rs`. This journal, POAM-035 and generated navigation and
inventory record the package. No implementation was moved or deleted.

Read README, alpha objective, C-07, POAM-035 and authoring/traceability procedures;
backup creation, catalogue and control capture, restore and verification;
control repository and journal tests; projected-reader byte resources/native
scan; snapshot capture; engine backup composition; backup catalogue fixtures
and Linux08 receipts. Complete changed functions, test file additions and diff
were reread. Mechanical identities are generated, not hand-maintained here.

Research/source adaptation: none required for this existing local storage
contract and reproduced failure. Trace/resource/debug decision: preserve actual
byte ceilings and native receipts; no new raw-content logging. Stop on lost
snapshot/chain verification, unbounded retry, relaxed limits, changed publication
semantics or scope expansion.

## Failure and acceptance

The first native regression used 96 audit transitions and an 8 KiB projected
output ceiling. Fixed paging failed with `OutputBufferBytes`, observed 8812.
After the change, application backup, archive verification, configured restore,
restored verification and exact state survive source and restored-root reopen
under the same options. An explicit 1,024-entry public read still fails.
The indivisible-entry test first expected output-buffer denial alone; the actual
native error was `BatchAllocatedBytes` (8192 / 33077). The assertion now covers
both typed page-byte resources; no runtime ceiling or success criterion changed.
It proves no backup entry or restored target is published, no native transaction
owner leaks, and explicit larger configuration permits exact recovery.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-store --test backup_catalogue application_backup_pages_fit -- --nocapture` before repair | Failed: 8192 / 8812 output bytes |
| Same focused command after repair | Passed |
| `cargo test --locked -p rrd-store --test backup_catalogue -- --nocapture` | 16 passed after the typed-error assertion correction |
| `cargo test --locked -p rrd-store` | 228 passed |
| `cargo test --locked -p rrd-engine --lib backup` | Five passed |
| `cargo clippy --locked -p rrd-store --all-targets -- -D warnings` | Passed |
| `cargo fmt --all`, navigation/inventory generation and checks, documentation check, `git diff --check` | Passed |

Initial documentation commands exposed a UTF-8 BOM in the new journal and an
incorrect checker filename. Encoding was normalized to UTF-8/LF and the actual
`check_documentation.py` was run; all documentation/navigation/inventory/diff
checks then passed. These were authoring errors, not runtime failures.

Raw local logs remain under `target/c07g-backup-page-*`. Exact candidate CI
and native comparison remain required. Full capacity profiles, 24-hour soak
and broad workspace tests were not claimed from these bounded checks.

## Retained native failures

Linux08 ran source `4aace1353b7a12f4adb328f2551fee573ddadb1e`, ELF
`785786b7daa63757d1a0cc5e7608df2b11e6c990e858625c7bcac761c1060105`, campaign
`b91c612ec96f0a0f15f481130a3debceea4df666dd369b99d9f63a91d89f1f3a`.
All 1,000 corpus commits/readbacks and two forced reopens completed. After three
segments and 40 reconciled load operations, backup failed:
`output_buffer_bytes limit 134217728 would be exceeded by 134944192`.
Launch elapsed 407878 ms and cleanup was confirmed. The previous journal-prefix
repair alone did not fix backup at this operating point. Local campaign/result/
launch/progress copies remain in `target/c07-linux-scale-evidence-08` and complete
output in `target/c07-linux-scale08-full.log`. A small regression is not proof
of the repaired native operating point; the unchanged workload must run again.

Windows14 on source `d209218da735b26197ec9feeb3c1873eb480383b` failed after
497 segments and 5000733 ms; cleanup was confirmed. Campaign
`182a12b50abb5c9dba6617db5ee71cb66b81efd25d48bde0c63e64c8ddcd8fbf`, harness
`0bb009f8ec2c940305e7bf40b746fcc5914ebb3d86c0ab489a95dac73afa589c`, failed segment
`b546437cdecd0e53f80ac470cb2face484f37ab1cdef61b7e6e2a2f44003ae3e` are retained
in `target/c07-windows-characterization-14/evidence`. Unlike the earlier overlap
bookkeeping defect, this series had an actual 2635177000 ns gap with the same
process identity. The 2-second criterion is unchanged. Completed soak segments
contained 4,870 attempts, 268 driver-concurrency-limit failures, 1,156 latency
overruns and zero unresolved operations. This is failed capacity evidence,
not a 24-hour pass. Windows15 is a separate ongoing run on PR87, not this
candidate and not yet accepted.

Checklist: pre-edit brief, canonical replacement map, first failure, correction,
focused/owning/cross-boundary checks, diff reread and linked journal are recorded.
C-07, OBJ-02 and POAM-035 remain open; no alpha, release or promotion status
changes. Merge requires the exact candidate CI pipeline.
