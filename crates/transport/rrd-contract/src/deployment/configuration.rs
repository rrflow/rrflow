//! Engram configuration, its input, the follow policy and its digest.

use super::*;

/// Operator-authored input. Revision and digest are assigned by the engine,
/// so neither can be forged or silently retained across an edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramConfigurationInput {
    pub format_version: u16,
    pub clock: ClockPolicy,
    pub storage: StorageLimits,
    pub reasoning: ReasoningLimits,
    pub recall: RecallLimits,
    pub query: QueryBudget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attunement_follow: Option<AttunementFollowPolicy>,
}

/// The served engine keeps its project attuned: every `interval_ms` it looks
/// for a change to the project's files, and when it finds one it derives the
/// successor attunement plan from the committed evidence, accepts it and steps
/// it to the end, as the installed operator, through the public attunement
/// operations and their receipts. Sealed into the installation plan; absent
/// means the engine never attunes on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementFollowPolicy {
    pub interval_ms: u64,
}

impl AttunementFollowPolicy {
    pub const MIN_INTERVAL_MS: u64 = 1_000;
    pub const MAX_INTERVAL_MS: u64 = 3_600_000;

    pub fn validate(&self) -> Result<()> {
        if !(Self::MIN_INTERVAL_MS..=Self::MAX_INTERVAL_MS).contains(&self.interval_ms) {
            return invalid(format!(
                "attunement follow interval must be in {}..={} ms",
                Self::MIN_INTERVAL_MS,
                Self::MAX_INTERVAL_MS
            ));
        }
        Ok(())
    }
}

impl EngramConfigurationInput {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != ENGRAM_CONFIGURATION_FORMAT_VERSION {
            return invalid(format!(
                "engram configuration format version must be {ENGRAM_CONFIGURATION_FORMAT_VERSION}"
            ));
        }
        self.clock.validate()?;
        self.storage.validate()?;
        self.reasoning.validate()?;
        self.recall.validate()?;
        self.query.validate()?;
        if let Some(follow) = &self.attunement_follow {
            follow.validate()?;
        }
        Ok(())
    }
}

/// Effective, immutable configuration bound to an installation plan and
/// surfaced to every client. Mutating it requires a future governed
/// configure-plan/configure-apply operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngramConfiguration {
    pub format_version: u16,
    pub revision: u64,
    pub clock: ClockPolicy,
    pub storage: StorageLimits,
    pub reasoning: ReasoningLimits,
    pub recall: RecallLimits,
    pub query: QueryBudget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attunement_follow: Option<AttunementFollowPolicy>,
    pub configuration_sha256: String,
}

impl EngramConfiguration {
    pub fn from_input(revision: u64, input: EngramConfigurationInput) -> Result<Self> {
        input.validate()?;
        if revision == 0 {
            return invalid("engram configuration revision must be greater than zero");
        }
        let mut configuration = Self {
            format_version: input.format_version,
            revision,
            clock: input.clock,
            storage: input.storage,
            reasoning: input.reasoning,
            recall: input.recall,
            query: input.query,
            attunement_follow: input.attunement_follow,
            configuration_sha256: String::new(),
        };
        configuration.configuration_sha256 = engram_configuration_sha256(&configuration)?;
        Ok(configuration)
    }

    pub fn validate(&self) -> Result<()> {
        EngramConfigurationInput {
            format_version: self.format_version,
            clock: self.clock.clone(),
            storage: self.storage.clone(),
            reasoning: self.reasoning.clone(),
            recall: self.recall.clone(),
            query: self.query.clone(),
            attunement_follow: self.attunement_follow,
        }
        .validate()?;
        if self.revision == 0 {
            return invalid("engram configuration revision must be greater than zero");
        }
        if self.configuration_sha256 != engram_configuration_sha256(self)? {
            return invalid("engram configuration digest does not match its content");
        }
        Ok(())
    }

    /// The same configuration with the engine keeping its project attuned at
    /// `policy`, or never when `None`; the digest is recomputed.
    pub fn with_attunement_follow(
        mut self,
        policy: Option<AttunementFollowPolicy>,
    ) -> Result<Self> {
        if let Some(policy) = &policy {
            policy.validate()?;
        }
        self.attunement_follow = policy;
        self.configuration_sha256 = engram_configuration_sha256(&self)?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate_query_budget(&self, requested: &QueryBudget) -> Result<()> {
        requested.validate()?;
        let configured = &self.query;
        for (name, requested, ceiling) in [
            (
                "max_storage_keys",
                requested.max_storage_keys,
                configured.max_storage_keys,
            ),
            ("max_rows", requested.max_rows, configured.max_rows),
            (
                "max_output_bytes",
                requested.max_output_bytes,
                configured.max_output_bytes,
            ),
            (
                "max_batch_rows",
                requested.max_batch_rows,
                configured.max_batch_rows,
            ),
            (
                "max_memory_bytes",
                requested.max_memory_bytes,
                configured.max_memory_bytes,
            ),
            (
                "max_spill_bytes",
                requested.max_spill_bytes,
                configured.max_spill_bytes,
            ),
            (
                "max_elapsed_ms",
                requested.max_elapsed_ms,
                configured.max_elapsed_ms,
            ),
        ] {
            if requested > ceiling {
                return invalid(format!(
                    "query request {name}={requested} exceeds configured ceiling {ceiling}"
                ));
            }
        }
        Ok(())
    }
}

impl Default for EngramConfiguration {
    fn default() -> Self {
        Self::from_input(
            1,
            EngramConfigurationInput {
                format_version: ENGRAM_CONFIGURATION_FORMAT_VERSION,
                clock: ClockPolicy {
                    maximum_rollback_ms: 0,
                },
                storage: StorageLimits::default(),
                reasoning: ReasoningLimits {
                    max_run_elapsed_ms: 900_000,
                    max_steps: 256,
                    max_step_elapsed_ms: 60_000,
                },
                recall: RecallLimits {
                    max_graph_depth: 4,
                    max_items: 128,
                    max_output_bytes: 512 * 1024,
                    max_storage_keys: 100_000,
                },
                query: QueryBudget::default(),
                attunement_follow: None,
            },
        )
        .expect("default engram configuration is valid")
    }
}

pub fn engram_configuration_sha256(configuration: &EngramConfiguration) -> Result<String> {
    let encoded = serde_json::to_vec(&(
        configuration.format_version,
        configuration.revision,
        &configuration.clock,
        &configuration.storage,
        &configuration.reasoning,
        &configuration.recall,
        &configuration.query,
    ))
    .map_err(|error| {
        crate::ContractError(format!("configuration digest encoding failed: {error}"))
    })?;
    let mut bytes = Vec::with_capacity(40 + encoded.len());
    bytes.extend_from_slice(b"rrflow-engram-configuration-v3\0");
    bytes.extend_from_slice(&encoded);
    Ok(sha256_bytes(&bytes))
}

pub(super) fn validate_nonzero_max(name: &str, value: u64, maximum: u64) -> Result<()> {
    if value == 0 || value > maximum {
        return invalid(format!("{name} must be in 1..={maximum}"));
    }
    Ok(())
}
