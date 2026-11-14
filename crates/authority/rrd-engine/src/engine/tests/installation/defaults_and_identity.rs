//! Page cache defaults, admitted profiles, installation identity and canonical planning.

use super::*;

/// The lab measured search against this page cache. Everything that states a
/// default has to agree, and changing it should come with a new measurement.
#[test]
pub(super) fn every_default_page_cache_is_the_one_search_was_measured_with() {
    const MEASURED: u64 = 64 * 1024 * 1024;
    let bundled = crate::load_estate_configuration(None).unwrap();
    assert_eq!(
        bundled.storage.page_cache_bytes, MEASURED,
        "the bundled configuration"
    );
    assert_eq!(
        rrd_contract::StorageLimits::default().page_cache_bytes,
        MEASURED,
        "the contract's default"
    );
    assert_eq!(
        rrd_store::DatabaseOptions::default().page_cache_bytes as u64,
        MEASURED,
        "the storage engine's default"
    );
    let installed = crate::engine::core::installed_storage_options(&bundled.storage).unwrap();
    assert_eq!(
        installed.page_cache_bytes as u64, MEASURED,
        "what a default install opens with"
    );
}

#[test]
pub(super) fn a_configuration_still_chooses_its_own_page_cache() {
    let mut storage = crate::load_estate_configuration(None).unwrap().storage;
    for chosen in [0, 1024 * 1024, 1024 * 1024 * 1024] {
        storage.page_cache_bytes = chosen;
        let installed = crate::engine::core::installed_storage_options(&storage).unwrap();
        assert_eq!(installed.page_cache_bytes as u64, chosen);
    }
}

#[test]
pub(super) fn installed_engine_retains_only_its_admitted_project_root_across_reopen() {
    use std::io::Read;
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let executable = test_executable();
    for (project, content) in [(first.path(), "first"), (second.path(), "second")] {
        std::fs::write(project.join("source.txt"), content).unwrap();
        let preview = RrdEngine::plan_installation(
            project,
            InstallationTargetKind::ExistingProject,
            &RrdEngine::generate_installation_identity().unwrap(),
            "default",
            None,
            &executable,
        )
        .unwrap();
        RrdEngine::apply_installation_at(
            project,
            InstallationTargetKind::ExistingProject,
            &preview,
            &preview.installation.plan_sha256,
            1_800_000_000_000,
            &executable,
        )
        .unwrap();
    }
    let first_engine = open_at(first.path(), &executable, 1_800_000_001_000).unwrap();
    let second_engine = open_at(second.path(), &executable, 1_800_000_001_000).unwrap();
    let read = |engine: &RrdEngine| {
        let root = engine
            .installed_project_root
            .as_ref()
            .expect("installed root is retained");
        assert!(root.join("../outside.txt").open_read().is_err());
        let mut content = String::new();
        root.join("source.txt")
            .open_read()
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        content
    };
    assert_eq!(read(&first_engine), "first");
    assert_eq!(read(&second_engine), "second");
    std::fs::write(first.path().join("source.txt"), "first edited").unwrap();
    assert_eq!(read(&first_engine), "first edited");
    assert_eq!(read(&second_engine), "second");
    drop(first_engine);
    let reopened = open_at(first.path(), &executable, 1_800_000_002_000).unwrap();
    assert_eq!(read(&reopened), "first edited");
    assert!(
        RrdEngine::rrflow_mx(CanonicalId::new("volatile").unwrap(), [0; 32])
            .installed_project_root
            .is_none()
    );
    let (_component_root, component) = isolated_engine();
    assert!(component.installed_project_root.is_none());
}

#[test]
pub(super) fn independent_installations_do_not_reuse_content_identity() {
    for mode in [
        InstallationTargetKind::FreshProject,
        InstallationTargetKind::ExistingProject,
    ] {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        if mode == InstallationTargetKind::ExistingProject {
            for project in [first.path(), second.path()] {
                std::fs::write(project.join("README.md"), "identical source\n").unwrap();
            }
        }
        let executable = test_executable();
        let first_plan = RrdEngine::plan_installation(
            first.path(),
            mode,
            &RrdEngine::generate_installation_identity().unwrap(),
            "default",
            None,
            &executable,
        )
        .unwrap();
        let second_plan = RrdEngine::plan_installation(
            second.path(),
            mode,
            &RrdEngine::generate_installation_identity().unwrap(),
            "default",
            None,
            &executable,
        )
        .unwrap();
        assert_eq!(
            first_plan.installation.project_precondition_sha256,
            second_plan.installation.project_precondition_sha256
        );
        assert_ne!(
            first_plan.installation.target, second_plan.installation.target,
            "distinct projects with identical content must not share an estate identity"
        );
        let before = inventory(second.path());
        assert!(matches!(
            RrdEngine::apply_installation_at(
                second.path(),
                mode,
                &first_plan,
                &first_plan.installation.plan_sha256,
                1_800_000_000_000,
                &executable,
            ),
            Err(ServiceError::ProjectBindingMismatch)
        ));
        assert_eq!(
            inventory(second.path()),
            before,
            "foreign plan must have no effects"
        );
        for (project, preview) in [(first.path(), &first_plan), (second.path(), &second_plan)] {
            RrdEngine::apply_installation_at(
                project,
                mode,
                preview,
                &preview.installation.plan_sha256,
                1_800_000_000_000,
                &executable,
            )
            .unwrap();
            let reopened = open_at(project, &executable, 1_800_000_000_001).unwrap();
            assert_eq!(
                reopened.installed_estate_identity(),
                Some(&preview.installation.target)
            );
        }
        let first_key = RrdEngine::read_installed_api_key(first.path()).unwrap();
        let second_key = RrdEngine::read_installed_api_key(second.path()).unwrap();
        assert_ne!(first_key.credential(), second_key.credential());
        let engine = open_at(second.path(), &executable, 1_800_000_000_001).unwrap();
        let session = rrd_contract::CreateSession {
            limits: rrd_contract::SessionLimits {
                idle_timeout_ms: 30_000,
                absolute_timeout_ms: 60_000,
                max_open_transactions: 1,
            },
        };
        assert!(engine
            .create_authenticated_session(
                &first_key.principal_id,
                first_key.credential().as_bytes(),
                &session,
                &id("foreign-session"),
                1_800_000_000_002,
                "foreign-request",
                "foreign-operation",
            )
            .is_err());
        assert!(engine
            .create_authenticated_session(
                &second_key.principal_id,
                second_key.credential().as_bytes(),
                &session,
                &id("own-session"),
                1_800_000_000_003,
                "own-request",
                "own-operation",
            )
            .is_ok());
    }
}

#[test]
pub(super) fn installation_identity_and_root_are_bound_before_effects() {
    let project = tempfile::tempdir().unwrap();
    let executable = test_executable();
    for invalid in [
        String::new(),
        "0".repeat(63),
        "A".repeat(64),
        "z".repeat(64),
    ] {
        assert!(RrdEngine::plan_installation(
            project.path(),
            InstallationTargetKind::FreshProject,
            &invalid,
            "default",
            None,
            &executable
        )
        .is_err());
        assert!(inventory(project.path()).is_empty());
    }
    let preview = plan(project.path());
    for change_identity in [true, false] {
        let mut forged = preview.clone();
        if change_identity {
            forged.installation.installation_identity = "2".repeat(64);
        } else {
            forged.installation.project_root_sha256 = "2".repeat(64);
        }
        forged.installation.plan_sha256 = installation_plan_sha256(&forged.installation).unwrap();
        assert!(RrdEngine::apply_installation_at(
            project.path(),
            InstallationTargetKind::ExistingProject,
            &forged,
            &forged.installation.plan_sha256,
            1_800_000_000_000,
            &executable
        )
        .is_err());
        assert!(inventory(project.path()).is_empty());
    }
}

#[test]
pub(super) fn planning_is_byte_identical_and_has_no_project_effect() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    let before = inventory(project.path());
    let first = plan(project.path());
    let second = plan(project.path());
    assert_eq!(
        first.installation.initial_grants,
        SecurityAction::ALL
            .into_iter()
            .filter(|action| *action != SecurityAction::ProjectSourceRead)
            .collect::<Vec<_>>()
    );
    assert_eq!(first.installation.managed_paths.len(), 13);
    assert!(serde_json::to_value(&first.installation)
        .unwrap()
        .get("project_root")
        .is_none());
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
    assert!(serde_json::to_value(&first)
        .unwrap()
        .get("source_read_resource_scope")
        .is_none());
    assert_eq!(inventory(project.path()), before);
}

#[test]
pub(super) fn source_read_is_an_explicit_canonical_installation_choice() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    let default = plan_with_source_read(project.path(), false);
    let legacy_default_entrypoint = plan(project.path());
    let opted_in = plan_with_source_read(project.path(), true);
    assert_eq!(default, legacy_default_entrypoint);
    assert_eq!(
        serde_json::to_vec(&default).unwrap(),
        serde_json::to_vec(&legacy_default_entrypoint).unwrap()
    );
    assert_eq!(
        default.installation.initial_grants,
        rrd_contract::installation_initial_grants(false)
    );
    assert_eq!(
        opted_in.installation.initial_grants,
        rrd_contract::installation_initial_grants(true)
    );
    assert!(default.source_read_resource_scope.is_none());
    assert_eq!(
        opted_in.source_read_resource_scope.as_deref(),
        Some("instance_prefix")
    );
    assert_ne!(
        default.installation.plan_sha256,
        opted_in.installation.plan_sha256
    );
    let before = inventory(project.path());
    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &opted_in,
        &"0".repeat(64),
        1_800_000_000_000,
        &test_executable()
    )
    .is_err());
    assert_eq!(inventory(project.path()), before);

    let mut forged = opted_in.clone();
    forged.installation.initial_grants = rrd_contract::installation_initial_grants(false);
    forged
        .installation
        .initial_grants
        .retain(|action| *action != SecurityAction::ServiceInspect);
    forged
        .installation
        .initial_grants
        .push(SecurityAction::ProjectSourceRead);
    forged.installation.initial_grants.sort();
    forged.installation.initial_grants.dedup();
    forged.installation.plan_sha256 = installation_plan_sha256(&forged.installation).unwrap();
    forged.validate().unwrap();
    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &forged,
        &forged.installation.plan_sha256,
        1_800_000_000_000,
        &test_executable()
    )
    .is_err());
    assert_eq!(inventory(project.path()), before);

    let mut missing_scope = opted_in;
    missing_scope.source_read_resource_scope = None;
    assert!(missing_scope.validate().is_err());
}

#[test]
pub(super) fn configuration_input_rejects_unknown_oversized_and_symbolic_sources() {
    let invalid = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        invalid.path(),
        "format_version = 1\nunknown_authority = true\n",
    )
    .unwrap();
    assert!(crate::load_estate_configuration(Some(invalid.path())).is_err());

    let oversized = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(oversized.path(), vec![b' '; 64 * 1024 + 1]).unwrap();
    assert!(crate::load_estate_configuration(Some(oversized.path())).is_err());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("configuration.toml");
        let symbolic = directory.path().join("selected.toml");
        std::fs::write(&target, "format_version = 1\n").unwrap();
        symlink(target, &symbolic).unwrap();
        assert!(crate::load_estate_configuration(Some(&symbolic)).is_err());
    }
}
