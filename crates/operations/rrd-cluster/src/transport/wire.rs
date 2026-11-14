//! Wire envelopes, requests and responses, peer identity checks and framing.

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireEnvelope {
    pub(super) version: u16,
    pub(super) cluster: ClusterId,
    pub(super) shard: ShardId,
    pub(super) source_raft_id: u64,
    pub(super) source_canonical_id: NodeId,
    pub(super) target_raft_id: u64,
    pub(super) target_canonical_id: NodeId,
    pub(super) request_digest: String,
    pub(super) request: WireRequest,
}

impl WireEnvelope {
    pub(super) fn new(
        source: &RrdTransportBinding,
        target_raft_id: u64,
        target: &RrdRaftNode,
        request: WireRequest,
    ) -> ClusterResult<Self> {
        source.validate()?;
        let target_canonical_id = NodeId::new(target.canonical_id.clone())?;
        let request_digest = wire_digest(&request)?;
        Ok(Self {
            version: RRFLOW_RAFT_TRANSPORT_VERSION,
            cluster: source.cluster.clone(),
            shard: source.shard,
            source_raft_id: source.raft_node_id,
            source_canonical_id: source.canonical_node_id.clone(),
            target_raft_id,
            target_canonical_id,
            request_digest,
            request,
        })
    }

    pub(super) fn validate(
        &self,
        local: &RrdTransportBinding,
        trust: &RrdTransportTrust,
        peer_spiffe_id: &str,
    ) -> io::Result<()> {
        if self.version != RRFLOW_RAFT_TRANSPORT_VERSION
            || self.cluster != local.cluster
            || self.shard != local.shard
            || self.target_raft_id != local.raft_node_id
            || self.target_canonical_id != local.canonical_node_id
            || self.request_digest != wire_digest(&self.request).map_err(invalid_data)?
        {
            return Err(invalid_data(
                "transport envelope binding or digest mismatch",
            ));
        }
        if self
            .request
            .source_raft_id()
            .is_some_and(|source| source != self.source_raft_id)
        {
            return Err(invalid_data(
                "authenticated envelope source does not match the Raft vote source",
            ));
        }
        let expected_peer = RrdTransportBinding {
            trust_domain: local.trust_domain.clone(),
            cluster: local.cluster.clone(),
            shard: local.shard,
            raft_node_id: self.source_raft_id,
            canonical_node_id: self.source_canonical_id.clone(),
        };
        if !trust.allows(self.source_raft_id, &self.source_canonical_id) {
            return Err(invalid_data(
                "transport source Raft id and canonical id are not authorized",
            ));
        }
        if expected_peer.spiffe_id().map_err(invalid_data)? != peer_spiffe_id {
            return Err(invalid_data(
                "authenticated peer identity does not match envelope source",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rpc", content = "body", rename_all = "snake_case")]
pub(super) enum WireRequest {
    Append(AppendEntriesRequest<RrdRaftTypeConfig>),
    Snapshot(InstallSnapshotRequest<RrdRaftTypeConfig>),
    Vote(VoteRequest<u64>),
    Artifact(ArtifactTransferRpc),
    RuntimeCommit(Box<RrdRaftCommand>),
}

impl WireRequest {
    pub(super) fn operation(&self) -> RrdTransportOperation {
        match self {
            Self::Append(_) => RrdTransportOperation::Append,
            Self::Snapshot(_) => RrdTransportOperation::Snapshot,
            Self::Vote(_) => RrdTransportOperation::Vote,
            Self::Artifact(_) => RrdTransportOperation::Artifact,
            Self::RuntimeCommit(_) => RrdTransportOperation::RuntimeCommit,
        }
    }

    fn source_raft_id(&self) -> Option<u64> {
        match self {
            Self::Append(request) => request.vote.leader_id().voted_for(),
            Self::Snapshot(request) => request.vote.leader_id().voted_for(),
            Self::Vote(request) => request.vote.leader_id().voted_for(),
            Self::Artifact(_) => None,
            Self::RuntimeCommit(_) => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "rpc", content = "body", rename_all = "snake_case")]
pub(super) enum WireResponse {
    Append(AppendResult),
    Snapshot(SnapshotResult),
    Vote(VoteResult),
    Artifact(std::result::Result<ArtifactTransferRpcResult, String>),
    RuntimeCommit(ConsensusCommitResult),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RrdTlsEndpoint {
    pub(super) address: String,
    pub(super) server_name: String,
}

impl RrdTlsEndpoint {
    pub(super) fn parse(value: &str) -> ClusterResult<Self> {
        let rest = value
            .strip_prefix("rrd+tls://")
            .ok_or_else(|| ClusterError::Invalid("Raft endpoint must use rrd+tls://".into()))?;
        let (address, server_name) = rest.split_once("?server_name=").ok_or_else(|| {
            ClusterError::Invalid("Raft TLS endpoint requires server_name".into())
        })?;
        if address.is_empty()
            || address.len() > 512
            || server_name.is_empty()
            || server_name.len() > 253
            || server_name.contains('&')
            || ServerName::try_from(server_name.to_owned()).is_err()
        {
            return Err(ClusterError::Invalid(
                "Raft TLS endpoint address or server_name is invalid".into(),
            ));
        }
        Ok(Self {
            address: address.to_owned(),
            server_name: server_name.to_owned(),
        })
    }
}

pub(super) fn peer_binding_for_node(
    local: &RrdTransportBinding,
    raft_node_id: u64,
    node: &RrdRaftNode,
) -> ClusterResult<RrdTransportBinding> {
    Ok(RrdTransportBinding {
        trust_domain: local.trust_domain.clone(),
        cluster: local.cluster.clone(),
        shard: local.shard,
        raft_node_id,
        canonical_node_id: NodeId::new(node.canonical_id.clone())?,
    })
}

pub fn verify_rrd_certificate_identity(
    certificates: Option<&[CertificateDer<'_>]>,
    expected: &RrdTransportBinding,
) -> ClusterResult<()> {
    let actual = certificate_spiffe_id(certificates)
        .map_err(|error| ClusterError::Denied(error.to_string()))?;
    if actual != expected.spiffe_id()? {
        return Err(ClusterError::Denied(
            "TLS peer SPIFFE identity does not match canonical target".into(),
        ));
    }
    Ok(())
}

pub(super) fn verify_peer_identity(
    certificates: Option<&[CertificateDer<'_>]>,
    expected: &RrdTransportBinding,
) -> ClusterResult<()> {
    verify_rrd_certificate_identity(certificates, expected)
}

pub(super) fn certificate_spiffe_id(
    certificates: Option<&[CertificateDer<'_>]>,
) -> io::Result<String> {
    let certificate = certificates
        .and_then(|certificates| certificates.first())
        .ok_or_else(|| invalid_data("mutual TLS peer supplied no leaf certificate"))?;
    let (remainder, certificate) = X509Certificate::from_der(certificate.as_ref())
        .map_err(|error| invalid_data(format!("peer certificate parse failed: {error}")))?;
    if !remainder.is_empty() {
        return Err(invalid_data(
            "peer certificate contains trailing non-certificate bytes",
        ));
    }
    let names = certificate
        .subject_alternative_name()
        .map_err(|error| invalid_data(format!("peer certificate SAN parse failed: {error}")))?
        .ok_or_else(|| invalid_data("peer certificate has no SAN extension"))?;
    let uri_names = names
        .value
        .general_names
        .iter()
        .filter_map(|name| match name {
            GeneralName::URI(uri) => Some((*uri).to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    match uri_names.as_slice() {
        [identity] if identity.starts_with("spiffe://") => Ok(identity.clone()),
        [] => Err(invalid_data("peer certificate has no URI SAN")),
        [_] => Err(invalid_data("peer certificate URI SAN is not a SPIFFE id")),
        _ => Err(invalid_data(
            "peer certificate has multiple ambiguous URI SANs",
        )),
    }
}

pub(super) fn wire_digest<T: Serialize>(value: &T) -> ClusterResult<String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| ClusterError::Invalid(format!("transport encoding failed: {error}")))
}

pub(super) fn result_outcome<T, E>(result: &std::result::Result<T, E>) -> RrdTransportOutcome {
    if result.is_ok() {
        RrdTransportOutcome::Allowed
    } else {
        RrdTransportOutcome::Failed
    }
}

pub(super) async fn write_frame<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    value: &T,
) -> io::Result<u64> {
    let bytes = serde_json::to_vec(value).map_err(invalid_data)?;
    if bytes.is_empty() || bytes.len() > RRFLOW_RAFT_MAX_RPC_FRAME_BYTES {
        return Err(invalid_data("transport frame exceeds its bounded size"));
    }
    writer
        .write_all(&(bytes.len() as u64).to_be_bytes())
        .await?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(bytes.len() as u64)
}

pub(super) async fn read_frame<R: AsyncRead + Unpin, T: for<'de> Deserialize<'de>>(
    reader: &mut R,
) -> io::Result<T> {
    read_frame_with_len(reader).await.map(|(value, _)| value)
}

pub(super) async fn read_frame_with_len<R: AsyncRead + Unpin, T: for<'de> Deserialize<'de>>(
    reader: &mut R,
) -> io::Result<(T, u64)> {
    let length = reader.read_u64().await?;
    if length == 0 || length > RRFLOW_RAFT_MAX_RPC_FRAME_BYTES as u64 {
        return Err(invalid_data("transport frame length is outside its bound"));
    }
    let mut bytes = vec![0; length as usize];
    reader.read_exact(&mut bytes).await?;
    serde_json::from_slice(&bytes)
        .map(|value| (value, length))
        .map_err(invalid_data)
}

#[allow(clippy::result_large_err)]
pub(super) fn remote_result<T, E>(
    target: u64,
    node: RrdRaftNode,
    result: std::result::Result<T, E>,
) -> std::result::Result<T, RPCError<u64, RrdRaftNode, E>>
where
    E: std::error::Error,
{
    result.map_err(|error| RPCError::RemoteError(RemoteError::new_with_node(target, node, error)))
}

pub(super) fn unreachable<E>(message: impl Into<String>) -> RPCError<u64, RrdRaftNode, E>
where
    E: std::error::Error,
{
    let error = io::Error::other(message.into());
    RPCError::Unreachable(Unreachable::new(&error))
}

pub(super) fn invalid_data(error: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}
