# DevWorld: owner direction of 2026-11-16

**Status:** active owner direction record; not an accepted decision, plan or status. The [kernel contract](kernel-contract.md) remains the one active engineering plan, and promoting any item below to a package needs the owner's decision
**Coordinate:** `rrflow://rrflow-instance/data/work-package/devworld-owner-direction`
**Owner:** records, verbatim, what the repository owner said DevWorld is on 2026-11-16, and maps each part to the record that owns it or to none; the [roadmap](rrflow-1.0.md) and [POA&M](../poam/rrflow-1.0-alpha.md) keep status authority
**Verified against:** `main` at `acf8f6231` on 2026-11-16, by searching `docs/` and `README.md` for the terms named in each row

DevWorld is the owner's name for what the repository calls the second brain ([ADR-0005](../decisions/0005-second-brain-personal-and-org-engrams.md)). The name appeared nowhere in the repository before this record.

## The owner's words

> DevWorld (our secondBrain) is suppose to be planned and built in RRFLOW. Connectome is suppose to have local optimized art, and crisp visuals, so we can create social media level content via connectome showing DevWORLD (commonly was referred to as second brain) this is the operators/teams/org (clear category that actually matters) where you can either take the time to run the crawls upfront or they will run naturally overtime during idles, and if they are a team or an org, they can have set standards for dev,reearch for that org/team
> solo devs will be able to share theirs effortlessly so others in our community/world will be able to easily click and import into rrflow. If they are only using the cloud hosted version we offer UI to import that DevWorld DB. DevWorld/SecondBrain/ The concept is that we are building the universe for which we want any AI we are using frontier, local to be able to effortlessly utilize and integrate with rrflow as its reasoning engine. The concept is to have experimental configuration where we have the DevWORLD represent the operator. That will have a uniqur small viewport into its neural modal/visualizor that allows us to see a unique blend of KV,DB,QL,the colors show what is LSM vs CLD
> we have different visualizors for the governance it provides its schemas,
> the concept we are experimenting with real effective UI toggle where we can show surrealDB method. Where it just goes out against rrflow as is.
> Then we get our initial library of schemas built to test against. These should be smartly built so our prompt can trigger. RRFLOW is suppose to have a true latest and greatest researched classifier built in. LFG is getting changed to be a true intent/predictor/scout for SWAG/rrflow/clyffy any AI for that matter as it emits clear signals and can be integrated effortlessly into rrflow.

Typos are the owner's and are kept.

## Where each part is owned today

| Direction | Owner today | State |
|---|---|---|
| DevWorld is planned and built in RRFlow | The kernel contract's package list | The second brain is K11 and [ADR-0005](../decisions/0005-second-brain-personal-and-org-engrams.md); nothing records DevWorld by that name or as a build-in-RRFlow target |
| Operators, teams and orgs as the category that matters; org standards for development and research | ADR-0005: personal engram, org engram, policies that only tighten, preferences that resolve nearest-first | Decided, not implemented ([POAM-077](../poam/rrflow-1.0-alpha/poam-077.md), H-10 links) |
| Crawls run upfront, or naturally over time during idle | No record found | Open. Attunement exists for a project; a crawl scheduled in idle time for a second brain has no owner |
| A solo developer shares theirs; others click to import into RRFlow | No record found | Open. No sharing format, signing, trust or import flow exists for a second brain. [Lab definition packs](../evidence/change-journals/gate-j/lab-definition-packs.md) are the nearest pattern for a signed, patchable bundle |
| Cloud-only users get a UI to import a DevWorld DB | No repository record found | Open |
| DevWorld represents the operator, with a small viewport into a neural model that blends KV, DB and QL, with colour for LSM against CLD | [Second-brain visualizer evidence review](../research/second-brain-visualizer-architecture-research.md) covers rendering and drill-down evidence only | Open. No design, and no tier-colour contract |
| Separate visualizers for governance and its schemas | No record found | Open |
| Connectome with locally optimized art and crisp visuals, usable for social-media-level content | The [Connectome checkout boundary](../reference/client/connectome.md#repository-checkout) owns where its code lives | Open; Connectome changes land in its own repository |
| A UI toggle that shows the SurrealDB method, where a query goes out against RRFlow as is | [Comparison lab](../../eval/lab/README.md) runs systems side by side | Open as a UI toggle |
| An initial library of schemas to test against, built so a prompt triggers them | Kernel contract S8 reasoning structures (K9) | Direction exists; no initial library recorded |
| A researched classifier built into RRFlow | [Kernel contract](kernel-contract.md) S2 and K8: classifier with per-feature toggles and measured accuracy; "best published query classifier was right 54.5% of the time" | Decided as K8; the owner now says RRFlow itself carries the classifier |
| LFG as intent predictor and scout that emits clear signals for SWAG, RRFlow, Clyffy or any AI | K8 (a classification submodule that "never authorizes and never dispatches a model") | Partly. "Predictor and scout" and signals any AI can consume are new wording and need the owner's reading of K8 against it |

## Open questions for the owner

1. Does DevWorld replace the term "second brain" in the repository, or sit beside it? Renaming is a sweep like the engram rename and is not started.
2. Is the share-and-import unit a whole personal engram, a defined subset (preferences, schemas, crawled knowledge), or both? What is signed, and who is trusted?
3. Should the LFG wording in K8 change, or is "predictor and scout" the same classifier given a new role?

Nothing here has been implemented, tested or measured.
