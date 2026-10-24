# D-01.5 Windows source-object denial

**Status:** active native candidate; hosted enforcement pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-windows-source-acl`
**Owner:** OBJ-07 prerequisite, D-01.5, POAM-006

Starting revision `ed783c7d89e01690e18464689899a01d354a3761`;
[PR 52](https://github.com/rrflow/rrflow-development/pull/52) brief amended before
implementation. The [startup matrix](d-01-native-windows-startup-matrix.md)
in [run 39669195049](https://github.com/rrflow/rrflow-development/actions/runs/39669195049)
passes the normal-token no-op and fails all three traversal-stripped variants
with 0xC0000142. This disproves console selection as the repair. The failed
receipt binds binary fca2785b8a89d191ea75a7aa3a2be58a9e7b47f54664aa087c9cbe613456ac4b;
no source/network probes or product cases ran under that token.

## Replacement and traceability

The same CI-only `windows_offline.py` principal/job/WFP boundary now retains the
standard user's traversal privilege and replaces root-only denial with inherited
per-principal denial throughout each declared source tree. It inventories every
object without following reparses, bounds each tree to 200,000 objects and 120
seconds, fingerprints original DACLs, and requires native AccessCheck denial of
content/listing/execution/mutation/delete/ACL-write/owner-write rights on every
object before any product process starts. The verification pass has its own
120-second deadline. Protected children or explicit grants that defeat inherited
denial fail qualification; the helper never repairs them or silently skips them.
After restoring the original root ACL, a complete inventory must match every
original path and ACL digest. Unknown concurrent source changes fail readback.

Native fixtures in the hosted runner prove a three-object inherited denial,
rejection of a protected child retaining access, and restoration after success
and rejection. The prior four-case diagnostic is replaced by one bounded no-op
startup after complete enforcement. All direct and descendant source/network
probes, WFP IPv4/IPv6 denial, usable loopback, scrubbed environment, private
desktop, owned suspended job admission and five actual lifecycle cases remain
required. Startup counts/status are retained in success or failure receipts.
Normal-user execution without complete source denial is not a fallback.

The [SetNamedSecurityInfo reference](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-setnamedsecurityinfow)
specifies inheritance propagation; [ACE ordering](https://learn.microsoft.com/en-us/windows/win32/secauthz/order-of-aces-in-a-dacl)
explains why root inheritance alone cannot prove denial. The
[AccessCheck contract](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-accesscheck)
requires an impersonation token and owner/group descriptor fields; both are
provided. No upstream implementation or product dependency is copied.

## Review and evidence

Full implementation reads: `windows_offline.py`, `run_offline_lifecycle.py`,
`test_offline_lifecycle.py`; the complete current helper diff and changed CI
operations section were reviewed. Owner/procedure reads: README bootstrap and
status sections, AGENTS, execution-map portal and complete change-authoring
procedure, Gate D acceptance matrix, POAM-006 and preceding linked native
journals. The existing PR owns the broader complete package read set.

Changed implementation: Windows qualification helper and receipt projection in
`run_offline_lifecycle.py`; changed supporting owner: `docs/operations/ci.md`.
This journal, a POAM-006 note and generated navigation/file inventory record the
replacement. No Rust, public API, storage, authority, dependency or version change.

Commands/results:
- `python -X utf8 -m unittest discover -s scripts/ci -p test_offline_lifecycle.py`:
  12 passed, final run 0.416 seconds.
- `python -X utf8 -m py_compile scripts/ci/windows_offline.py scripts/ci/run_offline_lifecycle.py`:
  passed.
- Read-only native ABI characterization via `python -X utf8 -`: OpenProcessToken
  with QUERY/DUPLICATE, DuplicateToken(SecurityImpersonation), source_security on
  README returns a 64-character ACL digest; AccessCheck with that same current-user
  token correctly rejects the assertion that this readable source is denied.
  No account, ACL, desktop or policy mutation ran on the workstation.
- `git diff --check`: passed.
- Navigation: 17 indexes, 371 nodes, 1607 edges; inventory: 1327 records;
  documentation: 372 statuses/370 coordinates; workflow: 3 workflows, 7 job
  families, 5 suites, 20 default/6 optional packages and 19 pins. All pass.

Hosted fixtures and the actual five-case isolated Windows run are not locally
executed; they require the ephemeral hosted runner. No Rust suite is repeated
for this Python/qualification-only replacement. Acceptance requires all native
receipts and the complete exact-candidate CI; compilation is not a substitute.

Checklist: baseline/brief/owner recorded before effects; failed native oracle
retained; old behavior and canonical replacement mapped; primary API research
and finite nonsecret diagnostics selected; full changed-file/diff review; local
checks and unrun native checks distinguished; linked journal and generated
projections supplied. Stop on incomplete/unsupported denial, reparse escape,
unbounded work, source restoration mismatch or credential-bearing output.
D-01.5, D-01.6, D-01, OBJ-07 and POAM-006 remain open; no alpha claim follows.
