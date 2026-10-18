# RRFlow version policy

**Status:** active release declaration policy
**Coordinate:** `rrflow://rrflow-instance/data/reference/release/version-policy`
**Owner:** product-version declaration, mirroring, and change control

The repository root [`README.md`](../../../README.md) remains the sole authority
for product identity and current maturity. The
[release roadmap](../../roadmap/rrflow-1.0.md) owns completion and readiness.

RRFlow's canonical current release-train version is `1.0.0`. The checked-in
[`VERSION`](../../../VERSION) file is the human- and automation-readable source
of truth.

The version is intentionally frozen while RRFlow converges on its first alpha
baseline. Pre-alpha, alpha, optimized, stable, and releasable are evidence
states recorded by the objective and roadmap; they are not inferred from the
numeric version. Do not increment or decrement `VERSION` to advertise partial
progress. The repository owner must explicitly authorize any later version
change after the alpha objective, optimization evidence, and release decision
are complete.

All Rust workspace crates inherit the same value from `[workspace.package]`.
The supported TypeScript, Python, Java, and .NET client packages use the exact
same release version.

Connectome is a separate repository and participates in the same public
release train. Its own version gate must verify its JavaScript and Tauri
manifests locally. RRFlow pins the optional root `connectome/` submodule but
does not mirror or inspect its product versions as engine declarations. It
does not inspect a sibling checkout or support a second, mounted
`apps/connectome` layout. A submodule revision update does not authorize a
product-version change in either repository.

The product release version is independent of internal protocol, contract,
fixture, persisted-format, and schema identifiers such as `v1`. Those are
independently versioned technical identities; they do not announce a new
RRFlow or Connectome product release and, during pre-release convergence, do
not authorize a reader, alias, or migration path for superseded RRFlow state.

## Change control

- No agent, automation, dependency update, or generated-code task may change
  `VERSION` or a mirrored package version as a side effect.
- A version change requires the repository owner's explicit approval in the
  pull request that changes `VERSION`.
- The pull request must state the old version, new version, compatibility
  impact, migration requirements, and release/rollback plan.
- [`scripts/check_version.py`](../../../scripts/check_version.py) and CI reject
  version drift. [`.github/CODEOWNERS`](../../../.github/CODEOWNERS) requests
  owner review for every version authority and guard. Repository branch
  protection must require Code Owner review for that approval rule to be
  enforced by GitHub.

The RRFlow release gate verifies only declarations owned by this repository.
Connectome conformance and version alignment are enforced in Connectome before
the matching release is declared.
