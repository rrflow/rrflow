//! Endpoint binding, profile, and hardware inventory contracts.
//!
//! Every model, store, and service that a class of work reaches is a
//! configured endpoint binding. Which machine, provider, or model serves a
//! class is configuration, never code, so a local, self-hosted, subscription,
//! or API binding changes no behavior. A binding names its endpoint and a
//! credential *reference*; the credential value never enters this contract, a
//! store, a client, or a log. A profile is a named, revisioned set of bindings
//! with ordered fallbacks per class, and a hardware inventory declares the
//! capacity a profile's placements must fit. These values govern durable
//! configuration; they do not describe or constrain a model's private
//! reasoning.

use crate::{invalid, sha256_bytes, validate_sha256, CanonicalId, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const ENDPOINT_PROFILE_CONTRACT_VERSION: u16 = 1;
pub const HARDWARE_INVENTORY_CONTRACT_VERSION: u16 = 1;
pub const MAX_PROFILE_BINDINGS: usize = 256;
pub const MAX_BINDINGS_PER_CLASS: usize = 16;
pub const MAX_INVENTORY_MACHINES: usize = 1_024;
pub const MAX_ENDPOINT_URL_BYTES: usize = 2_048;
pub const MAX_CREDENTIAL_NAME_BYTES: usize = 256;
pub const MAX_BINDING_CONCURRENCY: u32 = 65_536;
pub const MAX_BINDING_TIMEOUT_MS: u64 = 3_600_000;
pub const MAX_BINDING_INPUT_BYTES: u64 = 64 * 1024 * 1024 * 1024;
pub const MAX_DECLARED_BYTES: u64 = 64 * 1024 * 1024 * 1024 * 1024;

/// The eight workload classes. Code depends on a class contract; a profile
/// decides which endpoint serves it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadClass {
    /// Class 1: the large reasoning model, bandwidth-bound generation.
    LargeModelServing,
    /// Class 2: small models that must answer in milliseconds and stay loaded.
    AlwaysWarmSmallModels,
    /// Class 3: embedding and reranking models.
    EmbeddingAndReranking,
    /// Class 4: the vector store, a derived index rebuildable from canonical records.
    VectorStore,
    /// Class 5: the canonical store, the project brain. rrflowKV is the chosen binding.
    CanonicalStore,
    /// Class 6: learning by weight updates (fine-tuning, distillation).
    WeightUpdates,
    /// Class 7: learning by behaviour updates (traces, signals, experiments).
    BehaviorUpdates,
    /// Class 8: control plane and operations.
    ControlPlane,
}

/// How an endpoint is reached and paid for. The kind changes nothing about
/// the class contract.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum BindingKind {
    /// Same machine as the engine: in process, or a loopback endpoint.
    Local,
    /// A service on the operator's own estate, reached over the network or a mesh.
    SelfHosted,
    /// A vendor subscription reached through its endpoint.
    Subscription,
    /// Metered API access reached through its endpoint.
    Api,
}

/// Where a binding is reached. In-process execution is the narrow local
/// exception; everything else is a URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EndpointLocator {
    InProcess { component: CanonicalId },
    Url { url: String },
}

impl EndpointLocator {
    fn validate(&self, label: &str) -> Result<()> {
        match self {
            Self::InProcess { .. } => Ok(()),
            Self::Url { url } => validate_endpoint_url(url, label),
        }
    }
}

/// A reference to a secret held by a configured secret provider. The value is
/// resolved into process memory by that provider and never appears here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CredentialReference {
    pub provider: CanonicalId,
    pub name: String,
}

impl CredentialReference {
    fn validate(&self, label: &str) -> Result<()> {
        if self.name.is_empty() || self.name.len() > MAX_CREDENTIAL_NAME_BYTES {
            return invalid(format!(
                "{label} credential name length must be in 1..={MAX_CREDENTIAL_NAME_BYTES} bytes"
            ));
        }
        if self
            .name
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
        {
            return invalid(format!(
                "{label} credential name must not contain whitespace or control characters"
            ));
        }
        Ok(())
    }
}

/// GPU memory a binding declares it needs on its placement machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GpuRequirement {
    None,
    Required { memory_bytes: u64 },
}

/// Memory a binding declares it consumes on its placement machine. Declared
/// values are replaced by measured ones once observation exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeclaredFootprint {
    pub memory_bytes: u64,
    pub gpu: GpuRequirement,
}

impl DeclaredFootprint {
    fn validate(&self, label: &str) -> Result<()> {
        validate_nonzero_max(
            &format!("{label} footprint memory_bytes"),
            self.memory_bytes,
            MAX_DECLARED_BYTES,
        )?;
        if let GpuRequirement::Required { memory_bytes } = &self.gpu {
            validate_nonzero_max(
                &format!("{label} footprint gpu memory_bytes"),
                *memory_bytes,
                MAX_DECLARED_BYTES,
            )?;
        }
        Ok(())
    }

    fn gpu_memory_bytes(&self) -> u64 {
        match self.gpu {
            GpuRequirement::None => 0,
            GpuRequirement::Required { memory_bytes } => memory_bytes,
        }
    }
}

/// Generic admission limits for one binding. Class-specific limits stay with
/// their class contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BindingLimits {
    pub max_concurrency: u32,
    pub timeout_ms: u64,
    pub max_input_bytes: u64,
}

impl BindingLimits {
    fn validate(&self, label: &str) -> Result<()> {
        validate_nonzero_max(
            &format!("{label} max_concurrency"),
            u64::from(self.max_concurrency),
            u64::from(MAX_BINDING_CONCURRENCY),
        )?;
        validate_nonzero_max(
            &format!("{label} timeout_ms"),
            self.timeout_ms,
            MAX_BINDING_TIMEOUT_MS,
        )?;
        validate_nonzero_max(
            &format!("{label} max_input_bytes"),
            self.max_input_bytes,
            MAX_BINDING_INPUT_BYTES,
        )
    }
}

/// One configured endpoint serving one workload class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointBinding {
    pub id: CanonicalId,
    pub class: WorkloadClass,
    pub kind: BindingKind,
    pub endpoint: EndpointLocator,
    /// The model or service this binding reaches: a router model manifest id,
    /// an embedding backend id, a store, or a vendor model name as a canonical id.
    pub service_id: CanonicalId,
    /// Optional pin to the exact manifest or artifact the service must present.
    pub manifest_sha256: Option<String>,
    /// Required for subscription and API bindings. Never a value.
    pub credential: Option<CredentialReference>,
    /// The inventory machine this binding runs on. Required for local and
    /// self-hosted bindings, absent for subscription and API bindings.
    pub placement: Option<CanonicalId>,
    pub footprint: DeclaredFootprint,
    pub limits: BindingLimits,
}

impl EndpointBinding {
    pub fn validate(&self) -> Result<()> {
        let label = format!("binding {}", self.id);
        self.endpoint.validate(&label)?;
        if let Some(manifest_sha256) = &self.manifest_sha256 {
            validate_sha256(manifest_sha256, &format!("{label} manifest_sha256"))?;
        }
        if let Some(credential) = &self.credential {
            credential.validate(&label)?;
        }
        self.footprint.validate(&label)?;
        self.limits.validate(&label)?;
        match self.kind {
            BindingKind::Local => {
                if self.placement.is_none() {
                    return invalid(format!(
                        "{label}: a local binding names its placement machine"
                    ));
                }
            }
            BindingKind::SelfHosted => {
                if self.placement.is_none() {
                    return invalid(format!(
                        "{label}: a self-hosted binding names its placement machine"
                    ));
                }
                if matches!(self.endpoint, EndpointLocator::InProcess { .. }) {
                    return invalid(format!(
                        "{label}: a self-hosted binding is reached through a url endpoint"
                    ));
                }
            }
            BindingKind::Subscription | BindingKind::Api => {
                if self.credential.is_none() {
                    return invalid(format!(
                        "{label}: subscription and api bindings carry a credential reference"
                    ));
                }
                if matches!(self.endpoint, EndpointLocator::InProcess { .. }) {
                    return invalid(format!(
                        "{label}: subscription and api bindings are reached through a url endpoint"
                    ));
                }
                if self.placement.is_some() {
                    return invalid(format!(
                        "{label}: subscription and api bindings have no placement machine"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Operator-authored profile input. Revision and digest are assigned by the
/// engine, so neither can be forged or silently retained across an edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointProfileInput {
    pub contract_version: u16,
    pub name: CanonicalId,
    /// Bindings in declaration order. Within one class, the order is the
    /// fallback order: the first binding is primary.
    pub bindings: Vec<EndpointBinding>,
}

impl EndpointProfileInput {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != ENDPOINT_PROFILE_CONTRACT_VERSION {
            return invalid(format!(
                "endpoint profile contract version must be {ENDPOINT_PROFILE_CONTRACT_VERSION}"
            ));
        }
        if self.bindings.is_empty() || self.bindings.len() > MAX_PROFILE_BINDINGS {
            return invalid(format!(
                "endpoint profile binding count must be in 1..={MAX_PROFILE_BINDINGS}"
            ));
        }
        let mut ids = BTreeSet::new();
        let mut per_class: BTreeMap<WorkloadClass, usize> = BTreeMap::new();
        for binding in &self.bindings {
            binding.validate()?;
            if !ids.insert(binding.id.as_str()) {
                return invalid(format!(
                    "endpoint profile binding id {} repeats",
                    binding.id
                ));
            }
            let count = per_class.entry(binding.class).or_insert(0);
            *count += 1;
            if *count > MAX_BINDINGS_PER_CLASS {
                return invalid(format!(
                    "endpoint profile declares more than {MAX_BINDINGS_PER_CLASS} bindings for one class"
                ));
            }
        }
        Ok(())
    }
}

/// Effective, revisioned profile. Mutating it is a governed configuration
/// change that produces the next revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointProfile {
    pub contract_version: u16,
    pub name: CanonicalId,
    pub revision: u64,
    pub bindings: Vec<EndpointBinding>,
    pub profile_sha256: String,
}

impl EndpointProfile {
    pub fn from_input(revision: u64, input: EndpointProfileInput) -> Result<Self> {
        input.validate()?;
        if revision == 0 {
            return invalid("endpoint profile revision must be greater than zero");
        }
        let mut profile = Self {
            contract_version: input.contract_version,
            name: input.name,
            revision,
            bindings: input.bindings,
            profile_sha256: String::new(),
        };
        profile.profile_sha256 = endpoint_profile_sha256(&profile)?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<()> {
        EndpointProfileInput {
            contract_version: self.contract_version,
            name: self.name.clone(),
            bindings: self.bindings.clone(),
        }
        .validate()?;
        if self.revision == 0 {
            return invalid("endpoint profile revision must be greater than zero");
        }
        if self.profile_sha256 != endpoint_profile_sha256(self)? {
            return invalid("endpoint profile digest does not match its content");
        }
        Ok(())
    }

    /// Bindings for one class in fallback order.
    pub fn fallbacks(&self, class: WorkloadClass) -> impl Iterator<Item = &EndpointBinding> {
        self.bindings
            .iter()
            .filter(move |binding| binding.class == class)
    }

    /// The first binding for one class, if the profile serves it.
    pub fn primary(&self, class: WorkloadClass) -> Option<&EndpointBinding> {
        self.fallbacks(class).next()
    }

    /// Checks every placed binding against the inventory: the machine exists,
    /// the declared memory of all bindings placed on it fits, and a binding
    /// that requires a GPU finds one with enough declared memory. Returns the
    /// declared load per machine for display beside observed state.
    pub fn validate_placement(&self, inventory: &HardwareInventory) -> Result<Vec<MachineLoad>> {
        self.validate()?;
        inventory.validate()?;
        let mut loads: BTreeMap<&str, MachineLoad> = inventory
            .machines
            .iter()
            .map(|machine| {
                (
                    machine.id.as_str(),
                    MachineLoad {
                        machine_id: machine.id.clone(),
                        memory_declared_bytes: 0,
                        memory_capacity_bytes: machine.memory_bytes,
                        gpu_declared_bytes: 0,
                        gpu_capacity_bytes: machine.gpu.as_ref().map(|gpu| gpu.memory_bytes),
                    },
                )
            })
            .collect();
        for binding in &self.bindings {
            let Some(placement) = &binding.placement else {
                continue;
            };
            let Some(load) = loads.get_mut(placement.as_str()) else {
                return invalid(format!(
                    "binding {} is placed on machine {placement}, which the inventory does not declare",
                    binding.id
                ));
            };
            load.memory_declared_bytes = load
                .memory_declared_bytes
                .checked_add(binding.footprint.memory_bytes)
                .ok_or_else(|| crate::ContractError("declared memory overflowed".into()))?;
            if load.memory_declared_bytes > load.memory_capacity_bytes {
                return invalid(format!(
                    "machine {placement} declares {} memory bytes but its bindings declare {} after binding {}",
                    load.memory_capacity_bytes, load.memory_declared_bytes, binding.id
                ));
            }
            let gpu_bytes = binding.footprint.gpu_memory_bytes();
            if gpu_bytes > 0 {
                let Some(gpu_capacity) = load.gpu_capacity_bytes else {
                    return invalid(format!(
                        "binding {} requires a GPU but machine {placement} declares none",
                        binding.id
                    ));
                };
                load.gpu_declared_bytes = load
                    .gpu_declared_bytes
                    .checked_add(gpu_bytes)
                    .ok_or_else(|| crate::ContractError("declared GPU memory overflowed".into()))?;
                if load.gpu_declared_bytes > gpu_capacity {
                    return invalid(format!(
                        "machine {placement} declares {gpu_capacity} GPU memory bytes but its bindings declare {} after binding {}",
                        load.gpu_declared_bytes, binding.id
                    ));
                }
            }
        }
        Ok(loads.into_values().collect())
    }
}

pub fn endpoint_profile_sha256(profile: &EndpointProfile) -> Result<String> {
    let encoded = serde_json::to_vec(&(
        profile.contract_version,
        &profile.name,
        profile.revision,
        &profile.bindings,
    ))
    .map_err(|error| {
        crate::ContractError(format!("endpoint profile digest encoding failed: {error}"))
    })?;
    let mut bytes = Vec::with_capacity(32 + encoded.len());
    bytes.extend_from_slice(b"rrflow-endpoint-profile-v1\0");
    bytes.extend_from_slice(&encoded);
    Ok(sha256_bytes(&bytes))
}

/// Declared GPU capacity of one machine. On unified-memory hardware this is
/// the shared pool, declared once here and once as machine memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GpuCapacity {
    pub memory_bytes: u64,
}

/// Declared capacity of one machine a profile may place bindings on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineCapacity {
    pub id: CanonicalId,
    pub memory_bytes: u64,
    pub gpu: Option<GpuCapacity>,
    pub disk_bytes: u64,
}

impl MachineCapacity {
    fn validate(&self) -> Result<()> {
        let label = format!("machine {}", self.id);
        validate_nonzero_max(
            &format!("{label} memory_bytes"),
            self.memory_bytes,
            MAX_DECLARED_BYTES,
        )?;
        if let Some(gpu) = &self.gpu {
            validate_nonzero_max(
                &format!("{label} gpu memory_bytes"),
                gpu.memory_bytes,
                MAX_DECLARED_BYTES,
            )?;
        }
        validate_nonzero_max(
            &format!("{label} disk_bytes"),
            self.disk_bytes,
            MAX_DECLARED_BYTES,
        )
    }
}

/// Declared capacity per machine. Observed capacity replaces it once
/// measurement exists; a profile is validated against the declared values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HardwareInventory {
    pub contract_version: u16,
    pub machines: Vec<MachineCapacity>,
}

impl HardwareInventory {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != HARDWARE_INVENTORY_CONTRACT_VERSION {
            return invalid(format!(
                "hardware inventory contract version must be {HARDWARE_INVENTORY_CONTRACT_VERSION}"
            ));
        }
        if self.machines.is_empty() || self.machines.len() > MAX_INVENTORY_MACHINES {
            return invalid(format!(
                "hardware inventory machine count must be in 1..={MAX_INVENTORY_MACHINES}"
            ));
        }
        let mut ids = BTreeSet::new();
        for machine in &self.machines {
            machine.validate()?;
            if !ids.insert(machine.id.as_str()) {
                return invalid(format!(
                    "hardware inventory machine id {} repeats",
                    machine.id
                ));
            }
        }
        Ok(())
    }
}

/// Declared load of one machine under a profile, for display beside observed
/// state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineLoad {
    pub machine_id: CanonicalId,
    pub memory_declared_bytes: u64,
    pub memory_capacity_bytes: u64,
    pub gpu_declared_bytes: u64,
    pub gpu_capacity_bytes: Option<u64>,
}

fn validate_endpoint_url(url: &str, label: &str) -> Result<()> {
    if url.is_empty() || url.len() > MAX_ENDPOINT_URL_BYTES {
        return invalid(format!(
            "{label} endpoint url length must be in 1..={MAX_ENDPOINT_URL_BYTES} bytes"
        ));
    }
    if url
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return invalid(format!(
            "{label} endpoint url must not contain whitespace or control characters"
        ));
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return invalid(format!("{label} endpoint url must carry a scheme"));
    };
    if scheme.is_empty()
        || !scheme
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'+')
    {
        return invalid(format!(
            "{label} endpoint url scheme must be lowercase ASCII letters, digits, or '+'"
        ));
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() {
        return invalid(format!("{label} endpoint url must name a host"));
    }
    if authority.contains('@') {
        return invalid(format!(
            "{label} endpoint url must not carry user information; use a credential reference"
        ));
    }
    Ok(())
}

fn validate_nonzero_max(label: &str, value: u64, maximum: u64) -> Result<()> {
    if value == 0 || value > maximum {
        return invalid(format!("{label} must be in 1..={maximum}"));
    }
    Ok(())
}
