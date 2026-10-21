//! Independent observations of installation's previewed effect boundary.
use super::*;
use crate::engine::clock::FixedClock;
use crate::engine::installation::recovery::InstallApplyStage;
use rrd_contract::InstallationTargetKind;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[test]
fn installation_preview_covers_transient_intent_before_storage_creation() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), b"project-owned content\n").unwrap();
    let original: BTreeSet<PathBuf> = std::fs::read_dir(project.path())
        .unwrap()
        .map(|entry| PathBuf::from(entry.unwrap().file_name()))
        .collect();
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
    let declared: BTreeSet<PathBuf> = preview
        .installation
        .managed_paths
        .iter()
        .map(|path| PathBuf::from(&path.relative_path))
        .collect();
    let mut observed_intent_boundary = false;
    let result = RrdEngine::apply_installation_with_test_observer(
        project.path(),
        &preview,
        &executable,
        &mut |stage, _| {
            if stage == InstallApplyStage::IntentPublished {
                observed_intent_boundary = true;
                // Enumerate actual effects, independently of EstateLayout and
                // the intent filename constructor. Never read secret payloads.
                for entry in std::fs::read_dir(project.path()).unwrap() {
                    let relative = PathBuf::from(entry.unwrap().file_name());
                    if !original.contains(&relative) {
                        assert!(
                            declared.contains(&relative),
                            "installation created an unpreviewed path: {}",
                            relative.display()
                        );
                    }
                }
                return Err(ServiceError::Storage("observed intent boundary".into()));
            }
            Ok(())
        },
    );
    assert!(observed_intent_boundary);
    assert!(
        result.is_err(),
        "observer must stop before storage creation"
    );
    assert!(!project.path().join(".rrflow").exists());
    assert_eq!(
        std::fs::read(project.path().join("README.md")).unwrap(),
        b"project-owned content\n"
    );
}

fn executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

#[test]
fn shared_sdk_corpus_identity_matches_bundled_installation() {
    let anchor =
        include_bytes!("../../../../../../fixtures/rrflow-sdk-conformance-install-anchor-v1.bin");
    assert_eq!(
        digest::sha256_hex(anchor),
        "d42417daff2c8032efa0c090f1f7d6fb18525f3c2813aa8e1730ff935dd01ee7",
        "the SDK identity anchor must not undergo checkout newline conversion"
    );
    let corpus: rrd_contract::SdkConformanceCorpus = serde_json::from_slice(include_bytes!(
        "../../../../../../fixtures/rrd-sdk-conformance-v1.json"
    ))
    .unwrap();
    corpus.validate().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let distribution = temporary.path().join("install-anchor.bin");
    std::fs::write(&distribution, anchor).unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &digest::sha256_hex(anchor),
        "default",
        None,
        &distribution,
    )
    .unwrap();
    assert_eq!(
        preview.installation.target.instance_id,
        corpus.identity.instance
    );
    assert_eq!(
        preview.installation.target.estate_id,
        corpus.identity.estate
    );
    assert!(paths(&project).is_empty());
}

fn preview_for(project: &Path, profile: &str) -> crate::InstallationPreview {
    RrdEngine::plan_installation(
        project,
        InstallationTargetKind::ExistingProject,
        &"1".repeat(64),
        profile,
        None,
        &executable(),
    )
    .unwrap()
}

fn paths(root: &Path) -> BTreeSet<PathBuf> {
    fn visit(root: &Path, current: &Path, result: &mut BTreeSet<PathBuf>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            result.insert(path.strip_prefix(root).unwrap().to_owned());
            if entry.file_type().unwrap().is_dir() {
                visit(root, &path, result);
            }
        }
    }
    let mut result = BTreeSet::new();
    visit(root, root, &mut result);
    result
}

#[test]
fn bundled_bootstrap_closes_observed_files_records_and_inactive_specializations() {
    use rrd_contract::InstallationManagedPathKind as Kind;
    for profile in ["default", "clyffy"] {
        let project = tempfile::tempdir().unwrap();
        let owned = b"# Project-owned instructions\r\nKeep these exact bytes.\r\n";
        std::fs::write(project.path().join("AGENTS.md"), owned).unwrap();
        let preview = preview_for(project.path(), profile);
        assert_eq!(preview, preview_for(project.path(), profile));
        assert_eq!(
            paths(project.path()),
            BTreeSet::from([PathBuf::from("AGENTS.md")])
        );
        let plan = &preview.installation;
        let bootstrap = &plan.bootstrap;
        assert_eq!(
            bootstrap.specialization.id.as_str(),
            if profile == "default" {
                "generic"
            } else {
                "clyffy"
            }
        );
        assert!(bootstrap.specialization.endpoint_profile.is_none());
        assert!(bootstrap.process_invocations.is_empty());
        assert!(bootstrap.network_endpoints.is_empty());
        assert!(bootstrap.activated_provider_bindings.is_empty());
        assert_eq!(bootstrap.initial_records.len(), 10);
        assert!(!bootstrap.template_text.contains("Clyffy"));
        assert!(!bootstrap.template_text.contains('\r'));
        let storage = PathBuf::from(&plan.managed_path(Kind::StorageRoot).relative_path);
        // Independent literal paths: do not derive expected intent/staging
        // names by calling the implementation's EstateLayout builder.
        let expected: BTreeSet<_> = [
            ".rrflow".to_owned(),
            ".rrflow/rrd".to_owned(),
            ".rrflow/rrd/roots".to_owned(),
            ".rrflow/credentials".to_owned(),
            storage.to_string_lossy().into_owned(),
            format!("{}/RRD.TOKEN", storage.display()),
            ".rrflow/credentials/local-operator.json".to_owned(),
            ".rrflow/config.toml".to_owned(),
            ".rrflow-install.lock".to_owned(),
            format!(".rrflow-install-{}.json", plan.storage_root_id),
            format!(".rrflow/.config.{}.pending", plan.storage_root_id),
            ".rrflow/bootstrap.md".to_owned(),
            format!(".rrflow/.bootstrap.{}.pending", plan.storage_root_id),
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();
        assert_eq!(
            expected,
            plan.managed_paths
                .iter()
                .map(|path| PathBuf::from(&path.relative_path))
                .collect()
        );
        let applied = RrdEngine::apply_installation_with_test_observer(
            project.path(),
            &preview,
            &executable(),
            &mut |_, _| {
                for path in paths(project.path()) {
                    assert!(
                        path == Path::new("AGENTS.md")
                            || expected.contains(&path)
                            || path.starts_with(&storage),
                        "unpreviewed effect: {}",
                        path.display()
                    );
                }
                assert_eq!(
                    std::fs::read(project.path().join("AGENTS.md")).unwrap(),
                    owned
                );
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(applied.runtime_cursor, 2);
        assert_eq!(applied.control_journal_sequence, 8);
        assert_eq!(
            std::fs::read(project.path().join(".rrflow/bootstrap.md")).unwrap(),
            bootstrap.rendered_instructions().as_bytes()
        );
        for kind in [
            Kind::InstallationIntent,
            Kind::LocatorPending,
            Kind::BootstrapPending,
        ] {
            assert!(!project
                .path()
                .join(&plan.managed_path(kind).relative_path)
                .exists());
        }
        let engine = RrdEngine::open_installed_with_clock(
            project.path(),
            &executable(),
            &FixedClock::at(1_800_000_000_001),
        )
        .unwrap();
        let journal = engine.storage.control().journal_since(0, 32).unwrap();
        assert_eq!(
            journal
                .iter()
                .map(|entry| entry.action.as_str())
                .collect::<Vec<_>>(),
            [
                "security.initialized",
                "security.audit",
                "security.audit.head",
                "configuration.installed",
                "install.record",
                "install.checkpoint",
                "attunement.plan.created",
                "attunement.job.created",
            ]
        );
        assert!(journal
            .iter()
            .all(|entry| entry.before_sha256.is_none() && entry.verify()));
        let installed: Value =
            serde_json::from_slice(journal[4].replacement.as_ref().unwrap()).unwrap();
        assert_eq!(
            installed["installation"]["bootstrap"],
            serde_json::to_value(bootstrap).unwrap()
        );
        let job: Value = serde_json::from_slice(journal[7].replacement.as_ref().unwrap()).unwrap();
        assert_eq!(job["state"], "pending");
        let scope =
            rrd_core::ScopeId::new(format!("instance:{}", plan.target.instance_id)).unwrap();
        let read = engine.storage.runtime().read_stamp(&scope).unwrap();
        let changes = engine
            .storage
            .runtime()
            .read_versioned(
                &read,
                &rrd_store::RuntimeVersionedSource::all(),
                rrd_store::RuntimeReadBudget::new(1024).unwrap(),
            )
            .unwrap();
        assert_eq!(changes.changes.len(), 2);
        let mutations: Vec<_> = changes
            .changes
            .iter()
            .map(|change| &change.mutation)
            .collect();
        assert!(matches!(
            mutations.as_slice(),
            [
                rrd_core::RuntimeMutation::Schema { .. },
                rrd_core::RuntimeMutation::Record { .. }
            ]
        ));
        drop(engine);
        let inspected = rrd_store::RrflowKvInspector::open(&project.path().join(storage)).unwrap();
        assert_eq!(
            inspected.inspection().physical.application_format,
            Some(bootstrap.storage_generation.application_format)
        );
    }
}

#[test]
fn rehashed_bootstrap_drift_is_denied_before_any_project_effect() {
    for case in 0..6 {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("AGENTS.md"), b"owned").unwrap();
        let mut preview = preview_for(project.path(), "default");
        let bootstrap = &mut preview.installation.bootstrap;
        match case {
            0 => bootstrap.template_revision += 1,
            1 => bootstrap.template_text.push_str("\nAltered bundle input\n"),
            2 => bootstrap.specialization.revision += 1,
            3 => bootstrap
                .specialization
                .instructions
                .push_str(" Altered specialization."),
            4 => bootstrap.storage_generation.manifest_format += 1,
            5 => bootstrap.max_record_bytes -= 1,
            _ => unreachable!(),
        }
        bootstrap.template_sha256 = digest::sha256_hex(bootstrap.template_text.as_bytes());
        bootstrap.specialization_sha256 = bootstrap.specialization.digest().unwrap();
        bootstrap.instructions_sha256 =
            digest::sha256_hex(bootstrap.rendered_instructions().as_bytes());
        preview.installation.plan_sha256 =
            rrd_contract::installation_plan_sha256(&preview.installation).unwrap();
        preview.validate().unwrap(); // Structurally valid; not the linked bundle.
        let before = paths(project.path());
        assert!(RrdEngine::apply_installation_at(
            project.path(),
            InstallationTargetKind::ExistingProject,
            &preview,
            &preview.installation.plan_sha256,
            1_800_000_000_000,
            &executable()
        )
        .is_err());
        assert_eq!(paths(project.path()), before);
        assert_eq!(
            std::fs::read(project.path().join("AGENTS.md")).unwrap(),
            b"owned"
        );
    }
}

#[test]
fn altered_installed_instructions_deny_inspection_open_and_replay_without_repair() {
    let project = tempfile::tempdir().unwrap();
    let preview = preview_for(project.path(), "default");
    RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable(),
    )
    .unwrap();
    let bootstrap = project.path().join(".rrflow/bootstrap.md");
    std::fs::write(&bootstrap, b"operator changed the owned bootstrap").unwrap();
    assert!(RrdEngine::inspect_installed_with_clock(
        project.path(),
        &executable(),
        &FixedClock::at(1_800_000_000_001)
    )
    .is_err());
    assert!(RrdEngine::open_installed_with_clock(
        project.path(),
        &executable(),
        &FixedClock::at(1_800_000_000_001)
    )
    .is_err());
    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_001,
        &executable()
    )
    .is_err());
    assert_eq!(
        std::fs::read(bootstrap).unwrap(),
        b"operator changed the owned bootstrap"
    );
}

#[test]
fn linked_bootstrap_bytes_are_platform_stable_and_unknown_profiles_have_no_effects() {
    // Release asset identities deliberately use fixed expected bytes, not a
    // second read through include_bytes! that could repeat checkout drift.
    for (profile, sha256) in [
        (
            "default",
            "4cd907c211671714e33208bc13540a50c59e6b7488184d9e7378d1d5976d3f53",
        ),
        (
            "clyffy",
            "048de57d1a8f5aec86b58cfb1533996f6bad7f6927040d69876c05fc68eeab5e",
        ),
    ] {
        let project = tempfile::tempdir().unwrap();
        let preview = preview_for(project.path(), profile);
        assert_eq!(preview.installation.profile_sha256, sha256);
        assert_eq!(
            preview.installation.bootstrap.template_sha256,
            "2df8396b905776c27f3d5c0d9b178410a792671d1c2fd50eac7c1fe7c98bc6df"
        );
        assert!(paths(project.path()).is_empty());
    }
    let project = tempfile::tempdir().unwrap();
    for profile in [
        "unknown",
        "../profile.json",
        "https://example.invalid/profile.json",
    ] {
        assert!(RrdEngine::plan_installation(
            project.path(),
            InstallationTargetKind::FreshProject,
            &"1".repeat(64),
            profile,
            None,
            &executable()
        )
        .is_err());
        assert!(paths(project.path()).is_empty());
    }
}
