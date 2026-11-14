//! Route step decisions, their bindings and digests.

use super::*;

impl RouteStepDecision {
    pub const fn kind(&self) -> RouteDecisionKind {
        match self {
            Self::SelectRecipe { .. } => RouteDecisionKind::SelectRecipe,
            Self::AdvanceBranch { .. } => RouteDecisionKind::AdvanceBranch,
            Self::RequestContext { .. } => RouteDecisionKind::RequestContext,
        }
    }

    pub fn validate_for(
        &self,
        request: &RouteStepRequest,
        backend: &RouterBackendDescriptor,
    ) -> Result<()> {
        request.validate_for(backend)?;
        let binding = self.binding();
        validate_contract_version(binding.contract_version)?;
        if binding.request_id != &request.id || binding.request_sha256 != request.request_sha256 {
            return invalid("route decision does not bind its request identity and digest");
        }
        validate_sha256(binding.request_sha256, "route decision request_sha256")?;
        if binding.decided_at_unix_ms < request.requested_at_unix_ms
            || binding.decided_at_unix_ms > request.deadline_unix_ms
        {
            return invalid("route decision lies outside its request time window");
        }
        if !request.allowed_decisions.contains(&self.kind())
            || !backend.decisions.contains(&self.kind())
        {
            return invalid("route decision kind was not allowed by request and backend");
        }
        match self {
            Self::SelectRecipe {
                recipe_id,
                recipe_revision,
                parameters,
                ..
            } => {
                if !request.recipe_candidates.iter().any(|candidate| {
                    candidate.id == *recipe_id && candidate.revision == *recipe_revision
                }) {
                    return invalid("route decision selected an unknown recipe revision");
                }
                if parameters.len() > backend.limits.maximum_parameters as usize {
                    return invalid("route recipe parameters exceed the backend limit");
                }
                validate_parameters(parameters)?;
            }
            Self::AdvanceBranch { edge_id, .. } => {
                if !request
                    .branch_candidates
                    .iter()
                    .any(|candidate| candidate.id == *edge_id)
                {
                    return invalid("route decision selected an unknown branch");
                }
            }
            Self::RequestContext {
                query,
                seeds,
                budget,
                ..
            } => validate_context_decision(query, seeds, budget, request, backend)?,
        }
        validate_sha256(binding.decision_sha256, "route decision_sha256")?;
        if binding.decision_sha256 != route_step_decision_sha256(self)? {
            return invalid("route decision digest does not match its content");
        }
        validate_encoded_size(
            self,
            backend.limits.maximum_response_bytes,
            "route decision",
        )
    }

    fn binding(&self) -> RouteDecisionBinding<'_> {
        match self {
            Self::SelectRecipe {
                contract_version,
                request_id,
                request_sha256,
                decided_at_unix_ms,
                decision_sha256,
                ..
            }
            | Self::AdvanceBranch {
                contract_version,
                request_id,
                request_sha256,
                decided_at_unix_ms,
                decision_sha256,
                ..
            }
            | Self::RequestContext {
                contract_version,
                request_id,
                request_sha256,
                decided_at_unix_ms,
                decision_sha256,
                ..
            } => RouteDecisionBinding {
                contract_version: *contract_version,
                request_id,
                request_sha256,
                decided_at_unix_ms: *decided_at_unix_ms,
                decision_sha256,
            },
        }
    }
}

pub(super) struct RouteDecisionBinding<'a> {
    contract_version: u16,
    request_id: &'a CorrelationId,
    request_sha256: &'a str,
    decided_at_unix_ms: u64,
    decision_sha256: &'a str,
}

pub fn router_backend_descriptor_sha256(descriptor: &RouterBackendDescriptor) -> Result<String> {
    digest_json(
        b"rrflow-router-backend-descriptor-v1",
        &(
            descriptor.contract_version,
            &descriptor.id,
            descriptor.revision,
            &descriptor.model_manifest_id,
            descriptor.model_manifest_revision,
            &descriptor.model_manifest_sha256,
            &descriptor.decisions,
            &descriptor.limits,
        ),
    )
}

/// Digest of the exact closed JSON Schema that constrained router output must
/// satisfy. Canonical object ordering prevents workspace feature unification
/// from changing this identity.
pub fn route_step_decision_schema_sha256() -> Result<String> {
    let schema =
        serde_json::to_value(schemars::schema_for!(RouteStepDecision)).map_err(|error| {
            crate::ContractError(format!("router decision schema encoding failed: {error}"))
        })?;
    let encoded = serde_json::to_vec(&crate::canonical_json(schema)).map_err(|error| {
        crate::ContractError(format!(
            "router decision schema serialization failed: {error}"
        ))
    })?;
    Ok(sha256_bytes(&encoded))
}

pub fn route_step_request_sha256(request: &RouteStepRequest) -> Result<String> {
    digest_json(
        b"rrflow-route-step-request-v1",
        &(
            request.contract_version,
            &request.id,
            &request.backend_id,
            request.backend_revision,
            &request.backend_descriptor_sha256,
            &request.cursor,
            &request.intent,
            &request.signals,
            &request.recipe_candidates,
            &request.branch_candidates,
            &request.context,
            &request.allowed_decisions,
            request.requested_at_unix_ms,
            request.deadline_unix_ms,
        ),
    )
}

pub fn route_step_decision_sha256(decision: &RouteStepDecision) -> Result<String> {
    match decision {
        RouteStepDecision::SelectRecipe {
            contract_version,
            request_id,
            request_sha256,
            recipe_id,
            recipe_revision,
            parameters,
            decided_at_unix_ms,
            ..
        } => digest_json(
            b"rrflow-route-step-decision-v1",
            &(
                "select_recipe",
                contract_version,
                request_id,
                request_sha256,
                recipe_id,
                recipe_revision,
                parameters,
                decided_at_unix_ms,
            ),
        ),
        RouteStepDecision::AdvanceBranch {
            contract_version,
            request_id,
            request_sha256,
            edge_id,
            decided_at_unix_ms,
            ..
        } => digest_json(
            b"rrflow-route-step-decision-v1",
            &(
                "advance_branch",
                contract_version,
                request_id,
                request_sha256,
                edge_id,
                decided_at_unix_ms,
            ),
        ),
        RouteStepDecision::RequestContext {
            contract_version,
            request_id,
            request_sha256,
            query,
            seeds,
            budget,
            decided_at_unix_ms,
            ..
        } => digest_json(
            b"rrflow-route-step-decision-v1",
            &(
                "request_context",
                contract_version,
                request_id,
                request_sha256,
                query,
                seeds,
                budget,
                decided_at_unix_ms,
            ),
        ),
    }
}

pub(super) fn validate_context_decision(
    query: &str,
    seeds: &[DataReference],
    budget: &RouteContextBudget,
    request: &RouteStepRequest,
    backend: &RouterBackendDescriptor,
) -> Result<()> {
    let Some(allowance) = &request.context else {
        return invalid("route context decision has no request allowance");
    };
    if query.len() as u64 > allowance.maximum_query_bytes {
        return invalid("route context query exceeds its request allowance");
    }
    if seeds.len() > backend.limits.maximum_context_seeds as usize
        || !strictly_sorted_by(seeds, |seed| seed)
        || !seeds
            .iter()
            .all(|seed| allowance.eligible_seeds.binary_search(seed).is_ok())
    {
        return invalid("route context seeds are unsorted, duplicated, or ineligible");
    }
    budget.validate()?;
    if !budget.fits_within(&allowance.budget) {
        return invalid("route context decision exceeds its resource allowance");
    }
    AssembleContext {
        scope: allowance.scope.clone(),
        query: query.into(),
        valid_at: allowance.valid_at,
        seeds: seeds.to_vec(),
        max_graph_depth: budget.max_graph_depth,
        max_items: budget.max_items,
        max_output_bytes: budget.max_output_bytes,
        max_storage_keys: budget.max_storage_keys,
    }
    .validate()
}

pub(super) fn validate_branch_candidate(edge: &ReasoningEdge, node_id: &CanonicalId) -> Result<()> {
    if &edge.from != node_id || edge.to == edge.from {
        return invalid("route branch candidate is not an outbound reasoning edge");
    }
    if edge.conditions.len() > MAX_REASONING_EDGE_CONDITIONS {
        return invalid("route branch candidate exceeds its condition bound");
    }
    if !strictly_sorted_by(&edge.conditions, |condition| &condition.id) {
        return invalid("route branch conditions must be unique and sorted by id");
    }
    for condition in &edge.conditions {
        condition.validate()?;
    }
    Ok(())
}

pub(super) fn validate_candidate_presence(allowed: bool, present: bool, name: &str) -> Result<()> {
    if allowed != present {
        return invalid(format!(
            "route {name} candidates must be present exactly when the decision is allowed"
        ));
    }
    Ok(())
}

pub(super) fn validate_parameters(
    parameters: &BTreeMap<CanonicalId, RouteParameterValue>,
) -> Result<()> {
    let mut items = 0_usize;
    for value in parameters.values() {
        validate_parameter_value(value, 1, &mut items)?;
    }
    Ok(())
}

pub(super) fn validate_parameter_value(
    value: &RouteParameterValue,
    depth: usize,
    items: &mut usize,
) -> Result<()> {
    if depth > MAX_ROUTE_PARAMETER_DEPTH {
        return invalid("route recipe parameters exceed their nesting-depth limit");
    }
    *items = items
        .checked_add(1)
        .ok_or_else(|| crate::ContractError("route parameter item count overflowed".into()))?;
    if *items > MAX_ROUTE_PARAMETER_ITEMS {
        return invalid("route recipe parameters exceed their item limit");
    }
    match value {
        RouteParameterValue::Decimal(value) | RouteParameterValue::String(value) => {
            validate_bounded_text(value, "route recipe parameter", MAX_ROUTE_SIGNAL_BYTES)
        }
        RouteParameterValue::Digest(value) => {
            validate_sha256(value, "route recipe parameter digest")
        }
        RouteParameterValue::List(values) => {
            for value in values {
                validate_parameter_value(value, depth + 1, items)?;
            }
            Ok(())
        }
        RouteParameterValue::Map(values) => {
            for value in values.values() {
                validate_parameter_value(value, depth + 1, items)?;
            }
            Ok(())
        }
        RouteParameterValue::Null
        | RouteParameterValue::Bool(_)
        | RouteParameterValue::Integer(_)
        | RouteParameterValue::Unsigned(_) => Ok(()),
    }
}

pub(super) fn validate_scope(scope: &str) -> Result<()> {
    if scope.is_empty() || scope.len() > 256 || scope.as_bytes().contains(&0) {
        return invalid("route context scope must be a non-empty bounded string without NUL");
    }
    Ok(())
}

pub(super) fn validate_contract_version(version: u16) -> Result<()> {
    if version != ROUTER_CONTRACT_VERSION {
        return invalid(format!(
            "unsupported router contract version {version}; expected {ROUTER_CONTRACT_VERSION}"
        ));
    }
    Ok(())
}

pub(super) fn validate_limit(value: u64, maximum: u64, field: &str) -> Result<()> {
    if value == 0 || value > maximum {
        return invalid(format!("{field} must be in 1..={maximum}"));
    }
    Ok(())
}

pub(super) fn validate_bounded_text(value: &str, field: &str, maximum: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > maximum || value.as_bytes().contains(&0) {
        return invalid(format!(
            "{field} must be non-empty, contain no NUL, and use at most {maximum} bytes"
        ));
    }
    Ok(())
}

pub(super) fn validate_encoded_size<T: Serialize>(
    value: &T,
    maximum: u64,
    field: &str,
) -> Result<()> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| crate::ContractError(format!("{field} encoding failed: {error}")))?;
    let length = u64::try_from(encoded.len())
        .map_err(|_| crate::ContractError(format!("{field} byte length exceeds u64")))?;
    if length > maximum {
        return invalid(format!("{field} exceeds its encoded-byte limit"));
    }
    Ok(())
}

pub(super) fn strictly_sorted_by<T, K: Ord + ?Sized>(values: &[T], key: impl Fn(&T) -> &K) -> bool {
    values.windows(2).all(|pair| key(&pair[0]) < key(&pair[1]))
}

pub(super) fn digest_json<T: Serialize>(domain: &[u8], value: &T) -> Result<String> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        crate::ContractError(format!("contract digest encoding failed: {error}"))
    })?;
    let mut bytes = Vec::with_capacity(domain.len() + 1 + encoded.len());
    bytes.extend_from_slice(domain);
    bytes.push(0);
    bytes.extend_from_slice(&encoded);
    Ok(sha256_bytes(&bytes))
}
