//! Resume after fill, WAL and metadata pressure, and installed capacity cases.

use super::*;

pub(super) fn resume_after_fill(
    mut child: Child,
    control: Value,
    volume: Volume,
    metadata_directory: Option<PathBuf>,
    wal_root: PathBuf,
) -> (Child, Filler, Value) {
    let line = child.line();
    let hit: Value = serde_json::from_str(
        std::str::from_utf8(&line)
            .unwrap()
            .trim()
            .strip_prefix(PREFIX)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(hit["pid"], child.child.id());
    for key in ["nonce", "id", "point"] {
        assert_eq!(hit[key], control[key]);
    }
    println!("native_capacity_pause={hit}");
    println!(
        "native_capacity_wal_before_fill={}",
        observation::wal_state(&wal_root)
    );
    let filler = Filler::fill(&volume, metadata_directory.as_deref());
    println!(
        "native_capacity_fill={}",
        json!({"point":control["point"],
        "bytes":filler.bytes,"metadata_payload_bytes":filler.metadata_payload_bytes,"native_code":filler.native_code,"data_refills":filler.data_refills,
        "metadata_files":filler.metadata_paths.len(),
        "metadata_native_code":filler.metadata_native_code,"free_inodes":filler.free_inodes})
    );
    println!(
        "native_capacity_wal_after_fill={}",
        observation::wal_state(&wal_root)
    );
    child
        .child
        .resume_controlled(
            &serde_json::to_vec(&json!({"version":1,
        "nonce":control["nonce"],"id":control["id"]}))
            .unwrap(),
        )
        .unwrap();
    let line = match child.lines.recv_timeout(Duration::from_secs(60)) {
        Ok(Ok(line)) => line,
        failure => {
            println!(
                "native_capacity_wal_missing_event={}",
                observation::wal_state(&wal_root)
            );
            child.child.terminate();
            let output = child.child.finish().expect("reap failed capacity engine");
            let tail = |bytes: &[u8]| {
                String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(2048)..]).into_owned()
            };
            panic!("selected native capacity event missing: {failure:?}; exit={}; stdout_tail={}; stderr_tail={}",
                output.status, tail(&output.stdout), tail(&output.stderr));
        }
    };
    let error: Value = serde_json::from_str(
        std::str::from_utf8(&line)
            .unwrap()
            .trim()
            .strip_prefix("C07F_NATIVE_IO_V1=")
            .expect("selected native I/O must actually fail"),
    )
    .unwrap();
    for key in ["pid", "nonce", "id", "point"] {
        assert_eq!(error[key], hit[key]);
    }
    assert!(
        capacity_code(error["native_code"].as_i64().unwrap() as i32),
        "wrong native error: {error}"
    );
    let expected_role = match control["point"].as_str().unwrap() {
        "wal_before_write" => "wal",
        "segment_before_create" => "segment",
        "manifest_before_create" => "manifest",
        _ => "publication",
    };
    assert_eq!(error["role"], expected_role);
    (child, filler, json!({"hit":hit,"error":error}))
}

pub(super) async fn read_values(
    client: &RrdClient,
    session: &rrd_client::Session,
    at: u64,
) -> BTreeMap<String, String> {
    // Enumerate the complete identity set, then fetch bounded object windows at
    // that exact read cursor. Native capacity payloads exceed a single public
    // response; raising the contract's response cap would hide the boundary.
    let query = |text| ExecuteQuery {
        scope: format!("instance:{}", client.instance_id()),
        query: text,
        parameters: BTreeMap::new(),
        budget: QueryBudget {
            max_storage_keys: 100_000,
            max_rows: 1024,
            max_output_bytes: 512 * 1024,
            max_batch_rows: 64,
            max_memory_bytes: 64 * 1024 * 1024,
            max_spill_bytes: 64 * 1024 * 1024,
            max_elapsed_ms: 30_000,
        },
    };
    let index = client
        .execute_query(
            session,
            query(format!(
                "FROM claim:crash AT VALID {at} KNOWN HEAD PROJECT subject EXPLAIN CONTRACT"
            )),
            RequestOptions::read("capacity-index", "capacity-index").unwrap(),
        )
        .await
        .unwrap();
    assert!(!index.execution.truncated);
    let cursor = index.known_at_cursor;
    let mut subjects = index
        .rows
        .iter()
        .map(|row| {
            let QueryValue::String(subject) = &row.values["subject"] else {
                panic!("non-string subject")
            };
            subject.clone()
        })
        .collect::<Vec<_>>();
    subjects.sort();
    subjects.dedup();
    assert_eq!(subjects.len(), index.rows.len(), "duplicate claim subject");
    let mut values = BTreeMap::new();
    for window in subjects.chunks(64) {
        let first = serde_json::to_string(window.first().unwrap()).unwrap();
        let last = serde_json::to_string(window.last().unwrap()).unwrap();
        let result = client.execute_query(session, query(format!(
            "FROM claim:crash AT VALID {at} KNOWN {cursor} WHERE subject >= {first} AND subject <= {last} PROJECT subject, object EXPLAIN CONTRACT")),
            RequestOptions::read("capacity-window","capacity-window").unwrap()).await.unwrap();
        assert!(!result.execution.truncated);
        assert_eq!(result.known_at_cursor, cursor);
        assert_eq!(result.rows.len(), window.len());
        for row in result.rows {
            let QueryValue::String(value) = &row.values["object"] else {
                panic!("non-string claim")
            };
            assert!(values.insert(row.identity, value.clone()).is_none());
        }
    }
    assert_eq!(cursor, values.len() as u64 + 2);
    values
}

pub(super) fn requested_values(request: &CommitTransaction) -> BTreeMap<String, String> {
    request
        .mutations
        .iter()
        .map(|m| {
            let TransactionMutation::AssertClaim {
                subject,
                predicate,
                object,
                ..
            } = m
            else {
                panic!("claim corpus")
            };
            (
                format!(
                    "claim:{}",
                    rrd_core::digest::sha256_hex(
                        format!("rrd-context-claim-v1\0{subject}\0{predicate}").as_bytes()
                    )
                ),
                object.clone(),
            )
        })
        .collect()
}

pub(super) async fn begin(
    client: &RrdClient,
    session: &rrd_client::Session,
    name: &str,
) -> rrd_contract::TransactionLease {
    client
        .begin_transaction(
            session,
            BeginTransaction {
                scope: CanonicalId::new("claims").unwrap(),
                timeout_ms: 600_000,
            },
            options(&format!("{name}-begin")),
        )
        .await
        .unwrap()
}

pub(super) fn wal_bytes(estate: &Estate) -> u64 {
    let locator = RrdEngine::read_project_locator(&estate.project).unwrap();
    let root = estate.project.join(locator.storage_root).join("wal");
    std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .max()
        .unwrap()
}

pub(super) fn metadata_pressure_directory(
    point: &str,
    native_root: &std::path::Path,
    filesystem: &str,
) -> Option<PathBuf> {
    match point {
        // APFS can deny growth of the data filler while another existing file
        // still allocates. Consume metadata pressure outside the WAL directory,
        // where disposable entries must never be mistaken for authoritative WAL.
        "wal_before_write" if filesystem == "apfs" => Some(native_root.to_path_buf()),
        "wal_before_write" => None,
        "segment_before_create" => Some(native_root.join("segments")),
        "manifest_before_create" => Some(native_root.join("manifests")),
        "current_before_create" => Some(native_root.to_path_buf()),
        _ => panic!("unsupported native capacity point"),
    }
}

pub(super) fn apfs_wal_consumes_metadata_pressure_outside_authoritative_wal_directory() {
    let estate = tempfile::tempdir().unwrap();
    let root = estate.path();
    // A failed write to one APFS filler file did not prevent another existing
    // file from allocating. WAL pressure must include the metadata phase too.
    assert_eq!(
        metadata_pressure_directory("wal_before_write", root, "apfs"),
        Some(root.to_path_buf())
    );
    for filesystem in ["ext4", "ntfs"] {
        assert_eq!(
            metadata_pressure_directory("wal_before_write", root, filesystem),
            None
        );
    }
    for filesystem in ["apfs", "ext4", "ntfs"] {
        for (point, directory) in [
            ("segment_before_create", root.join("segments")),
            ("manifest_before_create", root.join("manifests")),
            ("current_before_create", root.to_path_buf()),
        ] {
            assert_eq!(
                metadata_pressure_directory(point, root, filesystem),
                Some(directory)
            );
        }
    }
}

pub(super) fn capacity_case(point: &str, mode: &str) {
    let volume = Volume::admitted();
    let estate = Estate::install_at(mode, Some(&volume.root), point != "wal_before_write");
    let locator = RrdEngine::read_project_locator(&estate.project).unwrap();
    let native_root = estate.project.join(locator.storage_root);
    let metadata_directory = metadata_pressure_directory(point, &native_root, &volume.filesystem);
    let credential = RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let mut ledger = AcknowledgementLedger::default();
    let clock = Instant::now();
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let (child, client) = estate.serve(&Value::Null);
        let session = client.create_session(credential.principal_id.clone(),credential.credential(),
            CreateSession {limits:SessionLimits {idle_timeout_ms:600_000,absolute_timeout_ms:3_600_000,max_open_transactions:2}},
            options("capacity-session")).await.unwrap();
        let mut values = BTreeMap::new();
        let mut acknowledged = Vec::new();
        // Consume the native WAL's initial preallocation with real acknowledged
        // operations. No direct file growth or sparse-file simulation is allowed.
        for index in 0..256 {
            let name = format!("prior-{index}");
            let mut request = request(&name, estate.at);
            for mutation in &mut request.mutations {
                if let TransactionMutation::AssertClaim {object,..} = mutation { *object = format!("{name}:").repeat(400); object.truncate(4096); }
            }
            request.operation_sha256 = transaction_operation_sha256(&request.mutations);
            let expected = requested_values(&request);
            let lease = begin(&client,&session,&name).await;
            let started = clock.elapsed().as_nanos() as u64;
            let receipt = client.commit_transaction(&session,&lease.transaction_id,request.clone(),options(&name)).await.unwrap();
            record(&mut ledger, &name, &request, &expected, &receipt, started, &clock);
            values.extend(expected);
            acknowledged.push((name,request,lease,receipt_hash(&receipt)));
            if point != "wal_before_write" || wal_bytes(&estate) > 2 * 1024 * 1024 { break; }
            assert!(index < 255,"failed to consume initial WAL allocation within corpus bound");
        }
        assert_eq!(read_values(&client,&session,estate.at).await,values);
        child.kill();
        let mutations = (0..180).map(|index| TransactionMutation::AssertClaim {
            subject:CanonicalId::new(format!("crash-uncertain-{index}")).unwrap(),
            predicate:CanonicalId::new("crash").unwrap(), object:format!("{index:04}").repeat(1024),
            valid_from:estate.at,tx_time:estate.at,producer:CanonicalId::new("independent-oracle").unwrap(),confidence:None,
        }).collect::<Vec<_>>();
        let target = CommitTransaction {operation_sha256:transaction_operation_sha256(&mutations),mutations};
        assert!(serde_json::to_vec(&target).unwrap().len() < 1024 * 1024);
        let target_values = requested_values(&target);
        ledger.expected.push(ExpectedOperation {logical_id:"uncertain".to_owned().try_into().unwrap(),
            request_sha256:digest("c07f.request",&target).unwrap(),effect_sha256:digest("c07f.effect",&target_values).unwrap(),
            resolution:ExpectedResolution::Present});
        let mut control = estate.control(point,1,Some(&target.operation_sha256));
        control["resume"] = json!(true);
        let (child,client) = estate.serve(&control);
        let lease = begin(&client,&session,"uncertain").await;
        let started = clock.elapsed().as_nanos() as u64;
        let controller = tokio::task::spawn_blocking(move ||resume_after_fill(child,control,volume,metadata_directory,native_root.join("wal")));
        let result = client.commit_transaction(&session,&lease.transaction_id,target.clone(),options("uncertain")).await;
        let result_class = match &result {
            Ok(_) => "acknowledged".to_owned(),
            Err(rrd_client::Error::Api { error, .. }) => format!("api-{:?}",error.code),
            Err(rrd_client::Error::Timeout) => "client-timeout".to_owned(),
            Err(_) => "client-failure".to_owned(),
        };
        let message = match &result {
            Err(rrd_client::Error::Api {error,..}) => Some(error.message.chars().take(512).collect::<String>()),
            _ => None,
        };
        println!("native_capacity_client={}",json!({"point":point,"mode":mode,"class":result_class,"message":message}));
        let (child,filler,native) = controller.await.unwrap();
        assert!(result.is_err(),"capacity operation unexpectedly acknowledged");
        child.kill();
        let fill = json!({"bytes":filler.bytes,"metadata_payload_bytes":filler.metadata_payload_bytes,"native_code":filler.native_code,"data_refills":filler.data_refills,
            "metadata_files":filler.metadata_paths.len(),"metadata_native_code":filler.metadata_native_code,"free_inodes":filler.free_inodes});
        drop(filler);
        ledger.attempts.push(Attempt {attempt_id:"capacity-uncertain".to_owned().try_into().unwrap(),
            logical_id:"uncertain".to_owned().try_into().unwrap(),request_sha256:digest("c07f.request",&target).unwrap(),
            started_ns:started,finished_ns:clock.elapsed().as_nanos() as u64,
            outcome:AttemptOutcome::Uncertain {reason:"native-capacity-error".to_owned().try_into().unwrap()}});
        // Ordinary verify cannot silently repair. Only a positively classified
        // tail yields a plan; any other failure remains fatal on serve/re-read.
        let mut command = estate.command();
        command.args(["repair","plan","--project"]).arg(&estate.project);
        let output = installed_command(&mut command,CommandPhase::RepairPlan,Duration::from_secs(120),8*LIMIT)
            .unwrap_or_else(|error| panic!("{error}"));
        let repair_receipt = if output.status.success() {
            let plan:Value = serde_json::from_slice(&output.stdout).unwrap();
            let path = estate.temp.path().join("repair.json");
            std::fs::write(&path,&output.stdout).unwrap();
            Some(estate.invoke(&["repair","apply","--project",estate.project.to_str().unwrap(),
                "--plan",path.to_str().unwrap(),"--expect",plan["plan_sha256"].as_str().unwrap()]))
        } else { None };
        let (child,client) = estate.serve(&Value::Null);
        let after = read_values(&client,&session,estate.at).await;
        let mut with_target = values.clone(); with_target.extend(target_values.clone());
        assert!(after == values || after == with_target,"prior ACK lost or non-atomic target after capacity recovery");
        // Resolve the unknown attempt via the original request identity, then
        // prove receipt stability for every prior acknowledged logical call.
        let retry_started = clock.elapsed().as_nanos() as u64;
        let receipt = client.commit_transaction(&session,&lease.transaction_id,target.clone(),options("uncertain")).await.unwrap();
        record(&mut ledger,"uncertain",&target,&target_values,&receipt,retry_started,&clock);
        values = with_target;
        for (name,request,lease,hash) in acknowledged {
            let receipt = client.commit_transaction(&session,&lease.transaction_id,request,options(&name)).await.unwrap();
            assert!(receipt.idempotent_replay); assert_eq!(receipt_hash(&receipt),hash);
        }
        let last = request("after",estate.at);
        let lease = begin(&client,&session,"after").await;
        let started = clock.elapsed().as_nanos() as u64;
        let receipt = client.commit_transaction(&session,&lease.transaction_id,last.clone(),options("after")).await.unwrap();
        record(&mut ledger,"after",&last,&expected("after"),&receipt,started,&clock);
        values.extend(expected("after"));
        assert_eq!(read_values(&client,&session,estate.at).await,values);
        child.kill();
        let (child,client) = estate.serve(&Value::Null);
        assert_eq!(read_values(&client,&session,estate.at).await,values);
        child.kill();
        let report = ledger.reconcile(1024, 1_800_000_000_000).unwrap();
        assert!(report.violations.is_empty());
        println!("\nnative_capacity_receipt={}",json!({"version":1,"mode":mode,"point":point,
            "build":estate.build,"native":native,"filler":fill,"repair":repair_receipt,
            "expected_sha256":digest("c07f.values",&values).unwrap(),"ledger":ledger,"report":report}));
    });
}

pub(super) fn record(
    ledger: &mut AcknowledgementLedger,
    name: &str,
    request: &CommitTransaction,
    values: &BTreeMap<String, String>,
    receipt: &CommitReceipt,
    started: u64,
    clock: &Instant,
) {
    let logical_id = name.to_owned().try_into().unwrap();
    let request_sha256 = digest("c07f.request", request).unwrap();
    let effect_sha256 = digest("c07f.effect", values).unwrap();
    if !ledger
        .expected
        .iter()
        .any(|item| item.logical_id == logical_id)
    {
        ledger.expected.push(ExpectedOperation {
            logical_id,
            request_sha256: request_sha256.clone(),
            effect_sha256: effect_sha256.clone(),
            resolution: ExpectedResolution::Present,
        });
    }
    ledger.attempts.push(Attempt {
        attempt_id: format!("ack-{name}").try_into().unwrap(),
        logical_id: name.to_owned().try_into().unwrap(),
        request_sha256: request_sha256.clone(),
        started_ns: started,
        finished_ns: clock.elapsed().as_nanos() as u64,
        outcome: AttemptOutcome::Acknowledged {
            receipt_sha256: receipt_hash(receipt),
        },
    });
    ledger.readback.push(Readback::Present {
        logical_id: name.to_owned().try_into().unwrap(),
        request_sha256,
        effect_sha256,
        receipt_sha256: receipt_hash(receipt),
        applications: 1.try_into().unwrap(),
    });
}

pub(super) fn installed_native_capacity_recovers_acknowledged_history() {
    let point = std::env::var("RRFLOW_CAPACITY_POINT").unwrap();
    assert!([
        "wal_before_write",
        "segment_before_create",
        "manifest_before_create",
        "current_before_create"
    ]
    .contains(&point.as_str()));
    let mode = std::env::var("RRFLOW_CAPACITY_MODE").unwrap();
    assert!(matches!(mode.as_str(), "fresh" | "existing"));
    capacity_case(&point, &mode);
}

/// Characterize the actual installed domain's first WAL append without filling
/// a filesystem. Native exhaustion remains the separate owned-image oracle.
pub(super) fn installed_capacity_wal_payload_is_observed_before_domain_commit() {
    let estate = Estate::install("existing");
    let credential = RrdEngine::read_installed_api_key(&estate.project).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mutations = (0..180)
                .map(|index| TransactionMutation::AssertClaim {
                    subject: CanonicalId::new(format!("crash-uncertain-{index}")).unwrap(),
                    predicate: CanonicalId::new("crash").unwrap(),
                    object: format!("{index:04}").repeat(1024),
                    valid_from: estate.at,
                    tx_time: estate.at,
                    producer: CanonicalId::new("independent-oracle").unwrap(),
                    confidence: None,
                })
                .collect::<Vec<_>>();
            let target = CommitTransaction {
                operation_sha256: transaction_operation_sha256(&mutations),
                mutations,
            };
            let mut control = estate.control("wal_before_write", 1, Some(&target.operation_sha256));
            control["resume"] = json!(true);
            let (mut child, client) = estate.serve(&control);
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
                    options("capacity-session"),
                )
                .await
                .unwrap();
            let lease = begin(&client, &session, "selection").await;
            let controller = tokio::task::spawn_blocking(move || {
                let line = child.line();
                let hit: Value = serde_json::from_str(
                    std::str::from_utf8(&line)
                        .unwrap()
                        .trim()
                        .strip_prefix(PREFIX)
                        .unwrap(),
                )
                .unwrap();
                for key in ["nonce", "id", "point"] {
                    assert_eq!(hit[key], control[key]);
                }
                assert_eq!(hit["pid"], child.child.id());
                println!("installed_capacity_wal_selection={hit}");
                child
                    .child
                    .resume_controlled(
                        &serde_json::to_vec(
                            &json!({"version":1,"nonce":hit["nonce"],"id":hit["id"]}),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                (child, hit)
            });
            let receipt = client
                .commit_transaction(
                    &session,
                    &lease.transaction_id,
                    target.clone(),
                    options("selection"),
                )
                .await
                .unwrap();
            let (child, hit) = controller.await.unwrap();
            assert!(hit["wal_payload_bytes"].as_u64().unwrap() > 0);
            let replay = client
                .commit_transaction(
                    &session,
                    &lease.transaction_id,
                    target.clone(),
                    options("selection"),
                )
                .await
                .unwrap();
            assert!(replay.idempotent_replay);
            assert_eq!(receipt_hash(&receipt), receipt_hash(&replay));
            assert_eq!(
                read_values(&client, &session, estate.at).await,
                requested_values(&target)
            );
            child.kill();
            let (child, client) = estate.serve(&Value::Null);
            assert_eq!(
                read_values(&client, &session, estate.at).await,
                requested_values(&target)
            );
            child.kill();
        });
}
