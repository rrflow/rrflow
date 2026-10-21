//! Bounded installation preconditions, not a committed attunement snapshot.
//!
//! Stable input retains the version-one digest grammar. Enumeration admits
//! entries before retaining them, and hashing reads at most the admitted file
//! length. Observed metadata drift fails closed; this is not an atomic
//! filesystem snapshot or the D-03 incremental inventory implementation.

use super::DefaultInstallProfile;
use crate::engine::{Result, ServiceError};
use rrd_core::digest::Sha256 as IncrementalSha256;
use rrd_store::filesystem::AnchoredPath;
use std::fs::Metadata;
use std::io::Read;
use std::path::Path;

pub(super) fn project_inventory_sha256(
    project: &AnchoredPath,
    profile: &DefaultInstallProfile,
) -> Result<String> {
    project_inventory_sha256_excluding(project, profile, None)
}

pub(super) fn project_inventory_sha256_excluding(
    project: &AnchoredPath,
    profile: &DefaultInstallProfile,
    excluded_relative: Option<&Path>,
) -> Result<String> {
    let mut entries = Vec::<(AnchoredPath, bool)>::new();
    collect_project_entries(
        project,
        excluded_relative,
        profile.inventory_max_files,
        &mut entries,
    )?;
    entries.sort_by(|left, right| left.0.relative_path().cmp(right.0.relative_path()));
    let mut aggregate = IncrementalSha256::new();
    aggregate.update(b"rrflow-project-inventory-v1\0");
    let mut total = 0u64;
    for (path, directory) in entries {
        let relative = path.relative_path();
        let relative = relative
            .to_str()
            .ok_or(ServiceError::ProjectBindingMismatch)?
            .replace('\\', "/");
        aggregate.update(if directory { b"d\0" } else { b"f\0" });
        aggregate.update(&(relative.len() as u64).to_be_bytes());
        aggregate.update(relative.as_bytes());
        if !directory {
            let metadata = path
                .metadata()
                .map_err(|error| ServiceError::Storage(error.to_string()))?;
            total = total.checked_add(metadata.len()).ok_or_else(|| {
                ServiceError::Contract("project inventory byte count overflowed".into())
            })?;
            if total > profile.inventory_max_bytes {
                return Err(ServiceError::Contract(
                    "project inventory exceeds the configured byte bound".into(),
                ));
            }
            aggregate.update(&metadata.len().to_be_bytes());
            aggregate.update(hash_inventory_file(&path, &metadata)?.as_bytes());
        }
    }
    Ok(aggregate.finalize_hex())
}

fn collect_project_entries(
    root: &AnchoredPath,
    excluded_relative: Option<&Path>,
    max_entries: u64,
    output: &mut Vec<(AnchoredPath, bool)>,
) -> Result<()> {
    let mut pending = vec![root.to_owned()];
    while let Some(current) = pending.pop() {
        let entries = current.entries().map_err(storage_error)?;
        // Do not collect or sort a whole directory before applying the bound.
        // The admitted output is sorted once, by the digest builder.
        for entry in entries {
            let path = entry.map_err(storage_error)?;
            let relative = path.relative_path();
            if excluded_relative == Some(relative)
                || relative.components().next().is_some_and(|component| {
                    matches!(component.as_os_str().to_str(), Some(".git" | ".rrflow"))
                })
            {
                continue;
            }
            if output.len() as u64 >= max_entries {
                return Err(ServiceError::Contract(
                    "project inventory exceeds the configured file bound".into(),
                ));
            }
            let metadata = path.metadata().map_err(storage_error)?;
            if metadata.file_type().is_symlink() {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            if metadata.is_dir() {
                pending.push(path.clone());
                output.push((path, true));
            } else if metadata.is_file() {
                output.push((path, false));
            } else {
                return Err(ServiceError::ProjectBindingMismatch);
            }
        }
    }
    Ok(())
}

fn hash_inventory_file(path: &AnchoredPath, admitted: &Metadata) -> Result<String> {
    if admitted.file_type().is_symlink() || !admitted.is_file() {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let mut file = path.open_read().map_err(storage_error)?;
    require_same_observation(admitted, &file.metadata().map_err(storage_error)?)?;
    let digest = hash_exact_contents(&mut file, admitted.len())?;
    require_same_observation(admitted, &file.metadata().map_err(storage_error)?)?;
    require_same_observation(admitted, &path.metadata().map_err(storage_error)?)?;
    Ok(digest)
}

fn require_same_observation(admitted: &Metadata, observed: &Metadata) -> Result<()> {
    if observed.file_type().is_symlink()
        || !observed.is_file()
        || observed.len() != admitted.len()
        || observed.modified().map_err(storage_error)?
            != admitted.modified().map_err(storage_error)?
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

fn hash_exact_contents(reader: &mut impl Read, admitted_bytes: u64) -> Result<String> {
    let mut bounded = reader.take(admitted_bytes);
    let mut hasher = IncrementalSha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = match bounded.read(&mut buffer) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(storage_error(error)),
        };
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    if bounded.limit() != 0 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(hasher.finalize_hex())
}

fn storage_error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    fn project_inventory_sha256(path: &Path, profile: &DefaultInstallProfile) -> Result<String> {
        super::project_inventory_sha256(
            &AnchoredPath::open_root(path).map_err(storage_error)?,
            profile,
        )
    }
    fn project_inventory_sha256_excluding(
        path: &Path,
        profile: &DefaultInstallProfile,
        exclude: Option<&Path>,
    ) -> Result<String> {
        super::project_inventory_sha256_excluding(
            &AnchoredPath::open_root(path).map_err(storage_error)?,
            profile,
            exclude,
        )
    }
    fn collect_project_entries(
        path: &Path,
        exclude: Option<&Path>,
        max: u64,
        output: &mut Vec<(AnchoredPath, bool)>,
    ) -> Result<()> {
        super::collect_project_entries(
            &AnchoredPath::open_root(path).map_err(storage_error)?,
            exclude,
            max,
            output,
        )
    }

    #[test]
    fn project_inventory_preserves_the_version_one_digest_grammar() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        std::fs::create_dir(root.path().join(".rrflow")).unwrap();
        std::fs::write(root.path().join("src/main.rs"), b"fn main() {}\n").unwrap();
        std::fs::write(root.path().join("empty"), b"").unwrap();
        std::fs::write(root.path().join("README.md"), b"RRFlow\n").unwrap();
        std::fs::write(root.path().join(".git/config"), b"excluded").unwrap();
        std::fs::write(root.path().join(".rrflow/cache"), b"excluded").unwrap();
        let (profile, _) = DefaultInstallProfile::load("default").unwrap();
        assert_eq!(
            project_inventory_sha256(root.path(), &profile).unwrap(),
            "a1a2f8baba5c66694e08b3263eb9e4df589451ef7d65d00891a26bd201937514"
        );
    }

    #[test]
    fn enumeration_never_retains_more_than_the_admitted_entries() {
        let root = tempfile::tempdir().unwrap();
        let mut directory = root.path().to_owned();
        for _ in 0..24 {
            directory.push("d");
            std::fs::create_dir(&directory).unwrap();
        }
        std::fs::write(directory.join("leaf"), b"source").unwrap();
        for bound in [0, 1, 7, 24] {
            let mut entries = Vec::new();
            let error =
                collect_project_entries(root.path(), None, bound, &mut entries).unwrap_err();
            assert!(error.to_string().contains("file bound"));
            assert_eq!(entries.len() as u64, bound);
        }
        let mut entries = Vec::new();
        collect_project_entries(root.path(), None, 25, &mut entries).unwrap();
        assert_eq!(entries.len(), 25);
    }

    #[test]
    fn exclusions_and_creation_order_do_not_change_the_digest_or_admitted_count() {
        let (mut profile, _) = DefaultInstallProfile::load("default").unwrap();
        profile.inventory_max_files = 3;
        profile.inventory_max_bytes = 3;
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for (root, names) in [
            (first.path(), ["alpha", "empty", "unicode-\u{3bb}"]),
            (second.path(), ["unicode-\u{3bb}", "empty", "alpha"]),
        ] {
            for name in names {
                std::fs::write(root.join(name), if name == "empty" { "" } else { "a" }).unwrap();
            }
            for name in [".git", ".rrflow"] {
                std::fs::create_dir(root.join(name)).unwrap();
                std::fs::write(root.join(name).join("not-source"), b"excluded").unwrap();
            }
        }
        let digest = project_inventory_sha256(first.path(), &profile).unwrap();
        assert_eq!(
            project_inventory_sha256(second.path(), &profile).unwrap(),
            digest
        );
        std::fs::write(first.path().join("intent"), b"owned intent").unwrap();
        assert_eq!(
            project_inventory_sha256_excluding(first.path(), &profile, Some(Path::new("intent")))
                .unwrap(),
            digest
        );
        assert!(project_inventory_sha256(first.path(), &profile).is_err());
        std::fs::remove_file(first.path().join("intent")).unwrap();
        profile.inventory_max_bytes = 1;
        assert!(project_inventory_sha256(first.path(), &profile)
            .unwrap_err()
            .to_string()
            .contains("byte bound"));
    }

    struct EndlessReader {
        consumed: usize,
        interrupt_once: bool,
    }

    impl Read for EndlessReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if self.interrupt_once {
                self.interrupt_once = false;
                return Err(std::io::ErrorKind::Interrupted.into());
            }
            buffer.fill(b'a');
            self.consumed += buffer.len();
            Ok(buffer.len())
        }
    }

    #[test]
    fn hashing_cannot_read_past_admission_even_when_the_source_never_ends() {
        for length in [0, 1, 65_537] {
            let mut reader = EndlessReader {
                consumed: 0,
                interrupt_once: true,
            };
            let digest = hash_exact_contents(&mut reader, length).unwrap();
            assert_eq!(reader.consumed as u64, length);
            assert_eq!(
                digest,
                rrd_core::digest::sha256_hex(&vec![b'a'; length as usize])
            );
        }
        let mut short = std::io::Cursor::new(b"abc");
        assert!(hash_exact_contents(&mut short, 4).is_err());
    }

    #[test]
    fn observed_file_growth_shrinkage_and_timestamp_changes_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source");
        std::fs::write(&path, b"abc").unwrap();
        let admitted = std::fs::metadata(&path).unwrap();
        assert_eq!(
            hash_inventory_file(&AnchoredPath::from_path(&path).unwrap(), &admitted).unwrap(),
            rrd_core::digest::sha256_hex(b"abc")
        );
        std::fs::write(&path, b"abcdef").unwrap();
        assert!(hash_inventory_file(&AnchoredPath::from_path(&path).unwrap(), &admitted).is_err());
        std::fs::write(&path, b"a").unwrap();
        assert!(hash_inventory_file(&AnchoredPath::from_path(&path).unwrap(), &admitted).is_err());
        std::fs::write(&path, b"abc").unwrap();
        let file = File::options().write(true).open(&path).unwrap();
        let modified = admitted.modified().unwrap() + std::time::Duration::from_secs(60);
        file.set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        assert!(hash_inventory_file(&AnchoredPath::from_path(&path).unwrap(), &admitted).is_err());
    }
}
