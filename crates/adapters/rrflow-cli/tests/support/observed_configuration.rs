//! The provider captures under `endpoint-selection-observed` and
//! `endpoint-chat-native` predate the estate-to-engram rename. They are
//! immutable recorded observations whose bytes and digests are recorded beside
//! them, so they are never rewritten. A test reads the recorded bytes and, in
//! memory only, (1) reads the one renamed field under its current name and
//! (2) re-binds the recorded configuration digest to the digest of that
//! projected configuration, because the digest covers the renamed field.
//! Nothing else in a capture changes.

#![allow(dead_code)] // each test crate uses the part of this shared helper it needs

use std::path::Path;

/// The recorded configuration with `identity.estate_id` read as `engram_id`.
pub fn project(bytes: &[u8]) -> Vec<u8> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let identity = value["identity"].as_object_mut().unwrap();
    let recorded = identity
        .remove("estate_id")
        .expect("the capture records its identity as estate_id");
    identity.insert("engram_id".into(), recorded);
    serde_json::to_vec(&value).unwrap()
}

/// `bytes` with the configuration digest the capture recorded replaced by the
/// digest of the projected configuration. `recorded_allowance` is the capture's
/// own allowance, which states the digest it recorded.
pub fn rebind(configuration: &[u8], recorded_allowance: &[u8], bytes: &[u8]) -> Vec<u8> {
    let recorded: serde_json::Value = serde_json::from_slice(recorded_allowance).unwrap();
    let old = recorded["configuration_sha256"].as_str().unwrap();
    let projected: rrd_contract::EndpointConfiguration =
        serde_json::from_slice(&project(configuration)).unwrap();
    let new = projected.sha256().unwrap();
    String::from_utf8(bytes.to_vec())
        .unwrap()
        .replace(old, &new)
        .into_bytes()
}

/// One file of a capture directory, projected and re-bound as described above.
pub fn read(directory: &Path, file: &str) -> Vec<u8> {
    let bytes = std::fs::read(directory.join(file)).unwrap();
    if file == "configuration.json" {
        return project(&bytes);
    }
    let configuration = std::fs::read(directory.join("configuration.json")).unwrap();
    let allowance = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("allowance.json"))
        })
        .expect("a capture records its allowance");
    rebind(&configuration, &std::fs::read(allowance).unwrap(), &bytes)
}
