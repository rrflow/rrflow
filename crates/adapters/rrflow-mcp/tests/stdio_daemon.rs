use rrd_contract::{CanonicalId, InstallationTargetKind};
use rrd_engine::RrdEngine;
use rrd_security::{Action, SecurityRepository};
use rrd_server::RrdHttpServer;
use rrd_store::RrflowKvStore;
use serde_json::{json, Value};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn distribution_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn install(project: &Path, executable: &Path) {
    std::fs::write(project.join("README.md"), "MCP daemon project\n").unwrap();
    let preview = RrdEngine::plan_installation(
        project,
        InstallationTargetKind::ExistingProject,
        "default",
        None,
        executable,
    )
    .unwrap();
    RrdEngine::apply_installation(
        project,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        executable,
    )
    .unwrap();
}

fn make_private(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
}

#[test]
fn daemon_mode_uses_one_authenticated_installed_authority() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let executable = distribution_executable();
    install(&project, &executable);
    let locator = RrdEngine::read_project_locator(&project).unwrap();
    let credential = RrdEngine::read_installed_api_key(&project).unwrap();
    let storage_root = project.join(&locator.storage_root);

    let engine = RrdEngine::open_installed(&project, &executable).unwrap();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let address = server.local_addr();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let server_thread = std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.serve_until(async move {
                let _ = receiver.await;
            }))
    });

    let api_key_file = temporary.path().join("mcp-api-key");
    std::fs::write(&api_key_file, credential.credential()).unwrap();
    make_private(&api_key_file);
    let mut child = Command::new(env!("CARGO_BIN_EXE_rrflow-mcp"))
        .arg("daemon")
        .arg("--url")
        .arg(format!("http://{address}"))
        .arg("--instance")
        .arg(locator.identity.instance_id.as_str())
        .arg("--principal")
        .arg(credential.principal_id.as_str())
        .arg("--api-key-file")
        .arg(&api_key_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    for request in [
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"rrflow_context","arguments":{"query":"","seeds":[{"kind":"rrflow-seat","id":"local-reasoning-seat"}],"valid_at":1800000000001_u64}}}),
    ] {
        serde_json::to_writer(child.stdin.as_mut().unwrap(), &request).unwrap();
        child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 2);
    let profile = &responses[0]["result"]["_meta"]["io.rrflow/runtimeProfile"];
    assert_eq!(profile["mode"], "daemon");
    assert_eq!(profile["execution_authority"], "rrd_server_via_rrd_client");
    assert_eq!(profile["storage_access"], "none");
    assert_eq!(
        profile["caller_authentication"],
        "principal_api_key_session"
    );
    assert_eq!(responses[0]["result"]["tools"].as_array().unwrap().len(), 1);
    assert_eq!(
        responses[1]["result"]["isError"], false,
        "unexpected daemon context response: {}",
        responses[1]
    );
    let packet: Value = serde_json::from_str(
        responses[1]["result"]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert!(packet["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["identity"] == "record:rrflow-seat:local-reasoning-seat"));

    shutdown.send(()).unwrap();
    server_thread.join().unwrap().unwrap();
    let storage = RrflowKvStore::open_existing(&storage_root).unwrap();
    let audit = SecurityRepository::new(&storage, locator.identity.instance_id)
        .audit_since(0, 256)
        .unwrap();
    assert!(audit.records.iter().any(|(_, record)| {
        record.principal_id.as_ref().map(CanonicalId::as_str) == Some("local-operator")
            && record.action == Action::MemoryContextRead
            && record.phase == rrd_contract::AuditPhase::Completed
            && record.decision == rrd_contract::AuditDecision::Allowed
    }));
}

#[test]
fn daemon_mode_refuses_an_explicit_unsecured_component_server() {
    let temporary = tempfile::tempdir().unwrap();
    let instance = CanonicalId::new("mcp-unsecured-component").unwrap();
    let engine = RrdEngine::open(
        &temporary.path().join("component-store"),
        instance.clone(),
        [7; 32],
    )
    .unwrap();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let address = server.local_addr();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let server_thread = std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.serve_until(async move {
                let _ = receiver.await;
            }))
    });

    let api_key_file = temporary.path().join("mcp-api-key");
    std::fs::write(&api_key_file, "not-an-authenticated-key").unwrap();
    make_private(&api_key_file);
    let output = Command::new(env!("CARGO_BIN_EXE_rrflow-mcp"))
        .arg("daemon")
        .arg("--url")
        .arg(format!("http://{address}"))
        .arg("--instance")
        .arg(instance.as_str())
        .arg("--principal")
        .arg("untrusted")
        .arg("--api-key-file")
        .arg(&api_key_file)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("daemon MCP requires an initialized RRD security policy"));

    shutdown.send(()).unwrap();
    server_thread.join().unwrap().unwrap();
}
