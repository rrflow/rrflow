//! Deterministic levels, graph validation and canonical encoding.

use super::*;

pub(super) fn deterministic_level(config: &HnswConfig, candidate: &VectorCandidate) -> u8 {
    let mut bytes = b"rrflow-hnsw-level-v1\0".to_vec();
    bytes.extend_from_slice(&config.seed.to_be_bytes());
    bytes.extend_from_slice(candidate.vector.reference.kind.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(candidate.vector.reference.id.as_str().as_bytes());
    bytes.extend_from_slice(&candidate.source_cursor.to_be_bytes());
    let hash = digest::sha256(&bytes);
    let mut random = u64::from_be_bytes(hash[..8].try_into().expect("eight-byte hash prefix"));
    let mut level = 0;
    while level < config.max_level && random % config.m as u64 == 0 {
        level += 1;
        random /= config.m as u64;
    }
    level
}

pub(super) fn validate_graph(
    config: &HnswConfig,
    source_cursor: u64,
    entrypoint: Option<usize>,
    nodes: &[HnswNode],
) -> Result<()> {
    config.validate()?;
    validate_candidate_versions(nodes.iter().map(|node| &node.candidate))?;
    if entrypoint.is_some_and(|entrypoint| entrypoint >= nodes.len())
        || (nodes.is_empty() != entrypoint.is_none())
    {
        return invalid("HNSW entrypoint is inconsistent with its nodes");
    }
    if let Some(entrypoint) = entrypoint {
        let maximum_level = nodes.iter().map(|node| node.level).max().unwrap_or(0);
        if nodes[entrypoint].level != maximum_level {
            return invalid("HNSW entrypoint does not own the maximum level");
        }
    }
    for (id, node) in nodes.iter().enumerate() {
        if node.candidate.scope != config.scope
            || node.candidate.source_cursor > source_cursor
            || node.candidate.vector.field != config.field
            || node.candidate.vector.value.dimensions() != config.dimensions
            || !node
                .candidate
                .matches_model(config.embedding_model.as_ref())
            || !matches!(node.candidate.vector.value, VectorValue::Dense { .. })
            || node.level > config.max_level
            || node.neighbors.len() != node.level as usize + 1
        {
            return invalid("HNSW node violates configuration or coverage");
        }
        for (layer, neighbors) in node.neighbors.iter().enumerate() {
            if neighbors.len() > layer_limit(config.m, layer) {
                return invalid("HNSW node exceeds configured degree");
            }
            let mut unique = BTreeSet::new();
            for neighbor in neighbors {
                if *neighbor >= nodes.len()
                    || *neighbor == id
                    || !unique.insert(*neighbor)
                    || nodes[*neighbor].level < layer as u8
                {
                    return invalid("HNSW neighbor reference is invalid");
                }
            }
        }
    }
    if let Some(entrypoint) = entrypoint {
        let mut reachable = vec![false; nodes.len()];
        let mut pending = vec![entrypoint];
        reachable[entrypoint] = true;
        while let Some(node) = pending.pop() {
            for neighbor in &nodes[node].neighbors[0] {
                if !reachable[*neighbor] {
                    reachable[*neighbor] = true;
                    pending.push(*neighbor);
                }
            }
        }
        if reachable.iter().any(|reachable| !reachable) {
            return invalid("HNSW layer zero is not reachable from the entrypoint");
        }
    }
    Ok(())
}

pub(super) fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| rrd_core::Error::InvalidRuntime {
        reason: format!("HNSW artifact cannot be encoded: {error}"),
    })
}
