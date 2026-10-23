//! Real-process proof for the installed, UI-discoverable walking product.
//!
//! This is one bounded child lifecycle, not a generic process-kill matrix: the
//! exact primary executable publishes its address, serves generated discovery,
//! accepts its generated credential, survives restart, and passes the read-only
//! verifier. The initial process is terminated after acknowledged writes; subsequent
//! Unix shutdowns use graceful SIGINT. Only test-owned children are affected.

use rrd_client::{ClientConfig, RequestOptions, RrdClient};
use rrd_contract::{
    transaction_operation_sha256, BackupCoverageSnapshot, BeginTransaction, CanonicalId,
    CloseSession, CommitTransaction, CreateInstanceBackup, CreateSession, ErrorCode, ExecuteQuery,
    QueryBudget, QueryResult, QueryValue, RestoreInstanceBackup, SessionLimits,
    TransactionMutation,
};
use rrd_engine::RrdEngine;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

#[path = "support/process.rs"]
mod process;
#[path = "support/storage_history.rs"]
mod storage_history;
// Authenticated readiness includes the generated discovery catalogue (>4 MiB).
const OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
#[cfg(unix)]
use std::time::Instant;

struct StandaloneDistribution {
    root: tempfile::TempDir,
    executable: PathBuf,
}

impl StandaloneDistribution {
    fn stage() -> Self {
        let source = std::env::var_os("RRFLOW_TEST_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_rrflow")));
        let root = tempfile::Builder::new()
            .prefix("rrflow standalone ")
            .tempdir()
            .unwrap();
        let executable = root.path().join(if cfg!(windows) {
            "rrflow.exe"
        } else {
            "rrflow"
        });
        process::stage_executable(&source, &executable)
            .expect("stage the actual primary executable");
        for directory in ["working", "home", "temporary", "empty-path"] {
            std::fs::create_dir(root.path().join(directory)).unwrap();
        }
        Self { root, executable }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        command
            .env_clear()
            .current_dir(self.root.path().join("working"));
        // Native OS discovery is the only inherited environment. No Cargo,
        // provider, checkout, credential, proxy or developer PATH survives.
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        for name in ["HOME", "USERPROFILE"] {
            command.env(name, self.root.path().join("home"));
        }
        for name in ["TMP", "TEMP", "TMPDIR"] {
            command.env(name, self.root.path().join("temporary"));
        }
        command.env("PATH", self.root.path().join("empty-path"));
        command.env("LANG", "C");
        command
    }
}

thread_local! {
    // The fixture outlives this test's child handles and is removed when its
    // test thread exits. No global working-directory/environment mutation.
    static DISTRIBUTION: StandaloneDistribution = StandaloneDistribution::stage();
}

fn binary() -> PathBuf {
    DISTRIBUTION.with(|distribution| distribution.executable.clone())
}

fn product_command() -> Command {
    DISTRIBUTION.with(StandaloneDistribution::command)
}

async fn verify_parquet_export(
    client: &RrdClient,
    session: &rrd_client::Session,
    expected: &QueryResult,
) {
    let request = rrd_contract::ExportQuery {
        query: ExecuteQuery {
            scope: format!("instance:{}", client.instance_id()),
            query: expected.canonical_query.clone(),
            parameters: BTreeMap::new(),
            budget: QueryBudget {
                max_storage_keys: 75_000,
                max_rows: 768,
                max_output_bytes: 393_216,
                max_batch_rows: 192,
                max_memory_bytes: 33_554_432,
                max_spill_bytes: 67_108_864,
                max_elapsed_ms: 20_000,
            },
        },
        max_output_bytes: 393_216,
    };
    let export = client
        .export_query(
            session,
            request.clone(),
            RequestOptions::read("parquet-read", "parquet-export").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export.source.canonical_query, expected.canonical_query);
    assert_eq!(
        export.source.read_manifest_sha256,
        expected.read_manifest_sha256
    );
    assert_eq!(export.source.known_at_cursor, expected.known_at_cursor);
    assert_eq!(export.source.schema_revision, expected.schema_revision);
    assert_eq!(
        export.source.plan.authorization_sha256,
        expected.plan.authorization_sha256
    );
    let reader = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(
        bytes::Bytes::from(export.decode_artifact().unwrap()),
    )
    .unwrap();
    let metadata = reader
        .metadata()
        .file_metadata()
        .key_value_metadata()
        .unwrap();
    let source = metadata
        .iter()
        .find(|entry| entry.key == "rrflow.export.source")
        .unwrap();
    assert_eq!(
        serde_json::from_str::<rrd_contract::QueryExportSource>(source.value.as_ref().unwrap())
            .unwrap(),
        export.source
    );
    let rows: Vec<_> = reader
        .build()
        .unwrap()
        .flat_map(|batch| rrd_query::record_batch_to_rows(&batch.unwrap()).unwrap())
        .collect();
    assert_eq!(rows.len(), expected.rows.len());
    for (row, expected) in rows.iter().zip(&expected.rows) {
        assert_eq!(row.identity, expected.identity);
        assert_eq!(
            row.values.get("object").unwrap(),
            &rrd_core::RuntimeValue::String(match &expected.values["object"] {
                QueryValue::String(value) => value.clone(),
                other => panic!("unexpected fixture object: {other:?}"),
            })
        );
    }
    let mut tiny = request.clone();
    tiny.max_output_bytes = 12;
    let denied = client
        .export_query(
            session,
            tiny,
            RequestOptions::read("parquet-small-read", "parquet-small-export").unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(denied, rrd_client::Error::Api { error, .. }
        if error.code == ErrorCode::ResourceExhausted));
    let mut foreign = request;
    foreign.query.scope = "instance:another-project".into();
    assert!(client
        .export_query(
            session,
            foreign,
            RequestOptions::read("parquet-foreign-read", "parquet-foreign-export").unwrap()
        )
        .await
        .is_err());
    let mut tampered = export;
    tampered.artifact_sha256 = "0".repeat(64);
    assert!(tampered.decode_artifact().is_err());
}

fn invoke(args: &[&str]) -> Output {
    let mut command = product_command();
    let mut process = process::Process::start(
        command.args(args),
        Duration::from_secs(120),
        OUTPUT_LIMIT,
        None,
    )
    .expect("run rrflow");
    let output = process
        .finish()
        .unwrap_or_else(|error| panic!("bounded rrflow command {:?}: {error}", args.get(1)));
    if args.contains(&"ready") {
        eprintln!(
            "native readiness output stdout_bytes={} stderr_bytes={}",
            output.stdout.len(),
            output.stderr.len()
        );
    }
    output
}

fn assert_success(output: &Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn capture_output(output: &Output, captured: &mut Vec<u8>) {
    assert!(
        captured.len() + output.stdout.len() + output.stderr.len() <= 128 * 1024 * 1024,
        "lifecycle output exceeded its aggregate limit"
    );
    captured.extend_from_slice(&output.stdout);
    captured.extend_from_slice(&output.stderr);
}

fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = std::fs::read_dir(current)
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, output);
            } else {
                output.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut output = BTreeMap::new();
    visit(root, root, &mut output);
    output
}

#[cfg(unix)]
fn stop_gracefully(child: &process::Process) {
    let mut command = Command::new("kill");
    command.args(["-INT", &child.id().to_string()]);
    let mut signal = process::Process::start(&mut command, Duration::from_secs(5), 4096, None)
        .expect("send SIGINT to the exact rrflow child");
    assert!(signal
        .finish()
        .expect("bounded signal helper")
        .status
        .success());
    let deadline = Instant::now() + Duration::from_secs(10);
    while child.running() {
        if Instant::now() >= deadline {
            child.terminate();
            panic!("rrflow serve did not complete graceful SIGINT shutdown");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

// Test-owned handles are always reaped, including assertion/startup failures.
// No PID search or process-name kill may affect another installed engine.
struct RunningEngine {
    child: process::Process,
    finished: bool,
}

impl RunningEngine {
    fn start(project: &Path, captured: &mut Vec<u8>) -> (Self, String) {
        let mut command = product_command();
        command
            .args(["--json", "serve", "--project"])
            .arg(project)
            .args(["--bind", "127.0.0.1:0"]);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        let mut process = Self {
            child: process::Process::start(
                &mut command,
                Duration::from_secs(480),
                OUTPUT_LIMIT,
                Some(send),
            )
            .expect("start bounded installed rrflow service"),
            finished: false,
        };
        assert!(process.child.id() > 0);
        let announcement = receive
            .recv_timeout(Duration::from_secs(60))
            .expect("installed engine must announce readiness within 60 seconds")
            .expect("read installed engine announcement");
        if announcement.is_empty() {
            process.stop(captured);
            panic!("serve exited before its listening announcement");
        }
        let announcement: serde_json::Value = serde_json::from_slice(&announcement).unwrap();
        assert_eq!(announcement["status"], "listening");
        let locator = RrdEngine::read_project_locator(project).unwrap();
        assert_eq!(
            announcement["instance_id"],
            locator.identity.instance_id.as_str()
        );
        static BUILD: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
        let build = BUILD.get_or_init(|| {
            let output = invoke(&["--json", "version"]);
            assert_success(&output, "build identity");
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["build"].clone()
        });
        assert_eq!(
            &announcement["build"], build,
            "serve/version identity differs"
        );
        let address = announcement["url"]
            .as_str()
            .unwrap()
            .strip_prefix("http://")
            .unwrap()
            .to_owned();
        (process, address)
    }

    fn terminate(&mut self, captured: &mut Vec<u8>) {
        self.child.terminate();
        let output = self.child.finish().expect("reap bounded rrflow process");
        capture_output(&output, captured);
        self.finished = true;
    }

    fn stop(&mut self, captured: &mut Vec<u8>) {
        if self.finished {
            return;
        }
        if self.child.running() {
            #[cfg(unix)]
            stop_gracefully(&self.child);
            #[cfg(not(unix))]
            self.child.terminate();
        }
        let output = self.child.finish().expect("capture bounded rrflow service");
        #[cfg(unix)]
        assert!(
            output.status.success(),
            "rrflow serve failed during shutdown"
        );
        capture_output(&output, captured);
        self.finished = true;
    }
}

fn assert_restarted_status(
    project: &Path,
    valid_at: u64,
    expected: &str,
    captured: &mut Vec<u8>,
) -> QueryResult {
    let (mut process, address) = RunningEngine::start(project, captured);
    let locator = RrdEngine::read_project_locator(project).unwrap();
    let ready = invoke(&[
        "--json",
        "ready",
        "--project",
        project.to_str().unwrap(),
        "--address",
        &address,
    ]);
    capture_output(&ready, captured);
    assert_success(&ready, "restarted authenticated discovery");
    let ready: serde_json::Value = serde_json::from_slice(&ready.stdout).unwrap();
    assert_eq!(ready["status"], "ready");
    assert_eq!(
        ready["capabilities"]["instance"]["id"],
        locator.identity.instance_id.as_str()
    );
    assert_eq!(ready["authentication"]["session_closed"], true);
    let credential = RrdEngine::read_installed_api_key(project).unwrap();
    let client = RrdClient::connect_local(
        address.parse().unwrap(),
        locator.identity.instance_id,
        ClientConfig::default(),
    )
    .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        let session = client
            .create_session(
                credential.principal_id.clone(),
                credential.credential(),
                CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 60_000,
                        absolute_timeout_ms: 300_000,
                        max_open_transactions: 1,
                    },
                },
                RequestOptions::mutation(
                    "restart-session-request",
                    "restart-session-operation",
                    "restart-session-idempotency",
                ).unwrap(),
            )
            .await
            .unwrap();
        let query = format!(
            "FROM claim:status AT VALID {valid_at} KNOWN HEAD WHERE subject = \"installed-walking-product\" PROJECT subject, object EXPLAIN CONTRACT"
        );
        let result = client
            .execute_query(
                &session,
                ExecuteQuery {
                    scope: format!("instance:{}", client.instance_id()),
                    query,
                    parameters: BTreeMap::new(),
                    budget: QueryBudget {
                        max_storage_keys: 75_000,
                        max_rows: 768,
                        max_output_bytes: 393_216,
                        max_batch_rows: 192,
                        max_memory_bytes: 33_554_432,
                        max_spill_bytes: 67_108_864,
                        max_elapsed_ms: 20_000,
                    },
                },
                RequestOptions::read("restart-query-request", "restart-query-operation").unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].identity, "claim:installed-walking-product:status");
        assert_eq!(result.rows[0].values.get("object"), Some(&QueryValue::String(expected.into())));
        verify_parquet_export(&client, &session, &result).await;
        client
            .close_session(
                &session,
                CloseSession {},
                RequestOptions::mutation(
                    "restart-close-request",
                    "restart-close-operation",
                    "restart-close-idempotency",
                ).unwrap(),
            )
            .await
            .unwrap();
        result
    });
    process.stop(captured);
    assert!(!String::from_utf8_lossy(captured).contains(credential.credential()));
    result
}

#[test]
fn primary_help_exposes_only_the_installed_product_lifecycle() {
    let output = invoke(&["--help"]);
    assert_success(&output, "primary help");
    let help = String::from_utf8(output.stdout).unwrap();
    for command in [
        "version", "install", "serve", "ready", "verify", "restore", "repair",
    ] {
        assert!(
            help.contains(command),
            "missing {command} from help:\n{help}"
        );
    }
    for removed in [
        "assert",
        "as-of",
        "history",
        "status",
        "invocations",
        "query",
        "context",
        "identity",
        "storage",
        "dev",
    ] {
        assert!(
            !help.lines().any(|line| {
                line.trim_start()
                    .strip_prefix(removed)
                    .is_some_and(|tail| tail.starts_with(char::is_whitespace))
            }),
            "removed command {removed} remains in help:\n{help}"
        );
    }
}

#[test]
fn raw_database_and_development_commands_fail_during_argument_parsing() {
    for removed in [
        "assert",
        "as-of",
        "history",
        "status",
        "invocations",
        "query",
        "context",
        "identity",
        "storage",
        "dev",
    ] {
        let output = invoke(&[removed]);
        assert!(!output.status.success(), "{removed} unexpectedly succeeded");
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(
            error.contains(&format!("unrecognized subcommand '{removed}'")),
            "unexpected {removed} rejection: {error}"
        );
    }

    for removed_option in ["--db", "--reader"] {
        let output = invoke(&[removed_option, "unbound", "version"]);
        assert!(
            !output.status.success(),
            "{removed_option} unexpectedly succeeded"
        );
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(
            error.contains("unexpected argument"),
            "unexpected {removed_option} rejection: {error}"
        );
    }
}

#[test]
fn install_requires_an_explicit_plan_digest_and_verify_is_read_only_by_shape() {
    let apply = invoke(&["install", "apply", "--project", "."]);
    assert!(!apply.status.success());
    let error = String::from_utf8(apply.stderr).unwrap();
    assert!(error.contains("--plan <PLAN>"));
    assert!(error.contains("--expect <EXPECT>"));

    let verify = invoke(&["verify", "--help"]);
    assert_success(&verify, "verify help");
    let help = String::from_utf8(verify.stdout).unwrap();
    assert!(help.contains("--level <LEVEL>"));
    assert!(help.contains("quick"));
    assert!(!help.contains("repair"));
    assert!(!help.contains("restore"));
}

#[test]
fn installed_engine_plans_applies_serves_discovers_authenticates_and_reopens() {
    installed_lifecycle("existing");
}

#[test]
fn fresh_installed_engine_survives_restart_and_restore() {
    installed_lifecycle("fresh");
}

fn installed_lifecycle(mode: &str) {
    eprintln!(
        "native {mode} lifecycle executable bytes={}",
        std::fs::metadata(binary()).unwrap().len()
    );
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    std::fs::create_dir(&project).unwrap();
    if mode == "existing" {
        std::fs::write(project.join("README.md"), "installed walking product\n").unwrap();
    }
    let plan_file = root.path().join("install-plan.json");
    let configuration_file = root.path().join("estate-configuration.toml");
    std::fs::write(
        &configuration_file,
        r#"format_version = 3

[storage]
mutable_accounted_bytes = 134217728
mutable_versions = 1048576
projected_output_bytes = 134217728
native_transactions = 64
native_transaction_bytes = 67108864
active_read_views = 1024
pinned_read_bytes = 4294967296
page_cache_bytes = 4194304
wal_flush_target_bytes = 67108864
memtable_flush_target_versions = 524288
compaction_input_bytes = 1073741824
compaction_input_logical_bytes = 268435456
compaction_input_versions = 4194304
compaction_output_segments = 256
compaction_elapsed_ms = 30000
compaction_l0_segments = 64
retained_mutable_bytes = 536870912
retained_mutable_versions = 4194304
retained_mutable_generations = 64
reclamation_inventory_entries = 262144
reclamation_metadata_bytes = 67108864
reclamation_delete_files = 1024
reclamation_delete_bytes = 1073741824
reclamation_elapsed_ms = 30000
transaction_admission_requests = 64
transaction_admission_bytes = 67108864
transaction_admission_wait_ms = 30000
recovery_metadata_bytes = 67108864
recovery_files = 262144
recovery_read_bytes = 1099511627776
recovery_wal_bytes = 268435456
recovery_wal_frames = 1048576
recovery_elapsed_ms = 120000
recovery_reserve_bytes = 16777216

[clock]
maximum_rollback_ms = 0

[reasoning]
max_run_elapsed_ms = 720000
max_steps = 192
max_step_elapsed_ms = 45000

[recall]
max_graph_depth = 3
max_items = 96
max_output_bytes = 393216
max_storage_keys = 75000

[query]
max_storage_keys = 75000
max_rows = 768
max_output_bytes = 393216
max_batch_rows = 192
max_memory_bytes = 33554432
max_spill_bytes = 67108864
max_elapsed_ms = 20000
"#,
    )
    .unwrap();
    let project_arg = project.to_str().unwrap();
    let plan_arg = plan_file.to_str().unwrap();
    let configuration_arg = configuration_file.to_str().unwrap();
    let mut lifecycle_output = Vec::new();

    let version = invoke(&["--json", "version"]);
    capture_output(&version, &mut lifecycle_output);
    assert_success(&version, "version");
    let version: serde_json::Value = serde_json::from_slice(&version.stdout).unwrap();
    assert_eq!(version["product_version"], "1.0.0");
    assert_eq!(version["installation_plan_contract_version"], 4);
    assert_eq!(version["release_stage"], "pre-release");

    let before = inventory(&project);
    let identity_output = invoke(&["install", "identity"]);
    assert_success(&identity_output, "allocate explicit installation identity");
    let identity: serde_json::Value = serde_json::from_slice(&identity_output.stdout).unwrap();
    let identity = identity["installation_identity"].as_str().unwrap();
    assert_eq!(identity.len(), 64);
    let next_identity = invoke(&["install", "identity"]);
    assert_success(&next_identity, "allocate another independent identity");
    assert_ne!(identity_output.stdout, next_identity.stdout);
    assert_eq!(
        inventory(&project),
        before,
        "identity allocation wrote project state"
    );
    let plan_arguments = [
        "install",
        "plan",
        "--identity",
        identity,
        "--mode",
        mode,
        "--project",
        project_arg,
        "--configuration",
        configuration_arg,
    ];
    let first = invoke(&plan_arguments);
    let second = invoke(&plan_arguments);
    capture_output(&first, &mut lifecycle_output);
    capture_output(&second, &mut lifecycle_output);
    assert_success(&first, "first install plan");
    assert_success(&second, "second install plan");
    assert_eq!(first.stdout, second.stdout, "preview must be byte stable");
    assert_eq!(inventory(&project), before, "preview mutated the project");
    let preview: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(preview["installation"]["contract_version"], 4);
    assert_eq!(preview["installation"]["installation_identity"], identity);
    assert_eq!(
        preview["installation"]["configuration"]["reasoning"]["max_run_elapsed_ms"],
        720_000
    );
    assert_eq!(
        preview["installation"]["configuration"]["recall"]["max_items"],
        96
    );
    assert_eq!(
        preview["installation"]["deployment"]["deployment_form"],
        "single_node_server"
    );
    let managed_paths = preview["installation"]["managed_paths"].as_array().unwrap();
    assert_eq!(managed_paths.len(), 13);
    let bootstrap: rrd_contract::InstallationBootstrap =
        serde_json::from_value(preview["installation"]["bootstrap"].clone()).unwrap();
    bootstrap.validate().unwrap();
    assert_eq!(bootstrap.specialization.id.as_str(), "generic");
    assert!(bootstrap.specialization.endpoint_profile.is_none());
    assert!(bootstrap.process_invocations.is_empty());
    assert!(bootstrap.network_endpoints.is_empty());

    assert!(managed_paths
        .iter()
        .any(|path| path["relative_path"] == ".rrflow/config.toml"));
    assert!(managed_paths
        .iter()
        .all(|path| !path["relative_path"].as_str().unwrap().starts_with('/')));
    let plan_sha256 = preview["installation"]["plan_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::write(&plan_file, &first.stdout).unwrap();

    let foreign_project = root.path().join("foreign-project");
    std::fs::create_dir(&foreign_project).unwrap();
    for (relative, content) in &before {
        std::fs::write(foreign_project.join(relative), content).unwrap();
    }
    let foreign = invoke(&[
        "install",
        "apply",
        "--mode",
        mode,
        "--project",
        foreign_project.to_str().unwrap(),
        "--plan",
        plan_arg,
        "--expect",
        &plan_sha256,
    ]);
    assert!(
        !foreign.status.success(),
        "a plan cannot initialize an identical foreign project"
    );
    assert_eq!(
        inventory(&foreign_project),
        before,
        "foreign apply wrote managed state"
    );

    let rejected = invoke(&[
        "install",
        "apply",
        "--mode",
        mode,
        "--project",
        project_arg,
        "--plan",
        plan_arg,
        "--expect",
        &"0".repeat(64),
    ]);
    capture_output(&rejected, &mut lifecycle_output);
    assert!(!rejected.status.success());
    assert!(!project.join(".rrflow").exists());

    let applied = invoke(&[
        "--json",
        "install",
        "apply",
        "--mode",
        mode,
        "--project",
        project_arg,
        "--plan",
        plan_arg,
        "--expect",
        &plan_sha256,
    ]);
    capture_output(&applied, &mut lifecycle_output);
    assert_success(&applied, "install apply");
    assert_eq!(
        std::fs::read(project.join(".rrflow/bootstrap.md")).unwrap(),
        bootstrap.rendered_instructions().as_bytes()
    );
    for (relative, bytes) in &before {
        assert_eq!(
            std::fs::read(project.join(relative)).unwrap(),
            *bytes,
            "installation changed project-owned content"
        );
    }

    let applied: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    assert_eq!(applied["plan_sha256"], plan_sha256);
    assert_eq!(applied["runtime_cursor"], 2);
    assert_eq!(applied["control_journal_sequence"], 8);
    assert_eq!(applied["idempotent_replay"], false);
    let installed_at = applied["applied_at_unix_ms"].as_u64().unwrap();

    let installed_before_verify = inventory(&project);
    let credential_path = project.join(
        managed_paths
            .iter()
            .find(|path| path["kind"] == "operator_credential")
            .unwrap()["relative_path"]
            .as_str()
            .unwrap(),
    );
    let alias = root.path().join("foreign-credential-alias");
    std::fs::hard_link(&credential_path, &alias).unwrap();
    let denied_verify = invoke(&[
        "--json",
        "verify",
        "--project",
        project_arg,
        "--level",
        "quick",
    ]);
    capture_output(&denied_verify, &mut lifecycle_output);
    assert!(
        !denied_verify.status.success(),
        "aliased credentials must fail verification"
    );
    assert_eq!(
        inventory(&project),
        installed_before_verify,
        "denied verify changed state"
    );
    assert!(
        std::fs::read(&alias).unwrap() == std::fs::read(&credential_path).unwrap(),
        "denial changed credential bytes"
    );
    std::fs::remove_file(&alias).unwrap();
    let verified = invoke(&[
        "--json",
        "verify",
        "--project",
        project_arg,
        "--level",
        "quick",
    ]);
    capture_output(&verified, &mut lifecycle_output);
    assert_success(&verified, "offline verify");
    let verified: serde_json::Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(verified["status"], "passed");
    assert_eq!(verified["runtime_cursor"], 2);
    assert_eq!(verified["clock"]["status"], "current");
    assert_eq!(verified["clock"]["anchor"]["source"], "host_system_time");
    assert_eq!(verified["clock"]["anchor"]["trust"], "unverified");
    assert_eq!(
        verified["clock"]["anchor"]["observed_at_unix_ms"],
        installed_at
    );
    assert_eq!(inventory(&project), installed_before_verify);

    let replay = invoke(&[
        "--json",
        "install",
        "apply",
        "--mode",
        mode,
        "--project",
        project_arg,
        "--plan",
        plan_arg,
        "--expect",
        &plan_sha256,
    ]);
    capture_output(&replay, &mut lifecycle_output);
    assert_success(&replay, "idempotent install replay");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&replay.stdout).unwrap()["idempotent_replay"],
        true
    );

    let (mut child, address) = RunningEngine::start(&project, &mut lifecycle_output);
    let address = address.as_str();

    let ready = invoke(&[
        "--json",
        "ready",
        "--project",
        project_arg,
        "--address",
        address,
    ]);
    capture_output(&ready, &mut lifecycle_output);
    assert_success(&ready, "live UI discovery");
    let ready: serde_json::Value = serde_json::from_slice(&ready.stdout).unwrap();
    assert_eq!(ready["status"], "ready");
    assert_eq!(ready["readiness"]["runtime_cursor"], 2);
    assert_eq!(ready["authentication"]["principal_id"], "local-operator");
    assert_eq!(ready["authentication"]["session_closed"], true);
    assert_eq!(ready["openapi"]["openapi"], "3.1.0");
    assert_eq!(
        ready["capabilities"]["configuration"]["reasoning"]["max_run_elapsed_ms"],
        720_000
    );
    assert_eq!(
        ready["capabilities"]["configuration"]["recall"]["max_items"],
        96
    );
    assert_eq!(
        ready["capabilities"]["deployment"]["endpoint_presentation"],
        "loopback_http_websocket"
    );
    assert!(
        ready["endpoint_catalogue"]["endpoints"]
            .as_array()
            .unwrap()
            .len()
            > 20
    );

    let locator = RrdEngine::read_project_locator(&project).unwrap();
    let credential = RrdEngine::read_installed_api_key(&project).unwrap();
    let client = RrdClient::connect_local(
        address.parse().unwrap(),
        locator.identity.instance_id,
        // Backup/restore include authenticated physical verification. Their
        // explicit budget is separate from the default short-request timeout;
        // overlapping attempts are covered by deterministic engine tests.
        ClientConfig {
            request_timeout: Duration::from_secs(60),
            ..ClientConfig::default()
        },
    )
    .unwrap();
    let (backup_sha256, first_query, committed_claim_sequence) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let session = client
                .create_session(
                    credential.principal_id.clone(),
                    credential.credential(),
                    CreateSession {
                        limits: SessionLimits {
                            idle_timeout_ms: 60_000,
                            absolute_timeout_ms: 300_000,
                            max_open_transactions: 1,
                        },
                    },
                    RequestOptions::mutation(
                        "installed-test-session-request",
                        "installed-test-session-operation",
                        "installed-test-session-idempotency",
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            let transaction = client
                .begin_transaction(
                    &session,
                    BeginTransaction {
                        scope: CanonicalId::new("claims").unwrap(),
                        timeout_ms: 10_000,
                    },
                    RequestOptions::mutation(
                        "installed-test-begin-request",
                        "installed-test-begin-operation",
                        "installed-test-begin-idempotency",
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            let mutations = vec![TransactionMutation::AssertClaim {
                subject: CanonicalId::new("installed-walking-product").unwrap(),
                predicate: CanonicalId::new("status").unwrap(),
                object: "ui-queryable".into(),
                valid_from: installed_at,
                tx_time: installed_at,
                producer: CanonicalId::new("local-operator").unwrap(),
                confidence: Some(1.0),
            }];
            let receipt = client
                .commit_transaction(
                    &session,
                    &transaction.transaction_id,
                    CommitTransaction {
                        operation_sha256: transaction_operation_sha256(&mutations),
                        mutations,
                    },
                    RequestOptions::new(
                        "installed-test-commit-request",
                        "installed-test-commit-operation",
                        Some("installed-test-commit-idempotency"),
                        Some(u64::MAX),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(receipt.mutation_count, 1);

            let query_text = format!(
                "FROM claim:status AT VALID {installed_at} KNOWN HEAD WHERE subject = \"installed-walking-product\" PROJECT subject, object EXPLAIN CONTRACT"
            );
            let denied = client
                .execute_query(
                    &session,
                    ExecuteQuery {
                        scope: format!("instance:{}", client.instance_id()),
                        query: query_text.clone(),
                        parameters: BTreeMap::new(),
                        budget: QueryBudget::default(),
                    },
                    RequestOptions::read(
                        "installed-test-over-ceiling-request",
                        "installed-test-over-ceiling-operation",
                    )
                    .unwrap(),
                )
                .await
                .unwrap_err();
            match denied {
                rrd_client::Error::Api { status, error } => {
                    assert_eq!(status.as_u16(), 429);
                    assert_eq!(error.code, ErrorCode::ResourceExhausted);
                }
                other => panic!("unexpected configured-ceiling error: {other}"),
            }

            let query = client
                .execute_query(
                    &session,
                    ExecuteQuery {
                        scope: format!("instance:{}", client.instance_id()),
                        query: query_text,
                        parameters: BTreeMap::new(),
                        budget: QueryBudget {
                            max_storage_keys: 75_000,
                            max_rows: 768,
                            max_output_bytes: 393_216,
                            max_batch_rows: 192,
                            max_memory_bytes: 33_554_432,
                            max_spill_bytes: 67_108_864,
                            max_elapsed_ms: 20_000,
                        },
                    },
                    RequestOptions::read(
                        "installed-test-query-request",
                        "installed-test-query-operation",
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(query.rows.len(), 1);
            assert_eq!(
                query.rows[0].identity,
                "claim:installed-walking-product:status"
            );
            assert_eq!(
                query.rows[0].values.get("object"),
                Some(&QueryValue::String("ui-queryable".into()))
            );
            verify_parquet_export(&client, &session, &query).await;

            // A backup at runtime cursor 3, a later commit the backup does not
            // carry, and a staged restore of the backup for offline publication.
            let backup = client
                .create_backup(
                    &session,
                    CreateInstanceBackup {
                        label: "cli-publication".into(),
                        created_at_unix_ms: installed_at + 1_000,
                    },
                    RequestOptions::mutation(
                        "installed-test-backup-request",
                        "installed-test-backup-operation",
                        "installed-test-backup-idempotency",
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            assert!(backup.backup.application_complete);
            assert_eq!(
                backup.backup.installed_control,
                BackupCoverageSnapshot::Included,
                "the SDK-visible backup names the installed control state it carries"
            );
            let later = client
                .begin_transaction(
                    &session,
                    BeginTransaction {
                        scope: CanonicalId::new("claims").unwrap(),
                        timeout_ms: 10_000,
                    },
                    RequestOptions::mutation(
                        "installed-test-later-begin-request",
                        "installed-test-later-begin-operation",
                        "installed-test-later-begin-idempotency",
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            let later_mutations = vec![TransactionMutation::AssertClaim {
                subject: CanonicalId::new("installed-walking-product").unwrap(),
                predicate: CanonicalId::new("status").unwrap(),
                object: "after-backup".into(),
                valid_from: installed_at + 2_000,
                tx_time: installed_at + 2_000,
                producer: CanonicalId::new("local-operator").unwrap(),
                confidence: Some(1.0),
            }];
            client
                .commit_transaction(
                    &session,
                    &later.transaction_id,
                    CommitTransaction {
                        operation_sha256: transaction_operation_sha256(&later_mutations),
                        mutations: later_mutations,
                    },
                    RequestOptions::new(
                        "installed-test-later-commit-request",
                        "installed-test-later-commit-operation",
                        Some("installed-test-later-commit-idempotency"),
                        Some(u64::MAX),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            let staged = client
                .restore_backup(
                    &session,
                    RestoreInstanceBackup {
                        backup_sha256: backup.backup.backup_sha256.clone(),
                        restore_id: CanonicalId::new("restore-cli").unwrap(),
                        restored_at_unix_ms: installed_at + 3_000,
                    },
                    RequestOptions::mutation(
                        "installed-test-restore-request",
                        "installed-test-restore-operation",
                        "installed-test-restore-idempotency",
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(staged.inventory.runtime_cursor, 3);
            client
                .close_session(
                    &session,
                    CloseSession {},
                    RequestOptions::mutation(
                        "installed-test-close-request",
                        "installed-test-close-operation",
                        "installed-test-close-idempotency",
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            (backup.backup.backup_sha256, query, receipt.last_claim_sequence)
        });

    let live_verify = invoke(&["verify", "--project", project_arg, "--level", "quick"]);
    capture_output(&live_verify, &mut lifecycle_output);
    assert!(
        !live_verify.status.success(),
        "offline verifier must not race the live writer"
    );

    child.terminate(&mut lifecycle_output);
    assert_restarted_status(
        &project,
        installed_at + 2_000,
        "after-backup",
        &mut lifecycle_output,
    );
    let reopened = invoke(&[
        "--json",
        "verify",
        "--project",
        project_arg,
        "--level",
        "quick",
    ]);
    capture_output(&reopened, &mut lifecycle_output);
    assert_success(&reopened, "post-shutdown reopen and verify");
    let reopened: serde_json::Value = serde_json::from_slice(&reopened.stdout).unwrap();
    assert_eq!(reopened["status"], "passed");
    assert_eq!(reopened["runtime_cursor"], 4);
    assert!(reopened["control_journal_sequence"].as_u64().unwrap() > 8);

    // Offline publication of the staged restore through the primary command
    // tree: a deterministic, no-write plan, an exact-digest apply, and a
    // verified estate at the backup's state with the old root quarantined.
    let publication_plan_file = root.path().join("restore-plan.json");
    let publication_plan_arg = publication_plan_file.to_str().unwrap();
    let plan_arguments = [
        "restore",
        "plan",
        "--project",
        project_arg,
        "--restore",
        "restore-cli",
        "--backup",
        &backup_sha256,
    ];
    let before_publication = inventory(&project);
    let first_plan = invoke(&plan_arguments);
    let second_plan = invoke(&plan_arguments);
    capture_output(&first_plan, &mut lifecycle_output);
    assert_success(&first_plan, "restore plan");
    assert_success(&second_plan, "second restore plan");
    assert_eq!(
        first_plan.stdout, second_plan.stdout,
        "plan must be byte stable"
    );
    assert_eq!(
        inventory(&project),
        before_publication,
        "planning mutated the project"
    );
    let publication: serde_json::Value = serde_json::from_slice(&first_plan.stdout).unwrap();
    assert_eq!(publication["active_root"], locator.storage_root);
    assert_eq!(publication["candidate"]["runtime_cursor"], 3);
    assert_eq!(publication["active"]["runtime_cursor"], 4);
    let publication_sha256 = publication["plan_sha256"].as_str().unwrap().to_owned();
    std::fs::write(&publication_plan_file, &first_plan.stdout).unwrap();
    let rejected = invoke(&[
        "restore",
        "apply",
        "--project",
        project_arg,
        "--plan",
        publication_plan_arg,
        "--expect",
        &"0".repeat(64),
    ]);
    capture_output(&rejected, &mut lifecycle_output);
    assert!(!rejected.status.success());
    assert_eq!(inventory(&project), before_publication);
    let published = invoke(&[
        "--json",
        "restore",
        "apply",
        "--project",
        project_arg,
        "--plan",
        publication_plan_arg,
        "--expect",
        &publication_sha256,
    ]);
    capture_output(&published, &mut lifecycle_output);
    assert_success(&published, "restore apply");
    let published: serde_json::Value = serde_json::from_slice(&published.stdout).unwrap();
    assert_eq!(published["plan_sha256"], publication_sha256);
    assert_eq!(published["runtime_cursor"], 3);
    assert_eq!(published["idempotent_replay"], false);
    assert!(project
        .join(published["quarantine_root"].as_str().unwrap())
        .is_dir());
    let after_publication = invoke(&[
        "--json",
        "verify",
        "--project",
        project_arg,
        "--level",
        "quick",
    ]);
    capture_output(&after_publication, &mut lifecycle_output);
    assert_success(&after_publication, "post-publication verify");
    let after_publication: serde_json::Value =
        serde_json::from_slice(&after_publication.stdout).unwrap();
    assert_eq!(after_publication["status"], "passed");
    assert_eq!(after_publication["runtime_cursor"], 3);

    let restored_query = assert_restarted_status(
        &project,
        installed_at,
        "ui-queryable",
        &mut lifecycle_output,
    );
    assert!(restored_query.known_at_cursor >= committed_claim_sequence);
    assert_eq!(restored_query.canonical_query, first_query.canonical_query);
    assert_eq!(restored_query.scope, first_query.scope);
    assert_eq!(restored_query.schema_revision, first_query.schema_revision);
    assert_eq!(
        restored_query.read_manifest_sha256,
        first_query.read_manifest_sha256
    );
    assert_eq!(restored_query.rows, first_query.rows);
    for (relative, bytes) in before {
        assert_eq!(std::fs::read(project.join(relative)).unwrap(), bytes);
    }

    // Neither successful nor rejected lifecycle stdout/stderr may contain the
    // generated credential.
    assert!(!String::from_utf8_lossy(&lifecycle_output).contains(credential.credential()));
    println!(
        "native_lifecycle_receipt={}",
        serde_json::json!({
            "mode": mode,
            "build": {
                "executable_sha256": version["build"]["executable_sha256"],
                "metadata_sha256": version["build"]["metadata_sha256"],
                "public_contract_sha256": version["build"]["public_contract_sha256"],
                "signal_catalogue_sha256": version["build"]["signal_catalogue_sha256"],
            },
            "installation_plan_sha256": plan_sha256,
            "restore_plan_sha256": publication_sha256,
            "backup_sha256": backup_sha256,
            "installed_runtime_cursor": applied["runtime_cursor"],
            "reopened_runtime_cursor": reopened["runtime_cursor"],
            "restored_runtime_cursor": after_publication["runtime_cursor"],
            "committed_claim_sequence": committed_claim_sequence,
            "restored_read_manifest_sha256": restored_query.read_manifest_sha256,
            "captured_output_bytes": lifecycle_output.len(),
            "stream_limit_bytes": OUTPUT_LIMIT,
        })
    );
    storage_history::run(
        &project,
        mode,
        installed_at,
        &version["build"],
        &mut lifecycle_output,
    );
}
