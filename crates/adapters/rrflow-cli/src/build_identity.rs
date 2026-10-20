//! Local executable observations. No Git, checkout, environment provenance, or
//! external service is consulted at runtime.
use crate::build_metadata::{
    BuildMetadata, BuildProvenance, CompilerIdentity, MAX_BUILD_MANIFEST_BYTES,
};
use rrd_engine::digest;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;

#[derive(Debug, Serialize)]
pub struct BuildReport {
    pub format_version: u16,
    pub qualification: &'static str,
    pub compiler: CompilerIdentity,
    pub provenance: Option<BuildProvenance>,
    pub metadata_sha256: String,
    pub executable_sha256: String,
    pub public_contract_sha256: String,
    pub signal_catalogue_sha256: String,
}

pub fn report() -> Result<BuildReport, Box<dyn std::error::Error>> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/rrflow-build.json"));
    if bytes.len() as u64 > MAX_BUILD_MANIFEST_BYTES {
        return Err("embedded build identity exceeds budget".into());
    }
    let metadata: BuildMetadata = serde_json::from_slice(bytes)?;
    if let Some(provenance) = &metadata.provenance {
        provenance.validate()?;
    }
    let mut file = std::fs::File::open(std::env::current_exe()?)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    let executable_sha256 = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    let openapi = rrd_contract::openapi_document()?;
    let signal_catalogue_sha256 = openapi["x-rrd-signal-catalogue-sha256"]
        .as_str()
        .ok_or("generated OpenAPI lacks its signal identity")?
        .to_owned();
    Ok(BuildReport {
        format_version: 1,
        // Source capture alone is not release, bundle or semantic-parity evidence.
        qualification: "unqualified",
        compiler: metadata.compiler,
        provenance: metadata.provenance,
        metadata_sha256: digest::sha256_hex(bytes),
        executable_sha256,
        public_contract_sha256: digest::sha256_hex(&serde_json::to_vec(&openapi)?),
        signal_catalogue_sha256,
    })
}
