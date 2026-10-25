# C-07c installed client history baseline

**Status:** active characterization evidence; installed history extension pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07c-client-history-baseline`
**Owner:** OBJ-02 prerequisite, [C-07c](../../../roadmap/rrflow-1.0/gate-c-07.md)

Branch `agent/c07c-installed-history` starts from clean public development main
`ecfeea332e6d7a0138b2dbb6c393e0d734d50db9`, tree
`a1901442d72742d3941f8d1e998fdde43e67af0c`. PR 57 separately carries the
DuckDB/remaining-execution plan. A pre-edit brief in `target/c07c-pr.md`
defines this first bounded C-07c transport-history extension; this baseline
seed opens its review before implementation.

`cargo test -p rrflow-eval --test storage_contract --locked` passed 11 tests
on the unchanged source baseline. The actual primary-binary
`cargo test -p rrflow-cli --test installed_lifecycle --locked -- --nocapture`
was started before this seed: compilation completed, three shape cases passed,
and fresh/existing cases reached authenticated restart. Its final outcome is
pending here and must be recorded in the final journal, not inferred as passed.

Inspection found durable transaction intent, exact commit replay and typed
definitive-rejection cleanup in the engine. Existing lifecycle fixtures kill
after ACK and exercise restore, but have no independent client unknown-result
history or deliberately suppressed response. The existing C-07a ledger can
check that history without adding a new evidence authority.

The proposed scope extends existing private test helpers, native client calls,
test-only response loss and offline receipt validation. Production engine
bytes acquire no fault switch. Precise internal WAL/visibility/publication
barriers, the dedicated 100-reopen profile corpus and C-07d-h remain open.
No storage-format, native power-loss, resource-envelope or full-alpha claim
follows from this package. No unrelated working-tree changes were present.
