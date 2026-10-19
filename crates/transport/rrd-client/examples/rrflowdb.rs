//! Run against an installed local estate: rrflowdb PROJECT ADDRESS init|read|update|abort.
//! Bootstrap only reads the installed locator and credential; all data operations
//! use the public HTTP client. No cloud connection is made by this example.
#[path = "support/rrflowdb.rs"]
mod database;

use database::{options, read_options, Result};
use rrd_client::{ClientConfig, RrdClient};
use rrd_contract::{CloseSession, CreateSession, ReadDataSchema, SessionLimits};
use rrd_engine::RrdEngine;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 || !["init", "read", "update", "abort", "schema"].contains(&args[2].as_str())
    {
        return Err(
            "usage: rrflowdb PROJECT LOOPBACK_ADDRESS init|read|update|abort|schema".into(),
        );
    }
    let project = std::path::Path::new(&args[0]);
    let locator = RrdEngine::read_project_locator(project)?;
    let credential = RrdEngine::read_installed_api_key(project)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let at = u64::try_from(now.as_millis())?;
    let key = format!("db-{}-{}", std::process::id(), now.as_nanos());
    let client = RrdClient::connect_local(
        args[1].parse()?,
        locator.identity.instance_id,
        ClientConfig::default(),
    )?;
    client.capabilities().await?;
    let session = client
        .create_session(
            credential.principal_id.clone(),
            credential.credential(),
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 300_000,
                    max_open_transactions: 2,
                },
            },
            options(&format!("{key}-session"))?,
        )
        .await?;
    let result: Result<serde_json::Value> = async {
        match args[2].as_str() {
            "init" => Ok(serde_json::to_value(database::initialize(&client, &session, at, &key).await?)?),
            "schema" => Ok(serde_json::to_value(client.read_data_schema(&session, ReadDataSchema {
                scope: format!("instance:{}", client.instance_id()), max_storage_keys: 4096,
            }, read_options(&format!("{key}-schema"))?).await?)?),
            "update" | "abort" => {
                let transaction = database::begin(&client, &session, &format!("{key}-begin")).await?;
                if args[2] == "abort" {
                    database::abort(&client, &session, &transaction, &format!("{key}-abort")).await?;
                    Ok(serde_json::json!({"state": "aborted", "data_changed": false}))
                } else {
                    Ok(serde_json::to_value(database::commit(&client, &session, &transaction,
                        vec![database::task_record("Build with rrflowDB", "done", at)], &format!("{key}-commit")).await?)?)
                }
            }
            _ => Ok(serde_json::json!({
                "relationships": database::relationships(&client, &session, at, &format!("{key}-join")).await?,
                "tasks": database::tasks(&client, &session, at, &format!("{key}-tasks")).await?,
                "cloud_sync": "disabled"
            })),
        }
    }.await;
    let closed = client
        .close_session(&session, CloseSession {}, options(&format!("{key}-close"))?)
        .await;
    let result = result?;
    closed?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
