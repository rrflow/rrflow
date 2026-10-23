# C-07e cursor contract count correction

**Status:** active implementation evidence; candidate acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07e-cursor-contract-correction`
**Owner:** [C-07e](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02

Starting revision 2625a275a0409168c41cf9735c0df20fc57012a7, PR 74;
pre-edit comment 6361208416. Combined candidate 39925765711 job
120322284709 exposed obsolete endpoint-count assertions: the real catalogue
and generated corpus contain 44 operations, while two Rust assertions said 43.
Reviewed both full conformance functions and catalogue-derived OpenAPI check.
Changed only those two expected counts in public_contract.rs; sorting,
transport neutrality, exact operation/model conformance and negative tests
remain. No runtime path, public operation or protocol version changed.

The combined source with checkpoint digest corrections passed all 39
public_contract tests (0.24s). This was local Windows changed-source
characterization, not candidate CI. Earlier .NET and reference corrections
remain separately recorded. No new research, trace or resource measurement is
needed for this mechanical projection. Full diff reviewed; generated inventory
and navigation are regenerated before commit. C-07 and alpha remain open;
exact candidate CI is required before merge.
