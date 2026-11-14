//! Path counters and the metered reader.

use super::*;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PathCounters {
    pub(super) point_reads: u64,
    pub(super) range_scans: u64,
    pub(super) keys_examined: u64,
    pub(super) values_decoded: u64,
    pub(super) decoded_bytes: u64,
}

impl PathCounters {
    pub(super) fn add(
        &mut self,
        point_reads: u64,
        range_scans: u64,
        keys_examined: u64,
        values_decoded: u64,
        decoded_bytes: u64,
    ) -> Result<()> {
        self.point_reads = checked_add(self.point_reads, point_reads)?;
        self.range_scans = checked_add(self.range_scans, range_scans)?;
        self.keys_examined = checked_add(self.keys_examined, keys_examined)?;
        self.values_decoded = checked_add(self.values_decoded, values_decoded)?;
        self.decoded_bytes = checked_add(self.decoded_bytes, decoded_bytes)?;
        Ok(())
    }
}

pub(super) struct MeteredRead<'a, R: AccessRead + ?Sized> {
    inner: &'a R,
    budget: RuntimeReadBudget,
    budget_observed: Cell<Option<u64>>,
    path: Cell<RuntimeReadAccessPath>,
    totals: RefCell<PathCounters>,
    paths: RefCell<BTreeMap<RuntimeReadAccessPath, PathCounters>>,
}

impl<'a, R: AccessRead + ?Sized> MeteredRead<'a, R> {
    pub(super) fn new(inner: &'a R, budget: RuntimeReadBudget) -> Self {
        Self {
            inner,
            budget,
            budget_observed: Cell::new(None),
            path: Cell::new(RuntimeReadAccessPath::ReadStamp),
            totals: RefCell::new(PathCounters::default()),
            paths: RefCell::new(BTreeMap::new()),
        }
    }

    pub(super) fn with_path<T>(
        &self,
        path: RuntimeReadAccessPath,
        operation: impl FnOnce() -> T,
    ) -> T {
        let prior = self.path.replace(path);
        let result = operation();
        self.path.set(prior);
        result
    }

    pub(super) fn preserve_budget_error<T>(&self, result: Result<T>) -> Result<T> {
        if let Some(observed) = self.budget_observed.get() {
            return Err(Error::RuntimeReadBudgetExceeded {
                limit: self.budget.max_keys,
                observed,
            });
        }
        result
    }

    fn record(
        &self,
        point_reads: u64,
        range_scans: u64,
        keys_examined: u64,
        values_decoded: u64,
        decoded_bytes: u64,
    ) -> Result<()> {
        self.totals.borrow_mut().add(
            point_reads,
            range_scans,
            keys_examined,
            values_decoded,
            decoded_bytes,
        )?;
        self.paths
            .borrow_mut()
            .entry(self.path.get())
            .or_default()
            .add(
                point_reads,
                range_scans,
                keys_examined,
                values_decoded,
                decoded_bytes,
            )
    }

    pub(super) fn finish(
        self,
        stamp_validation: RuntimeReadValidation,
    ) -> Result<RuntimeReadEvidence> {
        let totals = self.totals.into_inner();
        let paths = self
            .paths
            .into_inner()
            .into_iter()
            .map(|(path, counters)| RuntimeReadPathEvidence {
                path,
                point_reads: counters.point_reads,
                range_scans: counters.range_scans,
                keys_examined: counters.keys_examined,
                values_decoded: counters.values_decoded,
                decoded_bytes: counters.decoded_bytes,
            })
            .collect();
        Ok(RuntimeReadEvidence {
            contract_version: RUNTIME_VERSIONED_READ_CONTRACT_VERSION,
            key_budget: self.budget.max_keys,
            point_reads: totals.point_reads,
            range_scans: totals.range_scans,
            keys_examined: totals.keys_examined,
            values_decoded: totals.values_decoded,
            decoded_bytes: totals.decoded_bytes,
            stamp_validation,
            paths,
        })
    }
}

impl<R: AccessRead + ?Sized> AccessRead for MeteredRead<'_, R> {
    fn read_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let observed = checked_add(self.totals.borrow().keys_examined, 1)?;
        if observed > self.budget.max_keys {
            self.budget_observed.set(Some(observed));
            return Err(Error::RuntimeReadBudgetExceeded {
                limit: self.budget.max_keys,
                observed,
            });
        }
        let value = self.inner.read_key(key)?;
        let (decoded, bytes) = value.as_ref().map_or((0, 0), |value| {
            (1, u64::try_from(value.len()).unwrap_or(u64::MAX))
        });
        self.record(1, 0, 1, decoded, bytes)?;
        Ok(value)
    }

    fn scan_range(
        &self,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let examined = self.totals.borrow().keys_examined;
        let remaining = self.budget.max_keys.saturating_sub(examined);
        let probe_limit = usize::try_from(remaining.saturating_add(1))
            .unwrap_or(usize::MAX)
            .min(limit);
        let rows = self.inner.scan_range(start, end, probe_limit)?;
        let row_count = u64::try_from(rows.len()).unwrap_or(u64::MAX);
        let observed = checked_add(examined, row_count)?;
        if observed > self.budget.max_keys {
            self.budget_observed.set(Some(observed));
            return Err(Error::RuntimeReadBudgetExceeded {
                limit: self.budget.max_keys,
                observed,
            });
        }
        let decoded_bytes = rows.iter().try_fold(0_u64, |total, (_, value)| {
            checked_add(total, u64::try_from(value.len()).unwrap_or(u64::MAX))
        })?;
        self.record(0, 1, row_count, row_count, decoded_bytes)?;
        Ok(rows)
    }
}

pub(super) fn checked_add(left: u64, right: u64) -> Result<u64> {
    left.checked_add(right).ok_or(Error::SequenceOverflow)
}
