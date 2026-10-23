//! Refresh synthetic contract projections after an intentional contract change.
//! This does not run a workload or produce native storage qualification.
use rrflow_eval::storage::{
    catalogue_digest, manifest_digest, schema, validate_result, CampaignResult, CandidateCiReceipt,
    EvidenceClass, FleetResult, FleetTrialResult, ProgressEvent, StorageAcceptanceManifest,
    StorageAcceptanceReport, StorageCampaign, StorageLaunchReceipt, StorageManifest, StorageResult,
    SustainedSegment, ValidationReport,
};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

fn write(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    // Value produces the same sorted projection as the checked-in goldens.
    let mut bytes = serde_json::to_vec_pretty(&serde_json::to_value(value)?)?;
    bytes.push(b'\n');
    std::fs::write(path, bytes)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/fixtures/storage");
    let mut source: Value =
        serde_json::from_slice(&std::fs::read(fixtures.join("manifest.json"))?)?;
    let configuration = rrd_contract::EstateConfiguration::default();
    source["candidate"]["configuration_sha256"] = configuration.configuration_sha256.clone().into();
    source["candidate"]["public_catalogue_sha256"] = serde_json::to_value(catalogue_digest()?)?;
    source["installed_configuration"] = serde_json::to_value(configuration)?;
    let manifest: StorageManifest = serde_json::from_value(source)?;
    if !matches!(manifest.evidence_class, EvidenceClass::Synthetic) {
        return Err("only the synthetic contract fixture may be refreshed".into());
    }
    let mut result: StorageResult =
        serde_json::from_slice(&std::fs::read(fixtures.join("result.json"))?)?;
    result.manifest_sha256 = manifest_digest(&manifest)?;
    let report = validate_result(&manifest, &result)?;
    write(&fixtures.join("manifest.json"), &manifest)?;
    write(&fixtures.join("result.json"), &result)?;
    write(&fixtures.join("report.json"), &report)?;
    let schemas = root.join("../../../eval/schemas");
    write(
        &schemas.join("storage-manifest-v1.json"),
        &schema::<StorageManifest>(),
    )?;
    write(
        &schemas.join("storage-result-v1.json"),
        &schema::<StorageResult>(),
    )?;
    write(
        &schemas.join("storage-report-v1.json"),
        &schema::<ValidationReport>(),
    )?;
    write(
        &schemas.join("storage-campaign-v1.json"),
        &schema::<StorageCampaign>(),
    )?;
    write(
        &schemas.join("storage-segment-v1.json"),
        &schema::<SustainedSegment>(),
    )?;
    write(
        &schemas.join("storage-campaign-result-v1.json"),
        &schema::<CampaignResult>(),
    )?;
    write(
        &schemas.join("storage-progress-event-v1.json"),
        &schema::<ProgressEvent>(),
    )?;
    write(
        &schemas.join("storage-fleet-result-v1.json"),
        &schema::<FleetResult>(),
    )?;
    write(
        &schemas.join("storage-fleet-trial-v1.json"),
        &schema::<FleetTrialResult>(),
    )?;
    write(
        &schemas.join("storage-acceptance-manifest-v1.json"),
        &schema::<StorageAcceptanceManifest>(),
    )?;
    write(
        &schemas.join("storage-acceptance-report-v1.json"),
        &schema::<StorageAcceptanceReport>(),
    )?;
    write(
        &schemas.join("storage-candidate-ci-v1.json"),
        &schema::<CandidateCiReceipt>(),
    )?;
    write(
        &schemas.join("storage-launch-v1.json"),
        &schema::<StorageLaunchReceipt>(),
    )?;
    println!("refreshed synthetic storage contract projections; no native workload executed");
    Ok(())
}
