//! Transport limits, result types, errors, the transport gate, peer bindings and TLS configuration and reload.

use super::*;

pub const RRFLOW_RAFT_TRANSPORT_VERSION: u16 = 2;
pub const RRFLOW_RAFT_MAX_RPC_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub const RRFLOW_RAFT_MAX_IN_FLIGHT_RPCS: usize = 256;
pub const RRFLOW_RAFT_SERVER_RPC_TIMEOUT: Duration = Duration::from_secs(30);
pub const RRFLOW_ARTIFACT_RPC_TIMEOUT: Duration = Duration::from_secs(30);
pub const RRFLOW_CONSENSUS_COMMIT_RPC_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

pub(super) type RrdRaft = Raft<RrdRaftTypeConfig>;
pub(super) type AppendResult = std::result::Result<AppendEntriesResponse<u64>, RaftError<u64>>;
pub(super) type SnapshotResult =
    std::result::Result<InstallSnapshotResponse<u64>, RaftError<u64, InstallSnapshotError>>;
pub(super) type VoteResult = std::result::Result<VoteResponse<u64>, RaftError<u64>>;
pub(super) type ConsensusRaftError = RaftError<u64, ClientWriteError<u64, RrdRaftNode>>;
pub(super) type ConsensusCommitResult =
    std::result::Result<ClientWriteResponse<RrdRaftTypeConfig>, ConsensusCommitWireError>;
pub(super) type BoxedRpcError<E> = Box<RPCError<u64, RrdRaftNode, E>>;
pub(super) type WireRpcResult<E> = std::result::Result<WireResponse, BoxedRpcError<E>>;

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum ConsensusCommitWireError {
    Denied(String),
    Raft(ConsensusRaftError),
}

#[derive(Debug)]
pub enum RrdConsensusCommitError {
    ForwardToLeader,
    Rejected(String),
    Unavailable(String),
}

impl fmt::Display for RrdConsensusCommitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForwardToLeader => formatter.write_str("consensus leader changed while routing"),
            Self::Rejected(error) => write!(formatter, "consensus commit rejected: {error}"),
            Self::Unavailable(error) => write!(formatter, "consensus commit unavailable: {error}"),
        }
    }
}

impl std::error::Error for RrdConsensusCommitError {}

#[derive(Debug, Clone)]
pub struct RrdTransportGate {
    enabled: Arc<AtomicBool>,
}

impl RrdTransportGate {
    pub fn enabled() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RrdTransportBinding {
    pub trust_domain: String,
    pub cluster: ClusterId,
    pub shard: ShardId,
    pub raft_node_id: u64,
    pub canonical_node_id: NodeId,
}

impl RrdTransportBinding {
    pub fn validate(&self) -> ClusterResult<()> {
        let labels = self.trust_domain.split('.').collect::<Vec<_>>();
        if self.trust_domain.is_empty()
            || self.trust_domain.len() > 253
            || labels.iter().any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || label.bytes().any(|byte| {
                        !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && byte != b'-'
                    })
            })
        {
            return Err(ClusterError::Invalid(
                "transport trust domain must be a bounded lowercase DNS name".into(),
            ));
        }
        Ok(())
    }

    pub fn spiffe_id(&self) -> ClusterResult<String> {
        self.validate()?;
        Ok(format!(
            "spiffe://{}/rrd/{}/{}",
            self.trust_domain,
            sha256_hex(self.cluster.as_str().as_bytes()),
            sha256_hex(self.canonical_node_id.as_str().as_bytes())
        ))
    }
}

pub struct RrdTlsMaterial {
    pub certificate_chain: Vec<CertificateDer<'static>>,
    pub private_key: PrivateKeyDer<'static>,
    pub trust_roots: RootCertStore,
    pub revocation_lists: Vec<CertificateRevocationListDer<'static>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RrdTransportTrust {
    peers: BTreeMap<u64, NodeId>,
}

impl RrdTransportTrust {
    pub fn new(peers: impl IntoIterator<Item = (u64, NodeId)>) -> ClusterResult<Self> {
        let mut trusted = BTreeMap::new();
        let mut canonical_ids = BTreeSet::new();
        for (raft_id, canonical_id) in peers {
            if !canonical_ids.insert(canonical_id.clone()) {
                return Err(ClusterError::Invalid(
                    "transport trust contains a duplicate canonical node id".into(),
                ));
            }
            if trusted.insert(raft_id, canonical_id).is_some() {
                return Err(ClusterError::Invalid(
                    "transport trust contains a duplicate Raft node id".into(),
                ));
            }
        }
        if trusted.is_empty() {
            return Err(ClusterError::Invalid(
                "transport trust must authorize at least one peer".into(),
            ));
        }
        Ok(Self { peers: trusted })
    }

    pub(super) fn allows(&self, raft_id: u64, canonical_id: &NodeId) -> bool {
        self.peers.get(&raft_id) == Some(canonical_id)
    }
}

pub fn build_rrd_tls_configs(
    material: RrdTlsMaterial,
) -> ClusterResult<(Arc<ClientConfig>, Arc<ServerConfig>)> {
    if material.certificate_chain.is_empty() || material.trust_roots.is_empty() {
        return Err(ClusterError::Invalid(
            "TLS identity requires a certificate chain and at least one trust root".into(),
        ));
    }
    let client_verifier = if material.revocation_lists.is_empty() {
        WebPkiClientVerifier::builder(Arc::new(material.trust_roots.clone())).build()
    } else {
        WebPkiClientVerifier::builder(Arc::new(material.trust_roots.clone()))
            .with_crls(material.revocation_lists.clone())
            .only_check_end_entity_revocation()
            .enforce_revocation_expiration()
            .build()
    }
    .map_err(|error| ClusterError::Invalid(format!("client verifier: {error}")))?;
    let server = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(
            material.certificate_chain.clone(),
            material.private_key.clone_key(),
        )
        .map_err(|error| ClusterError::Invalid(format!("server TLS identity: {error}")))?;
    let server_verifier = if material.revocation_lists.is_empty() {
        WebPkiServerVerifier::builder(Arc::new(material.trust_roots)).build()
    } else {
        WebPkiServerVerifier::builder(Arc::new(material.trust_roots))
            .with_crls(material.revocation_lists)
            .only_check_end_entity_revocation()
            .enforce_revocation_expiration()
            .build()
    }
    .map_err(|error| ClusterError::Invalid(format!("server verifier: {error}")))?;
    let client = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(server_verifier)
        .with_client_auth_cert(material.certificate_chain, material.private_key)
        .map_err(|error| ClusterError::Invalid(format!("client TLS identity: {error}")))?;
    Ok((Arc::new(client), Arc::new(server)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrdTlsGeneration {
    pub generation: u64,
    pub leaf_digest: String,
}

#[derive(Debug)]
pub(super) struct RrdTlsState {
    identity: RrdTlsGeneration,
    client: Arc<ClientConfig>,
    server: Arc<ServerConfig>,
}

#[derive(Debug, Clone)]
pub struct RrdTlsReloader {
    pub(super) binding: RrdTransportBinding,
    pub(super) state: Arc<RwLock<RrdTlsState>>,
}

impl RrdTlsReloader {
    pub fn new(
        binding: RrdTransportBinding,
        generation: u64,
        material: RrdTlsMaterial,
    ) -> ClusterResult<Self> {
        if generation == 0 {
            return Err(ClusterError::Invalid(
                "TLS credential generation must begin above zero".into(),
            ));
        }
        let state = build_tls_state(&binding, generation, material)?;
        Ok(Self {
            binding,
            state: Arc::new(RwLock::new(state)),
        })
    }

    pub fn rotate(
        &self,
        expected_generation: u64,
        material: RrdTlsMaterial,
    ) -> ClusterResult<RrdTlsGeneration> {
        let next_generation = expected_generation
            .checked_add(1)
            .ok_or_else(|| ClusterError::Invalid("TLS credential generation overflow".into()))?;
        let next = build_tls_state(&self.binding, next_generation, material)?;
        let identity = next.identity.clone();
        let mut state = self
            .state
            .write()
            .map_err(|_| ClusterError::Unavailable("TLS credential state is poisoned".into()))?;
        if state.identity.generation != expected_generation {
            return Err(ClusterError::Denied(format!(
                "TLS credential generation expected {expected_generation} but was {}",
                state.identity.generation
            )));
        }
        *state = next;
        Ok(identity)
    }

    pub fn identity(&self) -> ClusterResult<RrdTlsGeneration> {
        self.state
            .read()
            .map(|state| state.identity.clone())
            .map_err(|_| ClusterError::Unavailable("TLS credential state is poisoned".into()))
    }

    pub(super) fn client_config(&self) -> ClusterResult<Arc<ClientConfig>> {
        self.state
            .read()
            .map(|state| Arc::clone(&state.client))
            .map_err(|_| ClusterError::Unavailable("TLS credential state is poisoned".into()))
    }

    pub(super) fn server_config(&self) -> ClusterResult<Arc<ServerConfig>> {
        self.state
            .read()
            .map(|state| Arc::clone(&state.server))
            .map_err(|_| ClusterError::Unavailable("TLS credential state is poisoned".into()))
    }
}

pub(super) fn build_tls_state(
    binding: &RrdTransportBinding,
    generation: u64,
    material: RrdTlsMaterial,
) -> ClusterResult<RrdTlsState> {
    verify_rrd_certificate_identity(Some(&material.certificate_chain), binding)?;
    let leaf = material
        .certificate_chain
        .first()
        .ok_or_else(|| ClusterError::Invalid("TLS identity has no leaf certificate".into()))?;
    let identity = RrdTlsGeneration {
        generation,
        leaf_digest: sha256_hex(leaf.as_ref()),
    };
    let (client, server) = build_rrd_tls_configs(material)?;
    Ok(RrdTlsState {
        identity,
        client,
        server,
    })
}
