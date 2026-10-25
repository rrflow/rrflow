//! Refresh only the SDK corpus identities from the actual read-only planner.
//! No installation, engine startup or native workload is executed.
use rrd_contract::{InstallationTargetKind, SdkConformanceCorpus};
use rrd_engine::RrdEngine;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let path = root.join("fixtures/rrd-sdk-conformance-v1.json");
    let bytes = std::fs::read(&path)?;
    let mut corpus: SdkConformanceCorpus = serde_json::from_slice(&bytes)?;
    corpus.validate()?;
    let anchor = std::fs::read(root.join("fixtures/rrflow-sdk-conformance-install-anchor-v1.bin"))?;
    let temporary = tempfile::tempdir()?;
    let executable = temporary.path().join("install-anchor.bin");
    std::fs::write(&executable, &anchor)?;
    let project = temporary.path().join("project");
    std::fs::create_dir(&project)?;
    let mut identity = String::with_capacity(64);
    for byte in Sha256::digest(&anchor) {
        write!(&mut identity, "{byte:02x}")?;
    }
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &identity,
        "default",
        None,
        &executable,
    )?;
    if std::fs::read_dir(&project)?.next().is_some() {
        return Err("read-only planner mutated the project".into());
    }
    corpus.identity.instance = preview.installation.target.instance_id;
    corpus.identity.estate = preview.installation.target.estate_id;
    let scope = format!("instance:{}", corpus.identity.instance);
    corpus.query.scope.clone_from(&scope);
    corpus.vector.ensure.scope.clone_from(&scope);
    corpus.vector.search.scope.clone_from(&scope);
    corpus.changefeed.read.scope = scope;
    corpus.validate()?;
    if std::env::args().skip(1).any(|arg| arg == "--check") {
        let original: SdkConformanceCorpus = serde_json::from_slice(&bytes)?;
        if corpus != original {
            return Err("SDK installation identity projection drift".into());
        }
    } else {
        // Preserve unrelated authored field order and fixture semantics.
        let mut original: serde_json::Value = serde_json::from_slice(&bytes)?;
        let generated = serde_json::to_value(corpus)?;
        for pointer in [
            "/identity/instance",
            "/identity/estate",
            "/query/scope",
            "/vector/ensure/scope",
            "/vector/search/scope",
            "/changefeed/read/scope",
        ] {
            *original
                .pointer_mut(pointer)
                .ok_or("missing fixture identity field")? = generated
                .pointer(pointer)
                .ok_or("missing generated identity field")?
                .clone();
        }
        let mut encoded = serde_json::to_vec_pretty(&original)?;
        encoded.push(b'\n');
        std::fs::write(path, encoded)?;
    }
    println!("SDK installation identity projection synchronized; no installation executed");
    Ok(())
}
