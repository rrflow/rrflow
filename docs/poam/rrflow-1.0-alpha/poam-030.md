# POAM-030 — bounded audit read across a partial final journal page

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-030`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

The [candidate correction](../../evidence/change-journals/gate-h/h-05-audit-partial-page.md)
tracks complete page consumption before declaring the control tail. Its local
memory, persistent-reopen and engine read/export tests pass for the tested page
sizes. Candidate CI and final closure reconciliation remain pending; this
deficiency and the separate POAM-029 remain open.

`SecurityRepository::audit_since` (`crates/authority/rrd-security/src/lib.rs`)
reads control-journal pages of `limit` entries and keeps audit records until it
holds `limit` of them. It marks the control tail as reached whenever the last
page it read was shorter than `limit`, including when it stopped inside that
page because the record limit was met. When the limit-th audit record lies in
the journal's final, partial page and a later audit record exists, the read
fails its head-closure check with 400 `InvalidArgument` "audit chain does not
reach its durable head", although the chain is valid.

The `rrd-security` test
`a_record_limit_met_inside_the_final_partial_journal_page_fails_the_audit_read`
reproduces it: over one valid 14-entry journal, a six-record read reaches the
head and a five-record read fails. A one-record page cannot trigger it, because
a non-empty one-record page is never partial.

### Additional scan-bound deficiency (source review)

The candidate corrects tail detection but does not independently bound the
number of control-journal entries examined. `audit_since` continues fetching
pages of `limit` control entries until it accumulates `limit` audit records or
reaches the tail; a long sequence of other control entries therefore causes
scan work to grow with journal length. The public `AuditPage` has a continuation
sequence but no explicit completion flag. A scan budget that can return an
empty nonterminal page requires a compatible continuation/completion contract
and chain validation; it cannot be inferred from the record limit alone. This
is a source-grounded finding, not a measured performance incident or a fix.
The candidate's unbounded-paging stop condition was active. On 2026-11-01 the
repository owner decided to land the partial-page fix without it, because main
already has this scan and the fix does not worsen it. The scan bound is now
[POAM-049](poam-049.md).

## Impact

`audit-read`, `audit-export`, and `diagnostics-read` reject valid bounded
reads for as long as the journal's length leaves the limit-th record in a
partial final page. On the Connectome fixture engram, a 64-record audit page
failed on every run; Connectome now requests one-record pages. Audit lineage
stays intact, but audit evidence is unavailable at those page sizes.

## Owning gates

H-05 (complete redacted causal evidence, including bounded audit read and
export). The related security-authority record is
[POAM-016](poam-016.md).

Roadmap owner: [Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

The read sets the tail only after an empty page or a fully consumed partial
page. The characterization test is inverted to expect the first five records
through sequence 11, and paged read and export over one journal return the same
chain prefix at every page size from 1 to the journal's length.
