//! rrflowKV, rrflowMX, archive, and object-storage errors.

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    TransactionLimit {
        resource: rrd_lsm::TransactionResource,
        limit: usize,
        observed: usize,
    },
    DiskAdmission {
        available: u64,
        required: u64,
        reserve: u64,
    },
    /// Propagated from the substrate.
    Substrate(String),
    /// Known admission rejection before this batch's domain publication.
    WriteBatchLimit {
        resource: rrd_lsm::WriteBatchResource,
        limit: u64,
        observed: u64,
    },
    /// Maintenance pressure before the incoming batch is appended.
    RecoveryLimit {
        resource: rrd_lsm::RecoveryResource,
        limit: u64,
        observed: u64,
    },
    ReclamationPending,
    ReclamationLimit {
        resource: rrd_lsm::ReclamationResource,
        limit: u64,
        observed: u64,
    },
    MaintenanceLimit {
        resource: rrd_lsm::MaintenanceResource,
        limit: u64,
        observed: u64,
    },
    /// Active mutable-state admission or read-only WAL materialization exceeded
    /// a process-local native limit. This does not classify unknown I/O errors.
    MutableLimit {
        resource: rrd_lsm::MutableResource,
        limit: usize,
        observed: usize,
    },
    /// Claim encoding or decoding failed.
    Codec(String),
    /// Propagated from the kernel.
    Kernel(rrd_core::Error),
    /// Sequence allocation overflowed. Reported rather than saturated, so that
    /// overflow cannot degrade into silent key reuse. `SPEC.md` §11 correction 2.
    SequenceOverflow,
    /// A recorded watermark was not a valid sequence value.
    CorruptWatermark(String),
    /// A projection was halted by grounding (`SPEC.md` §8.3) and refuses reads
    /// and rebuilds until an operator resets it.
    Quarantined(String),
    /// Optimistic concurrency rejected a writer that observed an older head.
    RuntimeConflict {
        expected: u64,
        actual: u64,
    },
    /// Snapshot isolation rejected a transaction because a key in its write
    /// set changed after the transaction began.
    TransactionConflict {
        snapshot_sequence: u64,
        conflicting_sequence: u64,
    },
    /// One accepted client idempotency key was rebound to different content.
    IdempotencyConflict(String),
    /// A control-plane transition observed different materialized state.
    ControlConflict(String),
    /// A relation or event named a record that is not present in its scope.
    DanglingRuntimeReference(String),
    /// A catalogue-bound retirement named no live value at its effective time.
    RuntimeTargetNotFound(String),
    /// A typed write was attempted before its scope installed a registry.
    RuntimeSchemaMissing(String),
    /// A schema update skipped or repeated a persisted revision.
    RuntimeSchemaConflict {
        expected: u64,
        actual: u64,
    },
    /// A schema-bound native index rejected an invalid definition or write.
    IndexConstraint(String),
    /// Governed-function catalogue or receipt state failed closed.
    FunctionConstraint(String),
    /// A leased snapshot is not present in the authoritative snapshot catalog.
    SnapshotNotFound(String),
    /// A leased snapshot was presented after its expiration instant.
    SnapshotExpired {
        id: String,
        expired_at: u64,
    },
    /// A caller supplied fields that do not match the persisted snapshot with
    /// the same identity.
    SnapshotMismatch(String),
    /// A stamped transaction read names a cursor that is not retained.
    ReadStampUnavailable(String),
    /// A stamped transaction read does not match the retained hash/schema state.
    ReadStampMismatch(String),
    /// A direct semantic read would examine more keys than its declared bound.
    RuntimeReadBudgetExceeded {
        limit: u64,
        observed: u64,
    },
    /// Object-tier I/O or capability error.
    Object(String),
    /// A transport-classified remote failure that the bounded object retry
    /// policy may repeat. All other errors fail immediately.
    RemoteObjectTransient(String),
    /// Logical archive export, validation, or restore failed closed.
    Archive(String),
    /// A referenced immutable object is absent.
    ObjectMissing(String),
    /// Stored bytes do not match their content address.
    ObjectCorrupt {
        expected: String,
        actual: String,
    },
    /// Stored bytes match the digest but not the committed length evidence.
    ObjectLengthMismatch {
        expected: u64,
        actual: u64,
    },
    /// Deterministic failure-injection boundary used by crash tests.
    FaultInjected(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Substrate(m) => write!(f, "substrate: {m}"),
            Error::TransactionLimit { resource, limit, observed } => write!(f, "transaction {} limit {limit} would be exceeded by {observed}", resource.as_str()),
            Error::WriteBatchLimit { resource, limit, observed } => write!(
                f, "write batch {} limit {limit} would be exceeded by {observed}", resource.as_str()
            ),
            Error::RecoveryLimit { resource, limit, observed } => write!(f, "recovery {} limit {limit} would be exceeded by {observed}", resource.as_str()),
            Error::ReclamationPending => write!(f, "bounded reclamation has remaining work; retry before admitting another write"),
            Error::ReclamationLimit { resource, limit, observed } => write!(f, "reclamation {} limit {limit} would be exceeded by {observed}", resource.as_str()),
            Error::DiskAdmission { available, required, reserve } => write!(f, "disk admission requires {required} bytes including {reserve} recovery reserve; observed {available}"),
            Error::MaintenanceLimit { resource, limit, observed } => write!(f, "maintenance {} limit {limit} would be exceeded by {observed}", resource.as_str()),
            Error::MutableLimit { resource, limit, observed } => write!(
                f, "mutable {} limit {limit} would be exceeded by {observed}", resource.as_str()
            ),
            Error::Codec(m) => write!(f, "claim codec: {m}"),
            Error::Kernel(e) => write!(f, "kernel: {e}"),
            Error::SequenceOverflow => write!(f, "sequence allocation overflowed"),
            Error::CorruptWatermark(m) => write!(f, "corrupt sequence watermark: {m}"),
            Error::Quarantined(m) => write!(f, "quarantined: {m}"),
            Error::RuntimeConflict { expected, actual } => write!(
                f,
                "runtime commit conflict: expected cursor {expected}, actual cursor {actual}"
            ),
            Error::TransactionConflict {
                snapshot_sequence,
                conflicting_sequence,
            } => write!(
                f,
                "storage transaction conflict: snapshot sequence {snapshot_sequence}, conflicting write sequence {conflicting_sequence}"
            ),
            Error::IdempotencyConflict(key) => {
                write!(
                    f,
                    "idempotency key is already bound to different content: {key}"
                )
            }
            Error::ControlConflict(key) => write!(f, "control state changed concurrently: {key}"),
            Error::DanglingRuntimeReference(reference) => {
                write!(f, "dangling runtime reference: {reference}")
            }
            Error::RuntimeTargetNotFound(reference) => {
                write!(f, "runtime retirement target is not live: {reference}")
            }
            Error::RuntimeSchemaMissing(scope) => {
                write!(f, "runtime schema is not installed for scope {scope}")
            }
            Error::RuntimeSchemaConflict { expected, actual } => write!(
                f,
                "runtime schema conflict: expected revision {expected}, actual revision {actual}"
            ),
            Error::IndexConstraint(message) => write!(f, "index constraint: {message}"),
            Error::FunctionConstraint(message) => {
                write!(f, "governed-function constraint: {message}")
            }
            Error::SnapshotNotFound(id) => write!(f, "runtime snapshot not found: {id}"),
            Error::SnapshotExpired { id, expired_at } => {
                write!(f, "runtime snapshot {id} expired at {expired_at}")
            }
            Error::SnapshotMismatch(id) => {
                write!(
                    f,
                    "runtime snapshot {id} does not match the persisted lease"
                )
            }
            Error::ReadStampUnavailable(id) => {
                write!(f, "runtime read stamp is not retained: {id}")
            }
            Error::ReadStampMismatch(id) => {
                write!(f, "runtime read stamp does not match retained state: {id}")
            }
            Error::RuntimeReadBudgetExceeded { limit, observed } => write!(
                f,
                "runtime versioned read exceeded key budget {limit}: observed {observed}"
            ),
            Error::Object(message) => write!(f, "object store: {message}"),
            Error::RemoteObjectTransient(message) => {
                write!(f, "transient remote object store: {message}")
            }
            Error::Archive(message) => write!(f, "logical archive: {message}"),
            Error::ObjectMissing(digest) => write!(f, "object missing: {digest}"),
            Error::ObjectCorrupt { expected, actual } => write!(
                f,
                "object corrupt: expected digest {expected}, actual digest {actual}"
            ),
            Error::ObjectLengthMismatch { expected, actual } => write!(
                f,
                "object length mismatch: expected {expected}, actual {actual}"
            ),
            Error::FaultInjected(point) => write!(f, "failure injected at {point}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rrd_lsm::Error> for Error {
    fn from(value: rrd_lsm::Error) -> Self {
        match value {
            rrd_lsm::Error::RecoveryLimit {
                resource,
                limit,
                observed,
            } => Self::RecoveryLimit {
                resource,
                limit,
                observed,
            },
            rrd_lsm::Error::ReclamationPending => Self::ReclamationPending,
            rrd_lsm::Error::ReclamationLimit {
                resource,
                limit,
                observed,
            } => Self::ReclamationLimit {
                resource,
                limit,
                observed,
            },
            rrd_lsm::Error::TransactionLimit {
                resource,
                limit,
                observed,
            } => Self::TransactionLimit {
                resource,
                limit,
                observed,
            },
            rrd_lsm::Error::DiskAdmission {
                available,
                required,
                reserve,
            } => Self::DiskAdmission {
                available,
                required,
                reserve,
            },
            rrd_lsm::Error::MaintenanceLimit {
                resource,
                limit,
                observed,
            } => Self::MaintenanceLimit {
                resource,
                limit,
                observed,
            },
            rrd_lsm::Error::MutableLimit {
                resource,
                limit,
                observed,
            } => Self::MutableLimit {
                resource,
                limit,
                observed,
            },
            rrd_lsm::Error::WriteBatchLimit {
                resource,
                limit,
                observed,
            } => Self::WriteBatchLimit {
                resource,
                limit,
                observed,
            },
            rrd_lsm::Error::TransactionConflict {
                snapshot_sequence,
                conflicting_sequence,
            } => Error::TransactionConflict {
                snapshot_sequence,
                conflicting_sequence,
            },
            other => Error::Substrate(other.to_string()),
        }
    }
}

impl From<rrd_core::Error> for Error {
    fn from(value: rrd_core::Error) -> Self {
        Error::Kernel(value)
    }
}

impl From<crate::key_codec::KeyCodecError> for Error {
    fn from(value: crate::key_codec::KeyCodecError) -> Self {
        Error::Codec(value.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Codec(value.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Object(value.to_string())
    }
}
