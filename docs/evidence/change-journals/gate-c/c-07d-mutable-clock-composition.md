# C-07d mutable limits and diagnostic oracle composition

**Status:** active local combined qualification; prerequisite acceptance and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-mutable-clock-composition`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

[PR 65](https://github.com/rrflow/rrflow-development/pull/65) continues from
`e4f546d1b37861e7bc8297db4a60b8ee206acffb`, containing accepted PR 64.
The [previous reconciliation](c-07d-mutable-lifetime-reconciliation.md)
records the real zero-duration assertion failure. This record preserves it
and composes the separately reviewed [PR 69](https://github.com/rrflow/rrflow-development/pull/69)
candidate `61435c152471ae7068f9ea3b08fe0b0933a2f52f`, tree
`e13a0106e4ec70eb746ff12231af1cf40acab6d5`. PR 69 candidate CI run
`39843394936` is still running; its acceptance is not inferred here.

## Scope and traceability

The existing PR brief authorizes prerequisite reconciliation and repeated
combined qualification. Native mutable accounting, write preflight, WAL replay,
read-only admission and all limits remain unchanged. The diagnostic test owner
accepts real zero durations, bounds measurements by an outer operation, and
checks exact forwarded context, bounded samples, maintenance and durable reads.
No production clock, sleep, retry, ignored test or alternate policy was added.

Four conflicts affected CI prose, POAM-035, the generated inventory and workflow
policy. Both packages' required native commands and explanatory records were
retained. The inventory was regenerated after staging resolution. Full rereads
of `write_diagnostics.rs` and `rrflow_kv_operator.rs`, the complete combined
diff, workflow/policy changes and both evidence records establish the mapped
replacement at the same test owners. README, AGENTS, the C-07 owner and
change-authoring procedure were revisited. No implementation was removed or
moved; no second authority was introduced.

The separate [clock repair journal](c-07a-diagnostic-clock.md) owns the Rust
Instant primary-source research. This composition needs no additional
algorithm adaptation or runtime trace. Existing physical I/O and durability
oracles remain responsible for actual synchronization; positive time is not
used as a substitute. Logical resource charges are not RSS measurements.

## Commands and results

Windows used isolated `target/c07d-mutable-build`:

- `cargo test -p rrd-lsm --test mutable_limits --test write_diagnostics --locked`:
  9 passed, 0 failed, 0 ignored.
- `cargo test -p rrd-lsm -p rrd-store --locked`: 357 passed, 0 failed,
  1 existing owned-child ignore across 57 result groups. Parent crash tests
  execute the child fixture. The previously failing store target passed.
- `cargo clippy -p rrd-lsm -p rrd-store --all-targets --locked -- -D warnings`:
  passed.

The earlier failing run remains evidence; this is a changed test oracle with
its own review, not a repeated unchanged run until success. No new test or lint
failure occurred. Format, version, workflow, documentation, generated navigation
and inventory, and staged whitespace checks are required before this merge
commit. An initial combined read exceeded the output budget; the relevant
procedure, C-07 owner and full two test targets were subsequently read separately.
The documentation check rejected this new record's initial status wording,
which lacked the required active/historical marker; it was corrected to active.

## Checklist and remaining qualification

The existing brief, baseline, single owner, exact changes, replacement oracle,
research/trace decisions, full affected-file/diff review and actual commands
are recorded. Generated navigation links this immutable node. There is no
objective, roadmap or POAM completion change. Stop on prerequisite CI failure,
changed runtime semantics or failed combined candidate checks.

Not run on this combination: engine rerun, final native candidate lanes,
installed configurable-budget operation, aggregate resource measurement,
actual disk exhaustion, capacity profiles or 24-hour soak. Prior engine
results do not establish a result for this source. Accepted PR 69 must be
reconciled before PR 65's final candidate is pushed and qualified. C-07 and
alpha remain open.
