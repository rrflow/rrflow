//! The single RRFlow engine composition root.
//!
//! Physical storage, query, vector, estate, and security components are
//! composed here. Daemon and embedded faces consume this API instead of
//! constructing those components independently.

#[cfg(feature = "full")]
mod capabilities;
pub mod edge;
#[cfg(feature = "full")]
mod engine;
#[cfg(feature = "full")]
mod operator;
#[cfg(feature = "full")]
pub mod runtime;

#[cfg(feature = "full")]
pub use capabilities::product_capability_catalogue;
pub use edge::{OfflineDocument, OfflineEdgeConfig, OfflineEdgeIndex, OfflineQueryResult};
#[cfg(feature = "full")]
pub use engine::{
    load_estate_configuration, AuthorizedInvocation, ClockAssessment, ClockAssessmentStatus,
    ClockObservation, ClockObservationFailure, ClockSourceKind, ClockTrustLevel,
    DefaultAttunementPhase, DefaultInstallProfile, DistributedAuthorityRead, DistributedReadRoute,
    EstateAdminAction, EstateAdminResult, EstateBackupReconcileOutcome, EstateReconcileOutcome,
    InstallationPreview, InstallationVerificationCheck, InstallationVerificationReport,
    InstallationVerificationStatus, InstalledEstateRecord, Invocation, InvocationCompletion,
    InvocationCredential, PreparedDistributedAuthority, ProjectLocator, Result, RrdEngine,
    RrdOperation, ServiceError, ServiceErrorKind, CLOCK_OBSERVATION_FORMAT_VERSION,
    MAX_AUDIT_PAGE_RECORDS, MAX_JWT_CREDENTIAL_BYTES, TOKEN_KEY_BYTES,
};
#[cfg(feature = "full")]
pub use operator::{
    digest, BackupCatalogue, BackupEntry, Claim, ClaimReader, LogicalArchiveInventory,
    LogicalRestoreReport, Millis, OperatorInvocation, OperatorInvocationInput, OperatorResult,
    Outcome, Predicate, Producer, Reader, RemovalReport, ScopeId, Subject, Trigger,
};
#[cfg(feature = "full")]
pub use runtime::*;
