use crate::{
    bind, execute, plan, BoundQuery, Catalog, Error, ExecutionBudget, Parameters, PhysicalPlan,
    Query, QueryExecution, Result,
};
use rrd_core::ReadStamp;
use rrd_store::{RuntimeReadBudget, RuntimeReadEvidence, StorageEngine};

#[path = "pipeline/evidence.rs"]
mod evidence;

/// One provider-neutral rrflowQL pipeline bound to a caller-owned read stamp.
///
/// The caller decides when the read coordinate is captured. Every catalogue,
/// bound query, physical plan, and execution produced here must retain that
/// coordinate, including when observability writes advance the live head.
pub struct StampedQueryPipeline<'a, E: StorageEngine> {
    engine: &'a E,
    read: ReadStamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StampedQueryExecution {
    pub bound: BoundQuery,
    pub plan: PhysicalPlan,
    pub execution: QueryExecution,
}

impl<'a, E: StorageEngine> StampedQueryPipeline<'a, E> {
    pub fn new(engine: &'a E, read: ReadStamp) -> Result<Self> {
        read.validate()
            .map_err(|error| Error::Integrity(error.to_string()))?;
        Ok(Self { engine, read })
    }

    pub fn read(&self) -> &ReadStamp {
        &self.read
    }

    pub fn bind(
        &self,
        query: &Query,
        parameters: &Parameters,
        budget: &ExecutionBudget,
    ) -> Result<BoundQuery> {
        self.bind_with_evidence(query, parameters, budget)
            .map(|(bound, _)| bound)
    }

    fn bind_with_evidence(
        &self,
        query: &Query,
        parameters: &Parameters,
        budget: &ExecutionBudget,
    ) -> Result<(BoundQuery, RuntimeReadEvidence)> {
        let catalog = Catalog::capture_for_sources_at(
            self.engine,
            self.read.clone(),
            &[],
            RuntimeReadBudget::new(budget.max_storage_keys)?,
        )?;
        let (candidate, required) = crate::plan::prepare_binding(query, parameters, &catalog)?;
        let (bound, captured) = if catalog.covers_version_sources(&required) {
            (candidate, catalog.read_evidence)
        } else {
            let remaining =
                evidence::remaining(budget.max_storage_keys, catalog.read_evidence.keys_examined)?;
            let full = Catalog::capture_for_query_at(
                self.engine,
                self.read.clone(),
                query,
                RuntimeReadBudget::new(remaining)?,
            )?;
            let bound = bind(query, parameters, &full)?;
            let captured = evidence::combine(
                catalog.read_evidence,
                full.read_evidence,
                budget.max_storage_keys,
            )?;
            (bound, captured)
        };
        self.require_bound_read(&bound)?;
        Ok((bound, captured))
    }

    pub fn plan(&self, bound: &BoundQuery) -> Result<PhysicalPlan> {
        self.require_bound_read(bound)?;
        let physical = plan(bound)?;
        self.require_plan_read(&physical)?;
        Ok(physical)
    }

    pub fn execute(
        &self,
        physical: &PhysicalPlan,
        budget: &ExecutionBudget,
    ) -> Result<QueryExecution> {
        self.require_plan_read(physical)?;
        let execution = execute(self.engine, physical, budget)?;
        let contract = &physical.explanation.contract;
        if execution.plan_digest != physical.digest
            || execution.read_manifest != self.read.manifest_id
            || execution.valid_at != contract.valid_at
            || execution.known_at_cursor != contract.known_at_cursor
        {
            return Err(Error::Integrity(
                "query execution changed its stamped plan coordinate".into(),
            ));
        }
        Ok(execution)
    }

    pub fn run(
        &self,
        query: &Query,
        parameters: &Parameters,
        budget: &ExecutionBudget,
    ) -> Result<StampedQueryExecution> {
        let (bound, captured) = self.bind_with_evidence(query, parameters, budget)?;
        let plan = self.plan(&bound)?;
        let mut execution_budget = budget.clone();
        execution_budget.max_storage_keys =
            evidence::remaining(budget.max_storage_keys, captured.keys_examined)?;
        let mut execution = self.execute(&plan, &execution_budget)?;
        execution.read_evidence =
            evidence::combine(captured, execution.read_evidence, budget.max_storage_keys)?;
        Ok(StampedQueryExecution {
            bound,
            plan,
            execution,
        })
    }

    fn require_bound_read(&self, bound: &BoundQuery) -> Result<()> {
        if bound.read != self.read {
            return Err(Error::Integrity(
                "bound query does not retain the pipeline read stamp".into(),
            ));
        }
        Ok(())
    }

    fn require_plan_read(&self, physical: &PhysicalPlan) -> Result<()> {
        physical.verify()?;
        if physical.logical.read != self.read
            || physical.explanation.contract.read_manifest != self.read.manifest_id
        {
            return Err(Error::Integrity(
                "physical plan does not retain the pipeline read stamp".into(),
            ));
        }
        Ok(())
    }
}
