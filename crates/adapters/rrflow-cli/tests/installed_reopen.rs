//! C-07c ordinary-primary ACK/kill/reopen corpus. Explicitly run by native CI.
//! This fixed correctness workload is not a capacity profile or a soak.
#[path = "support/process.rs"]
mod process;
#[path = "support/sustained.rs"]
mod sustained;

use rrd_client::{ClientConfig, RequestOptions, RrdClient};
use rrd_contract::{
    transaction_operation_sha256, BeginTransaction, CanonicalId, CommitReceipt, CommitTransaction,
    CreateSession, ExecuteQuery, QueryBudget, QueryValue, SessionLimits, TransactionMutation,
};
use rrd_engine::RrdEngine;
use rrflow_eval::storage::{
    digest, AcknowledgementLedger, Attempt, AttemptOutcome, ExpectedOperation, ExpectedResolution,
    Readback,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Read,
    path::PathBuf,
    process::Command,
    sync::mpsc,
    time::{Duration, Instant},
};

const CYCLES: usize = 100;
const STREAM_LIMIT: usize = 1024 * 1024;
const CORPUS_SECONDS: u64 = 3600;

struct Estate {
    temp: tempfile::TempDir,
    binary: PathBuf,
    project: PathBuf,
    build: Value,
    plan: Value,
    at: u64,
}

impl Estate {
    fn command(&self) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .env_clear()
            .current_dir(self.temp.path().join("working"));
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        for name in ["HOME", "USERPROFILE", "TMP", "TEMP", "TMPDIR", "PATH"] {
            command.env(name, self.temp.path().join("working"));
        }
        command
    }

    fn invoke(&self, args: &[&str]) -> Value {
        let mut child = process::Process::start(
            self.command().args(args),
            Duration::from_secs(120),
            STREAM_LIMIT,
            None,
        )
        .unwrap();
        let output = child.finish().unwrap();
        assert!(
            output.status.success(),
            "installed command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn install(mode: &str) -> Self {
        Self::install_configured(mode, None)
    }

    fn install_configured(mode: &str, configuration: Option<&std::path::Path>) -> Self {
        let temp = tempfile::Builder::new()
            .prefix("rrflow reopen ")
            .tempdir()
            .unwrap();
        let binary = temp.path().join(if cfg!(windows) {
            "rrflow.exe"
        } else {
            "rrflow"
        });
        process::stage_executable(std::path::Path::new(env!("CARGO_BIN_EXE_rrflow")), &binary)
            .unwrap();
        // Scan the actual staged artifact in bounded chunks, including seams.
        let marker = b"C07C_NATIVE_PAUSE_V1=";
        let mut file = std::fs::File::open(&binary).unwrap();
        let mut chunk = vec![0; STREAM_LIMIT];
        let mut tail = Vec::new();
        let mut artifact_digest = Sha256::new();
        loop {
            let count = file.read(&mut chunk).unwrap();
            if count == 0 {
                break;
            }
            artifact_digest.update(&chunk[..count]);
            tail.extend_from_slice(&chunk[..count]);
            assert!(
                !tail.windows(marker.len()).any(|bytes| bytes == marker),
                "diagnostic primary is not ordinary evidence"
            );
            assert!(
                !tail
                    .windows(b"C07F_NATIVE_IO_V1=".len())
                    .any(|bytes| bytes == b"C07F_NATIVE_IO_V1="),
                "native I/O observer is not ordinary evidence"
            );
            tail = tail[tail.len().saturating_sub(marker.len() - 1)..].to_vec();
        }
        let project = temp.path().join("project");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(temp.path().join("working")).unwrap();
        if mode == "existing" {
            std::fs::write(project.join("README.md"), "project-owned\n").unwrap();
        }
        let mut estate = Self {
            temp,
            binary,
            project,
            build: Value::Null,
            plan: Value::Null,
            at: 0,
        };
        estate.build = estate.invoke(&["--json", "version"])["build"].clone();
        let artifact_sha256 = artifact_digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(
            estate.build["executable_sha256"], artifact_sha256,
            "reported build does not identify the actual staged bytes"
        );
        let identity = estate.invoke(&["install", "identity"])["installation_identity"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut arguments = vec![
            "install",
            "plan",
            "--identity",
            &identity,
            "--mode",
            mode,
            "--project",
            estate.project.to_str().unwrap(),
        ];
        if let Some(configuration) = configuration {
            arguments.extend(["--configuration", configuration.to_str().unwrap()]);
        }
        let preview = estate.invoke(&arguments);
        let path = estate.temp.path().join("plan.json");
        std::fs::write(&path, serde_json::to_vec(&preview).unwrap()).unwrap();
        let applied = estate.invoke(&[
            "--json",
            "install",
            "apply",
            "--mode",
            mode,
            "--project",
            estate.project.to_str().unwrap(),
            "--plan",
            path.to_str().unwrap(),
            "--expect",
            preview["installation"]["plan_sha256"].as_str().unwrap(),
        ]);
        assert_eq!(applied["runtime_cursor"], 2);
        estate.at = applied["applied_at_unix_ms"].as_u64().unwrap();
        estate.plan = preview["installation"].clone();
        estate
    }

    fn start(&self, clock: Instant) -> (process::Process, RrdClient, Value) {
        self.start_for(clock, Duration::from_secs(180))
    }

    fn start_for(
        &self,
        clock: Instant,
        lifetime: Duration,
    ) -> (process::Process, RrdClient, Value) {
        let started_ns = clock.elapsed().as_nanos() as u64;
        let mut command = self.command();
        command
            .args(["--json", "serve", "--project"])
            .arg(&self.project)
            .args(["--bind", "127.0.0.1:0"]);
        let (send, receive) = mpsc::sync_channel(1);
        let child =
            process::Process::start(&mut command, lifetime, STREAM_LIMIT, Some(send)).unwrap();
        let announcement: Value = serde_json::from_slice(
            &receive
                .recv_timeout(Duration::from_secs(60))
                .expect("bounded ordinary startup")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(announcement["status"], "listening");
        assert_eq!(announcement["build"], self.build);
        let locator = RrdEngine::read_project_locator(&self.project).unwrap();
        assert_eq!(
            locator.configuration_sha256,
            self.plan["configuration"]["configuration_sha256"]
                .as_str()
                .unwrap()
        );
        assert_eq!(
            announcement["instance_id"],
            locator.identity.instance_id.as_str()
        );
        let client = RrdClient::connect_local(
            announcement["url"]
                .as_str()
                .unwrap()
                .strip_prefix("http://")
                .unwrap()
                .parse()
                .unwrap(),
            locator.identity.instance_id,
            ClientConfig {
                request_timeout: Duration::from_secs(60),
                max_attempts: 1,
                ..ClientConfig::default()
            },
        )
        .unwrap();
        let identity = json!({"pid":child.id(),"started_ns":started_ns,"ready_ns":clock.elapsed().as_nanos() as u64});
        (child, client, identity)
    }
}

fn kill(mut child: process::Process, clock: Instant) -> Value {
    assert!(child.running(), "owned server exited before requested kill");
    let pid = child.id();
    let killed_ns = clock.elapsed().as_nanos() as u64;
    child.terminate();
    let output = child.finish().expect("reap every owned server");
    assert!(!output.status.success());
    assert!(output.stdout.len() <= STREAM_LIMIT && output.stderr.len() <= STREAM_LIMIT);
    json!({"pid":pid,"killed_ns":killed_ns,"reaped_ns":clock.elapsed().as_nanos() as u64,
        "exit_status":output.status.to_string(),"stdout_bytes":output.stdout.len(),"stderr_bytes":output.stderr.len()})
}

fn options(name: &str) -> RequestOptions {
    RequestOptions::new(name, name, Some(name), Some(u64::MAX)).unwrap()
}

fn request(cycle: usize, at: u64) -> CommitTransaction {
    let mutations = (0..2)
        .map(|part| TransactionMutation::AssertClaim {
            subject: CanonicalId::new(format!("reopen-{cycle}-{part}")).unwrap(),
            predicate: CanonicalId::new("reopen").unwrap(),
            object: format!("expected-{cycle}-{part}"),
            valid_from: at,
            tx_time: at,
            producer: CanonicalId::new("reopen-oracle").unwrap(),
            confidence: None,
        })
        .collect::<Vec<_>>();
    CommitTransaction {
        operation_sha256: transaction_operation_sha256(&mutations),
        mutations,
    }
}

fn expected(cycle: usize) -> BTreeMap<String, String> {
    (0..2)
        .map(|part| {
            (
                format!("claim:reopen-{cycle}-{part}:reopen"),
                format!("expected-{cycle}-{part}"),
            )
        })
        .collect()
}

fn receipt_digest(receipt: &CommitReceipt) -> rrflow_eval::storage::Digest {
    let mut stable = receipt.clone();
    stable.idempotent_replay = false;
    digest("c07c.reopen.receipt", &stable).unwrap()
}

fn record(
    ledger: &mut AcknowledgementLedger,
    cycle: usize,
    started_ns: u64,
    clock: Instant,
    receipt: &CommitReceipt,
) {
    ledger.attempts.push(Attempt {
        attempt_id: format!("attempt-{}", ledger.attempts.len())
            .try_into()
            .unwrap(),
        logical_id: format!("cycle-{cycle}").try_into().unwrap(),
        request_sha256: ledger.expected[cycle].request_sha256.clone(),
        started_ns,
        finished_ns: clock.elapsed().as_nanos() as u64,
        outcome: AttemptOutcome::Acknowledged {
            receipt_sha256: receipt_digest(receipt),
        },
    });
}

async fn read(
    client: &RrdClient,
    session: &rrd_client::Session,
    at: u64,
    count: usize,
) -> BTreeMap<String, String> {
    let query = ExecuteQuery {
        scope: format!("instance:{}", client.instance_id()),
        query: format!(
            "FROM claim:reopen AT VALID {at} KNOWN HEAD PROJECT subject, object EXPLAIN CONTRACT"
        ),
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
    };
    let result = client
        .execute_query(
            session,
            query,
            RequestOptions::read("reopen-read", "reopen-read").unwrap(),
        )
        .await
        .unwrap();
    assert!(!result.execution.truncated);
    assert_eq!(
        result.known_at_cursor,
        (count * 2 + 2) as u64,
        "retry appended duplicate effects"
    );
    let mut values = BTreeMap::new();
    for row in result.rows {
        let QueryValue::String(value) = &row.values["object"] else {
            panic!("wrong recovered type")
        };
        assert!(values.insert(row.identity, value.clone()).is_none());
    }
    let requested = (0..count).flat_map(expected).collect::<BTreeMap<_, _>>();
    assert_eq!(
        values, requested,
        "lost, partial, changed or unexpected atomic effect"
    );
    values
}

fn run(mode: &str) {
    let estate = Estate::install(mode);
    let credential = RrdEngine::read_installed_api_key(&estate.project).unwrap();
    // Freeze every request/effect identity before the first operation.
    let requests = (0..CYCLES)
        .map(|cycle| request(cycle, estate.at))
        .collect::<Vec<_>>();
    let mut ledger = AcknowledgementLedger::default();
    for (cycle, request) in requests.iter().enumerate() {
        ledger.expected.push(ExpectedOperation {
            logical_id: format!("cycle-{cycle}").try_into().unwrap(),
            request_sha256: digest("c07c.reopen.request", request).unwrap(),
            effect_sha256: digest("c07c.reopen.effect", &expected(cycle)).unwrap(),
            resolution: ExpectedResolution::Present,
        });
    }
    let expected_sha256 = digest("c07c.reopen.expected", &ledger.expected).unwrap();
    let clock = Instant::now();
    let mut cycles = Vec::new();
    let mut stable = Vec::new();
    let mut leases = Vec::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let cleanup = runtime.block_on(async {
        let (mut child, mut client, initial) = estate.start(clock);
        let session = client
            .create_session(
                credential.principal_id.clone(),
                credential.credential(),
                CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 600_000,
                        absolute_timeout_ms: 3_600_000,
                        max_open_transactions: 2,
                    },
                },
                options("reopen-session"),
            )
            .await
            .unwrap();
        let mut previous = initial;
        for (cycle, request) in requests.iter().enumerate() {
            assert!(
                clock.elapsed() < Duration::from_secs(CORPUS_SECONDS),
                "fixed corpus deadline exceeded"
            );
            let lease = client
                .begin_transaction(
                    &session,
                    BeginTransaction {
                        scope: CanonicalId::new("claims").unwrap(),
                        timeout_ms: 600_000,
                    },
                    options(&format!("begin-{cycle}")),
                )
                .await
                .unwrap();
            let started = clock.elapsed().as_nanos() as u64;
            let receipt = client
                .commit_transaction(
                    &session,
                    &lease.transaction_id,
                    request.clone(),
                    options(&format!("commit-{cycle}")),
                )
                .await
                .unwrap();
            assert!(!receipt.idempotent_replay);
            assert_eq!(receipt.mutation_count, 2);
            assert_eq!(receipt.first_runtime_cursor, Some((cycle * 2 + 3) as u64));
            assert_eq!(receipt.last_runtime_cursor, Some((cycle * 2 + 4) as u64));
            record(&mut ledger, cycle, started, clock, &receipt);
            stable.push(receipt_digest(&receipt));
            leases.push(lease);
            let terminated = kill(child, clock);
            assert_eq!(terminated["pid"], previous["pid"]);
            let (reopened, reconnected, identity) = estate.start(clock);
            child = reopened;
            client = reconnected;
            // Read before retry so retry cannot conceal missing persisted effects.
            let observed = read(&client, &session, estate.at, cycle + 1).await;
            for index in 0..=cycle {
                let started = clock.elapsed().as_nanos() as u64;
                let replay = client
                    .commit_transaction(
                        &session,
                        &leases[index].transaction_id,
                        requests[index].clone(),
                        options(&format!("commit-{index}")),
                    )
                    .await
                    .unwrap();
                assert!(replay.idempotent_replay);
                assert_eq!(
                    receipt_digest(&replay),
                    stable[index],
                    "prior durable receipt changed"
                );
                record(&mut ledger, index, started, clock, &replay);
            }
            read(&client, &session, estate.at, cycle + 1).await;
            ledger.readback.clear();
            for (index, expected) in ledger.expected.iter().take(cycle + 1).enumerate() {
                let values = observed
                    .iter()
                    .filter(|(key, _)| key.starts_with(&format!("claim:reopen-{index}-")))
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<BTreeMap<_, _>>();
                ledger.readback.push(Readback::Present {
                    logical_id: expected.logical_id.clone(),
                    request_sha256: expected.request_sha256.clone(),
                    effect_sha256: digest("c07c.reopen.effect", &values).unwrap(),
                    receipt_sha256: stable[index].clone(),
                    applications: 1.try_into().unwrap(),
                });
            }
            cycles.push(json!({
                "cycle": cycle,
                "prior": previous,
                "termination": terminated,
                "reopened": identity,
                "checked_operations": cycle + 1,
                "runtime_cursor": (cycle + 1) * 2 + 2,
            }));
            previous = identity;
            eprintln!("installed_reopen_progress mode={mode} cycles={}", cycle + 1);
        }
        let cleanup = kill(child, clock);
        assert_eq!(cleanup["pid"], previous["pid"]);
        cleanup
    });
    let report = ledger
        .reconcile(10_000, CORPUS_SECONDS * 1_000_000_000)
        .unwrap();
    assert_eq!(cycles.len(), CYCLES);
    assert_eq!(report.logical_calls, CYCLES as u64);
    assert_eq!(report.attempts, (CYCLES + CYCLES * (CYCLES + 1) / 2) as u64);
    assert_eq!(report.acknowledged, report.attempts);
    assert_eq!(report.resolved_present, CYCLES as u64);
    assert_eq!(report.unresolved, 0);
    assert!(report.violations.is_empty(), "{report:?}");
    if mode == "existing" {
        assert_eq!(
            std::fs::read_to_string(estate.project.join("README.md")).unwrap(),
            "project-owned\n"
        );
    }
    let receipt = serde_json::to_vec(&json!({"instrumentation":"none","mode":mode,"build":estate.build,
        "native_os":std::env::consts::OS,"native_arch":std::env::consts::ARCH,"forced_reopens":CYCLES,
        "installation_plan_sha256":estate.plan["plan_sha256"],"expected_sha256":expected_sha256,
        "profile_id":estate.plan["profile_id"],"deployment":estate.plan["deployment"],"configuration":estate.plan["configuration"],
        "cycles":cycles,"cleanup":cleanup,"ledger":ledger,"report":report})).unwrap();
    assert!(receipt.len() <= 8 * STREAM_LIMIT);
    assert!(!String::from_utf8_lossy(&receipt).contains(credential.credential()));
    if let Some(directory) = std::env::var_os("RRFLOW_REOPEN_RECEIPT_DIR") {
        use std::io::Write;
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!("{mode}.json")))
            .unwrap();
        file.write_all(&receipt).unwrap();
        file.sync_all().unwrap();
        println!(
            "installed_reopen_summary={}",
            json!({"mode":mode,"build":estate.build,"forced_reopens":CYCLES,"report":report})
        );
    } else {
        println!(
            "installed_reopen_receipt={}",
            String::from_utf8(receipt).unwrap()
        );
    }
}

#[test]
#[ignore = "100 real ordinary-primary reopens; explicitly required by native candidate job"]
fn fresh_installed_100_acknowledged_reopens() {
    run("fresh");
}

#[test]
#[ignore = "100 real ordinary-primary reopens; explicitly required by native candidate job"]
fn existing_installed_100_acknowledged_reopens() {
    run("existing");
}
