//! Search execution and prepared vector searches.

use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchExecution {
    pub plan: SearchPlan,
    pub hits: Vec<SearchHit>,
}

/// A sealed, request-bound planner result. Private fields prevent callers from
/// substituting a cheaper or stale access path between planning and execution.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreparedVectorSearch {
    pub(super) request_digest: String,
    pub(super) catalog_revision: u64,
    pub(super) ef_search: usize,
    pub(super) plan_digest: String,
    pub(super) selected_stamp: ProjectionStamp,
    pub(super) plan: SearchPlan,
}

impl PreparedVectorSearch {
    pub fn request_digest(&self) -> &str {
        &self.request_digest
    }

    pub const fn catalog_revision(&self) -> u64 {
        self.catalog_revision
    }

    pub const fn ef_search(&self) -> usize {
        self.ef_search
    }

    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    pub fn selected_stamp(&self) -> &ProjectionStamp {
        &self.selected_stamp
    }

    pub fn plan(&self) -> &SearchPlan {
        &self.plan
    }

    pub(super) fn validate(&self) -> Result<()> {
        self.selected_stamp.validate()?;
        if self.ef_search == 0 || self.ef_search > 1_000_000 {
            return invalid("prepared vector ef_search must be in 1..=1000000");
        }
        if self.plan_digest
            != prepared_plan_digest(
                &self.request_digest,
                self.catalog_revision,
                self.ef_search,
                &self.selected_stamp,
                &self.plan,
            )?
        {
            return invalid("prepared vector plan digest does not match its coordinates");
        }
        Ok(())
    }
}
