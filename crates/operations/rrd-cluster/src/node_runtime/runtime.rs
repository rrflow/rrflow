//! Running the node: control lines, command execution, status and TLS material.

use super::*;

pub async fn run_rrflow_runtime(config: RrdNodeConfig) -> ClusterResult<()> {
    config.validate()?;
    let local = config.nodes[&config.raft_node_id].clone();
    let binding = RrdTransportBinding {
        trust_domain: config.trust_domain.clone(),
        cluster: config.cluster.clone(),
        shard: config.shard,
        raft_node_id: config.raft_node_id,
        canonical_node_id: NodeId::new(local.canonical_id)?,
    };
    let trust = RrdTransportTrust::new(config.nodes.iter().map(|(id, node)| {
        (
            *id,
            NodeId::new(node.canonical_id.clone()).expect("validated node identity"),
        )
    }))?;
    let material = load_tls_material(&RrdTlsFiles {
        certificate_der: config.certificate_der.clone(),
        private_key_der: config.private_key_der.clone(),
        trust_root_ders: vec![config.trust_root_der.clone()],
        revocation_list_ders: Vec::new(),
    })?;
    let credentials = RrdTlsReloader::new(binding.clone(), 1, material)?;
    let transport_gate = RrdTransportGate::enabled();
    let (log, state_machine) = RrdRaftStore::open(&config.data_root, config.shard)?;
    let storage_lifecycle = state_machine.storage_lifecycle();
    let application_objects = state_machine.application_objects();
    let artifact_receiver = ArtifactTransferReceiver::open(application_objects.clone())?;
    let artifact_status = artifact_receiver.clone();
    let trace_telemetry = Arc::new(ConsensusTraceTelemetry::new(node_now_millis()));
    let raft_slot = Arc::new(OnceLock::new());
    let network = RrdRaftNetworkFactory::new_reloadable_with_artifacts(
        binding.clone(),
        credentials.clone(),
        transport_gate.clone(),
        state_machine.clone(),
        application_objects,
        config.project_scope.clone(),
    )?;
    let trace_observer = Arc::new(ConsensusArtifactTransferObserver {
        raft: Arc::clone(&raft_slot),
        state_machine: state_machine.clone(),
        network: network.clone(),
        nodes: config.nodes.clone(),
        local_node_id: config.raft_node_id,
        project_scope: config.project_scope.clone(),
        actor: format!("cluster:artifact-transfer:{}", config.raft_node_id),
        telemetry: Arc::clone(&trace_telemetry),
    });
    let network = network.with_artifact_observer(trace_observer);
    let raft_config = Arc::new(
        Config {
            heartbeat_interval: config.raft_timing.heartbeat_interval_millis,
            election_timeout_min: config.raft_timing.election_timeout_min_millis,
            election_timeout_max: config.raft_timing.election_timeout_max_millis,
            snapshot_policy: SnapshotPolicy::Never,
            max_in_snapshot_log_to_keep: 0,
            purge_batch_size: 1,
            ..Config::default()
        }
        .validate()
        .map_err(|error| ClusterError::Invalid(error.to_string()))?,
    );
    let raft = Raft::new(
        config.raft_node_id,
        raft_config,
        network,
        log,
        state_machine.clone(),
    )
    .await
    .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
    raft_slot.set(raft.clone()).map_err(|_| {
        ClusterError::Unavailable("consensus trace writer was attached more than once".into())
    })?;
    let listener = TcpListener::bind(&config.raft_listen)
        .await
        .map_err(|error| ClusterError::Unavailable(format!("bind Raft transport: {error}")))?;
    let server = RrdRaftTlsServer::new_reloadable_with_artifacts(
        binding,
        trust,
        raft.clone(),
        credentials.clone(),
        transport_gate.clone(),
        artifact_receiver,
        config.project_scope.clone(),
    )?
    .with_admission_policy(config.transport_admission.clone())?;
    let telemetry_sources = RrdNodeTelemetrySources {
        transport: server.clone(),
        artifacts: artifact_status,
        traces: trace_telemetry,
    };
    let server_task = tokio::spawn(async move { server.serve(listener).await });

    write_reply(&RrdNodeReply::success(
        None,
        RrdNodeResult::Ready {
            raft_node_id: config.raft_node_id,
        },
    ))
    .await?;

    let mut input = BufReader::new(tokio::io::stdin());
    while let Some(line) = read_control_line(&mut input)
        .await
        .map_err(|error| ClusterError::Unavailable(format!("read node control: {error}")))?
    {
        let reply = match line {
            Err(()) => RrdNodeReply::failure(None, "control command exceeds 1048576 bytes"),
            Ok(line) if line.is_empty() => {
                RrdNodeReply::failure(None, "control command must not be empty")
            }
            Ok(line) => match serde_json::from_slice::<RrdNodeRequest>(&line) {
                Ok(request) if request.validate().is_err() => RrdNodeReply::failure(
                    Some(request.request_id),
                    "control request version or identity is invalid",
                ),
                Ok(RrdNodeRequest {
                    request_id,
                    command: RrdNodeCommand::Shutdown,
                    ..
                }) => {
                    write_reply(&RrdNodeReply::success(Some(request_id), RrdNodeResult::Ack))
                        .await?;
                    break;
                }
                Ok(RrdNodeRequest {
                    request_id,
                    command,
                    ..
                }) => match execute_command(
                    &raft,
                    &state_machine,
                    &config,
                    &transport_gate,
                    &credentials,
                    &telemetry_sources,
                    command,
                )
                .await
                {
                    Ok(value) => RrdNodeReply::success(Some(request_id), value),
                    Err(error) => RrdNodeReply::failure(Some(request_id), error),
                },
                Err(error) => {
                    RrdNodeReply::failure(None, format!("decode control request: {error}"))
                }
            },
        };
        write_reply(&reply).await?;
    }

    raft.shutdown()
        .await
        .map_err(|error| ClusterError::Unavailable(format!("shutdown Raft: {error}")))?;
    server_task.abort();
    match server_task.await {
        Err(error) if error.is_cancelled() => {}
        Err(error) => {
            return Err(ClusterError::Unavailable(format!(
                "join stopped Raft transport: {error}"
            )))
        }
        Ok(Err(error)) => {
            return Err(ClusterError::Unavailable(format!(
                "Raft transport stopped with an error: {error}"
            )))
        }
        Ok(Ok(())) => {}
    }
    drop(telemetry_sources);
    drop(state_machine);
    drop(raft);
    drop(raft_slot);
    tokio::time::timeout(STORAGE_RELEASE_TIMEOUT, async {
        while !storage_lifecycle.is_released() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| {
        ClusterError::Unavailable(format!(
            "Raft storage writer locks were not released within {} ms",
            STORAGE_RELEASE_TIMEOUT.as_millis()
        ))
    })?;
    Ok(())
}

pub(super) async fn read_control_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> std::io::Result<Option<Result<Vec<u8>, ()>>> {
    let mut line = Vec::new();
    let mut oversized = false;
    loop {
        let (consumed, newline, eof) = {
            let available = reader.fill_buf().await?;
            if available.is_empty() {
                (0, false, true)
            } else {
                let newline = available.iter().position(|byte| *byte == b'\n');
                let consumed = newline.map_or(available.len(), |index| index + 1);
                let content = if newline.is_some() {
                    &available[..consumed - 1]
                } else {
                    &available[..consumed]
                };
                if !oversized {
                    if line.len().saturating_add(content.len()) > RRFLOW_NODE_MAX_CONTROL_LINE_BYTES
                    {
                        oversized = true;
                        line.clear();
                    } else {
                        line.extend_from_slice(content);
                    }
                }
                (consumed, newline.is_some(), false)
            }
        };
        if eof {
            return if line.is_empty() && !oversized {
                Ok(None)
            } else if oversized {
                Ok(Some(Err(())))
            } else {
                Ok(Some(Ok(line)))
            };
        }
        reader.consume(consumed);
        if newline {
            return if oversized {
                Ok(Some(Err(())))
            } else {
                Ok(Some(Ok(line)))
            };
        }
    }
}

pub(super) async fn execute_command(
    raft: &RrdRaft,
    state_machine: &crate::RrdRaftStateMachine,
    config: &RrdNodeConfig,
    transport_gate: &RrdTransportGate,
    credentials: &RrdTlsReloader,
    telemetry: &RrdNodeTelemetrySources,
    command: RrdNodeCommand,
) -> Result<RrdNodeResult, String> {
    match command {
        RrdNodeCommand::Status => Ok(RrdNodeResult::Status {
            status: Box::new(node_status(
                raft,
                state_machine,
                credentials,
                config,
                telemetry,
            )?),
        }),
        RrdNodeCommand::Initialize => raft
            .initialize(BTreeMap::from([(
                config.raft_node_id,
                config.nodes[&config.raft_node_id].clone(),
            )]))
            .await
            .map(|_| RrdNodeResult::Ack)
            .map_err(|error| error.to_string()),
        RrdNodeCommand::Elect => raft
            .trigger()
            .elect()
            .await
            .map(|_| RrdNodeResult::Ack)
            .map_err(|error| error.to_string()),
        RrdNodeCommand::AddLearner { node_id } => {
            let node = config
                .nodes
                .get(&node_id)
                .cloned()
                .ok_or_else(|| format!("node inventory has no node {node_id}"))?;
            let response = raft
                .add_learner(node_id, node, false)
                .await
                .map_err(|error| error.to_string())?;
            let membership_log_id = response.log_id;
            let leader_id = config.raft_node_id;
            let metrics = raft
                .wait(Some(LEARNER_CATCH_UP_TIMEOUT))
                .metrics(
                    |metrics| {
                        let lost_leadership = metrics.current_leader != Some(leader_id);
                        let learner_matched = metrics
                            .replication
                            .as_ref()
                            .and_then(|replication| replication.get(&node_id))
                            .and_then(Option::as_ref)
                            .is_some_and(|matched| matched >= &membership_log_id);
                        lost_leadership || learner_matched
                    },
                    "learner must match its committed membership log",
                )
                .await
                .map_err(|error| error.to_string())?;
            if metrics.current_leader != Some(leader_id) {
                return Err("leadership changed before learner catch-up was proven".into());
            }
            let matched = metrics
                .replication
                .as_ref()
                .and_then(|replication| replication.get(&node_id))
                .and_then(Option::as_ref);
            if matched.is_none_or(|matched| matched < &membership_log_id) {
                return Err("learner catch-up was not proven through its membership log".into());
            }
            Ok(RrdNodeResult::Ack)
        }
        RrdNodeCommand::ChangeMembership { voters } => raft
            .change_membership(voters, false)
            .await
            .map(|_| RrdNodeResult::Ack)
            .map_err(|error| error.to_string()),
        RrdNodeCommand::PlacementTransition {
            request_id,
            placement,
            expected_commit_index,
        } => {
            write_command(
                raft,
                RrdRaftCommand::placement_transition(request_id, placement, expected_commit_index)
                    .map_err(|error| error.to_string())?,
            )
            .await
        }
        RrdNodeCommand::Probe {
            request_id,
            placement_epoch,
            expected_commit_index,
            payload,
        } => {
            write_command(
                raft,
                RrdRaftCommand::new(
                    request_id,
                    config.shard,
                    placement_epoch,
                    expected_commit_index,
                    payload,
                )
                .map_err(|error| error.to_string())?,
            )
            .await
        }
        RrdNodeCommand::RuntimeCommit {
            request_id,
            placement_epoch,
            expected_commit_index,
            commit,
        } => {
            if commit.scope != config.project_scope {
                return Err("runtime commit scope differs from the configured project".into());
            }
            write_command(
                raft,
                RrdRaftCommand::runtime_commit(
                    request_id,
                    config.shard,
                    placement_epoch,
                    expected_commit_index,
                    commit,
                )
                .map_err(|error| error.to_string())?,
            )
            .await
        }
        RrdNodeCommand::TriggerSnapshot => raft
            .trigger()
            .snapshot()
            .await
            .map(|_| RrdNodeResult::Ack)
            .map_err(|error| error.to_string()),
        RrdNodeCommand::PurgeLog { index } => raft
            .trigger()
            .purge_log(index)
            .await
            .map(|_| RrdNodeResult::Ack)
            .map_err(|error| error.to_string()),
        RrdNodeCommand::WaitApplied {
            index,
            timeout_millis,
        } => {
            if timeout_millis == 0 || timeout_millis > 60_000 {
                return Err("wait timeout must be within 1..=60000 milliseconds".into());
            }
            raft.wait(Some(Duration::from_millis(timeout_millis)))
                .ge(
                    Metric::AppliedIndex(Some(index)),
                    "process control wait-at-least",
                )
                .await
                .map(|_| RrdNodeResult::Status {
                    status: Box::new(
                        node_status(raft, state_machine, credentials, config, telemetry)
                            .expect("validated TLS credential state remains readable"),
                    ),
                })
                .map_err(|error| error.to_string())
        }
        RrdNodeCommand::SetTransportEnabled { enabled } => {
            transport_gate.set_enabled(enabled);
            Ok(RrdNodeResult::Ack)
        }
        RrdNodeCommand::RotateCredentials {
            expected_generation,
            files,
        } => credentials
            .rotate(
                expected_generation,
                load_tls_material(&files).map_err(|error| error.to_string())?,
            )
            .map(|credentials| RrdNodeResult::Credentials { credentials })
            .map_err(|error| error.to_string()),
        RrdNodeCommand::Shutdown => unreachable!("shutdown is handled by the control loop"),
    }
}

pub(super) async fn write_command(
    raft: &RrdRaft,
    command: RrdRaftCommand,
) -> Result<RrdNodeResult, String> {
    let response = raft
        .client_write(command)
        .await
        .map_err(|error| error.to_string())?;
    Ok(RrdNodeResult::Write {
        log_index: response.log_id.index,
        response: response.data,
    })
}

pub(super) fn node_status(
    raft: &RrdRaft,
    state_machine: &crate::RrdRaftStateMachine,
    credentials: &RrdTlsReloader,
    config: &RrdNodeConfig,
    telemetry: &RrdNodeTelemetrySources,
) -> Result<RrdNodeStatus, String> {
    let metrics = raft.metrics().borrow().clone();
    let snapshot_index = state_machine
        .persisted_snapshot_meta()
        .map_err(|error| error.to_string())?
        .and_then(|meta| meta.last_log_id.map(|log| log.index));
    let observed_at = node_now_millis();
    Ok(RrdNodeStatus {
        project_scope: config.project_scope.clone(),
        cluster: config.cluster.clone(),
        shard: config.shard,
        raft_node_id: metrics.id,
        canonical_node_id: NodeId::new(config.nodes[&config.raft_node_id].canonical_id.clone())
            .map_err(|error| error.to_string())?,
        current_term: metrics.current_term,
        current_leader: metrics.current_leader,
        last_log_index: metrics.last_log_index,
        last_applied_index: metrics.last_applied.map(|log| log.index),
        snapshot_index,
        purged_index: metrics.purged.map(|log| log.index),
        state: format!("{:?}", metrics.state).to_ascii_lowercase(),
        credentials: credentials.identity().map_err(|error| error.to_string())?,
        telemetry: RrdNodeTelemetrySnapshot {
            observed_at,
            transport_ingress: telemetry
                .transport
                .telemetry_snapshot(observed_at)
                .map_err(|error| error.to_string())?,
            artifacts: telemetry
                .artifacts
                .telemetry_snapshot(observed_at)
                .map_err(|error| error.to_string())?,
            consensus_traces: telemetry
                .traces
                .snapshot(observed_at)
                .map_err(|error| error.to_string())?,
        },
    })
}

pub(super) fn node_now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub(super) fn telemetry_increment(counter: &AtomicU64, overflowed: &AtomicBool) {
    if counter
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .is_err()
    {
        overflowed.store(true, Ordering::Relaxed);
    }
}

pub(super) fn load_tls_material(files: &RrdTlsFiles) -> ClusterResult<RrdTlsMaterial> {
    if files.trust_root_ders.is_empty()
        || files.trust_root_ders.len() > 32
        || files.revocation_list_ders.len() > 32
    {
        return Err(ClusterError::Invalid(
            "TLS file set requires 1..=32 roots and at most 32 CRLs".into(),
        ));
    }
    let certificate = read_bounded_der(&files.certificate_der, "certificate")?;
    let private_key = read_bounded_der(&files.private_key_der, "private key")?;
    let mut roots = RootCertStore::empty();
    for path in &files.trust_root_ders {
        roots
            .add(CertificateDer::from(read_bounded_der(path, "trust root")?))
            .map_err(|error| ClusterError::Invalid(format!("trust root DER: {error}")))?;
    }
    let revocation_lists = files
        .revocation_list_ders
        .iter()
        .map(|path| {
            read_bounded_der(path, "revocation list").map(CertificateRevocationListDer::from)
        })
        .collect::<ClusterResult<Vec<_>>>()?;
    Ok(RrdTlsMaterial {
        certificate_chain: vec![CertificateDer::from(certificate)],
        private_key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(private_key)),
        trust_roots: roots,
        revocation_lists,
    })
}

pub(super) fn read_bounded_der(path: &Path, label: &str) -> ClusterResult<Vec<u8>> {
    let metadata = fs::metadata(path)
        .map_err(|error| ClusterError::Unavailable(format!("{label} metadata: {error}")))?;
    if metadata.len() == 0 || metadata.len() > RRFLOW_NODE_MAX_CONFIG_BYTES {
        return Err(ClusterError::Invalid(format!(
            "{label} DER must contain 1..=1048576 bytes"
        )));
    }
    fs::read(path).map_err(|error| ClusterError::Unavailable(format!("read {label}: {error}")))
}

pub(super) async fn write_reply(reply: &RrdNodeReply) -> ClusterResult<()> {
    let mut bytes = serde_json::to_vec(reply)
        .map_err(|error| ClusterError::Unavailable(format!("encode node reply: {error}")))?;
    bytes.push(b'\n');
    let mut stdout = tokio::io::stdout();
    stdout
        .write_all(&bytes)
        .await
        .map_err(|error| ClusterError::Unavailable(format!("write node reply: {error}")))?;
    stdout
        .flush()
        .await
        .map_err(|error| ClusterError::Unavailable(format!("flush node reply: {error}")))
}
