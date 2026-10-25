# C-07f allocation-sync candidate: retained native failure

**Status:** active failure and integration evidence; capacity qualification open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-allocation-sync-candidate`
**Owner:** C-07f / OBJ-02 / POAM-035 / PR112

Candidate `399e2d9a92c68592f88382fdf43da2c551c67f5e` failed macOS
[candidate job120739861437](https://github.com/rrflow/rrflow-development/actions/runs/40049710974/job/120739861437).
At existing-project `wal_before_write`, the selected pause reported 2,660,375
payload bytes. The filler wrote 252,706,816 bytes and observed native ENOSPC
(28). The public request returned ResourceExhausted, but the required selected
native I/O receipt never arrived within 60 seconds. The harness killed/reaped
its child, failed the test and detached the owned image. This is not successful
native capacity qualification. Public ResourceExhausted alone cannot prove the
selected syscall failed, nor is the harness's later SIGKILL evidence of OOM.

The full failed log context and complete selected WAL write, native observer,
private pause/resume, domain scope, filler ownership/synchronization and
space-admission paths were reviewed. No single root cause is established by
this evidence. The original metadata synchronization repair remains intact;
no timeout, file-count limit, native-error oracle or engine durability barrier
is changed merely to obtain a green check.

At the owner's rework request, merged public main `66464e35` into this branch
cleanly. This imports accepted project-to-learning documentation and changes
no capacity source. PR112's landing brief precedes this evidence update.
Changed paths: this new journal, POAM035's linked failure record, generated
journal navigation and file inventory. No implementation deletion/move,
new research algorithm, runtime trace, product-version or gate-status change.
The existing Apple/Rust synchronization research remains in the
[implementation journal](c-07f-filler-allocation-sync.md).

Documentation/navigation/inventory and staged whitespace checks are executed
before publication. No unchanged Rust tests were repeated locally for this
evidence-only merge. Full native candidate CI remains required. C-07 and the
24-hour soak remain open; the next capacity repair must reproduce/attribute
the missing selected-I/O event rather than equating a client error with it.
Checklist: baseline/owner/brief, failure provenance, source inspection, bounded
scope, unchanged assertions, checks/omissions and continued deficiency recorded.
