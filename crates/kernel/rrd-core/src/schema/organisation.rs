//! Estate organisation: domains, categories, tags and their validation.

use super::*;

/// Built-in domain holding every kind and collection not placed elsewhere.
pub const DEFAULT_DOMAIN: &str = "default";
/// Built-in domain holding the engine-owned kinds and collections.
pub const ENGINE_DOMAIN: &str = "rrflow";
/// Identity prefix that marks a kind or collection as engine-owned.
pub const ENGINE_OWNED_PREFIX: &str = "rrflow-";
pub const MAX_DOMAINS: usize = 256;
pub const MAX_DOMAIN_DEPTH: usize = 8;
pub const MAX_CATEGORIES: usize = 64;
pub const MAX_TAGS_PER_CATEGORY: usize = 4_096;
pub const MAX_TAGS_PER_KIND: usize = 64;
pub const MAX_ORGANISATION_TITLE_BYTES: usize = 128;
pub const MAX_ORGANISATION_DESCRIPTION_BYTES: usize = 1_024;

/// One configurable part of an estate's catalogue, such as a service, package
/// or subject area. Isolation arrives with its enforcement in H-11d.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDomainSchema {
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// `None` and `default` both place the domain at the top level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<RuntimeType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCategoryTargets {
    Kinds,
    Records,
    KindsAndRecords,
}

impl RuntimeCategoryTargets {
    pub fn includes_kinds(self) -> bool {
        matches!(self, Self::Kinds | Self::KindsAndRecords)
    }

    pub fn includes_records(self) -> bool {
        matches!(self, Self::Records | Self::KindsAndRecords)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTagCardinality {
    Single,
    Multiple,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeTagSchema {
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// A governed vocabulary of tags. A closed category gains tags only through a
/// schema revision; an open one also admits tags written under its grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCategorySchema {
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub targets: RuntimeCategoryTargets,
    pub cardinality: RuntimeTagCardinality,
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub tags: BTreeMap<RuntimeType, RuntimeTagSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuntimeTagRef {
    pub category: RuntimeType,
    pub tag: RuntimeType,
}

/// Domains, categories and memberships of one estate catalogue. A kind or
/// collection without an entry belongs to its default domain, so a registry
/// written before organisation existed keeps its meaning and its digest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEstateOrganisation {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub domains: BTreeMap<RuntimeType, RuntimeDomainSchema>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub categories: BTreeMap<RuntimeType, RuntimeCategorySchema>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub kind_domains: BTreeMap<RuntimeType, RuntimeType>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub collection_domains: BTreeMap<RuntimeType, RuntimeType>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub kind_tags: BTreeMap<RuntimeType, BTreeSet<RuntimeTagRef>>,
}

impl RuntimeEstateOrganisation {
    pub fn is_default(&self) -> bool {
        self.domains.is_empty()
            && self.categories.is_empty()
            && self.kind_domains.is_empty()
            && self.collection_domains.is_empty()
            && self.kind_tags.is_empty()
    }

    /// The one domain a kind belongs to.
    pub fn domain_of_kind<'a>(&'a self, kind: &'a RuntimeType) -> &'a str {
        self.kind_domains
            .get(kind)
            .map_or_else(|| default_domain(kind.as_str()), RuntimeType::as_str)
    }

    /// The one domain a vector collection belongs to.
    pub fn domain_of_collection<'a>(&'a self, collection: &'a str) -> &'a str {
        self.collection_domains
            .iter()
            .find(|(id, _)| id.as_str() == collection)
            .map_or_else(|| default_domain(collection), |(_, domain)| domain.as_str())
    }

    pub fn has_domain(&self, domain: &str) -> bool {
        is_builtin_domain(domain) || self.domains.keys().any(|id| id.as_str() == domain)
    }

    pub(super) fn validate(
        &self,
        tables: &BTreeMap<RuntimeType, RuntimeTableSchema>,
    ) -> Result<()> {
        if self.domains.len() + 2 > MAX_DOMAINS {
            return invalid(format!(
                "an estate may declare at most {} domains besides default and rrflow",
                MAX_DOMAINS - 2
            ));
        }
        for (id, domain) in &self.domains {
            if is_builtin_domain(id.as_str()) || id.as_str().starts_with(ENGINE_OWNED_PREFIX) {
                return invalid(format!(
                    "domain {id} is built in or reserved for the engine and cannot be declared"
                ));
            }
            validate_label("domain", id, &domain.title, &domain.description)?;
            if let Some(parent) = &domain.parent {
                if parent.as_str() == ENGINE_DOMAIN {
                    return invalid(format!("domain {id} cannot be placed inside rrflow"));
                }
                if parent.as_str() != DEFAULT_DOMAIN && !self.domains.contains_key(parent) {
                    return invalid(format!("domain {id} names unknown parent {parent}"));
                }
            }
            self.domain_depth(id)?;
        }
        for (kind, domain) in &self.kind_domains {
            if !tables.contains_key(kind) {
                return invalid(format!(
                    "domain membership names unregistered table type {kind}"
                ));
            }
            self.validate_membership("table type", kind, domain)?;
        }
        for (collection, domain) in &self.collection_domains {
            self.validate_membership("vector collection", collection, domain)?;
        }
        if self.categories.len() > MAX_CATEGORIES {
            return invalid(format!(
                "an estate may declare at most {MAX_CATEGORIES} categories"
            ));
        }
        for (id, category) in &self.categories {
            if id.as_str() == ENGINE_DOMAIN || id.as_str().starts_with(ENGINE_OWNED_PREFIX) {
                return invalid(format!("category {id} is reserved for the engine"));
            }
            validate_label("category", id, &category.title, &category.description)?;
            if category.tags.len() > MAX_TAGS_PER_CATEGORY {
                return invalid(format!(
                    "category {id} may declare at most {MAX_TAGS_PER_CATEGORY} tags"
                ));
            }
            for (tag, schema) in &category.tags {
                validate_label("tag", tag, &schema.title, &schema.description)?;
            }
        }
        for (kind, tags) in &self.kind_tags {
            if !tables.contains_key(kind) {
                return invalid(format!("kind tags name unregistered table type {kind}"));
            }
            if tags.is_empty() {
                return invalid(format!(
                    "table type {kind} lists an empty tag set; omit the entry instead"
                ));
            }
            if tags.len() > MAX_TAGS_PER_KIND {
                return invalid(format!(
                    "table type {kind} may carry at most {MAX_TAGS_PER_KIND} tags"
                ));
            }
            let mut single = BTreeSet::new();
            for reference in tags {
                let category = self.categories.get(&reference.category).ok_or_else(|| {
                    Error::InvalidRuntime {
                        reason: format!(
                            "table type {kind} uses unknown category {}",
                            reference.category
                        ),
                    }
                })?;
                if !category.targets.includes_kinds() {
                    return invalid(format!(
                        "category {} does not tag table types",
                        reference.category
                    ));
                }
                if !category.tags.contains_key(&reference.tag) {
                    return invalid(format!(
                        "table type {kind} uses unknown tag {}:{}",
                        reference.category, reference.tag
                    ));
                }
                if category.cardinality == RuntimeTagCardinality::Single
                    && !single.insert(&reference.category)
                {
                    return invalid(format!(
                        "table type {kind} carries more than one tag of single-tag category {}",
                        reference.category
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_membership(
        &self,
        subject: &str,
        identity: &RuntimeType,
        domain: &RuntimeType,
    ) -> Result<()> {
        if identity.as_str().starts_with(ENGINE_OWNED_PREFIX) {
            return invalid(format!(
                "engine-owned {subject} {identity} always belongs to rrflow"
            ));
        }
        if domain.as_str() == ENGINE_DOMAIN {
            return invalid(format!(
                "{subject} {identity} is not engine-owned and cannot be placed in rrflow"
            ));
        }
        if domain.as_str() == DEFAULT_DOMAIN {
            return invalid(format!(
                "{subject} {identity} already belongs to default; omit the entry instead"
            ));
        }
        if !self.domains.contains_key(domain) {
            return invalid(format!(
                "{subject} {identity} names unknown domain {domain}"
            ));
        }
        Ok(())
    }

    /// Levels below `default`, refusing cycles and excess depth.
    fn domain_depth(&self, id: &RuntimeType) -> Result<usize> {
        let mut depth = 1;
        let mut current = id;
        while let Some(parent) = self.domains[current]
            .parent
            .as_ref()
            .filter(|parent| parent.as_str() != DEFAULT_DOMAIN)
        {
            if parent == id {
                return invalid(format!("domain {id} is its own ancestor"));
            }
            depth += 1;
            if depth > MAX_DOMAIN_DEPTH {
                return invalid(format!(
                    "domain {id} is deeper than {MAX_DOMAIN_DEPTH} levels or part of a cycle"
                ));
            }
            current = parent;
        }
        Ok(depth)
    }
}

pub(super) fn is_builtin_domain(domain: &str) -> bool {
    domain == DEFAULT_DOMAIN || domain == ENGINE_DOMAIN
}

pub(super) fn default_domain(identity: &str) -> &'static str {
    if identity.starts_with(ENGINE_OWNED_PREFIX) {
        ENGINE_DOMAIN
    } else {
        DEFAULT_DOMAIN
    }
}

pub(super) fn validate_label(
    subject: &str,
    id: &RuntimeType,
    title: &str,
    description: &str,
) -> Result<()> {
    if title.trim().is_empty() || title.len() > MAX_ORGANISATION_TITLE_BYTES {
        return invalid(format!(
            "{subject} {id} needs a title of 1 to {MAX_ORGANISATION_TITLE_BYTES} bytes"
        ));
    }
    if description.len() > MAX_ORGANISATION_DESCRIPTION_BYTES {
        return invalid(format!(
            "{subject} {id} description exceeds {MAX_ORGANISATION_DESCRIPTION_BYTES} bytes"
        ));
    }
    if title
        .chars()
        .chain(description.chars())
        .any(char::is_control)
    {
        return invalid(format!(
            "{subject} {id} title and description cannot contain control characters"
        ));
    }
    Ok(())
}
