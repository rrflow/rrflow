use super::*;
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

fn read(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    require(
        file.metadata().map_err(|e| e.to_string())?.is_file(),
        "evidence input must be a regular file",
    )?;
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    require(
        bytes.len() as u64 <= MAX_DOCUMENT_BYTES,
        "document exceeds 8 MiB bound",
    )?;
    Ok(bytes)
}

fn pretty<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    require(
        bytes.len() as u64 <= MAX_DOCUMENT_BYTES,
        "output exceeds 8 MiB bound",
    )?;
    Ok(bytes)
}

/// Offline, bounded, no model/service invocation and no implicit overwrite.
pub fn run(args: &[String]) -> Result<()> {
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["schema", kind] => {
            let generated = match *kind {
                "manifest" => schema::<StorageManifest>(),
                "result" => schema::<StorageResult>(),
                "report" => schema::<ValidationReport>(),
                "campaign" => schema::<StorageCampaign>(),
                "segment" => schema::<SustainedSegment>(),
                "campaign-result" => schema::<CampaignResult>(),
                "progress-event" => schema::<ProgressEvent>(),
                "fleet-result" => schema::<FleetResult>(),
                "fleet-trial" => schema::<FleetTrialResult>(),
                "acceptance-manifest" => schema::<StorageAcceptanceManifest>(),
                "acceptance-report" => schema::<StorageAcceptanceReport>(),
                "candidate-ci" => schema::<CandidateCiReceipt>(),
                "launch" => schema::<StorageLaunchReceipt>(),
                _ => return Err("unknown storage schema projection".into()),
            };
            std::io::stdout()
                .lock()
                .write_all(&pretty(&generated)?)
                .map_err(|e| e.to_string())
        }
        ["campaign", "profiles"] => {
            std::io::stdout().lock().write_all(&pretty(&capacity_profiles()?)?)
                .map_err(|e| e.to_string())
        }
        ["campaign", "validate", path] => {
            let campaign: StorageCampaign = decode(&read(Path::new(path))?)?;
            if campaign.allocation.estates.get()>1 {validate_fleet(&campaign)?;}
            println!("valid storage campaign: {} purpose={:?}; no workload executed",
                campaign.sha256()?.as_str(), campaign.purpose);
            Ok(())
        }
        ["campaign", "verify", directory] => {
            let campaign: StorageCampaign = decode(&read(&Path::new(directory).join("campaign.json"))?)?;
            if campaign.allocation.estates.get()>1 {
                let result=verify_fleet(Path::new(directory))?;
                std::io::stdout().lock().write_all(&pretty(&result)?).map_err(|e|e.to_string())?;
                require(result.status==CampaignStatus::Completed,"fleet failed; raw evidence retained")?;
                return require(campaign.purpose!=CampaignPurpose::Qualification || result.qualification_passed,"fleet completed without qualification");
            }
            let result = verify_campaign(Path::new(directory))?;
            std::io::stdout().lock().write_all(&pretty(&result)?)
                .map_err(|e| e.to_string())?;
            require(result.status == CampaignStatus::Completed,
                "campaign did not complete; failure evidence retained")?;
            let campaign: StorageCampaign = decode(&read(&Path::new(directory).join("campaign.json"))?)?;
            require(campaign.sha256()? == result.campaign_sha256,
                "campaign changed after evidence verification")?;
            require(campaign.purpose != CampaignPurpose::Qualification || result.qualification_passed,
                "campaign completed but qualification requirements were not met")
        }
        ["acceptance", "ci-receipt", directory] => {
            let root=Path::new(directory);
            let receipt=candidate_ci_from_api(&read(&root.join("run.json"))?,&read(&root.join("jobs.json"))?,&read(&root.join("commit.json"))?)?;
            let mut output=OpenOptions::new().write(true).create_new(true).open(root.join("receipt.json")).map_err(|e|e.to_string())?;
            output.write_all(&pretty(&receipt)?).and_then(|()|output.sync_all()).map_err(|e|e.to_string())?;
            println!("CI receipt: {}",digest("rrflow-storage-ci-receipt-v1",&receipt)?.as_str());
            Ok(())
        }
        ["acceptance", "verify", root, manifest_path, report_path] => {
            let manifest:StorageAcceptanceManifest=decode(&read(Path::new(manifest_path))?)?;
            let report=verify_storage_acceptance(Path::new(root),&manifest)?;
            let mut output=OpenOptions::new().write(true).create_new(true).open(report_path).map_err(|e|e.to_string())?;
            output.write_all(&pretty(&report)?).and_then(|()|output.sync_all()).map_err(|e|e.to_string())?;
            require(report.ready_for_owner_review,"C-07 evidence is incomplete or unqualified; retained report names coverage and failures")
        }
        ["validate", path] => {
            let manifest: StorageManifest = decode(&read(Path::new(path))?)?;
            manifest.validate()?;
            println!(
                "valid storage manifest: {} evidence={:?}; validation does not enforce runtime budgets",
                manifest_digest(&manifest)?.as_str(),
                manifest.evidence_class
            );
            Ok(())
        }
        ["verify", manifest_path, result_path, report_path] => {
            let manifest: StorageManifest = decode(&read(Path::new(manifest_path))?)?;
            let result: StorageResult = decode(&read(Path::new(result_path))?)?;
            let report = validate_result(&manifest, &result)?;
            let bytes = pretty(&report)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(report_path)
                .map_err(|e| format!("report requires a new file: {e}"))?;
            output.write_all(&bytes)
                .and_then(|()| output.sync_all())
                .map_err(|e| e.to_string())?;
            require(
                report.contract_passed,
                "storage evidence failed; bounded failure report retained",
            )?;
            println!(
                "storage contract passed: {} cases; evidence={:?}; not alpha or release qualification",
                report.passed,
                report.evidence_class
            );
            Ok(())
        }
        _ => Err("usage: rrflow-eval storage schema <manifest|result|report|campaign|segment|campaign-result|progress-event|fleet-result|fleet-trial|acceptance-manifest|acceptance-report|candidate-ci|launch> | validate <manifest> | verify <manifest> <result> <new-report> | campaign profiles | campaign validate <campaign> | campaign verify <directory> | acceptance ci-receipt <directory> | acceptance verify <root> <manifest> <new-report>".into()),
    }
}
