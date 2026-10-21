# POAM-035 — installed storage failure and growth qualification

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-035`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

## Observed deficiency

At `b6052feef8df2aa86e5a866ff0c8da196bc8f2ab`, accepted C-06 supplies the
physical reader/format/cache and D-01 supplies native installed lifecycle
evidence. The [C-07 planning baseline](../../evidence/change-journals/gate-c/c-07-planning-baseline.md)
confirms passing maintenance, compaction and failure fixtures, followed by
the full local `rrd-lsm` suite. These do not complete installed storage
qualification:

- `FailureMode::StorageFull`/`Crash` return injected errors at logical
  boundaries; they do not themselves exhaust a filesystem or discard volatile
  OS/device writes.
- Maintenance thresholds permit one oversized atomic batch and do not by
  themselves establish a hard aggregate process/queue/pinned-memory limit.
- Automatic compaction retains untracked snapshot history; growth and
  reclamation need explicit retention/pin admission and sustained evidence.
- `snapshot_memory.rs` is compiled only on Linux and executes zero tests on
  Windows. A green local suite does not establish three-platform RSS bounds.
- No joined C-07 accepted matrix presently binds native faults, independent
  acknowledged histories and installed sustained-capacity results.

These are observed coverage and resource-contract gaps. This record does not
claim a reproduced lost acknowledged write or reopen an accepted C-06/D-01
requirement. POAM-034 separately owns whole-scope semantic constraint cost;
POAM-025/026 retain broader repair and observability deficiencies.

## Impact

Component success could be mistaken for safe production growth, a true
disk-full result, power-loss durability or bounded multi-estate operation.
Storage-level success cannot establish native graph/vector scale, complete
SQL compatibility, model execution or UI/provider conformance.

## Owning gates

[C-07's ordered plan](../../roadmap/rrflow-1.0/gate-c-07.md) owns remediation
through packages a-h under OBJ-02.

Roadmap owner: [Gate C](../../roadmap/rrflow-1.0/gate-c.md).

## Closure evidence

C-07 requires independent exact histories,
I/O fault and native process/exhaustion evidence, hard resource admission,
snapshot/buffer/GC lifetime, and installed scale/soak qualification at declared
profiles. Gate C must accept the complete matrix at exact tested revisions
before this row closes. Missing facilities or unrun large profiles remain
visible gaps. J-02/J-03 retain OS/power-cut and distribution-profile claims;
J-04 retains full-product competitive evidence.
