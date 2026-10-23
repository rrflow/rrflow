use super::*;

pub type Result<T> = std::result::Result<T, ServiceError>;

#[derive(Debug)]
pub enum ServiceError {
    Contract(String),
    Estate(String),
    Storage(String),
    StorageConflict(String),
    StorageAdmission(String),
    AdmissionCancelled,
    SessionNotFound,
    Unauthenticated,
    SessionExpired,
    TransactionNotFound,
    TransactionExpired,
    TransactionClosed,
    TransactionQuota,
    RenewalQuota,
    IdempotencyConflict,
    InvocationInProgress,
    CommitInProgress,
    WrongScope,
    OperationDigestMismatch,
    Query(String),
    Inference(String),
    InferenceBusy,
    Vector(String),
    VectorPressure(String),
    Changefeed(String),
    Subscription(String),
    SubscriptionNotFound,
    SubscriptionExpired {
        requested: u64,
        retention_floor: u64,
        head: u64,
    },
    SubscriptionLeaseExpired,
    SubscriptionClosed,
    SubscriptionConnectionReplaced,
    SubscriptionBackpressure,
    Backup(String),
    Function(String),
    FunctionLimit(String),
    ConfigurationLimit(String),
    FunctionNotFound,
    FunctionCatalogueRevisionNotFound,
    MemoryTargetNotFound,
    FlightNotFound,
    TaskNotFound,
    TaskClosed,
    SeatNotRepresented,
    Runtime(String),
    PermissionDenied,
    DeadlineExceeded,
    ProjectBindingMismatch,
    ClockUnavailable(ClockObservationFailure),
    ClockRollback {
        observed_at_unix_ms: u64,
        anchor_unix_ms: u64,
        maximum_rollback_ms: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceErrorKind {
    InvalidArgument,
    NotFound,
    Unauthenticated,
    PermissionDenied,
    Conflict,
    FailedPrecondition,
    ResourceExhausted,
    DeadlineExceeded,
    Internal,
}

impl ServiceError {
    pub fn kind(&self) -> ServiceErrorKind {
        match self {
            Self::Contract(_)
            | Self::Changefeed(_)
            | Self::Subscription(_)
            | Self::Query(_)
            | Self::Inference(_)
            | Self::Function(_)
            | Self::Runtime(_)
            | Self::Vector(_)
            | Self::OperationDigestMismatch
            | Self::WrongScope => ServiceErrorKind::InvalidArgument,
            Self::SessionNotFound
            | Self::TransactionNotFound
            | Self::SubscriptionNotFound
            | Self::FunctionNotFound
            | Self::MemoryTargetNotFound
            | Self::FlightNotFound
            | Self::TaskNotFound => ServiceErrorKind::NotFound,
            Self::Unauthenticated => ServiceErrorKind::Unauthenticated,
            Self::PermissionDenied => ServiceErrorKind::PermissionDenied,
            Self::IdempotencyConflict | Self::InvocationInProgress => ServiceErrorKind::Conflict,
            Self::SessionExpired
            | Self::TransactionExpired
            | Self::TransactionClosed
            | Self::CommitInProgress
            | Self::SubscriptionExpired { .. }
            | Self::SubscriptionLeaseExpired
            | Self::SubscriptionClosed
            | Self::TaskClosed
            | Self::SubscriptionConnectionReplaced
            | Self::FunctionCatalogueRevisionNotFound
            | Self::SeatNotRepresented
            | Self::ClockUnavailable(_)
            | Self::ClockRollback { .. }
            | Self::AdmissionCancelled
            | Self::ProjectBindingMismatch => ServiceErrorKind::FailedPrecondition,
            Self::TransactionQuota
            | Self::StorageAdmission(_)
            | Self::InferenceBusy
            | Self::RenewalQuota
            | Self::SubscriptionBackpressure
            | Self::FunctionLimit(_)
            | Self::ConfigurationLimit(_)
            | Self::VectorPressure(_) => ServiceErrorKind::ResourceExhausted,
            Self::DeadlineExceeded => ServiceErrorKind::DeadlineExceeded,
            Self::StorageConflict(_) => ServiceErrorKind::Conflict,
            Self::Storage(_) | Self::Backup(_) | Self::Estate(_) => ServiceErrorKind::Internal,
        }
    }

    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::StorageConflict(_) | Self::InvocationInProgress | Self::InferenceBusy
        )
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClockUnavailable(failure) => {
                write!(f, "host clock is unavailable: {failure:?}")
            }
            Self::ClockRollback {
                observed_at_unix_ms,
                anchor_unix_ms,
                maximum_rollback_ms,
            } => write!(
                f,
                "host clock rollback exceeds policy: observed {observed_at_unix_ms}, installation anchor {anchor_unix_ms}, maximum rollback {maximum_rollback_ms} ms"
            ),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<rrd_store::Error> for ServiceError {
    fn from(value: rrd_store::Error) -> Self {
        let message = value.to_string();
        match value {
            // Transaction-owner exhaustion can occur while resolving an already
            // published commit. Only a proven-absent outcome permits rejection.
            rrd_store::Error::TransactionLimit { .. } => Self::Storage(message),
            rrd_store::Error::WriteBatchLimit { .. }
            | rrd_store::Error::MutableLimit { .. }
            | rrd_store::Error::MaintenanceLimit { .. }
            | rrd_store::Error::DiskAdmission { .. }
            | rrd_store::Error::RecoveryLimit { .. }
            | rrd_store::Error::ReclamationPending
            | rrd_store::Error::ReclamationLimit { .. } => Self::StorageAdmission(message),
            rrd_store::Error::RuntimeReadBudgetExceeded { .. } => Self::ConfigurationLimit(message),
            rrd_store::Error::IndexConstraint(_) => Self::Query(message),
            rrd_store::Error::RuntimeConflict { .. }
            | rrd_store::Error::RuntimeSchemaConflict { .. }
            | rrd_store::Error::IdempotencyConflict(_)
            | rrd_store::Error::ControlConflict(_)
            | rrd_store::Error::ReadStampMismatch(_) => Self::StorageConflict(message),
            _ => Self::Storage(message),
        }
    }
}

impl From<rrd_estate::Error> for ServiceError {
    fn from(value: rrd_estate::Error) -> Self {
        Self::Estate(value.to_string())
    }
}

impl From<rrd_security::Error> for ServiceError {
    fn from(value: rrd_security::Error) -> Self {
        match value {
            rrd_security::Error::Unauthenticated | rrd_security::Error::PrincipalNotFound => {
                Self::Unauthenticated
            }
            rrd_security::Error::PermissionDenied | rrd_security::Error::NotInitialized => {
                Self::PermissionDenied
            }
            rrd_security::Error::Invalid(message) => Self::Contract(message),
            rrd_security::Error::IdempotencyConflict | rrd_security::Error::AlreadyInitialized => {
                Self::IdempotencyConflict
            }
            rrd_security::Error::Store(error) => Self::from(error),
        }
    }
}
