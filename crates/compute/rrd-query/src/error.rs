use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Catalog(String),
    Graphql(String),
    Binding(String),
    Budget(String),
    StorageAdmission(String),
    Execution(String),
    Integrity(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, detail) = match self {
            Self::Catalog(value) => ("catalog", value),
            Self::Graphql(value) => ("GraphQL adapter", value),
            Self::Binding(value) => ("binding", value),
            Self::Budget(value) => ("budget", value),
            Self::StorageAdmission(value) => ("storage admission", value),
            Self::Execution(value) => ("execution", value),
            Self::Integrity(value) => ("integrity", value),
        };
        write!(formatter, "RRD query engine {kind} error: {detail}")
    }
}

impl std::error::Error for Error {}

impl From<rrd_store::Error> for Error {
    fn from(value: rrd_store::Error) -> Self {
        if matches!(
            value,
            rrd_store::Error::SegmentHandleLimit { .. }
                | rrd_store::Error::SourcePageLimit { .. }
                | rrd_store::Error::ProjectedReadLimit { .. }
        ) {
            Self::StorageAdmission(value.to_string())
        } else if matches!(value, rrd_store::Error::RuntimeReadBudgetExceeded { .. }) {
            Self::Budget(value.to_string())
        } else {
            Self::Execution(value.to_string())
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Execution(value.to_string())
    }
}
