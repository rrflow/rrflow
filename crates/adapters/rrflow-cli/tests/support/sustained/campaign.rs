//! Collection, sustained capacity, smoke campaigns, sampler traces and disjoint writes.

use super::*;

pub(super) type Completion = (
    u64,
    u64,
    u64,
    u64,
    u64,
    Prepared,
    DriverResult<workload::Completed>,
    Option<DriverResult<()>>,
    bool,
);
pub(super) fn collect(
    (
        sequence,
        scheduled_ns,
        started_ns,
        finished_ns,
        verification_finished_ns,
        operation,
        result,
        verification,
        rejected,
    ): Completion,
    segment_start: u64,
    attempts: &mut Vec<LoadAttempt>,
    ledger: &mut AcknowledgementLedger,
    failure: &mut ReadbackFailures,
    writer: &CampaignWriter,
) -> DriverResult<()> {
    let (outcome, result_sha256, recorded) = match result {
        Ok(result) => (
            LoadOutcome::Acknowledged,
            result.receipt_sha256.clone(),
            AttemptOutcome::Acknowledged {
                receipt_sha256: result.receipt_sha256,
            },
        ),
        Err(_) if rejected => (
            LoadOutcome::Rejected,
            digest("c07g.saturated", &sequence)?,
            AttemptOutcome::Rejected {
                reason: evidence_id("driver-concurrency-limit"),
            },
        ),
        Err(error) if error.starts_with("rejected-") => (
            LoadOutcome::Rejected,
            digest("c07g.rejected-request", &(sequence, &error))?,
            AttemptOutcome::Rejected {
                reason: failure_code(error.strip_prefix("rejected-").expect("matched prefix")),
            },
        ),
        Err(error) => (
            LoadOutcome::Uncertain,
            digest("c07g.request-failure", &(sequence, &error))?,
            AttemptOutcome::Uncertain {
                reason: failure_code(&error),
            },
        ),
    };
    let readback = match outcome {
        LoadOutcome::Acknowledged => match verification {
            Some(Ok(())) => Some(Readback::Present {
                logical_id: evidence_id(&operation.id),
                request_sha256: operation.request_sha256.clone(),
                effect_sha256: operation.effect_sha256.clone(),
                receipt_sha256: result_sha256.clone(),
                applications: 1u64.try_into().unwrap(),
            }),
            Some(Err(error)) => {
                failure.record(&operation.id, &error)?;
                None
            }
            None => return Err("acknowledged worker omitted independent verification".into()),
        },
        LoadOutcome::Rejected => Some(Readback::Absent {
            logical_id: evidence_id(&operation.id),
        }),
        _ => {
            failure.record(&operation.id, result_sha256.as_str())?;
            None
        }
    };
    ledger.attempts.push(Attempt {
        attempt_id: evidence_id(format!("attempt-{sequence}")),
        logical_id: evidence_id(&operation.id),
        request_sha256: operation.request_sha256.clone(),
        started_ns: started_ns - segment_start,
        finished_ns: finished_ns - segment_start,
        outcome: recorded,
    });
    attempts.push(LoadAttempt {
        sequence,
        kind: operation.kind,
        write: operation.write,
        scheduled_ns,
        started_ns,
        finished_ns,
        verification_finished_ns: Some(verification_finished_ns),
        outcome,
        logical_id: evidence_id(operation.id),
        request_sha256: operation.request_sha256,
        result_sha256,
    });
    writer.record_event(&ProgressEvent::LoadOutcome {
        segment: writer.next_index(),
        attempt: attempts.last().ok_or("missing completed attempt")?.clone(),
    })?;
    if let Some(observation) = readback {
        writer.record_event(&ProgressEvent::Readback {
            segment: writer.next_index(),
            observation: observation.clone(),
        })?;
        ledger.readback.push(observation);
    }
    Ok(())
}

pub(super) fn installed_sustained_capacity() {
    let campaign_path = PathBuf::from(
        std::env::var_os("RRFLOW_SUSTAINED_CAMPAIGN").expect("explicit campaign path"),
    );
    let evidence =
        PathBuf::from(std::env::var_os("RRFLOW_SUSTAINED_EVIDENCE").expect("new evidence path"));
    let campaign: StorageCampaign = bounded_json(&campaign_path).unwrap();
    campaign.validate().unwrap();
    if campaign.allocation.estates.get() > 1 {
        return fleet::execute(campaign, evidence);
    }
    let mut writer = CampaignWriter::create(&evidence, campaign.clone()).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run(&campaign, &mut writer))
    }))
    .unwrap_or_else(|_| {
        Err("installed driver panicked; retained progress is incomplete evidence".into())
    });
    let (status, limitations) = if result.is_ok() {
        (CampaignStatus::Completed, vec![])
    } else {
        if let Err(error) = writer.record_event(&ProgressEvent::Failure {
            reason: evidence_id("installed-driver-failed"),
        }) {
            eprintln!("failure event could not be appended: {error}; sealing failed result");
        }
        (
            CampaignStatus::Failed,
            vec![evidence_id("installed-driver-failed")],
        )
    };
    let report = writer.finish(status, limitations).unwrap();
    println!(
        "native_sustained_result={}",
        serde_json::to_string(&report).unwrap()
    );
    assert!(
        result.is_ok(),
        "installed sustained driver failed: {result:?}"
    );
    assert!(
        campaign.purpose != CampaignPurpose::Qualification || report.qualification_passed,
        "campaign completed but did not satisfy its frozen qualification requirements"
    );
}

/// Short executable integration of the campaign's actual workload adapter.
/// It deliberately emits no NativeCapacity/soak qualification claim.
pub(super) fn smoke_campaign() -> StorageCampaign {
    let mut storage: StorageManifest = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../evaluation/qualification/rrflow-qualification-storage/tests/fixtures/storage/manifest.json"
    )))
    .unwrap();
    storage.evidence_class = EvidenceClass::NativeCapacity;
    storage.budgets.estates = 1u64.try_into().unwrap();
    let limit = |value: u64| value.try_into().unwrap();
    StorageCampaign {
        format_version: 1,
        purpose: CampaignPurpose::Characterization,
        allocation: CapacityAllocation {
            owner_projection_sha256: digest("sample-smoke", &1).unwrap(),
            vcpus: limit(1),
            host_bytes: storage.budgets.host_bytes,
            engine_bytes: storage.budgets.process_bytes,
            estates: storage.budgets.estates,
            records: limit(8),
            edges: limit(24),
            vectors: limit(16),
            vector_dimensions: limit(16),
            logical_source_bytes: limit(8192),
        },
        schedule: SustainedSchedule {
            setup_ms: limit(1000),
            lifecycle_ms: limit(1000),
            warmup_ms: limit(2000),
            measured_ms: limit(2000),
            burst_ms: limit(2000),
            drain_ms: limit(2000),
            soak_ms: limit(2000),
            sample_ms: limit(100),
            operation_deadline_ms: limit(10_000),
            forced_reopens: limit(1),
        },
        read_distribution: ReadKeyDistribution::Uniform {},
        drain: DrainPolicy {
            maximum_mutable_growth_bytes: 0,
            maximum_projected_growth_bytes: 0,
            maximum_debt_segments: 0,
            stable_samples: limit(2),
        },
        seeds: vec![73],
        points: vec![OperatingPoint {
            id: evidence_id("sample-smoke"),
            clients: limit(4),
            requests_per_second: limit(5),
            maximum_error_basis_points: 10_000,
            maximum_p99_ns: limit(10_000_000_000),
        }],
        required_lanes: vec![CapacityLane::Warm],
        evidence: EvidenceLimits {
            segment_attempts: limit(6),
            segment_bytes: limit(MAX_DOCUMENT_BYTES),
            total_bytes: limit(64 * 1024 * 1024),
            segments: limit(10),
            samples_per_segment: limit(64),
        },
        reader_selection: None,
        storage,
    }
}

pub(super) fn backup_is_due_only_for_the_selected_lane_and_first_measured_mixed_write() {
    let mut campaign = smoke_campaign();
    // The default characterization requests warm only.
    assert!(!backup_due(&campaign, LoadPhase::Measured, 14));
    campaign.required_lanes.push(CapacityLane::BackupUnderWrite);
    assert_eq!(storage_load_kind(14), StorageLoadKind::MixedWrite);
    assert!(backup_due(&campaign, LoadPhase::Measured, 14));
    for phase in [
        LoadPhase::Warmup,
        LoadPhase::Burst,
        LoadPhase::Drain,
        LoadPhase::Soak,
    ] {
        assert!(!backup_due(&campaign, phase, 14));
    }
    assert!(!backup_due(&campaign, LoadPhase::Measured, 13));
    assert!(!backup_due(&campaign, LoadPhase::Measured, 15));
}

pub(super) fn installed_sampler_trace_matches_server_and_store() {
    let estate = Estate::install("fresh");
    let clock = Instant::now();
    let (mut child, client, _) = estate.start_for_with_trace(clock, Duration::from_secs(300), true);
    let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (request_id, mut work, before) = runtime.block_on(async {
        let lease = client
            .create_session(
                credential.principal_id.clone(),
                credential.credential(),
                CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 300_000,
                        absolute_timeout_ms: 600_000,
                        max_open_transactions: 4,
                    },
                },
                RequestOptions::mutation(
                    "trace-sample-session",
                    "trace-sample-session",
                    "trace-sample-session",
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let work = Workload {
            native_worker: Default::default(),
            client,
            session: lease,
            valid_at: estate.at,
            dimensions: 16,
            seed: 73,
            records: 8,
            read_distribution: ReadKeyDistribution::Uniform {},
            edges_per_record: 3,
            vectors_per_record: 2,
            payload_bytes: 1024,
            query_budget: serde_json::from_value(estate.plan["configuration"]["query"].clone())
                .unwrap(),
        };
        work.initialize().await.unwrap();
        for ordinal in 0..work.records {
            let seed = work.seed_record(ordinal).unwrap();
            assert!(!work.execute(&seed).await.unwrap().idempotent_replay);
        }
        let before = work
            .verify_joined_search("seed-00000000000000000007", 1, 7, 5.53125)
            .await
            .unwrap();
        let (sample, request_id, _) = sample_with_identity(&work, child.id(), clock, &None)
            .await
            .unwrap();
        assert!(sample.storage.is_some());
        (request_id, work, before)
    });
    child.terminate();
    let output = child.finish().expect("reap owned traced server");
    check_sample_trace(&output.stderr, &request_id, credential.credential());
    let (mut restarted, client, _) =
        estate.start_for_with_trace(clock, Duration::from_secs(300), true);
    work.client = client;
    let second_id = runtime.block_on(async {
        let after = work
            .verify_joined_search("seed-00000000000000000007", 1, 7, 5.53125)
            .await
            .unwrap();
        assert_eq!(after.known_at_cursor, before.known_at_cursor);
        assert_eq!(after.hits, before.hits);
        let (sample, id, _) = sample_with_identity(&work, restarted.id(), clock, &None)
            .await
            .unwrap();
        assert!(sample.storage.is_some());
        id
    });
    assert_ne!(request_id, second_id);
    restarted.terminate();
    let output = restarted.finish().expect("reap restarted traced server");
    check_sample_trace(&output.stderr, &second_id, credential.credential());
}

pub(super) fn check_sample_trace(bytes: &[u8], request_id: &str, credential: &str) -> [u64; 7] {
    let raw = std::str::from_utf8(bytes).unwrap();
    assert!(!raw.contains(request_id));
    assert!(!raw.contains(credential));
    let digest = rrd_core::digest::sha256_hex(request_id.as_bytes());
    let events: Vec<serde_json::Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).expect("trace is complete JSON"))
        .collect();
    let find = |target: &str| {
        let matching: Vec<_> = events
            .iter()
            .filter(|event| event["target"] == target && event["span"]["request_sha256"] == digest)
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "missing or duplicate correlated event: {target}"
        );
        matching[0]
    };
    let storage = find("rrd_store::diagnostic");
    let service = find("rrd_server::diagnostic");
    let field = |name: &str| storage["fields"][name].as_u64().unwrap();
    let wait = field("mutex_wait_ns");
    let assembly = field("mutex_held_assembly_ns");
    let profile = field("memtable_profile_ns");
    let elapsed = service["fields"]["service_elapsed_ns"].as_u64().unwrap();
    assert_eq!(service["fields"]["success"], true);
    assert!(profile <= assembly);
    assert!(u128::from(wait) + u128::from(assembly) <= u128::from(elapsed));
    let counts = [
        field("memtable_keys"),
        field("memtable_versions"),
        field("segment_count"),
    ];
    println!("installed sampler trace matched: request_sha256={digest} wait_ns={wait} assembly_ns={assembly} profile_ns={profile} service_ns={elapsed} keys={} versions={} segments={}", counts[0], counts[1], counts[2]);
    [
        wait, assembly, profile, elapsed, counts[0], counts[1], counts[2],
    ]
}

pub(super) fn installed_sampler_trace_during_write_load() {
    const RECORDS: u64 = 4_000;
    const PERIOD: u64 = 125;
    let estate = Estate::install("fresh");
    let clock = Instant::now();
    let (mut child, client, _) = estate.start_for_with_trace(clock, Duration::from_secs(600), true);
    let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (mut work, observations, acknowledged, failures) = runtime.block_on(async {
        let session = client
            .create_session(
                credential.principal_id.clone(),
                credential.credential(),
                CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 300_000,
                        absolute_timeout_ms: 600_000,
                        max_open_transactions: 4,
                    },
                },
                RequestOptions::mutation("loaded-session", "loaded-session", "loaded-session")
                    .unwrap(),
            )
            .await
            .unwrap();
        let work = Workload {
            native_worker: Default::default(),
            client,
            session,
            valid_at: estate.at,
            dimensions: 16,
            seed: 73,
            records: RECORDS,
            read_distribution: ReadKeyDistribution::Uniform {},
            edges_per_record: 3,
            vectors_per_record: 2,
            payload_bytes: 1024,
            query_budget: serde_json::from_value(estate.plan["configuration"]["query"].clone())
                .unwrap(),
        };
        work.initialize().await.unwrap();
        let mut observations = Vec::new();
        let mut acknowledged = 0u64;
        let mut failures = Vec::new();
        let failed = |stage: &str, ordinal: u64, detail: &str| {
            format!(
                "{stage} ordinal={ordinal} error_sha256={}",
                rrd_core::digest::sha256_hex(detail.as_bytes())
            )
        };
        for ordinal in 0..RECORDS {
            let seed = work.seed_record(ordinal).unwrap();
            let written = if ordinal % PERIOD == 0 {
                let write = async {
                    let start = ns(clock);
                    let result = work.execute(&seed).await;
                    (start, ns(clock), result)
                };
                let diagnostic = async {
                    let start = ns(clock);
                    let result = sample_with_identity(&work, child.id(), clock, &None).await;
                    (start, ns(clock), result)
                };
                let ((write_start, write_end, written), (sample_start, sample_end, sampled)) =
                    tokio::join!(write, diagnostic);
                match sampled {
                    Ok((sample, id, timing)) if sample.storage.is_some() => {
                        observations.push((
                            id,
                            ordinal,
                            write_start,
                            write_end,
                            sample_start,
                            sample_end,
                            timing,
                        ));
                    }
                    Ok(_) => {
                        failures.push(failed("sample-missing-storage", ordinal, "missing storage"))
                    }
                    Err(error) => failures.push(failed("sample", ordinal, &error)),
                }
                written
            } else {
                work.execute(&seed).await
            };
            match written {
                Ok(result) if !result.idempotent_replay => acknowledged += 1,
                Ok(_) => failures.push(failed("unexpected-replay", ordinal, "replayed new seed")),
                Err(error) => failures.push(failed("write", ordinal, &error)),
            }
            if !failures.is_empty() {
                break;
            }
            if acknowledged.is_multiple_of(500) {
                println!(
                    "loaded_progress acknowledged={acknowledged} samples={}",
                    observations.len()
                );
            }
        }
        if failures.is_empty() {
            for ordinal in [0, 1_333, 2_666, RECORDS - 1] {
                if let Err(error) = work.verify(&work.seed_record(ordinal).unwrap()).await {
                    failures.push(failed("readback", ordinal, &error));
                    break;
                }
            }
        }
        (work, observations, acknowledged, failures)
    });
    let overlapping = observations
        .iter()
        .filter(|(_, _, ws, we, ss, se, _)| ws < se && ss < we)
        .count();
    child.terminate();
    let output = child.finish().expect("reap loaded traced server");
    for (id, ordinal, write_start, write_end, sample_start, sample_end, timing) in &observations {
        let fields = check_sample_trace(&output.stderr, id, credential.credential());
        println!("loaded_sample ordinal={ordinal} write_start_ns={write_start} write_end_ns={write_end} sample_start_ns={sample_start} sample_end_ns={sample_end} total_ns={} host_ns={} rpc_ns={} memory_ns={} enforcement_ns={} wait_ns={} assembly_ns={} profile_ns={} service_ns={} keys={} versions={} segments={}", timing.total_ns, timing.host_ns, timing.rpc_ns, timing.memory_ns, timing.enforcement_ns, fields[0], fields[1], fields[2], fields[3], fields[4], fields[5], fields[6]);
    }
    println!("loaded_summary acknowledged={acknowledged} samples={} overlapping_client_intervals={overlapping} failures={failures:?}; unqualified synthetic workload", observations.len());
    assert!(
        failures.is_empty(),
        "loaded characterization failed: {failures:?}"
    );
    assert_eq!(acknowledged, RECORDS);
    assert_eq!(observations.len(), (RECORDS / PERIOD) as usize);
    assert!(
        overlapping > 0,
        "no client-observed diagnostic/write overlap"
    );
    let (mut restarted, client, _) = estate.start_for(clock, Duration::from_secs(300));
    work.client = client;
    runtime.block_on(async {
        for ordinal in [0, 1_333, 2_666, RECORDS - 1] {
            let seed = work.seed_record(ordinal).unwrap();
            work.verify(&seed).await.unwrap();
            assert!(work.execute(&seed).await.unwrap().idempotent_replay);
        }
    });
    restarted.terminate();
    restarted.finish().expect("reap reopened server");
}

pub(super) fn installed_disjoint_writes_from_one_stamp_conflict_without_partial_effect() {
    use rrd_contract::{
        AbortTransaction, BeginTransaction, CanonicalId, ErrorCode, TransactionState,
    };

    fn mutation_options(id: &str) -> RequestOptions {
        RequestOptions::new(id, id, Some(id), Some(u64::MAX)).unwrap()
    }

    let estate = Estate::install("fresh");
    let clock = Instant::now();
    let (child, client, _) = estate.start_for(clock, Duration::from_secs(300));
    let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let session = client
            .create_session(
                credential.principal_id.clone(),
                credential.credential(),
                CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 300_000,
                        absolute_timeout_ms: 600_000,
                        max_open_transactions: 4,
                    },
                },
                mutation_options("conflict-oracle-session"),
            )
            .await
            .unwrap();
        let mut work = Workload {
            native_worker: Default::default(),
            client,
            session,
            valid_at: estate.at,
            dimensions: 16,
            seed: 73,
            records: 8,
            read_distribution: ReadKeyDistribution::Uniform {},
            edges_per_record: 3,
            vectors_per_record: 2,
            payload_bytes: 1024,
            query_budget: serde_json::from_value(estate.plan["configuration"]["query"].clone())
                .unwrap(),
        };
        work.initialize().await.unwrap();
        let first = work.seed_record(0).unwrap();
        let second = work.seed_record(1).unwrap();
        let begin = || BeginTransaction {
            scope: CanonicalId::new("data").unwrap(),
            timeout_ms: 60_000,
        };
        let left = work
            .client
            .begin_transaction(
                &work.session,
                begin(),
                mutation_options("conflict-left-begin"),
            )
            .await
            .unwrap();
        let right = work
            .client
            .begin_transaction(
                &work.session,
                begin(),
                mutation_options("conflict-right-begin"),
            )
            .await
            .unwrap();
        assert_eq!(left.read_cursor, right.read_cursor);
        let accepted = work
            .client
            .commit_transaction(
                &work.session,
                &left.transaction_id,
                first.mutation.clone().unwrap(),
                mutation_options("conflict-left-commit"),
            )
            .await
            .unwrap();
        let published_cursor = accepted.last_runtime_cursor.unwrap();
        assert!(published_cursor > left.read_cursor);
        let rejected = work
            .client
            .commit_transaction(
                &work.session,
                &right.transaction_id,
                second.mutation.clone().unwrap(),
                mutation_options("conflict-right-stale-commit"),
            )
            .await
            .unwrap_err();
        match rejected {
            rrd_client::Error::Api { status, error } => {
                assert_eq!(error.code, ErrorCode::Conflict);
                assert!(
                    !error.retryable,
                    "a stale commit cannot be retried as the same transaction"
                );
                assert_eq!(
                    error
                        .details
                        .get(&CanonicalId::new("reason").unwrap())
                        .map(String::as_str),
                    Some("transaction-snapshot-stale")
                );
                assert_eq!(
                    error
                        .details
                        .get(&CanonicalId::new("next-action").unwrap())
                        .map(String::as_str),
                    Some("abort-and-reexecute-transaction")
                );
                assert_eq!(
                    error
                        .details
                        .get(&CanonicalId::new("expected-cursor").unwrap()),
                    Some(&right.read_cursor.to_string())
                );
                assert_eq!(
                    error
                        .details
                        .get(&CanonicalId::new("actual-cursor").unwrap()),
                    Some(&published_cursor.to_string())
                );
                let api = rrd_client::Error::Api { status, error };
                let trace = workload::runtime_conflict_trace(
                    &api,
                    "synthetic-conflict-key",
                    right.read_cursor,
                    &std::sync::atomic::AtomicUsize::new(0),
                )
                .expect("real stale response must retain bounded cursor diagnostics");
                assert!(trace.contains(&format!(
                    "read_cursor={} expected_cursor={} actual_cursor={}",
                    right.read_cursor, right.read_cursor, published_cursor
                )));
                assert!(!trace.contains("synthetic-conflict-key"));
                assert!(workload::client_error(api)
                    .starts_with("api-conflict-storage-runtime-conflict:"));
            }
            other => panic!("unexpected typed error: {other:?}"),
        }
        let repeated = work
            .client
            .commit_transaction(
                &work.session,
                &right.transaction_id,
                second.mutation.clone().unwrap(),
                mutation_options("conflict-right-stale-commit"),
            )
            .await
            .unwrap_err();
        match repeated {
            rrd_client::Error::Api { error, .. } => {
                assert_eq!(error.code, ErrorCode::Conflict);
                assert!(!error.retryable);
                assert_eq!(
                    error
                        .details
                        .get(&CanonicalId::new("reason").unwrap())
                        .map(String::as_str),
                    Some("transaction-snapshot-stale")
                );
            }
            other => panic!("unexpected repeated error: {other:?}"),
        }
        let aborted = work
            .client
            .abort_transaction(
                &work.session,
                &right.transaction_id,
                AbortTransaction {},
                mutation_options("conflict-right-abort"),
            )
            .await
            .unwrap();
        assert_eq!(aborted.state, TransactionState::Aborted);
        work.verify(&first).await.unwrap();
        work.verify_absent(&second.key).await.unwrap();
        let fresh = work.execute(&second).await.unwrap();
        assert!(!fresh.idempotent_replay);
        work.verify(&second).await.unwrap();
        super::super::kill(child, clock);
        let (reopened, client, _) = estate.start_for(clock, Duration::from_secs(300));
        work.client = client;
        work.verify(&first).await.unwrap();
        work.verify(&second).await.unwrap();
        let replay = work.execute(&second).await.unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(replay.receipt_sha256, fresh.receipt_sha256);
        super::super::kill(reopened, clock);
    });
}
