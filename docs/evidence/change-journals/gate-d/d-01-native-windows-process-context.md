# D-01.5 Windows isolated process context

**Status:** active qualification correction; native startup still unqualified
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-windows-process-context`
**Owner:** OBJ-07 prerequisite, D-01.5, POAM-006

Starting revision `1cae203a09d2d0df82477480f5ea5a542b6c41c7`, tree
`a398639dfb46a157101ce6882ddffbb69573db38`; [PR 52](https://github.com/rrflow/rrflow-development/pull/52)
brief amended before edits. This follows the
[Windows mask correction](d-01-native-windows-mask-policy.md).

## Observed failure and bounded response

[Run 39666133452](https://github.com/rrflow/rrflow-development/actions/runs/39666133452)
passes mask admission and reaches token-owned process creation. Both the main
interpreter and same-principal cleanup interpreter terminate before Python
with status 3221225794 (0xC0000142). The retained failed receipt has no inner
Python diagnostic and binds rrflow.exe SHA-256
8022084e8202737f325a5626739217787dbf531c390adbaffe8a91d97fe32d85.
Source/network probes and the actual isolated product did not execute; account,
ACL and WFP setup alone are not enforcement proof.

The Windows qualification helper now owns a unique create-only noninteractive
window station and desktop with explicit protected security descriptors for
System, Administrators and the disposable test SID. The parent process station
and thread desktop are restored immediately; owned object handles remain until
after the jobs are quiesced. Both token-owned launches name the private desktop.
No existing desktop ACL is explicitly edited, and no visible desktop is switched.

A bounded read probe impersonates the actual restricted token and checks only
the known interpreter, encoding module, staged runner and loaded runtime DLLs.
It restores the caller token in finally. This distinguishes filesystem startup
access failure from later initialization without granting more file access.
Launch errors now identify the phase and hexadecimal status. Source deny ACLs,
removed traversal bypass, scoped dynamic WFP, owned jobs, scrubbed environment,
finite deadlines and same-principal cleanup are unchanged. No local desktop,
account, ACL or policy mutation was run on the developer workstation.

## Research and traceability

Microsoft's [token-owned process API](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createprocesswithtokenw)
documents implicit desktop permission handling when lpDesktop is absent and
caller-owned access when explicit. Its [initialization troubleshooting](https://learn.microsoft.com/en-us/troubleshoot/windows/win32/user32-kernel32-not-initialize)
identifies desktop access as one possible initialization failure cause, not a
proof of this failure's root cause. The [window station API](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowstationw)
and [desktop API](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createdesktopw)
define owned handles and security attributes. Constants were checked against
the locked windows-sys 0.61.2 bindings. No upstream runtime is copied.

The complete windows_offline.py helper, offline runner/caller and latest failed
native log/receipt were reviewed, along with the D-01 owners and existing
change routine. Only windows_offline.py implementation changes; supporting
changes are CI operations, POAM-006, this linked journal and generated indexes
and inventory. No production code, runtime dependency, format or authority
changes. The launch path is replaced at its existing canonical helper; no
fallback that skips enforcement is introduced. Private-desktop success is an
inference to test, not a claimed repair until native CI proves it.

## Validation, checklist and remaining work

Python compilation passes. Navigation checks 17 indexes/369 nodes/1601 edges,
inventory checks 1325 records, documentation checks 370 statuses/368 coordinates,
and workflow policy passes. The existing twelve offline policy tests pass in
0.330 seconds. Read-only local ABI construction verifies a 24-byte security
attributes structure and explicit startup desktop field; no mutation API was
invoked. These checks cannot prove Windows launch or isolation. The full native
candidate pipeline, actual interpreter startup, permission/network probes and
five-case product corpus remain required.

Baseline and owners reconciled; pre-edit PR brief bound; exact native failure
retained; source/replacement boundary reviewed; primary research and resource
limits explicit; local checks and omitted native checks distinguished; full
diff reviewed; journal and generated projections included. Stop on missing
privilege/enforcement/cases, unrelated host effects, escaped fixture paths,
secret-bearing diagnostics, or a production-boundary defect. D-01.5, D-01.6,
D-01, OBJ-07 and POAM-006 remain open; no alpha or native Windows pass claimed.
