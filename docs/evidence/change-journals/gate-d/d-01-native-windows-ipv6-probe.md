# D-01.5 Windows IPv6 enforcement probe

**Status:** active native qualification candidate; final hosted result pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-windows-ipv6-probe`
**Owner:** OBJ-07 prerequisite, D-01.5, POAM-006

Starting revision `08654be83bb1442effcb0f1143da484755592dbf`;
[PR 52](https://github.com/rrflow/rrflow-development/pull/52) brief amended before
implementation. [Run 39671050140](https://github.com/rrflow/rrflow-development/actions/runs/39671050140)
passes inherited-denial/protected-child fixtures, native AccessCheck for source
trees of 8,048, 499 and 25,517 objects, standard-user startup, direct source
probes, IPv4 denial and complete ACL restoration. The isolated runner then
rejects IPv6 error 10051 (network unreachable), rather than the required native
permission denial. No actual product lifecycle cases ran in this failed isolated
attempt. Its receipt binds binary
b43acf7c95945407eabe23ad558461abd2441af5e47930816531b8a76f41de43.

## Scoped replacement

`windows_offline.py::ipv6_probe_scope` uses bounded read-only native NetTCPIP
queries to select a preferred link-local IPv6 address on a connected interface.
The closed integer scope is saved in the staged isolation configuration and
startup receipt. `run_offline_lifecycle.py::native_probe_targets` rejects absent,
zero, negative, boolean, string and oversized Windows scopes. The Windows TCP
probe uses fixed non-loopback unicast fe80::ffff:ffff:fffe with that explicit
scope; macOS retains its existing documentation-address probe. Both parent and
descendant still require EPERM/EACCES/WSAEACCES, and loopback must remain usable.
A missing interface, unreachable destination, timeout or successful connection
cannot qualify. No route/address is added and no default route is changed.

This corrects the test's global IPv6 routing assumption without changing the
principal-scoped WFP filter, whose only network exception is loopback. It does
not implement or alter a product sandbox. Existing source-object denial, owned
job cleanup, environment and five-case product requirements are preserved.

Primary research: [Winsock socket addresses](https://learn.microsoft.com/en-us/windows/win32/winsock/sockaddr-2)
for IPv6 scope, [Get-NetIPAddress](https://learn.microsoft.com/en-us/powershell/module/nettcpip/get-netipaddress?view=windowsserver2025-ps)
for native interface discovery, and [WFP ALE layers](https://learn.microsoft.com/en-us/windows/win32/fwp/ale-layers)
for TCP connect classification. No upstream code is copied. The native CI result
must establish enforcement; these references are not a substitute for it.

## Checklist and evidence

The current complete helper, runner and policy tests were reviewed, along with
the inherited source-ACL candidate, its native log/receipt, CI operations and
POAM-006. Existing README/Gate D/change-authoring owners and PR read set remain
applicable. Changed files are these three Python files, the CI operations note,
POAM-006 note, this new journal and generated navigation/inventory. No Rust,
production API, persistence, authority or version change is made.

First oracle: the new native scope-policy test fails with seven missing-function
errors before implementation; the other twelve policy tests pass. Afterwards:
- `python -X utf8 -m unittest discover -s scripts/ci -p test_offline_lifecycle.py`:
  all 13 pass, 0.300 seconds.
- `python -X utf8 -m py_compile scripts/ci/windows_offline.py scripts/ci/run_offline_lifecycle.py`:
  passes.
- `python -X utf8 -` importing `ipv6_probe_scope` and printing its result:
  read-only local native discovery returns interface 13; no socket connection,
  account, ACL, desktop, route or firewall mutation occurs locally.
- Navigation, inventory, documentation, workflow and diff checks run before
  commit; final exact-candidate CI remains mandatory.

Full changed-file and diff review, native failure/replacement traceability,
bounded discovery (30 seconds/4 KiB), primary-source decision and nonsecret
receipts are explicit. Hosted WFP and actual product cases are not run locally;
Rust suites are not repeated for this qualification-only Python change. Stop
on absent native IPv6 enforcement, incomplete source denial, failed cleanup,
missing lifecycle cases or secret-bearing diagnostics. D-01.5/.6, D-01, OBJ-07
and POAM-006 remain open. No alpha or compatibility completion is claimed.
