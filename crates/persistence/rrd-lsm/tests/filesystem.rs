use rrd_lsm::filesystem::{AnchoredPath, OpenOptions};
use std::io::{Read, Write};

#[test]
fn lexical_escape_and_hard_link_denial_preserve_outside_bytes() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let outside = fixture.path().join("outside");
    std::fs::write(&outside, b"outside sentinel").unwrap();
    let anchor = AnchoredPath::open_root(&root).unwrap();
    for escaped in [anchor.join("../outside"), anchor.join(&outside)] {
        assert!(escaped.read().is_err());
        assert!(OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&escaped)
            .is_err());
    }
    std::fs::hard_link(&outside, root.join("linked")).unwrap();
    let linked = anchor.join("linked");
    assert!(linked.read().is_err());
    assert!(OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&linked)
        .is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), b"outside sentinel");
}

#[test]
fn root_replacement_cannot_redirect_a_retained_authority() {
    let fixture = tempfile::tempdir().unwrap();
    let original = fixture.path().join("original");
    let retained = fixture.path().join("retained");
    std::fs::create_dir(&original).unwrap();
    let anchor = AnchoredPath::open_root(&original).unwrap();
    match std::fs::rename(&original, &retained) {
        Ok(()) => {
            std::fs::create_dir(&original).unwrap();
            std::fs::write(original.join("sentinel"), b"replacement").unwrap();
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&anchor.join("owned"))
                .unwrap();
            file.write_all(b"authorized").unwrap();
            assert!(!original.join("owned").exists());
            assert_eq!(
                std::fs::read(retained.join("owned")).unwrap(),
                b"authorized"
            );
            assert_eq!(
                std::fs::read(original.join("sentinel")).unwrap(),
                b"replacement"
            );
        }
        Err(error) => {
            // Windows directory capabilities deny deletion/renaming while held.
            #[cfg(not(windows))]
            panic!("unexpected rename denial: {error}");
            #[cfg(windows)]
            {
                assert!(matches!(error.raw_os_error(), Some(5 | 32)));
                drop(anchor);
                std::fs::rename(&original, &retained).unwrap();
            }
        }
    }
}

#[test]
fn opened_file_remains_the_validated_object_after_name_replacement() {
    let fixture = tempfile::tempdir().unwrap();
    let anchor = AnchoredPath::open_root(fixture.path()).unwrap();
    std::fs::write(fixture.path().join("value"), b"original").unwrap();
    let mut file = anchor.join("value").open_read().unwrap();
    std::fs::rename(fixture.path().join("value"), fixture.path().join("prior")).unwrap();
    std::fs::write(fixture.path().join("value"), b"replacement").unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"original");
}

#[cfg(unix)]
#[test]
fn symlink_components_and_nonregular_leaves_are_denied() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("root");
    let outside = fixture.path().join("outside");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
    let anchor = AnchoredPath::open_root(&root).unwrap();
    assert!(anchor.join("linked/sentinel").read().is_err());
    assert!(anchor.join("linked/created").create_dir().is_err());
    assert!(anchor.join("linked/sentinel").remove_file().is_err());
    assert!(AnchoredPath::open_root(&root.join("linked")).is_err());
    assert_eq!(std::fs::read(outside.join("sentinel")).unwrap(), b"outside");
    assert_eq!(std::fs::read_dir(outside).unwrap().count(), 1);
    assert!(anchor.open_read().is_err());
}

#[test]
fn failed_publication_cleans_only_the_temporary_file_it_created() {
    let fixture = tempfile::tempdir().unwrap();
    let anchor = AnchoredPath::open_root(fixture.path()).unwrap();
    let temporary = anchor.join("pending");
    let target = anchor.join("directory");
    target.create_dir().unwrap();
    assert!(rrd_lsm::filesystem::persist_rename(&temporary, &target, b"candidate").is_err());
    assert!(!temporary.try_exists().unwrap());
    assert!(target.metadata().unwrap().is_dir());
    std::fs::write(fixture.path().join("pending"), b"another owner").unwrap();
    assert!(rrd_lsm::filesystem::persist_rename(&temporary, &target, b"candidate").is_err());
    assert_eq!(temporary.read().unwrap(), b"another owner");
}

#[test]
fn exclusive_publication_and_cleanup_remain_under_retained_ancestors() {
    let fixture = tempfile::tempdir().unwrap();
    let nested = fixture.path().join("parent/child");
    std::fs::create_dir_all(&nested).unwrap();
    let anchor = AnchoredPath::open_root(&nested).unwrap();
    let temporary = anchor.join("temporary");
    let winner = anchor.join("winner");
    std::fs::write(nested.join("temporary"), b"candidate").unwrap();
    std::fs::write(nested.join("winner"), b"winner").unwrap();
    assert_eq!(
        temporary.rename_new_to(&winner).unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(winner.read().unwrap(), b"winner");
    #[cfg(windows)]
    {
        let error = std::fs::rename(
            fixture.path().join("parent"),
            fixture.path().join("replaced"),
        )
        .unwrap_err();
        #[cfg(windows)]
        assert!(matches!(error.raw_os_error(), Some(5 | 32)));
    }
    temporary.remove_file().unwrap();
    assert_eq!(winner.read().unwrap(), b"winner");
}

#[cfg(windows)]
#[test]
fn junction_substitution_denies_read_create_publication_and_cleanup() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("root");
    let outside = fixture.path().join("outside");
    std::fs::create_dir_all(root.join("child")).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside").unwrap();
    let anchor = AnchoredPath::open_root(&root).unwrap();
    std::fs::rename(root.join("child"), root.join("original-child")).unwrap();
    let result = std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "New-Item -ItemType Junction -Path $env:RRFLOW_TEST_LINK -Target $env:RRFLOW_TEST_TARGET -ErrorAction Stop | Out-Null"])
        .env("RRFLOW_TEST_LINK", root.join("child")).env("RRFLOW_TEST_TARGET", &outside).output().unwrap();
    assert!(
        result.status.success(),
        "junction fixture failed: {result:?}"
    );
    let child = anchor.join("child");
    assert!(child.join("sentinel").read().is_err());
    assert!(OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&child.join("sentinel"))
        .is_err());
    assert!(child.join("new").create_dir().is_err());
    assert!(child.join("sentinel").remove_file().is_err());
    std::fs::write(root.join("pending"), b"owned").unwrap();
    assert!(anchor
        .join("pending")
        .rename_to(&child.join("sentinel"))
        .is_err());
    assert_eq!(std::fs::read(outside.join("sentinel")).unwrap(), b"outside");
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 1);
    assert_eq!(anchor.join("pending").read().unwrap(), b"owned");
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "required native CI runs this in a private privileged mount namespace"]
fn bind_mount_crossing_is_denied_before_any_effect() {
    assert_eq!(
        std::env::var("RRFLOW_PRIVATE_MOUNT_FIXTURE").as_deref(),
        Ok("1"),
        "use scripts/ci/run_filesystem_containment.py"
    );
    assert_ne!(
        std::fs::read_link("/proc/self/ns/mnt").unwrap(),
        std::fs::read_link("/proc/1/ns/mnt").unwrap()
    );
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("root");
    let outside = fixture.path().join("outside");
    let mount = root.join("mounted");
    std::fs::create_dir_all(&mount).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside").unwrap();
    let anchor = AnchoredPath::open_root(&root).unwrap();
    assert!(std::process::Command::new("mount")
        .arg("--bind")
        .arg(&outside)
        .arg(&mount)
        .status()
        .unwrap()
        .success());
    struct Unmount(std::path::PathBuf);
    impl Drop for Unmount {
        fn drop(&mut self) {
            assert!(std::process::Command::new("umount")
                .arg(&self.0)
                .status()
                .unwrap()
                .success());
        }
    }
    let _unmount = Unmount(mount);
    let target = anchor.join("mounted/sentinel");
    assert_eq!(
        target.open_read().unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert!(OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&target)
        .is_err());
    assert!(anchor.join("mounted/new").create_dir().is_err());
    assert!(target.remove_file().is_err());
    std::fs::write(root.join("pending"), b"owned").unwrap();
    assert!(anchor.join("pending").rename_to(&target).is_err());
    assert_eq!(std::fs::read(outside.join("sentinel")).unwrap(), b"outside");
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 1);
    assert_eq!(anchor.join("pending").read().unwrap(), b"owned");
}
