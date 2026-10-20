# H-06 Connectome public repository association

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-06-connectome-public-association`
**Owner:** H-06 source-acquisition follow-up receipt for the associated UI repository

## Baseline and scope

Starting revision: `e38fd32260e175ee9fef458cf681f2d3de181ce7`,
[PR #20](https://github.com/rrflow/rrflow-development/pull/20), OBJ-09 and the
H-06/H-07 source prerequisite. On 2026-10-19 the owner explicitly instructed
that Connectome become a true associated RRFlow repository and be made public.
The PR brief was updated before the current-documentation corrections.

Current to target: private source prevented anonymous recursive CI checkout;
publish the authorized UI repository, identify its association in GitHub
metadata, and replace stale current-access statements with verified public
acquisition. The UI pin remains
`ce08e2f6e1efd60ea7762598c23dc4659d0df1a3`.

Read at this boundary: README and AGENTS; the complete Connectome README and
root GOVERNANCE; client contract repository-checkout section; POAM-011; CI
candidate-chain section; prior H-06 source and policy receipts; the linked
change-authoring procedure and journal template; GitHub repository metadata,
recursive-checkout job results, and redacted history-scan findings.

Changed paths: current access text in `docs/reference/client/connectome.md`,
`docs/operations/ci.md`, and `docs/poam/rrflow-1.0-alpha/poam-011.md`; this
receipt, its generated index, and generated file inventory. No source module
was moved, deleted, or replaced. Runtime traces and resource instrumentation
are inapplicable to this metadata/documentation follow-up. Stop conditions:
unavailable pin, new engine dependency, credential disclosure, or unproved
client/runtime qualification. Prior receipts remain immutable history.

## Publication and acquisition evidence

- Before publication, Gitleaks v8.30.1 scanned all fetched branch histories:
  1960 commits and 18.15 MB. The official Windows release archive SHA-256 was
  verified as `d29144deff3a68aa93ced33dddf84b7fdc26070add4aa0f4513094c8332afc4e`.
  Command: `gitleaks.exe git target/connectome-product/connectome
  --log-opts=--all --redact --no-banner --no-color --ignore-gitleaks-allow
  --report-format json --report-path target/connectome-public-review/history-redacted.json
  --timeout 120`, from the original RRFlow checkout.
- The scan exited 1 for two findings, both reviewed: an example login in
  historical GraphQL UI source and a public PostHog browser project key in
  historical Vite configuration. Neither finding was a private access
  credential. [PostHog's primary documentation](https://posthog.com/docs/product-analytics/troubleshooting)
  distinguishes public project keys from private personal API keys. No key
  value is reproduced here; this scoped scan is not an exhaustive audit.
- `gh repo edit rrflow/connectome-development --visibility public
  --accept-visibility-change-consequences` succeeded. The same edit set its
  description to the associated RRFlow operator/developer UI, homepage to
  `https://github.com/rrflow/rrflow-development`, and topics `rrflow`,
  `connectome`, and `developer-tools`.
- `gh repo view rrflow/connectome-development --json
  nameWithOwner,url,isPrivate,description,homepageUrl,repositoryTopics`
  confirmed `isPrivate: false` and the association metadata.
- `curl.exe -q --fail --silent --show-error --output NUL --write-out
  '%{http_code}' https://api.github.com/repos/rrflow/connectome-development`
  returned HTTP 200 without authentication.
- `git -c credential.helper= -c core.askPass= ls-remote
  https://github.com/rrflow/connectome-development.git refs/heads/main`
  returned the exact pinned revision above.
- Attempt 1 of [run 39307817468](https://github.com/rrflow/rrflow-development/actions/runs/39307817468)
  failed all 16 substantive jobs at private-submodule checkout, before tests.
  After `gh run rerun 39307817468 --repo rrflow/rrflow-development --failed`,
  attempt 2 passed every recursive checkout step on Linux, Windows, and macOS.
  The full pipeline was still running at this receipt; checkout success is
  only acquisition evidence.

## Verification and disposition

Acceptance commands and results: `python scripts/knowledge/render_navigation.py`
and `--check` passed (14 indexes, 292 nodes, 1261 edges);
`python scripts/ci/build_execution_inventory.py` and `--check` passed
(1189 records); `python scripts/ci/check_documentation.py` passed
(293 statuses, 291 coordinates); `git diff --check` passed. The complete
authored diff was reread; no new failure surfaced in these checks.

Checks not rerun: Rust, frontend, runtime, hosted execution, deployment, account
and synchronization acceptance; no implementation changed in this follow-up.
The preceding package's test results remain in its receipt. Candidate CI must
pass before the submodule PR merges; no merge or official promotion is claimed.

Change checklist: explicit publication authorization recorded; fetched history
reviewed without exposing values; exact UI pin preserved; anonymous access and
recursive checkout verified; current access owners corrected; historical
receipts preserved; generated projections refreshed and diff reviewed.

Roadmap/POA&M status: the source-access blocker is resolved. H-06/H-07 and
POAM-011 qualification remain open; no product-version or runtime-readiness
change is implied.
