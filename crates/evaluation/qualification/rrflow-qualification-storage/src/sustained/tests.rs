use super::*;
fn limit(value: u64) -> Limit {
    value.try_into().unwrap()
}
#[test]
fn reader_selection_is_explicit_and_bound_to_campaign_identity() {
    let (mut legacy, _) = fixture();
    legacy
        .required_lanes
        .push(CapacityLane::ReaderPinInterference);
    let original = serde_json::to_value(&legacy).unwrap();
    assert!(original.get("reader_selection").is_none());
    let original_digest = legacy.sha256().unwrap();
    let mut selected = original.clone();
    selected["reader_selection"] = serde_json::json!({"kind": "workload_records"});
    let selected: StorageCampaign = serde_json::from_value(selected).unwrap();
    assert_ne!(selected.sha256().unwrap(), original_digest);
    let old_roundtrip: StorageCampaign = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(serde_json::to_value(&old_roundtrip).unwrap(), original);
    assert_eq!(old_roundtrip.sha256().unwrap(), original_digest);
    let mut absent_lane = selected.clone();
    absent_lane
        .required_lanes
        .retain(|lane| *lane != CapacityLane::ReaderPinInterference);
    assert!(absent_lane.validate().is_err());
    let mut bad = original;
    bad["reader_selection"] =
        serde_json::json!({"kind": "workload_records", "record_kind": "other"});
    assert!(serde_json::from_value::<StorageCampaign>(bad).is_err());
}

#[test]
fn idle_segments_reserve_probe_time_and_respect_sample_capacity() {
    let (mut campaign, _) = fixture();
    campaign.storage.budgets.run_ms = limit(120_000);
    campaign.schedule.operation_deadline_ms = limit(30_000);
    campaign.schedule.sample_ms = limit(1000);
    campaign.evidence.samples_per_segment = limit(256);
    assert_eq!(campaign.idle_segment_ms(120_000).unwrap(), 90_000);
    assert_eq!(campaign.idle_segment_ms(20_000).unwrap(), 20_000);
    campaign.evidence.samples_per_segment = limit(4);
    assert_eq!(campaign.idle_segment_ms(120_000).unwrap(), 2000);
    campaign.evidence.samples_per_segment = limit(2);
    assert!(campaign.idle_segment_ms(120_000).is_err());
    campaign.evidence.samples_per_segment = limit(256);
    campaign.schedule.operation_deadline_ms = limit(120_000);
    assert!(campaign.idle_segment_ms(120_000).is_err());
    campaign.schedule.operation_deadline_ms = limit(30_000);
    assert!(campaign.idle_segment_ms(0).is_err());
}

pub(crate) fn fixture() -> (StorageCampaign, SustainedSegment) {
    // Contract fixtures deliberately do not represent native observations.
    let mut manifest: StorageManifest =
        serde_json::from_str(include_str!("../../tests/fixtures/storage/manifest.json")).unwrap();
    manifest.evidence_class = EvidenceClass::NativeCapacity;
    manifest.budgets.estates = limit(1);
    let mut result: StorageResult =
        serde_json::from_str(include_str!("../../tests/fixtures/storage/result.json")).unwrap();
    result.manifest_sha256 = manifest_digest(&manifest).unwrap();
    let campaign = StorageCampaign {
        format_version: 1,
        purpose: CampaignPurpose::Characterization,
        allocation: CapacityAllocation {
            owner_projection_sha256: digest("fixture", &1).unwrap(),
            vcpus: limit(1),
            host_bytes: manifest.budgets.host_bytes,
            engine_bytes: manifest.budgets.process_bytes,
            estates: manifest.budgets.estates,
            records: limit(1),
            edges: limit(1),
            vectors: limit(1),
            vector_dimensions: limit(1),
            logical_source_bytes: limit(1),
        },
        schedule: SustainedSchedule {
            setup_ms: limit(1000),
            lifecycle_ms: limit(1000),
            warmup_ms: limit(1000),
            measured_ms: limit(1000),
            burst_ms: limit(1000),
            drain_ms: limit(1000),
            soak_ms: limit(1000),
            sample_ms: limit(1000),
            operation_deadline_ms: limit(1000),
            forced_reopens: limit(1),
        },
        seeds: vec![7],
        read_distribution: ReadKeyDistribution::Uniform {},
        drain: super::super::DrainPolicy {
            maximum_mutable_growth_bytes: 0,
            maximum_projected_growth_bytes: 0,
            maximum_debt_segments: 0,
            stable_samples: limit(2),
        },
        points: vec![OperatingPoint {
            id: "one".to_owned().try_into().unwrap(),
            clients: limit(1),
            requests_per_second: limit(1),
            maximum_error_basis_points: 0,
            maximum_p99_ns: limit(1000),
        }],
        required_lanes: vec![CapacityLane::Warm],
        evidence: EvidenceLimits {
            segment_attempts: limit(100),
            segment_bytes: limit(MAX_DOCUMENT_BYTES),
            total_bytes: limit(32 * 1024 * 1024),
            segments: limit(100),
            samples_per_segment: limit(100),
        },
        reader_selection: None,
        storage: manifest.clone(),
    };
    let observed = NativeMetric::Observed { value: 0 };
    let sample = ResourceSample {
        elapsed_ns: 0,
        pid: 1,
        resident_bytes: observed.clone(),
        private_bytes: observed.clone(),
        mapped_bytes: observed.clone(),
        host_resident_bytes: observed.clone(),
        read_bytes: observed.clone(),
        written_bytes: observed.clone(),
        cpu_time_ns: observed.clone(),
        retained_bytes: observed.clone(),
        queue_requests: observed.clone(),
        maintenance_debt_segments: observed,
        allocation_enforcement: AllocationEnforcement::ObservedOnly,
        allocation_receipt: None,
        storage: Some(manifest.storage_observation.clone()),
    };
    let recorded = result.cases[0].ledger.attempts[0].clone();
    let AttemptOutcome::Acknowledged { receipt_sha256 } = recorded.outcome else {
        panic!("ACK fixture")
    };
    let segment = SustainedSegment {
        format_version: 1,
        campaign_sha256: campaign.sha256().unwrap(),
        index: 0,
        previous_sha256: None,
        point: campaign.points[0].id.clone(),
        seed: 7,
        phase: LoadPhase::Measured,
        started_ns: 0,
        finished_ns: 1_000_000_000,
        corpus: None,
        cold_start: None,
        reopens: None,
        backup_started_ns: None,
        backup: None,
        reader: None,
        manifest,
        result,
        attempts: vec![LoadAttempt {
            write: None,
            sequence: 0,
            kind: StorageLoadKind::PointRead,
            scheduled_ns: 0,
            started_ns: recorded.started_ns,
            verification_finished_ns: None,
            finished_ns: recorded.finished_ns,
            outcome: LoadOutcome::Acknowledged,
            logical_id: recorded.logical_id,
            request_sha256: recorded.request_sha256,
            result_sha256: receipt_sha256,
        }],
        resources: vec![sample],
        lanes: vec![],
    };
    (campaign, segment)
}
#[test]
fn verification_timing_is_optional_but_cannot_precede_response_or_escape_segment() {
    let (campaign, mut segment) = fixture();
    let original = encode_bounded(&segment, MAX_DOCUMENT_BYTES).unwrap();
    assert!(!String::from_utf8(original.clone())
        .unwrap()
        .contains("verification_finished_ns"));
    let historical: SustainedSegment = decode(&original).unwrap();
    assert!(historical.attempts[0].verification_finished_ns.is_none());
    validate_segment(&campaign, &historical).unwrap();
    segment.attempts[0].verification_finished_ns = Some(segment.attempts[0].finished_ns);
    validate_segment(&campaign, &segment).unwrap();
    segment.attempts[0].verification_finished_ns = Some(segment.finished_ns);
    validate_segment(&campaign, &segment).unwrap();
    for invalid in [segment.attempts[0].finished_ns - 1, segment.finished_ns + 1] {
        segment.attempts[0].verification_finished_ns = Some(invalid);
        assert!(validate_segment(&campaign, &segment)
            .unwrap_err()
            .contains("verification completion"));
    }
}

#[test]
fn short_run_is_characterization_and_failed_measurement_survives() {
    let (campaign, mut segment) = fixture();
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path().join("evidence");
    let mut writer = CampaignWriter::create(&directory, campaign.clone()).unwrap();
    writer.append(&segment).unwrap();
    segment.index = writer.next_index();
    segment.previous_sha256 = writer.previous_sha256();
    segment.started_ns = 1_000_000_000;
    segment.finished_ns = 2_000_000_000;
    segment.attempts[0].sequence = 1;
    segment.attempts[0].scheduled_ns = 1_000_000_000;
    segment.attempts[0].started_ns += 1_000_000_000;
    segment.attempts[0].finished_ns += 1_000_000_000;
    segment.resources[0].elapsed_ns = 1_000_000_000;
    segment.resources[0].resident_bytes = NativeMetric::Observed {
        value: campaign.allocation.engine_bytes.get() + 1,
    };
    assert!(writer.append(&segment).unwrap_err().contains("RSS"));
    let result = writer.finish(CampaignStatus::Failed, vec![]).unwrap();
    assert!(!result.qualification_passed);
    assert!(result.failed_segment.is_some());
    assert_eq!(
        verify_campaign(&directory).unwrap().status,
        CampaignStatus::Failed
    );
    let mut changed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("failed-segment.json")).unwrap())
            .unwrap();
    changed["resources"][0]["resident_bytes"]["value"] = 0.into();
    std::fs::write(
        directory.join("failed-segment.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    assert!(verify_campaign(&directory).is_err());
}

#[test]
fn drain_requires_consecutive_ownership_recovery_and_exact_baseline() {
    use super::super::{DrainOwnership, DrainSample};
    let (campaign, segment) = fixture();
    let owner = DrainOwnership {
        checkpoint_entries: 0,
        native_transactions: 0,
        native_transaction_bytes: 0,
        active_read_views: 0,
        pinned_read_bytes: 0,
        mutable_bytes: 10,
        projected_bytes: 20,
        debt_segments: 0,
        outstanding_requests: 0,
        outstanding_bytes: 0,
        queued_requests: 0,
        active: false,
        fenced: false,
    };
    let mut warmup = validate_segment(&campaign, &segment).unwrap();
    warmup.phase = LoadPhase::Warmup;
    warmup.ownership_baseline = Some(DrainSample {
        elapsed_ns: 0,
        ownership: Some(owner.clone()),
    });
    let mut drain = warmup.clone();
    drain.phase = LoadPhase::Drain;
    drain.ownership_baseline = None;
    drain.started_ns = warmup.finished_ns;
    drain.finished_ns = drain.started_ns + 10;
    drain.first_sequence = 0;
    drain.next_sequence = 0;
    drain.schedule_origin_ns = None;
    drain.drain_tail = vec![
        DrainSample {
            elapsed_ns: 1,
            ownership: Some(owner.clone()),
        },
        DrainSample {
            elapsed_ns: 2,
            ownership: Some(owner),
        },
    ];
    let check = |row: &OperatingPointResult| {
        super::super::drain::check(&campaign, &[&warmup, row], &row.point, row.seed)
    };
    check(&drain).unwrap();
    for field in 0..13 {
        let mut bad = drain.clone();
        let value = bad.drain_tail[0].ownership.as_mut().unwrap();
        match field {
            0 => value.mutable_bytes += 1,
            1 => value.projected_bytes += 1,
            2 => value.debt_segments = 1,
            3 => value.outstanding_requests = 1,
            4 => value.outstanding_bytes = 1,
            5 => value.queued_requests = 1,
            6 => value.active = true,
            7 => value.fenced = true,
            8 => value.active_read_views = 1,
            9 => value.pinned_read_bytes = 1,
            10 => value.native_transactions = 1,
            11 => value.native_transaction_bytes = 1,
            _ => value.checkpoint_entries = 1,
        }
        assert!(
            check(&bad).is_err(),
            "failed first sample cannot be hidden by settled last sample"
        );
    }
    let mut missing = drain.clone();
    missing.drain_tail[0].ownership = None;
    assert!(check(&missing).is_err());
    missing.drain_tail.remove(0);
    assert!(check(&missing).is_err());
    missing = drain.clone();
    missing.drain_tail[1].elapsed_ns = missing.drain_tail[0].elapsed_ns;
    assert!(check(&missing).is_err());
    assert!(super::super::drain::check(&campaign, &[&drain], &drain.point, drain.seed).is_err());
    let mut totals = BTreeMap::new();
    merge_summary(&mut totals, drain.clone(), 2).unwrap();
    drain.started_ns = drain.finished_ns;
    drain.finished_ns += 1;
    drain.drain_tail[0].elapsed_ns = 3;
    drain.drain_tail[1].elapsed_ns = 4;
    merge_summary(&mut totals, drain, 2).unwrap();
    let tail = &totals.values().next().unwrap().drain_tail;
    assert_eq!(tail.len(), 2);
    assert_eq!(tail[0].elapsed_ns, 3);
}

#[test]
fn drain_configuration_and_preload_observation_are_frozen() {
    let (mut campaign, mut segment) = fixture();
    let original = campaign.sha256().unwrap();
    campaign.drain.maximum_mutable_growth_bytes = 1;
    assert_ne!(original, campaign.sha256().unwrap());
    campaign.drain.stable_samples = limit(65);
    assert!(campaign.validate().is_err());
    campaign.drain.stable_samples = limit(1);
    assert!(campaign.validate().is_err());
    campaign.drain.stable_samples = limit(3);
    assert!(
        campaign.validate().is_err(),
        "window exceeds frozen drain time"
    );
    campaign.drain.stable_samples = limit(2);
    segment.campaign_sha256 = campaign.sha256().unwrap();
    segment.phase = LoadPhase::Warmup;
    segment.resources[0].elapsed_ns = segment.attempts[0].started_ns + 1;
    assert!(validate_segment(&campaign, &segment)
        .unwrap_err()
        .contains("baseline"));
}

#[test]
fn read_distribution_is_frozen_bounded_and_selects_the_requested_region() {
    let uniform = ReadKeyDistribution::Uniform {};
    let hot = ReadKeyDistribution::HotKey { ordinal: 99 };
    let skew = ReadKeyDistribution::HotSet {
        keys: limit(4),
        traffic_basis_points: 2500,
    };
    let all_hot = ReadKeyDistribution::HotSet {
        keys: limit(4),
        traffic_basis_points: 10_000,
    };
    let mut uniform_keys = BTreeSet::new();
    let mut hot_reads = 0;
    for sequence in 0..1024 {
        let ordinal = uniform.ordinal(73, sequence, 100).unwrap();
        assert!(ordinal < 100);
        uniform_keys.insert(ordinal);
        assert_eq!(hot.ordinal(73, sequence, 100).unwrap(), 99);
        assert!(all_hot.ordinal(73, sequence, 100).unwrap() < 4);
        let ordinal = skew.ordinal(73, sequence, 100).unwrap();
        assert!(ordinal < 100);
        hot_reads += usize::from(ordinal < 4);
    }
    assert_eq!(uniform_keys.len(), 100);
    assert!((180..=330).contains(&hot_reads));
    for invalid in [
        ReadKeyDistribution::HotKey { ordinal: 100 },
        ReadKeyDistribution::HotSet {
            keys: limit(101),
            traffic_basis_points: 2500,
        },
        ReadKeyDistribution::HotSet {
            keys: limit(100),
            traffic_basis_points: 9999,
        },
        ReadKeyDistribution::HotSet {
            keys: limit(1),
            traffic_basis_points: 0,
        },
        ReadKeyDistribution::HotSet {
            keys: limit(1),
            traffic_basis_points: 10001,
        },
    ] {
        assert!(invalid.ordinal(0, 0, 100).is_err());
    }
    assert!(uniform.ordinal(0, 0, 0).is_err());
    let (mut campaign, _) = fixture();
    let original = campaign.sha256().unwrap();
    campaign.read_distribution = ReadKeyDistribution::HotKey { ordinal: 0 };
    assert_ne!(campaign.sha256().unwrap(), original);
    assert!(decode::<ReadKeyDistribution>(br#"{"kind":"uniform","keys":1}"#).is_err());
}
#[test]
fn missing_sequence_and_changed_offered_schedule_are_rejected() {
    let (campaign, segment) = fixture();
    let mut totals = BTreeMap::new();
    merge_summary(
        &mut totals,
        validate_segment(&campaign, &segment).unwrap(),
        2,
    )
    .unwrap();
    let mut repeated = validate_segment(&campaign, &segment).unwrap();
    assert!(merge_summary(&mut totals, repeated.clone(), 2).is_err());
    repeated.first_sequence = 1;
    repeated.next_sequence = 2;
    repeated.schedule_origin_ns = Some(1);
    assert!(merge_summary(&mut totals, repeated, 2).is_err());
    let mut changed = segment.clone();
    changed.attempts[0].finished_ns = 1001;
    changed.result.cases[0].ledger.attempts[0].finished_ns = 1001;
    assert_eq!(
        validate_segment(&campaign, &changed)
            .unwrap()
            .latency_over_budget,
        1
    );
}

#[test]
fn cold_lane_requires_bounded_native_residency_and_matching_raw_receipt() {
    let (campaign, mut segment) = fixture();
    segment.phase = LoadPhase::Warmup;
    let cold = ColdStartEvidence {
        started_ns: 0,
        evicted_ns: 0,
        ready_ns: 0,
        finished_ns: 0,
        files: 2,
        file_bytes: 4097,
        pages: 2,
        resident_pages: 0,
        page_bytes: 4096,
        inventory_sha256: digest("fixture-inventory", &1).unwrap(),
        readback_sha256: digest("fixture-readback", &1).unwrap(),
    };
    let bind = |segment: &mut SustainedSegment, cold: ColdStartEvidence| {
        segment.lanes = vec![LaneEvidence {
            lane: CapacityLane::Cold,
            repetitions: limit(1),
            receipt_sha256: digest("c07g.cold-start", &cold).unwrap(),
            acknowledged_readback_verified: true,
        }];
        segment.cold_start = Some(cold);
    };
    bind(&mut segment, cold.clone());
    validate_segment(&campaign, &segment).unwrap();
    segment.cold_start = None;
    assert!(validate_segment(&campaign, &segment)
        .unwrap_err()
        .contains("residency"));
    for invalid in 0..6 {
        let mut changed = cold.clone();
        match invalid {
            0 => changed.resident_pages = 1,
            1 => {
                changed.files = campaign
                    .storage
                    .installed_configuration
                    .storage
                    .recovery_files
                    + 1
            }
            2 => {
                changed.file_bytes = campaign
                    .storage
                    .installed_configuration
                    .storage
                    .recovery_read_bytes
                    + 1
            }
            3 => changed.evicted_ns = 1,
            4 => changed.pages = 10,
            _ => changed.page_bytes = 0,
        }
        bind(&mut segment, changed);
        assert!(validate_segment(&campaign, &segment)
            .unwrap_err()
            .contains("cold lane"));
    }
    bind(&mut segment, cold);
    segment.lanes[0].receipt_sha256 = digest("changed", &1).unwrap();
    assert!(validate_segment(&campaign, &segment)
        .unwrap_err()
        .contains("cold lane"));
}

#[test]
fn campaign_completion_requires_final_readback_and_raw_cold_reconciliation() {
    use super::super::progress::{verify_progress, ProgressJournal};
    let (mut campaign, mut segment) = fixture();
    campaign.required_lanes.push(CapacityLane::Cold);
    let point = campaign.points[0].id.clone();
    let seed = campaign.seeds[0];
    let file_identity = digest("native-file-fixture", &1).unwrap();
    let cold = ColdStartEvidence {
        started_ns: 0,
        evicted_ns: 1,
        ready_ns: 2,
        finished_ns: 3,
        files: 1,
        file_bytes: 4097,
        pages: 2,
        resident_pages: 0,
        page_bytes: 4096,
        inventory_sha256: digest(
            "c07g.cold-inventory",
            &(
                digest("c07g.cold-inventory", &seed).unwrap(),
                &file_identity,
            ),
        )
        .unwrap(),
        readback_sha256: digest("readback", &1).unwrap(),
    };
    let file = ProgressEvent::ColdFile {
        point: point.clone(),
        seed,
        file_identity_sha256: file_identity,
        bytes: 4097,
        page_bytes: 4096,
        pages: 2,
        resident_pages: 0,
    };
    let observed = ProgressEvent::ColdStart {
        point: point.clone(),
        seed,
        observation: cold.clone(),
    };
    let final_readback = ProgressEvent::FinalReadback {
        point,
        seed,
        killed_pid: 1,
        reopened_pid: 2,
        seed_records: campaign.allocation.records.get(),
        working_slots: 0,
        states_sha256: digest("states", &1).unwrap(),
        started_ns: 10,
        finished_ns: 20,
    };
    let write = |events: &[ProgressEvent]| {
        let directory = tempfile::tempdir().unwrap();
        let mut journal =
            ProgressJournal::create(directory.path(), &campaign.sha256().unwrap()).unwrap();
        for event in events {
            journal.append(event, MAX_DOCUMENT_BYTES).unwrap();
        }
        (journal.finish().unwrap(), directory)
    };
    let events = vec![file.clone(), observed.clone(), final_readback.clone()];
    let (receipt, directory) = write(&events);
    let mut audit = verify_progress(
        directory.path(),
        &receipt,
        &campaign,
        CampaignStatus::Completed,
    )
    .unwrap();
    assert!(audit.require_bound(CampaignStatus::Completed).is_err());
    assert!(audit
        .check_readback_order(&segment.point, segment.seed, 21)
        .is_err());
    segment.finished_ns = 10;
    segment.cold_start = Some(cold.clone());
    audit.bind_segment(&segment).unwrap();
    audit.require_bound(CampaignStatus::Completed).unwrap();
    assert!(audit.bind_segment(&segment).is_err());

    for omission in 0..3 {
        let mut missing = events.clone();
        missing.remove(omission);
        let (receipt, directory) = write(&missing);
        assert!(verify_progress(
            directory.path(),
            &receipt,
            &campaign,
            CampaignStatus::Completed
        )
        .is_err());
    }
    let (receipt, directory) = write(std::slice::from_ref(&file));
    verify_progress(
        directory.path(),
        &receipt,
        &campaign,
        CampaignStatus::Interrupted,
    )
    .unwrap();
    for invalid in 0..6 {
        let mut changed = events.clone();
        match invalid {
            0 => {
                let ProgressEvent::ColdFile { resident_pages, .. } = &mut changed[0] else {
                    unreachable!()
                };
                *resident_pages = 1;
            }
            1 => {
                let ProgressEvent::ColdStart { observation, .. } = &mut changed[1] else {
                    unreachable!()
                };
                observation.file_bytes += 1;
            }
            2 => {
                let ProgressEvent::ColdStart { observation, .. } = &mut changed[1] else {
                    unreachable!()
                };
                observation.inventory_sha256 = digest("wrong", &1).unwrap();
            }
            3 => changed.push(final_readback.clone()),
            4 => {
                let ProgressEvent::FinalReadback { seed_records, .. } = &mut changed[2] else {
                    unreachable!()
                };
                *seed_records = 0;
            }
            _ => changed.push(ProgressEvent::Failure {
                reason: "failed-after-readback".to_owned().try_into().unwrap(),
            }),
        }
        let (receipt, directory) = write(&changed);
        assert!(verify_progress(
            directory.path(),
            &receipt,
            &campaign,
            CampaignStatus::Completed
        )
        .is_err());
    }
}
fn bind_desktop(campaign: &mut StorageCampaign) {
    let projection = capacity_profiles().unwrap();
    let profile = &projection.profiles[0];
    assert_eq!(profile.id, "desktop");
    campaign.storage.profile = profile.id.clone().try_into().unwrap();
    campaign.allocation = CapacityAllocation {
        owner_projection_sha256: projection.owner_projection_sha256,
        vcpus: limit(profile.vcpus),
        host_bytes: limit(profile.host_bytes),
        engine_bytes: limit(profile.engine_bytes),
        estates: limit(profile.estates),
        records: limit(profile.records_min),
        edges: limit(profile.edges_min),
        vectors: limit(profile.vectors_min),
        vector_dimensions: limit(profile.vector_dimensions),
        logical_source_bytes: limit(profile.logical_source_bytes_min),
    };
    campaign.storage.budgets.host_bytes = campaign.allocation.host_bytes;
    campaign.storage.budgets.process_bytes = campaign.allocation.engine_bytes;
    campaign.storage.budgets.estates = campaign.allocation.estates;
    campaign.points[0].requests_per_second = limit(profile.rates[0]);
}

#[test]
fn qualification_binds_owning_allocation_corpus_and_load_coordinates() {
    let (mut campaign, _) = fixture();
    bind_desktop(&mut campaign);
    super::super::capacity::validate_capacity_profile(&campaign).unwrap();
    for mutation in 0..12 {
        let mut invalid = campaign.clone();
        match mutation {
            0 => invalid.allocation.owner_projection_sha256 = digest("wrong", &0).unwrap(),
            1 => invalid.storage.profile = "unknown".to_owned().try_into().unwrap(),
            2 => invalid.allocation.vcpus = limit(1),
            3 => invalid.allocation.host_bytes = limit(1),
            4 => invalid.allocation.engine_bytes = limit(1),
            5 => invalid.allocation.estates = limit(2),
            6 => invalid.allocation.records = limit(1),
            7 => invalid.allocation.edges = limit(1),
            8 => invalid.allocation.vectors = limit(1),
            9 => invalid.allocation.vector_dimensions = limit(1),
            10 => invalid.allocation.logical_source_bytes = limit(1),
            _ => invalid.points[0].requests_per_second = limit(1),
        }
        assert!(
            super::super::capacity::validate_capacity_profile(&invalid).is_err(),
            "accepted changed owning coordinate {mutation}"
        );
    }
    campaign.points[0].clients = limit(2);
    assert!(super::super::capacity::validate_capacity_profile(&campaign).is_err());
    campaign.storage.profile = "fleet-isolation".to_owned().try_into().unwrap();
    assert!(super::super::capacity::validate_capacity_profile(&campaign)
        .unwrap_err()
        .contains("allocation differs"));
    // Smaller arbitrary characterization remains representable, without qualification.
    fixture().0.validate().unwrap();
}

#[test]
fn observed_only_shortened_or_omitted_work_cannot_qualify() {
    let (mut campaign, segment) = fixture();
    campaign.purpose = CampaignPurpose::Qualification;
    assert!(campaign.validate().is_err());
    campaign.schedule.warmup_ms = limit(300_000);
    campaign.schedule.measured_ms = limit(1_800_000);
    campaign.schedule.soak_ms = limit(86_400_000);
    campaign.schedule.forced_reopens = limit(100);
    campaign.seeds = vec![1, 2, 3];
    campaign.required_lanes = all_lanes().into_iter().collect();
    bind_desktop(&mut campaign);
    campaign.validate().unwrap();
    let mut result = CampaignResult {
        format_version: 1,
        campaign_sha256: campaign.sha256().unwrap(),
        status: CampaignStatus::Completed,
        segments: 15,
        last_segment_sha256: None,
        artifact_bytes: 0,
        progress: ProgressReceipt {
            bytes: 0,
            records: 1,
            sha256: digest("fixture", &1).unwrap(),
        },
        failed_segment: None,
        operating_points: vec![],
        limitations: vec![],
        qualification_passed: false,
    };
    for seed in &campaign.seeds {
        for phase in phases() {
            let mut row = validate_segment(&fixture().0, &segment).unwrap();
            row.seed = *seed;
            row.phase = phase;
            row.elapsed_ns = campaign.schedule.duration_ms(phase) * NANOSECONDS_PER_MS;
            row.missing_metrics.clear();
            row.lanes = all_lanes().into_iter().collect();
            row.forced_reopens = 100;
            if phase == LoadPhase::Warmup {
                row.corpus = Some(CorpusEvidence {
                    records: campaign.allocation.records.get(),
                    anchor_records: 1,
                    edges: campaign.allocation.edges.get(),
                    vectors: campaign.allocation.vectors.get(),
                    vector_dimensions: campaign.allocation.vector_dimensions.get(),
                    logical_source_bytes: campaign.allocation.logical_source_bytes.get(),
                    encoded_mutation_bytes: 2048,
                    verified_records: campaign.allocation.records.get(),
                    started_ns: 0,
                    finished_ns: 0,
                    input_sha256: digest("fixture", &1).unwrap(),
                    acknowledgement_sha256: digest("fixture", &2).unwrap(),
                    readback_sha256: digest("fixture", &3).unwrap(),
                });
            }
            row.acknowledged_transitions = [1; 4];
            row.attempts = if phase == LoadPhase::Drain {
                0
            } else {
                campaign.schedule.duration_ms(phase) / 1000
                    * campaign.points[0].requests_per_second.get()
                    * if phase == LoadPhase::Burst { 2 } else { 1 }
            };
            result.operating_points.push(row);
        }
    }
    assert!(qualifies(&campaign, &result).unwrap());
    let before_soak = result
        .operating_points
        .iter()
        .filter(|row| row.phase != LoadPhase::Soak)
        .cloned()
        .collect::<Vec<_>>();
    assert!(satisfies_point_requirements(
        &campaign,
        &campaign.points[0],
        1,
        before_soak.iter(),
        &phases()[..4]
    )
    .unwrap());
    let mut incomplete = result.clone();
    incomplete.operating_points = before_soak.clone();
    assert!(
        !qualifies(&campaign, &incomplete).unwrap(),
        "admitted preflight is not a completed soak"
    );
    for mutation in 0..9 {
        let mut rows = before_soak.clone();
        match mutation {
            0 => {
                rows.remove(0);
            }
            1 => rows[0].elapsed_ns -= 1,
            2 => rows[0].failures = 1,
            3 => rows[0].latency_over_budget = rows[0].attempts / 100 + 1,
            4 => rows[0].unresolved = 1,
            5 => rows[0].missing_metrics.push(
                "native-allocation-enforcement"
                    .to_owned()
                    .try_into()
                    .unwrap(),
            ),
            6 => rows[0].corpus = None,
            7 => {
                for row in &mut rows {
                    row.acknowledged_transitions = [0; 4];
                }
            }
            _ => {
                for row in &mut rows {
                    row.lanes.clear();
                }
            }
        }
        assert!(
            !satisfies_point_requirements(
                &campaign,
                &campaign.points[0],
                1,
                rows.iter(),
                &phases()[..4]
            )
            .unwrap(),
            "accepted invalid preflight {mutation}"
        );
    }
    let temporary = tempfile::tempdir().unwrap();
    let writer =
        CampaignWriter::create(&temporary.path().join("qualification"), campaign.clone()).unwrap();
    assert!(writer
        .admit_soak(&campaign.points[0].id, 1)
        .unwrap_err()
        .contains("prerequisite"));
    assert!(writer
        .admit_soak(&campaign.points[0].id, 999)
        .unwrap_err()
        .contains("seed"));
    let diagnostic =
        CampaignWriter::create(&temporary.path().join("characterization"), fixture().0).unwrap();
    assert!(diagnostic.admit_soak(&campaign.points[0].id, 1).is_ok());
    result.operating_points[0].attempts = 1;
    assert!(!qualifies(&campaign, &result).unwrap());
    result.operating_points[0].attempts = 1500;
    result.operating_points[0].latency_over_budget = 16;
    assert!(!qualifies(&campaign, &result).unwrap());
    result.operating_points[0].latency_over_budget = 0;
    result.operating_points[0].missing_metrics.push(
        "native-allocation-enforcement"
            .to_owned()
            .try_into()
            .unwrap(),
    );
    assert!(!qualifies(&campaign, &result).unwrap());
}
#[test]
fn evidence_admission_precedes_filesystem_effect() {
    let (mut campaign, _) = fixture();
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path().join("no-effect");
    campaign.evidence.total_bytes = limit(MAX_DOCUMENT_BYTES);
    assert!(CampaignWriter::create(&directory, campaign).is_err());
    assert!(!directory.exists());
}

#[test]
fn working_set_transitions_preserve_identity_and_repeat_retirement_cycles() {
    assert_eq!(
        (0..8)
            .map(|generation| WriteTransition {
                slot: 7,
                generation
            }
            .counter_index())
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 1, 2, 3, 1]
    );
    assert!(WriteTransition {
        slot: 7,
        generation: 5
    }
    .retired());
    assert!(!WriteTransition {
        slot: 7,
        generation: 6
    }
    .retired());
    let (campaign, mut segment) = fixture();
    segment.attempts[0].write = Some(WriteTransition {
        slot: 0,
        generation: 0,
    });
    assert!(validate_segment(&campaign, &segment)
        .unwrap_err()
        .contains("write transition"));
}

#[test]
fn corpus_receipt_requires_full_readback_and_frozen_setup_budget() {
    let (campaign, _) = fixture();
    let mut corpus = CorpusEvidence {
        records: 1,
        anchor_records: 1,
        edges: 1,
        vectors: 1,
        vector_dimensions: 1,
        logical_source_bytes: 1024,
        encoded_mutation_bytes: 2048,
        verified_records: 1,
        started_ns: 0,
        finished_ns: 1_000_000_000,
        input_sha256: digest("fixture", &1).unwrap(),
        acknowledgement_sha256: digest("fixture", &2).unwrap(),
        readback_sha256: digest("fixture", &3).unwrap(),
    };
    assert!(corpus.validate(&campaign, 1_000_000_000).is_ok());
    corpus.verified_records = 0;
    assert!(corpus.validate(&campaign, 1_000_000_000).is_err());
    corpus.verified_records = 1;
    corpus.finished_ns += 1;
    assert!(corpus.validate(&campaign, 2_000_000_000).is_err());
    corpus.finished_ns = 0;
    corpus.edges = 0;
    assert!(corpus.validate(&campaign, 1_000_000_000).is_err());
}

#[test]
fn reopen_receipt_denies_missing_ack_and_changed_replay() {
    let (campaign, _) = fixture();
    let hash = digest("fixture", &1).unwrap();
    let mut proof = ReopenEvidence {
        started_ns: 0,
        finished_ns: 1000,
        corpus_records_verified: 1,
        corpus_readback_sha256: hash.clone(),
        cycles: vec![ReopenObservation {
            cycle: 0,
            killed_pid: 1,
            reopened_pid: 2,
            killed_ns: 100,
            ready_ns: 200,
            finished_ns: 300,
            checked_operations: 1,
            expected_sha256: hash.clone(),
            original_receipts_sha256: hash.clone(),
            replay_receipts_sha256: hash.clone(),
            readback_sha256: hash,
        }],
    };
    assert!(proof.validate(&campaign, 1000).is_ok());
    proof.cycles[0].checked_operations = 0;
    assert!(proof.validate(&campaign, 1000).is_err());
    proof.cycles[0].checked_operations = 1;
    proof.cycles[0].replay_receipts_sha256 = digest("different", &1).unwrap();
    assert!(proof.validate(&campaign, 1000).is_err());
}

#[test]
fn native_allocation_claim_requires_matching_retained_receipt() {
    let (campaign, mut segment) = fixture();
    let receipt = NativeAllocationReceipt {
        format_version: 1,
        kind: NativeAllocationKind::WindowsJob,
        nonce: digest("nonce", &1).unwrap(),
        engine_bytes: campaign.allocation.engine_bytes,
        host_bytes: campaign.allocation.host_bytes,
        harness_sha256: digest("harness", &1).unwrap(),
        vcpus: campaign.allocation.vcpus,
        host_logical_cpus: limit(8),
    };
    segment.resources[0].allocation_enforcement = receipt.enforcement().unwrap();
    assert!(validate_segment(&campaign, &segment).is_err());
    segment.resources[0].allocation_receipt = Some(receipt.clone());
    assert!(validate_segment(&campaign, &segment).is_ok());
    segment.resources[0]
        .allocation_receipt
        .as_mut()
        .unwrap()
        .engine_bytes = limit(1);
    assert!(validate_segment(&campaign, &segment).is_err());
    segment.resources[0].allocation_receipt = Some(receipt);
    segment.resources[0].allocation_enforcement = AllocationEnforcement::CgroupV2 {
        receipt_sha256: digest("different", &1).unwrap(),
    };
    assert!(validate_segment(&campaign, &segment).is_err());
}
