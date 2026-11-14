//! The rrflowQL syntax tree: queries, joins, sources, selectors, filters, projections and transaction programs.

use super::*;

pub const QUERY_CONTRACT_VERSION: u16 = 1;
pub const MAX_TRAVERSAL_DEPTH: u16 = 32;
pub const MAX_TRANSACTION_STATEMENTS: usize = 256;
pub const MAX_TRANSACTION_PROGRAM_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    pub contract_version: u16,
    pub source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join: Option<Join>,
    pub temporal: TemporalSelector,
    #[serde(default)]
    pub filters: Vec<Filter>,
    pub projection: Projection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default)]
    pub explain_contract: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub explain_analyze: bool,
}

impl Query {
    pub fn new(source: Source, temporal: TemporalSelector) -> Self {
        Self {
            contract_version: QUERY_CONTRACT_VERSION,
            source,
            join: None,
            temporal,
            filters: Vec::new(),
            projection: Projection::All,
            limit: None,
            explain_contract: false,
            explain_analyze: false,
        }
    }

    pub fn validate(&self) -> Result<(), ParseError> {
        if self.contract_version != QUERY_CONTRACT_VERSION {
            return Err(ParseError::new(
                0,
                format!(
                    "unsupported query contract version {}",
                    self.contract_version
                ),
            ));
        }
        if self.limit == Some(0) {
            return Err(ParseError::new(0, "LIMIT must be greater than zero"));
        }
        if self.explain_contract && self.explain_analyze {
            return Err(ParseError::new(
                0,
                "EXPLAIN CONTRACT and EXPLAIN ANALYZE are mutually exclusive",
            ));
        }
        if matches!(
            &self.source,
            Source::Traversal { max_depth, .. }
                if !(1..=MAX_TRAVERSAL_DEPTH).contains(max_depth)
        ) {
            return Err(ParseError::new(
                0,
                format!("traversal depth must be in 1..={MAX_TRAVERSAL_DEPTH}"),
            ));
        }
        if let Some(join) = &self.join {
            validate_field(&join.left_field, 0)?;
            validate_field(&join.right_field, 0)?;
        }
        if let Projection::Fields(fields) = &self.projection {
            if fields.is_empty() {
                return Err(ParseError::new(0, "PROJECT requires at least one field"));
            }
            for field in fields {
                validate_field(field, 0)?;
            }
        }
        for filter in &self.filters {
            validate_field(&filter.field, 0)?;
            if let ValueExpr::Literal(value) = &filter.value {
                if !matches!(
                    value,
                    RuntimeValue::Null
                        | RuntimeValue::Bool(_)
                        | RuntimeValue::Integer(_)
                        | RuntimeValue::Unsigned(_)
                        | RuntimeValue::String(_)
                ) {
                    return Err(ParseError::new(
                        0,
                        "this query contract only supports scalar filter literals",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn canonical(&self) -> String {
        let mut out = format!("FROM {}", self.source.canonical());
        if let Some(join) = &self.join {
            out.push_str(" JOIN ");
            out.push_str(&join.source.canonical());
            out.push_str(" ON ");
            out.push_str(&join.left_field);
            out.push_str(" = ");
            out.push_str(&join.right_field);
        }
        out.push_str(" AT VALID ");
        out.push_str(&self.temporal.valid_at.canonical());
        out.push_str(" KNOWN ");
        out.push_str(&self.temporal.known_at.canonical());
        if !self.filters.is_empty() {
            out.push_str(" WHERE ");
            for (index, filter) in self.filters.iter().enumerate() {
                if index > 0 {
                    out.push_str(" AND ");
                }
                out.push_str(&filter.field);
                out.push(' ');
                out.push_str(filter.comparison.canonical());
                out.push(' ');
                out.push_str(&filter.value.canonical());
            }
        }
        out.push_str(" PROJECT ");
        match &self.projection {
            Projection::All => out.push('*'),
            Projection::Fields(fields) => out.push_str(&fields.join(", ")),
        }
        if let Some(limit) = self.limit {
            out.push_str(&format!(" LIMIT {limit}"));
        }
        if self.explain_contract {
            out.push_str(" EXPLAIN CONTRACT");
        } else if self.explain_analyze {
            out.push_str(" EXPLAIN ANALYZE");
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Join {
    pub source: Source,
    pub left_field: String,
    pub right_field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case")]
pub enum Source {
    Record {
        kind: RuntimeType,
    },
    Relation {
        kind: RuntimeType,
    },
    Event {
        kind: RuntimeType,
    },
    Series {
        kind: RuntimeType,
    },
    Geo {
        kind: RuntimeType,
    },
    Traversal {
        relation: RuntimeType,
        start: RuntimeRef,
        direction: TraversalDirection,
        max_depth: u16,
    },
    Claim {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        predicate: Option<Predicate>,
    },
}

impl Source {
    fn canonical(&self) -> String {
        match self {
            Self::Record { kind } => format!("record:{kind}"),
            Self::Relation { kind } => format!("relation:{kind}"),
            Self::Event { kind } => format!("event:{kind}"),
            Self::Series { kind } => format!("series:{kind}"),
            Self::Geo { kind } => format!("geo:{kind}"),
            Self::Traversal {
                relation,
                start,
                direction,
                max_depth,
            } => format!(
                "traverse:{relation} START {}:{} DIRECTION {} DEPTH {max_depth}",
                start.kind,
                traversal_id(start.id.as_str()),
                direction.canonical()
            ),
            Self::Claim {
                predicate: Some(predicate),
            } => format!("claim:{predicate}"),
            Self::Claim { predicate: None } => "claim".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraversalDirection {
    Outgoing,
    Incoming,
    Both,
}

impl TraversalDirection {
    fn canonical(self) -> &'static str {
        match self {
            Self::Outgoing => "OUTGOING",
            Self::Incoming => "INCOMING",
            Self::Both => "BOTH",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalSelector {
    pub valid_at: TimeExpr,
    pub known_at: CursorExpr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TimeExpr {
    Literal(u64),
    Parameter(String),
}

impl TimeExpr {
    fn canonical(&self) -> String {
        match self {
            Self::Literal(value) => value.to_string(),
            Self::Parameter(value) => format!("${value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum CursorExpr {
    Head,
    Literal(u64),
    Parameter(String),
}

impl CursorExpr {
    fn canonical(&self) -> String {
        match self {
            Self::Head => "HEAD".into(),
            Self::Literal(value) => value.to_string(),
            Self::Parameter(value) => format!("${value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    #[serde(default, skip_serializing_if = "ComparisonOperator::is_equal")]
    pub comparison: ComparisonOperator,
    pub value: ValueExpr,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    #[default]
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Match,
}

impl ComparisonOperator {
    pub fn is_equal(&self) -> bool {
        *self == Self::Equal
    }

    pub fn is_ordering(self) -> bool {
        matches!(
            self,
            Self::LessThan | Self::LessThanOrEqual | Self::GreaterThan | Self::GreaterThanOrEqual
        )
    }

    pub fn is_match(self) -> bool {
        self == Self::Match
    }

    fn canonical(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::LessThan => "<",
            Self::LessThanOrEqual => "<=",
            Self::GreaterThan => ">",
            Self::GreaterThanOrEqual => ">=",
            Self::Match => "MATCH",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ValueExpr {
    Literal(RuntimeValue),
    Parameter(String),
}

impl ValueExpr {
    fn canonical(&self) -> String {
        match self {
            Self::Parameter(value) => format!("${value}"),
            Self::Literal(RuntimeValue::Null) => "null".into(),
            Self::Literal(RuntimeValue::Bool(value)) => value.to_string(),
            Self::Literal(RuntimeValue::Integer(value)) => value.to_string(),
            Self::Literal(RuntimeValue::Unsigned(value)) => value.to_string(),
            Self::Literal(RuntimeValue::String(value)) => quote(value),
            Self::Literal(value) => quote(&format!("{value:?}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "fields", rename_all = "snake_case")]
pub enum Projection {
    All,
    Fields(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionDisposition {
    Commit,
    Cancel,
}

/// A bounded rrflowQL transaction program. Mutation payloads remain typed
/// bindings so the language never invents a second serialization contract for
/// the canonical transaction mutation vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionProgram {
    pub contract_version: u16,
    pub mutation_bindings: Vec<String>,
    pub disposition: TransactionDisposition,
}

impl TransactionProgram {
    pub fn validate(&self) -> Result<(), ParseError> {
        if self.contract_version != QUERY_CONTRACT_VERSION {
            return Err(ParseError::new(
                0,
                format!(
                    "unsupported transaction-program contract version {}",
                    self.contract_version
                ),
            ));
        }
        if self.mutation_bindings.is_empty()
            || self.mutation_bindings.len() > MAX_TRANSACTION_STATEMENTS
        {
            return Err(ParseError::new(
                0,
                format!("transaction program requires 1..={MAX_TRANSACTION_STATEMENTS} mutations"),
            ));
        }
        let mut unique = std::collections::BTreeSet::new();
        for binding in &self.mutation_bindings {
            validate_field(binding, 0)?;
            if !unique.insert(binding) {
                return Err(ParseError::new(
                    0,
                    format!("duplicate mutation binding ${binding}"),
                ));
            }
        }
        Ok(())
    }

    pub fn canonical(&self) -> String {
        let mut statements = Vec::with_capacity(self.mutation_bindings.len() + 2);
        statements.push("BEGIN".to_owned());
        statements.extend(
            self.mutation_bindings
                .iter()
                .map(|binding| format!("MUTATE ${binding}")),
        );
        statements.push(
            match self.disposition {
                TransactionDisposition::Commit => "COMMIT",
                TransactionDisposition::Cancel => "CANCEL",
            }
            .to_owned(),
        );
        format!("{};", statements.join("; "))
    }
}
