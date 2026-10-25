# D-01.5 Windows startup characterization

**Status:** active diagnostic candidate; restricted native execution pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-windows-startup-matrix`
**Owner:** OBJ-07 prerequisite, D-01.5, POAM-006

Starting revision `4c01dd517fa6b37b9054b18c0563b2ca27c0ca7c`, tree
`47664613499fa301b3f5d61f916c00d6a33a2832`; [PR 52](https://github.com/rrflow/rrflow-development/pull/52)
brief amended before edits. The [private-context candidate](d-01-native-windows-process-context.md)
did not resolve interpreter startup. [Run 39667588923](https://github.com/rrflow/rrflow-development/actions/runs/39667588923)
completed private context creation and restricted-token runtime reads, then both
interpreters still exited with 0xC0000142. The failed native receipt binds binary
f55dc526ec109f420df814d3c58d6d6fc7b3a30db0a27fd29454870945a3f5f8.
The prior desktop hypothesis is therefore not accepted as the cause or repair.

## Controlled diagnostic and replacement

The existing Windows helper now performs four bounded literal Python
`-I -S -c pass` launches: normal-token/no-window, restricted/no-window,
restricted/inherited-console and restricted/hidden-new-console. Only named
cases and hexadecimal statuses are retained, with fifteen seconds per launch.
The normal-token launch is an OS diagnostic; it never runs source/network
probes, cleanup, the harness or RRFlow. No normal-token fallback exists.

Actual qualification and cleanup now use inherited-console creation on the
owned noninteractive desktop, with STARTF_USESHOWWINDOW/SW_HIDE. This replaces
forced CREATE_NO_WINDOW without changing principal, removed traversal privilege,
source ACLs, WFP, job ownership, environment or required lifecycle cases. The
matrix is retained in bounded stdout and the sanitized native receipt. A passing
minimal interpreter cannot qualify the product; direct and descendant source
and network denials and all five actual lifecycle cases must still pass.

The [Windows process API](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createprocesswithtokenw)
is the launch-contract reference. A [first-hand restricted startup report](https://github.com/deepseek-ai/deepseek-harness/discussions/8772)
motivates separating console creation from token behavior; its results do not
prove RRFlow's cause. No upstream implementation is copied. This is a controlled
native characterization within D-01.5, not a new runtime or authority.

Full helper/runner and complete diff review retain the same owners and routine
as the linked prior record. Only windows_offline.py and run_offline_lifecycle.py
implementation change; POAM-006, this journal and generated projections record
why. No production Rust or product behavior changes. Local Python compilation
and all twelve runner policy tests pass (final run: 0.324 seconds); they do not exercise
native account/desktop/policy effects. Those remain restricted to hosted CI.
No local Rust suite is repeated for this qualification-only Python change.

Baseline/brief/failure/replacement map, primary research, bounded diagnostics,
local results and unrun native checks are explicit; full diff is reviewed and
navigation/inventory regenerated: 17 indexes, 370 nodes, 1604 edges and 1326
file records; documentation validates 371 statuses/369 coordinates and workflow
policy passes. Unsupported enforcement, missing cases,
unrelated host effects or credential-bearing output stop acceptance. D-01.5,
D-01.6, D-01, OBJ-07 and POAM-006 remain open. No Windows qualification or alpha
claim follows from this diagnostic candidate.
