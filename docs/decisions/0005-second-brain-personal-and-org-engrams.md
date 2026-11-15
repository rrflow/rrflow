# ADR-0005: The second brain lives in personal and org engrams

**Status:** active accepted architecture decision
**Coordinate:** `rrflow://rrflow-instance/data/decision/0005-second-brain-personal-and-org-engrams`
**Owner:** where global, user and org preferences and memories live, how they resolve against each other, and how they reach a project's reasoning
**Decision date:** 2026-11-11

## Context

The repository owner described the second brain on 2026-11-11 as "global and user preferences that apply to all projects". In their words, reasoning is "graph reasoning driven with the second brain tuned for global/operato/org preferences depending on their account indie vs enterprise / team rbac based". They define an engram as "one instance per major project", replacing "engram". This record says engram; the documentation-wide rename is tracked as POAM-078 (pull request #357).

At `b2d0d722` none of this has a home.

- A workspace "is never an engine scope, resource, or authorization boundary" ([system overview](../architecture/system-overview.md)).
- The installed identity has no organization: "a local installation must not fabricate a tenant merely to address itself" (`crates/transport/rrd-contract/src/deployment.rs:21-22`).
- Preference resolution exists only as a description: run, then project, then global, for effort. Its own text says this is unimplemented ([LFG and SWAG reference](../reference/inference/lfg-swag.md#inheriting-effort-from-the-calling-ai)).
- No code resolves preferences or records where a value came from (POAM-077, pull request #357).
- One project reads another only through [project links](../reference/data/rrflowdb-capability.md#project-links). Links are opt-in and read-only, carry both read stamps, and are audited in both engrams. H-10 owns them, and they are not implemented.

What already exists and can be built on:

- Bi-temporal claims with supersession in the kernel (`crates/kernel/rrd-core/src/claim.rs`).
- Principals and roles with inheritance per instance (`crates/security/rrd-security/src/lib.rs`).
- [ADR-0003](0003-configurable-engram-organisation.md)'s domains inside one engram.

## Research basis

Primary sources were read on 2026-11-11.

- **Preferences and policies are different things.**
  - A preference resolves "nearest scope wins": [VS Code settings](https://code.visualstudio.com/docs/configure/settings) and [Claude Code settings](https://code.claude.com/docs/en/settings).
  - A policy is a separate kind of setting, where the most restrictive value wins and descendants can be locked. Examples: [GitHub rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets), [GitLab cascading settings](https://docs.gitlab.com/development/cascading_settings/) with `lock_<setting>`, and Claude Code's tighten-only `maxEffortLevel` ([managed settings](https://code.claude.com/docs/en/managed-settings)).
- **The effective value should explain itself.** Postgres reports source, file, line and pending restart per setting ([`pg_settings`](https://www.postgresql.org/docs/current/view-pg-settings.html)). OpenFeature returns a reason with every evaluation ([types](https://openfeature.dev/specification/types)).
- **Going from solo to team should need no migration.** A Vercel personal account is a team of one from the start ([accounts](https://vercel.com/docs/accounts)). Converting a GitHub user into an organization needs a second account ([conversion](https://docs.github.com/en/account-and-profile/how-tos/account-management/converting-a-user-into-an-organization)).
- **Hierarchical access is relationship-based, and enforced before retrieval.**
  - Permissions inherit down the hierarchy in [Zanzibar](https://research.google/pubs/zanzibar-googles-consistent-global-authorization-system/) and [SpiceDB](https://authzed.com/docs/spicedb/concepts/schema).
  - Readable scopes are pushed into the search as a filter: SpiceDB `LookupResources` ([querying](https://authzed.com/docs/spicedb/concepts/querying-data)) and [OPA partial evaluation](https://www.openpolicyagent.org/docs/filtering).
- **Memories need scopes, read-only policy, and supersession instead of overwrite.**
  - Scope dimensions: user, agent and run in [Mem0](https://docs.mem0.ai/core-concepts/memory-types); hierarchical namespaces in [LangMem](https://langchain-ai.github.io/langmem/concepts/conceptual_guide/).
  - Read-only shared blocks in [Letta](https://docs.letta.com/guides/agents/memory-blocks).
  - Bi-temporal invalidation in [Zep](https://arxiv.org/abs/2501.13956) and [Graphiti](https://github.com/getzep/graphiti).

## Decision

Accepted on 2026-11-11 by the owner, who chose option A with an added org engram, the option recommended to them.

1. **Three kinds of engram, each with its own engine and isolation.**
   - A *project engram* for each major project.
   - A *personal engram* for each user: the user's, or operator's, second brain.
   - An *org engram* for a team or enterprise account.

   An indie account is an org of one, with a personal engram and no org engram. Adding an org engram later changes no project or personal engram, so going from solo to team needs no migration.
2. **A project reaches the second brain only through project links (H-10).** These are opt-in, read-only, stamped in both engrams and audited in both. There is no cross-engram transaction, and the workspace stays outside the engine. Without an active link, a preference resolves to unavailable, never to a guess.
3. **Preferences and policies are separate records.**
   - **Preferences** resolve with the nearest level winning, highest first: run, then project, then user, then org, then core (the engine default).
   - **Policies** live in the org engram, and in a project engram for its own work. A policy is a lock, maximum, minimum, allowlist or denylist. The most restrictive applies, and a lower level may only tighten it.
   - A run that asks for a value outside a policy is refused, naming the policy. Stored preferences are checked again at resolution, so a newer policy also clamps an older preference.
   - Only roles granted in the org engram can author org policy.
4. **Every resolution explains itself.** For each level the resolver reports:
   - the value, and the source record with its stamp;
   - the winner;
   - any clamping policy;
   - one reason: `DEFAULT`, `INHERITED`, `OVERRIDE`, `CLAMPED`, `LOCKED` or `REFUSED`.

   The explanation is written into the run's receipt and shown in its replay ([J11](../roadmap/one-engine-joins-engineering-plan.md#4-packages-in-order)).
5. **Preferences are graph memories with supersession.**
   - A preference or memory is a bi-temporal claim linked to the entities it concerns. A change supersedes the old value instead of overwriting it, so a replay at a stamp shows the preference that applied.
   - New memories land at the narrowest level. Moving one up to the user or org level is a governed write.
   - Hard erasure is a separate, governed forget.
6. **Retrieval is filtered by access before it runs.** Context assembly fetches preferences and memories in their own recorded step. The scopes the caller may read are pushed into the graph, lexical and vector paths as filters, checked at every graph hop.

## Consequences

- **What has to be built, in order:**
  - H-10 project links.
  - The preference and policy records and their resolver.
  - The explain view and its receipt.
  - The personal engram as an installation profile of RRFlow itself, so installing RRFlow also provides the user's second brain. Org engram installation and account tiers follow in Connectome (H-06).
  - The standardized preference schemas and templates that the AI addresses.
- **Who consumes the resolver:** the effort presets of [Gate G](../roadmap/rrflow-1.0/gate-g.md#aggregate-reasoning-budget-acceptance) and D-06's governed configuration.
- **The installed identity still carries no organization.** An org is an engram, not a field. The comment at `deployment.rs:21-22` therefore stays true.
- **A preference cannot grant anything.** It cannot add network access, sharing, model installation or unlimited work. This keeps the [effort-inheritance rule](../reference/inference/lfg-swag.md#inheriting-effort-from-the-calling-ai).

## Owner answers, 2026-11-11

- **Where the personal engram runs.** In the owner's words: "The personal engram runs with RRFLOW, it is just a tailored instance of rrflow for global preferences to weave into the different projects activate".
  - It is an ordinary RRFlow instance, installed and run with RRFlow itself and tailored for global preferences.
  - Every project the user activates reads it through its link.
  - It needs no separate service.
- **Team levels and per-project overrides.** In the owner's words: "they will be standardized schemas and templates to be addressed by the AI".
  - Neither one adds another fixed level to the resolver.
  - Each is a standardized, versioned preference schema with templates, which the AI fills in and applies through the same resolver, explanation and policy ceilings.
  - The schemas and templates are generated from one owner, consistent with `AGENTS.md`.

## Open question

- Where an org engram runs for a team or enterprise account. The owner's answer covers the personal engram only.

## Rejected alternatives

- **The workspace as an engine scope.** This breaks isolation that fails closed, and breaks the single authority of [ADR-0001](0001-single-engine-authority.md).
- **Preferences copied into every project engram.** Copies drift, and a change has no single owner.
- **One global preference store outside the engines.** That would be a second authority, which `AGENTS.md` forbids.
- **Option A without an org engram.** The owner rejected it, because enterprise and team accounts need org policy under RBAC.

## Links

- [Instance topology](../architecture/instance-topology.md)
- [Project links](../reference/data/rrflowdb-capability.md#project-links)
- [ADR-0003](0003-configurable-engram-organisation.md)
- [One-engine joins plan](../roadmap/one-engine-joins-engineering-plan.md)
- [Gate H](../roadmap/rrflow-1.0/gate-h.md)
