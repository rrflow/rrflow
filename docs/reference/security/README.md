# RRFlow security reference

**Status:** active security-reference index
**Coordinate:** `rrflow://rrflow-instance/data/reference-index/security`
**Owner:** identity, credential, authorization, session, audit, and transport-security discovery

RRFlow Security is enforced through `RrdEngine`; it is not a separate database,
transport policy, or provider lifecycle. These records define security
vocabulary and behavior without changing the single-engine authority owned by
the [system overview](../../architecture/system-overview.md#security-boundary).

| Subject | Durable warp | Checkout record | State |
|---|---|---|---|
| Security authority | [`rrflow://rrflow-instance/data/reference/security/authority`](rrflow://rrflow-instance/data/reference/security/authority) | [`authority.md`](authority.md) | implemented foundations; single-transaction, stamp, bootstrap, and release convergence remain open |
| Local-engram authorization | [`rrflow://rrflow-instance/data/reference/security/local-engram-authorization`](rrflow://rrflow-instance/data/reference/security/local-engram-authorization) | [`local-engram-authorization.md`](local-engram-authorization.md) | useful fail-closed constraints; current file policy is a second authority and must converge directly |

Both references remain subordinate to the single security boundary enforced
through `RrdEngine`.
