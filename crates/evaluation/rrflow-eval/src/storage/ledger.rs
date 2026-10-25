use super::{require, Digest, Id, Limit, Result, MAX_ATTEMPTS};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Written by the workload oracle BEFORE execution, not reconstructed from
/// the engine's response. Digested into the required case in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOperation {
    pub logical_id: Id,
    pub request_sha256: Digest,
    pub effect_sha256: Digest,
    pub resolution: ExpectedResolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedResolution {
    Present,
    Absent,
    Either,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AttemptOutcome {
    Acknowledged { receipt_sha256: Digest },
    Rejected { reason: Id },
    Uncertain { reason: Id },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub attempt_id: Id,
    pub logical_id: Id,
    pub request_sha256: Digest,
    /// Client monotonic offsets, including failed attempts, never host UTC.
    #[schemars(range(max = 9007199254740991_u64))]
    pub started_ns: u64,
    #[schemars(range(max = 9007199254740991_u64))]
    pub finished_ns: u64,
    pub outcome: AttemptOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Readback {
    Present {
        logical_id: Id,
        request_sha256: Digest,
        effect_sha256: Digest,
        receipt_sha256: Digest,
        applications: Limit,
    },
    /// A positive absence observation after authoritative recovery. Omission
    /// of a readback entry is unresolved, NOT proof of absence.
    Absent { logical_id: Id },
}
impl Readback {
    fn id(&self) -> &Id {
        match self {
            Self::Present { logical_id, .. } | Self::Absent { logical_id } => logical_id,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcknowledgementLedger {
    #[schemars(length(max = 10000))]
    pub expected: Vec<ExpectedOperation>,
    #[schemars(length(max = 10000))]
    pub attempts: Vec<Attempt>,
    #[schemars(length(max = 10000))]
    pub readback: Vec<Readback>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LedgerReport {
    pub logical_calls: u64,
    pub attempts: u64,
    pub acknowledged: u64,
    pub rejected: u64,
    pub uncertain: u64,
    pub resolved_present: u64,
    pub resolved_absent: u64,
    pub unresolved: u64,
    pub violations: Vec<Id>,
}

impl AcknowledgementLedger {
    /// Independent equality/identity checker. It never calls the engine and
    /// never fills expected values from recovered observations.
    pub fn reconcile(&self, max_attempts: u64, max_duration_ns: u64) -> Result<LedgerReport> {
        require(
            self.expected.len() <= MAX_ATTEMPTS
                && self.readback.len() <= MAX_ATTEMPTS
                && self.attempts.len() <= MAX_ATTEMPTS
                && self.attempts.len() as u64 <= max_attempts,
            "ledger exceeds bounded corpus",
        )?;
        let mut expected = BTreeMap::new();
        for item in &self.expected {
            require(
                expected.insert(&item.logical_id, item).is_none(),
                "duplicate expected logical identity",
            )?;
        }
        let mut attempts = BTreeMap::<&Id, Vec<&Attempt>>::new();
        let mut attempt_ids = BTreeSet::new();
        for attempt in &self.attempts {
            require(
                attempt_ids.insert(&attempt.attempt_id),
                "duplicate attempt identity",
            )?;
            let operation = expected
                .get(&attempt.logical_id)
                .ok_or("attempt has unknown logical identity")?;
            require(
                operation.request_sha256 == attempt.request_sha256,
                "retry changed request identity",
            )?;
            require(
                attempt.started_ns <= attempt.finished_ns && attempt.finished_ns <= max_duration_ns,
                "invalid client timing boundary",
            )?;
            attempts
                .entry(&attempt.logical_id)
                .or_default()
                .push(attempt);
        }
        let mut readback = BTreeMap::new();
        for observation in &self.readback {
            require(
                expected.contains_key(observation.id()),
                "unknown recovered identity",
            )?;
            require(
                readback.insert(observation.id(), observation).is_none(),
                "duplicate recovered identity",
            )?;
        }
        let mut report = LedgerReport {
            logical_calls: expected.len() as u64,
            attempts: self.attempts.len() as u64,
            ..LedgerReport::default()
        };
        let mut violations = BTreeSet::new();
        for attempt in &self.attempts {
            match attempt.outcome {
                AttemptOutcome::Acknowledged { .. } => report.acknowledged += 1,
                AttemptOutcome::Rejected { .. } => report.rejected += 1,
                AttemptOutcome::Uncertain { .. } => report.uncertain += 1,
            }
        }
        for (id, operation) in expected {
            let sent = attempts.get(id).map(Vec::as_slice).unwrap_or(&[]);
            if sent.is_empty() {
                violations.insert("missing-attempt");
            }
            let acknowledged: Vec<&Digest> = sent
                .iter()
                .filter_map(|attempt| match &attempt.outcome {
                    AttemptOutcome::Acknowledged { receipt_sha256 } => Some(receipt_sha256),
                    _ => None,
                })
                .collect();
            let all_rejected = !sent.is_empty()
                && sent
                    .iter()
                    .all(|a| matches!(a.outcome, AttemptOutcome::Rejected { .. }));
            match readback.get(id) {
                None => {
                    report.unresolved += 1;
                }
                Some(Readback::Absent { .. }) => {
                    report.resolved_absent += 1;
                    if !acknowledged.is_empty() {
                        violations.insert("lost-acknowledged-operation");
                    }
                    if operation.resolution == ExpectedResolution::Present {
                        violations.insert("required-effect-absent");
                    }
                }
                Some(Readback::Present {
                    request_sha256,
                    effect_sha256,
                    receipt_sha256,
                    applications,
                    ..
                }) => {
                    report.resolved_present += 1;
                    if operation.resolution == ExpectedResolution::Absent {
                        violations.insert("forbidden-effect-present");
                    }
                    if all_rejected || sent.is_empty() {
                        violations.insert("effect-without-admitted-attempt");
                    }
                    if *request_sha256 != operation.request_sha256
                        || *effect_sha256 != operation.effect_sha256
                    {
                        violations.insert("recovered-content-mismatch");
                    }
                    if acknowledged
                        .iter()
                        .any(|receipt| *receipt != receipt_sha256)
                    {
                        violations.insert("acknowledgement-receipt-mismatch");
                    }
                    if applications.get() != 1 {
                        violations.insert("duplicate-effect");
                    }
                }
            }
        }
        report.violations = violations
            .into_iter()
            .map(|v| Id::try_from(v.to_owned()).expect("static violation id"))
            .collect();
        Ok(report)
    }
}
