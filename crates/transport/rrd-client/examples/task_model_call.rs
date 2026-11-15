//! One explicitly selected installed-engram model operation through the Rust SDK.
//! Locator/credential bootstrap follows rrflowdb; all operations use public HTTP.
use rrd_client::{
    operations::{TaskEndpointPropose, TaskWorkerExecute},
    ClientConfig, OperationMetadata, RequestOptions, RrdClient,
};
use rrd_contract::{
    CloseSession, CreateSession, EndpointSelectionRequest, SessionLimits, TaskWorkerRequest,
};
use rrd_engine::RrdEngine;
use std::{
    io::Read,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

enum ModelRequest {
    Proposal(Box<EndpointSelectionRequest>),
    Worker(Box<TaskWorkerRequest>),
}

impl ModelRequest {
    fn parse(operation: &str, bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        match operation {
            TaskEndpointPropose::OPERATION => {
                let request: EndpointSelectionRequest = serde_json::from_slice(bytes)?;
                request.validate()?;
                Ok(Self::Proposal(Box::new(request)))
            }
            TaskWorkerExecute::OPERATION => {
                let request: TaskWorkerRequest = serde_json::from_slice(bytes)?;
                request.validate()?;
                Ok(Self::Worker(Box::new(request)))
            }
            _ => Err("expected task-endpoint-propose or task-worker-execute".into()),
        }
    }

    fn observed_request(&self) -> Result<serde_json::Value, serde_json::Error> {
        match self {
            Self::Proposal(request) => serde_json::to_value(request),
            Self::Worker(request) => serde_json::to_value(request),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        return Err(
            "usage: task_model_call OPERATION PROJECT LOOPBACK_ADDRESS REQUEST_JSON TIMEOUT_MS"
                .into(),
        );
    }
    let timeout_ms: u64 = args[4].parse()?;
    let limits = SessionLimits {
        idle_timeout_ms: timeout_ms,
        absolute_timeout_ms: timeout_ms.checked_mul(2).ok_or("timeout overflow")?,
        max_open_transactions: 1,
    };
    limits.validate()?;
    let mut bytes = Vec::new();
    std::fs::File::open(&args[3])?
        .take(rrd_contract::MAX_ROUTE_REQUEST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len())? > rrd_contract::MAX_ROUTE_REQUEST_BYTES {
        return Err("model operation request file exceeds the contract byte limit".into());
    }
    let request = ModelRequest::parse(&args[0], &bytes)?;
    let observed_request = request.observed_request()?;
    let project = Path::new(&args[1]);
    let locator = RrdEngine::read_project_locator(project)?;
    let credential = RrdEngine::read_installed_api_key(project)?;
    let client = RrdClient::connect_local(
        args[2].parse()?,
        locator.identity.instance_id,
        ClientConfig {
            request_timeout: Duration::from_millis(timeout_ms),
            max_attempts: 1,
            ..ClientConfig::default()
        },
    )?;
    client.capabilities().await?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let key = format!("model-native-{}-{}", std::process::id(), now.as_nanos());
    let session = client
        .create_session(
            credential.principal_id.clone(),
            credential.credential(),
            CreateSession { limits },
            RequestOptions::mutation(&key, &key, &key)?,
        )
        .await?;
    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let options = RequestOptions::new(
        &key,
        &key,
        Some(&key),
        Some(at.checked_add(timeout_ms).ok_or("deadline overflow")?),
    )?;
    let result = match request {
        ModelRequest::Proposal(request) => client
            .call_operation::<TaskEndpointPropose>(&session, *request, options)
            .await
            .map(|proposal| serde_json::json!({"status": "ok", "proposal": proposal})),
        ModelRequest::Worker(request) => client
            .execute_task_worker(&session, *request, options)
            .await
            .map(|worker| serde_json::json!({"status": "ok", "worker": worker})),
    };
    let close_key = format!("{key}-close");
    let closed = client
        .close_session(
            &session,
            CloseSession {},
            RequestOptions::mutation(&close_key, &close_key, &close_key)?,
        )
        .await;
    let close_error = closed.err().map(|error| error.to_string());
    let failed = result.is_err() || close_error.is_some();
    let outcome = match result {
        Ok(result) => result,
        Err(error) => serde_json::json!({"status": "error", "error": error.to_string()}),
    };
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "operation": args[0], "request": observed_request,
            "outcome": outcome, "sessionCloseError": close_error,
        }))?
    );
    if failed {
        std::process::exit(1);
    }
    Ok(())
}
