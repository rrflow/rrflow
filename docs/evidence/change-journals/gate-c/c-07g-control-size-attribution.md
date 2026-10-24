# C-07g: control-state capacity attribution

**Status:** active diagnostic evidence; native cause confirmation pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-control-size-attribution`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [109](https://github.com/rrflow/rrflow-development/pull/109)

Starting revision `eb96d7a6f424f617c38365f1e73d9593d0186666` extends the
same draft PR's closed-label scope. Linux21 on that exact revision retained
609 acknowledgements through ordinal 608, then failed with
`rejected-api-internal-storage-substrate`. Its launch lasted 97247 ms,
completed zero measured segments and confirmed cleanup. Campaign SHA-256 is
`943f713777ee7029fe79b3c695244f60a4f256d55233a27b4db253fbd92a3cb3`;
progress SHA-256 is
`6be0d0717430cc736dc9f7b989d6377fa9409aa5a9ca8ebf058ceca8072cba10`.
Safe raw evidence and individual digests are retained in operator-local
`target/c07-linux-scale-evidence-21/`. This is another failed short
characterization, not capacity or 24-hour qualification.

Source inspection found a candidate cause: session control state includes
historical transactions, while the control validator caps each value at one
MiB. The unchanged driver rotates sessions by elapsed time; faster corpus
setup therefore retains more transactions in one session. Linux21's generic
substrate label alone does not confirm this hypothesis.

Read the owner chain, authoring and traceability procedures, the preceding
journal, complete affected classifier and tests, `ControlTransition::validate`,
and engine session replacement/transaction admission. The only source change
is a constant `control-state-size` diagnostic for the exact existing error
message in `tests/support/sustained/workload.rs`. The test constructs the real
oversized control transition and passes its error through `ServiceError`;
Internal remains Internal and a private suffix cannot match the exact label.
No error text is disclosed, no runtime limit or outcome policy changes.
No external implementation or research is needed for this local invariant.
Stop for production repairs or changed effect semantics; those require a new
bounded package. Full changed functions and diff were reviewed.

- Before the classifier change, `cargo test --locked -p rrflow-cli --test installed_reopen diagnostic_labels_identify_control_state_capacity -- --nocapture`
  failed as intended: `storage-substrate` versus `control-state-size`.
- Afterward, `cargo test --locked -p rrflow-cli --test installed_reopen diagnostic_labels -- --nocapture`
  passed all three tests (19.10-second build; tests under 0.01 seconds).
- `cargo clippy --locked -p rrflow-cli --test installed_reopen -- -D warnings`
  passed (1.49 seconds).

Logs: `target/c07g-attribution-control-{red,green,clippy}.log`. The native
comparison is still required to attribute the actual failure. Unaffected
production suites and 24-hour campaigns were not run for this classifier.
POAM-035 stays open; no roadmap, product version or alpha status changes.
Candidate CI and merge remain pending. This record supplements, and does not
rewrite, the preceding immutable attribution journal.
