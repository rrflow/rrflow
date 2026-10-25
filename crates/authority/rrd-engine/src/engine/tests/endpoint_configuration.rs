use super::*;
use rrd_contract::{
    ApplyEndpointConfiguration, BindingKind, BindingLimits, CredentialReference, DeclaredFootprint,
    EndpointBinding, EndpointConfiguration, EndpointConfigurationPlan, EndpointLocator,
    EndpointProfileInput, InstalledEstateIdentity, PreviewEndpointConfiguration, SessionLease,
    WorkloadClass,
};

fn bind(engine: &mut RrdEngine, project: &str) {
    engine.installed_estate = Some(InstalledEstateIdentity {
        project_id: CanonicalId::new(project).unwrap(),
        estate_id: CanonicalId::new("endpoint-estate").unwrap(),
        instance_id: instance(),
    });
}
fn profile(name: &str) -> EndpointProfileInput {
    EndpointProfileInput {
        contract_version: 1,
        name: CanonicalId::new(name).unwrap(),
        bindings: vec![EndpointBinding {
            id: CanonicalId::new("knowledge-source").unwrap(),
            class: WorkloadClass::VectorStore,
            kind: BindingKind::Api,
            // An intentionally unreachable address proves configuration needs no probe.
            endpoint: EndpointLocator::Url {
                url: "https://not-a-provider.invalid/".into(),
            },
            service_id: CanonicalId::new("unqualified-vector-service").unwrap(),
            manifest_sha256: None,
            credential: Some(CredentialReference {
                provider: CanonicalId::new("unconfigured-secret-provider").unwrap(),
                name: "knowledge-read-key".into(),
            }),
            placement: None,
            footprint: DeclaredFootprint {
                memory_bytes: 1,
                gpu: rrd_contract::GpuRequirement::None,
            },
            limits: BindingLimits {
                max_concurrency: 1,
                timeout_ms: 1_000,
                max_input_bytes: 4096,
            },
        }],
    }
}
fn session(engine: &RrdEngine, at: u64) -> SessionLease {
    engine
        .create_session(
            &session_request(5_000, 2),
            &id(&format!("endpoint-session-{at}")),
            at,
            "session-request",
            "session-operation",
        )
        .unwrap()
}
fn preview(
    engine: &RrdEngine,
    lease: &SessionLease,
    revision: u64,
    name: Option<&str>,
) -> EndpointConfigurationPlan {
    engine
        .preview_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: revision,
                profile: name.map(profile),
            },
            1_001,
            "preview-request",
            "preview-operation",
        )
        .unwrap()
}
fn apply(
    engine: &RrdEngine,
    lease: &SessionLease,
    plan: &EndpointConfigurationPlan,
) -> crate::Result<EndpointConfiguration> {
    engine.apply_endpoint_configuration(
        &lease.session_id,
        &lease.token,
        &ApplyEndpointConfiguration {
            plan: plan.clone(),
            approved_plan_sha256: plan.plan_sha256.clone(),
        },
        1_002,
        "apply-request",
        "apply-operation",
    )
}
fn read(engine: &RrdEngine, lease: &SessionLease) -> EndpointConfiguration {
    engine
        .read_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            1_003,
            "read-request",
            "read-operation",
        )
        .unwrap()
}

#[test]
fn endpoint_configuration_flow_rejects_stale_replay_and_persists_revocation() {
    let (root, persistent) = isolated_engine();
    for (mut engine, reopen) in [
        (RrdEngine::rrflow_mx(instance(), TOKEN_KEY), false),
        (persistent, true),
    ] {
        bind(&mut engine, "endpoint-project");
        let lease = session(&engine, 1_000);
        let initial = read(&engine, &lease);
        let first = preview(&engine, &lease, 0, Some("first"));
        let competing = preview(&engine, &lease, 0, Some("competing"));
        assert_eq!(
            read(&engine, &lease),
            initial,
            "preview changed canonical profile"
        );
        let configured = apply(&engine, &lease, &first).unwrap();
        assert_eq!(configured.revision, 1);
        assert_eq!(
            apply(&engine, &lease, &first).unwrap(),
            configured,
            "lost ACK retry changed the head"
        );
        assert!(matches!(
            apply(&engine, &lease, &competing),
            Err(ServiceError::StorageConflict(_))
        ));
        if reopen {
            drop(engine);
            engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
            bind(&mut engine, "endpoint-project");
            assert_eq!(read(&engine, &lease), configured);
            assert_eq!(apply(&engine, &lease, &first).unwrap(), configured);
        }
        let revoked = apply(&engine, &lease, &preview(&engine, &lease, 1, None)).unwrap();
        assert_eq!(revoked.revision, 2);
        assert!(revoked.profile.is_none());
        assert!(
            matches!(
                apply(&engine, &lease, &first),
                Err(ServiceError::StorageConflict(_))
            ),
            "old plan resurrected revoked binding"
        );
        if reopen {
            drop(engine);
            engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
            bind(&mut engine, "endpoint-project");
            assert_eq!(read(&engine, &lease), revoked);
        }
        let replacement = apply(
            &engine,
            &lease,
            &preview(&engine, &lease, 2, Some("replacement")),
        )
        .unwrap();
        assert_eq!(replacement.revision, 3);
        assert_eq!(replacement.profile.unwrap().name.as_str(), "replacement");
    }
}

#[test]
fn endpoint_configuration_rejects_foreign_tampered_unapproved_and_expired_plans() {
    let mut engine = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    bind(&mut engine, "endpoint-project");
    let lease = session(&engine, 1_000);
    let plan = preview(&engine, &lease, 0, Some("first"));
    let before = read(&engine, &lease);
    let mut foreign = plan.clone();
    foreign.identity.project_id = CanonicalId::new("different-project").unwrap();
    foreign.plan_sha256 = foreign.sha256().unwrap();
    assert!(matches!(
        apply(&engine, &lease, &foreign),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    let mut tampered = plan.clone();
    tampered.profile.as_mut().unwrap().bindings[0]
        .limits
        .max_concurrency += 1;
    assert!(apply(&engine, &lease, &tampered).is_err());
    let request = ApplyEndpointConfiguration {
        plan: plan.clone(),
        approved_plan_sha256: "0".repeat(64),
    };
    assert!(engine
        .apply_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &request,
            1_002,
            "bad-approval",
            "bad-approval"
        )
        .is_err());
    assert!(engine
        .apply_endpoint_configuration(
            &lease.session_id,
            &id("wrong-token"),
            &ApplyEndpointConfiguration {
                plan: plan.clone(),
                approved_plan_sha256: plan.plan_sha256.clone()
            },
            1_002,
            "bad-token",
            "bad-token"
        )
        .is_err());
    assert_eq!(read(&engine, &lease), before);
    let later = session(&engine, plan.expires_at_unix_ms);
    assert!(matches!(
        engine.apply_endpoint_configuration(
            &later.session_id,
            &later.token,
            &ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan: plan.clone()
            },
            plan.expires_at_unix_ms,
            "expired",
            "expired"
        ),
        Err(ServiceError::DeadlineExceeded)
    ));
    // Expiry failure leaves the state absent, including its control key.
    assert!(engine
        .storage
        .control()
        .get(&format!(
            "server/state/{}/endpoint-configuration/active",
            instance()
        ))
        .unwrap()
        .is_none());
}

#[test]
fn endpoint_configuration_concurrent_apply_has_one_winner() {
    let (_root, mut engine) = isolated_engine();
    bind(&mut engine, "endpoint-project");
    let lease = session(&engine, 1_000);
    let a = preview(&engine, &lease, 0, Some("first"));
    let b = preview(&engine, &lease, 0, Some("second"));
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let left = scope.spawn(|| {
            barrier.wait();
            apply(&engine, &lease, &a)
        });
        let right = scope.spawn(|| {
            barrier.wait();
            apply(&engine, &lease, &b)
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
    assert_eq!(read(&engine, &lease).revision, 1);
}

#[test]
fn endpoint_configuration_read_permission_does_not_allow_preview_or_apply() {
    use std::collections::{BTreeMap, BTreeSet};
    let mut engine = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    bind(&mut engine, "endpoint-project");
    let bootstrap = session(&engine, 1_000);
    let plan = preview(&engine, &bootstrap, 0, Some("first"));
    let principal_id = CanonicalId::new("configuration-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let principal = Principal {
        id: principal_id.clone(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"reader-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: BTreeSet::new(),
        grants: [SecurityAction::SessionCreate, SecurityAction::EstateRead]
            .into_iter()
            .map(|action| ResourceGrant {
                action,
                resource_prefix: resource.clone(),
                data_policy: None,
            })
            .collect(),
    };
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([(principal_id.clone(), principal)]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1_000,
            "test",
            "security-request",
            "security-operation",
        )
        .unwrap();
    let reader = engine
        .create_authenticated_session(
            &principal_id,
            b"reader-key",
            &session_request(5_000, 2),
            &id("reader-session"),
            1_000,
            "reader-request",
            "reader-operation",
        )
        .unwrap();
    assert_eq!(read(&engine, &reader).revision, 0);
    assert!(engine
        .preview_endpoint_configuration(
            &reader.session_id,
            &reader.token,
            &PreviewEndpointConfiguration {
                expected_revision: 0,
                profile: Some(profile("unauthorized"))
            },
            1_001,
            "denied-preview",
            "denied-preview"
        )
        .is_err());
    assert!(apply(&engine, &reader, &plan).is_err());
    assert_eq!(read(&engine, &reader).revision, 0);
}
