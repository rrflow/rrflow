//! Evidence-derived attunement plans.
//!
//! A capability is a typed unit of attunement work with a precondition over
//! bounded evidence about the project. The engine composes a plan as the
//! ordered subset of capabilities whose preconditions the evidence satisfies
//! and that have an installed executor. The derivation is a pure function of
//! the evidence so a client can recompute and verify it; the engine remains
//! the only authority that observes the project and persists the plan.

use super::{
    attunement_plan_sha256, digest_json, AttunementPhase, AttunementPhasePlan, AttunementPlan,
    SourceParseBinding, SourceParsePolicy, MAX_ATTUNEMENT_PHASES,
};
use crate::{invalid, validate_sha256, CanonicalId, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const ATTUNEMENT_EVIDENCE_FORMAT_VERSION: u16 = 1;
pub const MAX_ATTUNEMENT_EVIDENCE_LANGUAGES: usize = 64;

/// Bounded, content-addressed facts the engine derived a plan from.
///
/// Every field is an observation of committed inventory records or installed
/// configuration, never a claim about work the plan has not yet done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementEvidence {
    pub format_version: u16,
    /// The committed inventory head this evidence was read from.
    pub inventory_sha256: String,
    pub entries: u64,
    pub files: u64,
    pub content_bytes: u64,
    /// Files per detected language, keyed by canonical language id.
    pub languages: BTreeMap<CanonicalId, u64>,
    /// Detected languages for which a parser grammar is bound.
    pub bound_grammars: BTreeSet<CanonicalId>,
    /// An applied endpoint binding of the embedding-and-reranking class exists.
    pub embedding_binding: bool,
    /// Capabilities the installed engine can execute. The engine owns this
    /// set; the derivation never selects outside it.
    pub available: BTreeSet<AttunementPhase>,
    pub evidence_sha256: String,
}

impl AttunementEvidence {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != ATTUNEMENT_EVIDENCE_FORMAT_VERSION {
            return invalid("unsupported attunement evidence version");
        }
        validate_sha256(
            &self.inventory_sha256,
            "attunement evidence inventory_sha256",
        )?;
        if self.files > self.entries || self.languages.len() > MAX_ATTUNEMENT_EVIDENCE_LANGUAGES {
            return invalid("attunement evidence counts are out of bounds");
        }
        if self.languages.values().sum::<u64>() > self.files {
            return invalid("attunement evidence language files exceed its files");
        }
        if !self
            .bound_grammars
            .iter()
            .all(|language| self.languages.contains_key(language))
        {
            return invalid("attunement evidence binds a grammar to an undetected language");
        }
        if !self.available.contains(&AttunementPhase::Connect)
            || !self.available.contains(&AttunementPhase::Inventory)
        {
            return invalid("attunement evidence must make connect and inventory available");
        }
        validate_sha256(&self.evidence_sha256, "attunement evidence_sha256")?;
        if self.evidence_sha256 != self.digest()? {
            return invalid("attunement evidence digest does not match its content");
        }
        Ok(())
    }

    /// The digest covers every observation; it is what the derived plan binds.
    pub fn digest(&self) -> Result<String> {
        digest_json(
            b"rrflow-attunement-evidence-v1",
            &(
                self.format_version,
                &self.inventory_sha256,
                self.entries,
                self.files,
                self.content_bytes,
                &self.languages,
                &self.bound_grammars,
                self.embedding_binding,
                &self.available,
            ),
        )
    }
}

/// What the evidence must show before a capability is selected, beyond the
/// capabilities it requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttunementPrecondition {
    Always,
    /// At least one detected language has a bound grammar.
    BoundGrammar,
    /// At least one file was inventoried.
    AnyFile,
    /// An applied embedding binding exists.
    EmbeddingBinding,
    /// At least one of the listed capabilities was selected.
    AnyOf(&'static [AttunementPhase]),
}

/// One typed unit of attunement work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttunementCapability {
    pub phase: AttunementPhase,
    /// Bumped whenever the capability's effect or configuration changes
    /// meaning; it is part of every derived step's configuration digest.
    pub version: u16,
    /// Capabilities that must be selected before this one.
    pub requires: &'static [AttunementPhase],
    pub precondition: AttunementPrecondition,
}

/// The catalogue in canonical execution order. Verify stays last so the plan
/// rule that verification is the final step holds for every derivation.
pub const ATTUNEMENT_CAPABILITIES: [AttunementCapability; 11] = [
    AttunementCapability {
        phase: AttunementPhase::Connect,
        version: 1,
        requires: &[],
        precondition: AttunementPrecondition::Always,
    },
    AttunementCapability {
        phase: AttunementPhase::Inventory,
        version: 1,
        requires: &[AttunementPhase::Connect],
        precondition: AttunementPrecondition::Always,
    },
    AttunementCapability {
        phase: AttunementPhase::Parse,
        version: 1,
        requires: &[AttunementPhase::Inventory],
        precondition: AttunementPrecondition::BoundGrammar,
    },
    AttunementCapability {
        phase: AttunementPhase::Normalize,
        version: 1,
        requires: &[AttunementPhase::Parse],
        precondition: AttunementPrecondition::Always,
    },
    AttunementCapability {
        phase: AttunementPhase::EntityLink,
        version: 1,
        requires: &[AttunementPhase::Parse],
        precondition: AttunementPrecondition::Always,
    },
    AttunementCapability {
        phase: AttunementPhase::LexicalIndex,
        version: 1,
        requires: &[AttunementPhase::Inventory],
        precondition: AttunementPrecondition::AnyFile,
    },
    AttunementCapability {
        phase: AttunementPhase::Embed,
        version: 1,
        requires: &[AttunementPhase::Parse],
        precondition: AttunementPrecondition::EmbeddingBinding,
    },
    AttunementCapability {
        phase: AttunementPhase::VectorIndex,
        version: 1,
        requires: &[AttunementPhase::Embed],
        precondition: AttunementPrecondition::Always,
    },
    AttunementCapability {
        phase: AttunementPhase::Graph,
        version: 1,
        requires: &[AttunementPhase::EntityLink],
        precondition: AttunementPrecondition::Always,
    },
    AttunementCapability {
        phase: AttunementPhase::Ground,
        version: 1,
        requires: &[AttunementPhase::Inventory],
        precondition: AttunementPrecondition::AnyOf(&[
            AttunementPhase::Parse,
            AttunementPhase::LexicalIndex,
            AttunementPhase::Embed,
            AttunementPhase::Graph,
        ]),
    },
    AttunementCapability {
        phase: AttunementPhase::Verify,
        version: 1,
        requires: &[AttunementPhase::Inventory],
        precondition: AttunementPrecondition::Always,
    },
];

pub fn attunement_capability(phase: AttunementPhase) -> &'static AttunementCapability {
    ATTUNEMENT_CAPABILITIES
        .iter()
        .find(|capability| capability.phase == phase)
        .expect("every attunement phase has a catalogue entry")
}

/// The ordered subset of capabilities the evidence selects. Pure.
pub fn derive_attunement_phases(evidence: &AttunementEvidence) -> Vec<AttunementPhase> {
    let mut selected: Vec<AttunementPhase> = Vec::new();
    for capability in &ATTUNEMENT_CAPABILITIES {
        if !evidence.available.contains(&capability.phase)
            || !capability
                .requires
                .iter()
                .all(|required| selected.contains(required))
        {
            continue;
        }
        let holds = match capability.precondition {
            AttunementPrecondition::Always => true,
            AttunementPrecondition::BoundGrammar => !evidence.bound_grammars.is_empty(),
            AttunementPrecondition::AnyFile => evidence.files > 0,
            AttunementPrecondition::EmbeddingBinding => evidence.embedding_binding,
            AttunementPrecondition::AnyOf(any) => any.iter().any(|phase| selected.contains(phase)),
        };
        if holds {
            selected.push(capability.phase);
        }
    }
    selected
}

/// Why a derived step is in the plan, from the evidence that selected it.
/// It states counts and bound grammars only, never what a step will achieve.
fn derived_step_description(
    phase: AttunementPhase,
    evidence: &AttunementEvidence,
    estimated_items: u64,
) -> Result<String> {
    let name = serde_json::to_value(phase)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| crate::ContractError("attunement phase has no name".into()))?;
    let grammars = evidence
        .bound_grammars
        .iter()
        .map(CanonicalId::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    Ok(match phase {
        AttunementPhase::Parse
        | AttunementPhase::Normalize
        | AttunementPhase::EntityLink
        | AttunementPhase::Embed
        | AttunementPhase::Graph => format!(
            "{name}: selected from project evidence; {estimated_items} files with a bound grammar ({grammars})"
        ),
        AttunementPhase::LexicalIndex => format!(
            "{name}: selected from project evidence; {} files",
            evidence.files
        ),
        AttunementPhase::Connect
        | AttunementPhase::Inventory
        | AttunementPhase::VectorIndex
        | AttunementPhase::Ground
        | AttunementPhase::Verify => format!("{name}: selected from project evidence"),
    })
}

/// A derived step's configuration digest binds the capability identity and
/// the evidence it was derived from, so two plans over different evidence
/// never share a checkpoint chain.
pub fn attunement_capability_configuration_sha256(
    phase: AttunementPhase,
    version: u16,
    evidence_sha256: &str,
) -> Result<String> {
    digest_json(
        b"rrflow-attunement-capability-v1",
        &(phase, version, evidence_sha256),
    )
}

/// Default bindings and limits for a derived Parse step when the predecessor
/// plan did not carry an operator-selected policy. Extensions follow the
/// grammar crate's language ids; limits sit inside the bundled inventory
/// policy so the engine's profile check admits them.
pub fn default_source_parse_policy(
    bound_grammars: &BTreeSet<CanonicalId>,
) -> Result<Option<SourceParsePolicy>> {
    let mut bindings: Vec<SourceParseBinding> = Vec::new();
    for language in bound_grammars {
        let extensions: &[&str] = match language.as_str() {
            "go" => &["go"],
            "python" => &["py", "pyi"],
            "rust" => &["rs"],
            _ => &[],
        };
        for extension in extensions {
            bindings.push(SourceParseBinding {
                extension: (*extension).to_owned(),
                language: language.clone(),
            });
        }
    }
    if bindings.is_empty() {
        return Ok(None);
    }
    bindings.sort_by(|left, right| left.extension.cmp(&right.extension));
    let policy = SourceParsePolicy {
        format_version: 1,
        bindings,
        max_files: 100_000,
        max_file_bytes: 8 * 1024 * 1024,
        max_input_bytes: 1024 * 1024 * 1024,
        max_nodes: 16_777_216,
        max_depth: 512,
        max_output_bytes: 64 * 1024 * 1024,
        max_commit_bytes: 16 * 1024 * 1024,
        max_elapsed_ms: 30_000,
        max_memory_bytes: 512 * 1024 * 1024,
    };
    policy.validate()?;
    Ok(Some(policy))
}

/// Derive the successor plan from evidence. The predecessor supplies the
/// target, the duration estimates and an operator-selected parse policy if
/// it carried one; everything else comes from the evidence.
pub fn derive_attunement_plan(
    predecessor: &AttunementPlan,
    evidence: &AttunementEvidence,
    id: CanonicalId,
    source_sha256: String,
) -> Result<AttunementPlan> {
    evidence.validate()?;
    let selected = derive_attunement_phases(evidence);
    if selected.is_empty() || selected.len() > MAX_ATTUNEMENT_PHASES {
        return invalid("attunement derivation selected no bounded sequence");
    }
    let parsed_files: u64 = evidence
        .bound_grammars
        .iter()
        .filter_map(|language| evidence.languages.get(language))
        .sum();
    let inherited_policy = predecessor
        .phases
        .iter()
        .find(|step| step.phase == AttunementPhase::Parse)
        .and_then(|step| step.parse_policy.clone());
    let mut phases = Vec::with_capacity(selected.len());
    for (index, phase) in selected.iter().copied().enumerate() {
        let capability = attunement_capability(phase);
        let (estimated_items, estimated_input_bytes) = match phase {
            AttunementPhase::Connect | AttunementPhase::Inventory | AttunementPhase::Verify => {
                (1, 0)
            }
            AttunementPhase::Parse
            | AttunementPhase::Normalize
            | AttunementPhase::EntityLink
            | AttunementPhase::Embed
            | AttunementPhase::Graph => (parsed_files, evidence.content_bytes),
            AttunementPhase::LexicalIndex => (evidence.files, evidence.content_bytes),
            AttunementPhase::VectorIndex | AttunementPhase::Ground => (0, 0),
        };
        let parse_policy = if phase == AttunementPhase::Parse {
            match &inherited_policy {
                Some(policy) => Some(policy.clone()),
                None => default_source_parse_policy(&evidence.bound_grammars)?,
            }
        } else {
            None
        };
        phases.push(AttunementPhasePlan {
            phase,
            sequence: u16::try_from(index + 1)
                .map_err(|_| crate::ContractError("attunement step index overflow".into()))?,
            description: derived_step_description(phase, evidence, estimated_items)?,
            configuration_sha256: attunement_capability_configuration_sha256(
                phase,
                capability.version,
                &evidence.evidence_sha256,
            )?,
            estimated_items,
            estimated_input_bytes,
            parse_policy,
        });
    }
    let mut plan = AttunementPlan {
        contract_version: predecessor.contract_version,
        id,
        target: predecessor.target.clone(),
        source_sha256,
        phases,
        estimated_min_duration_ms: predecessor.estimated_min_duration_ms,
        estimated_max_duration_ms: predecessor.estimated_max_duration_ms,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = attunement_plan_sha256(&plan)?;
    plan.validate()?;
    Ok(plan)
}
