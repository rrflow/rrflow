# H-06 existing Connectome profile recovery on D:

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h06-existing-profile-recovery`
**Owner:** bounded existing-profile recovery evidence for OBJ-07 / H-06

This record corrects the scope of the prior optional-client journal's local
installation smoke. That smoke used a separate estate and did not repair the
owner's saved Local RRFlow connection. It cannot establish existing-profile
recovery or cloud/bidirectional-sync readiness.

## Change brief and baseline

Engine baseline 470594178d25f1bf12d3ec6d9041488817a9e931; authored Connectome
baseline abc549cf7bb61f2ed4a20353ccb2e3621da515c9. Preserve the authored UI,
profile I9ZKPaj55, its saved query, original credential and original remote
estate. Fix the existing Edit/reconnect/serving code; refuse implicit blank
replacement installation. Work resumed under D:\localdev\rrflow after the
owner copied the repository; C: source is preserved during the remaining copy.

Reviewed existing client/native command and transport, RuntimeConnection and
session panel, Connection/defaults/config state, Edit connection and address
probe, serving action/adapter/native estate lifecycle, repository/client
contracts, and prior tests/diffs. Exact changed child source and test map is
recorded in connectome/docs/rrflow-native-rewrite.md. No engine runtime change
was necessary for this recovery. Existing server authority, protocol, storage,
version and public SDK pin are preserved. No source import was adapted.

## Real failure and repair

The saved origin was http://127.0.0.1:9477; its C: project directory contained
only an operator credential. The actual estate was already running on
`devstation`, project /workspace/warden-storage/scratch/rrflow-dev/connectome-demo,
as instance-07d784c6047ec8be023891a2. A remote fixed-hash presence check against
that installed configuration exited 0 with no output, matching the saved
credential's public plan digest. No credential/configuration payload was
printed. The initial broader remote configuration read was rejected by
automatic approval review; the digest-only check replaced it.

The existing profile's new optional SSH-host setting crosses the existing
native public bootstrap command. Native code owns an SSH process with direct
arguments, no shell, bounded readiness and stderr, strict known-host checking,
exact saved port/origin and host reuse, occupied-port rejection, and cleanup
on disconnect/application exit. It never starts or modifies the remote engine.
The existing profile and its original query are retained. App preferences were
backed up before adding devstation and the verified original instance.

Existing local profile starts now require an existing installed locator and
exact saved endpoint; missing project input opens that profile's existing Edit
form. SSH profiles expose Reconnect, never local installation/stop. Existing
Edit and public address checks use RRD bootstrap rather than an unrelated RPC
transport. Errors retain their actual native text.

## Acceptance evidence on 2026-10-18

- D: frozen-lockfile dependency installation passed after the owner's copy left
  empty node package placeholders.
- `pnpm check`: passed, full TypeScript and Biome on 597 files; ownership and
  dependency gates passed. The final serving-button edit was separately
  formatted and typechecked successfully.
- `pnpm test:rrflow`: 19 passed, zero failed/skipped.
- Focused managed-SSH native cases: 5 passed. Full native suite: 38 passed,
  zero failed, one explicit installed-process integration test ignored.
- D: `pnpm tauri:dev` built and launched the actual authored desktop app.
- Native UI at /runtime/I9ZKPaj55 authenticated as local-operator against the
  original instance-07d784c6047ec8be023891a2 at saved port 9477. Records rendered
  271 records / 986 relations at cursor 1529, and Vectors reported one existing
  collection / named vector. These were original data, not seeded test data.
- Clicking the actual profile Reconnect action returned to the same identity,
  session and data. HTTP transport, engine readiness and RRD protocol/identity
  cards each displayed VERIFIED.

No remote engine was restarted, installed, stopped, reconfigured or seeded.
The earlier manually owned temporary SSH tunnel was stopped before this
managed-forward qualification. No turnkey cloud, external-provider execution,
full H-06 conformance, candidate CI, merge or release gate is claimed.

## Follow-through

The product requirement is turnkey shippin.cloud with optional custom instances
and bidirectional synchronization. Read-only audit found native vectors and a
real external PostgreSQL/pgvector adapter, but no persisted knowledge-path
registry, HydraDB/MongoDB Atlas/Pinecone Nexus adapter, or completed cloud sync.
Those are separate engine/public-contract implementation work, not a reason
to substitute a new empty estate. The later knowledge-tree change has its own
bounded brief and evidence. Current roadmap and POA&M completion status are
unchanged by this recovery receipt. Navigation/inventory regeneration follows
stable source; PR/candidate CI remain landing requirements.
