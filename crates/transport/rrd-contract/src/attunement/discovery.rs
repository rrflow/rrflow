//! Bounded discovery of stored installed jobs, not current source verification.
use super::*;

pub const MAX_ATTUNEMENT_JOB_PAGE: u16 = 64;
pub const MAX_ATTUNEMENT_JOB_PAGE_BYTES: usize = 256 * 1024;

fn page_limit_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type": "integer", "minimum": 1, "maximum": MAX_ATTUNEMENT_JOB_PAGE})
}

fn summaries_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let mut schema = <Vec<AttunementJobSummary>>::json_schema(generator);
    schema.insert(
        "maxItems".into(),
        serde_json::json!(MAX_ATTUNEMENT_JOB_PAGE),
    );
    schema
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListAttunementJobs {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub after_job_id: Option<CanonicalId>,
    #[schemars(schema_with = "page_limit_schema")]
    pub limit: u16,
}

impl ListAttunementJobs {
    pub fn validate(&self) -> Result<()> {
        validate_attunement_contract_version(self.contract_version)?;
        if self.limit == 0 || self.limit > MAX_ATTUNEMENT_JOB_PAGE {
            return invalid("attunement job page limit is outside the supported bound");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementJobSummary {
    pub job_id: CanonicalId,
    pub predecessor_job_id: Option<CanonicalId>,
    pub generation: u64,
    pub plan_id: CanonicalId,
    pub plan_sha256: String,
    pub source_sha256: String,
    pub revision: u64,
    pub state: AttunementJobState,
    pub checkpoint_count: u16,
    pub next_phase: Option<AttunementPhase>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementJobPage {
    #[schemars(schema_with = "attunement_version_schema")]
    pub contract_version: u16,
    pub target: InstalledEngramIdentity,
    /// The control sequence was unchanged throughout this page's collection.
    /// Later pages are new reads, not one retained cross-page snapshot.
    pub control_sequence: u64,
    pub observed_at_unix_ms: u64,
    pub after_job_id: Option<CanonicalId>,
    #[schemars(schema_with = "summaries_schema")]
    pub jobs: Vec<AttunementJobSummary>,
    pub next_after_job_id: Option<CanonicalId>,
    pub source_verification: AttunementSourceVerification,
}

impl AttunementJobPage {
    pub fn validate_for(&self, request: &ListAttunementJobs, instance: &CanonicalId) -> Result<()> {
        request.validate()?;
        validate_attunement_contract_version(self.contract_version)?;
        self.target.validate()?;
        if &self.target.instance_id != instance
            || self.control_sequence == 0
            || self.observed_at_unix_ms == 0
            || self.after_job_id != request.after_job_id
            || self.jobs.len() > usize::from(request.limit)
            || (request.after_job_id.is_none() && self.jobs.is_empty())
        {
            return invalid("attunement discovery page identity or bounds differ from request");
        }
        let mut predecessor = request.after_job_id.as_ref();
        let mut generation: Option<u64> = None;
        let mut seen = BTreeSet::new();
        if let Some(id) = predecessor {
            seen.insert(id);
        }
        for job in &self.jobs {
            validate_sha256(&job.plan_sha256, "attunement discovered plan digest")?;
            validate_sha256(&job.source_sha256, "attunement discovered source digest")?;
            if !seen.insert(&job.job_id)
                || job.predecessor_job_id.as_ref() != predecessor
                || (job.generation == 0) != predecessor.is_none()
                || generation.is_some_and(|old| old.checked_add(1) != Some(job.generation))
                || job.revision == 0
                || usize::from(job.checkpoint_count) > MAX_ATTUNEMENT_PHASES
                || job.created_at_unix_ms == 0
                || job.updated_at_unix_ms < job.created_at_unix_ms
                || job.updated_at_unix_ms > self.observed_at_unix_ms
            {
                return invalid("attunement discovery summary has inconsistent lineage or state");
            }
            predecessor = Some(&job.job_id);
            generation = Some(job.generation);
        }
        if let Some(next) = &self.next_after_job_id {
            if self.jobs.len() != usize::from(request.limit)
                || self.jobs.last().map(|job| &job.job_id) != Some(next)
            {
                return invalid("attunement continuation does not name the page's final job");
            }
        }
        let bytes = serde_json::to_vec(self)
            .map_err(|error| crate::ContractError(format!("attunement page encoding: {error}")))?;
        if bytes.len() > MAX_ATTUNEMENT_JOB_PAGE_BYTES {
            return invalid("attunement discovery page exceeds its byte bound");
        }
        Ok(())
    }
}
