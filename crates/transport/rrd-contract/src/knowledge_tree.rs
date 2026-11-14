//! Versioned logical knowledge trees and their configured data-source paths.
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

/// A logical grouping or an explicitly configured source. Provider identifiers
/// are open canonical identities; their presence does not advertise an adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KnowledgePathBinding {
    Group,
    /// The project root (`None`) or one portable project-relative directory.
    /// This is configuration, not permission to read or ingest the directory.
    ProjectDirectory {
        relative_path: Option<String>,
    },
    Native {
        collection_id: CanonicalId,
        vector_name: CanonicalId,
    },
    External {
        provider_id: CanonicalId,
        /// Existing endpoint/credential-reference and admission-limit contract.
        /// API/subscription bindings require a credential reference; a declared
        /// local or self-hosted service may permit anonymous access.
        endpoint: Box<EndpointBinding>,
        namespace: Option<String>,
        index: Option<String>,
        collection: Option<String>,
    },
}

impl KnowledgePathBinding {
    pub fn validate(&self) -> Result<()> {
        if let Self::ProjectDirectory { relative_path } = self {
            if let Some(path) = relative_path {
                crate::knowledge::validate_source_path(path)?;
                if path.contains([':', '<', '>', '"', '|', '?', '*']) {
                    return invalid("project directory must use portable relative path syntax");
                }
                for segment in path.split('/') {
                    let stem = segment
                        .split('.')
                        .next()
                        .unwrap_or_default()
                        .to_ascii_uppercase();
                    let reserved = matches!(
                        stem.as_str(),
                        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                    ) || stem
                        .strip_prefix("COM")
                        .or_else(|| stem.strip_prefix("LPT"))
                        .is_some_and(|suffix| {
                            matches!(
                                suffix,
                                "1" | "2"
                                    | "3"
                                    | "4"
                                    | "5"
                                    | "6"
                                    | "7"
                                    | "8"
                                    | "9"
                                    | "\u{b9}"
                                    | "\u{b2}"
                                    | "\u{b3}"
                            )
                        });
                    if segment.ends_with(['.', ' ']) || reserved {
                        return invalid("project directory contains a non-portable segment");
                    }
                }
            }
            return Ok(());
        }
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
mod tests;
