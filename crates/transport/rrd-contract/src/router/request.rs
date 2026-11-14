//! Route step requests, signals, budgets and backend limits.

use super::*;

pub const ROUTER_CONTRACT_VERSION: u16 = 1;
pub const MAX_ROUTE_REQUEST_BYTES: u64 = 512 * 1024;
pub const MAX_ROUTE_RESPONSE_BYTES: u64 = 128 * 1024;
pub const MAX_ROUTE_INTENT_BYTES: u64 = 32 * 1024;
pub const MAX_ROUTE_SIGNAL_BYTES: usize = 4 * 1024;
pub const MAX_ROUTE_SIGNALS: u32 = 256;
pub const MAX_ROUTE_RECIPE_CANDIDATES: u32 = 256;
pub const MAX_ROUTE_BRANCH_CANDIDATES: u32 = 256;
pub const MAX_ROUTE_PARAMETERS: u32 = 128;
pub const MAX_ROUTE_PARAMETER_DEPTH: usize = 16;
pub const MAX_ROUTE_PARAMETER_ITEMS: usize = 1_024;
pub const MAX_ROUTE_EXECUTION_MS: u64 = 60_000;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RouteDecisionKind {
    SelectRecipe,
    AdvanceBranch,
    RequestContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouterBackendLimits {
    pub maximum_request_bytes: u64,
    pub maximum_response_bytes: u64,
    pub maximum_intent_bytes: u64,
    pub maximum_signals: u32,
    pub maximum_recipe_candidates: u32,
    pub maximum_branch_candidates: u32,
    pub maximum_context_seeds: u32,
    pub maximum_parameters: u32,
    pub maximum_execution_ms: u64,
}

impl RouterBackendLimits {
    pub fn validate(&self) -> Result<()> {
        validate_limit(
            self.maximum_request_bytes,
            MAX_ROUTE_REQUEST_BYTES,
            "router maximum_request_bytes",
        )?;
        validate_limit(
            self.maximum_response_bytes,
            MAX_ROUTE_RESPONSE_BYTES,
            "router maximum_response_bytes",
        )?;
        validate_limit(
            self.maximum_intent_bytes,
            MAX_ROUTE_INTENT_BYTES,
            "router maximum_intent_bytes",
        )?;
        validate_limit(
            u64::from(self.maximum_signals),
            u64::from(MAX_ROUTE_SIGNALS),
            "router maximum_signals",
        )?;
        validate_limit(
            u64::from(self.maximum_recipe_candidates),
            u64::from(MAX_ROUTE_RECIPE_CANDIDATES),
            "router maximum_recipe_candidates",
        )?;
        validate_limit(
            u64::from(self.maximum_branch_candidates),
            u64::from(MAX_ROUTE_BRANCH_CANDIDATES),
            "router maximum_branch_candidates",
        )?;
        validate_limit(
            u64::from(self.maximum_context_seeds),
            MAX_CONTEXT_SEEDS as u64,
            "router maximum_context_seeds",
        )?;
        validate_limit(
            u64::from(self.maximum_parameters),
            u64::from(MAX_ROUTE_PARAMETERS),
            "router maximum_parameters",
        )?;
        validate_limit(
            self.maximum_execution_ms,
            MAX_ROUTE_EXECUTION_MS,
            "router maximum_execution_ms",
        )?;
        if self.maximum_intent_bytes > self.maximum_request_bytes {
            return invalid("router intent byte limit exceeds its request byte limit");
        }
        Ok(())
    }
}

/// Capabilities and hard dispatch limits for one replaceable router backend.
/// The descriptor binds one exact model manifest; B-03 validates that
/// manifest's artifacts and runtime separately before any backend is loaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouterBackendDescriptor {
    pub contract_version: u16,
    pub id: CanonicalId,
    pub revision: u64,
    pub model_manifest_id: CanonicalId,
    pub model_manifest_revision: u64,
    pub model_manifest_sha256: String,
    pub decisions: BTreeSet<RouteDecisionKind>,
    pub limits: RouterBackendLimits,
    pub descriptor_sha256: String,
}

impl RouterBackendDescriptor {
    pub fn validate(&self) -> Result<()> {
        validate_contract_version(self.contract_version)?;
        if self.revision == 0 {
            return invalid("router backend revision must be greater than zero");
        }
        if self.model_manifest_revision == 0 {
            return invalid("router backend model manifest revision must be greater than zero");
        }
        validate_sha256(
            &self.model_manifest_sha256,
            "router backend model_manifest_sha256",
        )?;
        if self.decisions.is_empty() {
            return invalid("router backend must support at least one routing decision");
        }
        self.limits.validate()?;
        validate_sha256(&self.descriptor_sha256, "router backend descriptor_sha256")?;
        if self.descriptor_sha256 != router_backend_descriptor_sha256(self)? {
            return invalid("router backend descriptor digest does not match its content");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RouteSignalValue {
    Null,
    Bool(bool),
    Integer(i64),
    Unsigned(u64),
    Decimal(String),
    String(String),
    Digest(String),
}

/// Strict JSON-like recipe parameters. This is separate from rrflowQL values:
/// a routing model cannot smuggle query or storage semantics into a recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RouteParameterValue {
    Null,
    Bool(bool),
    Integer(i64),
    Unsigned(u64),
    Decimal(String),
    String(String),
    Digest(String),
    List(Vec<RouteParameterValue>),
    Map(BTreeMap<CanonicalId, RouteParameterValue>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouteSignal {
    pub id: CanonicalId,
    pub value: RouteSignalValue,
    pub evidence_sha256: String,
}

impl RouteSignal {
    fn validate(&self) -> Result<()> {
        match &self.value {
            RouteSignalValue::Decimal(value) => {
                validate_bounded_text(value, "route decimal signal", MAX_ROUTE_SIGNAL_BYTES)?;
            }
            RouteSignalValue::String(value) => {
                validate_bounded_text(value, "route string signal", MAX_ROUTE_SIGNAL_BYTES)?;
            }
            RouteSignalValue::Digest(value) => {
                validate_sha256(value, "route digest signal")?;
            }
            RouteSignalValue::Null
            | RouteSignalValue::Bool(_)
            | RouteSignalValue::Integer(_)
            | RouteSignalValue::Unsigned(_) => {}
        }
        validate_sha256(&self.evidence_sha256, "route signal evidence_sha256")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouteContextBudget {
    pub max_graph_depth: u8,
    pub max_items: u64,
    pub max_output_bytes: u64,
    pub max_storage_keys: u64,
}

impl RouteContextBudget {
    pub fn validate(&self) -> Result<()> {
        if self.max_graph_depth > MAX_CONTEXT_GRAPH_DEPTH {
            return invalid("route context graph depth exceeds its hard bound");
        }
        validate_limit(self.max_items, MAX_CONTEXT_ITEMS, "route context max_items")?;
        validate_limit(
            self.max_output_bytes,
            MAX_CONTEXT_OUTPUT_BYTES,
            "route context max_output_bytes",
        )?;
        validate_limit(
            self.max_storage_keys,
            MAX_CONTEXT_STORAGE_KEYS,
            "route context max_storage_keys",
        )
    }

    pub(super) fn fits_within(&self, allowance: &Self) -> bool {
        self.max_graph_depth <= allowance.max_graph_depth
            && self.max_items <= allowance.max_items
            && self.max_output_bytes <= allowance.max_output_bytes
            && self.max_storage_keys <= allowance.max_storage_keys
    }
}

/// The only context coordinates a router may influence. Scope, valid time,
/// authorization, and physical access-path selection remain engine inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouteContextAllowance {
    pub scope: String,
    pub valid_at: u64,
    pub maximum_query_bytes: u64,
    #[serde(default)]
    pub eligible_seeds: Vec<DataReference>,
    pub budget: RouteContextBudget,
}

impl RouteContextAllowance {
    fn validate(&self, limits: &RouterBackendLimits) -> Result<()> {
        validate_scope(&self.scope)?;
        if self.valid_at == 0 {
            return invalid("route context valid_at must be greater than zero");
        }
        validate_limit(
            self.maximum_query_bytes,
            MAX_CONTEXT_QUERY_BYTES as u64,
            "route context maximum_query_bytes",
        )?;
        if self.eligible_seeds.len() > limits.maximum_context_seeds as usize {
            return invalid("route context eligible seeds exceed the backend limit");
        }
        if !strictly_sorted_by(&self.eligible_seeds, |seed| seed) {
            return invalid("route context eligible seeds must be unique and sorted");
        }
        self.budget.validate()
    }
}

/// One bounded, stamped routing input. It projects the existing reasoning
/// cursor and candidate contracts; it does not create another tree schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RouteStepRequest {
    pub contract_version: u16,
    pub id: CorrelationId,
    pub backend_id: CanonicalId,
    pub backend_revision: u64,
    pub backend_descriptor_sha256: String,
    pub cursor: ReasoningActiveCursor,
    pub intent: String,
    #[serde(default)]
    pub signals: Vec<RouteSignal>,
    #[serde(default)]
    pub recipe_candidates: Vec<ReasoningRecipe>,
    #[serde(default)]
    pub branch_candidates: Vec<ReasoningEdge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<RouteContextAllowance>,
    pub allowed_decisions: BTreeSet<RouteDecisionKind>,
    pub requested_at_unix_ms: u64,
    pub deadline_unix_ms: u64,
    pub request_sha256: String,
}

impl RouteStepRequest {
    pub fn validate_for(&self, backend: &RouterBackendDescriptor) -> Result<()> {
        backend.validate()?;
        validate_contract_version(self.contract_version)?;
        if self.backend_id != backend.id
            || self.backend_revision != backend.revision
            || self.backend_descriptor_sha256 != backend.descriptor_sha256
        {
            return invalid("route request does not bind its backend descriptor");
        }
        validate_sha256(
            &self.backend_descriptor_sha256,
            "route request backend_descriptor_sha256",
        )?;
        self.cursor.validate()?;
        validate_bounded_text(
            &self.intent,
            "route intent",
            backend.limits.maximum_intent_bytes as usize,
        )?;
        if self.signals.len() > backend.limits.maximum_signals as usize {
            return invalid("route signals exceed the backend limit");
        }
        if !strictly_sorted_by(&self.signals, |signal| &signal.id) {
            return invalid("route signals must be unique and sorted by id");
        }
        for signal in &self.signals {
            signal.validate()?;
        }
        if self.recipe_candidates.len() > backend.limits.maximum_recipe_candidates as usize {
            return invalid("route recipe candidates exceed the backend limit");
        }
        if !strictly_sorted_by(&self.recipe_candidates, |recipe| &recipe.id) {
            return invalid("route recipe candidates must be unique and sorted by id");
        }
        for recipe in &self.recipe_candidates {
            recipe.validate()?;
        }
        if self.branch_candidates.len() > backend.limits.maximum_branch_candidates as usize {
            return invalid("route branch candidates exceed the backend limit");
        }
        if !strictly_sorted_by(&self.branch_candidates, |edge| &edge.id) {
            return invalid("route branch candidates must be unique and sorted by id");
        }
        for edge in &self.branch_candidates {
            validate_branch_candidate(edge, &self.cursor.node_id)?;
        }
        if self.allowed_decisions.is_empty()
            || !self.allowed_decisions.is_subset(&backend.decisions)
        {
            return invalid("route request allowed decisions are empty or unsupported");
        }
        validate_candidate_presence(
            self.allowed_decisions
                .contains(&RouteDecisionKind::SelectRecipe),
            !self.recipe_candidates.is_empty(),
            "select_recipe",
        )?;
        validate_candidate_presence(
            self.allowed_decisions
                .contains(&RouteDecisionKind::AdvanceBranch),
            !self.branch_candidates.is_empty(),
            "advance_branch",
        )?;
        validate_candidate_presence(
            self.allowed_decisions
                .contains(&RouteDecisionKind::RequestContext),
            self.context.is_some(),
            "request_context",
        )?;
        if let Some(context) = &self.context {
            context.validate(&backend.limits)?;
            if context.scope != self.cursor.read.scope {
                return invalid("route context allowance differs from the cursor read scope");
            }
        }
        if self.requested_at_unix_ms == 0
            || self.deadline_unix_ms <= self.requested_at_unix_ms
            || self.deadline_unix_ms - self.requested_at_unix_ms
                > backend.limits.maximum_execution_ms
        {
            return invalid("route request deadline exceeds the backend execution limit");
        }
        validate_sha256(&self.request_sha256, "route request_sha256")?;
        if self.request_sha256 != route_step_request_sha256(self)? {
            return invalid("route request digest does not match its content");
        }
        validate_encoded_size(self, backend.limits.maximum_request_bytes, "route request")
    }
}

/// Exactly one grammar-constrained proposal from a router backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "decision", rename_all = "snake_case", deny_unknown_fields)]
pub enum RouteStepDecision {
    SelectRecipe {
        contract_version: u16,
        request_id: CorrelationId,
        request_sha256: String,
        recipe_id: CanonicalId,
        recipe_revision: u64,
        #[serde(default)]
        parameters: BTreeMap<CanonicalId, RouteParameterValue>,
        decided_at_unix_ms: u64,
        decision_sha256: String,
    },
    AdvanceBranch {
        contract_version: u16,
        request_id: CorrelationId,
        request_sha256: String,
        edge_id: CanonicalId,
        decided_at_unix_ms: u64,
        decision_sha256: String,
    },
    RequestContext {
        contract_version: u16,
        request_id: CorrelationId,
        request_sha256: String,
        query: String,
        #[serde(default)]
        seeds: Vec<DataReference>,
        budget: RouteContextBudget,
        decided_at_unix_ms: u64,
        decision_sha256: String,
    },
}
