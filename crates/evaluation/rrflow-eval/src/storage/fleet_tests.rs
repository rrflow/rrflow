use super::*;
fn fixture() -> StorageCampaign {
    let mut campaign = super::super::sustained::tests::fixture().0;
    campaign.allocation.estates = 2u64.try_into().unwrap();
    campaign.storage.budgets.estates = campaign.allocation.estates;
    campaign.allocation.engine_bytes = campaign.allocation.host_bytes;
    campaign.storage.budgets.process_bytes = campaign.allocation.engine_bytes;
    campaign.storage.budgets.aggregate_process_bytes = Some(campaign.allocation.engine_bytes);
    campaign.allocation.records = 16u64.try_into().unwrap();
    campaign.allocation.edges = 48u64.try_into().unwrap();
    campaign.allocation.vectors = 32u64.try_into().unwrap();
    campaign.points[0].clients = 8u64.try_into().unwrap();
    campaign.points[0].requests_per_second = 20u64.try_into().unwrap();
    campaign.evidence.total_bytes = (512 * 1024 * 1024u64).try_into().unwrap();
    campaign
}

#[test]
fn canonical_ten_estate_profile_admits_one_shared_pool_and_denies_overcommit() {
    let projection = capacity_profiles().unwrap();
    let profile = projection
        .profiles
        .iter()
        .find(|p| p.id == "fleet-isolation")
        .unwrap();
    let mut campaign = fixture();
    campaign.purpose = CampaignPurpose::Qualification;
    campaign.storage.profile = profile.id.clone().try_into().unwrap();
    campaign.allocation = CapacityAllocation {
        owner_projection_sha256: projection.owner_projection_sha256,
        vcpus: profile.vcpus.try_into().unwrap(),
        host_bytes: profile.host_bytes.try_into().unwrap(),
        engine_bytes: profile.engine_bytes.try_into().unwrap(),
        estates: profile.estates.try_into().unwrap(),
        records: profile.records_min.try_into().unwrap(),
        edges: profile.edges_min.try_into().unwrap(),
        vectors: profile.vectors_min.try_into().unwrap(),
        vector_dimensions: profile.vector_dimensions.try_into().unwrap(),
        logical_source_bytes: profile.logical_source_bytes_min.try_into().unwrap(),
    };
    campaign.storage.budgets.estates = campaign.allocation.estates;
    campaign.storage.budgets.process_bytes = campaign.allocation.engine_bytes;
    campaign.storage.budgets.aggregate_process_bytes = Some(campaign.allocation.engine_bytes);
    campaign.storage.budgets.host_bytes = campaign.allocation.host_bytes;
    campaign.storage.budgets.queue_requests = 64u64.try_into().unwrap();
    campaign.points[0].clients = 40u64.try_into().unwrap();
    campaign.points[0].requests_per_second = 50u64.try_into().unwrap();
    campaign.schedule.warmup_ms = 300_000u64.try_into().unwrap();
    campaign.schedule.measured_ms = 1_800_000u64.try_into().unwrap();
    campaign.schedule.soak_ms = 86_400_000u64.try_into().unwrap();
    campaign.schedule.forced_reopens = 100u64.try_into().unwrap();
    campaign.seeds = vec![7, 8, 9];
    campaign.required_lanes = vec![
        CapacityLane::Cold,
        CapacityLane::Warm,
        CapacityLane::BackupUnderWrite,
        CapacityLane::ReaderPinInterference,
        CapacityLane::ForcedReopens,
    ];
    campaign.evidence.total_bytes = (4 * 1024 * 1024 * 1024u64).try_into().unwrap();
    validate_fleet(&campaign).unwrap();
    let child = fleet_child(&campaign, 0, 0, FleetTrial::Equal, 9).unwrap();
    assert_eq!(
        child.storage.budgets.aggregate_process_bytes,
        campaign.storage.budgets.aggregate_process_bytes
    );
    assert_eq!(
        child.allocation.engine_bytes,
        campaign.allocation.engine_bytes
    );
    for case in 0..4 {
        let mut invalid = campaign.clone();
        match case {
            0 => invalid.storage.budgets.aggregate_process_bytes = None,
            1 => {
                invalid.storage.budgets.aggregate_process_bytes =
                    Some((16 * 1024 * 1024 * 1024u64).try_into().unwrap())
            }
            2 => {
                invalid.storage.budgets.retained_bytes =
                    (4 * 1024 * 1024 * 1024u64).try_into().unwrap()
            }
            _ => {
                invalid.storage.budgets.host_bytes = (4 * 1024 * 1024 * 1024u64).try_into().unwrap()
            }
        }
        assert!(
            validate_fleet(&invalid).is_err(),
            "invalid pool case {case}"
        );
    }
}
#[test]
fn children_are_exact_mechanical_corpus_rate_seed_and_aggregate_views() {
    let parent = fixture();
    validate_fleet(&parent).unwrap();
    for trial in FleetTrial::ALL {
        for estate in 0..2 {
            let child = fleet_child(&parent, 0, 0, trial, estate).unwrap();
            assert_eq!(child.allocation.records.get(), 8);
            assert_eq!(child.allocation.edges.get(), 24);
            assert_eq!(child.allocation.vectors.get(), 16);
            assert_eq!(
                child.allocation.engine_bytes,
                parent.allocation.engine_bytes
            );
            assert_eq!(child.points[0].clients.get(), 4);
            assert_eq!(
                child.points[0].requests_per_second.get(),
                if trial == FleetTrial::Abusive && estate == 0 {
                    20
                } else {
                    10
                }
            );
            assert_eq!(child.seeds, vec![parent.seeds[0] + estate]);
            assert_eq!(child.purpose, CampaignPurpose::Characterization);
            assert_eq!(
                trial.active(estate),
                trial != FleetTrial::Quiet || estate == 0
            );
        }
    }
}
#[test]
fn fleet_denies_fractional_coordinates_starved_evidence_and_unknown_members() {
    for mutation in 0..6 {
        let mut parent = fixture();
        match mutation {
            0 => parent.allocation.records = 17u64.try_into().unwrap(),
            1 => parent.points[0].clients = 7u64.try_into().unwrap(),
            2 => parent.points[0].requests_per_second = 21u64.try_into().unwrap(),
            3 => parent.evidence.total_bytes = (32 * 1024 * 1024u64).try_into().unwrap(),
            4 => parent.read_distribution = ReadKeyDistribution::HotKey { ordinal: 8 },
            _ => parent.seeds = vec![MAX_EXACT_INTEGER],
        }
        assert!(
            fleet_child(&parent, 0, 0, FleetTrial::Equal, 1).is_err(),
            "accepted mutation {mutation}"
        );
    }
    assert!(fleet_child(&fixture(), 0, 0, FleetTrial::Equal, 2).is_err());
}
#[test]
fn incomplete_fleet_is_retained_failed_and_cannot_be_promoted_by_result_flag() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("fleet");
    let writer = FleetWriter::create(&root, fixture()).unwrap();
    let report = writer.finish(CampaignStatus::Failed).unwrap();
    assert!(!report.qualification_passed);
    assert_eq!(verify_fleet(&root).unwrap().status, CampaignStatus::Failed);
    let mut changed = report;
    changed.status = CampaignStatus::Completed;
    std::fs::write(
        root.join("fleet-result.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    assert!(verify_fleet(&root).is_err());
}
#[test]
fn complete_trial_requires_real_child_results_and_boundary_coverage() {
    let parent = fixture();
    let temp = tempfile::tempdir().unwrap();
    let mut trial = FleetTrialResult {
        format_version: 1,
        campaign_sha256: parent.sha256().unwrap(),
        index: 0,
        point: 0,
        seed: 0,
        trial: FleetTrial::Quiet,
        status: CampaignStatus::Completed,
        child_results: vec![],
        boundaries: vec![],
        sum_member_peak_resident_bytes: 0,
        measured_latency: vec![],
        qualification_passed: true,
    };
    assert!(assess_fleet_trial(temp.path(), &parent, &mut trial).is_err());
    trial.status = CampaignStatus::Failed;
    assess_fleet_trial(temp.path(), &parent, &mut trial).unwrap();
    assert!(!trial.qualification_passed);
}
#[test]
fn boundaries_require_distinct_engines_explicit_denial_and_real_reopen() {
    let parent = fixture();
    let sample = super::super::sustained::tests::fixture()
        .1
        .resources
        .remove(0);
    let mut boundaries = Vec::new();
    for (stage_index, stage) in [
        "warmup", "measured", "burst", "drain", "soak", "loaded", "reopened",
    ]
    .iter()
    .enumerate()
    {
        for estate in 0..2 {
            let mut sample = sample.clone();
            sample.pid = 1 + estate as u32 + if *stage == "reopened" { 2 } else { 0 };
            sample.elapsed_ns = 1 + stage_index as u64;
            boundaries.push(FleetBoundary {
                estate,
                stage: stage.to_string().try_into().unwrap(),
                instance: format!("estate-{estate}"),
                foreign_instance: format!("estate-{}", (estate + 1) % 2),
                denial: rrd_contract::ErrorCode::Unauthenticated,
                sample,
            });
        }
    }
    let valid = FleetTrialResult {
        format_version: 1,
        campaign_sha256: parent.sha256().unwrap(),
        index: 0,
        point: 0,
        seed: 0,
        trial: FleetTrial::Quiet,
        status: CampaignStatus::Completed,
        child_results: vec![],
        boundaries,
        sum_member_peak_resident_bytes: 0,
        measured_latency: vec![],
        qualification_passed: false,
    };
    check_boundaries(&parent, &valid).unwrap();
    for mutation in 0..5 {
        let mut wrong = valid.clone();
        match mutation {
            0 => {
                wrong.boundaries.pop();
            }
            1 => wrong.boundaries[1].sample.pid = 1,
            2 => wrong.boundaries[0].foreign_instance = "estate-0".into(),
            3 => wrong.boundaries[12].sample.pid = 1,
            _ => wrong.boundaries[2].sample.elapsed_ns = 1,
        }
        assert!(
            check_boundaries(&parent, &wrong).is_err(),
            "accepted boundary mutation {mutation}"
        );
    }
}
