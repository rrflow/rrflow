# Security policy

**Status:** active repository policy
**Owner:** Angryvibes LLC, the repository owner

## Supported versions

RRFlow is pre-release. The product version is frozen at `1.0.0` during
pre-release convergence, so there is no supported-version matrix yet: only the
current `main` branch of this repository receives fixes. A signed default
distribution has not been published; source builds carry
`build.qualification: "unqualified"` and are not supported for production use.

## Reporting a vulnerability

Report vulnerabilities privately through GitHub's private vulnerability
reporting for this repository (the repository's **Security** tab, then
**Report a vulnerability**). Do not open a public issue, pull request, or
discussion for a security report, and do not include secrets, credentials, or
private data in a report.

A report should state the affected component (engine crate, adapter, SDK,
deployment manifest, or documentation), the revision tested, exact steps to
reproduce, and the observed impact. A proof of concept that runs against an
installed engram created with `rrflow install plan` and `rrflow install apply`
is the most useful form.

## Response

The repository owner acknowledges a report, confirms or declines the finding,
and records a confirmed finding in the POA&M (`docs/poam/rrflow-1.0-alpha.md`)
under a new identifier before any fix lands. The fix lands through the normal
pull-request and candidate-CI gate described in `AGENTS.md`. The reporter is
credited in the pull request unless they ask not to be.

## Scope notes

- The engine authenticates and authorizes every invocation on an installed
  engram; an anonymous invocation is accepted only before security is
  initialized, and installation initializes it.
- Plain HTTP binds to loopback only; a remote bind requires configured mTLS.
- The engine refuses an executable whose file link count is not exactly one
  and refuses project trees whose inventory changed while a plan was computed.
  Reports that depend on defeating those checks should say so explicitly.
