# C-07d: protected history during foreground commit maintenance

**Status:** active evidence record; native comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07d-write-history-retention`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [96](https://github.com/rrflow/rrflow-development/pull/96)

Starting revision `693f29b2fd449b5762ce62ec31572fa14f6292c4`, tree
`733c35e35ba4e22ffbef063768b8e99582e1276c`, integrates PR95 from fetched main.
The pre-edit brief binds this separate foreground gap. A writer can compact
at the soft trigger before idle upkeep sees debt. Conservative retention there
kept every overwritten physical version even though the store owns all reader
lifetimes. Idle pruning alone cannot correct that case.

## Change and traceability

`Database::commit_transaction_with_protected_snapshots` forwards an explicit
snapshot set through the existing transaction validation, owned write and
write preparation. The shared compactor applies its existing precise retention
rule at both soft-trigger and hard-cap maintenance. Registered transactions
are automatically included; supplied untracked snapshots and the durable head
remain readable. Existing standalone write/commit methods preserve every
version. No default physical API silently changes its snapshot obligation.

The store selects the explicit owner contract for both ordinary and diagnostic
commits. The same caller-measured `WriteLockWait` reaches the existing collector;
no new state owner, option file, operation, scheduler or storage format exists.
The existing conflict check, WAL publication, uncertainty fencing, limits and
error handling are retained. Protected reader lifetimes, not operator preference,
determine which obsolete physical copies may be pruned. Logical versioned keys
and semantic history remain data.

Changed `rrd-lsm/src/database.rs` transaction/write/maintenance functions,
`rrd-store/src/rrflow_kv.rs::RrflowKvTransaction::commit`, native maintenance
tests and store operator tests. Installed reference, POAM-035, this journal and
generated navigation/inventory carry the evidence. Read owner chain, C-07,
authoring/traceability and installed maintenance contracts; complete native
commit validation, write, preparation, compaction/retention functions; store
transaction/private-lock/inspection/checkpoint lifetimes; PR95 and existing
native snapshot/compaction tests. Reread changed functions and test bodies and
reviewed the complete diff. No upstream code or external research is needed to
reuse this established retention contract. Preserve native measurements and
hard limits. Stop on implicit loss of untracked snapshots, logical history,
uncertain effects or resource admission.

## Evidence

The first repeated-write oracle failed with 191 physical entries. The repair
retains 67 after 64 logical history additions and repeated overwrites/deletes,
without calling idle maintenance or explicit compaction. An old registered
transaction reads its original values throughout. After reopen, all 64 history
keys, the latest hot value, deleted-key absence and physical sequence match.
With diagnostics enabled and disabled, manifest digest and sequence are equal;
the enabled path retains all 65 expected samples. A separate native history
preserves an explicitly supplied copyable snapshot through 16 foreground
commits/compactions and reopen. Existing conservative automatic-retention tests
remain the standalone default oracle.

- `cargo test --locked -p rrd-store --test rrflow_kv_operator foreground_maintenance_bounds -- --nocapture`:
  failed before repair, passed afterward in both diagnostic modes.
- `cargo test --locked -p rrd-lsm --test maintenance foreground_commit -- --nocapture`:
  passed.
- `cargo test --locked -p rrd-lsm -p rrd-store`: 435 passed, two existing ignored.
- `cargo test --locked -p rrd-engine --lib write_admission`: seven passed,
  including admission denial, uncertainty and accepted-effect preservation.
- `cargo clippy --locked -p rrd-lsm -p rrd-store --all-targets -- -D warnings`:
  passed. Formatting, documentation, navigation, inventory and diff checks passed.

Raw logs use `target/c07d-write-history-*`. Installed Linux13
is frozen on PR95 while this package is authored; its results cannot qualify
this source. No end-to-end latency or soak pass is asserted here.

## Native and CI failures retained

Windows17 used source `7bf996cdcc6e92c3e28506ad5687f3cfd035432a`, binary
`97cc3bb81d6fe76b78cdaed0659f3381be02ae23a77a8113c902f24eaf68e890`, campaign
`df43619809879c5e9f814406c0e5866add1c53350c1578da97cb61eb3eddba71`, harness
`510e980fc5505afb4f70f85d7b60559dbad7cf9dddf05e7b4ef804b4b0c0f91b`.
It failed after 2617500 ms and 259 retained segments, cleanup confirmed.
Failed segment digest
`f38c1e47b4eaaa2fa8aaadc37b8818f06239ab4c7de8e2c3e71b816d38e699bb`
contains a 2214598500-ns gap for PID 77100 between 2604234158400 and
2606448756900 ns. The same interval advances flush count 7 to 8 and automatic
compactions 0 to 1; reported compaction input/output is 48245032/48512710 bytes.
This is correlation, not a measured causal split. Retained load segments report
zero failures and zero latency violations before the rejected segment, but the
campaign fails sampling coverage and is not a 24-hour pass. Host compilation
overlapped this run; it is not an isolated-host comparison. Local raw receipts
remain at `target/c07-windows-characterization-17/evidence`.

Read exact failed macOS job logs and the `offline-macos-latest` artifact:
PR76 job120467760484's inherited-seatbelt lifecycle exceeded its deadline and
was killed (exit -9); both lifecycle receipts existed, but neither complete
storage-history receipt did. The outer wrapper returned 1. The artifact is
retained locally at `target/pr76-offline-macos/rrflow-offline.json`;
job120467760417 lacks its selected native I/O receipt after filler ENOSPC.
PR79 job120418709911 and PR82 job120443632745 exceed the 25-second metadata-fill
deadline at manifest/current creation respectively. These are recorded failures,
not policy-denial substitutes for actual native exhaustion. PR72's 22 test jobs
passed while its required final gate remained queued; auto-merge was rejected
because the repository does not enable it. No repository rule was changed.

Checklist: pre-edit brief, owner and replacement map, red/green/protected
histories, exact failure retention, diff review and linked journal are recorded.
Final candidate CI, native write/backup/drain comparison, full profiles and
24-hour soak remain required. No roadmap/objective/POA&M closure or alpha claim.
