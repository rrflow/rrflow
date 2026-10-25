use super::{require, Budgets, Result, StorageManifest, MAX_ATTEMPTS};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyKind {
    SoftFlushTarget,
    CacheCapacity,
    IoRequestBound,
    CompactionTrigger,
    OutputTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyUnit {
    Bytes,
    Versions,
    Segments,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyScope {
    EstateProcess,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyChangeRule {
    NoInstalledStorageReconfigurationCapability,
}

/// Read-only projection from the public DiagnosticStorageSnapshot owner.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectiveSetting {
    pub source: String,
    pub owner: String,
    pub value: u64,
    pub unit: PolicyUnit,
    pub kind: PolicyKind,
    pub scope: PolicyScope,
    pub change_rule: PolicyChangeRule,
}

impl StorageManifest {
    pub fn effective_settings(&self) -> Result<Vec<EffectiveSetting>> {
        self.storage_observation
            .validate()
            .map_err(|e| e.to_string())?;
        let kv = self
            .storage_observation
            .rrflow_kv
            .as_ref()
            .ok_or("rrflowKV physical observation required")?;
        let entries = [
            (
                "write_ahead_log.payload_max_bytes",
                "rrd_lsm::MaintenancePolicy.wal_payload_max_bytes",
                kv.write_ahead_log.payload_max_bytes,
                PolicyUnit::Bytes,
                PolicyKind::SoftFlushTarget,
            ),
            (
                "memtable.max_versions",
                "rrd_lsm::MaintenancePolicy.memtable_max_versions",
                kv.memtable.max_versions,
                PolicyUnit::Versions,
                PolicyKind::SoftFlushTarget,
            ),
            (
                "page_cache.capacity_bytes",
                "rrd_lsm::DatabaseOptions.page_cache_bytes",
                kv.page_cache.capacity_bytes,
                PolicyUnit::Bytes,
                PolicyKind::CacheCapacity,
            ),
            (
                "segment_io.max_request_bytes",
                "rrd_lsm::SegmentIoPolicy.max_request_bytes",
                kv.segment_io.max_request_bytes,
                PolicyUnit::Bytes,
                PolicyKind::IoRequestBound,
            ),
            (
                "segment_set.l0_compaction_trigger",
                "rrd_lsm::CompactionPolicy.l0_compaction_trigger",
                kv.segment_set.l0_compaction_trigger,
                PolicyUnit::Segments,
                PolicyKind::CompactionTrigger,
            ),
            (
                "segment_set.compaction_target_segment_bytes",
                "rrd_lsm::CompactionPolicy.target_segment_bytes",
                kv.segment_set.compaction_target_segment_bytes,
                PolicyUnit::Bytes,
                PolicyKind::OutputTarget,
            ),
        ];
        Ok(entries
            .into_iter()
            .map(|(source, owner, value, unit, kind)| EffectiveSetting {
                source: format!("DiagnosticStorageSnapshot.rrflow_kv.{source}"),
                owner: owner.into(),
                value,
                unit,
                kind,
                scope: PolicyScope::EstateProcess,
                change_rule: PolicyChangeRule::NoInstalledStorageReconfigurationCapability,
            })
            .collect())
    }
}

impl Budgets {
    pub fn validate(&self) -> Result<()> {
        let sum = [
            self.mutable_bytes,
            self.queue_bytes,
            self.maintenance_bytes,
            self.cache_bytes,
            self.decoded_bytes,
            self.retained_bytes,
            self.recovery_memory_bytes,
        ]
        .into_iter()
        .try_fold(0_u64, |sum, part| {
            sum.checked_add(part.get())
                .ok_or("reservation sum overflow")
        })?;
        require(
            sum <= self.process_bytes.get(),
            "disjoint memory reservations exceed process budget",
        )?;
        if let Some(aggregate) = self.aggregate_process_bytes {
            let reserved = sum
                .checked_mul(self.estates.get())
                .ok_or("aggregate reservation overflow")?;
            require(
                self.process_bytes.get() <= aggregate.get()
                    && reserved <= aggregate.get()
                    && aggregate.get() <= self.host_bytes.get(),
                "shared process pool does not contain estate reservations and fit host budget",
            )?;
        } else {
            let host = self
                .process_bytes
                .get()
                .checked_mul(self.estates.get())
                .ok_or("host reservation overflow")?;
            require(
                host <= self.host_bytes.get(),
                "estate reservations exceed host budget",
            )?;
        }
        require(
            self.atomic_batch_bytes.get() <= self.mutable_bytes.get()
                && self.atomic_batch_bytes.get() <= self.queue_bytes.get(),
            "atomic batch must fit mutable and queue reservations",
        )?;
        require(
            self.atomic_mutations.get() <= self.mutable_versions.get(),
            "atomic mutations exceed mutable version budget",
        )?;
        let disk = self
            .temporary_disk_bytes
            .get()
            .checked_add(self.recovery_reserve_bytes.get())
            .ok_or("disk reservation overflow")?;
        require(
            disk < self.disk_bytes.get(),
            "disk must retain canonical capacity beyond temporary and recovery reserve",
        )?;
        require(
            self.recovery_replay_bytes.get() <= self.disk_bytes.get(),
            "recovery replay budget exceeds disk allocation",
        )?;
        require(
            self.queue_wait_ms.get() <= self.run_ms.get()
                && self.recovery_ms.get() <= self.run_ms.get(),
            "wait/recovery deadline exceeds run bound",
        )?;
        require(
            self.attempts.get() <= MAX_ATTEMPTS as u64
                && self.queue_requests.get() <= self.attempts.get(),
            "attempt/queue count exceeds bounded corpus",
        )?;
        Ok(())
    }
}
