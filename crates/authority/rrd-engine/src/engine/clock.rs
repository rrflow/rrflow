use super::*;
use rrd_contract::ClockPolicy;
use std::time::{SystemTime, UNIX_EPOCH};

pub const CLOCK_OBSERVATION_FORMAT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockSourceKind {
    HostSystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockTrustLevel {
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockObservationFailure {
    BeforeUnixEpoch,
    Unrepresentable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockObservation {
    pub format_version: u16,
    pub source: ClockSourceKind,
    pub trust: ClockTrustLevel,
    pub observed_at_unix_ms: u64,
}

impl ClockObservation {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != CLOCK_OBSERVATION_FORMAT_VERSION || self.observed_at_unix_ms == 0
        {
            return Err(ServiceError::Contract(
                "clock observation has invalid format or coordinate".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockAssessmentStatus {
    Current,
    RollbackWithinTolerance,
    RollbackExceeded,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockAssessment {
    pub status: ClockAssessmentStatus,
    pub observation: Option<ClockObservation>,
    pub failure: Option<ClockObservationFailure>,
    pub anchor: ClockObservation,
    pub maximum_rollback_ms: u64,
    pub rollback_ms: Option<u64>,
}

impl ClockAssessment {
    pub fn passed(&self) -> bool {
        matches!(
            self.status,
            ClockAssessmentStatus::Current | ClockAssessmentStatus::RollbackWithinTolerance
        )
    }
}

pub(super) trait ClockSource {
    fn observe(&self) -> std::result::Result<ClockObservation, ClockObservationFailure>;
}

pub(super) struct HostSystemClock;

impl ClockSource for HostSystemClock {
    fn observe(&self) -> std::result::Result<ClockObservation, ClockObservationFailure> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ClockObservationFailure::BeforeUnixEpoch)?;
        let observed_at_unix_ms = u64::try_from(elapsed.as_millis())
            .map_err(|_| ClockObservationFailure::Unrepresentable)?;
        if observed_at_unix_ms == 0 {
            return Err(ClockObservationFailure::Unrepresentable);
        }
        Ok(ClockObservation {
            format_version: CLOCK_OBSERVATION_FORMAT_VERSION,
            source: ClockSourceKind::HostSystemTime,
            trust: ClockTrustLevel::Unverified,
            observed_at_unix_ms,
        })
    }
}

pub(super) fn observe_required(
    source: &dyn ClockSource,
    boundary: &'static str,
) -> Result<ClockObservation> {
    match source.observe() {
        Ok(observation) => {
            observation.validate()?;
            tracing::debug!(
                target: "rrflow::clock",
                boundary,
                source = ?observation.source,
                trust = ?observation.trust,
                observed_at_unix_ms = observation.observed_at_unix_ms,
                "host clock observed"
            );
            Ok(observation)
        }
        Err(failure) => {
            tracing::error!(
                target: "rrflow::clock",
                boundary,
                failure = ?failure,
                "host clock observation failed"
            );
            Err(ServiceError::ClockUnavailable(failure))
        }
    }
}

pub(super) fn assess(
    observed: std::result::Result<ClockObservation, ClockObservationFailure>,
    anchor: &ClockObservation,
    policy: &ClockPolicy,
    boundary: &'static str,
) -> Result<ClockAssessment> {
    anchor.validate()?;
    policy
        .validate()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    let assessment = match observed {
        Ok(observation) => {
            observation.validate()?;
            let rollback_ms = anchor
                .observed_at_unix_ms
                .saturating_sub(observation.observed_at_unix_ms);
            let status = if rollback_ms == 0 {
                ClockAssessmentStatus::Current
            } else if rollback_ms <= policy.maximum_rollback_ms {
                ClockAssessmentStatus::RollbackWithinTolerance
            } else {
                ClockAssessmentStatus::RollbackExceeded
            };
            ClockAssessment {
                status,
                observation: Some(observation),
                failure: None,
                anchor: anchor.clone(),
                maximum_rollback_ms: policy.maximum_rollback_ms,
                rollback_ms: Some(rollback_ms),
            }
        }
        Err(failure) => ClockAssessment {
            status: ClockAssessmentStatus::Unavailable,
            observation: None,
            failure: Some(failure),
            anchor: anchor.clone(),
            maximum_rollback_ms: policy.maximum_rollback_ms,
            rollback_ms: None,
        },
    };
    trace_assessment(boundary, &assessment);
    Ok(assessment)
}

pub(super) fn require_accepted(assessment: &ClockAssessment) -> Result<()> {
    match assessment.status {
        ClockAssessmentStatus::Current | ClockAssessmentStatus::RollbackWithinTolerance => Ok(()),
        ClockAssessmentStatus::Unavailable => {
            let failure = assessment.failure.ok_or_else(|| {
                ServiceError::Contract("unavailable clock assessment has no failure".into())
            })?;
            Err(ServiceError::ClockUnavailable(failure))
        }
        ClockAssessmentStatus::RollbackExceeded => {
            let observed_at_unix_ms = assessment
                .observation
                .as_ref()
                .ok_or_else(|| {
                    ServiceError::Contract("rollback clock assessment has no observation".into())
                })?
                .observed_at_unix_ms;
            Err(ServiceError::ClockRollback {
                observed_at_unix_ms,
                anchor_unix_ms: assessment.anchor.observed_at_unix_ms,
                maximum_rollback_ms: assessment.maximum_rollback_ms,
            })
        }
    }
}

fn trace_assessment(boundary: &'static str, assessment: &ClockAssessment) {
    let observed_at_unix_ms = assessment
        .observation
        .as_ref()
        .map(|observation| observation.observed_at_unix_ms);
    match assessment.status {
        ClockAssessmentStatus::Current => tracing::debug!(
            target: "rrflow::clock",
            boundary,
            status = ?assessment.status,
            observed_at_unix_ms,
            anchor_unix_ms = assessment.anchor.observed_at_unix_ms,
            "clock assessment accepted"
        ),
        ClockAssessmentStatus::RollbackWithinTolerance => tracing::warn!(
            target: "rrflow::clock",
            boundary,
            status = ?assessment.status,
            observed_at_unix_ms,
            anchor_unix_ms = assessment.anchor.observed_at_unix_ms,
            rollback_ms = assessment.rollback_ms,
            maximum_rollback_ms = assessment.maximum_rollback_ms,
            "clock rollback accepted by explicit policy"
        ),
        ClockAssessmentStatus::RollbackExceeded | ClockAssessmentStatus::Unavailable => {
            tracing::error!(
                target: "rrflow::clock",
                boundary,
                status = ?assessment.status,
                observed_at_unix_ms,
                anchor_unix_ms = assessment.anchor.observed_at_unix_ms,
                rollback_ms = assessment.rollback_ms,
                maximum_rollback_ms = assessment.maximum_rollback_ms,
                failure = ?assessment.failure,
                "clock assessment rejected"
            )
        }
    }
}

#[cfg(test)]
pub(super) struct FixedClock {
    result: std::result::Result<ClockObservation, ClockObservationFailure>,
}

#[cfg(test)]
impl FixedClock {
    pub(super) fn at(observed_at_unix_ms: u64) -> Self {
        Self {
            result: Ok(ClockObservation {
                format_version: CLOCK_OBSERVATION_FORMAT_VERSION,
                source: ClockSourceKind::HostSystemTime,
                trust: ClockTrustLevel::Unverified,
                observed_at_unix_ms,
            }),
        }
    }

    pub(super) fn unavailable(failure: ClockObservationFailure) -> Self {
        Self {
            result: Err(failure),
        }
    }
}

#[cfg(test)]
impl ClockSource for FixedClock {
    fn observe(&self) -> std::result::Result<ClockObservation, ClockObservationFailure> {
        self.result.clone()
    }
}
