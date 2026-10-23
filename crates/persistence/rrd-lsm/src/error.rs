use crate::segment::ProjectedReadResource;
use std::fmt;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    TransactionLimit {
        resource: crate::TransactionResource,
        limit: usize,
        observed: usize,
    },
    DiskAdmission {
        available: u64,
        required: u64,
        reserve: u64,
    },
    Io(std::io::Error),
    IoContext {
        operation: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    Corruption {
        offset: u64,
        reason: String,
    },
    UnsupportedVersion {
        object: &'static str,
        version: u16,
    },
    /// Maintenance pressure before the incoming batch is appended.
    RecoveryLimit {
        resource: crate::RecoveryResource,
        limit: u64,
        observed: u64,
    },
    ReclamationPending,
    ReclamationLimit {
        resource: crate::ReclamationResource,
        limit: u64,
        observed: u64,
    },
    MaintenanceLimit {
        resource: crate::MaintenanceResource,
        limit: u64,
        observed: u64,
    },
    MutableLimit {
        resource: crate::MutableResource,
        limit: usize,
        observed: usize,
    },
    InvalidConfiguration(String),
    InvalidBatch(String),
    /// Admission failed before this batch could reach a native effect.
    WriteBatchLimit {
        resource: crate::batch::WriteBatchResource,
        limit: u64,
        observed: u64,
    },
    InvalidManifest(String),
    ManifestConflict {
        expected: Option<String>,
        actual: Option<String>,
    },
    InvalidSegment(String),
    InvalidProjectedRead(String),
    InvalidProjectedReadBudget {
        resource: ProjectedReadResource,
    },
    ProjectedReadLimit {
        resource: ProjectedReadResource,
        limit: u64,
        observed: u64,
    },
    DuplicateProjectedSequence {
        manifest: String,
        sequence: u64,
    },
    PoisonedReadViewRegistry,
    TornTail {
        offset: u64,
    },
    DatabaseWriterLock {
        path: PathBuf,
    },
    RecoveryRequired {
        boundary: &'static str,
    },
    InvalidTransaction(String),
    TransactionConflict {
        snapshot_sequence: u64,
        conflicting_sequence: u64,
    },
    TransactionDatabaseMismatch,
    ActiveTransactions {
        operation: &'static str,
    },
    PoisonedTransactionState,
    PoisonedWriter,
    InjectedFailure {
        mode: &'static str,
        boundary: &'static str,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TransactionLimit { resource, limit, observed } => write!(formatter, "transaction {} limit {limit} would be exceeded by {observed}", resource.as_str()),
            Self::Io(error) => write!(formatter, "RRD LSM I/O: {error}"),
            Self::IoContext {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "RRD LSM I/O while {operation} {}: {source}",
                path.display()
            ),
            Self::Corruption { offset, reason } => {
                write!(formatter, "RRD LSM corruption at byte {offset}: {reason}")
            }
            Self::UnsupportedVersion { object, version } => {
                write!(formatter, "unsupported {object} format version {version}")
            }
            Self::RecoveryLimit { resource, limit, observed } => write!(formatter, "recovery {} limit {limit} would be exceeded by {observed}; reopen with explicitly configured capacity", resource.as_str()),
            Self::ReclamationPending => write!(formatter, "bounded reclamation has remaining work; retry before admitting another write"),
            Self::ReclamationLimit { resource, limit, observed } => write!(formatter, "reclamation {} limit {limit} would be exceeded by {observed}", resource.as_str()),
            Self::DiskAdmission { available, required, reserve } => write!(formatter, "disk admission requires {required} bytes including {reserve} recovery reserve; observed {available}"),
            Self::MaintenanceLimit { resource, limit, observed } => write!(formatter, "maintenance {} limit {limit} would be exceeded by {observed}", resource.as_str()),
            Self::MutableLimit { resource, limit, observed } => write!(formatter, "mutable {} limit {limit} would be exceeded by {observed}", resource.as_str()),
            Self::InvalidConfiguration(reason) => {
                write!(formatter, "invalid RRD LSM configuration: {reason}")
            }
            Self::InvalidBatch(reason) => write!(formatter, "invalid WAL batch: {reason}"),
            Self::WriteBatchLimit { resource, limit, observed } => write!(
                formatter, "write batch {} limit {limit} would be exceeded by {observed}", resource.as_str()
            ),
            Self::InvalidManifest(reason) => write!(formatter, "invalid manifest: {reason}"),
            Self::ManifestConflict { expected, actual } => write!(
                formatter,
                "manifest compare-and-swap conflict: expected {expected:?}, actual {actual:?}"
            ),
            Self::InvalidSegment(reason) => write!(formatter, "invalid segment: {reason}"),
            Self::InvalidProjectedRead(reason) => {
                write!(formatter, "invalid projected read: {reason}")
            }
            Self::InvalidProjectedReadBudget { resource } => write!(
                formatter,
                "invalid projected-read budget: {} must be greater than zero",
                resource.as_str()
            ),
            Self::ProjectedReadLimit {
                resource,
                limit,
                observed,
            } => write!(
                formatter,
                "projected-read {} limit {limit} would be exceeded by {observed}",
                resource.as_str()
            ),
            Self::DuplicateProjectedSequence { manifest, sequence } => write!(
                formatter,
                "projected read of manifest {manifest} found sequence {sequence} more than once for one key"
            ),
            Self::PoisonedReadViewRegistry => {
                write!(formatter, "projected-read view registry is poisoned")
            }
            Self::TornTail { offset } => write!(
                formatter,
                "WAL has an incomplete tail at byte {offset}; explicit repair is required"
            ),
            Self::DatabaseWriterLock { path } => write!(
                formatter,
                "RRD LSM database already has an active writer: {}",
                path.display()
            ),
            Self::RecoveryRequired { boundary } => write!(
                formatter,
                "RRD LSM writer crossed {boundary} with unresolved durable state; reopen is required"
            ),
            Self::InvalidTransaction(reason) => {
                write!(formatter, "invalid RRD LSM transaction: {reason}")
            }
            Self::TransactionConflict {
                snapshot_sequence,
                conflicting_sequence,
            } => write!(
                formatter,
                "RRD LSM transaction conflict: snapshot sequence {snapshot_sequence}, conflicting write sequence {conflicting_sequence}"
            ),
            Self::TransactionDatabaseMismatch => write!(
                formatter,
                "RRD LSM transaction belongs to a different database instance"
            ),
            Self::ActiveTransactions { operation } => write!(
                formatter,
                "RRD LSM cannot perform {operation} while transactions hold read snapshots"
            ),
            Self::PoisonedTransactionState => {
                write!(formatter, "RRD LSM transaction snapshot registry is poisoned")
            }
            Self::PoisonedWriter => write!(
                formatter,
                "WAL writer is poisoned after a failed write or sync and must be reopened"
            ),
            Self::InjectedFailure { mode, boundary } => {
                write!(formatter, "injected {mode} failure at {boundary}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::IoContext { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl Error {
    pub(crate) fn io(
        operation: &'static str,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        Self::IoContext {
            operation,
            path: path.into(),
            source,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidManifest(value.to_string())
    }
}
