//! The installed capacity workload smoke.

use super::*;

#[test]
#[ignore = "explicit native capacity-driver smoke; candidate portability job"]
fn installed_capacity_workload_smoke() {
    let estate = Estate::install("fresh");
    let clock = Instant::now();
    let (child, client, _) = estate.start_for(clock, Duration::from_secs(300));
    let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
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
                    "capacity-smoke-session",
                    "capacity-smoke-session",
                    "capacity-smoke-session",
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let mut work = Workload {
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
        let mut receipts = Vec::new();
        for ordinal in 0..work.records {
            let operation = work.seed_record(ordinal).unwrap();
            let receipt = work.execute(&operation).await.unwrap();
            assert!(!receipt.idempotent_replay);
            receipts.push(receipt.receipt_sha256);
        }
        // For seed 73 and 16 all-one query dimensions, vector 1 of seed 7
        // uniquely maximizes dot product: sum(81..=96) / 256 = 5.53125.
        let winning_key = "seed-00000000000000000007";
        let before = work.verify_joined_search(winning_key, 1, 7, 5.53125).await.unwrap();
        super::super::kill(child, clock);
        let (mut child, client, _) = estate.start_for(clock, Duration::from_secs(300));
        work.client = client;
        let after = work.verify_joined_search(winning_key, 1, 7, 5.53125).await.unwrap();
        assert_eq!(after.known_at_cursor, before.known_at_cursor);
        assert_eq!(after.hits, before.hits);
        let replay = work.execute(&work.seed_record(7).unwrap()).await.unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(replay.receipt_sha256, receipts[7]);
        let mut verified = 0;
        for first in (0..work.records).step_by(2) {
            let results = corpus::verify_window(
                &work,
                first,
                (first + 2).min(work.records),
                2,
                Duration::from_secs(30),
            )
            .await
            .unwrap();
            for (ordinal, effect) in results {
                assert_eq!(ordinal, verified);
                assert_eq!(effect, work.seed_record(ordinal).unwrap().effect_sha256);
                verified += 1;
            }
        }
        assert_eq!(verified, work.records);
        for (index, kind) in [
            StorageLoadKind::PointRead,
            StorageLoadKind::RangeRead,
            StorageLoadKind::Maintenance,
        ]
        .into_iter()
        .enumerate()
        {
            let operation = work
                .prepare(format!("smoke-read-{index}"), kind, index as u64)
                .unwrap();
            work.execute(&operation).await.unwrap();
            work.verify(&operation).await.unwrap();
        }
        // Exercise the campaign sampler against the owned native process as
        // well as the independent public diagnostics request below.
        let sampled = sample(&work, child.id(), clock, &None).await.unwrap();
        assert_eq!(sampled.pid, child.id());
        assert!(sampled.elapsed_ns <= ns(clock));
        assert!(sampled.storage.as_ref().is_some_and(|storage|
            storage.rrflow_kv.is_some() && storage.admission.is_some()));
        // Hold an owned asynchronous sample while a real engine request and
        // its independent readback make progress, then acquire the sample.
        let (release_sample, held_sample) = tokio::sync::oneshot::channel();
        let (sample_started, started_sample) = tokio::sync::oneshot::channel();
        let sample_work = work.clone();
        let sample_pid = child.id();
        let mut sampling = sample_task::SampleTask::new();
        sampling.start(async move {
            sample_started.send(()).map_err(|_| "smoke sample observer closed".to_owned())?;
            held_sample.await.map_err(|_| "smoke sample cancelled".to_owned())?;
            sample(&sample_work, sample_pid, clock, &None).await
        }).unwrap();
        started_sample.await.unwrap();
        let during_sample = work.prepare("smoke-sample-followup".into(), StorageLoadKind::PointRead, 2).unwrap();
        work.execute(&during_sample).await.unwrap();
        work.verify(&during_sample).await.unwrap();
        assert!(sampling.is_pending());
        release_sample.send(()).unwrap();
        assert_eq!(sampling.finish().await.unwrap().unwrap().pid, sample_pid);
        // This disposable contract-fixture campaign exercises the actual
        // scheduled segment. Its synthetic identity is not qualification.
        let campaign = smoke_campaign();
        let evidence_root = tempfile::tempdir().unwrap();
        let evidence = evidence_root.path().join("segment-evidence");
        let writer = CampaignWriter::create(&evidence, campaign.clone()).unwrap();
        writer.admit_next().unwrap();
        let mut point_run = PointRun {
            campaign: &campaign,
            point: &campaign.points[0],
            workload: work.clone(),
            pid: child.id(),
            clock,
            allocation: None,
            generations: vec![0; 6],
            previous_resources: Vec::new(),
            backup: management::ManagementTask::new(),
            sample_gate: None,
        };
        let segment = point_run.segment(&writer, LoadPhase::Warmup, ns(clock), 0, 6, true).await.unwrap();
        assert_eq!(segment.attempts.len(), 6);
        assert!(segment.resources.len() >= 3);
        assert!(segment.resources.iter().all(|value| value.pid == child.id()));
        writer.checkpoint_progress().unwrap();
        let progress = std::fs::read_to_string(evidence.join("progress.jsonl")).unwrap();
        let events: Vec<ProgressEvent> = progress.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let resources: Vec<_> = events.iter().filter_map(|event| match event {
            ProgressEvent::Resource { segment: 0, sample } => Some(serde_json::to_value(sample).unwrap()),
            _ => None,
        }).collect();
        let expected_resources: Vec<_> = segment.resources.iter().map(|sample| serde_json::to_value(sample).unwrap()).collect();
        assert_eq!(resources, expected_resources);
        let mut outcomes: Vec<_> = events.iter().filter_map(|event| match event {
            ProgressEvent::LoadOutcome { segment: 0, attempt } => Some(attempt),
            _ => None,
        }).collect();
        outcomes.sort_by_key(|attempt| attempt.sequence);
        assert_eq!(serde_json::to_value(outcomes).unwrap(), serde_json::to_value(&segment.attempts).unwrap());
        // A later segment's initial sample must not hold scheduled offers.
        // No carried samples are supplied, making the first Resource event a
        // deterministic distinction from the old synchronous initial sample.
        let mut initial_campaign = smoke_campaign();
        initial_campaign.schedule.sample_ms = 1_000u64.try_into().unwrap();
        let mut host = sysinfo::System::new();
        host.refresh_memory();
        let host_bytes = host.total_memory();
        assert!(host_bytes > 0, "real host memory is required for this installed fixture");
        initial_campaign.storage.budgets.host_bytes = host_bytes.try_into().unwrap();
        initial_campaign.allocation.host_bytes = host_bytes.try_into().unwrap();
        initial_campaign.storage.budgets.retained_bytes = (16 * 1024 * 1024u64).try_into().unwrap();
        initial_campaign.required_lanes.push(CapacityLane::ReaderPinInterference);
        initial_campaign.reader_selection = Some(ReaderSelection::WorkloadRecords {});
        let initial_evidence = evidence_root.path().join("initial-sample-segment");
        let initial_writer = CampaignWriter::create(&initial_evidence, initial_campaign.clone()).unwrap();
        initial_writer.admit_next().unwrap();
        let initial_gate = std::sync::Arc::new(sample_task::SampleGate::default());
        let initial_clock = Instant::now() - Duration::from_secs(10);
        let mut initial_run = PointRun {
            campaign: &initial_campaign, point: &initial_campaign.points[0],
            workload: work.clone(), pid: child.id(), clock: initial_clock, allocation: None,
            generations: vec![0; 5], previous_resources: Vec::new(),
            backup: management::ManagementTask::new(), sample_gate: Some(initial_gate.clone()),
        };
        // Sequences 15-18 are real writes; avoid sequence 14's backup lane.
        let origin = ns(initial_clock).checked_sub(3_000_000_000).unwrap();
        let mut initial_segment = Box::pin(initial_run.segment(&initial_writer, LoadPhase::Measured, origin, 15, 5, true));
        tokio::time::timeout(Duration::from_secs(20), async {
            tokio::select! {
                _ = initial_gate.entered.notified() => {}
                result = &mut initial_segment => panic!("segment ended before initial sample gate: {result:?}"),
            }
        }).await.unwrap();
        initial_writer.checkpoint_progress().unwrap();
        let progress = std::fs::read_to_string(initial_evidence.join("progress.jsonl")).unwrap();
        assert!(!progress.lines().map(|line| serde_json::from_str::<ProgressEvent>(line).unwrap())
            .any(|event| matches!(event, ProgressEvent::Resource { .. })), "initial sample completed before its gate");
        let acknowledged = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                tokio::select! {
                    result = &mut initial_segment => panic!("segment ended while initial sample held: {result:?}"),
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        initial_writer.checkpoint_progress().unwrap();
                        let raw = std::fs::read_to_string(initial_evidence.join("progress.jsonl")).unwrap();
                        let events: Vec<ProgressEvent> = raw.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
                        if let Some(attempt) = events.iter().find_map(|event| match event {
                            ProgressEvent::LoadOutcome { attempt, .. }
                                if attempt.outcome == LoadOutcome::Acknowledged
                                    && events.iter().any(|other| matches!(other,
                                        ProgressEvent::Readback { observation: Readback::Present { logical_id, .. }, .. }
                                            if logical_id == &attempt.logical_id)) => Some(attempt.clone()),
                            _ => None,
                        }) { break attempt; }
                    }
                }
            }
        }).await.unwrap();
        assert!(!initial_gate.dropped.load(std::sync::atomic::Ordering::SeqCst));
        initial_gate.open();
        let initial_segment = tokio::time::timeout(Duration::from_secs(30), initial_segment).await.unwrap().unwrap();
        assert!(initial_segment.attempts.iter().any(|attempt| attempt.sequence == acknowledged.sequence));
        assert!(!initial_segment.resources.is_empty());
        let reader = initial_segment.reader.as_ref().expect("reader remains mandatory");
        reader.validate().unwrap();
        assert!(reader.opened_ns <= acknowledged.started_ns && acknowledged.finished_ns <= reader.observed_ns);
        validate_segment(&initial_campaign, &initial_segment).unwrap_or_else(|error| {
            let retained: Vec<_> = initial_segment.resources.iter().map(|sample| &sample.retained_bytes).collect();
            panic!("segment validation failed: {error}; retained samples: {retained:?}")
        });
        let mut missing_samples = initial_segment.clone();
        missing_samples.resources.clear();
        assert_eq!(validate_segment(&initial_campaign, &missing_samples).unwrap_err(),
            "segment evidence missing or exceeds bound");
        // Hold a harness-only asynchronous sample in the actual scheduled
        // segment. A later real ACK must be journaled before the gate opens;
        // the completed segment must contain that exact acknowledged attempt.
        let mut held_campaign = smoke_campaign();
        held_campaign.schedule.warmup_ms = 5_000u64.try_into().unwrap();
        held_campaign.evidence.segment_attempts = 20u64.try_into().unwrap();
        let held_evidence = evidence_root.path().join("held-segment-evidence");
        let held_writer = CampaignWriter::create(&held_evidence, held_campaign.clone()).unwrap();
        held_writer.admit_next().unwrap();
        let held_gate = std::sync::Arc::new(sample_task::SampleGate::default());
        let mut held_run = PointRun {
            campaign: &held_campaign,
            point: &held_campaign.points[0],
            workload: work.clone(),
            pid: child.id(), clock, allocation: None, generations: vec![0; 20],
            previous_resources: Vec::new(), backup: management::ManagementTask::new(),
            sample_gate: Some(held_gate.clone()),
        };
        let mut held_segment = Box::pin(held_run.segment(&held_writer, LoadPhase::Warmup, ns(clock), 0, 20, true));
        tokio::time::timeout(Duration::from_secs(20), async {
            tokio::select! {
                _ = held_gate.entered.notified() => {}
                result = &mut held_segment => panic!("segment ended before held sample: {result:?}"),
            }
        }).await.unwrap();
        let held_ack = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                tokio::select! {
                    result = &mut held_segment => panic!("segment ended while sample was held: {result:?}"),
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        held_writer.checkpoint_progress().unwrap();
                        let raw = std::fs::read_to_string(held_evidence.join("progress.jsonl")).unwrap();
                        let acknowledged = raw.lines().map(|line| serde_json::from_str::<ProgressEvent>(line).unwrap()).find_map(|event|
                            match event {
                                ProgressEvent::LoadOutcome { attempt, .. }
                                    if attempt.sequence >= 2 && attempt.outcome == LoadOutcome::Acknowledged => Some(attempt),
                                _ => None,
                            });
                        if let Some(attempt) = acknowledged { break attempt; }
                    }
                }
            }
        }).await.unwrap();
        assert!(!held_gate.dropped.load(std::sync::atomic::Ordering::SeqCst));
        // Move the hold into the real native helper after the async gate.
        // Record a new ACK after observing that helper's entry, rather than
        // counting an earlier event as proof of concurrent scheduling.
        let native_marker = evidence_root.path().join("held-native-worker");
        work.native_worker.hold_after(0, native_marker.clone());
        held_gate.open();
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                tokio::select! {
                    result = &mut held_segment => panic!("segment ended before native hold: {result:?}"),
                    _ = tokio::time::sleep(Duration::from_millis(5)) => {
                        if native_marker.exists() { break; }
                    }
                }
            }
        }).await.unwrap();
        held_writer.checkpoint_progress().unwrap();
        let before_native = std::fs::read_to_string(held_evidence.join("progress.jsonl")).unwrap();
        let max_before = before_native.lines().map(|line| serde_json::from_str::<ProgressEvent>(line).unwrap())
            .filter_map(|event| match event {
                ProgressEvent::LoadOutcome { attempt, .. }
                    if attempt.outcome == LoadOutcome::Acknowledged => Some(attempt.sequence),
                _ => None,
            }).max().unwrap_or(0);
        let native_ack = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                tokio::select! {
                    result = &mut held_segment => panic!("segment ended while native helper was held: {result:?}"),
                    _ = tokio::time::sleep(Duration::from_millis(25)) => {
                        held_writer.checkpoint_progress().unwrap();
                        let raw = std::fs::read_to_string(held_evidence.join("progress.jsonl")).unwrap();
                        if let Some(attempt) = raw.lines().map(|line| serde_json::from_str::<ProgressEvent>(line).unwrap())
                            .find_map(|event| match event {
                                ProgressEvent::LoadOutcome { attempt, .. }
                                    if attempt.sequence > max_before && attempt.outcome == LoadOutcome::Acknowledged => Some(attempt),
                                _ => None,
                            }) { break attempt; }
                    }
                }
            }
        }).await.unwrap();
        std::fs::write(native_marker.with_extension("release"), b"release").unwrap();
        let held_result = tokio::time::timeout(Duration::from_secs(20), &mut held_segment).await.unwrap().unwrap();
        drop(held_segment);
        assert_eq!(held_result.attempts.len(), 20);
        assert!(held_result.attempts.iter().any(|attempt|
            attempt.sequence == native_ack.sequence && attempt.request_sha256 == native_ack.request_sha256
                && attempt.result_sha256 == native_ack.result_sha256 && attempt.outcome == LoadOutcome::Acknowledged));
        assert!(held_result.resources.len() >= 3);
        assert!(held_result.resources.iter().all(|sample| sample.pid == child.id()));
        assert!(held_result.attempts.iter().any(|attempt|
            attempt.sequence == held_ack.sequence && attempt.request_sha256 == held_ack.request_sha256
                && attempt.result_sha256 == held_ack.result_sha256 && attempt.outcome == LoadOutcome::Acknowledged));
        held_writer.checkpoint_progress().unwrap();
        let raw = std::fs::read_to_string(held_evidence.join("progress.jsonl")).unwrap();
        let events: Vec<ProgressEvent> = raw.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let resources: Vec<_> = events.iter().filter_map(|event| match event {
            ProgressEvent::Resource { segment: 0, sample } => Some(serde_json::to_value(sample).unwrap()),
            _ => None,
        }).collect();
        let expected_resources: Vec<_> = held_result.resources.iter().map(|sample| serde_json::to_value(sample).unwrap()).collect();
        assert_eq!(resources, expected_resources);
        let mut outcomes: Vec<_> = events.iter().filter_map(|event| match event {
            ProgressEvent::LoadOutcome { segment: 0, attempt } => Some(attempt),
            _ => None,
        }).collect();
        outcomes.sort_by_key(|attempt| attempt.sequence);
        assert_eq!(serde_json::to_value(outcomes).unwrap(), serde_json::to_value(&held_result.attempts).unwrap());
        println!("held_segment_summary async_ack_sequence={} native_ack_sequence={} attempts={} resources={} checkpoint_matches=true", held_ack.sequence, native_ack.sequence, held_result.attempts.len(), held_result.resources.len());
        // Interrupt a real segment while its harness-only async sample is held.
        // Only independently verified ACKs are replayed after an owned restart.
        let mut interrupted_campaign = smoke_campaign();
        interrupted_campaign.schedule.warmup_ms = 5_000u64.try_into().unwrap();
        interrupted_campaign.evidence.segment_attempts = 20u64.try_into().unwrap();
        let interrupted_evidence = evidence_root.path().join("interrupted-evidence");
        let interrupted_writer = CampaignWriter::create(&interrupted_evidence, interrupted_campaign.clone()).unwrap();
        interrupted_writer.admit_next().unwrap();
        let gate = std::sync::Arc::new(sample_task::SampleGate::default());
        let mut interrupted_run = PointRun {
            campaign: &interrupted_campaign,
            point: &interrupted_campaign.points[0],
            workload: work.clone(),
            pid: child.id(), clock, allocation: None, generations: vec![0; 20],
            previous_resources: Vec::new(), backup: management::ManagementTask::new(),
            sample_gate: Some(gate.clone()),
        };
        let mut interrupted = Box::pin(interrupted_run.segment(&interrupted_writer, LoadPhase::Warmup, ns(clock), 0, 20, true));
        tokio::time::timeout(Duration::from_secs(20), async {
            tokio::select! {
                _ = gate.entered.notified() => {}
                result = &mut interrupted => panic!("segment ended before sample gate: {result:?}"),
            }
        }).await.unwrap();
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                tokio::select! {
                    result = &mut interrupted => panic!("segment ended while sample was held: {result:?}"),
                    _ = tokio::time::sleep(Duration::from_millis(200)) => {
                        interrupted_writer.checkpoint_progress().unwrap();
                        let raw = std::fs::read_to_string(interrupted_evidence.join("progress.jsonl")).unwrap();
                        let seen = raw.lines().map(|line| serde_json::from_str::<ProgressEvent>(line).unwrap()).any(|event|
                            matches!(event, ProgressEvent::LoadOutcome { attempt, .. }
                                if attempt.write.is_some() && attempt.outcome == LoadOutcome::Acknowledged));
                        if seen { break; }
                    }
                }
            }
        }).await.unwrap();
        // Keep the original async interruption oracle, then move the held
        // acquisition into the native helper and interrupt that real segment.
        let cancel_marker = evidence_root.path().join("cancelled-native-worker");
        work.native_worker.hold_after(0, cancel_marker.clone());
        gate.open();
        let helper_pid = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                tokio::select! {
                    result = &mut interrupted => panic!("segment ended before native interruption: {result:?}"),
                    _ = tokio::time::sleep(Duration::from_millis(5)) => {
                        if let Ok(raw) = std::fs::read_to_string(&cancel_marker) {
                            if let Ok(pid) = raw.parse::<u32>() { break pid; }
                        }
                    }
                }
            }
        }).await.unwrap();
        drop(interrupted);
        work.native_worker.shutdown().await.unwrap();
        let mut processes = sysinfo::System::new();
        let helper = sysinfo::Pid::from_u32(helper_pid);
        processes.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[helper]), true);
        assert!(processes.process(helper).is_none(), "interrupted native helper survived");
        tokio::time::timeout(Duration::from_secs(2), async {
            while !gate.dropped.load(std::sync::atomic::Ordering::SeqCst) { tokio::task::yield_now().await; }
        }).await.unwrap();
        interrupted_writer.checkpoint_progress().unwrap();
        let raw = std::fs::read_to_string(interrupted_evidence.join("progress.jsonl")).unwrap();
        let interrupted_events: Vec<ProgressEvent> = raw.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let acknowledged: Vec<_> = interrupted_events.iter().filter_map(|event| match event {
            ProgressEvent::LoadOutcome { attempt, .. } if attempt.write.is_some() && attempt.outcome == LoadOutcome::Acknowledged => Some(attempt.clone()),
            _ => None,
        }).collect();
        assert!(!acknowledged.is_empty());
        assert_eq!(interrupted_events.iter().filter(|event| matches!(event, ProgressEvent::Resource { .. })).count(), 1);
        super::super::kill(child, clock);
        let (restarted, client, _) = estate.start_for(clock, Duration::from_secs(300));
        child = restarted;
        work.client = client;
        assert_eq!(std::fs::read_to_string(interrupted_evidence.join("progress.jsonl")).unwrap(), raw);
        for attempt in &acknowledged {
            let operation = work.prepare_load(format!("load-0-{}", attempt.sequence), attempt.write.unwrap()).unwrap();
            assert_eq!(operation.request_sha256, attempt.request_sha256);
            work.verify(&operation).await.unwrap();
            let replay = work.execute(&operation).await.unwrap();
            assert!(replay.idempotent_replay);
            assert_eq!(replay.receipt_sha256, attempt.result_sha256);
        }
        let physical = work.client.read_storage_diagnostics(&work.session,
            ReadStorageDiagnostics {scope: format!("instance:{}",work.client.instance_id())},
            RequestOptions::read("smoke-storage", "smoke-storage").unwrap()).await.unwrap();
        assert!(physical.rrflow_kv.is_some() && physical.admission.is_some());
        let mut final_states = Vec::new();
        for slot in 0..3 {
            for generation in 0..=slot+2 {
                let operation = work.prepare_load(format!("smoke-slot-{slot}-{generation}"),
                    WriteTransition { slot, generation }).unwrap();
                let receipt = work.execute(&operation).await.unwrap();
                assert!(!receipt.idempotent_replay);
                work.verify(&operation).await.unwrap();
                if generation == slot+2 { final_states.push((operation, receipt.receipt_sha256)); }
            }
        }
        let write = work
            .prepare(
                "smoke-concurrent-write".into(),
                StorageLoadKind::MixedWrite,
                99,
            )
            .unwrap();
        let retained_reader = reader::HeldReader::open(&work, 0, clock, None).await.unwrap();
        let mut management = management::ManagementTask::new();
        let (release_receipt, held_receipt) = tokio::sync::oneshot::channel();
        let backup_work = work.clone();
        management.start(Instant::now() + Duration::from_secs(30), async move {
            let receipt = lifecycle::backup(backup_work, "smoke-backup".into(), clock).await?;
            // Hold only the verified receipt handoff, not engine execution. This
            // makes the owned pending-task boundary deterministic in native CI.
            held_receipt.await.map_err(|_| "smoke receipt handoff cancelled".to_owned())?;
            Ok(receipt)
        }).unwrap();
        let written = work.execute(&write).await.unwrap();
        assert!(management.try_finish().unwrap().is_none());
        let followup = work.prepare("smoke-management-followup".into(), StorageLoadKind::PointRead, 1).unwrap();
        work.execute(&followup).await.unwrap();
        work.verify(&followup).await.unwrap();
        release_receipt.send(()).unwrap();
        let backup = management.finish().await.unwrap().unwrap();
        assert!(!management.is_pending());
        let reader_evidence = retained_reader.finish(&work, clock).await.unwrap();
        work.verify(&write).await.unwrap();
        let selected = reader::HeldReader::open(
            &work, 1, clock, Some(&ReaderSelection::WorkloadRecords {}),
        ).await.unwrap();
        let selected_write = work.prepare("smoke-selected-write".into(), StorageLoadKind::MixedWrite, 101).unwrap();
        let selected_receipt = work.execute(&selected_write).await.unwrap();
        work.verify(&selected_write).await.unwrap();
        let selected_evidence = selected.finish(&work, clock).await.unwrap();
        assert!(selected_evidence.opened.rows > 0);
        println!(
            "selected_reader_interference={}",
            serde_json::to_string(&selected_evidence).unwrap()
        );
        let killed = super::super::kill(child, clock);
        let (reopened, client, identity) = estate.start_for(clock, Duration::from_secs(300));
        work.client = client;
        // Exercise the production post-reopen bounded readback before replay.
        let mut reopened_digest =
            digest("c07g.reopened-corpus", &(work.seed, work.records)).unwrap();
        let mut expected_digest = reopened_digest.clone();
        for first in (0..work.records).step_by(2) {
            let (next_digest, values) = corpus::verify_reopened_window(
                &work,
                first,
                (first + 2).min(work.records),
                2,
                Duration::from_secs(30),
                reopened_digest,
            )
            .await
            .unwrap();
            for (ordinal, effect) in values {
                assert_eq!(effect, work.seed_record(ordinal).unwrap().effect_sha256);
                expected_digest =
                    digest("c07g.reopened-corpus", &(expected_digest, ordinal, &effect)).unwrap();
            }
            reopened_digest = next_digest;
        }
        assert_eq!(reopened_digest, expected_digest);
        // Every old value is read before replay; exact old request identities
        // must return the stored receipt after this real process boundary.
        for ordinal in 0..work.records {
            let operation = work.seed_record(ordinal).unwrap();
            work.verify(&operation).await.unwrap();
            let replay = work.execute(&operation).await.unwrap();
            assert!(replay.idempotent_replay);
            assert_eq!(replay.receipt_sha256, receipts[ordinal as usize]);
        }
        for (operation, original) in &final_states {
            work.verify(operation).await.unwrap();
            let replay = work.execute(operation).await.unwrap();
            assert!(replay.idempotent_replay);
            assert_eq!(&replay.receipt_sha256, original);
        }
        work.verify(&write).await.unwrap();
        let replay = work.execute(&write).await.unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(replay.receipt_sha256, written.receipt_sha256);
        work.verify(&selected_write).await.unwrap();
        let selected_replay = work.execute(&selected_write).await.unwrap();
        assert!(selected_replay.idempotent_replay);
        assert_eq!(selected_replay.receipt_sha256, selected_receipt.receipt_sha256);
        println!(
            "installed_capacity_workload_smoke={}",
            serde_json::json!({
            "build":estate.build,"records":8,"edges":24,"vectors":16,"dimensions":16,
            "replayed_transactions":12,"write_transitions":{"insert":3,"update":4,"retire":3,"recreate":2},"backup_sha256":backup.receipt.backup.backup_sha256,
            "management_followup_verified":true,"reader_interference":reader_evidence,"termination":killed,"reopened":identity,
            "interrupted_acknowledged_writes":acknowledged.len(),"interrupted_sample_cancelled":gate.dropped.load(std::sync::atomic::Ordering::SeqCst),
            "capacity_qualification":false})
        );
        super::super::kill(reopened, clock);
    });
}
