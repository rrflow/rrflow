//! Evidence for deriving a successor plan: bounded observations of the
//! committed Inventory, the installed profile and the applied endpoint
//! configuration. Nothing here reads the project tree; the records the
//! Inventory checkpoint published are the only source.

use super::*;
use crate::engine::source_language::source_language;
use rrd_attunement::inventory::EntryKind;
use rrd_contract::{AttunementEvidence, WorkloadClass, ATTUNEMENT_EVIDENCE_FORMAT_VERSION};
use std::collections::{BTreeMap, BTreeSet};

/// The capabilities this engine can execute on this platform. This is the one
/// owner of "what can run"; a profile may offer more, a derived plan never
/// selects more. Parse needs a native supervisor to contain its worker, which
/// exists on Windows and Linux; elsewhere it is reported unavailable rather
/// than planned and refused at its first step.
#[cfg(any(windows, target_os = "linux"))]
pub(in crate::engine) const EXECUTABLE_PHASES: &[AttunementPhase] = &[
    AttunementPhase::Connect,
    AttunementPhase::Inventory,
    AttunementPhase::Parse,
];
#[cfg(not(any(windows, target_os = "linux")))]
pub(in crate::engine) const EXECUTABLE_PHASES: &[AttunementPhase] =
    &[AttunementPhase::Connect, AttunementPhase::Inventory];

/// Evidence from the job's committed Inventory, or `None` when the job has no
/// Inventory checkpoint yet (a successor then only rebinds the plan).
pub(super) fn attunement_evidence(
    engine: &RrdEngine,
    job: &AttunementJob,
    plan: &AttunementPlan,
) -> Result<Option<AttunementEvidence>> {
    let Some(checkpoint) = job
        .checkpoints
        .iter()
        .rev()
        .find(|checkpoint| checkpoint.phase == AttunementPhase::Inventory)
    else {
        return Ok(None);
    };
    let installed = engine.attunement_installed_record()?;
    let (profile, profile_sha256) =
        DefaultInstallProfile::load(installed.installation.profile_id.as_str())?;
    if profile_sha256 != installed.installation.profile_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let limits = &profile.inventory_policy.limits;
    let accepted = inventory::load_accepted_source(
        engine,
        &plan.target.project_id,
        limits.output_bytes.min(
            engine
                .engram_configuration
                .storage
                .transaction_admission_bytes,
        ),
        limits.entries,
    )?;
    if accepted.proposal_sha256 != checkpoint.output_sha256 {
        // The head moved past this job's checkpoint; a derivation must bind
        // the records the checkpoint published, so refuse rather than guess.
        return Err(ServiceError::StorageConflict(
            "committed inventory head differs from the job's Inventory checkpoint".into(),
        ));
    }
    let mut entries = 0u64;
    let mut files = 0u64;
    let mut content_bytes = 0u64;
    let mut languages: BTreeMap<CanonicalId, u64> = BTreeMap::new();
    for entry in accepted.proposal.entries() {
        entries = entries.saturating_add(1);
        if let EntryKind::File { observation, .. } = &entry.kind {
            files = files.saturating_add(1);
            content_bytes = content_bytes.saturating_add(observation.bytes);
            if let Some(language) = source_language(&entry.path) {
                let id = CanonicalId::new(language.id).map_err(contract)?;
                *languages.entry(id).or_insert(0) += 1;
            }
        }
    }
    let bound_grammars: BTreeSet<CanonicalId> = languages
        .keys()
        .filter(|language| rrd_attunement::parse::SUPPORTED_LANGUAGES.contains(&language.as_str()))
        .cloned()
        .collect();
    let (_, configuration) = engine.load_endpoint_configuration()?;
    let embedding_binding = configuration.profile.as_ref().is_some_and(|profile| {
        profile
            .bindings
            .iter()
            .any(|binding| binding.class == WorkloadClass::EmbeddingAndReranking)
    });
    let available: BTreeSet<AttunementPhase> = profile
        .available_capabilities
        .iter()
        .copied()
        .filter(|phase| EXECUTABLE_PHASES.contains(phase))
        .collect();
    let mut evidence = AttunementEvidence {
        format_version: ATTUNEMENT_EVIDENCE_FORMAT_VERSION,
        inventory_sha256: accepted.proposal_sha256,
        entries,
        files,
        content_bytes,
        languages,
        bound_grammars,
        embedding_binding,
        available,
        evidence_sha256: String::new(),
    };
    evidence.evidence_sha256 = evidence.digest().map_err(contract)?;
    evidence.validate().map_err(contract)?;
    Ok(Some(evidence))
}
