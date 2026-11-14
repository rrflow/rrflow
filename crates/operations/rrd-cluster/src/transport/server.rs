//! The Raft TLS server.

use super::*;

#[derive(Clone)]
pub struct RrdRaftTlsServer {
    binding: RrdTransportBinding,
    trust: RrdTransportTrust,
    raft: RrdRaft,
    acceptor: TlsAcceptor,
    admission: Arc<Semaphore>,
    gate: RrdTransportGate,
    reloader: Option<RrdTlsReloader>,
    artifacts: Option<ArtifactTransferReceiver>,
    project_scope: Option<ScopeId>,
    telemetry: RrdTransportTelemetry,
}

impl RrdRaftTlsServer {
    pub fn new(
        binding: RrdTransportBinding,
        trust: RrdTransportTrust,
        raft: RrdRaft,
        server: Arc<ServerConfig>,
    ) -> ClusterResult<Self> {
        Self::new_with_gate(binding, trust, raft, server, RrdTransportGate::enabled())
    }

    pub fn new_with_gate(
        binding: RrdTransportBinding,
        trust: RrdTransportTrust,
        raft: RrdRaft,
        server: Arc<ServerConfig>,
        gate: RrdTransportGate,
    ) -> ClusterResult<Self> {
        binding.validate()?;
        let telemetry =
            RrdTransportTelemetry::new(RrdTransportAdmissionPolicy::default(), now_millis())?;
        Ok(Self {
            binding,
            trust,
            raft,
            acceptor: TlsAcceptor::from(server),
            admission: Arc::new(Semaphore::new(RRFLOW_RAFT_MAX_IN_FLIGHT_RPCS)),
            gate,
            reloader: None,
            artifacts: None,
            project_scope: None,
            telemetry,
        })
    }

    pub fn new_reloadable(
        binding: RrdTransportBinding,
        trust: RrdTransportTrust,
        raft: RrdRaft,
        credentials: RrdTlsReloader,
        gate: RrdTransportGate,
    ) -> ClusterResult<Self> {
        if binding != credentials.binding {
            return Err(ClusterError::Denied(
                "TLS credential binding differs from the Raft server binding".into(),
            ));
        }
        let server = credentials.server_config()?;
        binding.validate()?;
        let telemetry =
            RrdTransportTelemetry::new(RrdTransportAdmissionPolicy::default(), now_millis())?;
        Ok(Self {
            binding,
            trust,
            raft,
            acceptor: TlsAcceptor::from(server),
            admission: Arc::new(Semaphore::new(RRFLOW_RAFT_MAX_IN_FLIGHT_RPCS)),
            gate,
            reloader: Some(credentials),
            artifacts: None,
            project_scope: None,
            telemetry,
        })
    }

    pub fn new_reloadable_with_artifacts(
        binding: RrdTransportBinding,
        trust: RrdTransportTrust,
        raft: RrdRaft,
        credentials: RrdTlsReloader,
        gate: RrdTransportGate,
        artifacts: ArtifactTransferReceiver,
        project_scope: ScopeId,
    ) -> ClusterResult<Self> {
        let mut server = Self::new_reloadable(binding, trust, raft, credentials, gate)?;
        server.artifacts = Some(artifacts);
        server.project_scope = Some(project_scope);
        Ok(server)
    }

    pub fn with_admission_policy(
        mut self,
        policy: RrdTransportAdmissionPolicy,
    ) -> ClusterResult<Self> {
        policy.validate()?;
        self.admission = Arc::new(Semaphore::new(policy.max_global_in_flight));
        self.telemetry = RrdTransportTelemetry::new(policy, now_millis())?;
        Ok(self)
    }

    pub fn telemetry_snapshot(
        &self,
        observed_at: u64,
    ) -> ClusterResult<RrdTransportTelemetrySnapshot> {
        self.telemetry.snapshot(observed_at)
    }

    pub async fn serve(self, listener: TcpListener) -> io::Result<()> {
        loop {
            let permit = Arc::clone(&self.admission)
                .acquire_owned()
                .await
                .map_err(|_| io::Error::other("Raft transport admission closed"))?;
            let (stream, _) = listener.accept().await?;
            let server = self.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let _ = tokio::time::timeout(
                    RRFLOW_RAFT_SERVER_RPC_TIMEOUT,
                    server.serve_connection(stream),
                )
                .await;
            });
        }
    }

    pub async fn serve_connection(&self, stream: TcpStream) -> io::Result<()> {
        if !self.gate.is_enabled() {
            let _ = self.telemetry.reject_connection(0);
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "local Raft transport is disabled",
            ));
        }
        let acceptor = self
            .reloader
            .as_ref()
            .map(RrdTlsReloader::server_config)
            .transpose()
            .map_err(|error| io::Error::other(error.to_string()))?
            .map(TlsAcceptor::from)
            .unwrap_or_else(|| self.acceptor.clone());
        let mut stream = match acceptor.accept(stream).await {
            Ok(stream) => stream,
            Err(error) => {
                let _ = self.telemetry.reject_connection(0);
                return Err(error);
            }
        };
        let peer = match certificate_spiffe_id(stream.get_ref().1.peer_certificates()) {
            Ok(peer) => peer,
            Err(error) => {
                let _ = self.telemetry.reject_connection(0);
                return Err(error);
            }
        };
        let (envelope, request_bytes): (WireEnvelope, u64) =
            match read_frame_with_len(&mut stream).await {
                Ok(envelope) => envelope,
                Err(error) => {
                    let _ = self.telemetry.reject_connection(0);
                    return Err(error);
                }
            };
        if let Err(error) = envelope.validate(&self.binding, &self.trust, &peer) {
            let _ = self.telemetry.reject_connection(request_bytes);
            return Err(error);
        }
        self.telemetry
            .accept_connection(request_bytes)
            .map_err(invalid_data)?;
        let operation = envelope.request.operation();
        let admission = self
            .telemetry
            .admit(
                &envelope.source_canonical_id,
                operation,
                request_bytes,
                now_millis(),
            )
            .map_err(invalid_data)?;
        let source = envelope.source_canonical_id.clone();
        let (response, outcome) = match envelope.request {
            WireRequest::Append(request) => {
                let response = self.raft.append_entries(request).await;
                let outcome = result_outcome(&response);
                (WireResponse::Append(response), outcome)
            }
            WireRequest::Snapshot(request) => {
                let response = self.raft.install_snapshot(request).await;
                let outcome = result_outcome(&response);
                (WireResponse::Snapshot(response), outcome)
            }
            WireRequest::Vote(request) => {
                let response = self.raft.vote(request).await;
                let outcome = result_outcome(&response);
                (WireResponse::Vote(response), outcome)
            }
            WireRequest::Artifact(request) => {
                let (response, outcome) = match &self.artifacts {
                    Some(receiver) => {
                        match receiver.handle(&source, &self.binding.canonical_node_id, request) {
                            Ok(response) => (Ok(response), RrdTransportOutcome::Allowed),
                            Err(error @ (ClusterError::Invalid(_) | ClusterError::Denied(_))) => {
                                (Err(error.to_string()), RrdTransportOutcome::Denied)
                            }
                            Err(error) => (Err(error.to_string()), RrdTransportOutcome::Failed),
                        }
                    }
                    None => (
                        Err("artifact transport is not configured on this node".into()),
                        RrdTransportOutcome::Failed,
                    ),
                };
                (WireResponse::Artifact(response), outcome)
            }
            WireRequest::RuntimeCommit(command) => {
                let result = (|| {
                    command.validate().map_err(|error| error.to_string())?;
                    let Some(scope) = &self.project_scope else {
                        return Err("consensus runtime commit transport is not configured".into());
                    };
                    let crate::RrdRaftOperation::RuntimeCommit { commit } = &command.operation
                    else {
                        return Err("consensus route accepts only runtime commits".into());
                    };
                    if &commit.scope != scope {
                        return Err(
                            "consensus runtime commit scope differs from the configured project"
                                .into(),
                        );
                    }
                    Ok(())
                })();
                let response = match result {
                    Ok(()) => self
                        .raft
                        .client_write(*command)
                        .await
                        .map_err(ConsensusCommitWireError::Raft),
                    Err(error) => Err(ConsensusCommitWireError::Denied(error)),
                };
                let outcome = match &response {
                    Ok(response) if response.data.accepted => RrdTransportOutcome::Allowed,
                    Ok(_) => RrdTransportOutcome::Denied,
                    Err(ConsensusCommitWireError::Denied(_)) => RrdTransportOutcome::Denied,
                    Err(ConsensusCommitWireError::Raft(_)) => RrdTransportOutcome::Failed,
                };
                (WireResponse::RuntimeCommit(response), outcome)
            }
        };
        let response_bytes = write_frame(&mut stream, &response).await?;
        admission
            .finish(outcome, response_bytes)
            .map_err(invalid_data)
    }
}
