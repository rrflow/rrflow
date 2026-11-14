use super::*;
use openraft::Vote;

#[test]
fn envelope_binds_tls_identity_route_digest_and_raft_vote_source() {
    let cluster = ClusterId::new("cluster-transport-unit").unwrap();
    let source = binding(&cluster, 1);
    let target = binding(&cluster, 2);
    let target_node = RrdRaftNode {
        canonical_id: "node-2".into(),
        zone: "az-2".into(),
        endpoint: "rrd+tls://127.0.0.1:9443?server_name=node-2.rrflow.test".into(),
    };
    let trust = RrdTransportTrust::new([(1, NodeId::new("node-1").unwrap())]).unwrap();
    let mut envelope = WireEnvelope::new(
        &source,
        2,
        &target_node,
        WireRequest::Vote(VoteRequest::new(Vote::new(3, 1), None)),
    )
    .unwrap();
    let peer = source.spiffe_id().unwrap();
    envelope.validate(&target, &trust, &peer).unwrap();

    envelope.request_digest = "0".repeat(64);
    assert!(envelope.validate(&target, &trust, &peer).is_err());
    envelope.request_digest = wire_digest(&envelope.request).unwrap();
    envelope.request = WireRequest::Vote(VoteRequest::new(Vote::new(4, 3), None));
    envelope.request_digest = wire_digest(&envelope.request).unwrap();
    assert!(envelope.validate(&target, &trust, &peer).is_err());
    assert!(envelope
        .validate(&target, &trust, &binding(&cluster, 3).spiffe_id().unwrap())
        .is_err());
}

#[test]
fn only_explicit_tls_endpoints_and_canonical_trust_domains_parse() {
    assert!(
        RrdTlsEndpoint::parse("rrd+tls://127.0.0.1:9443?server_name=node-1.rrflow.test").is_ok()
    );
    assert!(RrdTlsEndpoint::parse("http://127.0.0.1:9443").is_err());
    assert!(RrdTlsEndpoint::parse("rrd+tls://127.0.0.1:9443").is_err());
    let mut invalid = binding(&ClusterId::new("cluster-unit").unwrap(), 1);
    invalid.trust_domain = "Uppercase.invalid".into();
    assert!(invalid.validate().is_err());
    invalid.trust_domain = "empty..label".into();
    assert!(invalid.validate().is_err());
    assert!(RrdTransportTrust::new([
        (1, NodeId::new("same-node").unwrap()),
        (2, NodeId::new("same-node").unwrap()),
    ])
    .is_err());
}

#[test]
fn snapshot_chunks_do_not_repeat_an_already_completed_artifact_hydration() {
    assert!(artifact_hydration_required(true, None, "snapshot-1"));
    assert!(artifact_hydration_required(
        true,
        Some("snapshot-0"),
        "snapshot-1"
    ));
    assert!(!artifact_hydration_required(
        true,
        Some("snapshot-1"),
        "snapshot-1"
    ));
    assert!(!artifact_hydration_required(false, None, "snapshot-1"));
}

fn binding(cluster: &ClusterId, id: u64) -> RrdTransportBinding {
    RrdTransportBinding {
        trust_domain: "rrd.test".into(),
        cluster: cluster.clone(),
        shard: ShardId(7),
        raft_node_id: id,
        canonical_node_id: NodeId::new(format!("node-{id}")).unwrap(),
    }
}
