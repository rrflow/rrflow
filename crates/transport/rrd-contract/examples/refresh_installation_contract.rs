//! Refresh default-policy projections using the owning typed digest functions.
//! No installation, engine workload, or qualification evidence is produced.
use rrd_contract::{
    installation_plan_sha256, installation_result_sha256, EstateConfiguration, InstallationPlan,
    InstallationResult,
};
use serde_json::Value;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/install-attunement-v1.json");
    let mut fixture: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    fixture["installation_plan"]["configuration"] =
        serde_json::to_value(EstateConfiguration::default())?;
    let mut plan: InstallationPlan = serde_json::from_value(fixture["installation_plan"].clone())?;
    plan.plan_sha256 = installation_plan_sha256(&plan)?;
    plan.validate()?;
    let mut result: InstallationResult =
        serde_json::from_value(fixture["installation_result"].clone())?;
    result.plan_sha256 = plan.plan_sha256.clone();
    result.result_sha256 = installation_result_sha256(&result)?;
    result.validate_for(&plan)?;
    fixture["installation_plan"] = serde_json::to_value(plan)?;
    fixture["installation_result"] = serde_json::to_value(result)?;
    let mut bytes = serde_json::to_vec_pretty(&fixture)?;
    bytes.push(b'\n');
    std::fs::write(path, bytes)?;
    let public_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/public-contract-v1.json");
    let mut public: Value = serde_json::from_slice(&std::fs::read(&public_path)?)?;
    public["service"]["configuration"] = serde_json::to_value(EstateConfiguration::default())?;
    let mut bytes = serde_json::to_vec_pretty(&public)?;
    bytes.push(b'\n');
    std::fs::write(public_path, bytes)?;
    println!("refreshed default installation contract projection; no engine workload executed");
    Ok(())
}
