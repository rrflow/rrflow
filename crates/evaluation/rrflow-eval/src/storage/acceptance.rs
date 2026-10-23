//! C-07 evidence reconciliation. This reports readiness for owner review;
//! it never changes roadmap, release or repository lifecycle state.
use super::sustained::read_bounded;
use super::*;
#[path = "acceptance_latency.rs"]
mod nominal;
pub use nominal::NominalObservation;
#[path = "acceptance_launch.rs"]
mod launch;
pub use launch::StorageLaunchReceipt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceSource {
    pub commit: String,
    pub tree: String,
    pub public_catalogue_sha256: Digest,
}
impl AcceptanceSource {
    pub fn validate(&self) -> Result<()> {
        for value in [&self.commit, &self.tree] {
            require(
                value.len() == 40
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "acceptance source requires exact lowercase Git revisions",
            )?;
        }
        require(
            self.public_catalogue_sha256 == catalogue_digest()?,
            "acceptance catalogue differs from executable owner",
        )
    }
    fn binds(&self, candidate: &Candidate) -> Result<()> {
        require(
            candidate.clean
                && candidate.commit == self.commit
                && candidate.tree == self.tree
                && candidate.public_catalogue_sha256 == self.public_catalogue_sha256,
            "campaign source differs from exact tested candidate",
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateCiStep {
    pub name: String,
    pub status: String,
    pub conclusion: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateCiJob {
    pub id: Limit,
    pub name: String,
    pub status: String,
    pub conclusion: String,
    #[schemars(length(max = 128))]
    pub steps: Vec<CandidateCiStep>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateCiReceipt {
    pub format_version: u16,
    pub repository: String,
    pub run_id: Limit,
    pub run_attempt: Limit,
    pub event: String,
    /// PR head and actual reusable-workflow/checkout revision remain distinct.
    pub head_commit: String,
    pub tested_commit: String,
    pub tested_tree: String,
    pub run_api_sha256: Digest,
    pub jobs_api_sha256: Digest,
    pub commit_api_sha256: Digest,
    #[schemars(length(min = 1, max = 128))]
    pub jobs: Vec<CandidateCiJob>,
}
impl CandidateCiReceipt {
    pub fn validate(&self, source: &AcceptanceSource) -> Result<()> {
        source.validate()?;
        require(
            self.format_version == 1
                && self.repository == "rrflow/rrflow-development"
                && self.tested_commit == source.commit
                && self.tested_tree == source.tree,
            "CI receipt does not bind the tested source",
        )?;
        require(
            matches!(
                self.event.as_str(),
                "pull_request" | "merge_group" | "workflow_dispatch"
            ),
            "CI event is not a candidate pipeline",
        )?;
        require(
            self.head_commit.len() == 40
                && self
                    .head_commit
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "CI head revision malformed",
        )?;
        require(
            !self.jobs.is_empty() && self.jobs.len() <= 128,
            "CI job count invalid",
        )?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for job in &self.jobs {
            require(
                ids.insert(job.id.get())
                    && names.insert(&job.name)
                    && !job.name.is_empty()
                    && job.name.len() <= 256
                    && job.status == "completed"
                    && job.conclusion == "success"
                    && job.steps.len() <= 128,
                "CI contains duplicate, incomplete or failed jobs",
            )?;
            for step in &job.steps {
                require(
                    !step.name.is_empty()
                        && step.name.len() <= 256
                        && step.status == "completed"
                        && matches!(step.conclusion.as_str(), "success" | "skipped"),
                    "CI step is not complete or failed",
                )?;
            }
        }
        self.job("pipeline / ci-gate")?;
        self.job("pipeline / installed RRFlow lifecycle")?;
        for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
            let crash = self.job(&format!("pipeline / installed domain crash ({os})"))?;
            Self::step(
                crash,
                "Test exact installed domain boundaries and compound recovery",
            )?;
            Self::step(
                crash,
                "Qualify installed native capacity failure and recovery",
            )?;
            let reopen = self.job(&format!("pipeline / installed 100-cycle reopen ({os})"))?;
            for name in [
                "Require both complete ordinary-primary receipts",
                "Require fresh and existing installed damage denial",
                "Exercise installed capacity workload and backup readback",
                "Independent installed estates under concurrent load",
            ] {
                Self::step(reopen, name)?;
            }
        }
        for os in ["macos-latest", "windows-latest"] {
            self.job(&format!("pipeline / estate process ({os})"))?;
        }
        Ok(())
    }
    fn job(&self, name: &str) -> Result<&CandidateCiJob> {
        self.jobs
            .iter()
            .find(|j| j.name == name)
            .ok_or_else(|| format!("required candidate job missing: {name}"))
    }
    fn step(job: &CandidateCiJob, name: &str) -> Result<()> {
        require(
            job.steps
                .iter()
                .filter(|s| s.name == name && s.status == "completed" && s.conclusion == "success")
                .count()
                == 1,
            "required native CI step did not succeed exactly once",
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCampaign {
    /// One non-symlink directory directly below the evidence root.
    pub directory: Id,
    pub campaign_sha256: Digest,
    pub result_sha256: Option<Digest>,
    pub launch_sha256: Option<Digest>,
    pub status: CampaignStatus,
    /// Missing results must be declared, never silently omitted from coverage.
    pub absence_reason: Option<Id>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorageAcceptanceManifest {
    pub format_version: u16,
    pub source: AcceptanceSource,
    pub owner_projection_sha256: Digest,
    pub ci_directory: Id,
    pub ci_receipt_sha256: Digest,
    #[schemars(length(min = 1, max = 128))]
    pub campaigns: Vec<AcceptanceCampaign>,
}
impl StorageAcceptanceManifest {
    pub fn validate(&self) -> Result<()> {
        self.source.validate()?;
        require(
            self.format_version == 1
                && self.owner_projection_sha256 == capacity_profiles()?.owner_projection_sha256,
            "acceptance profile projection is stale",
        )?;
        require(
            !self.campaigns.is_empty() && self.campaigns.len() <= 128,
            "acceptance campaign count invalid",
        )?;
        let mut paths = BTreeSet::new();
        let mut identities = BTreeSet::new();
        paths.insert(self.ci_directory.as_str());
        for item in &self.campaigns {
            require(
                paths.insert(item.directory.as_str())
                    && identities.insert(item.campaign_sha256.as_str()),
                "duplicate acceptance directory or campaign",
            )?;
            require(
                item.result_sha256.is_some()
                    || matches!(
                        item.status,
                        CampaignStatus::NotRun | CampaignStatus::Interrupted
                    ),
                "unsealed evidence cannot declare completion or a verified failure",
            )?;
            require(
                item.result_sha256.is_some() != item.absence_reason.is_some(),
                "campaign needs exactly one result identity or explicit absence",
            )?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptedOperatingPoint {
    pub directory: Id,
    pub profile: Id,
    pub platform: NativePlatform,
    pub clients: u64,
    pub requests_per_second: u64,
    pub status: CampaignStatus,
    pub qualification_passed: bool,
    pub evidence_verified: bool,
    pub launch_status: Option<CampaignStatus>,
    pub launch_cleanup_confirmed: bool,
    pub launch_reason: Option<Id>,
    pub absence_reason: Option<Id>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QualifiedEnvelope {
    pub clients: u64,
    pub maximum_tested_rate: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileCoverage {
    pub profile: Id,
    pub attempted_points: u64,
    pub qualified_points: u64,
    pub missing_coordinates: Vec<String>,
    /// Largest actually qualified rate; zero means no qualified observation.
    pub largest_qualified_rate: u64,
    pub qualified_envelopes: Vec<QualifiedEnvelope>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorageAcceptanceReport {
    pub format_version: u16,
    pub manifest_sha256: Digest,
    pub source: AcceptanceSource,
    pub ci_run_id: Limit,
    pub ci_run_attempt: Limit,
    pub candidate_ci_passed: bool,
    pub points: Vec<AcceptedOperatingPoint>,
    pub profiles: Vec<ProfileCoverage>,
    pub matrix_complete: bool,
    pub capacity_qualified: bool,
    pub nominal_thresholds_passed: bool,
    #[schemars(length(max = 1536))]
    pub nominal_observations: Vec<NominalObservation>,
    /// Conservative evidence readiness, not an automatic C-07/alpha status change.
    pub ready_for_owner_review: bool,
}
fn owned_directory(root: &Path, name: &Id) -> Result<std::path::PathBuf> {
    let path = root.join(name.as_str());
    let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    require(
        metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && path.canonicalize().map_err(|e| e.to_string())?.parent() == Some(root),
        "acceptance member must be a direct non-symlink directory",
    )?;
    Ok(path)
}
fn raw_sha256(bytes: &[u8]) -> Digest {
    use sha2::{Digest as _, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .try_into()
        .expect("SHA256 encoding")
}

/// Reconstruct the normalized receipt from retained primary API records.
/// This checks internal provenance; only the collector's live authenticated
/// request establishes where those bytes were obtained.
pub fn candidate_ci_from_api(
    run_raw: &[u8],
    jobs_raw: &[u8],
    commit_raw: &[u8],
) -> Result<CandidateCiReceipt> {
    use serde_json::Value;
    let run: Value = decode(run_raw)?;
    let jobs: Value = decode(jobs_raw)?;
    let commit: Value = decode(commit_raw)?;
    let text = |v: &Value, key: &str| -> Result<String> {
        v.get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty() && s.len() <= 512)
            .map(str::to_owned)
            .ok_or_else(|| format!("CI API field missing or oversized: {key}"))
    };
    let number = |v: &Value, key: &str| -> Result<u64> {
        v.get(key)
            .and_then(Value::as_u64)
            .filter(|n| *n > 0 && *n <= MAX_EXACT_INTEGER)
            .ok_or_else(|| format!("CI API number invalid: {key}"))
    };
    require(
        text(&run, "status")? == "completed"
            && text(&run, "conclusion")? == "success"
            && text(&run, "path")? == ".github/workflows/ci.yml",
        "CI API run did not complete the owning pipeline successfully",
    )?;
    let repository = text(
        run.get("repository").ok_or("CI repository absent")?,
        "full_name",
    )?;
    require(
        repository == "rrflow/rrflow-development",
        "CI API repository differs",
    )?;
    let referenced = run
        .get("referenced_workflows")
        .and_then(Value::as_array)
        .ok_or("CI reusable source missing")?;
    require(referenced.len() == 1, "CI reusable source is ambiguous")?;
    let tested_commit = text(&referenced[0], "sha")?;
    require(
        text(&referenced[0], "path")?
            == format!(
                "rrflow/rrflow-development/.github/workflows/ci-reusable.yml@{tested_commit}"
            ),
        "CI reusable workflow path differs",
    )?;
    require(
        text(&commit, "sha")? == tested_commit,
        "CI API commit differs from tested workflow revision",
    )?;
    let tested_tree = text(commit.get("tree").ok_or("CI commit tree absent")?, "sha")?;
    let run_id = number(&run, "id")?;
    let attempt = number(&run, "run_attempt")?;
    let head_commit = text(&run, "head_sha")?;
    let rows = jobs
        .as_array()
        .ok_or("CI jobs must be bounded collected API array")?;
    require(
        !rows.is_empty() && rows.len() <= 128,
        "CI API jobs count invalid",
    )?;
    let mut projected = Vec::new();
    for job in rows {
        require(
            number(job, "run_id")? == run_id
                && number(job, "run_attempt")? == attempt
                && text(job, "head_sha")? == head_commit,
            "CI job belongs to another run/attempt/source",
        )?;
        let steps = job
            .get("steps")
            .and_then(Value::as_array)
            .ok_or("CI job steps absent")?;
        require(steps.len() <= 128, "CI API steps count invalid")?;
        let steps = steps
            .iter()
            .map(|step| {
                Ok(CandidateCiStep {
                    name: text(step, "name")?,
                    status: text(step, "status")?,
                    conclusion: text(step, "conclusion")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        projected.push(CandidateCiJob {
            id: number(job, "id")?.try_into()?,
            name: text(job, "name")?,
            status: text(job, "status")?,
            conclusion: text(job, "conclusion")?,
            steps,
        });
    }
    let receipt = CandidateCiReceipt {
        format_version: 1,
        repository,
        run_id: run_id.try_into()?,
        run_attempt: attempt.try_into()?,
        event: text(&run, "event")?,
        head_commit,
        tested_commit,
        tested_tree,
        run_api_sha256: raw_sha256(run_raw),
        jobs_api_sha256: raw_sha256(jobs_raw),
        commit_api_sha256: raw_sha256(commit_raw),
        jobs: projected,
    };
    receipt.validate(&AcceptanceSource {
        commit: receipt.tested_commit.clone(),
        tree: receipt.tested_tree.clone(),
        public_catalogue_sha256: catalogue_digest()?,
    })?;
    Ok(receipt)
}

pub fn verify_storage_acceptance(
    root: &Path,
    manifest: &StorageAcceptanceManifest,
) -> Result<StorageAcceptanceReport> {
    manifest.validate()?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let ci_root = owned_directory(&root, &manifest.ci_directory)?;
    let ci: CandidateCiReceipt = decode(&read_bounded(
        &ci_root.join("receipt.json"),
        MAX_DOCUMENT_BYTES,
    )?)?;
    require(
        digest("rrflow-storage-ci-receipt-v1", &ci)? == manifest.ci_receipt_sha256,
        "CI receipt identity changed",
    )?;
    ci.validate(&manifest.source)?;
    let run_raw = read_bounded(&ci_root.join("run.json"), MAX_DOCUMENT_BYTES)?;
    let jobs_raw = read_bounded(&ci_root.join("jobs.json"), MAX_DOCUMENT_BYTES)?;
    let commit_raw = read_bounded(&ci_root.join("commit.json"), MAX_DOCUMENT_BYTES)?;
    let derived = candidate_ci_from_api(&run_raw, &jobs_raw, &commit_raw)?;
    require(
        digest("rrflow-storage-ci-receipt-v1", &derived)? == manifest.ci_receipt_sha256,
        "CI receipt projection differs from retained API source",
    )?;
    let mut points = Vec::new();
    let mut nominal_observations = Vec::new();
    for entry in &manifest.campaigns {
        let directory = owned_directory(&root, &entry.directory)?;
        let campaign: StorageCampaign = decode(&read_bounded(
            &directory.join("campaign.json"),
            MAX_DOCUMENT_BYTES,
        )?)?;
        require(
            campaign.sha256()? == entry.campaign_sha256,
            "declared campaign identity changed",
        )?;
        manifest.source.binds(&campaign.storage.candidate)?;
        let profile = capacity_profiles()?
            .profiles
            .into_iter()
            .find(|p| p.id == campaign.storage.profile.as_str())
            .ok_or("unknown acceptance capacity profile")?;
        require(
            campaign.allocation.estates.get() == profile.estates,
            "campaign estate topology differs from owning profile",
        )?;
        let (status, qualified) = if let Some(expected) = &entry.result_sha256 {
            if campaign.allocation.estates.get() > 1 {
                let result = verify_fleet(&directory)?;
                require(
                    digest("rrflow-storage-fleet-result-v1", &result)? == *expected,
                    "fleet result identity changed",
                )?;
                (result.status, result.qualification_passed)
            } else {
                let result = verify_campaign(&directory)?;
                let observations =
                    nominal::measure(&directory, &entry.directory, &campaign, &result)?;
                require(
                    nominal_observations.len() + observations.len() <= 1536,
                    "acceptance nominal report exceeds 512 cohorts",
                )?;
                nominal_observations.extend(observations);
                require(
                    digest("rrflow-storage-campaign-result-v1", &result)? == *expected,
                    "campaign result identity changed",
                )?;
                (result.status, result.qualification_passed)
            }
        } else {
            for name in ["result.json", "fleet-result.json"] {
                require(
                    !directory
                        .join(name)
                        .try_exists()
                        .map_err(|e| e.to_string())?,
                    "declared missing campaign has an unreviewed result",
                )?;
            }
            (entry.status, false)
        };
        require(
            status == entry.status,
            "campaign result status differs from declared acceptance member",
        )?;
        let launch_path = directory.join("launch.json");
        let (launch_status, launch_cleanup_confirmed, launch_reason, launch_passed) =
            if let Some(expected) = &entry.launch_sha256 {
                let receipt: StorageLaunchReceipt =
                    decode(&read_bounded(&launch_path, MAX_LAUNCH_RECEIPT_BYTES)?)?;
                require(
                    digest("rrflow-storage-launch-v1", &receipt)? == *expected,
                    "launch receipt identity changed",
                )?;
                let passed = launch::verify(
                    &directory,
                    &campaign,
                    &receipt,
                    entry.result_sha256.as_ref(),
                )?;
                (
                    Some(receipt.status),
                    receipt.cleanup_confirmed,
                    receipt.reason,
                    passed,
                )
            } else {
                require(
                    !launch_path.try_exists().map_err(|e| e.to_string())?,
                    "unreviewed launch receipt exists",
                )?;
                (None, false, None, false)
            };
        for point in &campaign.points {
            points.push(AcceptedOperatingPoint {
                directory: entry.directory.clone(),
                profile: campaign.storage.profile.clone(),
                platform: campaign.storage.environment.platform,
                clients: point.clients.get(),
                requests_per_second: point.requests_per_second.get(),
                status,
                qualification_passed: qualified && launch_passed,
                evidence_verified: entry.result_sha256.is_some(),
                launch_status,
                launch_cleanup_confirmed,
                launch_reason: launch_reason.clone(),
                absence_reason: entry.absence_reason.clone(),
            });
        }
    }
    let profiles = derive_coverage(&points)?;
    let matrix_complete = profiles.iter().all(|p| p.missing_coordinates.is_empty());
    // A failed higher point remains visible even if a lower sustainable point
    // passes. This conservative readiness flag never silently accepts that risk.
    let capacity_qualified = matrix_complete && points.iter().all(|p| p.qualification_passed);
    let nominal_thresholds_passed = nominal::complete(&nominal_observations)?;
    Ok(StorageAcceptanceReport {
        format_version: 1,
        manifest_sha256: digest("rrflow-storage-acceptance-manifest-v1", manifest)?,
        source: manifest.source.clone(),
        ci_run_id: ci.run_id,
        ci_run_attempt: ci.run_attempt,
        candidate_ci_passed: true,
        points,
        profiles,
        matrix_complete,
        capacity_qualified,
        nominal_thresholds_passed,
        nominal_observations,
        ready_for_owner_review: capacity_qualified && nominal_thresholds_passed,
    })
}
fn derive_coverage(points: &[AcceptedOperatingPoint]) -> Result<Vec<ProfileCoverage>> {
    let mut reports = Vec::new();
    for profile in capacity_profiles()?.profiles {
        let rows = points
            .iter()
            .filter(|p| p.profile.as_str() == profile.id)
            .collect::<Vec<_>>();
        let attempted = rows
            .iter()
            .filter(|r| r.evidence_verified && r.status != CampaignStatus::NotRun)
            .map(|r| (r.clients, r.requests_per_second))
            .collect::<BTreeSet<_>>();
        let qualified = rows
            .iter()
            .filter(|r| r.qualification_passed)
            .map(|r| (r.clients, r.requests_per_second))
            .collect::<BTreeSet<_>>();
        let mut missing = Vec::new();
        if profile.clients.is_empty() && profile.rates.is_empty() {
            if attempted.is_empty() {
                missing.push("declared-operating-point".into());
            }
        } else {
            for clients in &profile.clients {
                if profile.rates.is_empty() {
                    if !attempted.iter().any(|(c, _)| c == clients) {
                        missing.push(format!("clients={clients},rate=declared"));
                    }
                } else {
                    for rate in &profile.rates {
                        if !attempted.contains(&(*clients, *rate)) {
                            missing.push(format!("clients={clients},rate={rate}"));
                        }
                    }
                }
            }
        }
        reports.push(ProfileCoverage {
            profile: profile.id.try_into()?,
            attempted_points: attempted.len() as u64,
            qualified_points: qualified.len() as u64,
            missing_coordinates: missing,
            largest_qualified_rate: qualified.iter().map(|(_, r)| *r).max().unwrap_or(0),
            qualified_envelopes: qualified
                .iter()
                .map(|(c, _)| *c)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|clients| QualifiedEnvelope {
                    clients,
                    maximum_tested_rate: qualified
                        .iter()
                        .filter(|(c, _)| *c == clients)
                        .map(|(_, r)| *r)
                        .max()
                        .unwrap_or(0),
                })
                .collect(),
        });
    }
    Ok(reports)
}

#[cfg(test)]
#[path = "acceptance_tests.rs"]
mod tests;
