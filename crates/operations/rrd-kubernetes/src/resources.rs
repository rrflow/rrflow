//! Desired Kubernetes resources, the CRD document, labels and names.

use super::*;

pub fn desired_resources(input: &DesiredInstanceInput<'_>) -> Result<Vec<DesiredResource>, String> {
    input.spec.validate()?;
    validate_dns_name(input.namespace, "namespace")?;
    validate_dns_name(input.name, "metadata.name")?;
    if input.uid.is_empty() || input.generation <= 0 {
        return Err("metadata UID and positive generation are required".into());
    }
    let labels = labels(input.name);
    let owner_references = vec![json!({
        "apiVersion": RRD_INSTANCE_API_VERSION,
        "kind": RRD_INSTANCE_KIND,
        "name": input.name,
        "uid": input.uid,
        "controller": true,
        "blockOwnerDeletion": true,
    })];
    let metadata = |name: &str| {
        json!({
            "name": name,
            "namespace": input.namespace,
            "labels": labels,
            "ownerReferences": owner_references,
        })
    };
    let peer_name = bounded_name(input.name, "peer")?;
    let data_claim_name = "rrd-data";
    let service_port = json!({
        "name": "https",
        "port": RRD_CLIENT_PORT,
        "protocol": "TCP",
        "targetPort": "https",
    });
    let headless = json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": metadata(&peer_name),
        "spec": {
            "clusterIP": "None",
            "publishNotReadyAddresses": true,
            "selector": labels,
            "ports": [service_port.clone()],
        },
    });
    let client = json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": metadata(input.name),
        "spec": {
            "type": "ClusterIP",
            "selector": labels,
            "ports": [service_port],
        },
    });

    let mut claim = json!({
        "metadata": {"name": data_claim_name},
        "spec": {
            "accessModes": ["ReadWriteOnce"],
            "resources": {"requests": {"storage": input.spec.storage.size}},
        },
    });
    if let Some(class) = &input.spec.storage.storage_class_name {
        claim["spec"]["storageClassName"] = Value::String(class.clone());
    }
    let container_security = json!({
        "allowPrivilegeEscalation": false,
        "capabilities": {"drop": ["ALL"]},
        "readOnlyRootFilesystem": true,
        "runAsNonRoot": true,
        "runAsUser": 65532,
        "runAsGroup": 65532,
    });
    let pod_security = json!({
        "runAsNonRoot": true,
        "runAsUser": 65532,
        "runAsGroup": 65532,
        "fsGroup": 65532,
        "fsGroupChangePolicy": "OnRootMismatch",
        "seccompProfile": {"type": "RuntimeDefault"},
    });
    let stateful_set = json!({
        "apiVersion": "apps/v1",
        "kind": "StatefulSet",
        "metadata": metadata(input.name),
        "spec": {
            "serviceName": peer_name,
            "replicas": 1,
            "podManagementPolicy": "OrderedReady",
            "persistentVolumeClaimRetentionPolicy": {
                "whenDeleted": "Retain",
                "whenScaled": "Retain",
            },
            "updateStrategy": {"type": "OnDelete"},
            "selector": {"matchLabels": labels},
            "template": {
                "metadata": {"labels": labels},
                "spec": {
                    "automountServiceAccountToken": false,
                    "terminationGracePeriodSeconds": 30,
                    "securityContext": pod_security,
                    "initContainers": [
                        {
                            "name": "rrflow-install",
                            "image": input.spec.image,
                            "imagePullPolicy": "IfNotPresent",
                            "command": ["rrflow"],
                            "args": [
                                "install", "apply",
                                "--project", "/var/lib/rrd/project",
                                "--mode", "fresh",
                                "--plan", "/etc/rrflow/install/plan.json",
                                "--expect", input.spec.installation_plan_sha256,
                            ],
                            "securityContext": container_security,
                            "volumeMounts": [
                                {"name": data_claim_name, "mountPath": "/var/lib/rrd/project"},
                                {"name": "installation-plan", "mountPath": "/etc/rrflow/install", "readOnly": true},
                            ],
                        }
                    ],
                    "containers": [{
                        "name": "rrd",
                        "image": input.spec.image,
                        "imagePullPolicy": "IfNotPresent",
                        "command": ["rrd-server"],
                        "args": [
                            "--project", "/var/lib/rrd/project",
                            "--distribution-executable", "/usr/local/bin/rrflow",
                            "--bind", format!("0.0.0.0:{RRD_CLIENT_PORT}"),
                            "--tls-cert", "/etc/rrd/tls/tls.crt",
                            "--tls-key", "/etc/rrd/tls/tls.key",
                            "--tls-client-ca", "/etc/rrd/tls/ca.crt",
                        ],
                        "ports": [{"name": "https", "containerPort": RRD_CLIENT_PORT, "protocol": "TCP"}],
                        "startupProbe": {"tcpSocket": {"port": "https"}, "failureThreshold": 30, "periodSeconds": 2},
                        "livenessProbe": {"tcpSocket": {"port": "https"}, "failureThreshold": 3, "periodSeconds": 10},
                        "readinessProbe": {"tcpSocket": {"port": "https"}, "failureThreshold": 2, "periodSeconds": 5},
                        "resources": {
                            "requests": {"cpu": "250m", "memory": "512Mi"},
                            "limits": {"cpu": "2", "memory": "4Gi"},
                        },
                        "securityContext": container_security,
                        "volumeMounts": [
                            {"name": data_claim_name, "mountPath": "/var/lib/rrd/project"},
                            {"name": "rrd-tls", "mountPath": "/etc/rrd/tls", "readOnly": true},
                        ],
                    }],
                    "volumes": [
                        {"name": "rrd-tls", "secret": {"secretName": input.spec.tls_secret, "defaultMode": 288}},
                        {"name": "installation-plan", "configMap": {"name": input.spec.installation_plan_config_map, "defaultMode": 292, "items": [{"key": "plan.json", "path": "plan.json"}]}},
                    ],
                },
            },
            "volumeClaimTemplates": [claim],
        },
    });
    let disruption_budget = json!({
        "apiVersion": "policy/v1",
        "kind": "PodDisruptionBudget",
        "metadata": metadata(input.name),
        "spec": {
            "maxUnavailable": 0,
            "unhealthyPodEvictionPolicy": "AlwaysAllow",
            "selector": {"matchLabels": labels},
        },
    });
    let network_policy = json!({
        "apiVersion": "networking.k8s.io/v1",
        "kind": "NetworkPolicy",
        "metadata": metadata(input.name),
        "spec": {
            "podSelector": {"matchLabels": labels},
            "policyTypes": ["Ingress", "Egress"],
            "ingress": [{
                "from": [{"namespaceSelector": {"matchLabels": {"rrflow.io/rrd-client": "true"}}}],
                "ports": [{"protocol": "TCP", "port": RRD_CLIENT_PORT}],
            }],
            "egress": [],
        },
    });

    Ok(vec![
        resource(KubernetesResourceKind::HeadlessService, peer_name, headless)?,
        resource(
            KubernetesResourceKind::ClientService,
            input.name.into(),
            client,
        )?,
        resource(
            KubernetesResourceKind::StatefulSet,
            input.name.into(),
            stateful_set,
        )?,
        resource(
            KubernetesResourceKind::PodDisruptionBudget,
            input.name.into(),
            disruption_budget,
        )?,
        resource(
            KubernetesResourceKind::NetworkPolicy,
            input.name.into(),
            network_policy,
        )?,
    ])
}

pub fn applied_contract_sha256(resources: &[DesiredResource]) -> String {
    let identities = resources
        .iter()
        .map(|resource| (&resource.kind, &resource.name, &resource.sha256))
        .collect::<Vec<_>>();
    digest::sha256_hex(&serde_json::to_vec(&identities).expect("desired identities serialize"))
}

/// Generated structural CRD plus API-server CEL admission rules that mirror
/// the controller's fail-closed validation for deployment-critical fields.
pub fn crd_document() -> Value {
    let mut document = serde_json::to_value(RrdInstance::crd()).expect("RRD CRD serializes");
    let spec =
        &mut document["spec"]["versions"][0]["schema"]["openAPIV3Schema"]["properties"]["spec"];
    spec["x-kubernetes-validations"] = json!([
        {"rule": "self.contractVersion == 1", "message": "contractVersion must be 1"},
        {"rule": "self.installationPlanSha256.matches('^[0-9a-f]{64}$')", "message": "installationPlanSha256 must be canonical lowercase SHA-256"},
        {"rule": "self.storage.retainOnDelete == true", "message": "PVC retention is mandatory in v1alpha1"},
        {"rule": "self.image.matches('^.+@sha256:[0-9a-f]{64}$')", "message": "image must be pinned by canonical sha256 digest"},
        {"rule": "self.storage.size.matches('^[1-9][0-9]*(Ki|Mi|Gi|Ti)$')", "message": "storage size must be a positive binary quantity"},
        {"rule": "self.tlsSecret.matches('^[a-z0-9]([-a-z0-9]*[a-z0-9])?$')", "message": "tlsSecret must be one DNS label"},
        {"rule": "self.installationPlanConfigMap.matches('^[a-z0-9]([-a-z0-9]*[a-z0-9])?$')", "message": "installationPlanConfigMap must be one DNS label"}
    ]);
    for property in ["tlsSecret", "installationPlanConfigMap"] {
        spec["properties"][property]["minLength"] = json!(1);
        spec["properties"][property]["maxLength"] = json!(63);
    }
    document
}

pub(super) fn resource(
    kind: KubernetesResourceKind,
    name: String,
    body: Value,
) -> Result<DesiredResource, String> {
    let encoded = serde_json::to_vec(&body).map_err(|error| error.to_string())?;
    Ok(DesiredResource {
        kind,
        name,
        sha256: digest::sha256_hex(&encoded),
        body,
    })
}

pub(super) fn labels(name: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("app.kubernetes.io/name".into(), "rrd".into()),
        ("app.kubernetes.io/instance".into(), name.into()),
        (
            "app.kubernetes.io/managed-by".into(),
            RRD_OPERATOR_FIELD_MANAGER.into(),
        ),
    ])
}

pub(super) fn bounded_name(name: &str, suffix: &str) -> Result<String, String> {
    let combined = format!("{name}-{suffix}");
    if combined.len() > 63 {
        return Err(format!("resource name {combined:?} exceeds 63 bytes"));
    }
    Ok(combined)
}
