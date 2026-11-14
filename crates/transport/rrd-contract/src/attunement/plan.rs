//! Attunement plans, phase plans, runtime coordinates and checkpoints.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementPhasePlan {
    pub phase: AttunementPhase,
    pub sequence: u16,
    pub configuration_sha256: String,
    pub estimated_items: u64,
    pub estimated_input_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_policy: Option<SourceParsePolicy>,
}

impl AttunementPhasePlan {
    fn validate(&self) -> Result<()> {
        validate_sha256(
            &self.configuration_sha256,
            "attunement phase configuration_sha256",
        )?;
        if let Some(policy) = &self.parse_policy {
            if self.phase != AttunementPhase::Parse {
                return invalid("source parse policy requires a Parse step");
            }
            policy.validate()?;
        }
        Ok(())
    }
}

pub(super) fn attunement_version_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type": "integer", "const": ATTUNEMENT_CONTRACT_VERSION})
}

pub(super) fn attunement_steps_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    let mut schema = <Vec<AttunementPhasePlan>>::json_schema(generator);
    schema.insert("minItems".into(), serde_json::json!(1));
    schema.insert("maxItems".into(), serde_json::json!(MAX_ATTUNEMENT_PHASES));
    schema
}

/// A bounded, content-addressed plan over one project source snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementPlan {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub id: CanonicalId,
    pub target: InstalledEstateIdentity,
    pub source_sha256: String,
    #[schemars(schema_with = "attunement_steps_schema")]
    pub phases: Vec<AttunementPhasePlan>,
    pub estimated_min_duration_ms: u64,
    pub estimated_max_duration_ms: u64,
    pub plan_sha256: String,
}

impl AttunementPlan {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)?;
        self.target.validate()?;
        validate_sha256(&self.source_sha256, "attunement source_sha256")?;
        if self.phases.is_empty() || self.phases.len() > MAX_ATTUNEMENT_PHASES {
            return invalid("attunement plan must contain a bounded nonempty sequence");
        }
        for (index, phase) in self.phases.iter().enumerate() {
            phase.validate()?;
            if usize::from(phase.sequence) != index + 1 {
                return invalid("attunement phase sequence must match its plan position");
            }
            if phase.phase == AttunementPhase::Verify && index + 1 != self.phases.len() {
                return invalid("attunement verification must be the final step when selected");
            }
        }
        if self.estimated_min_duration_ms > self.estimated_max_duration_ms {
            return invalid("attunement duration estimate is inverted");
        }
        validate_sha256(&self.plan_sha256, "attunement plan_sha256")?;
        if self.plan_sha256 != attunement_plan_sha256(self)? {
            return invalid("attunement plan digest does not match its content");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementRuntimeCoordinates {
    pub runtime_manifest_sha256: String,
    pub runtime_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_revision: Option<u64>,
    pub catalogue_revision: u64,
}

impl AttunementRuntimeCoordinates {
    fn validate(&self) -> Result<()> {
        validate_sha256(
            &self.runtime_manifest_sha256,
            "attunement runtime_manifest_sha256",
        )?;
        if self.schema_revision == Some(0) {
            return invalid("attunement schema revision zero is not installed");
        }
        Ok(())
    }
}

/// One durably committed phase output. Checkpoints form a digest chain: the
/// first input is the plan source and each later input is the prior output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementPhaseCheckpoint {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub plan_sha256: String,
    pub phase: AttunementPhase,
    pub sequence: u16,
    pub configuration_sha256: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub coordinates: AttunementRuntimeCoordinates,
    pub committed_at_unix_ms: u64,
    pub checkpoint_sha256: String,
}

impl AttunementPhaseCheckpoint {
    pub fn validate_for(
        &self,
        plan: &AttunementPlan,
        job_id: &CanonicalId,
        previous: Option<&Self>,
    ) -> Result<()> {
        plan.validate()?;
        validate_attunement_contract_version(self.contract_version)?;
        if &self.job_id != job_id || self.plan_sha256 != plan.plan_sha256 {
            return invalid("attunement checkpoint does not bind its job and plan");
        }
        let planned = self
            .sequence
            .checked_sub(1)
            .and_then(|index| plan.phases.get(usize::from(index)))
            .ok_or_else(|| {
                crate::ContractError("attunement checkpoint sequence is outside its plan".into())
            })?;
        if self.phase != planned.phase || self.configuration_sha256 != planned.configuration_sha256
        {
            return invalid("attunement checkpoint configuration digest differs from its plan");
        }
        validate_sha256(&self.input_sha256, "attunement checkpoint input_sha256")?;
        validate_sha256(&self.output_sha256, "attunement checkpoint output_sha256")?;
        match previous {
            None if self.sequence == 1 && self.input_sha256 == plan.source_sha256 => {}
            Some(previous)
                if previous.sequence.checked_add(1) == Some(self.sequence)
                    && self.input_sha256 == previous.output_sha256 =>
            {
                if self.committed_at_unix_ms < previous.committed_at_unix_ms {
                    return invalid("attunement checkpoint commit time moved backwards");
                }
                if self.coordinates.runtime_cursor < previous.coordinates.runtime_cursor
                    || self.coordinates.schema_revision < previous.coordinates.schema_revision
                    || self.coordinates.catalogue_revision < previous.coordinates.catalogue_revision
                {
                    return invalid("attunement checkpoint coordinates moved backwards");
                }
            }
            _ => return invalid("attunement checkpoint skipped a phase or broke its digest chain"),
        }
        self.coordinates.validate()?;
        if self.committed_at_unix_ms == 0 {
            return invalid("attunement checkpoint commit time must be greater than zero");
        }
        validate_sha256(&self.checkpoint_sha256, "attunement checkpoint_sha256")?;
        if self.checkpoint_sha256 != attunement_checkpoint_sha256(self)? {
            return invalid("attunement checkpoint digest does not match its content");
        }
        Ok(())
    }
}
