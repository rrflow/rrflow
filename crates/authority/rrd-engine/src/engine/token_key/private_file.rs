//! Private leaf-file and estate-directory I/O. Validate each opened object.
//! Ancestor traversal and mount confinement remain the installation boundary's
//! responsibility; this module does not claim an atomic filesystem snapshot.

use rrd_store::filesystem::AnchoredPath;
use std::fs::{File, Metadata};
use std::io::{self, Read, Write};
#[cfg(test)]
use std::path::Path;

#[cfg(unix)]
#[path = "private_file/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "private_file/windows.rs"]
mod platform;
#[cfg(not(any(unix, windows)))]
mod platform {
    use super::*;
    pub(super) fn open_relative(
        _: &File,
        _: &std::ffi::OsStr,
        _: bool,
        _: bool,
    ) -> io::Result<File> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native private relative I/O unavailable",
        ))
    }
    pub(super) fn validate(_: &File) -> io::Result<Metadata> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native secret-file enforcement unavailable",
        ))
    }
    pub(super) fn open_empty_relative(
        parent: &File,
        name: &std::ffi::OsStr,
        create: bool,
    ) -> io::Result<File> {
        open_relative(parent, name, create, false)
    }
    pub(super) fn validate_directory(_: &File) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native directory enforcement unavailable",
        ))
    }
}

fn denied(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

pub(in crate::engine) fn pin_private_directory(path: &AnchoredPath) -> io::Result<AnchoredPath> {
    path.pin_directory_checked(platform::validate_directory)
}

pub(in crate::engine) fn create_private_directory_anchored(
    path: &AnchoredPath,
) -> io::Result<AnchoredPath> {
    let directory = path.open_directory_with(|parent, name| {
        let file = platform::open_relative(parent, name, true, true)?;
        platform::validate_directory(&file)?;
        Ok(file)
    })?;
    path.parent()
        .ok_or_else(|| denied("private directory has no parent"))?
        .sync_directory()?;
    Ok(directory)
}

pub(in crate::engine) fn read_private_anchored(
    path: &AnchoredPath,
    maximum: u64,
) -> io::Result<Vec<u8>> {
    let file =
        path.open_file_with(|parent, name| platform::open_relative(parent, name, false, false))?;
    read_opened(file, maximum)
}

/// Admit an empty private lease without reading or changing its contents. The
/// caller retains this exact creation/open handle for native lock ownership.
pub(in crate::engine) fn open_private_empty_anchored(
    path: &AnchoredPath,
    create: bool,
) -> io::Result<File> {
    let file =
        path.open_file_with(|parent, name| platform::open_empty_relative(parent, name, create))?;
    if platform::validate(&file)?.len() != 0 {
        return Err(denied("installation lease must be empty"));
    }
    if create {
        path.parent()
            .ok_or_else(|| denied("lease has no parent"))?
            .sync_directory()?;
    }
    Ok(file)
}

pub(in crate::engine) fn write_private_anchored(
    path: &AnchoredPath,
    bytes: &[u8],
) -> io::Result<()> {
    let mut file =
        path.open_file_with(|parent, name| platform::open_relative(parent, name, true, false))?;
    platform::validate(&file)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    platform::validate(&file)?;
    path.parent()
        .ok_or_else(|| denied("private file has no parent"))?
        .sync_directory()
}

#[cfg(test)]
pub(in crate::engine) fn validate_private_directory(path: &Path) -> io::Result<()> {
    pin_private_directory(&AnchoredPath::from_path(path)?).map(|_| ())
}

#[cfg(test)]
pub(in crate::engine) fn create_private_directory(path: &Path) -> io::Result<()> {
    create_private_directory_anchored(&AnchoredPath::from_path(path)?).map(|_| ())
}

#[cfg(test)]
pub(in crate::engine) fn read_private_bounded(path: &Path, maximum: u64) -> io::Result<Vec<u8>> {
    read_private_anchored(&AnchoredPath::from_path(path)?, maximum)
}

fn read_opened(mut file: File, maximum: u64) -> io::Result<Vec<u8>> {
    let limit = maximum.checked_add(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "secret-file byte bound overflow",
        )
    })?;
    let before = platform::validate(&file)?;
    if before.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "secret file exceeds its byte bound",
        ));
    }
    let mut bytes = Vec::new();
    (&mut file).take(limit).read_to_end(&mut bytes)?;
    let after = platform::validate(&file)?;
    if bytes.len() as u64 > maximum
        || bytes.len() as u64 != before.len()
        || after.len() != before.len()
        || after.modified()? != before.modified()?
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "secret file changed during bounded read",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
pub(in crate::engine) fn write_private_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_private_anchored(&AnchoredPath::from_path(path)?, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(unix, windows))]
    #[test]
    fn anchored_private_creation_retains_native_security() {
        let fixture = tempfile::tempdir().unwrap();
        let root = AnchoredPath::open_root(fixture.path()).unwrap();
        let directory = create_private_directory_anchored(&root.join("private")).unwrap();
        pin_private_directory(&directory).unwrap();
        let secret = directory.join("secret");
        write_private_anchored(&secret, b"private bytes").unwrap();
        assert_eq!(
            read_private_anchored(&secret, 32).unwrap(),
            b"private bytes"
        );
        assert!(write_private_anchored(&secret, b"replacement").is_err());
        assert!(read_private_anchored(&secret, 2).is_err());
        assert_eq!(
            read_private_anchored(&secret, 32).unwrap(),
            b"private bytes"
        );
        assert!(create_private_directory_anchored(&root.join("private")).is_err());
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn permissive_estate_directory_cannot_advance_recovery() {
        use crate::engine::estate_layout::EstateLayout;
        use crate::engine::installation::recovery::InstallApplyStage;
        use crate::RrdEngine;
        use rrd_contract::InstallationTargetKind;

        for relative in [
            ".rrflow",
            ".rrflow/rrd",
            ".rrflow/rrd/roots",
            ".rrflow/credentials",
        ] {
            let project = tempfile::tempdir().unwrap();
            let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
            let preview = RrdEngine::plan_installation(
                project.path(),
                InstallationTargetKind::ExistingProject,
                &"1".repeat(64),
                "default",
                None,
                &executable,
            )
            .unwrap();
            let layout = EstateLayout::new(
                &preview.installation.storage_root_id,
                &preview.installation.initial_principal_id,
            )
            .unwrap();
            let error = RrdEngine::apply_installation_with_failure(
                project.path(),
                InstallationTargetKind::ExistingProject,
                &preview,
                &preview.installation.plan_sha256,
                1_800_000_000_000,
                &executable,
                InstallApplyStage::EstateDirectoriesReady,
            )
            .unwrap_err();
            assert!(error.to_string().contains("test installation failpoint"));
            let intent_path = project.path().join(&layout.install_intent);
            let intent = std::fs::read(&intent_path).unwrap();
            platform::make_directory_permissive_for_test(&project.path().join(relative));
            assert!(
                RrdEngine::apply_installation_at(
                    project.path(),
                    InstallationTargetKind::ExistingProject,
                    &preview,
                    &preview.installation.plan_sha256,
                    1_800_000_000_001,
                    &executable,
                )
                .is_err(),
                "permissive estate directory must deny recovery: {relative}"
            );
            assert_eq!(std::fs::read(&intent_path).unwrap(), intent);
            assert!(!project.path().join(&layout.storage_root).exists());
            assert!(!project.path().join(&layout.operator_credential).exists());
            assert!(validate_private_directory(&project.path().join(relative)).is_err());
            platform::restore_private_directory_for_test(&project.path().join(relative));
            let result = RrdEngine::apply_installation_at(
                project.path(),
                InstallationTargetKind::ExistingProject,
                &preview,
                &preview.installation.plan_sha256,
                1_800_000_000_002,
                &executable,
            )
            .unwrap();
            assert_eq!(result.applied_at_unix_ms, 1_800_000_000_000);
            assert!(!intent_path.exists());

            // Permission drift on an installed estate must deny every admission
            // without touching the locator, credential or canonical store bytes.
            let before = test_file_bytes(project.path());
            platform::make_directory_permissive_for_test(&project.path().join(relative));
            assert!(RrdEngine::read_project_locator(project.path()).is_err());
            assert!(RrdEngine::read_installed_api_key(project.path()).is_err());
            assert!(RrdEngine::inspect_installed(project.path(), &executable).is_err());
            assert!(RrdEngine::open_installed(project.path(), &executable).is_err());
            assert!(RrdEngine::apply_installation_at(
                project.path(),
                InstallationTargetKind::ExistingProject,
                &preview,
                &preview.installation.plan_sha256,
                1_800_000_000_003,
                &executable,
            )
            .is_err());
            assert_eq!(test_file_bytes(project.path()), before);
            assert!(validate_private_directory(&project.path().join(relative)).is_err());
            platform::restore_private_directory_for_test(&project.path().join(relative));
            assert!(
                RrdEngine::apply_installation_at(
                    project.path(),
                    InstallationTargetKind::ExistingProject,
                    &preview,
                    &preview.installation.plan_sha256,
                    1_800_000_000_004,
                    &executable,
                )
                .unwrap()
                .idempotent_replay
            );
        }
    }

    #[cfg(any(unix, windows))]
    fn test_file_bytes(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        let mut bytes = std::collections::BTreeMap::new();
        for entry in std::fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                bytes.extend(test_file_bytes(&entry.path()));
            } else {
                bytes.insert(entry.path(), std::fs::read(entry.path()).unwrap());
            }
        }
        bytes
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn private_directory_creation_is_exclusive_under_a_permissive_parent() {
        let root = tempfile::tempdir().unwrap();
        platform::make_directory_permissive_for_test(root.path());
        let path = root.path().join("owned");
        create_private_directory(&path).unwrap();
        validate_private_directory(&path).unwrap();
        assert_eq!(
            create_private_directory(&path).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        // Creating an estate must not tighten the user's parent directory.
        assert!(validate_private_directory(root.path()).is_err());
        let file = root.path().join("file");
        std::fs::write(&file, b"retained").unwrap();
        assert!(validate_private_directory(&file).is_err());
        assert!(create_private_directory(&file).is_err());
        assert_eq!(std::fs::read(file).unwrap(), b"retained");
    }

    #[test]
    fn bounded_private_reads_and_exclusive_creation_preserve_bytes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("secret");
        write_private_new(&path, b"secret").unwrap();
        assert_eq!(read_private_bounded(&path, 6).unwrap(), b"secret");
        assert!(read_private_bounded(&path, 5).is_err());
        assert!(read_private_bounded(&path, u64::MAX).is_err());
        assert_eq!(
            write_private_new(&path, b"replace").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"secret");
        assert!(read_private_bounded(root.path(), 64).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn path_replacement_cannot_redirect_an_open_secret_read() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("secret");
        write_private_new(&path, b"original").unwrap();
        let file = platform::open(&path, false).unwrap();
        std::fs::rename(&path, root.path().join("retained")).unwrap();
        write_private_new(&path, b"replaced").unwrap();
        assert_eq!(read_opened(file, 8).unwrap(), b"original");
        assert_eq!(read_private_bounded(&path, 8).unwrap(), b"replaced");
    }

    #[cfg(windows)]
    #[test]
    fn open_secret_handle_excludes_replacement_and_concurrent_writers() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("secret");
        write_private_new(&path, b"original").unwrap();
        let file = platform::open(&path, false).unwrap();
        assert!(std::fs::rename(&path, root.path().join("moved")).is_err());
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
        assert_eq!(read_opened(file, 8).unwrap(), b"original");
        std::fs::rename(&path, root.path().join("moved")).unwrap();
    }
}
