//! Product commands, Parquet export and the primary command surface.

use super::*;

pub(super) fn product_command() -> Command {
    DISTRIBUTION.with(StandaloneDistribution::command)
}

pub(super) async fn verify_parquet_export(
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

pub(super) fn invoke(args: &[&str]) -> Output {
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

pub(super) fn assert_success(output: &Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(super) fn capture_output(output: &Output, captured: &mut Vec<u8>) {
    assert!(
        captured.len() + output.stdout.len() + output.stderr.len() <= 128 * 1024 * 1024,
        "lifecycle output exceeded its aggregate limit"
    );
    captured.extend_from_slice(&output.stdout);
    captured.extend_from_slice(&output.stderr);
}

pub(super) fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
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
pub(super) fn stop_gracefully(child: &process::Process) {
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

pub(super) fn repository_discovery_is_read_only_without_an_installed_engine() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("source.txt"), b"project-owned").unwrap();
    let before = inventory(project.path());
    let output = invoke(&[
        "--json",
        "install",
        "discover",
        "--project",
        project.path().to_str().unwrap(),
    ]);
    assert_success(&output, "repository discovery");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["git_index_verified"], false);
    assert_eq!(report["selection_available"], false);
    assert_eq!(
        report["coverage"],
        "present_tracked_gitlink_directories_only"
    );
    assert_eq!(report["candidates"], serde_json::json!([]));
    assert_eq!(inventory(project.path()), before);
    assert!(!project.path().join(".rrflow").exists());
}

pub(super) fn primary_help_exposes_only_the_installed_product_lifecycle() {
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

pub(super) fn raw_database_and_development_commands_fail_during_argument_parsing() {
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
