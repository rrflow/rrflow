use super::*;
use serde_json::{json, Value};
fn api_fixture() -> (Value, Value, Value) {
    let tested = "2".repeat(40);
    let head = "1".repeat(40);
    let run = json!({"id":7,"run_attempt":2,"head_sha":head,"status":"completed","conclusion":"success","event":"pull_request","path":".github/workflows/ci.yml","repository":{"full_name":"rrflow/rrflow-development"},"referenced_workflows":[{"sha":tested,"path":format!("rrflow/rrflow-development/.github/workflows/ci-reusable.yml@{tested}")}]});
    let mut jobs = Vec::new();
    let mut add = |name: String, steps: &[&str]| {
        jobs.push(json!({"id":jobs.len()+1,"run_id":7,"run_attempt":2,"head_sha":head,"name":name,"status":"completed","conclusion":"success","steps":steps.iter().map(|name|json!({"name":name,"status":"completed","conclusion":"success"})).collect::<Vec<_>>()}));
    };
    add("pipeline / ci-gate".into(), &[]);
    add("pipeline / installed RRFlow lifecycle".into(), &[]);
    for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        add(
            format!("pipeline / installed domain crash ({os})"),
            &[
                "Test exact installed domain boundaries and compound recovery",
                "Qualify installed native capacity failure and recovery",
            ],
        );
        add(
            format!("pipeline / installed 100-cycle reopen ({os})"),
            &[
                "Require both complete ordinary-primary receipts",
                "Require fresh and existing installed damage denial",
                "Exercise installed capacity workload and backup readback",
                "Independent installed estates under concurrent load",
            ],
        );
    }
    for os in ["macos-latest", "windows-latest"] {
        add(format!("pipeline / estate process ({os})"), &[]);
    }
    (
        run,
        json!(jobs),
        json!({"sha":tested,"tree":{"sha":"3".repeat(40)}}),
    )
}
fn receipt(run: &Value, jobs: &Value, commit: &Value) -> Result<CandidateCiReceipt> {
    candidate_ci_from_api(
        &serde_json::to_vec(run).unwrap(),
        &serde_json::to_vec(jobs).unwrap(),
        &serde_json::to_vec(commit).unwrap(),
    )
}
#[test]
fn raw_ci_requires_exact_tested_merge_and_every_native_step() {
    let (run, jobs, commit) = api_fixture();
    let ci = receipt(&run, &jobs, &commit).unwrap();
    assert_ne!(ci.head_commit, ci.tested_commit);
    for mutation in 0..6 {
        let (mut run, mut jobs, mut commit) = (run.clone(), jobs.clone(), commit.clone());
        match mutation {
            0 => run["conclusion"] = json!("failure"),
            1 => jobs[2]["steps"][1]["conclusion"] = json!("skipped"),
            2 => jobs[0]["run_attempt"] = json!(1),
            3 => commit["sha"] = run["head_sha"].clone(),
            4 => {
                jobs.as_array_mut().unwrap().pop();
            }
            _ => run["referenced_workflows"] = json!([]),
        }
        assert!(
            receipt(&run, &jobs, &commit).is_err(),
            "accepted CI mutation {mutation}"
        );
    }
}
fn acceptance_fixture(root: &Path) -> StorageAcceptanceManifest {
    let (run, jobs, commit) = api_fixture();
    let ci = receipt(&run, &jobs, &commit).unwrap();
    let ci_root = root.join("ci");
    std::fs::create_dir(&ci_root).unwrap();
    for (name, value) in [
        ("run.json", run),
        ("jobs.json", jobs),
        ("commit.json", commit),
    ] {
        std::fs::write(ci_root.join(name), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    std::fs::write(
        ci_root.join("receipt.json"),
        serde_json::to_vec(&ci).unwrap(),
    )
    .unwrap();
    let source = AcceptanceSource {
        commit: ci.tested_commit.clone(),
        tree: ci.tested_tree.clone(),
        public_catalogue_sha256: catalogue_digest().unwrap(),
    };
    let mut campaign = super::super::sustained::tests::fixture().0;
    campaign.storage.profile = "desktop".to_owned().try_into().unwrap();
    campaign.allocation.estates = 1u64.try_into().unwrap();
    campaign.storage.budgets.estates = campaign.allocation.estates;
    campaign.storage.candidate.commit = source.commit.clone();
    campaign.storage.candidate.tree = source.tree.clone();
    let campaign_root = root.join("desktop");
    std::fs::create_dir(&campaign_root).unwrap();
    std::fs::write(
        campaign_root.join("campaign.json"),
        serde_json::to_vec(&campaign).unwrap(),
    )
    .unwrap();
    StorageAcceptanceManifest {
        format_version: 1,
        source,
        owner_projection_sha256: capacity_profiles().unwrap().owner_projection_sha256,
        ci_directory: "ci".to_owned().try_into().unwrap(),
        ci_receipt_sha256: digest("rrflow-storage-ci-receipt-v1", &ci).unwrap(),
        campaigns: vec![AcceptanceCampaign {
            directory: "desktop".to_owned().try_into().unwrap(),
            campaign_sha256: campaign.sha256().unwrap(),
            result_sha256: None,
            launch_sha256: None,
            status: CampaignStatus::NotRun,
            absence_reason: Some("not-run".to_owned().try_into().unwrap()),
        }],
    }
}
#[test]
fn exact_ci_does_not_turn_absent_profiles_or_soak_into_acceptance() {
    let temp = tempfile::tempdir().unwrap();
    let manifest = acceptance_fixture(temp.path());
    let report = verify_storage_acceptance(temp.path(), &manifest).unwrap();
    assert!(report.candidate_ci_passed);
    assert!(!report.matrix_complete);
    assert!(!report.capacity_qualified);
    assert!(!report.ready_for_owner_review);
    assert_eq!(report.points[0].status, CampaignStatus::NotRun);
    assert!(!report.points[0].evidence_verified);
    assert!(report
        .profiles
        .iter()
        .all(|p| !p.missing_coordinates.is_empty()));
}
#[test]
fn acceptance_denies_source_drift_hidden_results_and_forged_ci_projection() {
    for mutation in 0..4 {
        let temp = tempfile::tempdir().unwrap();
        let mut manifest = acceptance_fixture(temp.path());
        match mutation {
            0 => manifest.source.commit = "1".repeat(40),
            1 => std::fs::write(temp.path().join("desktop/result.json"), b"{}").unwrap(),
            2 => {
                let p = temp.path().join("ci/receipt.json");
                let mut ci: CandidateCiReceipt =
                    serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
                ci.run_id = 8u64.try_into().unwrap();
                manifest.ci_receipt_sha256 = digest("rrflow-storage-ci-receipt-v1", &ci).unwrap();
                std::fs::write(p, serde_json::to_vec(&ci).unwrap()).unwrap();
            }
            _ => manifest.campaigns.push(manifest.campaigns[0].clone()),
        }
        assert!(
            verify_storage_acceptance(temp.path(), &manifest).is_err(),
            "accepted mutation {mutation}"
        );
    }
    assert!(Id::try_from("../outside".to_owned()).is_err());
}
#[test]
fn coverage_keeps_failed_knee_and_reports_only_measured_envelope() {
    let mut point = AcceptedOperatingPoint {
        directory: "desktop".to_owned().try_into().unwrap(),
        profile: "desktop".to_owned().try_into().unwrap(),
        platform: NativePlatform::LinuxExt4,
        clients: 1,
        requests_per_second: 25,
        status: CampaignStatus::Completed,
        qualification_passed: true,
        evidence_verified: true,
        launch_status: Some(CampaignStatus::Completed),
        launch_cleanup_confirmed: true,
        launch_reason: None,
        absence_reason: None,
    };
    let mut points = vec![point.clone()];
    point.requests_per_second = 50;
    point.status = CampaignStatus::Failed;
    point.qualification_passed = false;
    points.push(point);
    let report = derive_coverage(&points).unwrap();
    let desktop = &report[0];
    assert_eq!(desktop.attempted_points, 2);
    assert_eq!(desktop.qualified_points, 1);
    assert_eq!(desktop.largest_qualified_rate, 25);
    assert_eq!(desktop.qualified_envelopes[0].clients, 1);
    assert_eq!(desktop.qualified_envelopes[0].maximum_tested_rate, 25);
    assert!(!desktop.missing_coordinates.is_empty());
}
