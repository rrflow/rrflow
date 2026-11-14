//! Result shaping and stage configuration.

use super::*;

impl RetrievalExecutionContext<'_> {
    pub(super) fn shape(
        &mut self,
        mut candidates: Vec<RankedCandidate>,
    ) -> Result<RetrievalOutput> {
        sort_candidates(&mut candidates);
        match &self.request.result_shape {
            RetrievalResultShape::Points => {
                let input = candidates.len();
                candidates.truncate(usize_limit(self.request.limit)?);
                self.register_stage("points", input, &mut candidates, true, &self.request.limit)?;
                Ok(RetrievalOutput::Points {
                    hits: candidates.into_iter().map(public_hit).collect(),
                })
            }
            RetrievalResultShape::Groups {
                property,
                max_groups,
                hits_per_group,
            } => {
                let input = candidates.len();
                let mut grouped = BTreeMap::<Vec<u8>, (QueryValue, Vec<RankedCandidate>)>::new();
                for candidate in candidates {
                    let Some(value) = candidate.properties.get(property.as_str()) else {
                        continue;
                    };
                    let public = query_value(value)?;
                    let key = serde_json::to_vec(&public).map_err(contract_json)?;
                    grouped
                        .entry(key)
                        .or_insert_with(|| (public, Vec::new()))
                        .1
                        .push(candidate);
                }
                let mut grouped = grouped.into_values().collect::<Vec<_>>();
                grouped.sort_by(|left, right| {
                    right
                        .1
                        .first()
                        .map(|candidate| candidate.score)
                        .unwrap_or(f64::NEG_INFINITY)
                        .total_cmp(
                            &left
                                .1
                                .first()
                                .map(|candidate| candidate.score)
                                .unwrap_or(f64::NEG_INFINITY),
                        )
                });
                grouped.truncate(usize_limit(*max_groups)?);
                let mut groups = Vec::with_capacity(grouped.len());
                for (value, mut hits) in grouped {
                    hits.truncate(usize_limit(*hits_per_group)?);
                    groups.push(RetrievalGroup {
                        value,
                        hits: hits.into_iter().map(public_hit).collect(),
                    });
                }
                self.register_shape_stage(
                    "groups",
                    input,
                    groups.len(),
                    &self.request.result_shape,
                )?;
                Ok(RetrievalOutput::Groups { groups })
            }
            RetrievalResultShape::Facets { property, limit } => {
                let input = candidates.len();
                let mut counts = BTreeMap::<Vec<u8>, (QueryValue, u64)>::new();
                for candidate in candidates {
                    let Some(value) = candidate.properties.get(property.as_str()) else {
                        continue;
                    };
                    let public = query_value(value)?;
                    let key = serde_json::to_vec(&public).map_err(contract_json)?;
                    let entry = counts.entry(key).or_insert((public, 0));
                    entry.1 = entry.1.checked_add(1).ok_or_else(|| {
                        ServiceError::Vector("retrieval facet count overflowed".into())
                    })?;
                }
                let mut counts = counts.into_iter().collect::<Vec<_>>();
                counts.sort_by(|left, right| {
                    right
                        .1
                         .1
                        .cmp(&left.1 .1)
                        .then_with(|| left.0.cmp(&right.0))
                });
                counts.truncate(usize_limit(*limit)?);
                let facets = counts
                    .into_iter()
                    .map(|(_, (value, count))| RetrievalFacet { value, count })
                    .collect::<Vec<_>>();
                self.register_shape_stage(
                    "facets",
                    input,
                    facets.len(),
                    &self.request.result_shape,
                )?;
                Ok(RetrievalOutput::Facets { facets })
            }
            RetrievalResultShape::Matrix { using, sample } => {
                let input = candidates.len();
                let config = self.configuration(using)?.clone();
                let by_subject = self.visible_by_subject(using, None)?;
                let sampled = candidates
                    .into_iter()
                    .filter(|candidate| by_subject.contains_key(&candidate.subject))
                    .take(usize_limit(*sample)?)
                    .collect::<Vec<_>>();
                let subjects = sampled
                    .iter()
                    .map(|candidate| candidate.subject.clone())
                    .collect::<Vec<_>>();
                let mut cells = Vec::new();
                for left in &sampled {
                    let left_vector = &by_subject[&left.subject].vector.value;
                    let query = query_from_value(left_vector);
                    for right in &sampled {
                        if left.subject == right.subject {
                            continue;
                        }
                        let right_vector = &by_subject[&right.subject].vector.value;
                        cells.push(RetrievalMatrixCell {
                            left: left.subject.clone(),
                            right: right.subject.clone(),
                            score: rrd_vector::score_query(&query, right_vector, config.metric)
                                .map_err(core_vector)?,
                        });
                    }
                }
                self.register_shape_stage(
                    "matrix",
                    input,
                    cells.len(),
                    &self.request.result_shape,
                )?;
                Ok(RetrievalOutput::Matrix { subjects, cells })
            }
        }
    }

    pub(super) fn configuration(
        &self,
        using: &CanonicalId,
    ) -> Result<&rrd_vector::NamedVectorConfig> {
        self.configurations
            .get(&ProjectionId::new(using.as_str()).map_err(core_vector)?)
            .ok_or_else(|| {
                ServiceError::Vector(format!(
                    "unknown named vector {using} in collection {}",
                    self.request.collection_id
                ))
            })
    }
}
