mod fusion;
mod shape;
mod stages;

use super::vector::{
    collection_versions, core_vector, internal_vector_filter, internal_vector_query,
    public_data_ref, read_vector_versions, validate_collection_query, vector_collection_error,
};

use super::*;

use std::collections::BTreeSet;

#[derive(Debug, Clone)]
struct RankedCandidate {
    subject: DataReference,
    vector_references: BTreeSet<DataReference>,
    score: f64,
    contributions: Vec<RetrievalContribution>,
    properties: RuntimeProperties,
}

#[derive(Debug)]
struct QueryCandidates {
    values: Vec<RankedCandidate>,
    exact: bool,
}

struct RetrievalExecutionContext<'a> {
    engine: &'a RrdEngine,
    request: &'a ExecuteRetrievalQuery,
    scope: ScopeId,
    read: ReadStamp,
    vectors: Vec<rrd_vector::VectorCandidate>,
    configurations: BTreeMap<ProjectionId, rrd_vector::NamedVectorConfig>,
    stages: Vec<RetrievalStageEvidence>,
    selected_versions: u64,
    read_evidence: Vec<rrd_contract::ReadEvidence>,
}

impl RrdEngine {
    /// Executes one bounded recursive retrieval program against one immutable
    /// RRD read stamp. Every leaf remains collection-bound; every fusion,
    /// reranking, and result-shaping transition publishes stage evidence.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_retrieval_query(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ExecuteRetrievalQuery,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<RetrievalQueryResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::QueryExecute,
            now,
            request_id,
            operation_id,
        )?;
        self.authorize(
            session_id,
            token,
            SecurityAction::VectorSearch,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let catalogue = rrd_vector::VectorCollectionRepository::new(&self.storage, scope.clone())
            .load()
            .map_err(vector_collection_error)?;
        let collection_id =
            ProjectionId::new(request.collection_id.as_str()).map_err(core_vector)?;
        let collection = catalogue.collections.get(&collection_id).ok_or_else(|| {
            ServiceError::Vector(format!(
                "unknown vector collection {}",
                request.collection_id
            ))
        })?;
        for property in required_payload_properties(&request.query, &request.result_shape) {
            let property_id = ProjectionId::new(&property).map_err(core_vector)?;
            if !collection.payload_indexes.contains_key(&property_id) {
                return Err(ServiceError::Vector(format!(
                    "retrieval property {property} is not an active typed payload index"
                )));
            }
        }
        let direct = read_vector_versions(self, &read, request.max_storage_keys)?;
        // The complete history of every identity with a version in this
        // collection, so each named vector resolves where it moved.
        let all = rrd_vector::candidates_from_changes(&direct.changes, &scope);
        let addressed = all
            .iter()
            .filter(|candidate| {
                candidate.vector.collection.collection_id == request.collection_id.as_str()
            })
            .map(|candidate| candidate.vector.reference.clone())
            .collect::<BTreeSet<_>>();
        let vectors = all
            .into_iter()
            .filter(|candidate| addressed.contains(&candidate.vector.reference))
            .collect();
        let mut context = RetrievalExecutionContext {
            engine: self,
            request,
            scope,
            read: read.clone(),
            vectors,
            configurations: collection.definition.vectors.clone(),
            stages: Vec::new(),
            selected_versions: direct.selected_versions,
            read_evidence: vec![direct.read_evidence],
        };
        let candidates = context.execute(&request.query, request.candidate_limit)?;
        let output = context.shape(candidates.values)?;
        let query_plan_sha256 = digest::sha256_hex(
            &serde_json::to_vec(&(
                &read.manifest_id,
                read.commit_cursor,
                request,
                &context.stages,
            ))
            .map_err(contract_json)?,
        );
        let read_evidence =
            crate::runtime::merge_read_evidence(request.max_storage_keys, context.read_evidence)?;
        Ok(RetrievalQueryResult {
            scope: request.scope.clone(),
            collection_id: request.collection_id.clone(),
            read_manifest_sha256: read.manifest_id,
            known_at_cursor: read.commit_cursor,
            selected_versions: context.selected_versions,
            read_evidence,
            query_plan_sha256,
            stages: context.stages,
            output,
        })
    }
}

impl RetrievalExecutionContext<'_> {
    fn execute(&mut self, query: &RetrievalQuery, limit: u64) -> Result<QueryCandidates> {
        match query {
            RetrievalQuery::Nearest {
                using,
                query,
                filter,
                mode,
            } => self.nearest(using, query, filter.as_ref(), *mode, limit),
            RetrievalQuery::Keyword {
                document_kind,
                text_field,
                query,
            } => self.keyword(document_kind, text_field, query, limit),
            RetrievalQuery::Recommend {
                using,
                positive,
                negative,
                strategy,
                filter,
            } => self.recommend(using, positive, negative, *strategy, filter.as_ref(), limit),
            RetrievalQuery::Discover {
                using,
                target,
                context,
                filter,
            } => self.discover(using, Some(target), context, filter.as_ref(), limit),
            RetrievalQuery::Context {
                using,
                context,
                filter,
            } => self.discover(using, None, context, filter.as_ref(), limit),
            RetrievalQuery::Fusion { prefetch, fusion } => self.fusion(prefetch, fusion, limit),
            RetrievalQuery::Rerank { prefetch, stages } => self.rerank(prefetch, stages, limit),
        }
    }

    fn visible_named(
        &self,
        using: &CanonicalId,
        filter: Option<&VectorPayloadFilter>,
    ) -> Result<Vec<rrd_vector::VectorCandidate>> {
        let config = self.configuration(using)?;
        let visibility = rrd_vector::VectorVisibilityRequest {
            scope: self.scope.clone(),
            read: self.read.clone(),
            valid_at: self.request.valid_at,
            field: config.field.clone(),
            embedding_model: config.embedding_model.clone(),
            filter: filter.map(internal_vector_filter).transpose()?,
        };
        let held = collection_versions(
            &visibility,
            self.vectors.clone(),
            &self.request.collection_id,
            using,
        )?;
        rrd_vector::materialize_visible(&visibility, held).map_err(core_vector)
    }

    fn visible_by_subject(
        &self,
        using: &CanonicalId,
        filter: Option<&VectorPayloadFilter>,
    ) -> Result<BTreeMap<DataReference, rrd_vector::VectorCandidate>> {
        let mut values = BTreeMap::new();
        for candidate in self.visible_named(using, filter)? {
            let subject = public_data_ref(&candidate.vector.subject)?;
            if values.insert(subject, candidate).is_some() {
                return Err(ServiceError::Vector(format!(
                    "collection subject has multiple active {using} vectors"
                )));
            }
        }
        Ok(values)
    }

    fn subject_payloads(&self) -> Result<BTreeMap<DataReference, RuntimeProperties>> {
        let mut payloads = BTreeMap::<DataReference, RuntimeProperties>::new();
        for name in self.configurations.keys() {
            let name = CanonicalId::new(name.as_str())
                .map_err(|error| ServiceError::Vector(error.to_string()))?;
            for candidate in self.visible_named(&name, None)? {
                let subject = public_data_ref(&candidate.vector.subject)?;
                merge_properties(
                    payloads.entry(subject).or_default(),
                    candidate.vector.properties,
                )?;
            }
        }
        Ok(payloads)
    }

    fn resolve_example(
        &self,
        using: &CanonicalId,
        example: &RetrievalVectorExample,
    ) -> Result<rrd_vector::VectorQuery> {
        let config = self.configuration(using)?;
        match example {
            RetrievalVectorExample::Vector { value } => {
                validate_collection_query(config, value)?;
                Ok(internal_vector_query(value))
            }
            RetrievalVectorExample::Reference { reference } => self
                .visible_named(using, None)?
                .into_iter()
                .find(|candidate| {
                    public_data_ref(&candidate.vector.reference)
                        .is_ok_and(|candidate| &candidate == reference)
                })
                .map(|candidate| query_from_value(&candidate.vector.value))
                .ok_or_else(|| {
                    ServiceError::Vector(format!(
                        "retrieval example {}:{} is not visible in named vector {using}",
                        reference.kind, reference.id
                    ))
                }),
        }
    }

    fn register_stage<T: Serialize + ?Sized>(
        &mut self,
        kind: &str,
        input: usize,
        candidates: &mut [RankedCandidate],
        exact: bool,
        contract: &T,
    ) -> Result<u32> {
        let ordinal = self.push_stage(kind, input, candidates.len(), exact, contract)?;
        for (offset, candidate) in candidates.iter_mut().enumerate() {
            candidate.contributions.push(RetrievalContribution {
                stage_ordinal: ordinal,
                rank: u64::try_from(offset + 1)
                    .map_err(|_| ServiceError::Vector("retrieval rank overflowed".into()))?,
                score: candidate.score,
            });
        }
        Ok(ordinal)
    }

    fn register_shape_stage<T: Serialize + ?Sized>(
        &mut self,
        kind: &str,
        input: usize,
        output: usize,
        contract: &T,
    ) -> Result<()> {
        self.push_stage(kind, input, output, true, contract)?;
        Ok(())
    }

    fn push_stage<T: Serialize + ?Sized>(
        &mut self,
        kind: &str,
        input: usize,
        output: usize,
        exact: bool,
        contract: &T,
    ) -> Result<u32> {
        let ordinal = u32::try_from(self.stages.len() + 1)
            .map_err(|_| ServiceError::Vector("retrieval stage ordinal overflowed".into()))?;
        let plan_sha256 = digest::sha256_hex(
            &serde_json::to_vec(&(
                &self.read.manifest_id,
                self.read.commit_cursor,
                ordinal,
                kind,
                contract,
            ))
            .map_err(contract_json)?,
        );
        self.stages.push(RetrievalStageEvidence {
            ordinal,
            kind: CanonicalId::new(kind)
                .map_err(|error| ServiceError::Contract(error.to_string()))?,
            input_candidates: u64::try_from(input)
                .map_err(|_| ServiceError::Vector("retrieval input count overflowed".into()))?,
            output_candidates: u64::try_from(output)
                .map_err(|_| ServiceError::Vector("retrieval output count overflowed".into()))?,
            exact,
            plan_sha256,
        });
        Ok(ordinal)
    }

    fn observe_read(
        &mut self,
        selected_versions: u64,
        evidence: rrd_contract::ReadEvidence,
    ) -> Result<()> {
        self.selected_versions = self
            .selected_versions
            .checked_add(selected_versions)
            .ok_or_else(|| ServiceError::Vector("retrieval selected versions overflowed".into()))?;
        let examined =
            self.read_evidence
                .iter()
                .try_fold(evidence.keys_examined, |total, read| {
                    total.checked_add(read.keys_examined).ok_or_else(|| {
                        ServiceError::Vector("retrieval read-key counter overflowed".into())
                    })
                })?;
        if examined > self.request.max_storage_keys {
            return Err(ServiceError::Vector(format!(
                "retrieval examined {examined} storage keys, budget allows {}",
                self.request.max_storage_keys
            )));
        }
        self.read_evidence.push(evidence);
        Ok(())
    }
}

fn text_subject(identity: &str, kind: &CanonicalId) -> Result<DataReference> {
    super::retrieval::text_subject(identity, kind)
}

fn required_payload_properties(
    query: &RetrievalQuery,
    shape: &RetrievalResultShape,
) -> BTreeSet<String> {
    let mut properties = BTreeSet::new();
    fn visit(query: &RetrievalQuery, properties: &mut BTreeSet<String>) {
        match query {
            RetrievalQuery::Nearest { filter, .. }
            | RetrievalQuery::Recommend { filter, .. }
            | RetrievalQuery::Discover { filter, .. }
            | RetrievalQuery::Context { filter, .. } => {
                if let Some(filter) = filter {
                    properties.extend(public_filter_properties(filter));
                }
            }
            RetrievalQuery::Fusion { prefetch, .. } => {
                for branch in prefetch {
                    visit(&branch.query, properties);
                }
            }
            RetrievalQuery::Rerank { prefetch, stages } => {
                visit(&prefetch.query, properties);
                for stage in stages {
                    if let RetrievalRerankStage::ScoreBoost { filter, .. } = stage {
                        properties.extend(public_filter_properties(filter));
                    }
                }
            }
            RetrievalQuery::Keyword { .. } => {}
        }
    }
    visit(query, &mut properties);
    match shape {
        RetrievalResultShape::Groups { property, .. }
        | RetrievalResultShape::Facets { property, .. } => {
            properties.insert(property.as_str().to_owned());
        }
        RetrievalResultShape::Points | RetrievalResultShape::Matrix { .. } => {}
    }
    properties
}

fn public_filter_properties(filter: &VectorPayloadFilter) -> Vec<String> {
    let mut properties = Vec::new();
    match filter {
        VectorPayloadFilter::Condition { condition } => {
            properties.push(condition.property.as_str().to_owned());
        }
        VectorPayloadFilter::All { filters } | VectorPayloadFilter::Any { filters } => {
            for filter in filters {
                properties.extend(public_filter_properties(filter));
            }
        }
        VectorPayloadFilter::Not { filter } => {
            properties.extend(public_filter_properties(filter));
        }
    }
    properties
}

fn average_query(
    positive: &[rrd_vector::VectorQuery],
    negative: &[rrd_vector::VectorQuery],
) -> Result<rrd_vector::VectorQuery> {
    let values = positive
        .iter()
        .map(rrd_vector::VectorQuery::as_value)
        .chain(negative.iter().map(rrd_vector::VectorQuery::as_value))
        .collect::<Vec<_>>();
    let first = values
        .first()
        .ok_or_else(|| ServiceError::Vector("recommendation has no positive vector".into()))?;
    let signed_scale = |position: usize| {
        if position < positive.len() {
            1.0 / positive.len() as f32
        } else if negative.is_empty() {
            0.0
        } else {
            -1.0 / negative.len() as f32
        }
    };
    let value = match first {
        VectorValue::Dense { values: first } => {
            let mut mean = vec![0.0_f32; first.len()];
            for (position, value) in values.iter().enumerate() {
                let VectorValue::Dense { values } = value else {
                    return Err(ServiceError::Vector(
                        "recommendation examples use different vector kinds".into(),
                    ));
                };
                if values.len() != mean.len() {
                    return Err(ServiceError::Vector(
                        "recommendation examples use different dimensions".into(),
                    ));
                }
                let scale = signed_scale(position);
                for (output, value) in mean.iter_mut().zip(values) {
                    *output += value * scale;
                }
            }
            VectorValue::Dense { values: mean }
        }
        VectorValue::Sparse {
            dimensions,
            indices: _,
            values: _,
        } => {
            let mut mean = BTreeMap::<u32, f32>::new();
            for (position, value) in values.iter().enumerate() {
                let VectorValue::Sparse {
                    dimensions: current,
                    indices,
                    values,
                } = value
                else {
                    return Err(ServiceError::Vector(
                        "recommendation examples use different vector kinds".into(),
                    ));
                };
                if current != dimensions {
                    return Err(ServiceError::Vector(
                        "recommendation examples use different dimensions".into(),
                    ));
                }
                let scale = signed_scale(position);
                for (index, value) in indices.iter().zip(values) {
                    *mean.entry(*index).or_default() += value * scale;
                }
            }
            mean.retain(|_, value| *value != 0.0);
            VectorValue::Sparse {
                dimensions: *dimensions,
                indices: mean.keys().copied().collect(),
                values: mean.values().copied().collect(),
            }
        }
        VectorValue::MultiDense {
            dimensions,
            vectors: first,
        } => {
            let mut mean = vec![vec![0.0_f32; *dimensions as usize]; first.len()];
            for (position, value) in values.iter().enumerate() {
                let VectorValue::MultiDense {
                    dimensions: current,
                    vectors,
                } = value
                else {
                    return Err(ServiceError::Vector(
                        "recommendation examples use different vector kinds".into(),
                    ));
                };
                if current != dimensions || vectors.len() != mean.len() {
                    return Err(ServiceError::Vector(
                        "multi-vector recommendation examples use different shapes".into(),
                    ));
                }
                let scale = signed_scale(position);
                for (output_row, row) in mean.iter_mut().zip(vectors) {
                    for (output, value) in output_row.iter_mut().zip(row) {
                        *output += value * scale;
                    }
                }
            }
            VectorValue::MultiDense {
                dimensions: *dimensions,
                vectors: mean,
            }
        }
    };
    value.validate().map_err(core_vector)?;
    Ok(query_from_value(&value))
}

fn best_score_recommendation(
    candidate: &VectorValue,
    positive: &[rrd_vector::VectorQuery],
    negative: &[rrd_vector::VectorQuery],
    metric: rrd_vector::ScoreMetric,
) -> rrd_core::Result<f64> {
    let positive = positive
        .iter()
        .map(|query| rrd_vector::score_query(query, candidate, metric))
        .collect::<rrd_core::Result<Vec<_>>>()?
        .into_iter()
        .max_by(f64::total_cmp)
        .expect("recommendation has positive examples");
    let negative = negative
        .iter()
        .map(|query| rrd_vector::score_query(query, candidate, metric))
        .collect::<rrd_core::Result<Vec<_>>>()?
        .into_iter()
        .max_by(f64::total_cmp);
    Ok(negative.map_or(positive, |negative| {
        if positive > negative {
            positive
        } else {
            -negative
        }
    }))
}

fn discovery_score(
    candidate: &VectorValue,
    target: Option<&rrd_vector::VectorQuery>,
    context: &[(rrd_vector::VectorQuery, rrd_vector::VectorQuery)],
    metric: rrd_vector::ScoreMetric,
) -> rrd_core::Result<f64> {
    let mut wins = 0_u64;
    let mut bounded_margin = 0.0;
    for (positive, negative) in context {
        let margin = rrd_vector::score_query(positive, candidate, metric)?
            - rrd_vector::score_query(negative, candidate, metric)?;
        if margin > 0.0 {
            wins += 1;
        }
        bounded_margin += margin.atan() / std::f64::consts::PI;
    }
    let context_score = wins as f64 + bounded_margin / context.len() as f64;
    let target_score = target
        .map(|target| rrd_vector::score_query(target, candidate, metric))
        .transpose()?
        .map_or(0.0, |score| score.atan() / std::f64::consts::PI * 1e-6);
    Ok(context_score + target_score)
}

fn query_from_value(value: &VectorValue) -> rrd_vector::VectorQuery {
    match value {
        VectorValue::Dense { values } => rrd_vector::VectorQuery::Dense {
            values: values.clone(),
        },
        VectorValue::Sparse {
            dimensions,
            indices,
            values,
        } => rrd_vector::VectorQuery::Sparse {
            dimensions: *dimensions,
            indices: indices.clone(),
            values: values.clone(),
        },
        VectorValue::MultiDense {
            dimensions,
            vectors,
        } => rrd_vector::VectorQuery::MultiDense {
            dimensions: *dimensions,
            vectors: vectors.clone(),
            comparator: rrd_vector::MultiVectorComparator::MaxSim,
        },
    }
}

fn bounded_similarity(score: f64) -> f64 {
    0.5 + score.atan() / std::f64::consts::PI
}

fn sort_candidates(candidates: &mut [RankedCandidate]) {
    candidates.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.subject.cmp(&right.subject))
    });
}

fn merge_properties(target: &mut RuntimeProperties, source: RuntimeProperties) -> Result<()> {
    for (name, value) in source {
        if target
            .insert(name.clone(), value.clone())
            .is_some_and(|current| current != value)
        {
            return Err(ServiceError::Vector(format!(
                "named vectors disagree on point payload property {name}"
            )));
        }
    }
    Ok(())
}

fn public_hit(candidate: RankedCandidate) -> RetrievalHit {
    RetrievalHit {
        subject: candidate.subject,
        vector_references: candidate.vector_references.into_iter().collect(),
        score: candidate.score,
        contributions: candidate.contributions,
    }
}

fn usize_limit(limit: u64) -> Result<usize> {
    usize::try_from(limit).map_err(|_| ServiceError::Vector("retrieval limit exceeds usize".into()))
}
