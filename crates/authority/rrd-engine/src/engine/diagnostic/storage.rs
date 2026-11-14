//! Storage diagnostics, counters and the rrflowKV snapshot.

use super::*;

/// Projects the storage port's physical evidence into the public contract.
/// A profile that reports counters must report every one of them; a missing
/// counter or unknown level fails the read instead of rendering a guess.
pub(super) fn diagnostic_storage(
    engine: &RrdEngine,
    observed_at_unix_ms: u64,
) -> Result<DiagnosticStorageSnapshot> {
    let evidence = engine.storage.physical_store_evidence()?;
    let rrflow_cld = engine.rrflow_cld_snapshot()?;
    if evidence.backend != engine.storage.backend_name() {
        return Err(ServiceError::Storage(format!(
            "storage evidence names backend {} but the instance runs {}",
            evidence.backend,
            engine.storage.backend_name()
        )));
    }
    let (evidence_level, rrflow_kv) = match evidence.evidence_level.as_str() {
        rrd_store::LOGICAL_ONLY_EVIDENCE => (DiagnosticStorageEvidenceLevel::LogicalOnly, None),
        rrd_store::RRFLOW_KV_COUNTER_EVIDENCE => (
            DiagnosticStorageEvidenceLevel::RrflowKvCounters,
            Some(rrflow_kv_snapshot(&evidence, &rrflow_cld)?),
        ),
        level => {
            return Err(ServiceError::Storage(format!(
                "storage reported unknown physical evidence level {level}"
            )))
        }
    };
    Ok(DiagnosticStorageSnapshot {
        observed_at_unix_ms,
        profile: engine.storage_profile_kind,
        evidence_level,
        rrflow_kv,
        admission: if evidence_level == DiagnosticStorageEvidenceLevel::RrflowKvCounters {
            let queue = engine.transaction_admission_snapshot()?;
            Some(DiagnosticStorageAdmission {
                checkpoint_entries: counter(evidence.checkpoint_entries, "checkpoint_entries")?,
                checkpoint_limit: counter(evidence.checkpoint_limit, "checkpoint_limit")?,
                native_transactions: counter(evidence.native_transactions, "native_transactions")?,
                native_transaction_bytes: counter(
                    evidence.native_transaction_bytes,
                    "native_transaction_bytes",
                )?,
                active_read_views: counter(evidence.active_read_views, "active_read_views")?,
                pinned_read_bytes: counter(evidence.pinned_read_bytes, "pinned_read_bytes")?,
                retained_mutable_bytes: counter(
                    evidence.retained_mutable_bytes,
                    "retained_mutable_bytes",
                )?,
                retained_source_bytes: counter(
                    evidence.retained_source_bytes,
                    "retained_source_bytes",
                )?,
                peak_source_bytes: counter(evidence.peak_source_bytes, "peak_source_bytes")?,
                denied_source_allocations: counter(
                    evidence.denied_source_allocations,
                    "denied_source_allocations",
                )?,
                retained_projected_bytes: counter(
                    evidence.retained_projected_bytes,
                    "retained_projected_bytes",
                )?,
                outstanding_requests: queue.outstanding_requests,
                outstanding_logical_bytes: queue.logical_bytes,
                queued_requests: queue.queued_requests as u64,
                active: queue.active,
                fenced: queue.fenced,
            })
        } else {
            None
        },
    })
}

pub(super) fn counter(value: Option<u64>, name: &str) -> Result<u64> {
    value.ok_or_else(|| ServiceError::Storage(format!("rrflowKV physical evidence omitted {name}")))
}

pub(super) fn segment_io_mode(mode: Option<&str>) -> Result<DiagnosticSegmentIoMode> {
    match mode {
        Some("auto") => Ok(DiagnosticSegmentIoMode::Auto),
        Some("mmap") => Ok(DiagnosticSegmentIoMode::Mmap),
        Some("io_uring") => Ok(DiagnosticSegmentIoMode::IoUring),
        Some("bounded") => Ok(DiagnosticSegmentIoMode::Bounded),
        mode => Err(ServiceError::Storage(format!(
            "rrflowKV physical evidence names unknown segment I/O mode {mode:?}"
        ))),
    }
}

/// A fallback reason is diagnostic text; it is bounded to one contract
/// message so a long error cannot fail the whole snapshot.
pub(super) fn bounded_reason(reason: Option<&String>) -> Option<String> {
    let reason = reason?.replace('\0', "");
    if reason.is_empty() {
        return None;
    }
    let mut end = reason.len().min(MAX_MESSAGE_BYTES);
    while !reason.is_char_boundary(end) {
        end -= 1;
    }
    Some(reason[..end].to_owned())
}

pub(super) fn rrflow_kv_snapshot(
    evidence: &PhysicalStoreEvidence,
    rrflow_cld: &crate::RrflowCldSnapshot,
) -> Result<DiagnosticRrflowKvSnapshot> {
    Ok(DiagnosticRrflowKvSnapshot {
        physical_sequence: counter(evidence.physical_sequence, "physical_sequence")?,
        manifest_generation: counter(evidence.manifest_generation, "manifest_generation")?,
        durable_sequence: counter(evidence.durable_sequence, "durable_sequence")?,
        memtable: DiagnosticMemtableSnapshot {
            versions: counter(evidence.memtable_versions, "memtable_versions")?,
            max_versions: counter(evidence.memtable_max_versions, "memtable_max_versions")?,
            approximate_bytes: counter(evidence.memtable_bytes, "memtable_bytes")?,
            keys: counter(evidence.memtable_keys, "memtable_keys")?,
            key_payload_bytes: counter(
                evidence.memtable_key_payload_bytes,
                "memtable_key_payload_bytes",
            )?,
            value_payload_bytes: counter(
                evidence.memtable_value_payload_bytes,
                "memtable_value_payload_bytes",
            )?,
            tombstones: counter(evidence.memtable_tombstones, "memtable_tombstones")?,
            spilled_chains: counter(evidence.memtable_spilled_chains, "memtable_spilled_chains")?,
            spilled_version_capacity: counter(
                evidence.memtable_spilled_version_capacity,
                "memtable_spilled_version_capacity",
            )?,
            version_record_bytes: counter(
                evidence.memtable_version_record_bytes,
                "memtable_version_record_bytes",
            )?,
            owned_bytes_lower_bound: counter(
                evidence.memtable_owned_bytes_lower_bound,
                "memtable_owned_bytes_lower_bound",
            )?,
        },
        write_ahead_log: DiagnosticWriteAheadLogSnapshot {
            payload_bytes: counter(evidence.wal_payload_bytes, "wal_payload_bytes")?,
            payload_max_bytes: counter(evidence.wal_payload_max_bytes, "wal_payload_max_bytes")?,
        },
        maintenance: DiagnosticMaintenanceSnapshot {
            automatic_flushes: counter(evidence.automatic_flushes, "automatic_flushes")?,
            failed_flushes: counter(
                evidence.failed_maintenance_flushes,
                "failed_maintenance_flushes",
            )?,
            write_stalls: counter(
                evidence.maintenance_write_stalls,
                "maintenance_write_stalls",
            )?,
            oversized_batches: counter(evidence.oversized_batches, "oversized_batches")?,
            automatic_compactions: counter(
                evidence.automatic_compactions,
                "automatic_compactions",
            )?,
            failed_compactions: counter(evidence.failed_compactions, "failed_compactions")?,
            compaction_input_bytes: counter(
                evidence.compaction_input_bytes,
                "compaction_input_bytes",
            )?,
            compaction_output_bytes: counter(
                evidence.compaction_output_bytes,
                "compaction_output_bytes",
            )?,
            peak_compaction_buffer_bytes: counter(
                evidence.peak_compaction_buffer_bytes,
                "peak_compaction_buffer_bytes",
            )?,
        },
        segment_set: DiagnosticSegmentSetSnapshot {
            segments: counter(evidence.segment_count, "segment_count")?,
            segment_bytes: counter(evidence.segment_bytes, "segment_bytes")?,
            l0_segments: counter(evidence.l0_segment_count, "l0_segment_count")?,
            l0_compaction_trigger: counter(
                evidence.l0_compaction_trigger,
                "l0_compaction_trigger",
            )?,
            compaction_debt_segments: counter(
                evidence.compaction_debt_segments,
                "compaction_debt_segments",
            )?,
            compaction_target_segment_bytes: counter(
                evidence.compaction_target_segment_bytes,
                "compaction_target_segment_bytes",
            )?,
        },
        page_cache: DiagnosticPageCacheSnapshot {
            capacity_bytes: counter(evidence.cache_capacity_bytes, "cache_capacity_bytes")?,
            resident_bytes: counter(evidence.cache_resident_bytes, "cache_resident_bytes")?,
            entries: counter(evidence.cache_entries, "cache_entries")?,
            hits: counter(evidence.cache_hits, "cache_hits")?,
            misses: counter(evidence.cache_misses, "cache_misses")?,
            evictions: counter(evidence.cache_evictions, "cache_evictions")?,
            page_loads: counter(evidence.page_loads, "page_loads")?,
            bytes_read: counter(evidence.page_bytes_read, "page_bytes_read")?,
            bytes_decoded: counter(evidence.page_bytes_decoded, "page_bytes_decoded")?,
            bytes_borrowed: counter(evidence.page_bytes_borrowed, "page_bytes_borrowed")?,
            bytes_allocated: counter(evidence.page_bytes_allocated, "page_bytes_allocated")?,
            bytes_copied: counter(evidence.page_bytes_copied, "page_bytes_copied")?,
            bytes_decompressed: counter(
                evidence.page_bytes_decompressed,
                "page_bytes_decompressed",
            )?,
            filter_checks: counter(evidence.filter_checks, "filter_checks")?,
            filter_negatives: counter(evidence.filter_negatives, "filter_negatives")?,
        },
        segment_io: DiagnosticSegmentIoSnapshot {
            max_open_files: counter(
                evidence.segment_io_max_open_files,
                "segment_io_max_open_files",
            )?,
            open_files: counter(evidence.segment_io_open_files, "segment_io_open_files")?,
            peak_open_files: counter(
                evidence.segment_io_peak_open_files,
                "segment_io_peak_open_files",
            )?,
            handle_evictions: counter(
                evidence.segment_io_handle_evictions,
                "segment_io_handle_evictions",
            )?,
            handle_reopens: counter(
                evidence.segment_io_handle_reopens,
                "segment_io_handle_reopens",
            )?,
            handle_denials: counter(
                evidence.segment_io_handle_denials,
                "segment_io_handle_denials",
            )?,

            requested_mode: segment_io_mode(evidence.segment_io_requested_mode.as_deref())?,
            mmap_segments: counter(
                evidence.segment_io_mmap_segments,
                "segment_io_mmap_segments",
            )?,
            io_uring_segments: counter(
                evidence.segment_io_uring_segments,
                "segment_io_uring_segments",
            )?,
            bounded_segments: counter(
                evidence.segment_io_bounded_segments,
                "segment_io_bounded_segments",
            )?,
            fallbacks: counter(evidence.segment_io_fallbacks, "segment_io_fallbacks")?,
            last_fallback_reason: bounded_reason(evidence.segment_io_last_fallback.as_ref()),
            read_operations: counter(
                evidence.segment_io_read_operations,
                "segment_io_read_operations",
            )?,
            mmap_reads: counter(evidence.segment_io_mmap_reads, "segment_io_mmap_reads")?,
            io_uring_reads: counter(evidence.segment_io_uring_reads, "segment_io_uring_reads")?,
            bounded_reads: counter(
                evidence.segment_io_bounded_reads,
                "segment_io_bounded_reads",
            )?,
            bytes_read: counter(evidence.segment_io_bytes_read, "segment_io_bytes_read")?,
            max_request_bytes: counter(
                evidence.segment_io_max_request_bytes,
                "segment_io_max_request_bytes",
            )?,
            peak_request_bytes: counter(
                evidence.segment_io_peak_request_bytes,
                "segment_io_peak_request_bytes",
            )?,
        },
        rrflow_cld: Some(DiagnosticRrflowCldSnapshot {
            trigger_count: rrflow_cld.trigger_count,
            cold_source_hits: rrflow_cld.cold_source_hits,
            cold_mapped_hits: rrflow_cld.cold_mapped_hits,
            cold_owned_hits: rrflow_cld.cold_owned_hits,
            cache_bypass_hits: rrflow_cld.cache_bypass_hits,
            last_trigger_at_unix_ms: rrflow_cld.last_trigger_at_unix_ms,
        }),
    })
}

pub(super) fn graph_difference_row_count(difference: &DiagnosticGraphDifference) -> usize {
    difference
        .added_records
        .len()
        .saturating_add(difference.removed_records.len())
        .saturating_add(difference.changed_records.len())
        .saturating_add(difference.added_relations.len())
        .saturating_add(difference.removed_relations.len())
        .saturating_add(difference.changed_relations.len())
}

pub(super) fn diagnostic_count(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| ServiceError::Contract("diagnostic count exceeds u64".into()))
}

pub(super) fn section(
    id: &str,
    authority: DiagnosticAuthority,
    coverage: DiagnosticCoverage,
    row_count: usize,
    known_at_cursor: u64,
    requested_after: Option<u64>,
) -> Result<DiagnosticSectionSnapshot> {
    Ok(DiagnosticSectionSnapshot {
        id: CanonicalId::new(id).map_err(|error| ServiceError::Contract(error.to_string()))?,
        authority,
        coverage,
        row_count: u64::try_from(row_count)
            .map_err(|_| ServiceError::Contract("diagnostic row count exceeds u64".into()))?,
        known_at_cursor,
        requested_after,
    })
}
