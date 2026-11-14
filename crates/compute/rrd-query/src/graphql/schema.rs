//! GraphQL requests, source definitions and the derived schema.

use super::*;

pub const GRAPHQL_ADAPTER_CONTRACT_VERSION: u16 = 1;
pub const MAX_GRAPHQL_DOCUMENT_BYTES: usize = 64 * 1024;
pub const MAX_GRAPHQL_VARIABLES: usize = 256;
pub(super) const MAX_GRAPHQL_ARGUMENTS: usize = 32;
pub(super) const MAX_GRAPHQL_PROJECTION_FIELDS: usize = 512;

/// One transport-neutral GraphQL request. HTTP carriage is intentionally a
/// later adapter gate; values already use RRFlow's canonical scalar model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlRequest {
    pub contract_version: u16,
    pub document: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub variables: Parameters,
}

impl GraphqlRequest {
    pub(super) fn validate_envelope(&self) -> Result<()> {
        if self.contract_version != GRAPHQL_ADAPTER_CONTRACT_VERSION {
            return graphql_error(format!(
                "unsupported GraphQL adapter contract version {}",
                self.contract_version
            ));
        }
        if self.document.is_empty() || self.document.len() > MAX_GRAPHQL_DOCUMENT_BYTES {
            return graphql_error(format!(
                "GraphQL document length must be in 1..={MAX_GRAPHQL_DOCUMENT_BYTES} bytes"
            ));
        }
        if self.variables.len() > MAX_GRAPHQL_VARIABLES {
            return graphql_error(format!(
                "GraphQL request exceeds {MAX_GRAPHQL_VARIABLES} variables"
            ));
        }
        if let Some(name) = &self.operation_name {
            validate_graphql_name(name, "operation name")?;
        }
        for name in self.variables.keys() {
            validate_graphql_name(name, "variable name")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphqlSourceFamily {
    Claim,
    Event,
    Geo,
    Record,
    Relation,
    Series,
    Traversal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlFieldDefinition {
    pub name: String,
    pub accepted_types: Vec<RuntimeValueType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlSourceDefinition {
    pub family: GraphqlSourceFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub fields: Vec<GraphqlFieldDefinition>,
}

/// A deterministic GraphQL-visible schema projection at one historical
/// schema revision inside a captured read stamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlSchema {
    pub contract_version: u16,
    pub read: ReadStamp,
    pub known_at_cursor: u64,
    pub schema_revision: u64,
    pub sources: Vec<GraphqlSourceDefinition>,
    pub digest: String,
}

impl GraphqlSchema {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != GRAPHQL_ADAPTER_CONTRACT_VERSION {
            return graphql_error("GraphQL schema contract version is unsupported");
        }
        self.read
            .validate()
            .map_err(|error| Error::Graphql(error.to_string()))?;
        if self.known_at_cursor > self.read.commit_cursor || self.schema_revision == 0 {
            return graphql_error("GraphQL schema carries an invalid read coordinate");
        }
        if self.sources.is_empty()
            || self
                .sources
                .windows(2)
                .any(|pair| source_sort_key(&pair[0]) >= source_sort_key(&pair[1]))
        {
            return graphql_error("GraphQL schema sources must be non-empty, sorted, and unique");
        }
        for source in &self.sources {
            if source.fields.is_empty()
                || source
                    .fields
                    .windows(2)
                    .any(|pair| pair[0].name >= pair[1].name)
            {
                return graphql_error(
                    "GraphQL source fields must be non-empty, sorted, and unique",
                );
            }
            if let Some(kind) = &source.kind {
                if kind.is_empty() {
                    return graphql_error("GraphQL source kind must not be empty");
                }
            }
            for field in &source.fields {
                validate_graphql_name(&field.name, "schema field")?;
                if field.accepted_types.is_empty() {
                    return graphql_error("GraphQL schema field requires an accepted type");
                }
            }
        }
        if self.digest != schema_digest(self)? {
            return graphql_error("GraphQL schema digest does not match its captured contents");
        }
        Ok(())
    }

    pub(super) fn admits(&self, source: &Source) -> bool {
        let (family, kind) = source_coordinate(source);
        self.sources.iter().any(|candidate| {
            candidate.family == family
                && (candidate.kind.is_none() || candidate.kind.as_deref() == kind)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoweredGraphqlQuery {
    pub schema_digest: String,
    pub response_key: String,
    pub query: Query,
    pub parameters: Parameters,
}
