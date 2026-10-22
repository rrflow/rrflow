# DevPulse project runtimes and bidirectional cloud or homelab synchronization

**Status:** active integration contract; local native database path exists, account synchronization is not implemented
**Coordinate:** `rrflow://rrflow-instance/data/reference/client/devpulse-cloud`
**Owner:** DevPulse project-runtime integration and explicit bidirectional managed-cloud or self-hosted synchronization boundary

DevPulse will include the local RRFlow reasoning engine and its native rrflowDB.
Installation, local queries, durable writes, recovery and local reasoning that
uses an installed model must not require a shippin.cloud account. Hosted model
calls remain separately authorized provider capabilities. The
[local integration guide](../sdk/rrflowdb-local.md) provides a runnable native
database path now. It is not yet a packaged DevPulse application.

DevPulse is the owner's project development application. The required product
experience includes provisioning local VMs or containers for projects, running
their toolchains and UIs, and integrating each project with RRFlow. A user can
stay local, select an admitted managed service, or connect to an
operator-controlled homelab. Both remote choices must support explicitly
selected bidirectional synchronization; remote access alone enables no transfer. A
hosted account is not a prerequisite for local or homelab operation.

The available Windows development engine/Connectome bundle proves only the
bounded local behavior in its receipt. It is not a qualified DevPulse package,
managed database service, sync implementation, or competitive parity result.

## Optional managed service roles

The owner-directed delivery targets distinguish the following roles. These
names describe intended experiences, not verified domain ownership, deployed
endpoints, account federation or available service APIs:

| Experience | Planned role | Independent qualification |
|---|---|---|
| Local RRFlow and Connectome | Open-source engine with one estate per project, usable without an account. Connectome offers guided operation and learning from visible evidence. | D-01/C-07 and H-06/J installed product and client acceptance. First-party licence texts/notices remain a J-05/POAM-013 release requirement. |
| rrflow.ai | Optional managed access to hosted project engines and explicitly selected synchronization, including access from another authorized device. | H-07 account, instance, transfer and real-service acceptance; remote access is not proof of two-way synchronization. |
| rroflow.ai | Optional service for authorized learning datasets, experiments and qualified improvement artifacts exposed through Connectome. | G-06/I-03/I-07 learning acceptance plus H-07 transfer/account controls for selected data. Upload and artifact activation require distinct authorization. |
| shippin.cloud | Optional private development space that consumes RRFlow and can run project toolchains, containers/VMs and private UI previews. | H-07e/f/g project-runtime, file/environment and real-service acceptance. A synchronized database is not a working devspace. |
| Operator-controlled homelab | Self-hosted engine and optional development/learning capabilities under the same admitted public profiles. | Independent identity, recovery and transfer qualification with managed services unavailable. |

Use configured endpoint profiles, trust and credential references rather than
hardcoded domains in engine or client dispatch. Show the actual service,
account and destination estate for each operation. Linking one service does
not imply that the others share its account, can read its data or can provision
resources. If federation is later supported, its trust and audience must be
qualified explicitly. Unknown or unsupported services remain unavailable.

Connectome can offer these extensions after local operation is established.
Account creation, remote engine access, bidirectional sync, cross-project
context sharing, training export and compute provisioning are independent
choices. They may share an understandable setup flow but retain separate
previews, scopes and receipts. Default installation enables none. A failed or
revoked service binding leaves local data and operation usable.

RROFLOW may be prepared and used locally when its capabilities qualify; the
cloud is not a prerequisite for learning. Conversely, a user may choose remote
engine access without consenting to training. The
[learning contract](../inference/learning/controller-training.md#staged-learning-and-feedback)
owns candidate methods and promotion. H-07 may develop independently of Gate I;
the combined learning-service experience requires both accepted boundaries.

## Guided installation and Windows-first DevPulse

DevPulse ships first on Windows and bundles the qualified RRFlow artifact and
its public-contract identity. Standalone RRFlow with the optional Connectome
desktop must also offer guided installation on Windows, macOS and Linux.
These are release requirements; the current development bundle does not yet
prove clean-machine installation on all three platforms.

The normal flow is **Install → Choose or create a project → Start**. Advanced
configuration remains available without making compiler, CLI, database,
provider or container knowledge a prerequisite. Show one understandable
summary of the selected project, local data location, permissions and resource
budget before applying the engine's exact installation plan. Progress and
errors must describe the next useful action. The UI never invents readiness:
it verifies the installed engine, catalogue identity and authenticated session.

Docker is a project-runtime capability managed by DevPulse, never a
prerequisite for RRFlow readiness. When a project's declared runtime needs
containers, first detect an existing compatible runtime and offer its reuse.
If missing, explain the download, runtime/backend, disk requirement,
permissions and applicable terms, then let the user authorize guided setup.
Use a reviewed version/publisher/checksum manifest, invoke the supported
installer directly, and verify the running daemon and a bounded project build
or run before reporting success. Never silently accept third-party agreements,
change host virtualization settings, or mount another project's data.

On Windows, prefer the supported per-user Docker Desktop path with WSL 2 when
it fits the project. Installing Docker itself may not require elevation, while
enabling missing OS features can. Detect unsupported hardware, disabled
virtualization, an unavailable WSL backend, administrator denial and required
restart as distinct states. Save setup progress and revalidate after restart;
do not repeat completed installation or report a running project early.
Platform requirements must be checked against the selected installer release.
[Docker Windows installation](https://docs.docker.com/desktop/setup/install/windows-install/)
and [Microsoft WSL installation](https://learn.microsoft.com/en-us/windows/wsl/install).

Docker/WSL provisioning is a separately authorized DevPulse host effect;
`rrflow install apply` continues to use only the acquired RRFlow bundle.
Container placement, isolated engine instances, context-sharing consent and
cloud synchronization remain separate settings. A shared Docker daemon does
not authorize shared credentials, storage mounts or AI context.

Acceptance belongs to D-01/H-06 for guided engine/client installation, H-07
for DevPulse project-runtime adapters, and J for qualified distribution.
Clean-machine fixtures must cover all declared OS/architecture targets,
signed/publisher-verified acquisition, default local startup without an account
or Docker, existing-project preservation, canceled setup, retry, update and
ownership-safe removal. Windows-first DevPulse additionally proves Docker
already present/missing, WSL missing, denied elevation, restart/resume,
offline/interrupted acquisition, engine-ready/runtime-not-ready distinction,
and a real containerized project whose UI opens privately. Cloud opt-in stays
off throughout these local tests.

## Relationship and authority

```mermaid
flowchart TB
    DP[DevPulse project application] --> Provision[Authorized VM/container provisioning adapter]
    Provision --> LocalRun[Local project runtime and private UI preview]
    DP --> Local[Local RRFlow estate]
    Local --> LocalKV[Local rrflowKV]
    Local <-. selected native data and durable receipts .-> Cloud[Selected managed RRFlow estate]
    Local <-. selected native data and durable receipts .-> Lab[Homelab RRFlow estate]
    DP -. managed account and project provisioning .-> CloudRun[Cloud VM/container devspace]
    DP -. operator-configured identity and provisioning .-> LabRun[Homelab VM/container devspace]
    CloudRun --> Cloud
    LabRun --> Lab
    Cloud --> CloudKV[Cloud rrflowKV]
    Lab --> LabKV[Homelab rrflowKV]
```

This is the target ownership diagram, not an implementation receipt. Dotted
sync and provisioning paths require qualification. Account and workspace grouping remain
outside the engine as defined by the [instance topology](../../architecture/instance-topology.md).
Each endpoint authenticates an exact instance. A cloud account, UI or resolver
cannot become a database authorization or lifecycle authority. Separate local
and cloud instances do not imply an active-active replica or cross-instance ACID.

DevPulse groups corresponding project checkouts and their estates above the
engine. Every estate retains its own project, instance, authorization and local
commit coordinates; a sync binding maps identities between selected peers.
This is distinct from a read-only cross-project link (H-10) and from replication
inside one distributed RRD instance. Neither is a shortcut to bidirectional sync.

## Separate effect boundaries

| Concern | Authoritative operation | Required visible result |
|---|---|---|
| Project runtime | DevPulse calls an explicitly bound VM/container provider with an approved image/toolchain, mounts, ports and CPU/memory/disk budgets. Shippin or the homelab provider owns its infrastructure state. | Provision/start/stop/failure receipts, private UI URL, cancellation and ownership-safe removal. A running VM is not database readiness. |
| Native database state | Public RRFlow synchronization operations validate source identity, authorization, schema, conflicts and the complete atomic write group. | Destination durable receipt, acknowledged source checkpoint, pending/conflicting changes and per-direction lag. A live connection is not synchronized data. |
| Project source/assets | A project-inventory transfer adapter moves explicitly selected files or content-addressed assets, with revisions, exclusions and conflict handling. | Verified manifest/content digests and destination revision. Database convergence alone does not synchronize source files. |
| Environment reconstruction | Versioned environment manifests name runtime/toolchain/images and approved configuration. Target providers recreate them using explicit capabilities. | Reproducible project build/test/run and private UI preview. Live VM memory, host credentials and raw database files are not copied implicitly. |

The same versioned RRFlow implementation is packaged for each supported native
target and service image. Platform-specific binary digests differ; the release
manifest binds them to one revision and supported protocol/profile. DevPulse,
the managed service and the homelab consume those artifacts and public APIs.
They do not embed a new persistence or reasoning implementation. Process
separation keeps project code execution outside the database authority.

## Required user behavior

1. Start and use the local project without sign-in. The initial sync state is
   disabled; there is no account discovery request or background upload.
2. Offer an admitted managed-service sign-in or an operator-configured homelab
   endpoint and trust/credential binding. Creating an account or discovering a
   host alone does not authorize provisioning or data transfer.
3. Preview the exact local project/estate, remote account and destination
   instance, selected data families and files, both transfer directions, permissions, limits,
   excluded secrets and initial-transfer size. Require explicit sync consent.
4. Establish authenticated transport, verify remote protocol/instance/catalogue
   identities, then activate the authorized adapter binding. Never upload the
   local operator credential, token signing key, model-provider secrets, or raw
   database files as an implicit sync mechanism.
5. Display progress, last acknowledged checkpoint, pending local changes,
   conflicts and paused/offline status for each direction independently. Cloud
   or homelab outage leaves local work usable. Do not display "synced" while
   either direction has pending writes, failed receipts or unresolved conflicts.
6. Disconnect disables future transfer and revokes adapter credentials. Local
   data is retained; remote deletion is a separate explicit action.

## Required synchronization mechanics

The following project settings govern selection and placement independently of
the synchronization protocol.

### Project preferences, sharing, and isolation

Project requirements/preferences are versioned governed data. The user-facing
specification may compose these fields, but cannot become a second authority:
`EstateConfiguration` owns effective engine settings, `EndpointProfile` owns
endpoint bindings, H-10 records own project-link consent, H-07 records own sync
consent, and DevPulse/provider records own physical provisioning. Each view
references those identities/revisions and shows proposed versus observed
state.

Use **AI Development Profile** for the user-facing global development
preferences and **AIDevSpec** as the proposed name of their structured,
versioned representation. These are RRFlow product terms, not a claim of an
industry standard or a new executable schema. Project-scoped overrides,
process requirements, testing expectations and model preferences compose
through the existing configuration owners. A global profile is a reusable
input above estates; each estate explicitly accepts a revision. Editing the
profile cannot silently change every project's effective configuration.

Show effective values, origins, precedence, conflicts, proposed changes and
the accepted revision in Connectome. Preserve project-owned instructions;
hard policy and authorization remain independently enforced, and unresolved
mandatory conflicts block the dependent effect. `AGENTS.md` and
provider-specific instruction projections are integration formats, not extra
configuration authorities. [AGENTS.md](https://agents.md/) defines an existing
project instruction format, while [Claude Code memory](https://code.claude.com/docs/en/memory)
documents scoped instructions; neither defines this complete product profile.
Generate or reference authorized projections during the existing
[bootstrap flow](../agent-bootstrap.md#independent-engine-and-project-integration),
with ownership-safe removal and no provider hooks.

Attunement may propose profile updates or learning examples. Acceptance of a
preference is separate from training consent, model activation and cloud
sharing. The [learning contract](../inference/learning/controller-training.md)
owns those distinctions. D-06 owns effective configuration and H-06/H-07 own
its client projections; no profile UI or executable AIDevSpec schema is
implemented by this naming decision.

The UI must present separate choices with explicit scope:

| Choice | Required preview and effect |
|---|---|
| Context sharing | Private by default. Select exact peer projects, readable families/resources, purpose, expiration and revocation. H-10 offer/acceptance establishes the read-only link; common account membership does not imply consent. |
| Data synchronization | Independently select native state, project files and environment transfer, destination and directions under the existing sync contract. Read sharing does not authorize copying, remote writes or synchronization. |
| Execution isolation | Select required isolation level and eligible placement: dedicated VM/microVM, or explicitly permitted containers within a VM in a declared trust group. Show shared kernel/host boundaries, mounts, network/egress, secrets, ports, resource caps and provider enforcement. |
| Cost and capacity | Preview measured/declared CPU, RAM, GPU, storage, warm/idle policy, regional rates and estimated recurring/usage costs with currency and observation time. Unknown prices remain unknown. Apply hard limits before optimizing placement. |
| Models and knowledge sources | Show active versus candidate LFG/SWAG policies and connector requirements, locality, data disclosure, credentials by reference, budgets, health and unavailable reasons from the engine catalogue. |

A sharing graph may inform which placements are candidates, but never selects
placement by itself. Two sharing projects can require separate VMs; two
nonsharing projects may occupy one explicitly allowed trust-group VM with
separate containers and independent estate authorization. Containers sharing a
guest kernel do not provide the same boundary as separate VMs. Untrusted
mutually isolated workloads require an approved stronger isolation profile;
if the provider cannot enforce it, placement fails rather than silently
downgrading to save money.

Optimize only within approved constraints: bin packing, resource reservations,
idle suspension, cold storage and warm pools need measured contention,
wake latency and cleanup evidence. Sharing data never grants host filesystem,
process, network or credential access. Project code cannot mount RRFlow data
directories or management sockets. Account switches and restore must not attach
another project's volume. A VM is also not proof of complete security:
the actual host/guest, sandbox, patching and network controls require provider
qualification. [Firecracker production guidance](https://github.com/firecracker-microvm/firecracker/blob/main/docs/prod-host-setup.md)
is a reference for those controls, not a selected RRFlow dependency.

Show a reviewable proposal and its digest before provisioning or relocation.
User intent, engine admission and provider enforcement produce distinct
receipts; they are not one cross-service transaction. Interrupted provisioning
must reconcile only owned resources. A cost optimization cannot change sharing,
residency, secret access or isolation without a new authorized configuration.
Revoking a link stops subsequent reads; it does not promise to erase previously
exported data or stop a VM. Display these effects separately.

H-07e qualification must exercise all four sharing/separate-or-shared-placement
combinations, foreign-account/volume/network denial, resource exhaustion,
unsupported isolation, relocation interruption and per-project cost accounting.
H-10 supplies actual two-party link/revocation evidence. A UI toggle or
configuration schema does not qualify either behavior.

### Optional login and maintained updates

RRFlow authentication works locally through its configured identity authority.
Offer configured managed-service account linking as an explicit extension,
alongside a homelab endpoint; never substitute hosted identity for local engine readiness.
Sign-in lists authorized service choices and prices but enables no upload,
connector, provisioning, shared cache or paid model call by itself. Disconnect
must leave local login and local data usable.

Publish signed, digest-bound engine and adapter bundles through Gate J's
release mechanism. Preview platform/protocol/schema/format compatibility,
backup and recovery requirements, downtime, permissions and data movement.
Pin an installed version and rollout channel; check/apply updates only under
the configured authorization. No install-time fetch is introduced into the
default offline bundle. Verify the replacement and reopen/health evidence
before routing work to it; retain the old artifact and verified backup.
Do not call binary rollback safe if the new writer changed an incompatible
storage format. Provider-managed API changes likewise invalidate adapter
conformance until the admitted profile is verified again.

### Transfer protocol requirements

Use the engine's typed public mutation, changefeed and receipt capabilities.
Account authentication and user consent authorize a binding; they are not
themselves a replication algorithm. A future sync package must implement:

- a durable, engine-owned binding naming local instance, remote instance,
  managed-account or self-hosted principal reference, both directions,
  schema/profile digest, consent revision and
  revocable credential reference;
- an explicit initial snapshot/checkpoint handoff with bounded batches,
  interruption/restart and detectable retention gaps;
- source identity and original commit identity on every batch, an immutable
  content digest and a destination idempotency key derived from those identities;
- destination authorization and schema/constraint validation through
  `RrdEngine`; ACK only after the destination's durable receipt, and checkpoint
  advancement only after validating that receipt;
- replay deduplication and origin tracking so a downloaded commit cannot echo
  back as a new local change;
- tombstone/delete propagation, schema incompatibility denial and explicit
  rebootstrap when the source checkpoint is no longer retained;
- optimistic base-version conflict detection; concurrent edits produce a
  visible conflict and an explicitly authorized resolution, not clock-based
  silent overwrites; and
- pause/revoke races, account switching, wrong destination, token expiry,
  tampering, quotas and slow/offline receivers with bounded retry/backpressure.

Commit identity must survive session replacement and transit through a third
authorized peer. Reconnecting with a new session cannot turn a replay into a
new write, and imported commits cannot circulate local -> cloud -> homelab ->
local indefinitely. Persist the origin/deduplication result atomically with the
destination mutation group. A client-side queue or a session-scoped transaction
idempotency key alone is insufficient. A schema/record/relation/vector group
is applied completely or rejected; transfer batching cannot split its semantics.

Initial synchronization must bind a snapshot to its changefeed cursor without
a missing or double-applied interval. Public export/import support must first
exist for every admitted family. Unsupported families, unresolved dangling
references, incompatible schema and unavailable content must fail explicitly.
Retained deletes and conflict state must survive reboot and disconnection.
Conflict resolution is itself an authorized, replay-safe operation with a
receipt, never an unrecorded UI choice or wall-clock overwrite policy.

The current native snapshot-isolation transaction model and changefeed do not
by themselves satisfy these requirements. This contract establishes no
rrflow.ai, rroflow.ai or shippin.cloud account endpoint, sync binding,
cross-instance reconciliation implementation or production credential.
Cloud code execution and private UI previews for Rust/Python/Go need their own
isolated devspace service;
they must not execute inside the database process.

## Service and homelab operation

The hosted service must bind authenticated accounts/projects to exact estates,
enforce tenant isolation and quotas, expose bounded health/readiness, and prove
backup/restore, upgrade compatibility, recovery, credential rotation, deletion
and observability against the shipped server artifact. Provisioning a container
or adding a reverse proxy does not supply these properties.

The homelab profile uses the same engine/data protocol and explicit trust
configuration, with operator-controlled storage and lifecycle. It must work
without contacting shippin.cloud. TLS, certificates, authentication, volume
ownership, resource budgets and recoverability are checked, not bypassed to
make first startup appear successful. Supported VM/container providers are
chosen from the actual DevPulse and Shippin implementations and qualified one
at a time; this contract does not invent an available provider.

## Delivery and acceptance

[D-01/D-06](../../roadmap/rrflow-1.0/gate-d.md) own installed local operation and
explicit adapter activation. [H-04/H-07](../../roadmap/rrflow-1.0/gate-h.md) own
public client/account endpoint integration. [POAM-011](../../poam/rrflow-1.0-alpha/poam-011.md)
tracks the absent cloud integration, and [Gate J](../../roadmap/rrflow-1.0/gate-j.md)
owns distributable platform evidence. No gate is closed by this design.

Qualify a packaged DevPulse build with no network/account; restart it and prove
local records/relations unchanged. Then use two installed engines and a real
account service to prove opt-in transfer, initial snapshot plus live changes,
idempotent reconnect after lost ACK, concurrent-edit conflict, deletes,
schema changes, source retention gap, foreign-account denial and revocation
during a transfer. Restart both ends at each durable handoff. Finally repeat
against a real shippin.cloud private devspace and a separately administered
homelab with hosted account access unavailable. A mock account, copied files,
green HTTP status, or local two-process demo is not cloud-sync acceptance.

The release qualification must additionally prove:

- a clean DevPulse installation provisions a supported local VM/container,
  builds and runs Rust, Python and Go project fixtures, and opens each private
  UI preview through its authorized project runtime;
- edits made independently on both peers converge or produce the specified
  visible conflict, including deletes, schema changes and binary assets;
- lost ACK, killed sender/receiver, full disk, expired/rotated credentials,
  retention loss and interrupted upgrade cause no acknowledged data loss or
  duplicate effect; each endpoint reopens and verifies independently;
- local, managed and homelab three-peer topologies suppress echo/replay and
  enforce every binding's data selection and revocation;
- source, environment and database states are reported separately, so a green
  database checkpoint cannot conceal an untransferred file or failed runtime;
- disconnect preserves local use; deleting remote resources requires a
  separate authorized operation and never deletes user-owned host resources.

H-07's package sequence owns implementation order. J-03/J-05 qualify artifacts
and deployment; J-04 owns comparative claims. No numeric performance target,
competitor parity or end-to-end completion is established by this document.
