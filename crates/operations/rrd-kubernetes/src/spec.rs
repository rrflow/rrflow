//! The RrdInstance custom resource: spec, storage, phase and status.

use super::*;

#[derive(CustomResource, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "rrflow.io",
    version = "v1alpha1",
    kind = "RrdInstance",
    plural = "rrdinstances",
    namespaced,
    status = "RrdInstanceStatus",
    shortname = "rrd"
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RrdInstanceSpec {
    pub contract_version: u16,
    /// Immutable image reference. Tags are rejected; a sha256 digest is required.
    pub image: String,
    pub storage: RrdStorageSpec,
    pub tls_secret: String,
    /// ConfigMap containing the exact installation preview at `plan.json`.
    pub installation_plan_config_map: String,
    /// Explicit operator acceptance of the installation plan's canonical digest.
    pub installation_plan_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RrdStorageSpec {
    pub size: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_class_name: Option<String>,
    #[serde(default = "default_true")]
    pub retain_on_delete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum RrdInstancePhase {
    #[default]
    Pending,
    Reconciling,
    Ready,
    Degraded,
    Deleting,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RrdInstanceStatus {
    pub observed_generation: i64,
    pub phase: RrdInstancePhase,
    pub ready_replicas: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    pub applied_contract_sha256: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KubernetesResourceKind {
    HeadlessService,
    ClientService,
    StatefulSet,
    PodDisruptionBudget,
    NetworkPolicy,
}

impl KubernetesResourceKind {
    pub const fn api_kind(self) -> &'static str {
        match self {
            Self::HeadlessService | Self::ClientService => "Service",
            Self::StatefulSet => "StatefulSet",
            Self::PodDisruptionBudget => "PodDisruptionBudget",
            Self::NetworkPolicy => "NetworkPolicy",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesiredResource {
    pub kind: KubernetesResourceKind,
    pub name: String,
    pub body: Value,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredInstanceInput<'a> {
    pub namespace: &'a str,
    pub name: &'a str,
    pub uid: &'a str,
    pub generation: i64,
    pub spec: &'a RrdInstanceSpec,
}

impl RrdInstanceSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.contract_version != RRD_KUBERNETES_CONTRACT_VERSION {
            return Err("unsupported RRD Kubernetes contract version".into());
        }
        validate_image(&self.image)?;
        validate_dns_name(&self.tls_secret, "tls_secret")?;
        validate_dns_name(
            &self.installation_plan_config_map,
            "installation_plan_config_map",
        )?;
        validate_sha256(&self.installation_plan_sha256, "installation_plan_sha256")?;
        self.storage.validate()
    }
}

impl RrdStorageSpec {
    fn validate(&self) -> Result<(), String> {
        if !self.retain_on_delete {
            return Err("alpha Kubernetes storage must retain PVCs on deletion".into());
        }
        validate_quantity(&self.size)?;
        if let Some(class) = &self.storage_class_name {
            validate_dns_name(class, "storage_class_name")?;
        }
        Ok(())
    }
}
