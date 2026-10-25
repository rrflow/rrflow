//! Bounded, read-only observations of the synthetic capacity estate.
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn wal_state(root: &Path) -> Value {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root).expect("owned WAL directory") {
        assert!(files.len() < 128, "bounded capacity WAL observation");
        let entry = entry.expect("owned WAL entry");
        let kind = entry.file_type().expect("owned WAL entry type");
        assert!(kind.is_file(), "capacity WAL directory must contain files");
        let metadata = entry.metadata().expect("owned WAL metadata");
        #[cfg(unix)]
        let allocated = {
            use std::os::unix::fs::MetadataExt;
            Some(
                metadata
                    .blocks()
                    .checked_mul(512)
                    .expect("allocation bytes"),
            )
        };
        #[cfg(not(unix))]
        let allocated: Option<u64> = None;
        files.push(json!({"logical_bytes":metadata.len(),"allocated_bytes":allocated}));
    }
    files.sort_by_key(|row| row["logical_bytes"].as_u64().unwrap());
    json!({"files":files})
}

#[test]
fn records_real_lengths_without_paths_or_file_contents() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("first.wal"), b"private bytes").unwrap();
    std::fs::write(root.path().join("second.wal"), b"x").unwrap();
    let observed = wal_state(root.path());
    let files = observed["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0]["logical_bytes"], 1);
    assert_eq!(files[1]["logical_bytes"], 13);
    let encoded = observed.to_string();
    assert!(!encoded.contains("private") && !encoded.contains(".wal"));
    #[cfg(unix)]
    for row in files {
        assert_eq!(row["allocated_bytes"].as_u64().unwrap() % 512, 0);
    }
}
