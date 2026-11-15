# Build against local rrflowDB

**Status:** active native database integration guide; pre-alpha, not PostgreSQL qualification
**Coordinate:** `rrflow://rrflow-instance/data/reference/sdk/rrflowdb-local`
**Owner:** runnable installed native database example and its integration instructions

rrflowDB is usable through the native Rust engine and authenticated RRD client.
This guide builds a real persistent local database, creates typed relational
records, a typed nested document, and graph relationships in one transaction,
queries them with rrflowQL at a common read stamp, and reads them after restart.
The example uses the local authenticated service; it does not demonstrate a
stable daemon-free embedded library API. It uses no Turso, libSQL, SQLite, PostgreSQL server,
cloud account, or external model. The [PostgreSQL profile](../protocol/postgresql.md)
remains a separate unimplemented compatibility requirement.

The example's document has an explicit schema. The query engine also supports
bounded projection from registered schemaless record-like tables; dynamic
property predicates and arbitrary join keys remain separate open work in
[POAM-005](../../poam/rrflow-1.0-alpha/poam-005.md). This example does not
establish general schemaless queries or production ACID qualification.

## How the components relate

```mermaid
flowchart TB
    UI[DevPulse or Connectome application] --> SDK[RRD public client]
    SDK --> API[Authenticated local RRD service]
    subgraph DB[rrflowDB: logical database within RRFlow]
        API --> Engine[RrdEngine: authorization, transactions, read stamps]
        Engine --> QL[rrflowQL: parse, bind, plan, execute]
        QL --> Reads[rrd-store: stamped semantic reads]
        Engine --> Writes[rrd-store: schema and constraint validation]
        Writes --> Commit[One atomic semantic commit and receipt]
        Reads --> KV
        Commit --> KV
        subgraph KV[rrflowKV: native physical persistence]
            WAL[Write-ahead log] --> MVCC[MVCC memtable]
            MVCC --> Segments[Immutable segments and manifest]
        end
    end
```

rrflowQL is a language/execution component; rrflowDB is the logical database;
rrflowKV stores its durable bytes. They are not three databases to synchronize.
`RrdEngine` owns authorization and commits. DataFusion is an analytical executor
under that authority. The [engine data-flow owner](../../architecture/engine-data-flow.md)
defines the complete boundary and its remaining qualification gaps.

## Build and install

Use the repository's pinned Rust toolchain. From the repository root in PowerShell 7:

```powershell
cargo build --locked -p rrflow-cli --bin rrflow -p rrd-client --example rrflowdb
if ($LASTEXITCODE -ne 0) { throw 'RRFlow and example build failed' }
$binaryDir = Join-Path $env:TEMP ('rrflow-bin-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $binaryDir | Out-Null
$rrflow = Join-Path $binaryDir 'rrflow.exe'
Copy-Item -LiteralPath .\target\debug\rrflow.exe -Destination $rrflow
if ((Get-FileHash -Algorithm SHA256 .\target\debug\rrflow.exe).Hash -ne (Get-FileHash -Algorithm SHA256 $rrflow).Hash) { throw 'Staged binary differs from build' }
$rrflow
$project = Join-Path $env:TEMP ('rrflowdb-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $project | Out-Null
$project
$planFile = "$project-install-plan.json"
$identity = & $rrflow install identity --json | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Installation identity generation failed' }
$plan = & $rrflow install plan --project $project --identity $identity.installation_identity --mode fresh --json
if ($LASTEXITCODE -ne 0) { throw 'Installation planning failed' }
$plan | Set-Content -LiteralPath $planFile -Encoding utf8NoBOM
$preview = Get-Content -Raw $planFile | ConvertFrom-Json
$preview.installation
# Review the exact managed paths, configuration and permissions above.
& $rrflow install apply --project $project --mode fresh --plan $planFile --expect $preview.installation.plan_sha256
if ($LASTEXITCODE -ne 0) { throw 'Installation apply failed' }
& $rrflow serve --project $project --bind 127.0.0.1:9477
```

`serve` stays in the foreground. In a second terminal, set `$project` and
`$rrflow` to the same printed paths, then run:

```powershell
& $rrflow ready --project $project --address 127.0.0.1:9477
.\target\debug\examples\rrflowdb.exe $project 127.0.0.1:9477 init
.\target\debug\examples\rrflowdb.exe $project 127.0.0.1:9477 verify open
.\target\debug\examples\rrflowdb.exe $project 127.0.0.1:9477 read
.\target\debug\examples\rrflowdb.exe $project 127.0.0.1:9477 update
.\target\debug\examples\rrflowdb.exe $project 127.0.0.1:9477 abort
.\target\debug\examples\rrflowdb.exe $project 127.0.0.1:9477 verify done
.\target\debug\examples\rrflowdb.exe $project 127.0.0.1:9477 schema
```

Cargo may hard-link its build output to another target path. The installed
engine rejects multiply linked executables, so stage a separate copy and check
its digest as above; do not weaken the link check. On Linux/macOS likewise copy
`target/debug/rrflow` to a new private directory, preserve its executable mode,
and independently compare the original and copy with `sha256sum` on Linux or
`shasum -a 256` on macOS. Use the copied binary for each lifecycle command and
`target/debug/examples/rrflowdb` for the example. A custom `CARGO_TARGET_DIR`
changes the build output location. These are source-built development artifacts,
not a signed official release. Installed engrams bind the executable digest:
retain the binary used to install them; replacing it is not an automatic upgrade.

Stop the service, run `rrflow verify --project <path> --level quick`, restart
`serve`, and run the example's `verify done`. A nonzero exit is a failed
readback, even if a previous step printed a success. The task remains `done`. The
database lives under the engine-managed `.rrflow/rrd/` root selected by
`.rrflow/config.toml`. Do not open that root with a second engine process.
The example reads the installed locator and API credential locally without
printing the credential, then performs all data work through the public client.

## Implemented example model

```mermaid
erDiagram
    DEVPULSE_PROJECT ||--o{ DEVPULSE_OWNS : from
    DEVPULSE_TASK ||--o| DEVPULSE_OWNS : to
    DEVPULSE_DOCUMENT ||--o{ DEVPULSE_SUPPORTS : from
    DEVPULSE_TASK ||--o{ DEVPULSE_SUPPORTS : to
    DEVPULSE_PROJECT {
        string id PK
        string slug UK
        string name
    }
    DEVPULSE_TASK {
        string id PK
        string title
        string status
    }
    DEVPULSE_OWNS {
        string id PK
        string from_id FK
        string to_id FK
    }
    DEVPULSE_DOCUMENT {
        string id PK
        string title
        map metadata
    }
    DEVPULSE_SUPPORTS {
        string id PK
        string from_id FK
        string to_id FK
        string reason
    }
```

The native record identity is `(kind, id)`. Each owns relationship references
existing project/task identities; its endpoint kinds, unique pair, and maximum
one incoming relationship per task are schema constraints. Unlinked tasks are
permitted. `slug` is required and unique; `name`, `title`, and `status` are
required strings. The document's title and metadata map are required; its
supports relationship names a document and a task, requires a reason, and
enforces a unique endpoint pair. Values also have valid-time intervals and
commit coordinates.
The diagram describes this application's schema, not additional platform
project/engram/account authorities.

The parameterized native join is:

```text
FROM record:devpulse_project
JOIN relation:devpulse_owns ON id = from_id
AT VALID $at KNOWN HEAD
WHERE left.slug = $project
PROJECT left.name, right.to_id
```

`$at` is an unsigned millisecond timestamp; `$project` is the string `local-ai`.
Parameters remain typed values. This is rrflowQL syntax, not PostgreSQL SQL.

## Application integration

The runnable source is [rrflowdb.rs](../../../crates/transport/rrd-client/examples/rrflowdb.rs);
its [application operations](../../../crates/transport/rrd-client/examples/support/rrflowdb.rs)
use `RrdClient` and public contract types. Adapt these operations inside
DevPulse's backend, using its installed process adapter and secure credential
handling. The example is integration source, not a second supported database
binary or a replacement for `rrflow` installation/lifecycle.

`read_data_schema` resolves `data-schema-read` from the operation catalogue.
It requires an unrestricted instance-level `diagnostics-read` grant, returns
one authenticated native schema/read stamp and bounded I/O evidence, and does
not read all application records. An application planning DDL begins a data
transaction, reads the schema, and requires the returned cursor to match the
lease before extending the catalogue. A stale revision/transaction is rejected;
it must be replanned, never silently merged. Ordinary data access uses separately
authorized queries and transactions.

`init` adds tables to the current catalogue and atomically creates the sample
records/relationship. It refuses a second initialization. `update` commits a
new temporal task version. `abort` opens and cancels an empty transaction.
`read` captures the task query's committed cursor and uses that cursor for
the project/relationship join, document query, and document/relationship join
at the same valid time. It rejects truncated
results, row-count inconsistencies, foreign scope or differing cursor/schema
stamps. `verify open|done` additionally compares the example task, document,
and relationships against fixed expected values.
Successful output retains all four public query results and their read evidence,
plus `verified_expected_status`; no credential is included. Running
`verify open` after `update` must fail. The verifier is specific to this
sample's one-task model, not a general health check for arbitrary user tables.

Commit requests bind a payload digest, idempotency key and lease-bounded
deadline. An unknown transport or storage outcome must be reconciled with its
original identity; do not blindly retry a write with a new key.

The [real-server regression](../../../crates/transport/rrd-client/tests/rrflowdb_local.rs)
tests schema preservation, parameterized joins, typed/unique rejection with
no partial write, explicit abort, receipt replay, historical reads, scope and
budget denial, closed-session denial, and disk reopen. It also checks an
intervening commit between reads, rejects mixed snapshots and incomplete or
unexpected sample data, and verifies initial/updated/aborted/reopened states.
Run it with:

```text
cargo test --locked -p rrd-client --test rrflowdb_local
```

DevPulse's optional [shippin.cloud account and synchronization boundary](../client/devpulse-cloud.md)
is explicit. Running this local example does not create an account, enable
synchronization, publish a project, or upload data.
