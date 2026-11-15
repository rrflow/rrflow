use super::*;

pub struct RrdHttpServer {
    listener: TcpListener,
    app: Router,
    state: Arc<AppState>,
    tls: Option<TlsAcceptor>,
}

/// In-memory verifier material for RRD-issued HS256 session credentials.
/// Only its SHA-256 is persisted in the security authority.
#[derive(Clone)]
pub struct RrdJwtVerificationKey {
    bytes: Arc<[u8]>,
}

impl RrdJwtVerificationKey {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() < 32 || bytes.len() > 64 * 1024 {
            return Err(HttpError::Contract(
                "JWT verification key must contain 32..=65536 bytes".into(),
            ));
        }
        Ok(Self {
            bytes: Arc::from(bytes),
        })
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A server identity whose verifier requires a trusted client certificate.
///
/// The inner Rustls configuration is private so a remote RRD listener cannot
/// accidentally be constructed with anonymous TLS.
#[derive(Clone)]
pub struct RrdMutualTlsServerConfig {
    inner: Arc<ServerConfig>,
}

impl RrdMutualTlsServerConfig {
    pub fn new(
        certificate_chain: Vec<CertificateDer<'static>>,
        private_key: PrivateKeyDer<'static>,
        client_roots: RootCertStore,
    ) -> Result<Self> {
        if certificate_chain.is_empty() || client_roots.is_empty() {
            return Err(HttpError::Tls(
                "RRD mTLS requires a certificate chain and client trust roots".into(),
            ));
        }
        let verifier = WebPkiClientVerifier::builder(Arc::new(client_roots))
            .build()
            .map_err(|error| HttpError::Tls(format!("RRD client verifier: {error}")))?;
        let mut inner = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_client_cert_verifier(verifier)
            .with_single_cert(certificate_chain, private_key)
            .map_err(|error| HttpError::Tls(format!("RRD server identity: {error}")))?;
        inner.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}

pub(super) struct AppState {
    pub(super) service: RrdEngine,
    pub(super) capabilities: ServiceCapabilities,
    pub(super) jwt_verification_key: Option<RrdJwtVerificationKey>,
}

impl RrdHttpServer {
    pub fn bind(engine: RrdEngine, bind: SocketAddr) -> Result<Self> {
        if !bind.ip().is_loopback() {
            return Err(HttpError::RemoteBindDenied(bind));
        }
        Self::bind_inner(engine, bind, None, None)
    }

    pub fn bind_with_jwt(
        engine: RrdEngine,
        bind: SocketAddr,
        jwt_verification_key: RrdJwtVerificationKey,
    ) -> Result<Self> {
        if !bind.ip().is_loopback() {
            return Err(HttpError::RemoteBindDenied(bind));
        }
        Self::bind_inner(engine, bind, None, Some(jwt_verification_key))
    }

    pub fn bind_mtls(
        engine: RrdEngine,
        bind: SocketAddr,
        tls: RrdMutualTlsServerConfig,
    ) -> Result<Self> {
        Self::bind_inner(engine, bind, Some(TlsAcceptor::from(tls.inner)), None)
    }

    pub fn bind_mtls_with_jwt(
        engine: RrdEngine,
        bind: SocketAddr,
        tls: RrdMutualTlsServerConfig,
        jwt_verification_key: RrdJwtVerificationKey,
    ) -> Result<Self> {
        Self::bind_inner(
            engine,
            bind,
            Some(TlsAcceptor::from(tls.inner)),
            Some(jwt_verification_key),
        )
    }

    fn bind_inner(
        engine: RrdEngine,
        bind: SocketAddr,
        tls: Option<TlsAcceptor>,
        jwt_verification_key: Option<RrdJwtVerificationKey>,
    ) -> Result<Self> {
        let instance = engine.instance_id().clone();
        let security_enforced = engine
            .security_enforced()
            .map_err(|error| HttpError::Contract(error.to_string()))?;
        if tls.is_some() && !security_enforced {
            return Err(HttpError::RemoteSecurityRequired);
        }
        if jwt_verification_key.is_some() && !security_enforced {
            return Err(HttpError::Contract(
                "RRD JWT verification requires an initialized security authority".into(),
            ));
        }
        let tls_enabled = tls.is_some();
        let jwt_enabled = jwt_verification_key.is_some();
        let listener =
            TcpListener::bind(bind).map_err(|error| HttpError::Bind(error.to_string()))?;
        listener.set_nonblocking(true)?;
        let endpoint_presentation = if listener
            .local_addr()
            .map_err(HttpError::Io)?
            .ip()
            .is_loopback()
        {
            EndpointPresentation::LoopbackHttpWebsocket
        } else {
            EndpointPresentation::NetworkHttpWebsocket
        };
        let deployment = match engine.installed_deployment_profile() {
            Some(installed)
                if installed.deployment_form == DeploymentForm::SingleNodeServer
                    && installed.storage_profile == engine.storage_profile_kind()
                    && installed.endpoint_presentation == endpoint_presentation =>
            {
                installed.clone()
            }
            Some(_) => {
                return Err(HttpError::Contract(
                    "installed deployment profile does not match server composition".into(),
                ));
            }
            None => DeploymentProfile {
                contract_version: rrd_contract::DEPLOYMENT_PROFILE_CONTRACT_VERSION,
                deployment_form: DeploymentForm::SingleNodeServer,
                storage_profile: engine.storage_profile_kind(),
                endpoint_presentation,
            },
        };
        let capabilities = capabilities(
            &engine,
            &instance,
            deployment,
            security_enforced,
            tls_enabled,
            jwt_enabled,
        );
        capabilities
            .validate()
            .map_err(|error| HttpError::Contract(error.to_string()))?;
        tracing::debug!(
            target: "rrflow::deployment",
            instance_id = %instance,
            deployment_form = ?capabilities.deployment.deployment_form,
            storage_profile = ?capabilities.deployment.storage_profile,
            endpoint_presentation = ?capabilities.deployment.endpoint_presentation,
            configuration_revision = capabilities.configuration.revision,
            configuration_sha256 = %capabilities.configuration.configuration_sha256,
            security_enforced,
            tls_enabled,
            jwt_enabled,
            "RRD server composition bound"
        );
        let state = Arc::new(AppState {
            service: engine,
            capabilities,
            jwt_verification_key,
        });
        let endpoint_catalogue = rrd_contract::endpoint_catalogue();
        let websocket_endpoint = endpoint_catalogue
            .websocket_endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == "websocket-connect")
            .ok_or_else(|| {
                HttpError::Contract(
                    "endpoint catalogue omitted the websocket-connect dispatch".into(),
                )
            })?;
        let app = Router::new()
            .route(&websocket_endpoint.path, get(websocket_upgrade))
            .fallback(any(dispatch))
            .with_state(Arc::clone(&state));
        Ok(Self {
            listener,
            app,
            state,
            tls,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("bound RRD listener has a local address")
    }

    pub fn is_tls(&self) -> bool {
        self.tls.is_some()
    }

    pub async fn serve(self) -> Result<()> {
        self.serve_until(std::future::pending()).await
    }

    pub async fn serve_until<F>(self, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let listener = tokio::net::TcpListener::from_std(self.listener)?;
        let state = self.state;
        let mut cold_sweep = cold_maintenance_interval(
            state
                .service
                .engram_configuration()
                .storage
                .cold_history
                .as_ref(),
        )?;
        let maintenance_state = Arc::clone(&state);
        let maintenance = super::maintenance::MaintenanceWorker::start(move |work| {
            let outcome = match work {
                super::maintenance::MaintenanceWork::Native => {
                    maintenance_state.service.maintain_native_storage()
                }
                super::maintenance::MaintenanceWork::ColdHistory => {
                    maintenance_state.service.maintain_cold_history()
                }
            };
            if outcome.is_err() {
                // Cold failures are recorded in engine status when possible;
                // native maintenance has separate counters. Never log source
                // payloads or turn a failed quantum into a success receipt.
                tracing::warn!(target: "rrd_server::maintenance", "bounded storage maintenance quantum failed");
            }
        })?;
        // Under a sealed follow policy the served engine keeps its project
        // attuned on a thread of its own, so a long Parse never delays the
        // storage maintenance above. Dropping the sender stops it.
        let (follow_stop, follow_stopped) = std::sync::mpsc::channel::<()>();
        let follower = state
            .service
            .engram_configuration()
            .attunement_follow
            .map(|policy| {
                let follow_state = Arc::clone(&state);
                std::thread::Builder::new()
                    .name("rrflow-attunement-follow".into())
                    .spawn(move || {
                        let interval = std::time::Duration::from_millis(policy.interval_ms);
                        let mut fingerprint: Option<String> = None;
                        loop {
                            match follow_state
                                .service
                                .follow_attunement(fingerprint.as_deref())
                            {
                                Ok(tick) => {
                                    if !matches!(tick.outcome, rrd_engine::FollowOutcome::Unchanged)
                                    {
                                        tracing::info!(
                                            target: "rrd_server::attunement_follow",
                                            outcome = ?tick.outcome,
                                            "followed the project's attunement"
                                        );
                                    }
                                    fingerprint = tick.fingerprint;
                                }
                                Err(error) => {
                                    // The next interval observes again from scratch.
                                    fingerprint = None;
                                    tracing::warn!(
                                        target: "rrd_server::attunement_follow",
                                        error = %error,
                                        "attunement follow pass failed"
                                    );
                                }
                            }
                            match follow_stopped.recv_timeout(interval) {
                                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                                _ => return,
                            }
                        }
                    })
            })
            .transpose()?;
        let _follow_guard = FollowGuard {
            stop: Some(follow_stop),
            thread: follower,
        };
        let serve = async move {
            if let Some(acceptor) = self.tls {
                return serve_mtls(listener, self.app, acceptor, shutdown).await;
            }
            axum::serve(listener, self.app)
                .with_graceful_shutdown(shutdown)
                .await
                .map_err(HttpError::Io)
        };
        tokio::pin!(serve);
        let mut sweep = tokio::time::interval(std::time::Duration::from_secs(1));
        sweep.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                result = &mut serve => return result,
                _ = sweep.tick() => {
                    state.service.reap_storage_cursors()
                        .map_err(|error| HttpError::Contract(error.to_string()))?;
                    maintenance.offer(super::maintenance::MaintenanceWork::Native)?;
                }
                _ = async {
                    match cold_sweep.as_mut() {
                        Some(interval) => { interval.tick().await; }
                        None => std::future::pending::<()>().await,
                    }
                } => {
                    maintenance.offer(super::maintenance::MaintenanceWork::ColdHistory)?;
                }
            }
        }
    }
}

/// Stops the attunement follower when serving ends, after its current pass.
struct FollowGuard {
    stop: Option<std::sync::mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for FollowGuard {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn cold_maintenance_interval(
    policy: Option<&rrd_contract::ColdHistoryPolicy>,
) -> Result<Option<tokio::time::Interval>> {
    let Some(policy) = policy else {
        return Ok(None);
    };
    policy
        .validate()
        .map_err(|error| HttpError::Contract(error.to_string()))?;
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(policy.interval_ms));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    Ok(Some(interval))
}

async fn serve_mtls<F>(
    listener: tokio::net::TcpListener,
    app: Router,
    acceptor: TlsAcceptor,
    shutdown: F,
) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let mut connections = JoinSet::new();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let acceptor = acceptor.clone();
                let app = app.clone();
                connections.spawn(async move {
                    let stream = acceptor.accept(stream).await.map_err(io::Error::other)?;
                    let service = TowerToHyperService::new(app);
                    let builder = http1::Builder::new();
                    builder
                        .serve_connection(TokioIo::new(stream), service)
                        .with_upgrades()
                        .await
                        .map_err(io::Error::other)
                });
            }
            completed = connections.join_next(), if !connections.is_empty() => {
                match completed {
                    Some(Ok(Err(error))) => {
                        tracing::debug!(error = %error, "RRD TLS connection closed");
                    }
                    Some(Err(error)) => {
                        tracing::warn!(error = %error, "RRD TLS connection task failed");
                    }
                    Some(Ok(Ok(()))) | None => {}
                }
            }
        }
    }
    if tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while connections.join_next().await.is_some() {}
    })
    .await
    .is_err()
    {
        connections.abort_all();
    }
    Ok(())
}

#[cfg(test)]
mod cold_schedule_tests {
    use super::*;
    use std::time::Duration;

    fn policy() -> rrd_contract::ColdHistoryPolicy {
        rrd_contract::ColdHistoryPolicy {
            interval_ms: 25,
            max_changes: 8,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            max_memory_bytes: 1_048_576,
            max_elapsed_ms: 1000,
            row_group_rows: 4,
            compression: rrd_contract::ColdHistoryCompression::Uncompressed,
            dictionary: false,
            statistics: false,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn configured_interval_skips_missed_ticks_and_disabled_policy_has_none() {
        assert!(cold_maintenance_interval(None).unwrap().is_none());
        let mut invalid = policy();
        invalid.interval_ms = 0;
        assert!(cold_maintenance_interval(Some(&invalid)).is_err());
        let mut interval = cold_maintenance_interval(Some(&policy())).unwrap().unwrap();
        interval.tick().await;
        let start = tokio::time::Instant::now();
        interval.tick().await;
        let elapsed = tokio::time::Instant::now() - start;
        assert!(elapsed >= Duration::from_millis(25));
        assert!(elapsed <= Duration::from_millis(30));
        tokio::time::sleep(Duration::from_millis(100)).await;
        interval.tick().await;
        let after_missed = tokio::time::Instant::now();
        interval.tick().await;
        let resumed = tokio::time::Instant::now() - after_missed;
        assert!(resumed > Duration::ZERO);
        assert!(resumed <= Duration::from_millis(30));
    }
}
