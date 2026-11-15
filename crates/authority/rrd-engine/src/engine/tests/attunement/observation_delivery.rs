//! Installed-engine protocol/fault tests. These fixtures are not model or training data.
use super::*;
use crate::EndpointCredentials;
use rrd_contract::{
    ApplyEndpointConfiguration, BindingKind, BindingLimits, CredentialReference, DeclaredFootprint,
    DeliverObservation, EndpointBinding, EndpointConfiguration, EndpointLocator,
    EndpointProfileInput, GpuRequirement, ObservationDeliveryDisposition as Disposition,
    ObservationDeliverySnapshot, ObservationDeliveryState as DeliveryState,
    ObservationExportPolicy, ObservationSource, PrepareObservationDelivery,
    PreviewEndpointConfiguration, ReadAttunement, ReadObservationDelivery, WorkloadClass,
    ATTUNEMENT_CONTRACT_VERSION,
};
use rrd_observation::TransportObservation;

fn profile(engine: &RrdEngine, retained: u32) -> EndpointProfileInput {
    EndpointProfileInput {
        contract_version: 1,
        name: CanonicalId::new("observation-test").unwrap(),
        selection_policy: None,
        worker_policy: None,
        bindings: vec![EndpointBinding {
            id: CanonicalId::new("observation-sink").unwrap(),
            class: WorkloadClass::BehaviorUpdates,
            kind: BindingKind::Api,
            endpoint: EndpointLocator::Url {
                url: "http://127.0.0.1:9/".into(),
            },
            service_id: CanonicalId::new("protocol-fixture").unwrap(),
            manifest_sha256: None,
            credential: Some(credential()),
            placement: None,
            footprint: DeclaredFootprint {
                memory_bytes: 1,
                gpu: GpuRequirement::None,
            },
            limits: BindingLimits {
                max_concurrency: 1,
                timeout_ms: 100,
                max_input_bytes: 16384,
            },
        }],
        observation_policy: Some(ObservationExportPolicy {
            contract_version: 1,
            binding_id: CanonicalId::new("observation-sink").unwrap(),
            expected_devspace: "test-devspace".into(),
            protocol_evidence_sha256: digest::sha256_hex(b"test protocol"),
            allow_network: true,
            allow_metadata_export: true,
            allowed_scopes: vec![format!("instance:{}", engine.instance)],
            max_retained_deliveries: retained,
            retention_ms: 10_000,
            retire_expired_uncertain: false,
            max_attempts: 3,
            retry_delay_ms: 10,
            max_response_bytes: 1024,
        }),
        assessment_policy: None,
    }
}
fn credential() -> CredentialReference {
    CredentialReference {
        provider: CanonicalId::new("test-process-registry").unwrap(),
        name: "export".into(),
    }
}
fn install_secret(engine: &mut RrdEngine) {
    engine
        .install_endpoint_credentials(
            EndpointCredentials::new(vec![(credential(), "protocol-test-secret".into())]).unwrap(),
        )
        .unwrap();
}
fn configure(
    engine: &RrdEngine,
    session: &SessionLease,
    profile: Option<EndpointProfileInput>,
    at: u64,
) -> EndpointConfiguration {
    let current = engine
        .read_endpoint_configuration(
            &session.session_id,
            &session.token,
            at,
            "read-config",
            "read-config",
        )
        .unwrap();
    let plan = engine
        .preview_endpoint_configuration(
            &session.session_id,
            &session.token,
            &PreviewEndpointConfiguration {
                expected_revision: current.revision,
                profile,
            },
            at,
            "preview-config",
            "preview-config",
        )
        .unwrap();
    engine
        .apply_endpoint_configuration(
            &session.session_id,
            &session.token,
            &ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan,
            },
            at + 1,
            "apply-config",
            "apply-config",
        )
        .unwrap()
}
fn setup(
    retained: u32,
) -> (
    Installed,
    RrdEngine,
    SessionLease,
    PrepareObservationDelivery,
) {
    let (installed, mut engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    install_secret(&mut engine);
    let configuration = configure(
        &engine,
        &session,
        Some(profile(&engine, retained)),
        BASE + 200,
    );
    let request = PrepareObservationDelivery {
        contract_version: 1,
        delivery_id: id("delivery-one"),
        expected_configuration_sha256: configuration.sha256().unwrap(),
        source: ObservationSource::AttunementJob {
            request: ReadAttunement {
                contract_version: ATTUNEMENT_CONTRACT_VERSION,
                job_id: installed.job.clone(),
            },
            expected_revision: 1,
            expected_plan_sha256: installed.preview.attunement.plan_sha256.clone(),
        },
    };
    (installed, engine, session, request)
}
fn prepare(
    engine: &RrdEngine,
    session: &SessionLease,
    request: &PrepareObservationDelivery,
    at: u64,
) -> crate::Result<ObservationDeliverySnapshot> {
    engine.prepare_observation_delivery_at(
        &session.session_id,
        &session.token,
        request,
        at,
        "prepare-observation",
        "prepare-observation",
    )
}
fn delivery(snapshot: &ObservationDeliverySnapshot) -> DeliverObservation {
    DeliverObservation {
        contract_version: 1,
        delivery_id: snapshot.delivery_id.clone(),
        expected_payload_sha256: snapshot.payload_sha256.clone(),
        expected_configuration_sha256: snapshot.configuration_sha256.clone(),
    }
}
fn read(engine: &RrdEngine, session: &SessionLease, at: u64) -> ObservationDeliverySnapshot {
    engine
        .read_observation_delivery_at(
            &session.session_id,
            &session.token,
            &ReadObservationDelivery {
                contract_version: 1,
                delivery_id: id("delivery-one"),
            },
            at,
            "read-observation",
            "read-observation",
        )
        .unwrap()
        .delivery
        .unwrap()
}
fn acknowledgement() -> TransportObservation {
    TransportObservation {
        disposition: Disposition::Acknowledged,
        request_may_have_been_sent: true,
        http_status: Some(200),
        reply_sha256: Some(digest::sha256_hex(b"protocol acknowledgement")),
    }
}

#[test]
fn observation_delivery_reduced_retention_cap_retires_expired_history_in_bounded_batches() {
    let (installed, engine, session, request) = setup(128);
    for number in 0..128 {
        let mut request = request.clone();
        request.delivery_id = id(&format!("expired-{number}"));
        prepare(&engine, &session, &request, BASE + 300 + number).unwrap();
    }
    let configuration = configure(&engine, &session, Some(profile(&engine, 1)), BASE + 11_000);
    let request = PrepareObservationDelivery {
        expected_configuration_sha256: configuration.sha256().unwrap(),
        ..request
    };
    let prepared = prepare(&engine, &session, &request, BASE + 11_100).unwrap();
    assert_eq!(prepared.state, DeliveryState::Prepared);
    for number in 0..128 {
        let retired = engine
            .read_observation_delivery_at(
                &session.session_id,
                &session.token,
                &ReadObservationDelivery {
                    contract_version: 1,
                    delivery_id: id(&format!("expired-{number}")),
                },
                BASE + 11_200,
                "read-retired",
                "read-retired",
            )
            .unwrap();
        assert!(retired.delivery.is_none());
    }
    drop(engine);
    let engine = installed.reopen(BASE + 11_300);
    let session = installed.session(&engine, "retention-reopen", BASE + 11_301);
    assert_eq!(read(&engine, &session, BASE + 11_302), prepared);
    let extra = PrepareObservationDelivery {
        delivery_id: id("beyond-reduced-cap"),
        ..request
    };
    assert!(matches!(
        prepare(&engine, &session, &extra, BASE + 11_303),
        Err(ServiceError::ConfigurationLimit(_))
    ));
}

#[test]
fn observation_delivery_competing_preparations_cannot_overrun_the_shared_history_cap() {
    let (_installed, engine, session, request) = setup(1);
    let barrier = Barrier::new(8);
    let results = std::thread::scope(|threads| {
        let handles: Vec<_> = (0..8)
            .map(|number| {
                let request = PrepareObservationDelivery {
                    delivery_id: id(&format!("competing-{number}")),
                    ..request.clone()
                };
                let (engine, session, barrier) = (&engine, &session, &barrier);
                threads.spawn(move || {
                    barrier.wait();
                    prepare(engine, session, &request, BASE + 300)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    for result in results {
        match result {
            Ok(snapshot) => {
                let retained = engine
                    .read_observation_delivery_at(
                        &session.session_id,
                        &session.token,
                        &ReadObservationDelivery {
                            contract_version: 1,
                            delivery_id: snapshot.delivery_id.clone(),
                        },
                        BASE + 400,
                        "read-winner",
                        "read-winner",
                    )
                    .unwrap();
                assert_eq!(retained.delivery, Some(snapshot));
            }
            Err(ServiceError::ConfigurationLimit(_) | ServiceError::StorageConflict(_)) => {}
            other => panic!("unexpected competing preparation result: {other:?}"),
        }
    }
}

#[test]
fn observation_delivery_source_job_can_advance_during_export_without_rewriting_history() {
    use crate::engine::clock::{ClockObservation, ClockObservationFailure, ClockSource};
    use std::cell::Cell;
    struct MutableClock(Cell<u64>);
    impl ClockSource for MutableClock {
        fn observe(&self) -> std::result::Result<ClockObservation, ClockObservationFailure> {
            FixedClock::at(self.0.get()).observe()
        }
    }
    let (installed, engine, session, mut request) = setup(2);
    let mut configured = profile(&engine, 2);
    configured.bindings[0].limits.timeout_ms = 10_000;
    request.expected_configuration_sha256 =
        configure(&engine, &session, Some(configured), BASE + 220)
            .sha256()
            .unwrap();
    let prepared = prepare(&engine, &session, &request, BASE + 300).unwrap();
    let clock = MutableClock(Cell::new(BASE + 400));
    let accepted = engine
        .deliver_observation_with(
            &session.session_id,
            &session.token,
            &delivery(&prepared),
            "send",
            "send",
            &clock,
            |_, _, _| {
                let advanced = call(
                    &engine,
                    &session,
                    &installed.start(1000),
                    "advance-during-export",
                    BASE + 401,
                )
                .unwrap();
                assert!(advanced.status.job.revision > 1);
                clock.0.set(advanced.status.observed_at_unix_ms + 1);
                acknowledgement()
            },
        )
        .unwrap();
    assert_eq!(accepted.state, DeliveryState::Acknowledged);
    assert_eq!(accepted.source, prepared.source);
    assert_eq!(
        accepted.source_response_sha256,
        prepared.source_response_sha256
    );
    assert_eq!(read(&engine, &session, clock.0.get() + 1), accepted);
    let stale = PrepareObservationDelivery {
        delivery_id: id("stale-after-job-advanced"),
        ..request
    };
    assert!(matches!(
        prepare(&engine, &session, &stale, clock.0.get() + 2),
        Err(ServiceError::StorageConflict(_))
    ));
}

#[test]
fn observation_delivery_preparation_survives_time_and_job_changes_without_blocking_work() {
    let (installed, engine, session, request) = setup(1);
    let first = prepare(&engine, &session, &request, BASE + 300).unwrap();
    assert_eq!(first.state, DeliveryState::Prepared);
    assert_eq!(
        prepare(&engine, &session, &request, BASE + 400).unwrap(),
        first
    );
    let mut other = request.clone();
    other.delivery_id = id("delivery-two");
    assert!(matches!(
        prepare(&engine, &session, &other, BASE + 401),
        Err(ServiceError::ConfigurationLimit(_))
    ));
    let advanced = call(
        &engine,
        &session,
        &installed.start(1000),
        "advance-job",
        BASE + 402,
    )
    .unwrap();
    assert!(advanced.status.job.revision > 1);
    let after = advanced.status.observed_at_unix_ms + 1;
    assert_eq!(read(&engine, &session, after), first);
    let retained = engine
        .read_observation_delivery_at(
            &session.session_id,
            &session.token,
            &ReadObservationDelivery {
                contract_version: 1,
                delivery_id: first.delivery_id.clone(),
            },
            after,
            "historical-read",
            "historical-read",
        )
        .unwrap();
    retained.validate().unwrap();
    let source: rrd_contract::AttunementReadResult =
        serde_json::from_str(retained.source_response_json.as_deref().unwrap()).unwrap();
    assert_eq!(source.status.job.revision, 1);
    let mut altered = retained;
    altered.source_response_json.as_mut().unwrap().push(' ');
    assert!(
        altered.validate().is_err(),
        "exact source bytes must match the captured digest"
    );
    assert_eq!(
        prepare(&engine, &session, &request, after + 1).unwrap(),
        first
    );
    // A new capture must identify the new source revision, even with capacity available.
    configure(&engine, &session, Some(profile(&engine, 2)), after + 2);
    let current = engine.load_endpoint_configuration().unwrap().1;
    other.expected_configuration_sha256 = current.sha256().unwrap();
    assert!(matches!(
        prepare(&engine, &session, &other, after + 4),
        Err(ServiceError::StorageConflict(_))
    ));
    if let ObservationSource::AttunementJob {
        expected_revision, ..
    } = &mut other.source
    {
        *expected_revision = advanced.status.job.revision;
    }
    let second = prepare(&engine, &session, &other, after + 5).unwrap();
    assert_ne!(second.source_response_sha256, first.source_response_sha256);
}

#[test]
fn observation_delivery_lost_ack_reopens_and_retries_same_event_then_never_dispatches_again() {
    let (installed, engine, session, request) = setup(2);
    let first = prepare(&engine, &session, &request, BASE + 300).unwrap();
    let command = delivery(&first);
    let uncertain = engine
        .deliver_observation_with(
            &session.session_id,
            &session.token,
            &command,
            "send",
            "send",
            &FixedClock::at(BASE + 400),
            |_, _, _| TransportObservation {
                disposition: Disposition::TransportUnavailable,
                request_may_have_been_sent: true,
                http_status: None,
                reply_sha256: None,
            },
        )
        .unwrap();
    assert_eq!(uncertain.state, DeliveryState::Uncertain);
    assert!(engine
        .deliver_observation_with(
            &session.session_id,
            &session.token,
            &command,
            "early",
            "early",
            &FixedClock::at(BASE + 401),
            |_, _, _| panic!("cooldown must prevent network")
        )
        .is_err());
    drop(engine);
    let mut engine = installed.reopen(BASE + 500);
    install_secret(&mut engine);
    let session = installed.session(&engine, "reopened-export", BASE + 501);
    assert_eq!(read(&engine, &session, BASE + 502), uncertain);
    let ack = engine
        .deliver_observation_with(
            &session.session_id,
            &session.token,
            &command,
            "retry",
            "retry",
            &FixedClock::at(BASE + 503),
            |_, _, _| acknowledgement(),
        )
        .unwrap();
    assert_eq!(ack.state, DeliveryState::Acknowledged);
    assert_eq!(ack.attempts, 2);
    assert_eq!(ack.event_id, first.event_id);
    assert_eq!(ack.payload_sha256, first.payload_sha256);
    let replay = engine
        .deliver_observation_with(
            &session.session_id,
            &session.token,
            &command,
            "again",
            "again",
            &FixedClock::at(BASE + 504),
            |_, _, _| panic!("acknowledged replay must not dispatch"),
        )
        .unwrap();
    assert_eq!(replay, ack);
}

#[test]
fn observation_delivery_configuration_revocation_during_io_cannot_publish_acknowledgement() {
    let (_installed, engine, session, request) = setup(2);
    let first = prepare(&engine, &session, &request, BASE + 300).unwrap();
    let result = engine.deliver_observation_with(
        &session.session_id,
        &session.token,
        &delivery(&first),
        "send",
        "send",
        &FixedClock::at(BASE + 400),
        |_, _, _| {
            configure(&engine, &session, None, BASE + 401);
            acknowledgement()
        },
    );
    assert!(matches!(result, Err(ServiceError::StorageConflict(_))));
    assert_eq!(
        read(&engine, &session, BASE + 450).state,
        DeliveryState::InFlight
    );
    let uncertain = read(&engine, &session, BASE + 501);
    assert_eq!(uncertain.state, DeliveryState::Uncertain);
    assert_eq!(uncertain.acknowledgement_sha256, None);
    assert!(engine
        .deliver_observation_with(
            &session.session_id,
            &session.token,
            &delivery(&first),
            "retry",
            "retry",
            &FixedClock::at(BASE + 502),
            |_, _, _| panic!("revoked configuration cannot dispatch")
        )
        .is_err());
}

#[test]
fn observation_delivery_missing_secret_and_mismatched_digest_never_dispatch() {
    let (installed, engine, session, request) = setup(2);
    let first = prepare(&engine, &session, &request, BASE + 300).unwrap();
    drop(engine);
    let engine = installed.reopen(BASE + 400);
    let session = installed.session(&engine, "no-secret", BASE + 401);
    let mut command = delivery(&first);
    assert!(engine
        .deliver_observation_with(
            &session.session_id,
            &session.token,
            &command,
            "send",
            "send",
            &FixedClock::at(BASE + 402),
            |_, _, _| panic!("missing credential cannot dispatch")
        )
        .is_err());
    assert_eq!(read(&engine, &session, BASE + 403), first);
    command.expected_payload_sha256 = digest::sha256_hex(b"different payload");
    assert!(matches!(
        engine.deliver_observation_with(
            &session.session_id,
            &session.token,
            &command,
            "wrong",
            "wrong",
            &FixedClock::at(BASE + 404),
            |_, _, _| panic!("mismatched digest cannot dispatch")
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
}

#[test]
fn observation_delivery_uncertain_retirement_requires_explicit_policy_and_new_remote_identity() {
    let (_installed, engine, session, request) = setup(1);
    let first = prepare(&engine, &session, &request, BASE + 300).unwrap();
    engine
        .deliver_observation_with(
            &session.session_id,
            &session.token,
            &delivery(&first),
            "send",
            "send",
            &FixedClock::at(BASE + 400),
            |_, _, _| TransportObservation {
                disposition: Disposition::TransportUnavailable,
                request_may_have_been_sent: true,
                http_status: None,
                reply_sha256: None,
            },
        )
        .unwrap();
    let mut second = request.clone();
    second.delivery_id = id("second");
    assert!(matches!(
        prepare(&engine, &session, &second, BASE + 10_301),
        Err(ServiceError::ConfigurationLimit(_))
    ));
    assert_eq!(
        read(&engine, &session, BASE + 10_301).state,
        DeliveryState::Uncertain
    );
    let mut approved = profile(&engine, 2);
    approved
        .observation_policy
        .as_mut()
        .unwrap()
        .retire_expired_uncertain = true;
    let configuration = configure(&engine, &session, Some(approved), BASE + 10_302);
    second.expected_configuration_sha256 = configuration.sha256().unwrap();
    prepare(&engine, &session, &second, BASE + 10_304).unwrap();
    assert!(engine
        .read_observation_delivery_at(
            &session.session_id,
            &session.token,
            &ReadObservationDelivery {
                contract_version: 1,
                delivery_id: request.delivery_id.clone()
            },
            BASE + 10_305,
            "read",
            "read"
        )
        .unwrap()
        .delivery
        .is_none());
    let mut reused = request;
    reused.expected_configuration_sha256 = configuration.sha256().unwrap();
    let next = prepare(&engine, &session, &reused, BASE + 10_306).unwrap();
    assert_ne!(next.event_id, first.event_id);
    assert_eq!(next.state, DeliveryState::Prepared);
    assert_eq!(next.attempts, 0);
}

#[test]
fn observation_delivery_late_ack_cannot_publish_after_dispatch_lease() {
    use crate::engine::clock::{ClockObservation, ClockObservationFailure, ClockSource};
    use std::cell::Cell;
    struct AdvancingClock(Cell<u64>);
    impl ClockSource for AdvancingClock {
        fn observe(&self) -> std::result::Result<ClockObservation, ClockObservationFailure> {
            let time = self.0.get();
            self.0.set(time + 101);
            FixedClock::at(time).observe()
        }
    }
    let (_installed, engine, session, request) = setup(2);
    let first = prepare(&engine, &session, &request, BASE + 300).unwrap();
    let result = engine.deliver_observation_with(
        &session.session_id,
        &session.token,
        &delivery(&first),
        "send",
        "send",
        &AdvancingClock(Cell::new(BASE + 400)),
        |_, _, _| acknowledgement(),
    );
    assert!(matches!(result, Err(ServiceError::DeadlineExceeded)));
    let uncertain = read(&engine, &session, BASE + 502);
    assert_eq!(uncertain.state, DeliveryState::Uncertain);
    assert_eq!(uncertain.acknowledgement_sha256, None);
}

fn revoke(engine: &RrdEngine, action: SecurityAction, at: u64) {
    let repository =
        rrd_security::SecurityRepository::new(&engine.storage, engine.instance.clone());
    let mut state = repository.load().unwrap().unwrap();
    let revision = state.revision;
    state.revision += 1;
    for principal in state.principals.values_mut() {
        principal.grants.retain(|grant| grant.action != action);
    }
    for role in state.roles.values_mut() {
        role.grants.retain(|grant| grant.action != action);
    }
    repository
        .replace(
            revision,
            state,
            at,
            "observation-permission-test",
            "revoke",
            "revoke",
        )
        .unwrap();
}

#[test]
fn observation_delivery_current_source_and_export_permissions_are_required() {
    for denied in [
        SecurityAction::ObservationPrepare,
        SecurityAction::ObservationDeliver,
        SecurityAction::ObservationRead,
        SecurityAction::EngramRead,
    ] {
        let (_installed, engine, session, request) = setup(2);
        let first = prepare(&engine, &session, &request, BASE + 300).unwrap();
        revoke(&engine, denied, BASE + 350);
        match denied {
            SecurityAction::ObservationPrepare => {
                assert!(prepare(&engine, &session, &request, BASE + 400).is_err())
            }
            SecurityAction::ObservationRead => assert!(engine
                .read_observation_delivery_at(
                    &session.session_id,
                    &session.token,
                    &ReadObservationDelivery {
                        contract_version: 1,
                        delivery_id: first.delivery_id.clone()
                    },
                    BASE + 400,
                    "read",
                    "read"
                )
                .is_err()),
            _ => assert!(engine
                .deliver_observation_with(
                    &session.session_id,
                    &session.token,
                    &delivery(&first),
                    "send",
                    "send",
                    &FixedClock::at(BASE + 400),
                    |_, _, _| panic!("revoked source or export permission must prevent dispatch")
                )
                .is_err()),
        }
    }
}

#[test]
fn observation_delivery_live_fence_prevents_competing_dispatch_and_closed_session_cannot_ack() {
    let (installed, engine, session, request) = setup(2);
    let first = prepare(&engine, &session, &request, BASE + 300).unwrap();
    let command = delivery(&first);
    let result = engine.deliver_observation_with(
        &session.session_id,
        &session.token,
        &command,
        "send",
        "send",
        &FixedClock::at(BASE + 400),
        |_, _, _| {
            assert!(matches!(
                engine.deliver_observation_with(
                    &session.session_id,
                    &session.token,
                    &command,
                    "other",
                    "other",
                    &FixedClock::at(BASE + 401),
                    |_, _, _| panic!("active fence must prevent second dispatch")
                ),
                Err(ServiceError::StorageConflict(_))
            ));
            engine
                .close_session(
                    &session.session_id,
                    &session.token,
                    &CloseSession {},
                    &id("close-export-session"),
                    BASE + 402,
                    "close",
                    "close",
                )
                .unwrap();
            acknowledgement()
        },
    );
    assert!(result.is_err());
    let renewed = installed.session(&engine, "new-export-session", BASE + 501);
    let uncertain = read(&engine, &renewed, BASE + 502);
    assert_eq!(uncertain.state, DeliveryState::Uncertain);
    assert_eq!(uncertain.attempts, 1);
    assert_eq!(uncertain.acknowledgement_sha256, None);
}
