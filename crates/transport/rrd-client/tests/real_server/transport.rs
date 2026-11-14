//! The shared deployment corpus over loopback and mutual TLS for remote transport.

use super::*;

#[tokio::test]
pub(super) async fn loopback_rust_client_passes_the_shared_deployment_corpus() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("local-conformance");
    let storage = RrflowKvStore::open(&root).unwrap();
    let instance = CanonicalId::new("local-conformance").unwrap();
    let principal = Principal {
        id: CanonicalId::new("local-client").unwrap(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"local-conformance-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: [
            Action::SessionCreate,
            Action::TransactionBegin,
            Action::TransactionCommit,
            Action::QueryExecute,
        ]
        .into_iter()
        .map(|action| ResourceGrant {
            action,
            resource_prefix: ResourcePath {
                segments: vec![ResourceId::new(ResourceKind::Instance, instance.as_str()).unwrap()],
            },
            data_policy: None,
        })
        .collect(),
    };
    SecurityRepository::new(&storage, instance.clone())
        .initialize(
            SecurityState {
                format_version: SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([(principal.id.clone(), principal)]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "bootstrap",
            "request-local-conformance-bootstrap",
            "operation-local-conformance-bootstrap",
        )
        .unwrap();
    drop(storage);

    let engine = RrdEngine::open(&root, instance.clone(), COMPONENT_TOKEN_KEY).unwrap();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let address = server.local_addr();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));

    let client = RrdClient::connect_local(address, instance, ClientConfig::default()).unwrap();
    let session = client
        .create_session(
            CanonicalId::new("local-client").unwrap(),
            "local-conformance-key",
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 300_000,
                    max_open_transactions: 1,
                },
            },
            RequestOptions::mutation(
                "request-local-conformance-session",
                "operation-local-conformance-session",
                "local-conformance-session-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    commit_client_deployment_corpus(&client, &session, "local-conformance").await;
    assert_client_deployment_corpus(
        &client,
        &session,
        EndpointPresentation::LoopbackHttpWebsocket,
    )
    .await;

    shutdown.send(()).unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
pub(super) async fn remote_transport_requires_mutual_tls_and_exact_server_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("mtls-instance");
    let storage = RrflowKvStore::open(&root).unwrap();
    let instance = CanonicalId::new("mtls-sdk-test").unwrap();
    let principal = Principal {
        id: CanonicalId::new("mtls-client").unwrap(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"mtls-api-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: [
            Action::SessionCreate,
            Action::TransactionBegin,
            Action::TransactionCommit,
            Action::QueryExecute,
            Action::ChangefeedFollow,
            Action::WebSocketConnect,
            Action::SubscriptionOpen,
            Action::SubscriptionConnect,
            Action::SubscriptionAck,
            Action::SubscriptionClose,
        ]
        .into_iter()
        .map(|action| ResourceGrant {
            action,
            resource_prefix: ResourcePath {
                segments: vec![ResourceId::new(ResourceKind::Instance, instance.as_str()).unwrap()],
            },
            data_policy: None,
        })
        .collect(),
    };
    SecurityRepository::new(&storage, instance.clone())
        .initialize(
            SecurityState {
                format_version: SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([(principal.id.clone(), principal)]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "bootstrap",
            "request-bootstrap-mtls",
            "operation-bootstrap-mtls",
        )
        .unwrap();
    drop(storage);

    let (ca, issuer) = test_ca();
    let (server_chain, server_key) = test_identity(
        &issuer,
        vec!["localhost".into()],
        ExtendedKeyUsagePurpose::ServerAuth,
    );
    let (client_chain, client_key) =
        test_identity(&issuer, Vec::new(), ExtendedKeyUsagePurpose::ClientAuth);
    let server_tls = RrdMutualTlsServerConfig::new(server_chain, server_key, roots(&ca)).unwrap();
    let engine = RrdEngine::open(&root, instance.clone(), COMPONENT_TOKEN_KEY).unwrap();
    let server =
        RrdHttpServer::bind_mtls(engine, "127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
    let endpoint = format!("https://localhost:{}", server.local_addr().port());
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));

    let anonymous_tls = RustlsClientConfig::builder()
        .with_root_certificates(roots(&ca))
        .with_no_client_auth();
    let anonymous = RrdClient::connect_mtls(
        endpoint.clone(),
        instance.clone(),
        anonymous_tls,
        ClientConfig::default(),
    )
    .unwrap();
    assert!(matches!(
        anonymous.capabilities().await,
        Err(Error::Transport(_))
    ));

    let (wrong_name_chain, wrong_name_key) =
        test_identity(&issuer, Vec::new(), ExtendedKeyUsagePurpose::ClientAuth);
    let wrong_name_tls = RustlsClientConfig::builder()
        .with_root_certificates(roots(&ca))
        .with_client_auth_cert(wrong_name_chain, wrong_name_key)
        .unwrap();
    let wrong_identity = RrdClient::connect_mtls(
        format!("https://127.0.0.1:{}", server_address_port(&endpoint)),
        instance.clone(),
        wrong_name_tls,
        ClientConfig::default(),
    )
    .unwrap();
    assert!(matches!(
        wrong_identity.capabilities().await,
        Err(Error::Transport(_))
    ));

    let client_tls = RustlsClientConfig::builder()
        .with_root_certificates(roots(&ca))
        .with_client_auth_cert(client_chain, client_key)
        .unwrap();
    let client =
        RrdClient::connect_mtls(endpoint, instance, client_tls, ClientConfig::default()).unwrap();
    let capabilities = client.capabilities().await.unwrap();
    assert_eq!(capabilities.protocol_version, 1);
    assert_eq!(
        capabilities.deployment.deployment_form,
        DeploymentForm::SingleNodeServer
    );
    assert_eq!(
        capabilities.deployment.storage_profile,
        StorageProfileKind::RrflowKv
    );
    assert_eq!(
        capabilities.deployment.endpoint_presentation,
        EndpointPresentation::LoopbackHttpWebsocket
    );
    assert_eq!(
        capabilities
            .capabilities
            .iter()
            .find(|capability| capability.name.as_str() == "remote-listen")
            .unwrap()
            .status,
        rrd_contract::CapabilityStatus::Experimental
    );
    let session = client
        .create_session(
            CanonicalId::new("mtls-client").unwrap(),
            "mtls-api-key",
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 300_000,
                    max_open_transactions: 1,
                },
            },
            RequestOptions::mutation(
                "request-mtls-session",
                "operation-mtls-session",
                "mtls-session-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    commit_client_deployment_corpus(&client, &session, "remote-conformance").await;
    assert_client_deployment_corpus(
        &client,
        &session,
        EndpointPresentation::LoopbackHttpWebsocket,
    )
    .await;
    let subscription_id = rrd_contract::CorrelationId::new("mtls-subscription").unwrap();
    let opened = client
        .open_subscription(
            &session,
            OpenSubscription {
                subscription_id: subscription_id.clone(),
                stream: SubscriptionStream::Changefeed {
                    scope: "instance:mtls-sdk-test".into(),
                },
                after_cursor: 0,
                batch_size: 3,
                max_in_flight: 1,
                retention_cursor_window: 128,
                lease_ms: 60_000,
                heartbeat_interval_ms: 100,
            },
            RequestOptions::mutation(
                "request-mtls-subscription",
                "operation-mtls-subscription",
                "mtls-subscription-key",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let mut socket = client.connect_websocket(&session).await.unwrap();
    socket
        .subscribe(SubscriptionResume::from_snapshot(&opened.subscription))
        .await
        .unwrap();
    match receive_application_frame(&mut socket).await.payload {
        WebSocketPayload::Subscribed(result) => {
            assert_eq!(result.subscription.connection_generation, 1);
        }
        payload => panic!("unexpected mTLS connect payload: {payload:?}"),
    }
    let pushed = receive_application_frame(&mut socket).await;
    assert!(matches!(
        pushed.payload,
        WebSocketPayload::Delivery(ref delivery)
            if matches!(
                &delivery.delivery,
                SubscriptionDelivery::Changefeed { page }
                    if page.requested_after_cursor == 0 && page.through_cursor == 3
            )
    ));
    socket.close().await.unwrap();

    drop(client);
    drop(anonymous);
    shutdown.send(()).unwrap();
    task.await.unwrap().unwrap();
}

pub(super) fn server_address_port(endpoint: &str) -> u16 {
    endpoint
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse().ok())
        .expect("test TLS endpoint carries a port")
}
