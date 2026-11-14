//! Executing operator searches and the reference adapter.

use super::*;

/// Validates both sides of the adapter call. An adapter cannot silently change
/// project, model space, source, path, or freshness policy.
pub fn execute_operator_search<A: OperatorKnowledgeAdapter>(
    adapter: &mut A,
    binding: &OperatorKnowledgeBinding,
    request: &OperatorSearchRequest,
) -> Result<OperatorSearchResult> {
    binding.validate()?;
    request.validate()?;
    let descriptor = adapter.descriptor().clone();
    descriptor.validate()?;
    if descriptor.adapter != binding.adapter {
        return invalid("operator adapter does not match the project binding");
    }
    if request.binding_digest != binding.digest()? {
        return invalid("operator request uses another project binding");
    }
    if request.search.scope != binding.scope {
        return invalid("operator request scope differs from its project binding");
    }
    if binding.projection.source_cursor < request.required_source_cursor
        || binding.projection.source_cursor > request.search.read.commit_cursor
    {
        return invalid("operator projection is stale or newer than the requested read stamp");
    }
    if request.search.query.dimensions() != binding.dimensions as usize
        || request.search.query.dimensions() > descriptor.max_dimensions as usize
    {
        return invalid("operator query dimensions differ from the bound model space");
    }
    if request.search.embedding_model.as_ref() != Some(&binding.model) {
        return invalid("operator query does not bind the exact project model space");
    }
    if !descriptor
        .vector_kinds
        .contains(&vector_kind(&request.search.query))
        || !descriptor
            .search_capabilities
            .get(&request.controls.requested_path)
            .is_some_and(|metrics| metrics.contains(&request.search.metric))
    {
        return invalid("operator adapter lacks a requested search capability");
    }
    if !descriptor.supports_tenant_filter {
        return invalid("operator adapter cannot enforce the bound tenant filter");
    }
    if request.search.filter.is_some() && !descriptor.supports_payload_filter {
        return invalid("operator adapter cannot enforce the requested payload filter");
    }
    if request.expected_stable_revision.is_some() && !descriptor.supports_stable_revision {
        return invalid("operator adapter cannot enforce a stable source revision");
    }

    let result = adapter.search(binding, request)?;
    if adapter.descriptor() != &descriptor {
        return invalid("operator adapter changed its descriptor during execution");
    }
    result.validate()?;
    if result.request_digest != request.digest()?
        || result.revision.adapter != binding.adapter
        || result.revision.project_id != binding.project_id
        || result.revision.source_identity_digest != binding.source_identity_digest
        || result.hits.len() > request.search.top_k
    {
        return invalid("operator adapter result differs from the bound request or source");
    }
    if result.plan.controls != request.controls {
        return invalid("operator adapter reported controls other than the sealed request");
    }
    for pair in result.hits.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if left.score.total_cmp(&right.score).is_lt()
            || (left.score.total_cmp(&right.score).is_eq()
                && (&left.external_id, &left.subject_id) > (&right.external_id, &right.subject_id))
        {
            return invalid("operator adapter returned hits outside canonical best-first order");
        }
    }
    if let Some(expected) = &request.expected_stable_revision {
        if result.revision.stable_revision.as_ref() != Some(expected) {
            return invalid("operator source revision is stale");
        }
    }
    if result.plan.selected_path != request.controls.requested_path
        && (!matches!(request.search.mode, SearchMode::AllowApproximate { .. })
            || result.plan.selected_path != OperatorAccessPath::Exact
            || result.plan.fallback_reason_digest.is_none())
    {
        return invalid("operator adapter performed an unauthorized access-path fallback");
    }
    if !descriptor
        .search_capabilities
        .get(&result.plan.selected_path)
        .is_some_and(|metrics| metrics.contains(&request.search.metric))
    {
        return invalid("operator adapter reported an undeclared selected access path");
    }
    Ok(result)
}

/// Deterministic adapter used as a semantic oracle for external implementations.
pub struct ReferenceOperatorAdapter {
    descriptor: OperatorAdapterDescriptor,
    binding_digest: String,
    revision: OperatorSourceRevision,
    runtime: VectorRuntime,
}

impl ReferenceOperatorAdapter {
    pub fn new(
        descriptor: OperatorAdapterDescriptor,
        binding: &OperatorKnowledgeBinding,
        revision: OperatorSourceRevision,
        runtime: VectorRuntime,
    ) -> Result<Self> {
        descriptor.validate()?;
        binding.validate()?;
        revision.validate()?;
        Ok(Self {
            descriptor,
            binding_digest: binding.digest()?,
            revision,
            runtime,
        })
    }
}

impl OperatorKnowledgeAdapter for ReferenceOperatorAdapter {
    fn descriptor(&self) -> &OperatorAdapterDescriptor {
        &self.descriptor
    }

    fn search(
        &mut self,
        binding: &OperatorKnowledgeBinding,
        request: &OperatorSearchRequest,
    ) -> Result<OperatorSearchResult> {
        if binding.digest()? != self.binding_digest {
            return invalid("reference operator adapter is bound to another project");
        }
        if request.controls.requested_path != OperatorAccessPath::Exact {
            return invalid("reference operator adapter implements the exact oracle only");
        }
        let execution = self.runtime.search(&request.search, 1)?;
        let plan_digest = content_digest(b"rrflow-reference-operator-plan-v1\0", &execution.plan)?;
        Ok(OperatorSearchResult {
            contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
            request_digest: request.digest()?,
            revision: self.revision.clone(),
            plan: OperatorPlanEvidence {
                selected_path: OperatorAccessPath::Exact,
                plan_digest,
                index_digest: None,
                fallback_reason_digest: None,
                controls: request.controls.clone(),
                filter_applied_after_ann: false,
                ordering_exact: true,
                candidates_examined: None,
            },
            hits: execution
                .hits
                .into_iter()
                .map(|hit| OperatorSearchHit {
                    external_id: format!("{}:{}", hit.reference.kind, hit.reference.id),
                    subject_id: format!("{}:{}", hit.subject.kind, hit.subject.id),
                    score: hit.score,
                })
                .collect(),
            elapsed_micros: 0,
        })
    }
}
