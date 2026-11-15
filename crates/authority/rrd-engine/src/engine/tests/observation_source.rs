use super::*;
use crate::engine::clock::FixedClock;
use rrd_contract::{
    ApplyObservationSourceConfiguration, InstallationTargetKind, ObservationContentScope,
    ObservationSourceBinding, ObservationTransport, PreviewObservationSourceConfiguration,
};
use std::path::Path;

#[test]
fn installed_source_binding_preview_apply_restart_revoke_and_replay() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "project").unwrap();
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
    RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();
    let mut engine = RrdEngine::open_installed_with_clock(
        project.path(),
        &executable,
        &FixedClock::at(1_800_000_001_000),
    )
    .unwrap();
    let key = RrdEngine::read_installed_api_key(project.path()).unwrap();
    let lease = engine
        .create_authenticated_session(
            &key.principal_id,
            key.credential().as_bytes(),
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 60_000,
                    max_open_transactions: 1,
                },
            },
            &id("source-session"),
            1_800_000_001_000,
            "session",
            "session",
        )
        .unwrap();
    let now = 1_800_000_001_001;
    let read = |engine: &RrdEngine| {
        engine
            .read_observation_source_configuration_at(
                &lease.session_id,
                &lease.token,
                now,
                "read",
                "read",
            )
            .unwrap()
    };
    let initial = read(&engine);
    let source = ObservationSourceBinding {
        id: CanonicalId::new("codex-local").unwrap(),
        producer: CanonicalId::new("codex").unwrap(),
        collector_principal: CanonicalId::new("collector").unwrap(),
        transport: ObservationTransport::OtlpLogs,
        content_scope: ObservationContentScope::MetadataOnly,
        max_event_bytes: 4096,
        max_batch_events: 10,
        max_batch_bytes: 40960,
    };
    let make = |engine: &RrdEngine, revision, sources| {
        engine
            .preview_observation_source_configuration_at(
                &lease.session_id,
                &lease.token,
                &PreviewObservationSourceConfiguration {
                    expected_revision: revision,
                    sources,
                },
                now,
                "preview",
                "preview",
            )
            .unwrap()
    };
    let other_lease = engine
        .create_authenticated_session(
            &key.principal_id,
            key.credential().as_bytes(),
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 600_000,
                    absolute_timeout_ms: 600_000,
                    max_open_transactions: 1,
                },
            },
            &id("other-session"),
            now,
            "other-session",
            "other-session",
        )
        .unwrap();
    let first = make(&engine, 0, vec![source.clone()]);
    assert!(engine
        .apply_observation_source_configuration_at(
            &other_lease.session_id,
            &other_lease.token,
            &ApplyObservationSourceConfiguration {
                plan: first.clone(),
                approved_plan_sha256: first.plan_sha256.clone()
            },
            now + 1,
            "cross-session",
            "cross-session"
        )
        .is_err());
    let competing = make(&engine, 0, vec![]);
    let mut tampered = first.clone();
    tampered.sources[0].max_event_bytes += 1;
    let request = ApplyObservationSourceConfiguration {
        plan: tampered,
        approved_plan_sha256: first.plan_sha256.clone(),
    };
    assert!(engine
        .apply_observation_source_configuration_at(
            &lease.session_id,
            &lease.token,
            &request,
            now + 1,
            "tampered",
            "tampered"
        )
        .is_err());
    let mut foreign = first.clone();
    foreign.identity.project_id = CanonicalId::new("foreign-project").unwrap();
    foreign.plan_sha256 = foreign.sha256().unwrap();
    let mut wrong_session = first.clone();
    wrong_session.session_id = id("different-session");
    wrong_session.plan_sha256 = wrong_session.sha256().unwrap();
    let mut forged = first.clone();
    forged.sources[0].content_scope = ObservationContentScope::ContentAuthorized;
    forged.plan_sha256 = forged.sha256().unwrap();
    let invalid_duplicate = PreviewObservationSourceConfiguration {
        expected_revision: 0,
        sources: vec![source.clone(), source.clone()],
    };
    assert!(invalid_duplicate.validate().is_err());
    let mut invalid_limit = source.clone();
    invalid_limit.max_event_bytes = 0;
    assert!(PreviewObservationSourceConfiguration {
        expected_revision: 0,
        sources: vec![invalid_limit]
    }
    .validate()
    .is_err());
    let mut oversized = source;
    oversized.max_batch_bytes = 8_388_609;
    assert!(PreviewObservationSourceConfiguration {
        expected_revision: 0,
        sources: vec![oversized]
    }
    .validate()
    .is_err());
    assert!(serde_json::from_str::<ObservationTransport>("\"unknown\"").is_err());
    assert!(engine
        .read_observation_source_configuration_at(
            &lease.session_id,
            &id("wrong-token"),
            now,
            "denied",
            "denied"
        )
        .is_err());
    assert_eq!(read(&engine), initial);
    let apply = |engine: &RrdEngine, plan: &rrd_contract::ObservationSourceConfigurationPlan| {
        engine.apply_observation_source_configuration_at(
            &lease.session_id,
            &lease.token,
            &ApplyObservationSourceConfiguration {
                plan: plan.clone(),
                approved_plan_sha256: plan.plan_sha256.clone(),
            },
            now + 1,
            "apply",
            "apply",
        )
    };
    assert!(matches!(
        apply(&engine, &foreign),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    assert!(apply(&engine, &wrong_session).is_err());
    assert!(apply(&engine, &forged).is_err());
    let configured = apply(&engine, &first).unwrap();
    assert!(matches!(
        apply(&engine, &competing),
        Err(ServiceError::StorageConflict(_))
    ));
    assert_eq!(configured.revision, 1);
    assert_eq!(apply(&engine, &first).unwrap(), configured);
    drop(engine);
    engine = RrdEngine::open_installed_with_clock(
        project.path(),
        &executable,
        &FixedClock::at(1_800_000_001_002),
    )
    .unwrap();
    assert_eq!(read(&engine), configured);
    let revoked = apply(&engine, &make(&engine, 1, vec![])).unwrap();
    assert_eq!(revoked.revision, 2);
    assert!(revoked.sources.is_empty());
    assert!(matches!(
        apply(&engine, &first),
        Err(ServiceError::StorageConflict(_))
    ));
    let audit = engine
        .read_audit(
            &lease.session_id,
            &lease.token,
            &rrd_contract::ReadAudit {
                after_sequence: 0,
                limit: 1024,
            },
            now + 1,
            "audit-read",
            "audit-read",
        )
        .unwrap();
    assert!(audit
        .records
        .iter()
        .any(|record| record.operation_id == "apply"
            && record.action == SecurityAction::EngramAdmin
            && record.decision == AuditDecision::Allowed));
    let entries = engine.storage.control().journal_since(0, 1024).unwrap();
    let actions: Vec<_> = entries
        .iter()
        .filter(|e| e.action.starts_with("observation.sources."))
        .map(|e| e.action.as_str())
        .collect();
    assert_eq!(
        actions,
        vec!["observation.sources.applied", "observation.sources.revoked"]
    );
    drop(engine);
    let reopened = RrdEngine::open_installed_with_clock(
        project.path(),
        &executable,
        &FixedClock::at(1_800_000_001_003),
    )
    .unwrap();
    assert_eq!(read(&reopened), revoked);
    let persisted = reopened.storage.control().journal_since(0, 1024).unwrap();
    assert_eq!(
        persisted
            .iter()
            .filter(|e| e.action.starts_with("observation.sources."))
            .count(),
        2
    );
    let first_candidate = make(&reopened, 2, vec![]);
    let second_candidate = reopened
        .preview_observation_source_configuration_at(
            &lease.session_id,
            &lease.token,
            &PreviewObservationSourceConfiguration {
                expected_revision: 2,
                sources: vec![],
            },
            now + 1,
            "preview-other",
            "preview-other",
        )
        .unwrap();
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let left = scope.spawn(|| {
            barrier.wait();
            apply(&reopened, &first_candidate)
        });
        let right = scope.spawn(|| {
            barrier.wait();
            apply(&reopened, &second_candidate)
        });
        [left.join().unwrap(), right.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(ServiceError::StorageConflict(_))))
            .count(),
        1
    );
    assert_eq!(read(&reopened).revision, 3);
    let expiring = reopened
        .preview_observation_source_configuration_at(
            &other_lease.session_id,
            &other_lease.token,
            &PreviewObservationSourceConfiguration {
                expected_revision: 3,
                sources: vec![],
            },
            now + 2,
            "expiry-preview",
            "expiry-preview",
        )
        .unwrap();
    assert!(matches!(
        reopened.apply_observation_source_configuration_at(
            &other_lease.session_id,
            &other_lease.token,
            &ApplyObservationSourceConfiguration {
                approved_plan_sha256: expiring.plan_sha256.clone(),
                plan: expiring.clone()
            },
            expiring.expires_at_unix_ms,
            "expiry-apply",
            "expiry-apply"
        ),
        Err(ServiceError::DeadlineExceeded)
    ));
    // An authenticated reader can inspect configuration, but cannot preview or apply it.
    let repository = SecurityRepository::new(&reopened.storage, reopened.instance.clone());
    let mut security = repository.load().unwrap().unwrap();
    let revision = security.revision;
    let reader_id = CanonicalId::new("source-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![
            ResourceId::new(ResourceKind::Instance, reopened.instance.as_str()).unwrap(),
        ],
    };
    security.principals.insert(
        reader_id.clone(),
        Principal {
            id: reader_id.clone(),
            kind: PrincipalKind::Service,
            credential_sha256: digest::sha256_hex(b"reader-key"),
            credential_revision: 1,
            not_before_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            disabled: false,
            role_ids: std::collections::BTreeSet::new(),
            grants: [SecurityAction::SessionCreate, SecurityAction::EngramRead]
                .into_iter()
                .map(|action| ResourceGrant {
                    action,
                    resource_prefix: resource.clone(),
                    data_policy: None,
                })
                .collect(),
        },
    );
    security.revision += 1;
    repository
        .replace(
            revision,
            security,
            now + 3,
            "test",
            "reader-policy",
            "reader-policy",
        )
        .unwrap();
    let reader = reopened
        .create_authenticated_session(
            &reader_id,
            b"reader-key",
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 60_000,
                    max_open_transactions: 1,
                },
            },
            &id("reader-session"),
            now + 3,
            "reader-session",
            "reader-session",
        )
        .unwrap();
    assert_eq!(
        reopened
            .read_observation_source_configuration_at(
                &reader.session_id,
                &reader.token,
                now + 4,
                "reader",
                "reader"
            )
            .unwrap()
            .revision,
        3
    );
    assert!(matches!(
        reopened.preview_observation_source_configuration_at(
            &reader.session_id,
            &reader.token,
            &PreviewObservationSourceConfiguration {
                expected_revision: 3,
                sources: vec![]
            },
            now + 4,
            "reader-preview",
            "reader-preview"
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(matches!(
        reopened.apply_observation_source_configuration_at(
            &reader.session_id,
            &reader.token,
            &ApplyObservationSourceConfiguration {
                plan: expiring.clone(),
                approved_plan_sha256: expiring.plan_sha256.clone()
            },
            now + 4,
            "reader-apply",
            "reader-apply"
        ),
        Err(ServiceError::PermissionDenied)
    ));
}

#[test]
fn public_source_preview_uses_host_clock_on_an_installed_engram() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let time = || {
        u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
    };
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "project").unwrap();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let before = time();
    let install = RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &"1".repeat(64),
        "default",
        None,
        &executable,
    )
    .unwrap();
    RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &install,
        &install.installation.plan_sha256,
        before,
        &executable,
    )
    .unwrap();
    let engine = RrdEngine::open_installed(project.path(), &executable).unwrap();
    let key = RrdEngine::read_installed_api_key(project.path()).unwrap();
    let lease = engine
        .create_authenticated_session(
            &key.principal_id,
            key.credential().as_bytes(),
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 60_000,
                    max_open_transactions: 1,
                },
            },
            &id("host-clock-session"),
            time(),
            "session",
            "session",
        )
        .unwrap();
    let started = time();
    let plan = engine
        .preview_observation_source_configuration(
            &lease.session_id,
            &lease.token,
            &PreviewObservationSourceConfiguration {
                expected_revision: 0,
                sources: vec![],
            },
            "preview",
            "preview",
        )
        .unwrap();
    let ended = time();
    assert!(started <= plan.issued_at_unix_ms && plan.issued_at_unix_ms <= ended);
    let state = engine
        .apply_observation_source_configuration(
            &lease.session_id,
            &lease.token,
            &ApplyObservationSourceConfiguration {
                plan: plan.clone(),
                approved_plan_sha256: plan.plan_sha256.clone(),
            },
            "apply",
            "apply",
        )
        .unwrap();
    assert_eq!(state.revision, 1);
    assert_eq!(
        engine
            .read_observation_source_configuration(&lease.session_id, &lease.token, "read", "read")
            .unwrap(),
        state
    );
}
