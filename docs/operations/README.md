# RRFlow operations

**Status:** active operations index
**Coordinate:** `rrflow://rrflow-instance/data/operations-index/rrflow-operations`
**Owner:** operator-runbook and repository-operations discovery; linked from `docs/README.md`

Operations records explain how an operator runs or verifies the checked-in
system. They do not define RRFlow semantics, release order, acceptance status,
or a second lifecycle authority. Architecture belongs under `architecture/`,
stable interfaces under `reference/`, gate status under `roadmap/`, and
measured results under `evidence/`.

| Subject | Durable warp | Checkout record | State |
|---|---|---|---|
| Candidate CI, diagnostic workflows, and private runner administration | [`rrflow://rrflow-instance/data/operations/ci`](rrflow://rrflow-instance/data/operations/ci) | [`ci.md`](ci.md) | active checkout behavior and operator procedure; not RRFlow alpha or release evidence |
| How an AI session starts, works, claims results, uses machine resources and ends | [`rrflow://rrflow-instance/data/operations/agent-sessions`](rrflow://rrflow-instance/data/operations/agent-sessions) | [`agent-sessions.md`](agent-sessions.md) | active operational contract; enforced where a check is named |
