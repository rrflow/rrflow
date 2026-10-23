# C-07d checkpoint projection corrections

**Status:** active implementation evidence; candidate acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07d-checkpoint-projection-correction`
**Owner:** [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02

Starting revision 08b613499ef408b1095949d59ef3a42fd729d9f1, PR 77.
Pre-edit comments 6361113190 and 6361208894 bind these corrections.
Combined candidate 39925765711 jobs 120322284722/120322284709 exposed two
explicit installation TOML fixtures missing checkpoint_entries, a stale
OpenAPI digest constant and one missing test-only source-closure occurrence.

Read installed_lifecycle's complete explicit storage fixture and plan checks,
engine/tests/installation explicit configuration and seal checks,
public_contract OpenAPI derivation/hash comparison, the source-closure
allowlist and native_checkpoint_quota_preserves_snapshot_state_and_reuses_released_capacity.
Both fixtures now explicitly include checkpoint_entries=1024. The constant
matches the previously generated/validated document SHA256
03639d8aff602c966b800becaa5b8442d0e912904cb50383f1ecee86ad32fb9a.
The source-closure count for tests/snapshot.rs increases 12 to 13 solely for
the checkpoint quota reopen oracle. No normal read path was allowlisted and
no validation, format or default was relaxed. No source relocation or new
semantic owner; research and additional tracing are unnecessary.

Actual native Windows planning first rejected the omitted field, then accepted
the exact corrected fixture and projected checkpoint_entries=1024 while leaving
the project empty. The first manual check used Cargo's multiply linked binary
and correctly failed file containment; staging a separate copied executable,
as the real lifecycle harness does, resolved that validation setup issue.
Combined changed-source tests: public_contract 39 passed (0.24s), engine library
160 passed/2 ignored (23.96s), source closure 1 passed (0.19s). Strict CLI
installed-reopen/lifecycle lint passed (11.86s) before the later session repair;
final combined lint is recorded on the candidate. Full installed lifecycle,
three-platform candidate acceptance and long capacity/soak remain required.
No gate or alpha-status change. Diff reviewed and navigation/inventory are
regenerated before commit; this correction does not constitute native quota
qualification by itself.
