# C-07d native mutable limits baseline

**Status:** active implementation baseline; no acceptance or alpha claim
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-mutable-limits-baseline`
**Owner:** OBJ-02 prerequisite, [C-07](../../../roadmap/rrflow-1.0/gate-c-07.md)

This clean isolated branch starts at accepted development main
`f4e404be38a0c21a4a3665b1907bad2226f5c13a`, tree
`e10ae21641df4eeea7ac647979d4f9268bce70ec`. This differing evidence commit
allows a preimplementation draft PR. It changes no runtime policy.

Native maintenance uses soft WAL-byte/version targets and deliberately allows
one oversized atomic batch. Both ordinary open and read-only inspection
materialize recovered batches without a hard active-state budget. This
package will separate native hard accounted-byte/version caps from those soft
targets, reject impossible atomic batches before effects, and flush admissible
writes under pressure through the existing owner. Replay denial must preserve
all authoritative bytes and permit reopening under an adequate native budget.

The PR brief binds exact source/test boundaries and characterization. Accounting
is active memtable key/value/version charge, not aggregate heap or RSS. Retained
generations, queues, maintenance work/debt, whole-startup budgets and installed
configuration binding remain open C-07 work. No format, dependency, transaction
authority, product version or gate status changes are authorized by this record.
A successor journal will record the complete change checklist and actual
red/green evidence. The accepted-base reconciliation must include any intervening
PR 63/64 changes before final candidate CI.
