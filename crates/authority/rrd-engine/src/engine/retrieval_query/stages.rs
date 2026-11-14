//! Nearest, keyword, recommend and discover retrieval stages.

use super::*;

impl RetrievalExecutionContext<'_> {
    pub(super) fn nearest(
        &mut self,
        using: &CanonicalId,
        query: &VectorSearchQuery,
        filter: Option<&VectorPayloadFilter>,
        mode: VectorSearchMode,
        limit: u64,
    ) -> Result<QueryCandidates> {
        let vector_request = SearchVectors {
            scope: self.request.scope.clone(),
            valid_at: self.request.valid_at,
            collection_id: Some(self.request.collection_id.clone()),
            vector_name: Some(using.clone()),
            field: None,
            query: query.clone(),
            filter: filter.cloned(),
            metric: None,
            top_k: limit,
            mode,
            max_storage_keys: self.request.max_storage_keys,
        };
        let search = self
            .engine
            .search_vectors_at(&vector_request, self.read.clone())?;
        let visible = self.visible_named(using, filter)?;
        let by_version = visible
            .into_iter()
            .map(|candidate| {
                Ok((
                    (
                        public_data_ref(&candidate.vector.reference)?,
                        candidate.source_cursor,
                    ),
                    candidate,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let search_exact = search.exact;
        let search_selected_versions = search.selected_versions;
        let search_read_evidence = search.read_evidence.clone();
        let mut values = search
            .hits
            .into_iter()
            .map(|hit| {
                let candidate = by_version
                    .get(&(hit.reference.clone(), hit.source_cursor))
                    .ok_or_else(|| {
                        ServiceError::Vector(
                            "vector search hit is absent from its authoritative visible set".into(),
                        )
                    })?;
                Ok(RankedCandidate {
                    subject: hit.subject,
                    vector_references: BTreeSet::from([hit.reference]),
                    score: hit.score,
                    contributions: Vec::new(),
                    properties: candidate.vector.properties.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let input = self.vectors.len();
        self.register_stage("nearest", input, &mut values, search_exact, &vector_request)?;
        self.observe_read(search_selected_versions, search_read_evidence)?;
        Ok(QueryCandidates {
            values,
            exact: search_exact,
        })
    }

    pub(super) fn keyword(
        &mut self,
        document_kind: &CanonicalId,
        text_field: &CanonicalId,
        text: &str,
        limit: u64,
    ) -> Result<QueryCandidates> {
        let query = rrd_query::parse(&format!(
            "FROM record:{document_kind} AT VALID {} KNOWN {} WHERE {text_field} MATCH $retrieval_text LIMIT {limit}",
            self.request.valid_at, self.read.commit_cursor,
        ))
        .map_err(|error| ServiceError::Query(error.to_string()))?;
        let parameters = rrd_query::Parameters::from([(
            "retrieval_text".into(),
            RuntimeValue::String(text.to_owned()),
        )]);
        let pipeline =
            rrd_query::StampedQueryPipeline::new(&self.engine.storage, self.read.clone())
                .map_err(ServiceError::from)?;
        let limit = usize::try_from(limit)
            .map_err(|_| ServiceError::Query("retrieval keyword limit exceeds usize".into()))?;
        let max_input_rows = usize::try_from(self.request.max_storage_keys)
            .map_err(|_| ServiceError::Query("retrieval key budget exceeds usize".into()))?;
        let budget = rrd_query::ExecutionBudget {
            max_storage_keys: self.request.max_storage_keys,
            max_input_rows,
            max_rows: limit,
            max_output_bytes: 8 * 1024 * 1024,
            max_batch_rows: limit.clamp(1, 1_024),
            ..rrd_query::ExecutionBudget::default()
        };
        let bound = pipeline
            .bind(&query, &parameters, &budget)
            .map_err(ServiceError::from)?;
        let plan = pipeline.plan(&bound).map_err(ServiceError::from)?;
        let execution = pipeline
            .execute(&plan, &budget)
            .map_err(ServiceError::from)?;
        if execution.truncated
            || execution.read_manifest != self.read.manifest_id
            || execution.known_at_cursor != self.read.commit_cursor
        {
            return Err(ServiceError::StorageConflict(
                "retrieval keyword branch was truncated or changed read stamp".into(),
            ));
        }
        let payloads = self.subject_payloads()?;
        let mut values = execution
            .batches
            .iter()
            .flat_map(|batch| batch.rows.iter())
            .map(|row| {
                let subject = text_subject(&row.identity, document_kind)?;
                let Some(properties) = payloads.get(&subject).cloned() else {
                    return Ok(None);
                };
                let score = match row.values.get("_score") {
                    Some(RuntimeValue::Decimal(value)) => value.parse::<f64>().map_err(|_| {
                        ServiceError::Query("keyword score is not a finite decimal".into())
                    })?,
                    _ => {
                        return Err(ServiceError::Query(
                            "keyword result is missing its score".into(),
                        ))
                    }
                };
                if !score.is_finite() {
                    return Err(ServiceError::Query("keyword score is not finite".into()));
                }
                Ok(Some(RankedCandidate {
                    subject,
                    vector_references: BTreeSet::new(),
                    score,
                    contributions: Vec::new(),
                    properties,
                }))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        sort_candidates(&mut values);
        values.truncate(limit);
        self.register_stage("keyword", payloads.len(), &mut values, true, &plan.digest)?;
        let selected_versions = u64::try_from(execution.selected_versions)
            .map_err(|_| ServiceError::Query("selected keyword versions exceed u64".into()))?;
        self.observe_read(
            selected_versions,
            crate::runtime::public_read_evidence(execution.read_evidence)?,
        )?;
        Ok(QueryCandidates {
            values,
            exact: true,
        })
    }

    pub(super) fn recommend(
        &mut self,
        using: &CanonicalId,
        positive: &[RetrievalVectorExample],
        negative: &[RetrievalVectorExample],
        strategy: RetrievalRecommendStrategy,
        filter: Option<&VectorPayloadFilter>,
        limit: u64,
    ) -> Result<QueryCandidates> {
        let config = self.configuration(using)?.clone();
        let positive = positive
            .iter()
            .map(|example| self.resolve_example(using, example))
            .collect::<Result<Vec<_>>>()?;
        let negative = negative
            .iter()
            .map(|example| self.resolve_example(using, example))
            .collect::<Result<Vec<_>>>()?;
        let averaged = (strategy == RetrievalRecommendStrategy::AverageVector)
            .then(|| average_query(&positive, &negative))
            .transpose()?;
        let visible = self.visible_named(using, filter)?;
        let input = visible.len();
        let mut values = visible
            .into_iter()
            .map(|candidate| {
                let score = match &averaged {
                    Some(query) => {
                        rrd_vector::score_query(query, &candidate.vector.value, config.metric)
                    }
                    None => best_score_recommendation(
                        &candidate.vector.value,
                        &positive,
                        &negative,
                        config.metric,
                    ),
                }
                .map_err(core_vector)?;
                Ok(RankedCandidate {
                    subject: public_data_ref(&candidate.vector.subject)?,
                    vector_references: BTreeSet::from([public_data_ref(
                        &candidate.vector.reference,
                    )?]),
                    score,
                    contributions: Vec::new(),
                    properties: candidate.vector.properties.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        sort_candidates(&mut values);
        values.truncate(usize_limit(limit)?);
        self.register_stage(
            "recommend",
            input,
            &mut values,
            true,
            &(using, positive.len(), negative.len(), strategy),
        )?;
        Ok(QueryCandidates {
            values,
            exact: true,
        })
    }

    pub(super) fn discover(
        &mut self,
        using: &CanonicalId,
        target: Option<&RetrievalVectorExample>,
        context: &[RetrievalContextPair],
        filter: Option<&VectorPayloadFilter>,
        limit: u64,
    ) -> Result<QueryCandidates> {
        let config = self.configuration(using)?.clone();
        let target = target
            .map(|example| self.resolve_example(using, example))
            .transpose()?;
        let context = context
            .iter()
            .map(|pair| {
                Ok((
                    self.resolve_example(using, &pair.positive)?,
                    self.resolve_example(using, &pair.negative)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let visible = self.visible_named(using, filter)?;
        let input = visible.len();
        let mut values = visible
            .into_iter()
            .map(|candidate| {
                let score = discovery_score(
                    &candidate.vector.value,
                    target.as_ref(),
                    &context,
                    config.metric,
                )
                .map_err(core_vector)?;
                Ok(RankedCandidate {
                    subject: public_data_ref(&candidate.vector.subject)?,
                    vector_references: BTreeSet::from([public_data_ref(
                        &candidate.vector.reference,
                    )?]),
                    score,
                    contributions: Vec::new(),
                    properties: candidate.vector.properties.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        sort_candidates(&mut values);
        values.truncate(usize_limit(limit)?);
        let kind = if target.is_some() {
            "discover"
        } else {
            "context"
        };
        self.register_stage(kind, input, &mut values, true, &(using, context.len()))?;
        Ok(QueryCandidates {
            values,
            exact: true,
        })
    }
}
