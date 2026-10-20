# POAM-006 — installed product and attunement lifecycle

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-006`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

The source-built primary `rrflow` binary now owns deterministic install plan/apply,
installed serve, authenticated ready, read-only quick verify, and version. Exact-plan
retry recovers across eight typed durable stages without duplicating installation
authority, and the pre-canonical manifest, raw product CLI, development supervisor,
standalone security initializer, and server initializer are removed. The lifecycle is
still missing execution of its persisted pending attunement job, a committed deterministic
project-tree inventory, generic bundle-resident scaffolding and adapter bindings, real attunement phase
executors, configuration apply, full verify, canonical installed backup resolution,
repair, restore, ownership-safe uninstall, service installation, and a signed offline
platform distribution. Public apply no longer accepts installation time: the engine
seals one versioned `host_system_time` / `unverified` observation, exact retry preserves
it, recovery/replay/open enforce the format-2 rollback policy, and read-only verification
reports typed current/tolerated/exceeded/unavailable evidence. Authenticated time, OS synchronization diagnosis and
authorized correction and runtime-wide temporal high-water rules remain open.
The estate-controller
backup, prune, and restore paths resolve a managed instance through its installed locator
and distribution binding and act on the instance's own catalogue and staging roots; the
estate cannot yet register backups the served engine created, so a retention decision over
such a catalogue is refused.

## Impact

RRFlow has a coherent local pre-release startup spine, but it is not yet a repeatable
turnkey distribution. It cannot discover and attune a project durably, repair or restore
damaged state, remove only owned integration, or prove clean offline and cross-platform
operation. Unqualified process and Kubernetes controllers can still overstate readiness
or maintain lifecycle state outside the finished engine contract.

## Owning gates

D-01 through D-11, J-01 through J-05

Roadmap owners: [Gate D](../../roadmap/rrflow-1.0/gate-d.md) and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

The D-01 single-primary-executable fresh/existing lifecycle passes deterministic plan,
no-write preview, apply, authenticated ready, commit, close/reopen, and baseline read-only
verification without checkout companions; old initializer/supervisor/doctor/create-on-open
success paths are absent. Engine-owned anchor/rollback tests and the primary Linux
abrupt-kill/reopen journey are a bounded prerequisite, not closure. D-02 through D-05
prove durable attunement and inventory/index work; D-11 proves safe
repair/restore/uninstall; native clock, filesystem, packaging, and signed release
qualification pass in J.

## Native installed-process coverage

The [native lifecycle qualification receipt](../../evidence/change-journals/gate-d/d-01-native-installed-lifecycle.md)
records a verified test gap: the primary installed-process case was compiled
only on Unix and substituted a manifest file for the distribution executable.
Windows ran only three CLI argument tests. The replacement corpus binds the
actual executable and tests fresh and existing project installation, durable
commit, authenticated readback from a restarted process, and published-backup
readback. It preserves deterministic preview, configured ceilings, exact-plan
denials, live-writer exclusion and credential-redaction assertions.
Platform CI now runs that corpus explicitly. Windows owned-process
termination is not graceful shutdown qualification; signed offline bundles,
hostile-path/ACL proof and the broader D-01/C-07 matrix remain open.

The first native candidate exposed an overlapping-invocation defect: rejected
admission replaced the active thread owner, and completion identified work only
by caller coordinates. A stale completion could therefore clear newer work.
The [invocation correction receipt](../../evidence/change-journals/gate-d/d-01-native-invocation-correction.md)
records deterministic failures and their replacement evidence. Admission now
preserves the occupied reservation, reports a retryable in-progress conflict
with a completed audit, and binds completion to a private per-admission token.
This does not close durable reasoning, generic uncertain-outcome recovery,
or the wider installed-product qualification item.
