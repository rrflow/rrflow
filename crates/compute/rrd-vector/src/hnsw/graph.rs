//! Graph construction and repair: insertion, neighbour selection, pruning and layer search.

use super::*;

pub(super) fn insert_node(
    config: &HnswConfig,
    nodes: &mut Vec<HnswNode>,
    entrypoint: &mut Option<usize>,
    candidate: VectorCandidate,
) -> Result<()> {
    let level = deterministic_level(config, &candidate);
    let new_id = nodes.len();
    let Some(mut current) = *entrypoint else {
        nodes.push(HnswNode {
            candidate,
            level,
            neighbors: vec![Vec::new(); level as usize + 1],
        });
        *entrypoint = Some(0);
        return Ok(());
    };
    let query = dense_candidate(&candidate)?;
    let entry_level = nodes[current].level;
    for layer in ((level + 1)..=entry_level).rev() {
        current = greedy_layer(
            nodes,
            query,
            current,
            layer as usize,
            config.metric,
            HnswKernel::Scalar,
        )?;
    }
    let mut selected_by_layer = Vec::new();
    for layer in (0..=level.min(entry_level)).rev() {
        let found = search_layer(
            nodes,
            query,
            &[current],
            SearchLayerOptions {
                ef: config.ef_construction,
                layer: layer as usize,
                metric: config.metric,
                kernel: HnswKernel::Scalar,
                eligible: None,
            },
        )?;
        let selected = select_neighbors(nodes, found, config.m, config.metric)?;
        if let Some(best) = selected.first() {
            current = best.id;
        }
        selected_by_layer.push((layer as usize, selected));
    }
    let mut neighbors = vec![Vec::new(); level as usize + 1];
    for (layer, selected) in &selected_by_layer {
        neighbors[*layer] = selected.iter().map(|scored| scored.id).collect();
    }
    nodes.push(HnswNode {
        candidate,
        level,
        neighbors,
    });
    for (layer, selected) in selected_by_layer {
        for scored in selected {
            if !nodes[scored.id].neighbors[layer].contains(&new_id) {
                nodes[scored.id].neighbors[layer].push(new_id);
            }
            if nodes[scored.id].neighbors[layer].len() > layer_limit(config.m, layer) {
                prune_neighbors(
                    nodes,
                    scored.id,
                    layer,
                    layer_limit(config.m, layer),
                    config.metric,
                )?;
            }
        }
    }
    if level > entry_level {
        *entrypoint = Some(new_id);
    }
    Ok(())
}

pub(super) fn prune_neighbors(
    nodes: &mut [HnswNode],
    node: usize,
    layer: usize,
    m: usize,
    metric: ScoreMetric,
) -> Result<()> {
    let query = dense_candidate(&nodes[node].candidate)?.to_vec();
    let mut neighbors = nodes[node].neighbors[layer].clone();
    let scored = neighbors
        .drain(..)
        .map(|id| {
            Ok(ScoredNode {
                id,
                score: score_dense_query(&query, &nodes[id].candidate, metric, HnswKernel::Scalar)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    nodes[node].neighbors[layer] = select_neighbors(nodes, scored, m, metric)?
        .into_iter()
        .map(|value| value.id)
        .collect();
    Ok(())
}

/// Chooses a node's neighbors from `candidates`, each scored against that node, by the heuristic
/// of Malkov and Yashunin's Algorithm 4. A candidate is kept only if it is closer to the node than
/// to any neighbor already kept, since otherwise that neighbor already leads to it. Keeping only
/// the closest would fill a node's list from its own cluster and cut the links between clusters.
pub(super) fn select_neighbors(
    nodes: &[HnswNode],
    mut candidates: Vec<ScoredNode>,
    limit: usize,
    metric: ScoreMetric,
) -> Result<Vec<ScoredNode>> {
    sort_scored(&mut candidates);
    if candidates.len() <= limit {
        return Ok(candidates);
    }
    let mut selected: Vec<ScoredNode> = Vec::with_capacity(limit);
    for candidate in candidates {
        if selected.len() == limit {
            break;
        }
        let values = dense_candidate(&nodes[candidate.id].candidate)?;
        let mut closer_to_the_node = true;
        for kept in &selected {
            let to_kept = score_dense_query(
                values,
                &nodes[kept.id].candidate,
                metric,
                HnswKernel::Scalar,
            )?;
            if to_kept > candidate.score {
                closer_to_the_node = false;
                break;
            }
        }
        if closer_to_the_node {
            selected.push(candidate);
        }
    }
    Ok(selected)
}

/// Marks everything reachable on layer zero from `start`, on top of what is already marked.
pub(super) fn mark_reachable(nodes: &[HnswNode], reachable: &mut [bool], start: usize) {
    reachable[start] = true;
    let mut pending = vec![start];
    while let Some(node) = pending.pop() {
        for neighbor in &nodes[node].neighbors[0] {
            if !reachable[*neighbor] {
                reachable[*neighbor] = true;
                pending.push(*neighbor);
            }
        }
    }
}

/// The heuristic makes a connected graph likely and does not promise one, and a node no search can
/// reach is a wrong answer waiting to happen. So anything left unreachable on layer zero is linked
/// from its closest reachable node with room for it, in a fixed order, so a build stays
/// deterministic. Pruning never runs here, so no link that already reaches a node is lost.
pub(super) fn repair_reachability(
    config: &HnswConfig,
    nodes: &mut [HnswNode],
    entrypoint: Option<usize>,
) -> Result<()> {
    let Some(entrypoint) = entrypoint else {
        return Ok(());
    };
    let limit = layer_limit(config.m, 0);
    let mut reachable = vec![false; nodes.len()];
    mark_reachable(nodes, &mut reachable, entrypoint);
    for orphan in 0..nodes.len() {
        if reachable[orphan] {
            continue;
        }
        let query = dense_candidate(&nodes[orphan].candidate)?.to_vec();
        let mut found = search_layer(
            nodes,
            &query,
            &[entrypoint],
            SearchLayerOptions {
                ef: config.ef_construction,
                layer: 0,
                metric: config.metric,
                kernel: HnswKernel::Scalar,
                eligible: None,
            },
        )?;
        sort_scored(&mut found);
        let nearby = found
            .iter()
            .find(|scored| nodes[scored.id].neighbors[0].len() < limit)
            .map(|scored| scored.id);
        let host = match nearby {
            Some(host) => Some(host),
            // Everything the search reached is full, so look at every reachable node.
            None => {
                let mut best: Option<ScoredNode> = None;
                for (id, node) in nodes.iter().enumerate() {
                    if reachable[id] && node.neighbors[0].len() < limit {
                        let score = score_dense_query(
                            &query,
                            &node.candidate,
                            config.metric,
                            HnswKernel::Scalar,
                        )?;
                        let scored = ScoredNode { id, score };
                        if best.is_none_or(|current| scored > current) {
                            best = Some(scored);
                        }
                    }
                }
                best.map(|scored| scored.id)
            }
        };
        // With no room anywhere, the validator refuses the graph, as it would have before.
        let Some(host) = host else {
            return Ok(());
        };
        nodes[host].neighbors[0].push(orphan);
        // Pruning is one-way, so the orphan may already point at its host.
        if nodes[orphan].neighbors[0].len() < limit && !nodes[orphan].neighbors[0].contains(&host) {
            nodes[orphan].neighbors[0].push(host);
        }
        mark_reachable(nodes, &mut reachable, orphan);
    }
    Ok(())
}

pub(super) fn layer_limit(m: usize, layer: usize) -> usize {
    if layer == 0 {
        m.saturating_mul(2)
    } else {
        m
    }
}

pub(super) fn greedy_layer(
    nodes: &[HnswNode],
    query: &[f32],
    start: usize,
    layer: usize,
    metric: ScoreMetric,
    kernel: HnswKernel,
) -> Result<usize> {
    let mut current = start;
    let mut current_score = score_dense_query(query, &nodes[current].candidate, metric, kernel)?;
    loop {
        let mut improved = false;
        if let Some(neighbors) = nodes[current].neighbors.get(layer) {
            for neighbor in neighbors {
                let score = score_dense_query(query, &nodes[*neighbor].candidate, metric, kernel)?;
                if score > current_score || (score == current_score && *neighbor < current) {
                    current = *neighbor;
                    current_score = score;
                    improved = true;
                }
            }
        }
        if !improved {
            return Ok(current);
        }
    }
}

pub(super) struct SearchLayerOptions<'a> {
    pub(super) ef: usize,
    pub(super) layer: usize,
    pub(super) metric: ScoreMetric,
    pub(super) kernel: HnswKernel,
    pub(super) eligible: Option<&'a dyn Fn(usize) -> bool>,
}

pub(super) fn search_layer(
    nodes: &[HnswNode],
    query: &[f32],
    entries: &[usize],
    options: SearchLayerOptions<'_>,
) -> Result<Vec<ScoredNode>> {
    let mut visited = BTreeSet::new();
    let mut frontier = BinaryHeap::new();
    let mut best = BinaryHeap::<Reverse<ScoredNode>>::new();
    for entry in entries {
        if *entry >= nodes.len() || !visited.insert(*entry) {
            continue;
        }
        let scored = ScoredNode {
            id: *entry,
            score: score_dense_query(
                query,
                &nodes[*entry].candidate,
                options.metric,
                options.kernel,
            )?,
        };
        frontier.push(scored);
        if options.eligible.is_none_or(|eligible| eligible(*entry)) {
            best.push(Reverse(scored));
        }
    }
    while let Some(current) = frontier.pop() {
        if best.len() >= options.ef
            && current.score
                < best
                    .peek()
                    .map(|value| value.0.score)
                    .unwrap_or(f64::NEG_INFINITY)
        {
            break;
        }
        if let Some(neighbors) = nodes[current.id].neighbors.get(options.layer) {
            for neighbor in neighbors {
                if !visited.insert(*neighbor) {
                    continue;
                }
                let scored = ScoredNode {
                    id: *neighbor,
                    score: score_dense_query(
                        query,
                        &nodes[*neighbor].candidate,
                        options.metric,
                        options.kernel,
                    )?,
                };
                if best.len() < options.ef
                    || scored.score
                        > best
                            .peek()
                            .map(|value| value.0.score)
                            .unwrap_or(f64::NEG_INFINITY)
                {
                    frontier.push(scored);
                    if options.eligible.is_none_or(|eligible| eligible(*neighbor)) {
                        best.push(Reverse(scored));
                        if best.len() > options.ef {
                            best.pop();
                        }
                    }
                }
            }
        }
    }
    let mut best = best.into_iter().map(|value| value.0).collect::<Vec<_>>();
    sort_scored(&mut best);
    Ok(best)
}

pub(super) fn sort_scored(values: &mut [ScoredNode]) {
    values.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.id.cmp(&right.id))
    });
}
