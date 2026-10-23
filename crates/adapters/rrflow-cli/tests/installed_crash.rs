//! Explicit diagnostic-primary proof. Never a default distribution artifact.
#[path = "support/capacity.rs"]
mod capacity;
#[path = "support/process.rs"]
mod process;

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
    path::PathBuf,
    process::Command,
    sync::mpsc,
    time::{Duration, Instant},
};

const LIMIT: usize = 1024 * 1024;
const PREFIX: &str = "C07C_NATIVE_PAUSE_V1=";

struct Estate {
    temp: tempfile::TempDir,
    _volume_project: Option<tempfile::TempDir>,
    working: PathBuf,
    binary: PathBuf,
    project: PathBuf,
    identity: String,
    at: u64,
    build: Value,
}

impl Estate {
    fn command(&self) -> Command {
        let mut command = Command::new(&self.binary);
        command.env_clear().current_dir(&self.working);
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        for name in ["HOME", "USERPROFILE", "TMP", "TEMP", "TMPDIR", "PATH"] {
            command.env(name, &self.working);
        }
        command
    }

    fn invoke(&self, args: &[&str]) -> Value {
        let mut command = self.command();
        let mut child =
            process::Process::start(command.args(args), Duration::from_secs(120), LIMIT, None)
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
        Self::install_at(mode, None, false)
    }

    fn install_at(mode: &str, volume: Option<&std::path::Path>, flush_each: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join(if cfg!(windows) {
            "rrflow.exe"
        } else {
            "rrflow"
        });
        process::stage_executable(std::path::Path::new(env!("CARGO_BIN_EXE_rrflow")), &binary)
            .unwrap();
        let volume_project = volume.map(|root| {
            tempfile::Builder::new()
                .prefix("rrflow-estate-")
                .tempdir_in(root)
                .unwrap()
        });
        let estate_root = volume_project
            .as_ref()
            .map_or(temp.path(), |owner| owner.path());
        let project = estate_root.join("project");
        let working = estate_root.join("working");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&working).unwrap();
        if mode == "existing" {
            std::fs::write(project.join("README.md"), "owned project\n").unwrap();
        }
        let mut estate = Self {
            temp,
            _volume_project: volume_project,
            working,
            binary,
            project,
            identity: String::new(),
            at: 0,
            build: Value::Null,
        };
        estate.build = estate.invoke(&["--json", "version"])["build"].clone();
        estate.identity = estate.invoke(&["install", "identity"])["installation_identity"]
            .as_str()
            .unwrap()
            .to_owned();
        let configuration = estate.temp.path().join("configuration.toml");
        let mut policy = include_str!(
            "../../../authority/rrd-engine/assets/install/default-estate-configuration-v3.toml"
        )
        .to_owned();
        if flush_each {
            policy = policy.replace(
                "wal_flush_target_bytes = 67108864",
                "wal_flush_target_bytes = 1",
            );
        }
        std::fs::write(&configuration, policy).unwrap();
        let preview = estate.invoke(&[
            "install",
            "plan",
            "--identity",
            &estate.identity,
            "--mode",
            mode,
            "--project",
            estate.project.to_str().unwrap(),
            "--configuration",
            configuration.to_str().unwrap(),
        ]);
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
        estate.at = applied["applied_at_unix_ms"].as_u64().unwrap();
        assert_eq!(applied["runtime_cursor"], 2);
        estate
    }

    fn control(&self, point: &str, id: u16, operation: Option<&str>) -> Value {
        let nonce = Sha256::digest(format!("{}:{id}:{point}", self.identity))
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        json!({"version":1,"nonce":nonce,
            "id":id,"point":point,"operation_sha256":operation})
    }

    fn launch(&self, control: &Value) -> Child {
        let mut command = self.command();
        command
            .args(["--json", "serve", "--project"])
            .arg(&self.project)
            .args(["--bind", "127.0.0.1:0"]);
        let (send, lines) = mpsc::sync_channel(4);
        let input = serde_json::to_vec(control).unwrap();
        let child = if control["resume"] == true {
            process::Process::start_controlled(
                &mut command,
                Duration::from_secs(180),
                LIMIT,
                send,
                &input,
            )
        } else {
            process::Process::start_observed(
                &mut command,
                Duration::from_secs(180),
                LIMIT,
                send,
                &input,
            )
        }
        .unwrap();
        Child { child, lines }
    }

    fn serve(&self, control: &Value) -> (Child, RrdClient) {
        let child = self.launch(control);
        let announcement: Value = serde_json::from_slice(&child.line()).unwrap();
        assert_eq!(announcement["status"], "listening");
        assert_eq!(announcement["build"], self.build);
        let locator = RrdEngine::read_project_locator(&self.project).unwrap();
        let client = RrdClient::connect_local(
            announcement["url"]
                .as_str()
                .unwrap()
                .trim_start_matches("http://")
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
        (child, client)
    }
}

struct Child {
    child: process::Process,
    lines: mpsc::Receiver<std::io::Result<Vec<u8>>>,
}
impl Child {
    fn line(&self) -> Vec<u8> {
        self.lines
            .recv_timeout(Duration::from_secs(60))
            .expect("bounded installed diagnostic event")
            .unwrap()
    }
    fn kill(mut self) {
        assert!(self.child.running());
        self.child.terminate();
        assert!(!self.child.finish().unwrap().status.success());
    }
    fn hit_and_kill(self, control: Value) -> Value {
        let started = Instant::now();
        let line = self.line();
        let line = std::str::from_utf8(&line).unwrap().trim();
        let hit: Value = serde_json::from_str(
            line.strip_prefix(PREFIX)
                .expect("exact installed boundary must be reached"),
        )
        .unwrap();
        assert_eq!(hit["pid"], self.child.id());
        for key in ["nonce", "id", "point"] {
            assert_eq!(hit[key], control[key], "wrong diagnostic {key}");
        }
        let hit_ns = started.elapsed().as_nanos() as u64;
        self.kill();
        json!({"hit":hit,"hit_ns":hit_ns,"reaped_ns":started.elapsed().as_nanos() as u64})
    }
}

fn options(name: &str) -> RequestOptions {
    RequestOptions::new(name, name, Some(name), Some(u64::MAX)).unwrap()
}
fn request(name: &str, at: u64) -> CommitTransaction {
    let mutations = (0..2)
        .map(|i| TransactionMutation::AssertClaim {
            subject: CanonicalId::new(format!("crash-{name}-{i}")).unwrap(),
            predicate: CanonicalId::new("crash").unwrap(),
            object: format!("expected-{name}-{i}"),
            valid_from: at,
            tx_time: at,
            producer: CanonicalId::new("independent-oracle").unwrap(),
            confidence: None,
        })
        .collect::<Vec<_>>();
    CommitTransaction {
        operation_sha256: transaction_operation_sha256(&mutations),
        mutations,
    }
}
fn expected(name: &str) -> BTreeMap<String, String> {
    (0..2)
        .map(|i| {
            (
                format!("claim:crash-{name}-{i}:crash"),
                format!("expected-{name}-{i}"),
            )
        })
        .collect()
}
fn receipt_hash(receipt: &CommitReceipt) -> rrflow_eval::storage::Digest {
    let mut receipt = receipt.clone();
    receipt.idempotent_replay = false;
    digest("c07c.installed.receipt", &receipt).unwrap()
}
async fn read(
    client: &RrdClient,
    session: &rrd_client::Session,
    at: u64,
    values: &BTreeMap<String, String>,
) {
    let result = client.execute_query(session, ExecuteQuery { scope: format!("instance:{}",client.instance_id()),
        query:format!("FROM claim:crash AT VALID {at} KNOWN HEAD PROJECT subject, object EXPLAIN CONTRACT"),
        parameters:BTreeMap::new(), budget: QueryBudget {max_storage_keys:75_000,max_rows:768,max_output_bytes:393_216,
            max_batch_rows:192,max_memory_bytes:33_554_432,max_spill_bytes:67_108_864,max_elapsed_ms:20_000}},
        RequestOptions::read("crash-read","crash-read").unwrap()).await.unwrap();
    assert!(!result.execution.truncated);
    assert_eq!(result.known_at_cursor, values.len() as u64 + 2);
    let mut observed = BTreeMap::new();
    for row in result.rows {
        let QueryValue::String(value) = &row.values["object"] else {
            panic!("unexpected value type")
        };
        assert!(observed.insert(row.identity, value.clone()).is_none());
    }
    assert_eq!(&observed, values, "independent atomic history differs");
}

fn run(mode: &str, points: &[&str]) {
    let estate = Estate::install(mode);
    let credential = RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut hits = Vec::new();
    let mut ledger = AcknowledgementLedger::default();
    let clock = Instant::now();
    runtime.block_on(async {
        let (child, client) = estate.serve(&Value::Null);
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
                options("crash-session"),
            )
            .await
            .unwrap();
        let mut values = BTreeMap::new();
        let baseline = request("baseline", estate.at);
        ledger.expected.push(ExpectedOperation {
            logical_id: "baseline".to_owned().try_into().unwrap(),
            request_sha256: digest("c07c.installed.request", &baseline).unwrap(),
            effect_sha256: digest("c07c.installed.effect", &expected("baseline")).unwrap(),
            resolution: ExpectedResolution::Present,
        });
        let lease = client
            .begin_transaction(
                &session,
                BeginTransaction {
                    scope: CanonicalId::new("claims").unwrap(),
                    timeout_ms: 600_000,
                },
                options("baseline-begin"),
            )
            .await
            .unwrap();
        let baseline_started = clock.elapsed().as_nanos() as u64;
        let acknowledged = client
            .commit_transaction(
                &session,
                &lease.transaction_id,
                baseline.clone(),
                options("baseline-commit"),
            )
            .await
            .unwrap();
        assert_eq!(acknowledged.last_runtime_cursor, Some(4));
        let baseline_lease = lease;
        let baseline_hash = receipt_hash(&acknowledged);
        ledger.attempts.push(Attempt {
            attempt_id: "baseline-ack".to_owned().try_into().unwrap(),
            logical_id: "baseline".to_owned().try_into().unwrap(),
            request_sha256: digest("c07c.installed.request", &baseline).unwrap(),
            started_ns: baseline_started,
            finished_ns: clock.elapsed().as_nanos() as u64,
            outcome: AttemptOutcome::Acknowledged {
                receipt_sha256: baseline_hash.clone(),
            },
        });
        ledger.readback.push(Readback::Present {
            logical_id: "baseline".to_owned().try_into().unwrap(),
            request_sha256: digest("c07c.installed.request", &baseline).unwrap(),
            effect_sha256: digest("c07c.installed.effect", &expected("baseline")).unwrap(),
            receipt_sha256: baseline_hash.clone(),
            applications: 1.try_into().unwrap(),
        });
        values.extend(expected("baseline"));
        child.kill();
        for (index, point) in points.iter().enumerate() {
            eprintln!("installed_domain_case mode={mode} point={point}");
            let name = format!("case-{index}");
            let request = request(&name, estate.at);
            let logical_id = name.clone().try_into().unwrap();
            ledger.expected.push(ExpectedOperation {
                logical_id,
                request_sha256: digest("c07c.installed.request", &request).unwrap(),
                effect_sha256: digest("c07c.installed.effect", &expected(&name)).unwrap(),
                resolution: ExpectedResolution::Present,
            });
            let control = estate.control(point, index as u16 + 1, Some(&request.operation_sha256));
            let (child, client) = estate.serve(&control);
            let lease = client
                .begin_transaction(
                    &session,
                    BeginTransaction {
                        scope: CanonicalId::new("claims").unwrap(),
                        timeout_ms: 600_000,
                    },
                    options(&format!("begin-{name}")),
                )
                .await
                .unwrap();
            let commit_options = options(&format!("commit-{name}"));
            let started = clock.elapsed().as_nanos() as u64;
            let kill = tokio::task::spawn_blocking(move || child.hit_and_kill(control));
            let lost = client
                .commit_transaction(
                    &session,
                    &lease.transaction_id,
                    request.clone(),
                    commit_options.clone(),
                )
                .await;
            hits.push(kill.await.unwrap());
            assert!(
                matches!(lost, Err(rrd_client::Error::Transport(_))),
                "interrupted response must be uncertain: {lost:?}"
            );
            ledger.attempts.push(Attempt {
                attempt_id: format!("uncertain-{index}").try_into().unwrap(),
                logical_id: name.clone().try_into().unwrap(),
                request_sha256: digest("c07c.installed.request", &request).unwrap(),
                started_ns: started,
                finished_ns: clock.elapsed().as_nanos() as u64,
                outcome: AttemptOutcome::Uncertain {
                    reason: "process-killed".to_owned().try_into().unwrap(),
                },
            });
            let recovery = estate.control("recovery_read", index as u16 + 1, None);
            hits.push(estate.launch(&recovery).hit_and_kill(recovery));
            let (child, client) = estate.serve(&Value::Null);
            if *point != "wal_before_write" {
                values.extend(expected(&name));
            }
            read(&client, &session, estate.at, &values).await;
            let started = clock.elapsed().as_nanos() as u64;
            let prior = client
                .commit_transaction(
                    &session,
                    &baseline_lease.transaction_id,
                    baseline.clone(),
                    options("baseline-commit"),
                )
                .await
                .unwrap();
            assert!(prior.idempotent_replay);
            assert_eq!(
                receipt_hash(&prior),
                baseline_hash,
                "lost or changed prior ACK"
            );
            ledger.attempts.push(Attempt {
                attempt_id: format!("baseline-retry-{index}").try_into().unwrap(),
                logical_id: "baseline".to_owned().try_into().unwrap(),
                request_sha256: digest("c07c.installed.request", &baseline).unwrap(),
                started_ns: started,
                finished_ns: clock.elapsed().as_nanos() as u64,
                outcome: AttemptOutcome::Acknowledged {
                    receipt_sha256: baseline_hash.clone(),
                },
            });
            let mut stable = None;
            for retry in 0..2 {
                let started = clock.elapsed().as_nanos() as u64;
                let receipt = client
                    .commit_transaction(
                        &session,
                        &lease.transaction_id,
                        request.clone(),
                        commit_options.clone(),
                    )
                    .await
                    .unwrap();
                assert_eq!(receipt.mutation_count, 2);
                assert_eq!(
                    receipt.idempotent_replay,
                    retry > 0 || *point != "wal_before_write"
                );
                let hash = receipt_hash(&receipt);
                if let Some(previous) = &stable {
                    assert_eq!(previous, &hash);
                }
                stable = Some(hash.clone());
                ledger.attempts.push(Attempt {
                    attempt_id: format!("retry-{index}-{retry}").try_into().unwrap(),
                    logical_id: name.clone().try_into().unwrap(),
                    request_sha256: digest("c07c.installed.request", &request).unwrap(),
                    started_ns: started,
                    finished_ns: clock.elapsed().as_nanos() as u64,
                    outcome: AttemptOutcome::Acknowledged {
                        receipt_sha256: hash,
                    },
                });
            }
            values.extend(expected(&name));
            read(&client, &session, estate.at, &values).await;
            ledger.readback.push(Readback::Present {
                logical_id: name.try_into().unwrap(),
                request_sha256: digest("c07c.installed.request", &request).unwrap(),
                effect_sha256: digest("c07c.installed.effect", &expected(&format!("case-{index}")))
                    .unwrap(),
                receipt_sha256: stable.unwrap(),
                applications: 1.try_into().unwrap(),
            });
            child.kill();
        }
    });
    let report = ledger.reconcile(32, 1_800_000_000_000).unwrap();
    assert_eq!(report.logical_calls, points.len() as u64 + 1);
    assert_eq!(report.attempts, points.len() as u64 * 4 + 1);
    assert_eq!(report.uncertain, points.len() as u64);
    assert_eq!(hits.len(), points.len() * 2);
    assert_eq!(report.unresolved, 0);
    assert!(report.violations.is_empty(), "{report:?}");
    println!(
        "installed_domain_crash_receipt={}",
        json!({"instrumentation":"crash-test-primary","mode":mode,"build":estate.build,
        "native_os":std::env::consts::OS,"hits":hits,"ledger":ledger,"report":report})
    );
}

#[test]
fn fresh_installed_domain_boundaries() {
    run(
        "fresh",
        &[
            "wal_before_write",
            "wal_appended",
            "wal_synced",
            "visible",
            "receipt_persisted",
        ],
    );
}
#[test]
fn existing_installed_domain_boundaries() {
    run(
        "existing",
        &[
            "wal_before_write",
            "wal_appended",
            "wal_synced",
            "visible",
            "receipt_persisted",
        ],
    );
}

#[test]
fn diagnostic_controls_reject_invalid_schedules_and_ignore_other_operations() {
    let estate = Estate::install("fresh");
    let control = estate.control("wal_synced", 42, Some(&"f".repeat(64)));
    let invalid = [
        ("version", json!(2)),
        ("nonce", json!("invalid")),
        ("id", json!(0)),
        ("point", json!("unreachable")),
        ("operation_sha256", Value::Null),
        ("root", json!("foreign")),
    ];
    for (field, value) in invalid {
        let mut invalid = control.clone();
        invalid[field] = value;
        let mut child = estate.launch(&invalid);
        let output = child.child.finish().unwrap();
        assert!(
            !output.status.success(),
            "invalid control accepted: {field}"
        );
        assert!(
            output.stdout.is_empty(),
            "invalid control reached service or pause"
        );
    }
    let credential = RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let (child, client) = estate.serve(&control);
    tokio::runtime::Builder::new_current_thread()
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
                            idle_timeout_ms: 600_000,
                            absolute_timeout_ms: 3_600_000,
                            max_open_transactions: 2,
                        },
                    },
                    options("negative-session"),
                )
                .await
                .unwrap();
            let lease = client
                .begin_transaction(
                    &session,
                    BeginTransaction {
                        scope: CanonicalId::new("claims").unwrap(),
                        timeout_ms: 600_000,
                    },
                    options("negative-begin"),
                )
                .await
                .unwrap();
            let request = request("nonmatching", estate.at);
            assert_ne!(
                request.operation_sha256,
                control["operation_sha256"].as_str().unwrap()
            );
            let receipt = client
                .commit_transaction(
                    &session,
                    &lease.transaction_id,
                    request,
                    options("negative-commit"),
                )
                .await
                .unwrap();
            assert_eq!(receipt.last_runtime_cursor, Some(4));
            read(&client, &session, estate.at, &expected("nonmatching")).await;
        });
    assert!(
        matches!(child.lines.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "unrelated operation armed a crash"
    );
    child.kill();
    println!(
        "installed_crash_admission_receipt={}",
        json!({"instrumentation":"crash-test-primary",
        "build":estate.build,"invalid_schedules":6,"nonmatching_commit":true})
    );
}
