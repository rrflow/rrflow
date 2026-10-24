# C-07g bounded capacity-cause diagnostics

**Status:** active implementation evidence; native capacity remains unqualified
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-capacity-cause-labels`
**Owner:** [C-07g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Starting revision: 4a811d8d. PR72 pre-edit comment 6362511459 binds this
correction. No version, public operation, outcome classification, admission
limit, retry policy or acceptance threshold changes.

## Observation and implementation

The clean combined c0a0a85c Windows characterization at four clients/20 requests
per second failed with one uncertain mixed write, sequence 1198, slot 198,
generation 3. Its API classification was ResourceExhausted; the detailed cause
was safely hashed but not distinguishable. The failed segment digest is
2d7172febfdd21a8c6af98a200c33b29a900f4eda5ed2ee629d4905c5b435703.
No particular storage quota or lost acknowledgement is inferred from that code.

`tests/support/sustained/workload.rs::client_error` now projects a fixed
allowlist of current engine error variant labels alongside the existing closed
API code and detail digest. The helper returns static constants, never message
slices. Unknown variants remain unclassified. This private test diagnostic is
not a public error protocol or evidence of absent effects; explicit abort and
independent readback still determine outcomes. Raw server bodies and credentials
remain excluded. No implementation was moved or deleted.

Reviewed files: that workload; `tests/support/sustained.rs` completion/failure
collection; `rrd-client/src/error.rs`; `rrd-server/src/http/response.rs`;
`rrd-engine/src/engine/error.rs`; existing failure-provenance journal; retained
campaign04 failed segment and campaign06 progress. Existing canonical owners
remain unchanged. No external research or source adaptation is needed. Stop on
raw detail disclosure, inferred effect resolution, or relaxed native limits.

## Validation and remaining work

`cargo test --locked -p rrflow-cli --test installed_reopen -- sustained::workload::diagnostic_labels_never_copy_server_details_or_imply_outcomes --exact`
passed one regression, 0.00s. It distinguishes capacity variants and rejects
unknown or extended names without copying private suffixes. `cargo fmt --all`
passed. Changed paths: workload, this journal, generated navigation/inventory.

The combined d470e590 evaluator suite also passed in a fresh isolated target;
reusing the earlier fleet target exposed stale cross-worktree contract artifacts,
not a source repair. The four-CPU one-request/second native campaign06 passed
warmup, measured, burst and drain and was still running its short soak when this
record was written. Neither observation qualifies the nominal profile, complete
fleet, 24-hour soak or C-07. POAM-035 retains the native failures and full matrix.
