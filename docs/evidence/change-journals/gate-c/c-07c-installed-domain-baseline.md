# C-07c installed domain-crash baseline

**Status:** active characterization baseline; implementation pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07c-installed-domain-baseline`
**Owner:** OBJ-02 prerequisite, [C-07c](../../../roadmap/rrflow-1.0/gate-c-07.md)

Public main starts at `14c0ef5f0228f92ae3db00f25904564f1575d08f`, tree
`070eb687cb3ac413ed06e5928de456c47d0098e6`. PR 59's 17 passing candidate
jobs and 960 native component crash/recovery identities do not establish
installed domain interruption. No unrelated worktree changes were present.
The pre-edit brief in `target/c07c-installed-domain-pr.md` opens one bounded
review package before implementation.

Inspection found distinct commit-intent, domain-publication and final-receipt
writes in `RrdEngine::commit_transaction`. The proposed exact-operation scope
must select the domain write after authorized intent persistence, rather than
accidentally terminate an authentication/session write. Reuse the existing
native rendezvous source in an explicit nondefault diagnostic build of the
primary executable, compiled out of ordinary distributions. Private bounded
stdin is the owned-controller input, never a public API/header/configuration.

The brief binds exact files, replacement traceability, fresh/existing installed
oracles, negative schedules, production exclusion, native CI and stop rules.
Existing C-07 contracts and implementation inspection suffice; no upstream
source or algorithm adaptation is proposed. Test receipts are bounded and
sanitized. Full-file/diff review, first failing oracle and exact command
results belong in the successor implementation journal.

No gate status changes. Publication/reclamation composition, 100-cycle installed
profiles, hard resource limits, retained-owner safety, real capacity failures
and 24-hour sustained qualification remain required. This seed is not evidence
that any of those behaviors is implemented or accepted.
