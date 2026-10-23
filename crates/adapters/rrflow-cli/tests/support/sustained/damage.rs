//! Ordinary installed denial and exact recovery of fixture-owned damaged bytes.
//! Restoring saved bytes is an explicit test action, never product salvage.
use super::*;
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

fn inventory(root: &Path) -> BTreeMap<String, (u64, String)> {
    let mut pending = vec![(root.to_owned(), 0u8)];
    let mut result = BTreeMap::new();
    let mut entries = 0u64;
    let mut bytes = 0u64;
    let mut path_bytes = 0u64;
    while let Some((directory, depth)) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            entries += 1;
            assert!(entries <= 4096, "bounded damage inventory entries");
            path_bytes = path_bytes
                .checked_add(path.as_os_str().len() as u64 * 4 + 512)
                .unwrap();
            assert!(
                path_bytes <= 16 * 1024 * 1024,
                "bounded damage path metadata"
            );
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            assert!(
                !metadata.file_type().is_symlink(),
                "owned fixture cannot contain aliases"
            );
            if metadata.is_dir() {
                assert!(depth < 8, "bounded damage inventory depth");
                pending.push((path, depth + 1));
                continue;
            }
            assert!(metadata.is_file());
            bytes = bytes.checked_add(metadata.len()).unwrap();
            assert!(bytes <= 128 * 1024 * 1024, "bounded damage inventory bytes");
            let mut input = File::open(&path).unwrap();
            let mut hash = Sha256::new();
            let mut buffer = [0u8; 64 * 1024];
            let mut read = 0u64;
            loop {
                let count = input.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                read += count as u64;
                assert!(read <= metadata.len(), "fixture changed during inventory");
                hash.update(&buffer[..count]);
            }
            assert_eq!(read, metadata.len());
            let identity = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
            result.insert(
                path.strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
                (read, identity),
            );
        }
    }
    result
}

fn require_denied(estate: &Estate, serve: bool) -> serde_json::Value {
    let mut command = estate.command();
    command
        .args([
            "--json",
            if serve { "serve" } else { "verify" },
            "--project",
        ])
        .arg(&estate.project);
    if serve {
        command.args(["--bind", "127.0.0.1:0"]);
    } else {
        command.args(["--level", "full"]);
    }
    let mut child = super::super::process::Process::start(
        &mut command,
        Duration::from_secs(60),
        super::super::STREAM_LIMIT,
        None,
    )
    .unwrap();
    let output = child
        .finish()
        .expect("damaged installed command must terminate within its deadline");
    assert!(
        !output.status.success(),
        "damaged authoritative bytes were accepted"
    );
    assert!(
        !output
            .stdout
            .windows(b"listening".len())
            .any(|v| v == b"listening"),
        "damaged engine published readiness"
    );
    serde_json::json!({"exit_status":output.status.to_string(),
        "stdout_bytes":output.stdout.len(),"stderr_bytes":output.stderr.len()})
}

fn target(estate: &Estate, role: &str) -> PathBuf {
    let locator = rrd_engine::RrdEngine::read_project_locator(&estate.project).unwrap();
    let root = estate.project.join(locator.storage_root);
    let inspector = rrd_store::RrflowKvInspector::open(&root).unwrap();
    let physical = &inspector.inspection().physical;
    let relative = match role {
        "current" => "CURRENT".to_owned(),
        "manifest" => format!("manifests/{}.json", physical.current_manifest_sha256),
        "segment" => format!(
            "segments/{}.seg",
            physical
                .reachable_segments
                .first()
                .expect("configured native writes must publish a reachable segment")
        ),
        "wal" => physical
            .files
            .iter()
            .find(|file| file.relative_path.ends_with(".wal"))
            .expect("active WAL inventory")
            .relative_path
            .clone(),
        _ => panic!("closed damage role"),
    };
    let path = root.join(relative);
    assert!(std::fs::canonicalize(&path)
        .unwrap()
        .starts_with(std::fs::canonicalize(&estate.project).unwrap()));
    assert!(path.is_file() && std::fs::metadata(&path).unwrap().len() > 0);
    path
}

async fn restore_backup(
    estate: &Estate,
    work: &mut Workload,
    backup: &BackupEvidence,
    originals: &[(Prepared, Digest)],
    clock: Instant,
) -> serde_json::Value {
    let (child, client, _) = estate.start_for(clock, Duration::from_secs(180));
    work.client = client;
    let backup_sha = &backup.receipt.backup.backup_sha256;
    let staged = work
        .client
        .restore_backup(
            &work.session,
            rrd_contract::RestoreInstanceBackup {
                backup_sha256: backup_sha.clone(),
                restore_id: rrd_contract::CanonicalId::new("damage-roundtrip").unwrap(),
                restored_at_unix_ms: estate.at + 3000,
            },
            RequestOptions::mutation("damage-restore", "damage-restore", "damage-restore").unwrap(),
        )
        .await
        .unwrap();
    super::super::kill(child, clock);
    let before = inventory(&estate.project);
    let arguments = [
        "restore",
        "plan",
        "--project",
        estate.project.to_str().unwrap(),
        "--restore",
        "damage-roundtrip",
        "--backup",
        backup_sha,
    ];
    let plan = estate.invoke(&arguments);
    assert_eq!(
        estate.invoke(&arguments),
        plan,
        "restore plan must be stable"
    );
    assert_eq!(
        inventory(&estate.project),
        before,
        "planning changed project state"
    );
    let path = estate.temp.path().join("damage-restore-plan.json");
    std::fs::write(&path, serde_json::to_vec(&plan).unwrap()).unwrap();
    let published = estate.invoke(&[
        "--json",
        "restore",
        "apply",
        "--project",
        estate.project.to_str().unwrap(),
        "--plan",
        path.to_str().unwrap(),
        "--expect",
        plan["plan_sha256"].as_str().unwrap(),
    ]);
    assert_eq!(published["plan_sha256"], plan["plan_sha256"]);
    assert_eq!(published["runtime_cursor"], staged.inventory.runtime_cursor);
    let quarantine = estate
        .project
        .join(published["quarantine_root"].as_str().unwrap());
    assert!(quarantine.is_dir());
    assert!(std::fs::canonicalize(quarantine)
        .unwrap()
        .starts_with(std::fs::canonicalize(&estate.project).unwrap()));
    let (child, client, _) = estate.start_for(clock, Duration::from_secs(180));
    work.client = client;
    // Application backups exclude transient sessions and their operation
    // receipts. Restoring one must not resurrect old session authorization.
    let stale = work
        .client
        .read_storage_diagnostics(
            &work.session,
            ReadStorageDiagnostics {
                scope: format!("instance:{}", work.client.instance_id()),
            },
            RequestOptions::read("damage-stale-session", "damage-stale-session").unwrap(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(stale, rrd_client::Error::Api { error, .. }
            if error.code == rrd_contract::ErrorCode::NotFound),
        "stale session must receive an API denial"
    );
    let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project).unwrap();
    work.session = work
        .client
        .create_session(
            credential.principal_id.clone(),
            credential.credential(),
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 600_000,
                    absolute_timeout_ms: 3_600_000,
                    max_open_transactions: 4,
                },
            },
            RequestOptions::mutation(
                "damage-restored-session",
                "damage-restored-session",
                "damage-restored-session",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    for (operation, _) in originals.iter().take(work.records as usize) {
        work.verify(operation).await.unwrap();
    }
    work.verify_absent(&originals.last().unwrap().0.key)
        .await
        .unwrap();
    let subsequent = work
        .prepare_load(
            "damage-post-restore".into(),
            WriteTransition {
                slot: 1,
                generation: 0,
            },
        )
        .unwrap();
    let receipt = work.execute(&subsequent).await.unwrap();
    assert!(!receipt.idempotent_replay);
    work.verify(&subsequent).await.unwrap();
    super::super::kill(child, clock);
    let (child, client, _) = estate.start_for(clock, Duration::from_secs(180));
    work.client = client;
    for (operation, _) in originals.iter().take(work.records as usize) {
        work.verify(operation).await.unwrap();
    }
    work.verify_absent(&originals.last().unwrap().0.key)
        .await
        .unwrap();
    work.verify(&subsequent).await.unwrap();
    let replay = work.execute(&subsequent).await.unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.receipt_sha256, receipt.receipt_sha256);
    super::super::kill(child, clock);
    serde_json::json!({"backup_sha256":backup_sha,"restore_plan_sha256":plan["plan_sha256"],
        "restored_runtime_cursor":staged.inventory.runtime_cursor,"verified_records":work.records,
        "verified_relationships":work.records * u64::from(work.edges_per_record),
        "verified_vectors":work.records * u64::from(work.vectors_per_record),
        "later_batch_absent":true,"old_session_denied":true,
        "new_work_receipt_sha256":receipt.receipt_sha256,"new_work_replayed_after_restart":true})
}

fn run(mode: &str) {
    // An application backup is a read-only logical snapshot. Publish native
    // segments through ordinary writes under an explicit installed policy,
    // rather than depending on an unrelated backup flush side effect.
    let configuration = tempfile::Builder::new()
        .prefix("rrflow damage policy ")
        .tempdir()
        .unwrap();
    let path = configuration.path().join("configuration.toml");
    let mut policy: toml::Value = toml::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../authority/rrd-engine/assets/install/default-estate-configuration-v3.toml"
    )))
    .unwrap();
    policy["storage"]["wal_flush_target_bytes"] = toml::Value::Integer(1);
    std::fs::write(&path, toml::to_string(&policy).unwrap()).unwrap();
    let estate = Estate::install_configured(mode, Some(&path));
    assert_eq!(
        estate.plan["configuration"]["storage"]["wal_flush_target_bytes"],
        1
    );
    let clock = Instant::now();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (child, client, _) = estate.start_for(clock, Duration::from_secs(180));
        let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project).unwrap();
        let session = client
            .create_session(
                credential.principal_id.clone(),
                credential.credential(),
                CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 600_000,
                        absolute_timeout_ms: 3_600_000,
                        max_open_transactions: 4,
                    },
                },
                RequestOptions::mutation("damage-session", "damage-session", "damage-session")
                    .unwrap(),
            )
            .await
            .unwrap();
        let mut work = Workload {
            client,
            session,
            valid_at: estate.at,
            dimensions: 16,
            seed: 97,
            records: 4,
            read_distribution: ReadKeyDistribution::Uniform {},
            edges_per_record: 2,
            vectors_per_record: 2,
            payload_bytes: 1024,
            query_budget: serde_json::from_value(estate.plan["configuration"]["query"].clone())
                .unwrap(),
        };
        work.initialize().await.unwrap();
        let mut originals = Vec::new();
        for ordinal in 0..work.records {
            let operation = work.seed_record(ordinal).unwrap();
            let receipt = work.execute(&operation).await.unwrap();
            assert!(!receipt.idempotent_replay);
            work.verify(&operation).await.unwrap();
            originals.push((operation, receipt.receipt_sha256));
        }
        let backup = lifecycle::backup(work.clone(), "damage-baseline-backup".into(), clock)
            .await
            .unwrap();
        let tail = work
            .prepare_load(
                "damage-tail".into(),
                WriteTransition {
                    slot: 0,
                    generation: 0,
                },
            )
            .unwrap();
        let receipt = work.execute(&tail).await.unwrap();
        assert!(!receipt.idempotent_replay);
        work.verify(&tail).await.unwrap();
        originals.push((tail, receipt.receipt_sha256));
        super::super::kill(child, clock);
        let mut cases = Vec::new();
        for role in ["wal", "segment", "manifest", "current"] {
            for fault in ["missing", "corrupt"] {
                let before = inventory(&estate.project);
                let path = target(&estate, role);
                let saved = estate.temp.path().join(format!("saved-{role}-{fault}"));
                if fault == "missing" {
                    std::fs::rename(&path, &saved).unwrap();
                } else {
                    std::fs::copy(&path, &saved).unwrap();
                    let mut file = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&path)
                        .unwrap();
                    let offset = file.metadata().unwrap().len() / 2;
                    file.seek(SeekFrom::Start(offset)).unwrap();
                    let mut byte = [0u8; 1];
                    file.read_exact(&mut byte).unwrap();
                    byte[0] ^= 0x80;
                    file.seek(SeekFrom::Start(offset)).unwrap();
                    file.write_all(&byte).unwrap();
                    file.sync_all().unwrap();
                }
                let damaged = inventory(&estate.project);
                assert_ne!(
                    before, damaged,
                    "fixture must change actual authoritative bytes"
                );
                let verify = require_denied(&estate, false);
                assert_eq!(
                    inventory(&estate.project),
                    damaged,
                    "verify changed damage evidence"
                );
                let serve = require_denied(&estate, true);
                assert_eq!(
                    inventory(&estate.project),
                    damaged,
                    "open changed damage evidence"
                );
                // Only the fixture restores exact saved bytes, after both denials.
                if fault == "corrupt" {
                    std::fs::remove_file(&path).unwrap();
                }
                std::fs::rename(&saved, &path).unwrap();
                assert_eq!(inventory(&estate.project), before, "restored bytes differ");
                let (reopened, client, _) = estate.start_for(clock, Duration::from_secs(180));
                work.client = client;
                for (operation, _) in &originals {
                    work.verify(operation).await.unwrap();
                }
                for (operation, expected) in &originals {
                    let replay = work.execute(operation).await.unwrap();
                    assert!(replay.idempotent_replay);
                    assert_eq!(&replay.receipt_sha256, expected);
                }
                super::super::kill(reopened, clock);
                cases.push(
                    serde_json::json!({"role":role,"fault":fault,"verify":verify,"serve":serve,
                    "damaged_inventory_sha256":digest("c07f.damage-inventory", &damaged).unwrap(),
                    "restored_inventory_sha256":digest("c07f.damage-inventory", &before).unwrap(),
                    "readback_operations":originals.len(),"exact_replays":originals.len()}),
                );
            }
        }
        assert_eq!(cases.len(), 8);
        let restored = restore_backup(&estate, &mut work, &backup, &originals, clock).await;
        println!(
            "installed_damage_receipt={}",
            serde_json::json!({"mode":mode,"build":estate.build,
            "backup_sha256":backup.receipt.backup.backup_sha256,"cases":cases,
            "restore":restored,"implicit_repair":false,"capacity_qualification":false})
        );
    });
}

#[test]
#[ignore = "ordinary installed native damage matrix; explicit candidate CI"]
fn fresh_installed_damage_preserves_evidence_and_acknowledged_state() {
    run("fresh");
}

#[test]
#[ignore = "ordinary installed native damage matrix; explicit candidate CI"]
fn existing_installed_damage_preserves_evidence_and_acknowledged_state() {
    run("existing");
}
