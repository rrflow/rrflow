# C-07g: bounded control-journal pages

**Status:** active evidence record; native comparison and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-bounded-control-journal`
**Owner:** [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035
**Pull request:** [86](https://github.com/rrflow/rrflow-development/pull/86)

## Brief and replacement

Starting revision `8efef0d981ae88c113aa2d060f57c83295a6435f` integrates
PR85 from fetched main. The pre-edit PR brief binds scope, acceptance,
traceability and stop conditions. `ControlRepository::journal_since` formerly
read the remaining META family with an unlimited row request, then selected
the journal prefix and requested page in memory. Backup repeatedly calls this
path. The replacement uses the existing `scan_space_bounded_from` reader with
the canonical journal prefix and requested count. The native projected-output
and source-buffer ceilings remain enforced; no point-read loop bypasses them.

One transaction captures the head, anchor and page. Missing requested entries,
missing/corrupt anchors, divergent stored identities and hash-chain mismatches
fail closed. The head bounds the expected page length; terminal and maximum
cursors cannot wrap. The existing chain verifier remains authoritative.
No data format, public operation, schema, dependency or lifecycle changed.

## Scope, review and decisions

Changed `rrd-store/src/repository/control.rs::journal_since`, added the
canonical prefix beside `keyspaces.rs::control_journal_key`, added private
`repository/control_page_tests.rs` and a native/reopen integration regression
in `tests/control_journal.rs`. This journal, POAM-035 and generated indexes/
inventory record the package. No implementation was moved or deleted.

Read README and the objective/roadmap/POA&M owner chain, change-authoring and
traceability procedures; complete control repository, control validation and
existing journal tests; relevant keyspace grammar and bounded scan helpers;
snapshot store, application backup, logical export, native transaction scan,
projected-read budgets and native failure records. Complete changed functions,
new tests and diff were reread. Mechanical file identities come from inventory.

Research: no external algorithm or source adaptation; use existing canonical
key and transaction contracts. Trace/resource decision: retain configured
limits and existing ownership evidence; the oracle uses an actual native
8 KiB output ceiling. Stop on altered durable semantics, relaxed admission,
unrelated scope expansion or unsupported completion claims.

## Failure and acceptance evidence

The first regression requested three entries from 96 retained control
transitions. Before the fix it failed with `output_buffer_bytes` limit 8192,
observed 8480. Afterward every page exactly matches the original entries,
including after reopening native storage. Requesting all 96 at once still
fails admission. Private tests cover missing/tampered anchor, middle and tail,
stored-key mismatch, earlier-page independence, terminal cursor, and growth
in a later META subfamily. Both memory and native stores reject corruption.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-store --test control_journal small_journal_pages -- --nocapture` before fix | Failed at native output ceiling as above |
| `cargo test --locked -p rrd-store --test control_journal -- --nocapture` after fix | Four passed |
| `cargo test --locked -p rrd-store --lib repository::control::page_tests -- --nocapture` | Two passed |
| `cargo test --locked -p rrd-store` | 221 passed |
| `cargo clippy --locked -p rrd-store --all-targets -- -D warnings` | Passed |
| `cargo test --locked -p rrd-engine --lib backup` | Five passed |
| `cargo fmt --all` | Passed |

Intermediate compilation exposed an incorrect helper name and a nested
Result in anchor decoding. Both were corrected directly; logs remain under
the local `target/c07g-journal-*` command labels. No acceptance assertion or
resource limit was weakened.

## Retained native failures and remaining work

Source `1282dca419b907eba2d186501b71655d7d3d0131` completed all 1,000 corpus
commits and verifications, plus two forced reopens, where earlier candidates
timed out during setup. ELF digest is
`5b0f11655ade440cd714903f1432de0e9c2c4a8d872c0db567223480972a4452`;
harness digest is
`eba3f03505b188a78ffb5528412028eef1e40fe5a5dfff0208499eefa3c3a03e`.
Both subsequent backup-interference runs failed after three saved segments:

| Run | Campaign SHA-256 | Launch milliseconds | Cleanup |
|---|---|---:|---|
| Linux06 | `b23e4b19d7b868b1439703d6589b91a3dcd2de584288ebea81657b445567e15a` | 388120 | Confirmed |
| Linux07 | `43225b525012c715a123a91411a21f6843be7213328377017f14a408e639d5b4` | 405542 | Confirmed |

The first outer log truncated its error. The repeat captured stdout/stderr
directly on the Linux filesystem and preserved the complete error:
`projected-read output_buffer_bytes limit 134217728 would be exceeded by 134381688`.
This is a failed characterization under the same 4-CPU/2-GiB allocation,
1,000 records, 5,000 edges, 1,000 512-dimensional vectors, four clients and
1 request/second. All 40 recorded load operations had outcomes/readback;
backup failure still fails the campaign. Local copies retain campaign,
launch, progress and result in `target/c07-linux-scale-evidence-06` and `-07`;
the full repeat log is `target/c07-linux-scale07-full.log`.

The bounded page regression reproduces the same error class at its owning
boundary. Exact native attribution and repair acceptance require the next
unchanged workload on this candidate; they are not inferred from unit passes.
Projected-read limit conversion still becomes a generic substrate/internal
error at the service boundary and needs separate typed-denial remediation.
Windows14's requested 24-hour run remains independent and incomplete.

Checklist: pre-edit brief, canonical replacement, failure oracle, correction,
focused/owning/cross-boundary checks, complete diff review and linked journal
are recorded. Navigation, inventory and documentation checks accompany commit;
exact candidate CI is required before merge. Full capacity profiles and
24-hour qualification were not completed. C-07, OBJ-02 and POAM-035 stay open;
no alpha or promotion claim is made.
