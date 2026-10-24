//! The primary executable must explain its own identity without a checkout.
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::process::Command;

fn sha256(path: &Path) -> String {
    let mut input = std::fs::File::open(path).unwrap();
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = input.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn relocated_binary_reports_identity_without_git_or_runtime_overrides() {
    let directory = tempfile::tempdir().unwrap();
    let original = Path::new(env!("CARGO_BIN_EXE_rrflow"));
    let relocated = directory.path().join(original.file_name().unwrap());
    std::fs::copy(original, &relocated).unwrap();
    let output = Command::new(&relocated)
        .args(["--json", "version"])
        .current_dir(directory.path())
        .env("PATH", "")
        .env(
            "RRFLOW_BUILD_MANIFEST",
            "runtime-spoof-must-not-be-read.json",
        )
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["product_version"], "1.0.0");
    let build = &report["build"];
    assert_eq!(build["format_version"], 1);
    assert_eq!(build["executable_sha256"], sha256(&relocated));
    assert!(build["compiler"]["rustc"]
        .as_str()
        .unwrap()
        .starts_with("rustc "));
    assert!(!build["compiler"]["target"].as_str().unwrap().is_empty());
    assert_eq!(build["public_contract_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(build["signal_catalogue_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(build["metadata_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(build["qualification"], "unqualified");
    assert_eq!(
        build["compiler"]["debug_assertions"],
        cfg!(debug_assertions)
    );
    assert!(build.get("provenance").is_some());
    let second = Command::new(&relocated)
        .args(["--json", "version"])
        .current_dir(directory.path())
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(output.stdout, second.stdout, "identity must be stable");
}
