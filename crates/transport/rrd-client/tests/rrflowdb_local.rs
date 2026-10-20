//! Real installed rrflowKV + HTTP proof of the runnable local database example.
#[path = "../examples/support/rrflowdb.rs"]
mod database;

use database::{options, read_options};
use rrd_client::{ClientConfig, RrdClient, Session};
use rrd_contract::{
    CloseSession, CreateSession, DataSchemaSnapshot, InstallationTargetKind, QueryValue,
    ReadDataSchema, SessionLimits, TransactionMutation,
};
use rrd_engine::RrdEngine;
use rrd_server::RrdHttpServer;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn executable() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

async fn start(
    project: &Path,
) -> (
    RrdClient,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<std::result::Result<(), rrd_server::HttpError>>,
) {
    let engine = RrdEngine::open_installed(project, &executable()).unwrap();
    let instance = engine.instance_id().clone();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let address = server.local_addr();
    let (shutdown, stopped) = oneshot::channel();
    let task = tokio::spawn(server.serve_until(async {
        let _ = stopped.await;
    }));
    (
        RrdClient::connect_local(address, instance, ClientConfig::default()).unwrap(),
        shutdown,
        task,
    )
}

async fn login(client: &RrdClient, project: &Path, key: &str) -> Session {
    let credential = RrdEngine::read_installed_api_key(project).unwrap();
    client
        .create_session(
            credential.principal_id.clone(),
            credential.credential(),
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 300_000,
                    max_open_transactions: 4,
                },
            },
            options(key).unwrap(),
        )
        .await
        .unwrap()
}

async fn schema(client: &RrdClient, session: &Session, key: &str) -> DataSchemaSnapshot {
    client
        .read_data_schema(
            session,
            ReadDataSchema {
                scope: format!("instance:{}", client.instance_id()),
                max_storage_keys: 4096,
            },
            read_options(key).unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn installed_native_database_relations_atomicity_temporal_reads_and_reopen() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::FreshProject,
        "default",
        None,
        &executable(),
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &preview,
        &preview.installation.plan_sha256,
        now(),
        &executable(),
    )
    .unwrap();
    let (client, shutdown, server) = start(&project).await;
    client.capabilities().await.unwrap();
    let session = login(&client, &project, "db-session").await;
    let initial_schema = schema(&client, &session, "db-before").await.schema.unwrap();
    let created_at = now();
    let created = database::initialize(&client, &session, created_at, "db-init")
        .await
        .unwrap();
    assert_eq!(created.mutation_count, 4);
    let installed_schema = schema(&client, &session, "db-after").await;
    for (kind, table) in &initial_schema.tables {
        assert_eq!(
            installed_schema.schema.as_ref().unwrap().tables.get(kind),
            Some(table)
        );
    }
    assert_eq!(
        installed_schema.schema.as_ref().unwrap().revision,
        initial_schema.revision + 1
    );
    installed_schema.read_evidence.validate().unwrap();
    let joined = database::relationships(&client, &session, created_at, "db-join")
        .await
        .unwrap();
    assert_eq!(joined.rows.len(), 1);
    assert_eq!(
        joined.rows[0].values["right.to_id"],
        QueryValue::String("first-task".into())
    );
    let before = database::tasks(&client, &session, created_at, "db-tasks-before")
        .await
        .unwrap();
    assert_eq!(
        before.rows[0].values["status"],
        QueryValue::String("open".into())
    );

    // A second init must not overwrite schema or application data.
    assert!(
        database::initialize(&client, &session, created_at, "db-init-again")
            .await
            .is_err()
    );
    assert_eq!(
        schema(&client, &session, "db-after-second-init").await,
        installed_schema
    );

    // Unique violation after a valid task update must roll back both mutations.
    let transaction = database::begin(&client, &session, "db-invalid-begin")
        .await
        .unwrap();
    let rejected_at = created_at + 1;
    assert!(database::commit(
        &client,
        &session,
        &transaction,
        vec![
            database::task_record("must not survive", "done", rejected_at),
            database::project_record("duplicate-project", "local-ai", rejected_at),
        ],
        "db-invalid-commit"
    )
    .await
    .is_err());
    database::abort(&client, &session, &transaction, "db-invalid-abort")
        .await
        .unwrap();
    let unchanged = database::tasks(&client, &session, rejected_at, "db-after-invalid")
        .await
        .unwrap();
    assert_eq!(unchanged.rows, before.rows);
    assert_eq!(unchanged.known_at_cursor, before.known_at_cursor);

    // Required typed fields cannot silently accept another type.
    let mut invalid = database::task_record("bad type", "done", rejected_at);
    if let TransactionMutation::PutRecord { properties, .. } = &mut invalid {
        properties.insert("status".into(), QueryValue::Unsigned(1));
    }
    let transaction = database::begin(&client, &session, "db-type-begin")
        .await
        .unwrap();
    assert!(database::commit(
        &client,
        &session,
        &transaction,
        vec![invalid],
        "db-type-commit"
    )
    .await
    .is_err());
    database::abort(&client, &session, &transaction, "db-type-abort")
        .await
        .unwrap();

    // A canceled transaction cannot subsequently commit its proposed write.
    let transaction = database::begin(&client, &session, "db-cancel-begin")
        .await
        .unwrap();
    database::abort(&client, &session, &transaction, "db-cancel-abort")
        .await
        .unwrap();
    assert!(database::commit(
        &client,
        &session,
        &transaction,
        vec![database::task_record("canceled", "done", rejected_at)],
        "db-cancel-commit"
    )
    .await
    .is_err());

    let updated_at = created_at + 2;
    let stale = database::begin(&client, &session, "db-stale-begin")
        .await
        .unwrap();
    let transaction = database::begin(&client, &session, "db-update-begin")
        .await
        .unwrap();
    let mutations = vec![database::task_record(
        "Build with rrflowDB",
        "done",
        updated_at,
    )];
    let committed = database::commit(
        &client,
        &session,
        &transaction,
        mutations.clone(),
        "db-update-commit",
    )
    .await
    .unwrap();
    let replay = database::commit(
        &client,
        &session,
        &transaction,
        mutations,
        "db-update-commit",
    )
    .await
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(
        replay.runtime_commit_sha256,
        committed.runtime_commit_sha256
    );
    assert!(database::commit(
        &client,
        &session,
        &stale,
        vec![database::task_record("stale write", "lost", updated_at)],
        "db-stale-commit"
    )
    .await
    .is_err());
    database::abort(&client, &session, &stale, "db-stale-abort")
        .await
        .unwrap();
    let updated = database::tasks(&client, &session, updated_at, "db-updated")
        .await
        .unwrap();
    assert_eq!(
        updated.rows[0].values["status"],
        QueryValue::String("done".into())
    );
    assert_eq!(
        database::tasks(&client, &session, created_at, "db-history")
            .await
            .unwrap()
            .rows,
        before.rows
    );

    // Scope and resource budgets are enforced by the real authenticated server.
    for (scope, max_storage_keys) in [
        ("instance:foreign".into(), 4096),
        (format!("instance:{}", client.instance_id()), 1),
    ] {
        assert!(client
            .read_data_schema(
                &session,
                ReadDataSchema {
                    scope,
                    max_storage_keys
                },
                read_options("db-denied").unwrap()
            )
            .await
            .is_err());
    }
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
    drop(client);

    let (reopened, shutdown, server) = start(&project).await;
    let session = login(&reopened, &project, "db-reopen-session").await;
    let durable = database::tasks(&reopened, &session, updated_at, "db-reopened")
        .await
        .unwrap();
    assert_eq!(durable.rows, updated.rows);
    assert_eq!(durable.known_at_cursor, updated.known_at_cursor);
    assert_eq!(durable.schema_revision, updated.schema_revision);
    assert_eq!(
        database::relationships(&reopened, &session, updated_at, "db-reopened-join")
            .await
            .unwrap()
            .rows,
        joined.rows
    );
    assert_eq!(
        schema(&reopened, &session, "db-reopened-schema")
            .await
            .schema,
        installed_schema.schema
    );
    reopened
        .close_session(&session, CloseSession {}, options("db-close").unwrap())
        .await
        .unwrap();
    assert!(reopened
        .read_data_schema(
            &session,
            ReadDataSchema {
                scope: format!("instance:{}", reopened.instance_id()),
                max_storage_keys: 4096,
            },
            read_options("db-closed-session").unwrap()
        )
        .await
        .is_err());
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
}
