//! `RrdEngine`-owned embedded and operator operations.
//!
//! This module is private: outward adapters receive [`crate::RrdEngine`] and
//! engine-owned value types, never a second storage-opening handle or an
//! `rrd_store::StorageEngine` escape.

use crate::RrdEngine;
use rrd_core::{RuntimeCommit, RuntimeMutation};
use rrd_store::StorageEngine as _;
use std::path::Path;

pub use rrd_core::{
    digest, Claim, ClaimReader, Millis, Predicate, Producer, Reader, ScopeId, Subject,
};
pub use rrd_store::{
    BackupCatalogue, BackupEntry, Invocation as OperatorInvocation,
    InvocationInput as OperatorInvocationInput, LogicalArchiveInventory, LogicalRestoreReport,
    Outcome, RemovalReport, Trigger,
};

pub type OperatorResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl RrdEngine {
    fn persistent_storage(&self) -> OperatorResult<&rrd_store::RrflowKvStore> {
        self.storage
            .as_rrflow_kv()
            .ok_or_else(|| "operation requires a persistent RRD root".into())
    }

    pub fn path(&self) -> OperatorResult<&Path> {
        self.storage_root
            .as_ref()
            .map(|root| root.diagnostic_path())
            .ok_or_else(|| "in-memory RRD has no persistent path".into())
    }

    pub fn backend_name(&self) -> &'static str {
        self.storage.backend_name()
    }

    pub fn record_operator_invocation(
        &self,
        input: OperatorInvocationInput<'_>,
    ) -> OperatorResult<OperatorInvocation> {
        Ok(self.persistent_storage()?.invocations().record(input)?)
    }

    pub fn invocations_since(&self, since: Millis) -> OperatorResult<Vec<OperatorInvocation>> {
        Ok(self.persistent_storage()?.invocations().since(since)?)
    }

    pub fn invocation_count(&self) -> OperatorResult<u64> {
        Ok(self.persistent_storage()?.invocations().count()?)
    }

    pub fn access_count(&self) -> OperatorResult<usize> {
        Ok(self.persistent_storage()?.claims().access_count()?)
    }

    pub fn sequence(&self) -> OperatorResult<u64> {
        Ok(self.storage.claims().sequence()?)
    }

    pub fn observe(
        &self,
        reader: &Reader,
        subject: &Subject,
        predicate: &Predicate,
        at: Millis,
    ) -> OperatorResult<()> {
        Ok(self
            .storage
            .claims()
            .observe(reader, subject, predicate, at)?)
    }

    pub fn assert_claim(&self, claim: &Claim) -> OperatorResult<rrd_store::AppendOutcome> {
        claim.validate()?;
        let _gate = self.admit_transaction(claim, "", "")?;
        let scope = ScopeId::new(format!("instance:{}", self.instance_id()))?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let previous =
            self.storage
                .claims()
                .as_of(&claim.subject, &claim.predicate, claim.valid_from)?;
        let claims = match previous {
            Some(previous) if previous.valid_from < claim.valid_from => {
                rrd_core::supersede(&previous, claim.clone())?.to_vec()
            }
            _ => vec![claim.clone()],
        };
        let mut mutations =
            Vec::with_capacity(claims.len() + usize::from(read.schema_revision.is_none()));
        if read.schema_revision.is_none() {
            let mut schema = rrd_core::RuntimeSchemaRegistry::empty(
                1,
                "install canonical reasoning claim model",
            );
            schema.tables.insert(
                rrd_core::RuntimeType::new("claim")?,
                rrd_core::RuntimeTableSchema::strict(rrd_core::RuntimeLogicalModel::ReasoningClaim),
            );
            mutations.push(RuntimeMutation::Schema { registry: schema });
        }
        mutations.extend(
            claims
                .iter()
                .cloned()
                .map(|claim| RuntimeMutation::Claim { claim }),
        );
        let outcome = self.storage.runtime().commit(&RuntimeCommit {
            scope,
            at: claim.tx_time,
            actor: claim.producer.actor.clone(),
            expected_cursor: read.commit_cursor,
            mutations,
        })?;
        Ok(rrd_store::AppendOutcome {
            first_sequence: outcome
                .first_claim_sequence
                .ok_or("canonical claim commit did not assign a claim sequence")?,
            last_sequence: outcome
                .last_claim_sequence
                .ok_or("canonical claim commit did not assign a claim sequence")?,
            count: claims.len(),
        })
    }

    pub fn claim_as_of(
        &self,
        subject: &Subject,
        predicate: &Predicate,
        at: Millis,
    ) -> OperatorResult<Option<Claim>> {
        Ok(self.storage.claims().as_of(subject, predicate, at)?)
    }

    pub fn claim_history(
        &self,
        subject: &Subject,
        predicate: &Predicate,
    ) -> OperatorResult<Vec<Claim>> {
        Ok(self.storage.claims().history(subject, predicate)?)
    }

    pub fn removal_report(
        &self,
        since: Millis,
        evaluated_at: Millis,
    ) -> OperatorResult<RemovalReport> {
        Ok(self
            .persistent_storage()?
            .claims()
            .removal_report(since, evaluated_at)?)
    }

    pub fn export_logical_archive(
        &self,
        archive: &Path,
    ) -> OperatorResult<LogicalArchiveInventory> {
        Ok(rrd_store::export_logical_archive(&self.storage, archive)?)
    }

    pub fn create_logical_backup(
        &self,
        catalogue: &Path,
        label: &str,
        now: Millis,
    ) -> OperatorResult<BackupEntry> {
        Ok(rrd_store::create_logical_backup(
            &self.storage,
            catalogue,
            label,
            now,
        )?)
    }

    pub fn execute_operator_query(
        &self,
        scope: ScopeId,
        source: &str,
        parameters: &crate::Parameters,
        budget: &crate::ExecutionBudget,
        actor: &str,
        at: Millis,
    ) -> OperatorResult<crate::TracedQueryExecution> {
        crate::execute_traced_query(&self.storage, scope, source, parameters, budget, actor, at)
    }

    pub fn inspect_logical_archive(archive: &Path) -> OperatorResult<LogicalArchiveInventory> {
        Ok(rrd_store::inspect_logical_archive(archive)?)
    }

    pub fn restore_logical_archive(
        archive: &Path,
        db: &Path,
        now: Millis,
    ) -> OperatorResult<LogicalRestoreReport> {
        Ok(rrd_store::restore_logical_archive_to_new_root(
            archive, db, now,
        )?)
    }

    pub fn verify_backup_catalogue(
        catalogue: &(impl rrd_store::filesystem::AnchorInput + ?Sized),
    ) -> OperatorResult<BackupCatalogue> {
        Ok(rrd_store::verify_backup_catalogue(catalogue)?)
    }

    pub fn restore_catalogued_backup(
        catalogue: &Path,
        backup_id: &str,
        db: &Path,
        now: Millis,
    ) -> OperatorResult<LogicalRestoreReport> {
        Ok(rrd_store::restore_catalogued_backup(
            catalogue, backup_id, db, now,
        )?)
    }
}
