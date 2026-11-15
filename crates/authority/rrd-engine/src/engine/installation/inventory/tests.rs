use super::*;
use std::fs::File;

#[test]
fn selected_source_read_binds_inventory_identity_and_charges_budget() {
    use rrd_attunement::inventory::{EntryKind, SourceScope};

    let fixture = tempfile::tempdir().unwrap();
    std::fs::create_dir(fixture.path().join("src")).unwrap();
    let file = fixture.path().join("src/main.rs");
    let content = b"fn main() { }\n";
    std::fs::write(&file, content).unwrap();
    std::fs::write(fixture.path().join(".env"), b"must never be read").unwrap();
    let root = AnchoredPath::open_root(fixture.path()).unwrap();
    let (profile, _) = DefaultInstallProfile::load("default").unwrap();
    let (_, proposal) = prepare_project_inventory(
        &root,
        &profile,
        SourceScope {
            engram: "engram".into(),
            project: "project".into(),
            root: "root".into(),
        },
        None,
        &|| Ok(()),
    )
    .unwrap();
    let entry = proposal
        .entries()
        .iter()
        .find(|e| e.path == "src/main.rs")
        .unwrap();
    let secret = proposal
        .entries()
        .iter()
        .find(|e| e.path == ".env")
        .unwrap();
    assert!(matches!(secret.kind, EntryKind::Excluded { .. }));
    let mut remaining = content.len() as u64;
    let mut output = Vec::new();
    read_selected_source(
        &root,
        &proposal,
        entry,
        content.len() as u64,
        &mut remaining,
        &mut output,
        &|| Ok(()),
    )
    .unwrap();
    assert_eq!(output, content);
    assert_eq!(remaining, 0);
    assert!(read_selected_source(
        &root,
        &proposal,
        entry,
        content.len() as u64,
        &mut remaining,
        &mut output,
        &|| Ok(())
    )
    .is_err());
    remaining = content.len() as u64;
    assert!(read_selected_source(
        &root,
        &proposal,
        secret,
        100,
        &mut remaining,
        &mut output,
        &|| Ok(())
    )
    .is_err());
    assert_eq!(remaining, content.len() as u64);
    assert!(read_selected_source(
        &root,
        &proposal,
        entry,
        content.len() as u64 - 1,
        &mut remaining,
        &mut output,
        &|| Ok(())
    )
    .is_err());
    assert_eq!(remaining, content.len() as u64);
    let mut substituted = entry.clone();
    substituted.identity = secret.identity.clone();
    assert!(read_selected_source(
        &root,
        &proposal,
        &substituted,
        100,
        &mut remaining,
        &mut output,
        &|| Ok(())
    )
    .is_err());
    assert_eq!(remaining, content.len() as u64);
    let checks = std::cell::Cell::new(0);
    assert!(matches!(
        read_selected_source(
            &root,
            &proposal,
            entry,
            content.len() as u64,
            &mut remaining,
            &mut output,
            &|| {
                checks.set(checks.get() + 1);
                if checks.get() == 2 {
                    Err(ServiceError::DeadlineExceeded)
                } else {
                    Ok(())
                }
            }
        ),
        Err(ServiceError::DeadlineExceeded)
    ));
    assert_eq!(remaining, 0, "an interrupted read retains its reservation");
    remaining = content.len() as u64;
    struct InterruptedSink(Vec<u8>);
    impl std::io::Write for InterruptedSink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if !self.0.is_empty() {
                return Err(std::io::Error::other("simulated worker pipe failure"));
            }
            let count = bytes.len().min(3);
            self.0.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut sink = InterruptedSink(Vec::new());
    assert!(read_selected_source(
        &root,
        &proposal,
        entry,
        100,
        &mut remaining,
        &mut sink,
        &|| Ok(())
    )
    .is_err());
    assert_eq!(sink.0, content[..3]);
    assert_eq!(remaining, 0, "a failed pipe retains its reservation");
    remaining = content.len() as u64;
    std::fs::write(&file, b"fn changed(){}\n").unwrap();
    assert!(read_selected_source(
        &root,
        &proposal,
        entry,
        100,
        &mut remaining,
        &mut output,
        &|| Ok(())
    )
    .is_err());
    std::fs::remove_file(&file).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("outside.rs");
    std::fs::write(&outside_file, content).unwrap();
    std::fs::hard_link(&outside_file, &file).unwrap();
    assert!(read_selected_source(
        &root,
        &proposal,
        entry,
        100,
        &mut remaining,
        &mut output,
        &|| Ok(())
    )
    .is_err());
    assert_eq!(std::fs::read(outside_file).unwrap(), content);
}

#[test]
fn selected_path_retries_only_bounded_git_observation_changes() {
    let attempts = std::cell::Cell::new(0);
    retry_selected_path_observation(|| {
        let current = attempts.get() + 1;
        attempts.set(current);
        if current < 3 {
            Err(ServiceError::AttunementSourceChanged)
        } else {
            Ok(())
        }
    })
    .unwrap();
    assert_eq!(attempts.get(), 3);

    let attempts = std::cell::Cell::new(0);
    assert!(matches!(
        retry_selected_path_observation(|| {
            attempts.set(attempts.get() + 1);
            Err(ServiceError::AttunementSourceChanged)
        }),
        Err(ServiceError::AttunementSourceChanged)
    ));
    assert_eq!(attempts.get(), 3);

    let attempts = std::cell::Cell::new(0);
    assert!(matches!(
        retry_selected_path_observation(|| {
            attempts.set(attempts.get() + 1);
            Err(ServiceError::ProjectBindingMismatch)
        }),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    assert_eq!(attempts.get(), 1);
}

fn hash_exact_contents(reader: &mut impl Read, admitted_bytes: u64) -> Result<String> {
    super::hash_exact_contents(reader, admitted_bytes, &WorkBudget::new(30_000))
}
fn hash_inventory_file(path: &AnchoredPath, admitted: &Metadata) -> Result<String> {
    super::hash_inventory_file(path, admitted, &WorkBudget::new(30_000))
}
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
    let (mut profile, _) = DefaultInstallProfile::load("default")?;
    profile.inventory_policy.limits.entries = max;
    let policy = profile.inventory_policy.compile()?;
    let root = AnchoredPath::open_root(path).map_err(storage_error)?;
    let budget = WorkBudget::new(policy.source.limits.elapsed_ms);
    let mut git = GitInputs::read(&root, &policy, &budget)?;
    let mut entries = Vec::new();
    let result = super::collect_project_entries(
        &root,
        WalkOptions {
            excluded_relative: exclude,
            lease_admitted: false,
            record_exclusions: false,
        },
        &policy,
        &budget,
        &mut git,
        &mut entries,
    );
    output.extend(entries.into_iter().map(|entry| {
        let directory = matches!(&entry.observed, Some(SourceLeaf::Ordinary(obs)) if obs.kind() == SourceKind::Directory);
        (entry.path, directory)
    }));
    result
}

#[test]
fn project_precondition_binds_version_three_policy_git_inputs_and_eligible_source() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    std::fs::create_dir(root.path().join(".rrflow")).unwrap();
    std::fs::write(root.path().join("src/main.rs"), b"fn main() {}\n").unwrap();
    std::fs::write(root.path().join("empty"), b"").unwrap();
    std::fs::write(root.path().join("README.md"), b"RRFlow\n").unwrap();
    std::fs::write(
        root.path().join(".git/config"),
        b"[core]\nrepositoryFormatVersion = 0\n",
    )
    .unwrap();
    std::fs::write(root.path().join(".rrflow/cache"), b"excluded").unwrap();
    let (profile, _) = DefaultInstallProfile::load("default").unwrap();
    assert_eq!(
        project_inventory_sha256(root.path(), &profile).unwrap(),
        "edb4ae9b577c16fc70140c5e5f046d2e57700bbda5aa62bc13cbbae16935f475"
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
    for bound in [1, 7, 24] {
        let mut entries = Vec::new();
        let error = collect_project_entries(root.path(), None, bound, &mut entries).unwrap_err();
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
    profile.inventory_policy.limits.entries = 6;
    profile.inventory_policy.limits.content_bytes = 3;
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
    profile.inventory_policy.limits.content_bytes = 1;
    assert!(project_inventory_sha256(first.path(), &profile)
        .unwrap_err()
        .to_string()
        .contains("byte bound"));
}

#[test]
fn git_walk_retains_tracked_ignored_source_and_binds_metadata_inputs() {
    let root = tempfile::tempdir().unwrap();
    for directory in [".git/info", "src", "ignored", ".github/workflows"] {
        std::fs::create_dir_all(root.path().join(directory)).unwrap();
    }
    std::fs::write(
        root.path().join(".git/config"),
        b"[core]\nrepositoryFormatVersion=0\nignoreCase=false\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join(".git/index"),
        include_bytes!(
            "../../../../../../compute/rrd-attunement/tests/fixtures/git-index/sha1-v2.index"
        ),
    )
    .unwrap();
    std::fs::write(root.path().join(".gitignore"), b"ignored/\n*.log\n").unwrap();
    std::fs::write(root.path().join(".git/info/exclude"), b"*.bak\n").unwrap();
    std::fs::write(root.path().join("src/.gitignore"), b"!keep.log\n").unwrap();
    for path in [
        "src/main.rs",
        "ignored/tracked.txt",
        ".github/workflows/check.yml",
        "src/keep.log",
    ] {
        std::fs::write(root.path().join(path), b"source").unwrap();
    }
    let (profile, _) = DefaultInstallProfile::load("default").unwrap();
    let first = project_inventory_sha256(root.path(), &profile).unwrap();
    for path in [
        "ignored/untracked.txt",
        "other.log",
        "other.bak",
        "src/drop.log",
        ".env",
    ] {
        std::fs::write(root.path().join(path), b"excluded").unwrap();
    }
    assert_eq!(
        project_inventory_sha256(root.path(), &profile).unwrap(),
        first
    );
    for path in [
        "src/main.rs",
        "ignored/tracked.txt",
        ".github/workflows/check.yml",
        "src/keep.log",
    ] {
        std::fs::write(root.path().join(path), b"changed").unwrap();
        assert_ne!(
            project_inventory_sha256(root.path(), &profile).unwrap(),
            first,
            "{path}"
        );
        std::fs::write(root.path().join(path), b"source").unwrap();
    }
    std::fs::write(root.path().join(".git/info/exclude"), b"*.bak\n*.new\n").unwrap();
    assert_ne!(
        project_inventory_sha256(root.path(), &profile).unwrap(),
        first
    );
    std::fs::write(
        root.path().join(".git/config"),
        b"[include]\npath=../outside\n",
    )
    .unwrap();
    assert!(project_inventory_sha256(root.path(), &profile).is_err());
}

#[cfg(windows)]
#[test]
fn git_ignored_locked_file_is_never_opened() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    std::fs::write(root.path().join(".gitignore"), b"*.log\n").unwrap();
    let (profile, _) = DefaultInstallProfile::load("default").unwrap();
    let first = project_inventory_sha256(root.path(), &profile).unwrap();
    let locked = File::options()
        .create_new(true)
        .read(true)
        .write(true)
        .share_mode(0)
        .open(root.path().join("private.log"))
        .unwrap();
    assert_eq!(
        project_inventory_sha256(root.path(), &profile).unwrap(),
        first
    );
    drop(locked);
}

#[test]
fn git_inputs_detect_appearance_change_and_bound_bytes() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    let (mut profile, _) = DefaultInstallProfile::load("default").unwrap();
    let anchor = AnchoredPath::open_root(root.path()).unwrap();
    let budget = WorkBudget::new(30_000);
    let policy = profile.inventory_policy.compile().unwrap();
    let mut git = GitInputs::read(&anchor, &policy, &budget).unwrap();
    git.enter(&anchor, &policy, &budget).unwrap();
    git.revalidate(&budget).unwrap();
    std::fs::write(root.path().join(".gitignore"), b"*.log\n").unwrap();
    assert!(git.revalidate(&budget).is_err());
    let mut git = GitInputs::read(&anchor, &policy, &budget).unwrap();
    git.enter(&anchor, &policy, &budget).unwrap();
    std::fs::write(root.path().join(".gitignore"), b"*.bak\n").unwrap();
    assert!(git.revalidate(&budget).is_err());
    std::fs::write(root.path().join(".git/config"), vec![b' '; 2048]).unwrap();
    profile.inventory_policy.limits.git_input_bytes = 1024;
    assert!(project_inventory_sha256(root.path(), &profile)
        .unwrap_err()
        .to_string()
        .contains("Git input byte bound"));
}

#[test]
fn directory_alias_is_a_leaf_and_target_content_is_not_read() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("private"), b"first").unwrap();
    let link = root.path().join("alias");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    #[cfg(windows)]
    {
        let result = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let (profile, _) = DefaultInstallProfile::load("default").unwrap();
    let first = project_inventory_sha256(root.path(), &profile).unwrap();
    std::fs::write(outside.path().join("private"), b"changed outside content").unwrap();
    assert_eq!(
        project_inventory_sha256(root.path(), &profile).unwrap(),
        first
    );
    let anchored = AnchoredPath::open_root(root.path()).unwrap();
    let policy = profile.inventory_policy.compile().unwrap();
    let budget = WorkBudget::new(30_000);
    let mut git = GitInputs::read(&anchored, &policy, &budget).unwrap();
    let mut entries = Vec::new();
    super::collect_project_entries(
        &anchored,
        WalkOptions {
            excluded_relative: None,
            lease_admitted: false,
            record_exclusions: false,
        },
        &policy,
        &budget,
        &mut git,
        &mut entries,
    )
    .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].observed, Some(SourceLeaf::Symlink));
}

#[test]
fn discovery_reports_real_gitlinks_without_following_child_metadata() {
    use std::process::Command;
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(root.path())
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .env_remove("GIT_TEMPLATE_DIR")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--quiet", "--object-format=sha1"]);
    for child in ["z-child", "a-child"] {
        std::fs::create_dir(root.path().join(child)).unwrap();
        std::fs::write(
            root.path().join(child).join(".git"),
            b"gitdir: ../outside\n",
        )
        .unwrap();
        std::fs::write(root.path().join(child).join(".gitignore"), b"first\n").unwrap();
        git(&[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,1111111111111111111111111111111111111111,{child}"),
        ]);
    }
    std::fs::create_dir_all(root.path().join("untracked/.git")).unwrap();
    let (profile, digest) = DefaultInstallProfile::load("default").unwrap();
    let anchor = AnchoredPath::open_root(root.path()).unwrap();
    let first = super::discover_gitlinks(&anchor, &profile, digest.clone()).unwrap();
    assert!(first.git_index_verified);
    assert_eq!(first.candidates, ["a-child", "z-child"]);
    assert!(!first.selection_available);
    assert_eq!(first.coverage, "present_tracked_gitlink_directories_only");
    assert!(!root.path().join(".rrflow").exists());
    std::fs::write(root.path().join("a-child/.git"), b"gitdir: ../elsewhere\n").unwrap();
    std::fs::write(root.path().join("a-child/.gitignore"), b"later\n").unwrap();
    let second = super::discover_gitlinks(&anchor, &profile, digest.clone()).unwrap();
    assert_eq!(first, second, "child metadata must not be read");
    git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        "160000,2222222222222222222222222222222222222222,a-child",
    ]);
    let third = super::discover_gitlinks(&anchor, &profile, digest).unwrap();
    assert_eq!(first.candidates, third.candidates);
    assert_ne!(first.git_inputs_sha256, third.git_inputs_sha256);
    assert!(!root.path().join(".rrflow").exists());
}

#[test]
fn discovery_classifies_missing_index_and_enforces_existing_bounds() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source"), b"source").unwrap();
    let anchor = AnchoredPath::open_root(root.path()).unwrap();
    let (mut profile, digest) = DefaultInstallProfile::load("default").unwrap();
    let result = super::discover_gitlinks(&anchor, &profile, digest.clone()).unwrap();
    assert!(!result.git_index_verified);
    assert!(result.candidates.is_empty());
    assert!(!result.selection_available);
    profile.inventory_policy.limits.output_bytes = 1;
    assert!(super::discover_gitlinks(&anchor, &profile, digest)
        .unwrap_err()
        .to_string()
        .contains("discovery output byte bound"));
    assert!(!root.path().join(".rrflow").exists());
}

#[test]
fn linked_worktree_metadata_requires_explicit_admission() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join(".git"), b"gitdir: ../outside\n").unwrap();
    let (profile, _) = DefaultInstallProfile::load("default").unwrap();
    assert!(project_inventory_sha256(root.path(), &profile)
        .unwrap_err()
        .to_string()
        .contains("requires admission"));
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

#[cfg(windows)]
#[test]
fn secret_exclusions_precede_actual_file_opening() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = tempfile::tempdir().unwrap();
    let (profile, _) = DefaultInstallProfile::load("default").unwrap();
    std::fs::write(root.path().join("source.rs"), b"source").unwrap();
    let expected = project_inventory_sha256(root.path(), &profile).unwrap();
    std::fs::create_dir(root.path().join(".swag")).unwrap();
    let secrets: Vec<_> = [".env", "server.env", ".swag/session"]
        .into_iter()
        .map(|name| {
            File::options()
                .create_new(true)
                .write(true)
                .read(true)
                .share_mode(0)
                .open(root.path().join(name))
                .unwrap()
        })
        .collect();
    assert_eq!(
        project_inventory_sha256(root.path(), &profile).unwrap(),
        expected
    );
    let eligible = File::options()
        .create_new(true)
        .write(true)
        .read(true)
        .share_mode(0)
        .open(root.path().join("ordinary.rs"))
        .unwrap();
    assert!(project_inventory_sha256(root.path(), &profile).is_err());
    drop(eligible);
    drop(secrets);
}

#[test]
fn generated_and_oversized_files_are_excluded_but_hidden_configuration_matters() {
    let root = tempfile::tempdir().unwrap();
    let (mut profile, _) = DefaultInstallProfile::load("default").unwrap();
    profile.inventory_policy.limits.file_bytes = 8;
    let expected = project_inventory_sha256(root.path(), &profile).unwrap();
    std::fs::create_dir(root.path().join("target")).unwrap();
    std::fs::write(root.path().join("target/generated"), b"generated").unwrap();
    std::fs::write(root.path().join("large"), b"oversized source").unwrap();
    assert_eq!(
        project_inventory_sha256(root.path(), &profile).unwrap(),
        expected
    );
    std::fs::create_dir(root.path().join(".github")).unwrap();
    std::fs::write(root.path().join(".github/config"), b"config").unwrap();
    let hidden = project_inventory_sha256(root.path(), &profile).unwrap();
    assert_ne!(hidden, expected);
    profile.inventory_policy.limits.file_bytes += 1;
    assert_ne!(
        project_inventory_sha256(root.path(), &profile).unwrap(),
        hidden
    );
}

#[test]
fn metadata_and_depth_budgets_reject_before_complete_precondition() {
    let root = tempfile::tempdir().unwrap();
    let (mut profile, _) = DefaultInstallProfile::load("default").unwrap();
    for number in 0..16 {
        std::fs::write(root.path().join(format!("source-{number}")), b"a").unwrap();
    }
    profile.inventory_policy.limits.metadata_bytes = 1024;
    profile.inventory_policy.limits.git_input_bytes = 1024;
    profile.inventory_policy.limits.path_bytes = 512;
    assert!(project_inventory_sha256(root.path(), &profile)
        .unwrap_err()
        .to_string()
        .contains("metadata byte bound"));
    let (mut profile, _) = DefaultInstallProfile::load("default").unwrap();
    profile.inventory_policy.limits.depth = 1;
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("src/nested"), b"a").unwrap();
    assert!(project_inventory_sha256(root.path(), &profile)
        .unwrap_err()
        .to_string()
        .contains("PathLimit"));
}

#[test]
fn expired_budget_performs_no_content_read() {
    let mut reader = EndlessReader {
        consumed: 0,
        interrupt_once: false,
    };
    assert!(matches!(
        super::hash_exact_contents(&mut reader, 10, &WorkBudget::new(0)),
        Err(ServiceError::DeadlineExceeded)
    ));
    assert_eq!(reader.consumed, 0);
}
