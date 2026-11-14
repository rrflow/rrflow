//! Fusion, reranking and maximal-marginal-relevance selection.

use super::*;

impl RetrievalExecutionContext<'_> {
    pub(super) fn fusion(
        &mut self,
        prefetch: &[RetrievalPrefetch],
        fusion: &RetrievalFusion,
        limit: u64,
    ) -> Result<QueryCandidates> {
        let RetrievalFusion::ReciprocalRank {
            rank_constant,
            weights_millionths,
        } = fusion;
        let mut branches = Vec::with_capacity(prefetch.len());
        for branch in prefetch {
            branches.push(self.execute(&branch.query, branch.limit)?);
        }
        let exact = branches.iter().all(|branch| branch.exact);
        let input = branches.iter().map(|branch| branch.values.len()).sum();
        let mut merged = BTreeMap::<DataReference, RankedCandidate>::new();
        for (branch, weight) in branches.into_iter().zip(weights_millionths) {
            for (offset, candidate) in branch.values.into_iter().enumerate() {
                let rank = u64::try_from(offset + 1)
                    .map_err(|_| ServiceError::Vector("retrieval branch rank overflowed".into()))?;
                let contribution =
                    f64::from(*weight) / 1_000_000.0 / (f64::from(*rank_constant) + rank as f64);
                let entry =
                    merged
                        .entry(candidate.subject.clone())
                        .or_insert_with(|| RankedCandidate {
                            subject: candidate.subject.clone(),
                            vector_references: BTreeSet::new(),
                            score: 0.0,
                            contributions: Vec::new(),
                            properties: RuntimeProperties::new(),
                        });
                entry.score += contribution;
                entry.vector_references.extend(candidate.vector_references);
                entry.contributions.extend(candidate.contributions);
                merge_properties(&mut entry.properties, candidate.properties)?;
            }
        }
        let mut values = merged.into_values().collect::<Vec<_>>();
        sort_candidates(&mut values);
        values.truncate(usize_limit(limit)?);
        self.register_stage("fusion-rrf", input, &mut values, exact, fusion)?;
        Ok(QueryCandidates { values, exact })
    }

    pub(super) fn rerank(
        &mut self,
        prefetch: &RetrievalPrefetch,
        stages: &[RetrievalRerankStage],
        limit: u64,
    ) -> Result<QueryCandidates> {
        let mut result = self.execute(&prefetch.query, prefetch.limit)?;
        for stage in stages {
            let input = result.values.len();
            let (kind, exact) = match stage {
                RetrievalRerankStage::ScoreBoost {
                    filter,
                    add_millionths,
                    multiply_millionths,
                } => {
                    let filter = internal_vector_filter(filter)?;
                    for candidate in &mut result.values {
                        if filter.matches(&candidate.properties) {
                            candidate.score = candidate.score * f64::from(*multiply_millionths)
                                / 1_000_000.0
                                + f64::from(*add_millionths) / 1_000_000.0;
                            if !candidate.score.is_finite() {
                                return Err(ServiceError::Vector(
                                    "retrieval score boost produced a non-finite score".into(),
                                ));
                            }
                        }
                    }
                    sort_candidates(&mut result.values);
                    ("score-boost", true)
                }
                RetrievalRerankStage::Exact { using, query } => {
                    self.exact_rerank(&mut result.values, using, query, None)?;
                    ("exact-rerank", true)
                }
                RetrievalRerankStage::Model {
                    using,
                    model,
                    query,
                } => {
                    self.exact_rerank(&mut result.values, using, query, Some(model))?;
                    ("model-rerank", true)
                }
                RetrievalRerankStage::Mmr {
                    using,
                    diversity_millionths,
                } => {
                    self.mmr(&mut result.values, using, *diversity_millionths)?;
                    ("mmr", true)
                }
            };
            self.register_stage(kind, input, &mut result.values, exact, stage)?;
        }
        let input = result.values.len();
        result.values.truncate(usize_limit(limit)?);
        self.register_stage("limit", input, &mut result.values, true, &limit)?;
        Ok(result)
    }

    pub(super) fn exact_rerank(
        &self,
        candidates: &mut Vec<RankedCandidate>,
        using: &CanonicalId,
        query: &VectorSearchQuery,
        model: Option<&VectorEmbeddingModel>,
    ) -> Result<()> {
        let config = self.configuration(using)?;
        validate_collection_query(config, query)?;
        if let Some(model) = model {
            let expected = config.embedding_model.as_ref().ok_or_else(|| {
                ServiceError::Vector("model rerank requires a model-bound named vector".into())
            })?;
            if expected.name != model.name || expected.digest != model.digest {
                return Err(ServiceError::Vector(
                    "model rerank identity differs from the named-vector binding".into(),
                ));
            }
        }
        let by_subject = self.visible_by_subject(using, None)?;
        let query = internal_vector_query(query);
        let mut reranked = Vec::with_capacity(candidates.len());
        for mut candidate in candidates.drain(..) {
            let Some(vector) = by_subject.get(&candidate.subject) else {
                continue;
            };
            let score = rrd_vector::score_query(&query, &vector.vector.value, config.metric)
                .map_err(core_vector)?;
            let reference = public_data_ref(&vector.vector.reference)?;
            candidate.score = score;
            candidate.vector_references.insert(reference);
            reranked.push(candidate);
        }
        *candidates = reranked;
        sort_candidates(candidates);
        Ok(())
    }

    pub(super) fn mmr(
        &self,
        candidates: &mut Vec<RankedCandidate>,
        using: &CanonicalId,
        diversity_millionths: u32,
    ) -> Result<()> {
        let config = self.configuration(using)?;
        let by_subject = self.visible_by_subject(using, None)?;
        candidates.retain(|candidate| by_subject.contains_key(&candidate.subject));
        sort_candidates(candidates);
        let mut remaining = std::mem::take(candidates)
            .into_iter()
            .enumerate()
            .collect::<Vec<_>>();
        let diversity = f64::from(diversity_millionths) / 1_000_000.0;
        let relevance_weight = 1.0 - diversity;
        let mut selected = Vec::<RankedCandidate>::new();
        while !remaining.is_empty() {
            let mut best = None::<(usize, f64)>;
            for (position, (original_rank, candidate)) in remaining.iter().enumerate() {
                let relevance = 1.0 / (*original_rank as f64 + 1.0);
                let candidate_vector = &by_subject[&candidate.subject].vector.value;
                let max_similarity = selected
                    .iter()
                    .filter_map(|chosen| by_subject.get(&chosen.subject))
                    .map(|chosen| {
                        let query = query_from_value(&chosen.vector.value);
                        rrd_vector::score_query(&query, candidate_vector, config.metric)
                            .map(bounded_similarity)
                    })
                    .collect::<rrd_core::Result<Vec<_>>>()
                    .map_err(core_vector)?
                    .into_iter()
                    .max_by(f64::total_cmp)
                    .unwrap_or(0.0);
                let score = relevance_weight * relevance - diversity * max_similarity;
                if best.is_none_or(|(_, current)| score > current) {
                    best = Some((position, score));
                }
            }
            let (position, score) = best.expect("remaining MMR candidates are non-empty");
            let (_, mut candidate) = remaining.remove(position);
            candidate.score = score;
            selected.push(candidate);
        }
        *candidates = selected;
        Ok(())
    }
}
