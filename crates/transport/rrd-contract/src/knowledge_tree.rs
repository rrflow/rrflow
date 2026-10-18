//! Versioned logical knowledge trees and their configured vector-store paths.
//!
//! The engine owns persistence, authorization, native collection validation and
//! provider execution. A saved external binding is configuration, not evidence
//! that its provider is available, connected, searchable or synchronized.

use crate::{
    invalid, sha256_bytes, validate_vector_scope, CanonicalId, ContractError, EndpointBinding,
    EndpointLocator, Result, WorkloadClass,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const KNOWLEDGE_TREE_CONTRACT_VERSION: u16 = 1;
pub const MAX_KNOWLEDGE_TREES: usize = 64;
pub const MAX_KNOWLEDGE_PATHS_PER_TREE: usize = 512;
pub const MAX_KNOWLEDGE_PATHS: usize = 4_096;
pub const MAX_KNOWLEDGE_PATH_DEPTH: usize = 64;
pub const MAX_KNOWLEDGE_NAME_BYTES: usize = 256;
pub const MAX_KNOWLEDGE_PROVIDER_IDENTIFIER_BYTES: usize = 256;
// Leave room under the public 1MiB HTTP bound for request envelopes and
// engine control-record / exact-result receipt metadata.
pub const MAX_KNOWLEDGE_CATALOGUE_BYTES: usize = 768 * 1024;

/// A logical grouping or an explicit vector-store source. Provider identifiers
/// are open canonical identities; their presence does not advertise an adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KnowledgePathBinding {
    Group,
    Native {
        collection_id: CanonicalId,
        vector_name: CanonicalId,
    },
    External {
        provider_id: CanonicalId,
        /// Existing endpoint/credential-reference and admission-limit contract.
        /// API/subscription bindings require a credential reference; a declared
        /// local or self-hosted service may permit anonymous access.
        endpoint: EndpointBinding,
        namespace: Option<String>,
        index: Option<String>,
        collection: Option<String>,
    },
}

impl KnowledgePathBinding {
    pub fn validate(&self) -> Result<()> {
        let Self::External {
            endpoint,
            namespace,
            index,
            collection,
            ..
        } = self
        else {
            return Ok(());
        };
        endpoint.validate()?;
        if endpoint.class != WorkloadClass::VectorStore {
            return invalid("knowledge external endpoint must serve the vector_store class");
        }
        let EndpointLocator::Url { url } = &endpoint.endpoint else {
            return invalid("knowledge external endpoint must be a URL");
        };
        // EndpointBinding already rejects userinfo and invalid endpoint values.
        // Query strings and fragments are excluded because they commonly carry
        // secret values; credentials belong in its CredentialReference only.
        if url.contains(['?', '#']) {
            return invalid("knowledge endpoint URL must not contain a query or fragment");
        }
        let authority = url
            .split_once("://")
            .map(|(_, rest)| rest.split('/').next().unwrap_or_default())
            .unwrap_or_default();
        if authority.contains('%') {
            return invalid("knowledge endpoint authority must not contain percent encoding");
        }
        if namespace.is_none() && index.is_none() && collection.is_none() {
            return invalid("knowledge external path must name a namespace, index or collection");
        }
        for (label, value) in [
            ("namespace", namespace),
            ("index", index),
            ("collection", collection),
        ] {
            if let Some(value) = value {
                validate_text(value, MAX_KNOWLEDGE_PROVIDER_IDENTIFIER_BYTES, label)?;
            }
        }
        Ok(())
    }
}

/// IDs are identities; names are editable human-readable labels. A parent is
/// resolved within this path's tree, never against another tree or scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgePath {
    pub id: CanonicalId,
    pub name: String,
    pub parent_id: Option<CanonicalId>,
    pub binding: KnowledgePathBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeTree {
    pub id: CanonicalId,
    pub name: String,
    pub paths: Vec<KnowledgePath>,
}

/// One project's catalogue within an existing data scope. Revision zero is
/// the empty initial state. Replacements receive the next revision from the
/// engine; clients do not submit an independently asserted revision or digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeTreeCatalogue {
    pub contract_version: u16,
    pub project_id: CanonicalId,
    pub scope: String,
    pub revision: u64,
    pub trees: Vec<KnowledgeTree>,
}

impl KnowledgeTreeCatalogue {
    pub fn empty(project_id: CanonicalId, scope: String) -> Result<Self> {
        let catalogue = Self {
            contract_version: KNOWLEDGE_TREE_CONTRACT_VERSION,
            project_id,
            scope,
            revision: 0,
            trees: Vec::new(),
        };
        catalogue.validate()?;
        Ok(catalogue)
    }

    pub fn validate(&self) -> Result<()> {
        if self.contract_version != KNOWLEDGE_TREE_CONTRACT_VERSION {
            return invalid("unsupported knowledge-tree contract version");
        }
        validate_vector_scope(&self.scope)?;
        if self.revision == 0 && !self.trees.is_empty() {
            return invalid("knowledge catalogue revision zero must be empty");
        }
        validate_trees(&self.trees)?;
        validate_encoded_size(self)
    }

    /// Canonical identity ordering makes graph declaration order irrelevant.
    /// Revision, project and scope remain part of the content identity.
    pub fn sha256(&self) -> Result<String> {
        self.validate()?;
        let mut canonical = self.clone();
        canonicalize_trees(&mut canonical.trees);
        let encoded = serde_json::to_vec(&canonical)
            .map_err(|error| ContractError(format!("knowledge digest encoding failed: {error}")))?;
        let mut bytes = Vec::with_capacity(32 + encoded.len());
        bytes.extend_from_slice(b"rrflow-knowledge-tree-v1\0");
        bytes.extend_from_slice(&encoded);
        Ok(sha256_bytes(&bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListKnowledgeTrees {
    pub project_id: CanonicalId,
    pub scope: String,
}

impl ListKnowledgeTrees {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)
    }
}

/// Full desired catalogue contents under compare-and-swap. Omitting an entry
/// removes it. Changing a label does not change identity; moving a retained
/// path identity between trees is rejected by validate_against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplaceKnowledgeTrees {
    pub project_id: CanonicalId,
    pub scope: String,
    pub expected_revision: u64,
    pub trees: Vec<KnowledgeTree>,
}

impl ReplaceKnowledgeTrees {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        self.expected_revision
            .checked_add(1)
            .ok_or_else(|| ContractError("knowledge catalogue revision overflow".into()))?;
        validate_trees(&self.trees)?;
        // Check the actual resulting catalogue as well so its revision and
        // versioned wrapper cannot take an otherwise legal request over budget.
        validate_encoded_size(self)?;
        validate_encoded_size(&self.snapshot()?)
    }

    /// Structural and identity checks against a previously validated snapshot.
    /// The engine still owns atomic CAS, security and idempotent replay.
    pub fn validate_against(&self, previous: &KnowledgeTreeCatalogue) -> Result<()> {
        self.validate()?;
        previous.validate()?;
        if self.project_id != previous.project_id || self.scope != previous.scope {
            return invalid("knowledge replacement project or scope does not match catalogue");
        }
        if self.expected_revision != previous.revision {
            return invalid("knowledge replacement expected revision does not match catalogue");
        }
        let owners: BTreeMap<_, _> = previous
            .trees
            .iter()
            .flat_map(|tree| tree.paths.iter().map(move |path| (&path.id, &tree.id)))
            .collect();
        for tree in &self.trees {
            for path in &tree.paths {
                if owners.get(&path.id).is_some_and(|owner| *owner != &tree.id) {
                    return invalid("knowledge path identity cannot move between trees");
                }
            }
        }
        Ok(())
    }

    pub fn into_catalogue(self) -> Result<KnowledgeTreeCatalogue> {
        self.validate()?;
        let mut catalogue = self.snapshot()?;
        canonicalize_trees(&mut catalogue.trees);
        Ok(catalogue)
    }

    fn snapshot(&self) -> Result<KnowledgeTreeCatalogue> {
        Ok(KnowledgeTreeCatalogue {
            contract_version: KNOWLEDGE_TREE_CONTRACT_VERSION,
            project_id: self.project_id.clone(),
            scope: self.scope.clone(),
            revision: self
                .expected_revision
                .checked_add(1)
                .ok_or_else(|| ContractError("knowledge catalogue revision overflow".into()))?,
            trees: self.trees.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplaceKnowledgeTreesResult {
    pub catalogue: KnowledgeTreeCatalogue,
    pub idempotent_replay: bool,
}

fn canonicalize_trees(trees: &mut [KnowledgeTree]) {
    trees.sort_by(|left, right| left.id.cmp(&right.id));
    for tree in trees {
        tree.paths.sort_by(|left, right| left.id.cmp(&right.id));
    }
}

fn validate_trees(trees: &[KnowledgeTree]) -> Result<()> {
    if trees.len() > MAX_KNOWLEDGE_TREES {
        return invalid("knowledge catalogue exceeds its tree count budget");
    }
    let mut tree_ids = BTreeSet::new();
    let mut tree_names = BTreeSet::new();
    let mut path_ids = BTreeSet::new();
    let mut total_paths = 0;
    for tree in trees {
        validate_text(&tree.name, MAX_KNOWLEDGE_NAME_BYTES, "tree name")?;
        if !tree_ids.insert(&tree.id) {
            return invalid("knowledge tree identity repeats");
        }
        if !tree_names.insert(&tree.name) {
            return invalid("knowledge tree name repeats");
        }
        if tree.paths.len() > MAX_KNOWLEDGE_PATHS_PER_TREE {
            return invalid("knowledge tree exceeds its path count budget");
        }
        total_paths += tree.paths.len();
        if total_paths > MAX_KNOWLEDGE_PATHS {
            return invalid("knowledge catalogue exceeds its total path budget");
        }
        let paths: BTreeMap<_, _> = tree.paths.iter().map(|path| (&path.id, path)).collect();
        let mut sibling_names = BTreeSet::new();
        for path in &tree.paths {
            validate_text(&path.name, MAX_KNOWLEDGE_NAME_BYTES, "path name")?;
            if !path_ids.insert(&path.id) {
                return invalid("knowledge path identity repeats");
            }
            if !sibling_names.insert((&path.parent_id, &path.name)) {
                return invalid("knowledge sibling path name repeats");
            }
            path.binding.validate()?;
            if let Some(parent_id) = &path.parent_id {
                let Some(parent) = paths.get(parent_id) else {
                    return invalid("knowledge path parent does not exist in its tree");
                };
                if !matches!(parent.binding, KnowledgePathBinding::Group) {
                    return invalid("knowledge path parent must be a grouping node");
                }
            }
        }
        // Bounded iterative walks avoid recursion on untrusted parent edges.
        for path in &tree.paths {
            let mut seen = BTreeSet::new();
            let mut cursor = Some(&path.id);
            while let Some(id) = cursor {
                if !seen.insert(id) {
                    return invalid("knowledge path parent graph contains a cycle");
                }
                if seen.len() > MAX_KNOWLEDGE_PATH_DEPTH {
                    return invalid("knowledge path exceeds its depth budget");
                }
                cursor = paths.get(id).and_then(|node| node.parent_id.as_ref());
            }
        }
    }
    Ok(())
}

fn validate_text(value: &str, maximum: usize, label: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > maximum || value.trim() != value {
        return invalid(format!(
            "knowledge {label} must be nonempty, bounded and trimmed"
        ));
    }
    if value.chars().any(|character| {
        character.is_control()
            || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
    }) {
        return invalid(format!("knowledge {label} must not contain control or bidi-format characters"));
    }
    Ok(())
}

fn validate_encoded_size(value: &impl Serialize) -> Result<()> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| ContractError(format!("knowledge catalogue encoding failed: {error}")))?;
    if encoded.len() > MAX_KNOWLEDGE_CATALOGUE_BYTES {
        return invalid("knowledge catalogue exceeds its encoded byte budget");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BindingKind, BindingLimits, CredentialReference, DeclaredFootprint, GpuRequirement,
    };

    fn id(value: &str) -> CanonicalId {
        CanonicalId::new(value).unwrap()
    }

    fn group(value: &str, parent: Option<&str>) -> KnowledgePath {
        KnowledgePath {
            id: id(value),
            name: value.into(),
            parent_id: parent.map(id),
            binding: KnowledgePathBinding::Group,
        }
    }

    fn native(value: &str, parent: Option<&str>) -> KnowledgePath {
        KnowledgePath {
            binding: KnowledgePathBinding::Native {
                collection_id: id("documents"),
                vector_name: id("semantic"),
            },
            ..group(value, parent)
        }
    }

    fn request() -> ReplaceKnowledgeTrees {
        ReplaceKnowledgeTrees {
            project_id: id("project-alpha"),
            scope: "development".into(),
            expected_revision: 0,
            trees: vec![KnowledgeTree {
                id: id("research"),
                name: "Research 知識".into(),
                paths: vec![group("papers", None), native("embeddings", Some("papers"))],
            }],
        }
    }

    fn external() -> KnowledgePathBinding {
        KnowledgePathBinding::External {
            provider_id: id("postgres-pgvector"),
            endpoint: EndpointBinding {
                id: id("vector-postgres"),
                class: WorkloadClass::VectorStore,
                kind: BindingKind::SelfHosted,
                endpoint: EndpointLocator::Url {
                    url: "postgresql://localhost:5432/knowledge".into(),
                },
                service_id: id("knowledge-postgres"),
                manifest_sha256: None,
                credential: Some(CredentialReference {
                    provider: id("local-vault"),
                    name: "knowledge/postgres".into(),
                }),
                placement: Some(id("local-machine")),
                footprint: DeclaredFootprint {
                    memory_bytes: 1024,
                    gpu: GpuRequirement::None,
                },
                limits: BindingLimits {
                    max_concurrency: 1,
                    timeout_ms: 1000,
                    max_input_bytes: 1024,
                },
            },
            namespace: Some("Public".into()),
            index: None,
            collection: Some("ResearchDocuments".into()),
        }
    }

    #[test]
    fn knowledge_tree_group_and_native_roundtrip_keeps_human_names() {
        let catalogue = request().into_catalogue().unwrap();
        catalogue.validate().unwrap();
        assert_eq!(catalogue.revision, 1);
        assert_eq!(catalogue.trees[0].name, "Research 知識");
        let json = serde_json::to_string(&catalogue).unwrap();
        let decoded: KnowledgeTreeCatalogue = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, catalogue);
        assert!(json.contains("\"kind\":\"group\""));
    }

    #[test]
    fn knowledge_tree_rejects_duplicate_ids_names_and_cross_tree_paths() {
        let mut duplicate_tree = request();
        duplicate_tree.trees.push(duplicate_tree.trees[0].clone());
        assert!(duplicate_tree.validate().is_err());
        let mut duplicate_name = request();
        let mut tree = duplicate_name.trees[0].clone();
        tree.id = id("another-tree");
        tree.paths.clear();
        duplicate_name.trees.push(tree);
        assert!(duplicate_name.validate().is_err());
        let mut duplicate_path = request();
        duplicate_path.trees[0]
            .paths
            .push(native("embeddings", None));
        assert!(duplicate_path.validate().is_err());
        let mut cross_tree = request();
        cross_tree.trees.push(KnowledgeTree {
            id: id("second"),
            name: "Second".into(),
            paths: vec![group("papers", None)],
        });
        assert!(cross_tree.validate().is_err());
        let mut siblings = request();
        let mut path = native("second-leaf", Some("papers"));
        path.name = "embeddings".into();
        siblings.trees[0].paths.push(path);
        assert!(siblings.validate().is_err());
    }

    #[test]
    fn knowledge_tree_rejects_cycles_missing_parents_and_non_group_parents() {
        for paths in [
            vec![group("self-cycle", Some("self-cycle"))],
            vec![group("left", Some("right")), group("right", Some("left"))],
            vec![group("orphan", Some("missing"))],
            vec![native("leaf", None), native("child", Some("leaf"))],
        ] {
            let mut candidate = request();
            candidate.trees[0].paths = paths;
            assert!(candidate.validate().is_err());
        }
        let mut other_tree = request();
        other_tree.trees.push(KnowledgeTree {
            id: id("other"),
            name: "Other".into(),
            paths: vec![group("wrong-tree-parent", Some("papers"))],
        });
        assert!(other_tree.validate().is_err());
    }

    #[test]
    fn knowledge_tree_depth_budget_is_inclusive() {
        let mut candidate = request();
        candidate.trees[0].paths = (0..MAX_KNOWLEDGE_PATH_DEPTH)
            .map(|index| {
                group(
                    &format!("node-{index}"),
                    (index > 0)
                        .then(|| format!("node-{}", index - 1))
                        .as_deref(),
                )
            })
            .collect();
        candidate.validate().unwrap();
        candidate.trees[0].paths.push(group(
            "too-deep",
            Some(&format!("node-{}", MAX_KNOWLEDGE_PATH_DEPTH - 1)),
        ));
        assert!(candidate.validate().is_err());
    }

    #[test]
    fn knowledge_tree_enforces_tree_path_and_total_count_budgets() {
        let mut candidate = request();
        candidate.trees = (0..=MAX_KNOWLEDGE_TREES)
            .map(|index| KnowledgeTree {
                id: id(&format!("tree-{index}")),
                name: format!("Tree {index}"),
                paths: Vec::new(),
            })
            .collect();
        assert!(candidate.validate().is_err());
        candidate.trees = vec![KnowledgeTree {
            id: id("large-tree"),
            name: "Large tree".into(),
            paths: (0..=MAX_KNOWLEDGE_PATHS_PER_TREE)
                .map(|index| group(&format!("path-{index}"), None))
                .collect(),
        }];
        assert!(candidate.validate().is_err());
        candidate.trees = (0..9)
            .map(|tree_index| KnowledgeTree {
                id: id(&format!("tree-{tree_index}")),
                name: format!("Tree {tree_index}"),
                paths: (0..MAX_KNOWLEDGE_PATHS_PER_TREE)
                    .map(|path_index| group(&format!("path-{tree_index}-{path_index}"), None))
                    .collect(),
            })
            .collect();
        assert!(candidate.validate().is_err());
    }

    #[test]
    fn knowledge_tree_checks_encoded_budget_after_individual_field_bounds() {
        let mut candidate = request();
        candidate.trees = (0..3)
            .map(|tree_index| KnowledgeTree {
                id: id(&format!("tree-{tree_index}")),
                name: format!("Tree {tree_index}"),
                paths: (0..MAX_KNOWLEDGE_PATHS_PER_TREE)
                    .map(|path_index| {
                        let mut path = native(&format!("path-{tree_index}-{path_index}"), None);
                        path.binding = external();
                        if let KnowledgePathBinding::External { endpoint, .. } = &mut path.binding {
                            endpoint.endpoint = EndpointLocator::Url {
                                url: format!("https://example.test/{}", "p".repeat(1800)),
                            };
                        }
                        path
                    })
                    .collect(),
            })
            .collect();
        let error = candidate.validate().unwrap_err();
        assert!(error.to_string().contains("encoded byte budget"));
    }

    #[test]
    fn knowledge_tree_rejects_unsafe_names_and_counts_utf8_bytes() {
        for name in [
            "",
            " ",
            " leading",
            "trailing ",
            "line\nbreak",
            "spoof\u{202e}name",
        ] {
            let mut candidate = request();
            candidate.trees[0].name = name.into();
            assert!(candidate.validate().is_err());
        }
        let mut candidate = request();
        candidate.trees[0].name = "知".repeat(MAX_KNOWLEDGE_NAME_BYTES / 3 + 1);
        assert!(candidate.validate().is_err());
    }

    #[test]
    fn knowledge_tree_external_configuration_reuses_endpoint_credential_rules() {
        external().validate().unwrap();
        let mut anonymous = external();
        if let KnowledgePathBinding::External { endpoint, .. } = &mut anonymous {
            endpoint.credential = None;
        }
        anonymous.validate().unwrap();
        if let KnowledgePathBinding::External { endpoint, .. } = &mut anonymous {
            endpoint.kind = BindingKind::Api;
            endpoint.placement = None;
        }
        assert!(anonymous.validate().is_err());
        let mut invalid_credential = external();
        if let KnowledgePathBinding::External { endpoint, .. } = &mut invalid_credential {
            endpoint.credential.as_mut().unwrap().name = "secret value".into();
        }
        assert!(invalid_credential.validate().is_err());
        let mut wrong_class = external();
        if let KnowledgePathBinding::External { endpoint, .. } = &mut wrong_class {
            endpoint.class = WorkloadClass::CanonicalStore;
        }
        assert!(wrong_class.validate().is_err());
        let mut no_destination = external();
        if let KnowledgePathBinding::External {
            namespace,
            collection,
            ..
        } = &mut no_destination
        {
            *namespace = None;
            *collection = None;
        }
        assert!(no_destination.validate().is_err());
    }

    #[test]
    fn knowledge_tree_rejects_secret_urls_without_echoing_values() {
        for url in [
            "https://operator:do-not-print@example.test/vectors",
            "https://example.test/vectors?api_key=do-not-print",
            "https://example.test/vectors#do-not-print",
            "https://operator%40example.test/vectors",
        ] {
            let mut binding = external();
            if let KnowledgePathBinding::External { endpoint, .. } = &mut binding {
                endpoint.endpoint = EndpointLocator::Url { url: url.into() };
            }
            let error = binding.validate().unwrap_err();
            assert!(!error.to_string().contains("do-not-print"));
        }
        let mut in_process = external();
        if let KnowledgePathBinding::External { endpoint, .. } = &mut in_process {
            endpoint.kind = BindingKind::Local;
            endpoint.endpoint = EndpointLocator::InProcess {
                component: id("not-external"),
            };
        }
        assert!(in_process.validate().is_err());
    }

    #[test]
    fn knowledge_tree_deserialization_rejects_unknown_secret_and_identity_fields() {
        let mut value = serde_json::to_value(request()).unwrap();
        value["password"] = serde_json::json!("do-not-store");
        assert!(serde_json::from_value::<ReplaceKnowledgeTrees>(value).is_err());
        let mut value = serde_json::to_value(external()).unwrap();
        value["endpoint"]["credential"]["value"] = serde_json::json!("do-not-store");
        assert!(serde_json::from_value::<KnowledgePathBinding>(value).is_err());
        let mut value = serde_json::to_value(request()).unwrap();
        value["trees"][0]["id"] = serde_json::json!("Not Canonical");
        assert!(serde_json::from_value::<ReplaceKnowledgeTrees>(value).is_err());
    }

    #[test]
    fn knowledge_tree_canonical_digest_binds_scope_project_revision_and_content() {
        let first = request().into_catalogue().unwrap();
        let mut reordered = first.clone();
        reordered.trees[0].paths.reverse();
        assert_eq!(first.sha256().unwrap(), reordered.sha256().unwrap());
        for changed in [
            {
                let mut value = first.clone();
                value.scope = "another".into();
                value
            },
            {
                let mut value = first.clone();
                value.project_id = id("project-beta");
                value
            },
            {
                let mut value = first.clone();
                value.revision += 1;
                value
            },
            {
                let mut value = first.clone();
                value.trees[0].name = "Changed".into();
                value
            },
        ] {
            assert_ne!(first.sha256().unwrap(), changed.sha256().unwrap());
        }
        let mut two_trees = request();
        two_trees.trees.push(KnowledgeTree {
            id: id("a-first"),
            name: "First".into(),
            paths: Vec::new(),
        });
        let catalogue = two_trees.into_catalogue().unwrap();
        assert_eq!(catalogue.trees[0].id, id("a-first"));
        let mut reversed = catalogue.clone();
        reversed.trees.reverse();
        assert_eq!(catalogue.sha256().unwrap(), reversed.sha256().unwrap());
    }

    #[test]
    fn knowledge_tree_revision_and_identity_replacement_checks_are_explicit() {
        let previous = request().into_catalogue().unwrap();
        let mut next = request();
        next.expected_revision = 1;
        next.trees[0].name = "Renamed".into();
        next.validate_against(&previous).unwrap();
        let mut stale = next.clone();
        stale.expected_revision = 0;
        assert!(stale.validate_against(&previous).is_err());
        let mut wrong_project = next.clone();
        wrong_project.project_id = id("project-beta");
        assert!(wrong_project.validate_against(&previous).is_err());
        let mut wrong_scope = next.clone();
        wrong_scope.scope = "another".into();
        assert!(wrong_scope.validate_against(&previous).is_err());
        let mut moved_identity = next.clone();
        moved_identity.trees[0].id = id("different-tree");
        assert!(moved_identity.validate_against(&previous).is_err());
        let mut overflow = next;
        overflow.expected_revision = u64::MAX;
        assert!(overflow.validate().is_err());
    }

    #[test]
    fn knowledge_tree_initial_and_empty_replacement_are_versioned_and_scoped() {
        let initial =
            KnowledgeTreeCatalogue::empty(id("project-alpha"), "development".into()).unwrap();
        assert_eq!(initial.revision, 0);
        let mut candidate = request();
        candidate.validate_against(&initial).unwrap();
        candidate.trees.clear();
        let empty = candidate.into_catalogue().unwrap();
        assert_eq!(empty.revision, 1);
        empty.validate().unwrap();
        let mut invalid_initial = request().into_catalogue().unwrap();
        invalid_initial.revision = 0;
        assert!(invalid_initial.validate().is_err());
        let mut future = empty;
        future.contract_version += 1;
        assert!(future.validate().is_err());
        for scope in ["", "with\0nul", &"s".repeat(crate::MAX_ID_BYTES + 1)] {
            assert!(ListKnowledgeTrees {
                project_id: id("project-alpha"),
                scope: scope.into()
            }
            .validate()
            .is_err());
        }
    }
}
