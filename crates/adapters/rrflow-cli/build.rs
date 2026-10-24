#[path = "src/build_metadata.rs"]
mod build_metadata;

use build_metadata::{BuildMetadata, BuildProvenance, CompilerIdentity, MAX_BUILD_MANIFEST_BYTES};
use std::{env, fs, io::Read, path::PathBuf, process::Command};

fn required(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("missing Cargo build input {name}"))
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/build_metadata.rs");
    println!("cargo:rerun-if-env-changed=RRFLOW_BUILD_MANIFEST");
    let output = Command::new(required("RUSTC"))
        .arg("--version")
        .output()
        .expect("read compiler identity");
    assert!(output.status.success(), "compiler identity failed");
    let compiler = CompilerIdentity {
        rustc: String::from_utf8(output.stdout)
            .expect("UTF-8 rustc version")
            .trim()
            .into(),
        target: required("TARGET"),
        root_profile: required("PROFILE"),
        opt_level: required("OPT_LEVEL"),
        panic: required("CARGO_CFG_PANIC"),
        debug_assertions: env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some(),
    };
    let provenance = env::var_os("RRFLOW_BUILD_MANIFEST").map(|path| {
        let path = PathBuf::from(path);
        assert!(
            path.is_absolute(),
            "build manifest must have an absolute path"
        );
        println!("cargo:rerun-if-changed={}", path.display());
        let mut bytes = Vec::new();
        fs::File::open(&path)
            .expect("open declared build manifest")
            .take(MAX_BUILD_MANIFEST_BYTES + 1)
            .read_to_end(&mut bytes)
            .expect("read declared build manifest");
        assert!(
            bytes.len() as u64 <= MAX_BUILD_MANIFEST_BYTES,
            "build manifest exceeds budget"
        );
        let provenance: BuildProvenance =
            serde_json::from_slice(&bytes).expect("invalid declared build manifest");
        provenance.validate().expect("invalid build provenance");
        assert_eq!(
            provenance.rustc, compiler.rustc,
            "compiler differs from build manifest"
        );
        assert_eq!(
            provenance.target, compiler.target,
            "target differs from build manifest"
        );
        let expected_root = if provenance.cargo_profile == "dev" {
            "debug"
        } else {
            "release"
        };
        assert_eq!(
            expected_root, compiler.root_profile,
            "profile differs from build manifest"
        );
        provenance
    });
    let bytes = serde_json::to_vec(&BuildMetadata {
        compiler,
        provenance,
    })
    .expect("serialize build identity");
    fs::write(
        PathBuf::from(required("OUT_DIR")).join("rrflow-build.json"),
        bytes,
    )
    .expect("embed build identity");
}
