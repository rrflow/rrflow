use rrd_contract::InstallationTargetKind;
use rrd_engine::RrdEngine;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn distribution_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn install(project: &Path, executable: &Path) {
    std::fs::write(project.join("README.md"), "MCP installed project\n").unwrap();
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

fn embedded(project: &Path, executable: &Path) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_rrflow-mcp"))
        .arg("embedded")
        .arg("--project")
        .arg(project)
        .arg("--distribution-executable")
        .arg(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn invoke_embedded(project: &Path, executable: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rrflow-mcp"))
        .arg("embedded")
        .arg("--project")
        .arg(project)
        .arg("--distribution-executable")
        .arg(executable)
        .output()
        .unwrap()
}

#[test]
fn stdio_context_flows_through_the_installed_authenticated_engine() {
    let project = tempfile::tempdir().unwrap();
    let executable = distribution_executable();
    install(project.path(), &executable);

    let mut child = embedded(project.path(), &executable);
    for request in [
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"rrflow_context","arguments":{"query":"","seeds":[{"kind":"rrflow-seat","id":"local-reasoning-seat"}],"valid_at":1800000000001_u64,"max_items":8,"max_output_bytes":16384,"max_storage_keys":100}}}),
        serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"obsolete_tool","arguments":{}}}),
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
    let responses: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 4);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-11-25");
    let profile = &responses[0]["result"]["_meta"]["io.rrflow/runtimeProfile"];
    assert_eq!(profile["execution_authority"], "rrd_engine");
    assert_eq!(profile["storage_access"], "installed_engine_only");
    assert_eq!(
        profile["caller_authentication"],
        "installed_operator_api_key_session"
    );
    let tools = responses[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "rrflow_context");
    assert_eq!(responses[2]["result"]["isError"], false);
    let packet_text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let packet: rrd_contract::ContextPacket = serde_json::from_str(packet_text).unwrap();
    packet.validate().unwrap();
    let packet_value: serde_json::Value = serde_json::from_str(packet_text).unwrap();
    assert_eq!(packet.read.runtime_cursor, 2);
    assert!(packet_value["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["identity"] == "record:rrflow-seat:local-reasoning-seat"
                && item["evidence"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|evidence| evidence["kind"] == "seed")
        }));
    assert_eq!(responses[3]["error"]["code"], -32602);
}

#[test]
fn stdio_server_supports_the_stateless_2026_discovery_era() {
    let project = tempfile::tempdir().unwrap();
    let executable = distribution_executable();
    install(project.path(), &executable);
    let input = [
        serde_json::json!({"jsonrpc":"2.0","id":"d","method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1"}}}}),
        serde_json::json!({"jsonrpc":"2.0","id":"l","method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"test","version":"1"}}}}),
    ];
    let mut child = embedded(project.path(), &executable);
    for message in input {
        serde_json::to_writer(child.stdin.as_mut().unwrap(), &message).unwrap();
        child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses[0]["result"]["supportedVersions"][0], "2026-07-28");
    assert_eq!(
        responses[1]["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "rrflow-mcp"
    );
    assert_eq!(responses[1]["result"]["tools"].as_array().unwrap().len(), 1);
}

#[test]
fn stdio_server_rejects_absent_or_distribution_mismatched_installed_state() {
    let absent = tempfile::tempdir().unwrap();
    let executable = distribution_executable();
    let absent_output = invoke_embedded(absent.path(), &executable);
    assert!(!absent_output.status.success());
    assert!(absent_output.stdout.is_empty());

    let installed = tempfile::tempdir().unwrap();
    install(installed.path(), &executable);
    let wrong_distribution = absent.path().join("wrong-rrflow");
    std::fs::write(&wrong_distribution, b"not the sealed distribution").unwrap();
    let mismatch = invoke_embedded(installed.path(), &wrong_distribution);
    assert!(!mismatch.status.success());
    assert!(mismatch.stdout.is_empty());
    assert!(String::from_utf8_lossy(&mismatch.stderr).contains("ProjectBindingMismatch"));
}
