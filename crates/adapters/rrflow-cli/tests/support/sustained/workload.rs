//! Installed/public-contract mixed-family corpus; expected values are generated
//! independently from request coordinates, never copied out of engine answers.
use rrd_client::{RequestOptions, RrdClient, Session};
use rrd_contract::*;
use rrflow_qualification_storage::{
    digest, Digest, ReadKeyDistribution, StorageLoadKind, WriteTransition,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

pub(super) const WORKLOAD_RECORD_KIND: &str = "capacity_document";

type Result<T> = std::result::Result<T, String>;

#[test]
fn slow_write_phase_trace_is_bounded_and_private() {
    let counter = AtomicUsize::new(0);
    assert_eq!(
        admit_write_trace(&counter, "load-secret", 199_999_999),
        WriteTraceAdmission::Skip
    );
    assert_eq!(
        admit_write_trace(&counter, "seed-secret", 200_000_000),
        WriteTraceAdmission::Skip
    );
    let results = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    (0..16)
                        .map(|_| admit_write_trace(&counter, "load-secret", 200_000_000))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        results
            .iter()
            .filter(|value| **value == WriteTraceAdmission::Record)
            .count(),
        64
    );
    assert_eq!(
        results
            .iter()
            .filter(|value| **value == WriteTraceAdmission::Exhausted)
            .count(),
        1
    );
    let begin_id = "load-secret-begin";
    let line = format_write_trace(
        "load-secret",
        begin_id,
        123,
        456,
        600,
        WriteTraceOutcome::Accepted,
    );
    assert!(line.contains(&rrd_core::digest::sha256_hex(b"load-secret")));
    let actual = options(begin_id);
    assert!(line.contains(&format!(
        "begin_request_sha256={}",
        rrd_core::digest::sha256_hex(actual.request_id.as_str().as_bytes())
    )));
    assert!(line.contains("begin_ns=123 commit_ns=456 total_ns=600 outcome=accepted"));
    assert!(!line.contains("load-secret"));
}

const SLOW_WRITE_NS: u64 = 200_000_000;
const MAX_WRITE_TRACES: usize = 64;
static WRITE_TRACES: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Eq)]
enum WriteTraceAdmission {
    Skip,
    Record,
    Exhausted,
}

#[derive(Clone, Copy)]
enum WriteTraceOutcome {
    Accepted,
    CommitError,
    BeginError,
}

fn admit_write_trace(counter: &AtomicUsize, key: &str, total_ns: u64) -> WriteTraceAdmission {
    if !key.starts_with("load-") || total_ns < SLOW_WRITE_NS {
        return WriteTraceAdmission::Skip;
    }
    match counter.try_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
        (count <= MAX_WRITE_TRACES).then_some(count + 1)
    }) {
        Ok(count) if count < MAX_WRITE_TRACES => WriteTraceAdmission::Record,
        Ok(_) => WriteTraceAdmission::Exhausted,
        Err(_) => WriteTraceAdmission::Skip,
    }
}

fn format_write_trace(
    key: &str,
    begin_request_id: &str,
    begin_ns: u64,
    commit_ns: u64,
    total_ns: u64,
    outcome: WriteTraceOutcome,
) -> String {
    let outcome = match outcome {
        WriteTraceOutcome::Accepted => "accepted",
        WriteTraceOutcome::CommitError => "commit-error",
        WriteTraceOutcome::BeginError => "begin-error",
    };
    format!(
        "c07_write_phase_trace request_sha256={} begin_ns={begin_ns} commit_ns={commit_ns} total_ns={total_ns} outcome={outcome} begin_request_sha256={}",
        rrd_core::digest::sha256_hex(key.as_bytes()),
        rrd_core::digest::sha256_hex(begin_request_id.as_bytes())
    )
}

fn record_write_trace(
    key: &str,
    begin_request_id: &str,
    begin_ns: u64,
    commit_ns: u64,
    total_ns: u64,
    outcome: WriteTraceOutcome,
) {
    match admit_write_trace(&WRITE_TRACES, key, total_ns) {
        WriteTraceAdmission::Skip => {}
        WriteTraceAdmission::Record => {
            eprintln!(
                "{}",
                format_write_trace(
                    key,
                    begin_request_id,
                    begin_ns,
                    commit_ns,
                    total_ns,
                    outcome
                )
            )
        }
        WriteTraceAdmission::Exhausted => {
            eprintln!("c07_write_phase_trace_exhausted limit={MAX_WRITE_TRACES}")
        }
    }
}

fn elapsed_ns(started: Instant) -> u64 {
    started.elapsed().as_nanos().try_into().unwrap_or(u64::MAX)
}

#[derive(Clone)]
pub(super) struct Workload {
    pub native_worker: super::native_worker::Owner,
    pub client: RrdClient,
    pub session: Session,
    pub valid_at: u64,
    pub dimensions: u32,
    pub seed: u64,
    pub records: u64,
    pub read_distribution: ReadKeyDistribution,
    pub edges_per_record: u32,
    pub vectors_per_record: u32,
    pub payload_bytes: usize,
    pub query_budget: QueryBudget,
}

#[derive(Clone)]
pub(super) struct Prepared {
    pub id: String,
    pub kind: StorageLoadKind,
    pub request_sha256: Digest,
    pub effect_sha256: Digest,
    pub mutation: Option<CommitTransaction>,
    pub revision: u64,
    pub key: String,
    pub write: Option<WriteTransition>,
}

pub(super) struct Completed {
    pub receipt_sha256: Digest,
    pub idempotent_replay: bool,
}

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).expect("bounded corpus identifier")
}
fn reference(kind: &str, value: &str) -> DataReference {
    DataReference {
        kind: id(kind),
        id: id(value),
    }
}
fn options(value: &str) -> RequestOptions {
    RequestOptions::new(value, value, Some(value), Some(u64::MAX)).unwrap()
}
fn read_options(family: &str) -> Result<RequestOptions> {
    static NEXT_READ: AtomicU64 = AtomicU64::new(0);
    let ordinal = NEXT_READ
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| "read invocation counter exhausted")?;
    let name = format!("{family}-{ordinal:016x}");
    RequestOptions::read(&name, &name).map_err(error)
}

fn check(value: bool, reason: &str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(reason.into())
    }
}
fn error(value: impl std::fmt::Display) -> String {
    value.to_string()
}

// The conflict subtype is a diagnostic hint, not an authorization decision or
// evidence that any effect committed. Numeric forms must match the closed
// native error grammar; no server-supplied content is returned.
fn conflict_numeric_pair(detail: &str, prefix: &str, separator: &str) -> bool {
    let Some((expected, actual)) = detail
        .strip_prefix(prefix)
        .and_then(|rest| rest.split_once(separator))
    else {
        return false;
    };
    [expected, actual].into_iter().all(|value| {
        value
            .parse::<u64>()
            .is_ok_and(|number| number.to_string() == value)
    })
}

// This is diagnostic classification only; it does not infer publication or
// retry effects. Never copy dynamic server text into the evidence stream.
fn structured_snapshot_conflict(error: &ErrorBody) -> Option<(u64, u64)> {
    if error.code != ErrorCode::Conflict || error.retryable || error.details.len() != 4 {
        return None;
    }
    let value = |name: &str| error.details.get(&CanonicalId::new(name).ok()?);
    if value("reason")?.as_str() != "transaction-snapshot-stale"
        || value("next-action")?.as_str() != "abort-and-reexecute-transaction"
    {
        return None;
    }
    let expected = value("expected-cursor")?;
    let actual = value("actual-cursor")?;
    let expected_number = expected.parse::<u64>().ok()?;
    let actual_number = actual.parse::<u64>().ok()?;
    (expected_number.to_string() == *expected
        && actual_number.to_string() == *actual
        && actual_number > expected_number)
        .then_some((expected_number, actual_number))
}

// Only closed numeric cursor values and a one-way synthetic request digest are
// retained. A reported conflict is not proof of effect or of its internal origin.
pub(super) fn runtime_conflict_trace(
    error: &rrd_client::Error,
    key: &str,
    read_cursor: u64,
    counter: &AtomicUsize,
) -> Option<String> {
    let rrd_client::Error::Api { error, .. } = error else {
        return None;
    };
    if error.code != ErrorCode::Conflict {
        return None;
    }
    let (expected_number, actual_number) = if let Some(pair) = structured_snapshot_conflict(error) {
        pair
    } else {
        let detail = error
            .message
            .strip_prefix("StorageConflict(\"runtime commit conflict: expected cursor ")?
            .strip_suffix("\")")?;
        let (expected, actual) = detail.split_once(", actual cursor ")?;
        let expected_number = expected.parse::<u64>().ok()?;
        let actual_number = actual.parse::<u64>().ok()?;
        if expected_number.to_string() != expected || actual_number.to_string() != actual {
            return None;
        }
        (expected_number, actual_number)
    };
    counter
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
            (count < 32).then_some(count + 1)
        })
        .ok()?;
    Some(format!(
        "c07_runtime_conflict_trace request_sha256={} read_cursor={} expected_cursor={} actual_cursor={}",
        rrd_core::digest::sha256_hex(key.as_bytes()), read_cursor, expected_number, actual_number
    ))
}

// Diagnostic-only projection of the current engine's error spelling. This is
// never used to infer effects or retryability. Return constants, not message
// slices: even an arbitrary server body can disclose only an allowlisted label.
fn engine_error_label(message: &str) -> Option<&'static str> {
    if message == "Storage(\"substrate: control state exceeds one MiB\")" {
        return Some("control-state-size");
    }
    // Read pressure may be wrapped as Internal storage or ResourceExhausted
    // storage admission depending on the engine boundary. Labels never infer
    // durable effects; abort/readback evidence remains authoritative.
    if let Some(detail) = message
        .strip_prefix("Storage(\"")
        .or_else(|| message.strip_prefix("StorageAdmission(\""))
    {
        for (prefix, label) in [
            ("projected-read active_views limit ", "read-active-views"),
            ("projected-read ranges limit ", "read-ranges"),
            ("projected-read runs limit ", "read-runs"),
            ("projected-read pinned_bytes limit ", "read-pinned-bytes"),
            ("projected-read versions_examined limit ", "read-versions"),
            ("projected-read page_requests limit ", "read-page-requests"),
            (
                "projected-read page_logical_bytes limit ",
                "read-page-logical-bytes",
            ),
            ("projected-read output_rows limit ", "read-output-rows"),
            (
                "projected-read output_buffer_bytes limit ",
                "read-output-buffer-bytes",
            ),
            ("projected-read batch_rows limit ", "read-batch-rows"),
            (
                "projected-read batch_allocated_bytes limit ",
                "read-batch-allocated-bytes",
            ),
            (
                "projected-read retained_output_bytes limit ",
                "read-retained-output-bytes",
            ),
            ("source page allocation limit ", "read-source-pages"),
            ("segment open-file limit ", "read-segment-handles"),
        ] {
            if detail.starts_with(prefix) {
                return Some(label);
            }
        }
    }
    if let Some(detail) = message.strip_prefix("Storage(\"") {
        for (prefix, label) in [
            ("transaction active owners limit ", "transaction-active"),
            (
                "transaction owned buffer bytes limit ",
                "transaction-buffered-bytes",
            ),
            ("substrate: ", "storage-substrate"),
            ("claim codec: ", "storage-codec"),
            ("kernel: ", "storage-kernel"),
            ("corrupt sequence watermark: ", "storage-watermark"),
        ] {
            if detail.starts_with(prefix) {
                return Some(label);
            }
        }
    }
    if let Some(detail) = message
        .strip_prefix("StorageConflict(\"")
        .and_then(|detail| detail.strip_suffix("\")"))
    {
        if conflict_numeric_pair(
            detail,
            "runtime commit conflict: expected cursor ",
            ", actual cursor ",
        ) {
            return Some("storage-runtime-conflict");
        }
        if conflict_numeric_pair(
            detail,
            "runtime schema conflict: expected revision ",
            ", actual revision ",
        ) {
            return Some("storage-schema-conflict");
        }
        for (prefix, label) in [
            (
                "idempotency key is already bound to different content: ",
                "storage-idempotency-conflict",
            ),
            (
                "control state changed concurrently: ",
                "storage-control-conflict",
            ),
            (
                "runtime read stamp does not match retained state: ",
                "storage-read-stamp-conflict",
            ),
        ] {
            if detail.starts_with(prefix) {
                return Some(label);
            }
        }
    }
    if let Some(detail) = message.strip_prefix("StorageAdmission(\"") {
        for (prefix, label) in [
            ("maintenance input_bytes limit ", "maintenance-input-bytes"),
            (
                "maintenance input_logical_bytes limit ",
                "maintenance-input-logical",
            ),
            (
                "maintenance input_versions limit ",
                "maintenance-input-versions",
            ),
            (
                "maintenance output_segments limit ",
                "maintenance-output-segments",
            ),
            ("maintenance elapsed_ms limit ", "maintenance-elapsed"),
            ("maintenance l0_segments limit ", "maintenance-l0-segments"),
            ("checkpoint entry limit ", "storage-admission-checkpoint"),
            ("transaction admission ", "storage-admission-queue"),
            ("transaction request ", "storage-admission-request"),
            ("transaction ", "storage-admission-transaction"),
            ("write batch ", "storage-admission-batch"),
            ("mutable ", "storage-admission-mutable"),
            ("maintenance ", "storage-admission-maintenance"),
            ("recovery ", "storage-admission-recovery"),
            ("reclamation ", "storage-admission-reclamation"),
            (
                "bounded reclamation ",
                "storage-admission-reclamation-pending",
            ),
            ("disk admission ", "storage-admission-disk"),
        ] {
            if detail.starts_with(prefix) {
                return Some(label);
            }
        }
    }
    let variant = message.split_once('(').map_or(message, |(name, _)| name);
    match variant {
        "Storage" => Some("storage"),
        "TransactionQuota" => Some("transaction-quota"),
        "RenewalQuota" => Some("renewal-quota"),
        "StorageAdmission" => Some("storage-admission"),
        "ConfigurationLimit" => Some("configuration-limit"),
        "StorageConflict" => Some("storage-conflict"),
        "InvocationInProgress" => Some("invocation-in-progress"),
        "IdempotencyConflict" => Some("idempotency-conflict"),
        "CommitInProgress" => Some("commit-in-progress"),
        _ => None,
    }
}

// Accept only the exact closed native error grammar. Untrusted text is never
// echoed; even a valid prefix with an added suffix must not be exported.
fn pinned_read_denial(message: &str) -> Option<(u64, u64)> {
    let detail = message
        .strip_prefix("StorageAdmission(\"")
        .or_else(|| message.strip_prefix("Storage(\""))?
        .strip_suffix("\")")?
        .strip_prefix("projected-read pinned_bytes limit ")?;
    let (limit, observed) = detail.split_once(" would be exceeded by ")?;
    if limit.is_empty()
        || observed.is_empty()
        || !limit.bytes().all(|byte| byte.is_ascii_digit())
        || !observed.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let parsed_limit = limit.parse::<u64>().ok()?;
    let parsed_observed = observed.parse::<u64>().ok()?;
    (parsed_limit > 0
        && parsed_observed > parsed_limit
        && parsed_limit.to_string() == limit
        && parsed_observed.to_string() == observed)
        .then_some((parsed_limit, parsed_observed))
}

// Preserve only the closed client/API classification and a digest of details.
// Server messages, transport text and credentials never enter campaign output.
pub(super) fn client_error(value: rrd_client::Error) -> String {
    use rrd_client::Error;
    let code = match &value {
        Error::Api { error, .. } => {
            let mut code = format!("api-{:?}", error.code).to_ascii_lowercase();
            if let Some((limit, observed)) = pinned_read_denial(&error.message) {
                eprintln!(
                    "native_read_admission={}",
                    serde_json::json!({"resource":"pinned_bytes","limit":limit,"observed":observed})
                );
            }
            let label = if structured_snapshot_conflict(error).is_some() {
                Some("storage-runtime-conflict")
            } else {
                engine_error_label(&error.message)
            };
            if let Some(label) = label {
                code.push('-');
                code.push_str(label);
            }
            code
        }
        Error::Contract(_) => "client-contract".into(),
        Error::Transport(_) => "client-transport".into(),
        Error::Timeout => "client-timeout".into(),
        Error::ResponseTooLarge => "client-response-too-large".into(),
        Error::Decode(_) => "client-decode".into(),
        Error::WebSocket(_) => "client-websocket".into(),
        Error::WebSocketProtocol(_) => "client-websocket-protocol".into(),
        Error::ResponseIdentityMismatch => "client-response-identity".into(),
        Error::UnsupportedProtocol { .. } => "client-unsupported-protocol".into(),
    };
    let detail = digest("c07g.client-error", &value.to_string()).expect("error digest");
    format!("{code}:{}", detail.as_str())
}

#[test]
fn runtime_conflict_trace_is_closed_correlatable_and_bounded() {
    use std::sync::atomic::AtomicUsize;
    let api = |message: &str| rrd_client::Error::Api {
        status: 409_u16.try_into().unwrap(),
        error: ErrorBody {
            code: ErrorCode::Conflict,
            message: message.into(),
            retryable: false,
            details: BTreeMap::new(),
        },
    };
    let engine_error = rrd_engine::ServiceError::from(rrd_store::Error::RuntimeConflict {
        expected: 7,
        actual: 9,
    });
    let valid = api(&engine_error.to_string());
    let counter = AtomicUsize::new(0);
    let line = runtime_conflict_trace(&valid, "synthetic-secret-key", 7, &counter).unwrap();
    assert!(line.contains("read_cursor=7 expected_cursor=7 actual_cursor=9"));
    assert!(line.contains(&rrd_core::digest::sha256_hex(b"synthetic-secret-key")));
    assert!(!line.contains("synthetic-secret-key"));
    for invalid in [
        "StorageConflict(\"runtime commit conflict: expected cursor 07, actual cursor 9\")",
        "StorageConflict(\"runtime commit conflict: expected cursor 7, actual cursor 9 secret\")",
        "StorageConflict(\"runtime commit conflict: expected cursor -1, actual cursor 9\")",
        "StorageConflict(\"runtime commit conflict: expected cursor 18446744073709551616, actual cursor 9\")",
        "StorageConflict(\"runtime schema conflict: expected revision 7, actual revision 9\")",
    ] {
        assert!(runtime_conflict_trace(&api(invalid), "key", 7, &counter).is_none());
    }
    let concurrent = AtomicUsize::new(0);
    let admitted = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    (0..16)
                        .filter(|_| runtime_conflict_trace(&valid, "key", 7, &concurrent).is_some())
                        .count()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .sum::<usize>()
    });
    assert_eq!(admitted, 32);
}

#[test]
fn structured_snapshot_conflict_is_closed_and_retains_diagnostic_evidence() {
    let mut error = ErrorBody {
        code: ErrorCode::Conflict,
        message: "private-server-text".into(),
        retryable: false,
        details: [
            ("reason", "transaction-snapshot-stale"),
            ("next-action", "abort-and-reexecute-transaction"),
            ("expected-cursor", "7"),
            ("actual-cursor", "9"),
        ]
        .into_iter()
        .map(|(key, value)| (CanonicalId::new(key).unwrap(), value.into()))
        .collect(),
    };
    assert_eq!(structured_snapshot_conflict(&error), Some((7, 9)));
    let api = |body: ErrorBody| rrd_client::Error::Api {
        status: 409_u16.try_into().unwrap(),
        error: body,
    };
    let line =
        runtime_conflict_trace(&api(error.clone()), "secret-key", 7, &AtomicUsize::new(0)).unwrap();
    assert!(line.contains("read_cursor=7 expected_cursor=7 actual_cursor=9"));
    assert!(!line.contains("secret"));
    let classified = client_error(api(error.clone()));
    assert!(classified.starts_with("api-conflict-storage-runtime-conflict:"));
    assert!(!classified.contains("private"));
    for invalid in ["07", "-1", "18446744073709551616", "9 private", "7"] {
        error
            .details
            .insert(CanonicalId::new("actual-cursor").unwrap(), invalid.into());
        assert_eq!(structured_snapshot_conflict(&error), None);
        assert!(
            runtime_conflict_trace(&api(error.clone()), "secret-key", 7, &AtomicUsize::new(0))
                .is_none()
        );
    }
    error
        .details
        .insert(CanonicalId::new("actual-cursor").unwrap(), "9".into());
    error.retryable = true;
    assert_eq!(structured_snapshot_conflict(&error), None);
    error.retryable = false;
    error.code = ErrorCode::Internal;
    assert_eq!(structured_snapshot_conflict(&error), None);
    error.code = ErrorCode::Conflict;
    error
        .details
        .insert(CanonicalId::new("next-action").unwrap(), "private".into());
    assert_eq!(structured_snapshot_conflict(&error), None);
    error.details.insert(
        CanonicalId::new("next-action").unwrap(),
        "abort-and-reexecute-transaction".into(),
    );
    error
        .details
        .insert(CanonicalId::new("extra").unwrap(), "private".into());
    assert_eq!(structured_snapshot_conflict(&error), None);
}

#[test]
fn pinned_read_denial_reports_only_exact_numeric_resource_values() {
    let read = rrd_store::Error::ProjectedReadLimit {
        resource: rrd_store::ProjectedReadResource::PinnedBytes,
        limit: 4_294_967_296,
        observed: 4_294_967_297,
    };
    let admission = rrd_engine::ServiceError::StorageAdmission(read.to_string());
    assert_eq!(
        pinned_read_denial(&admission.to_string()),
        Some((4_294_967_296, 4_294_967_297))
    );
    let storage = rrd_engine::ServiceError::Storage(read.to_string());
    assert_eq!(
        pinned_read_denial(&storage.to_string()),
        Some((4_294_967_296, 4_294_967_297))
    );
    for invalid in [
        "StorageAdmission(\"projected-read pinned_bytes limit 5 would be exceeded by 6 private\")",
        "StorageAdmission(\"projected-read pinned_bytes limit 5 would be exceeded by 5\")",
        "StorageAdmission(\"projected-read pinned_bytes limit 18446744073709551616 would be exceeded by 7\")",
        "StorageAdmission(\"projected-read pinned_bytes limit 5 would be exceeded by -1\")",
        "StorageAdmission(\"projected-read pinned_bytes limit 05 would be exceeded by 6\")",
        "StorageAdmission(\"projected-read pinned_bytes limit 0 would be exceeded by 6\")",
        "StorageAdmission(\"projected-read runs limit 5 would be exceeded by 6\")",
        "StorageAdmission(\"projected-read pinned_bytes limit 5 would be exceeded by 6\")suffix",
    ] {
        assert_eq!(pinned_read_denial(invalid), None, "accepted malformed denial");
    }
}

#[test]
fn diagnostic_labels_distinguish_storage_conflicts_without_private_data() {
    let cases = [
        (
            rrd_store::Error::RuntimeConflict {
                expected: 7,
                actual: 9,
            },
            "storage-runtime-conflict",
        ),
        (
            rrd_store::Error::RuntimeSchemaConflict {
                expected: 1,
                actual: 2,
            },
            "storage-schema-conflict",
        ),
        (
            rrd_store::Error::IdempotencyConflict("secret-key".into()),
            "storage-idempotency-conflict",
        ),
        (
            rrd_store::Error::ControlConflict("secret-control".into()),
            "storage-control-conflict",
        ),
        (
            rrd_store::Error::ReadStampMismatch("secret-stamp".into()),
            "storage-read-stamp-conflict",
        ),
    ];
    for (error, label) in cases {
        let converted = rrd_engine::ServiceError::from(error);
        assert_eq!(converted.kind(), rrd_engine::ServiceErrorKind::Conflict);
        assert_eq!(engine_error_label(&converted.to_string()), Some(label));
        let encoded = client_error(rrd_client::Error::Api {
            status: 409_u16.try_into().unwrap(),
            error: ErrorBody {
                code: ErrorCode::Conflict,
                message: converted.to_string(),
                retryable: false,
                details: BTreeMap::new(),
            },
        });
        assert!(encoded.starts_with(&format!("api-conflict-{label}:")));
        assert_eq!(
            super::failure_code(&encoded).as_str(),
            format!("api-conflict-{label}")
        );
        assert!(!encoded.contains("secret"));
    }
    for malformed in [
        "StorageConflict(\"runtime commit conflict: expected cursor not-a-number, actual cursor 9\")",
        "StorageConflict(\"runtime commit conflict: expected cursor 01, actual cursor 9\")",
        "StorageConflict(\"runtime commit conflict: expected cursor 18446744073709551616, actual cursor 9\")",
        "StorageConflict(\"unknown private secret\")",
        "StorageConflict(\"runtime schema conflict: expected revision 1, actual revision 2\")suffix",
    ] {
        assert_eq!(engine_error_label(malformed), Some("storage-conflict"));
    }
    assert_eq!(
        engine_error_label("StorageConflictSecret(\"private\")"),
        None
    );
}

#[test]
fn diagnostic_labels_attribute_read_pressure_without_changing_internal_semantics() {
    use rrd_store::ProjectedReadResource as Resource;
    for (resource, label) in [
        (Resource::ActiveViews, "read-active-views"),
        (Resource::Ranges, "read-ranges"),
        (Resource::Runs, "read-runs"),
        (Resource::PinnedBytes, "read-pinned-bytes"),
        (Resource::VersionsExamined, "read-versions"),
        (Resource::PageRequests, "read-page-requests"),
        (Resource::PageLogicalBytes, "read-page-logical-bytes"),
        (Resource::OutputRows, "read-output-rows"),
        (Resource::OutputBufferBytes, "read-output-buffer-bytes"),
        (Resource::BatchRows, "read-batch-rows"),
        (Resource::BatchAllocatedBytes, "read-batch-allocated-bytes"),
        (Resource::RetainedOutputBytes, "read-retained-output-bytes"),
    ] {
        let error = rrd_engine::ServiceError::from(rrd_store::Error::ProjectedReadLimit {
            resource,
            limit: 5,
            observed: 6,
        });
        assert_eq!(error.kind(), rrd_engine::ServiceErrorKind::Internal);
        assert_eq!(engine_error_label(&error.to_string()), Some(label));
        let read_error = rrd_store::Error::ProjectedReadLimit {
            resource,
            limit: 5,
            observed: 6,
        };
        let admission = rrd_engine::ServiceError::StorageAdmission(read_error.to_string());
        assert_eq!(
            admission.kind(),
            rrd_engine::ServiceErrorKind::ResourceExhausted
        );
        assert_eq!(engine_error_label(&admission.to_string()), Some(label));
        assert_eq!(
            engine_error_label(&format!(
                "Storage(\"projected-read {} limit private secret\")",
                resource.as_str()
            )),
            Some(label)
        );
        assert_eq!(
            engine_error_label(&format!(
                "Storage(\"projected-read {}_private limit secret\")",
                resource.as_str()
            )),
            Some("storage")
        );
        assert!(format!("rejected-api-internal-{label}:{}", "a".repeat(64)).len() <= 128);
    }
    for (error, label) in [
        (
            rrd_store::Error::SourcePageLimit {
                limit: 5,
                observed: 6,
            },
            "read-source-pages",
        ),
        (
            rrd_store::Error::TransactionLimit {
                resource: rrd_store::TransactionResource::Active,
                limit: 5,
                observed: 6,
            },
            "transaction-active",
        ),
        (
            rrd_store::Error::TransactionLimit {
                resource: rrd_store::TransactionResource::BufferedBytes,
                limit: 5,
                observed: 6,
            },
            "transaction-buffered-bytes",
        ),
    ] {
        let error = rrd_engine::ServiceError::from(error);
        assert_eq!(error.kind(), rrd_engine::ServiceErrorKind::Internal);
        assert_eq!(engine_error_label(&error.to_string()), Some(label));
    }
    assert_eq!(engine_error_label("StorageSecret(\"private\")"), None);
    assert_eq!(
        engine_error_label("Storage(\"substrate: private secret\")"),
        Some("storage-substrate")
    );
    assert_eq!(
        engine_error_label("Storage(\"private secret\")"),
        Some("storage")
    );
}

#[test]
fn read_admission_uses_real_store_error_labels_without_private_details() {
    let cases = [
        (
            rrd_store::Error::ProjectedReadLimit {
                resource: rrd_store::ProjectedReadResource::Runs,
                limit: 5,
                observed: 6,
            },
            "read-runs",
        ),
        (
            rrd_store::Error::SourcePageLimit {
                limit: 5,
                observed: 6,
            },
            "read-source-pages",
        ),
        (
            rrd_store::Error::SegmentHandleLimit {
                limit: 5,
                observed: 6,
            },
            "read-segment-handles",
        ),
        (
            rrd_store::Error::CheckpointLimit {
                limit: 5,
                observed: 6,
            },
            "storage-admission-checkpoint",
        ),
    ];
    for (cause, expected) in cases {
        let error = rrd_engine::ServiceError::StorageAdmission(cause.to_string());
        assert_eq!(
            error.kind(),
            rrd_engine::ServiceErrorKind::ResourceExhausted
        );
        assert_eq!(engine_error_label(&error.to_string()), Some(expected));
        assert!(format!("api-resourceexhausted-{expected}:{}", "a".repeat(64)).len() <= 128);
    }
    assert_eq!(
        engine_error_label("StorageAdmission(\"projected-read runs_private limit secret\")"),
        Some("storage-admission")
    );
}

#[test]
fn diagnostic_labels_identify_control_state_capacity_from_the_real_validator() {
    let error = rrd_store::ControlTransition {
        key: "server/state/test/capacity".into(),
        expected: None,
        replacement: Some(vec![0; 1024 * 1024 + 1]),
        at: 1,
        actor: "test".into(),
        action: "test".into(),
        request_id: "test".into(),
        operation_id: "test".into(),
    }
    .validate()
    .unwrap_err();
    let error = rrd_engine::ServiceError::from(error);
    assert_eq!(error.kind(), rrd_engine::ServiceErrorKind::Internal);
    assert_eq!(
        engine_error_label(&error.to_string()),
        Some("control-state-size")
    );
    assert_eq!(
        engine_error_label("Storage(\"substrate: control state exceeds one MiB private\")"),
        Some("storage-substrate")
    );
}

#[test]
fn diagnostic_labels_never_copy_server_details_or_imply_outcomes() {
    for (resource, label) in [
        ("input_bytes", "maintenance-input-bytes"),
        ("input_logical_bytes", "maintenance-input-logical"),
        ("input_versions", "maintenance-input-versions"),
        ("output_segments", "maintenance-output-segments"),
        ("elapsed_ms", "maintenance-elapsed"),
        ("l0_segments", "maintenance-l0-segments"),
    ] {
        assert_eq!(
            engine_error_label(&format!(
                "StorageAdmission(\"maintenance {resource} limit secret suffix\")"
            )),
            Some(label)
        );
        // The complete API label and digest must also fit campaign evidence.
        assert!(format!("api-resourceexhausted-{label}:{}", "a".repeat(64)).len() <= 128);
    }
    assert_eq!(
        engine_error_label("StorageAdmission(\"maintenance input_bytes_private limit secret\")"),
        Some("storage-admission-maintenance")
    );
    assert_eq!(
        engine_error_label("StorageAdmission(\"mutable secret suffix\")"),
        Some("storage-admission-mutable")
    );
    assert_eq!(
        engine_error_label("StorageAdmission(\"transaction admission secret suffix\")"),
        Some("storage-admission-queue")
    );
    assert_eq!(
        engine_error_label("StorageAdmission(\"transaction secret suffix\")"),
        Some("storage-admission-transaction")
    );
    assert_eq!(
        engine_error_label("TransactionQuota"),
        Some("transaction-quota")
    );
    assert_eq!(
        engine_error_label("StorageAdmission(\"private payload\")"),
        Some("storage-admission")
    );
    assert_eq!(
        engine_error_label("ConfigurationLimit(\"private payload\")"),
        Some("configuration-limit")
    );
    for message in [
        "TransactionQuota private payload",
        "StorageAdmissionSecret",
        "private payload",
        "(StorageAdmission)",
    ] {
        assert_eq!(engine_error_label(message), None);
    }
}

#[path = "workload/run.rs"]
mod run;
