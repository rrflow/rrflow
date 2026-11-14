//! Physical-storage and execution trace attributes, source identity and query failures.

use super::*;

pub(super) fn physical_storage_attributes(
    before: &Result<PhysicalStoreEvidence, rrd_store::Error>,
    after: &Result<PhysicalStoreEvidence, rrd_store::Error>,
    execution: Option<&QueryExecution>,
) -> RuntimeProperties {
    let mut attributes = RuntimeProperties::new();
    match (before, after) {
        (Ok(before), Ok(after)) => {
            attributes.insert(
                "backend".into(),
                RuntimeValue::String(after.backend.clone()),
            );
            attributes.insert(
                "physical_evidence".into(),
                RuntimeValue::String(after.evidence_level.clone()),
            );
            attributes.insert(
                "physical_evidence_consistent".into(),
                RuntimeValue::Bool(
                    before.backend == after.backend
                        && before.evidence_level == after.evidence_level,
                ),
            );
            insert_counter(
                &mut attributes,
                "physical_sequence",
                after.physical_sequence,
            );
            insert_counter(
                &mut attributes,
                "manifest_generation",
                after.manifest_generation,
            );
            insert_counter(&mut attributes, "durable_sequence", after.durable_sequence);
            insert_counter(
                &mut attributes,
                "memtable_versions",
                after.memtable_versions,
            );
            insert_counter(&mut attributes, "memtable_bytes", after.memtable_bytes);
            insert_counter(
                &mut attributes,
                "memtable_max_versions",
                after.memtable_max_versions,
            );
            insert_counter(
                &mut attributes,
                "wal_payload_bytes",
                after.wal_payload_bytes,
            );
            insert_counter(
                &mut attributes,
                "wal_payload_max_bytes",
                after.wal_payload_max_bytes,
            );
            insert_delta(
                &mut attributes,
                "automatic_flushes_delta",
                before.automatic_flushes,
                after.automatic_flushes,
            );
            insert_delta(
                &mut attributes,
                "maintenance_write_stalls_delta",
                before.maintenance_write_stalls,
                after.maintenance_write_stalls,
            );
            insert_delta(
                &mut attributes,
                "failed_maintenance_flushes_delta",
                before.failed_maintenance_flushes,
                after.failed_maintenance_flushes,
            );
            insert_delta(
                &mut attributes,
                "oversized_batches_delta",
                before.oversized_batches,
                after.oversized_batches,
            );
            insert_delta(
                &mut attributes,
                "automatic_compactions_delta",
                before.automatic_compactions,
                after.automatic_compactions,
            );
            insert_delta(
                &mut attributes,
                "failed_compactions_delta",
                before.failed_compactions,
                after.failed_compactions,
            );
            insert_delta(
                &mut attributes,
                "compaction_input_bytes_delta",
                before.compaction_input_bytes,
                after.compaction_input_bytes,
            );
            insert_delta(
                &mut attributes,
                "compaction_output_bytes_delta",
                before.compaction_output_bytes,
                after.compaction_output_bytes,
            );
            insert_counter(
                &mut attributes,
                "peak_compaction_buffer_bytes",
                after.peak_compaction_buffer_bytes,
            );
            insert_counter(&mut attributes, "l0_segment_count", after.l0_segment_count);
            insert_counter(
                &mut attributes,
                "l0_compaction_trigger",
                after.l0_compaction_trigger,
            );
            insert_counter(
                &mut attributes,
                "compaction_debt_segments",
                after.compaction_debt_segments,
            );
            insert_counter(
                &mut attributes,
                "compaction_target_segment_bytes",
                after.compaction_target_segment_bytes,
            );
            insert_counter(&mut attributes, "segment_count", after.segment_count);
            insert_counter(&mut attributes, "segment_bytes", after.segment_bytes);
            insert_counter(
                &mut attributes,
                "cache_capacity_bytes",
                after.cache_capacity_bytes,
            );
            insert_counter(
                &mut attributes,
                "cache_resident_bytes",
                after.cache_resident_bytes,
            );
            insert_counter(&mut attributes, "cache_entries", after.cache_entries);
            insert_delta(
                &mut attributes,
                "cache_hits_delta",
                before.cache_hits,
                after.cache_hits,
            );
            insert_delta(
                &mut attributes,
                "cache_misses_delta",
                before.cache_misses,
                after.cache_misses,
            );
            insert_delta(
                &mut attributes,
                "cache_evictions_delta",
                before.cache_evictions,
                after.cache_evictions,
            );
            insert_delta(
                &mut attributes,
                "page_bytes_read_delta",
                before.page_bytes_read,
                after.page_bytes_read,
            );
            insert_delta(
                &mut attributes,
                "page_bytes_decoded_delta",
                before.page_bytes_decoded,
                after.page_bytes_decoded,
            );
            insert_delta(
                &mut attributes,
                "page_bytes_borrowed_delta",
                before.page_bytes_borrowed,
                after.page_bytes_borrowed,
            );
            insert_delta(
                &mut attributes,
                "page_bytes_allocated_delta",
                before.page_bytes_allocated,
                after.page_bytes_allocated,
            );
            insert_delta(
                &mut attributes,
                "page_bytes_copied_delta",
                before.page_bytes_copied,
                after.page_bytes_copied,
            );
            insert_delta(
                &mut attributes,
                "page_bytes_decompressed_delta",
                before.page_bytes_decompressed,
                after.page_bytes_decompressed,
            );
            insert_delta(
                &mut attributes,
                "filter_checks_delta",
                before.filter_checks,
                after.filter_checks,
            );
            insert_delta(
                &mut attributes,
                "filter_negatives_delta",
                before.filter_negatives,
                after.filter_negatives,
            );
            if let Some(mode) = &after.segment_io_requested_mode {
                attributes.insert(
                    "segment_io_requested_mode".into(),
                    RuntimeValue::String(mode.clone()),
                );
            }
            insert_counter(
                &mut attributes,
                "segment_io_mmap_segments",
                after.segment_io_mmap_segments,
            );
            insert_counter(
                &mut attributes,
                "segment_io_uring_segments",
                after.segment_io_uring_segments,
            );
            insert_counter(
                &mut attributes,
                "segment_io_bounded_segments",
                after.segment_io_bounded_segments,
            );
            insert_delta(
                &mut attributes,
                "segment_io_fallbacks_delta",
                before.segment_io_fallbacks,
                after.segment_io_fallbacks,
            );
            if let Some(reason) = &after.segment_io_last_fallback {
                attributes.insert(
                    "segment_io_last_fallback".into(),
                    RuntimeValue::String(reason.clone()),
                );
            }
            insert_delta(
                &mut attributes,
                "segment_io_mmap_reads_delta",
                before.segment_io_mmap_reads,
                after.segment_io_mmap_reads,
            );
            insert_delta(
                &mut attributes,
                "segment_io_uring_reads_delta",
                before.segment_io_uring_reads,
                after.segment_io_uring_reads,
            );
            insert_delta(
                &mut attributes,
                "segment_io_bounded_reads_delta",
                before.segment_io_bounded_reads,
                after.segment_io_bounded_reads,
            );
            insert_delta(
                &mut attributes,
                "segment_io_bytes_read_delta",
                before.segment_io_bytes_read,
                after.segment_io_bytes_read,
            );
            insert_counter(
                &mut attributes,
                "segment_io_max_request_bytes",
                after.segment_io_max_request_bytes,
            );
            insert_counter(
                &mut attributes,
                "segment_io_peak_request_bytes",
                after.segment_io_peak_request_bytes,
            );
        }
        (Err(before_error), Err(after_error)) => {
            attributes.insert(
                "physical_evidence".into(),
                RuntimeValue::String("unavailable".into()),
            );
            attributes.insert(
                "physical_error_digest".into(),
                RuntimeValue::Digest(digest::sha256_hex(
                    format!("{before_error}; {after_error}").as_bytes(),
                )),
            );
        }
        (Err(error), Ok(after)) | (Ok(after), Err(error)) => {
            attributes.insert(
                "backend".into(),
                RuntimeValue::String(after.backend.clone()),
            );
            attributes.insert(
                "physical_evidence".into(),
                RuntimeValue::String("partial".into()),
            );
            attributes.insert(
                "physical_error_digest".into(),
                RuntimeValue::Digest(digest::sha256_hex(error.to_string().as_bytes())),
            );
        }
    }
    if let Some(execution) = execution {
        attributes.insert(
            TraceAttribute::SelectedVersions.into(),
            RuntimeValue::Unsigned(execution.selected_versions as u64),
        );
        attributes.insert(
            TraceAttribute::ReadPointReads.into(),
            RuntimeValue::Unsigned(execution.read_evidence.point_reads),
        );
        attributes.insert(
            TraceAttribute::ReadRangeScans.into(),
            RuntimeValue::Unsigned(execution.read_evidence.range_scans),
        );
        attributes.insert(
            TraceAttribute::KeysExamined.into(),
            RuntimeValue::Unsigned(execution.read_evidence.keys_examined),
        );
        attributes.insert(
            TraceAttribute::ReadValuesDecoded.into(),
            RuntimeValue::Unsigned(execution.read_evidence.values_decoded),
        );
        attributes.insert(
            TraceAttribute::DecodedBytes.into(),
            RuntimeValue::Unsigned(execution.read_evidence.decoded_bytes),
        );
        attributes.insert(
            TraceAttribute::ReadStampValidation.into(),
            RuntimeValue::String(execution.read_evidence.stamp_validation.method.clone()),
        );
        attributes.insert(
            TraceAttribute::ReadStampValidationChangeReads.into(),
            RuntimeValue::Unsigned(execution.read_evidence.stamp_validation.change_reads),
        );
        attributes.insert(
            TraceAttribute::ReadStampValidationProofNodes.into(),
            RuntimeValue::Unsigned(u64::from(
                execution.read_evidence.stamp_validation.proof_nodes,
            )),
        );
        attributes.insert(
            "logical_output_bytes".into(),
            RuntimeValue::Unsigned(execution.output_bytes as u64),
        );
    }
    attributes
}

pub(super) fn insert_counter(attributes: &mut RuntimeProperties, name: &str, value: Option<u64>) {
    if let Some(value) = value {
        attributes.insert(name.into(), RuntimeValue::Unsigned(value));
    }
}

pub(super) fn insert_delta(
    attributes: &mut RuntimeProperties,
    name: &str,
    before: Option<u64>,
    after: Option<u64>,
) {
    if let (Some(before), Some(after)) = (before, after) {
        attributes.insert(
            name.into(),
            RuntimeValue::Unsigned(after.saturating_sub(before)),
        );
    }
}

pub(super) fn execution_attributes(execution: &QueryExecution) -> RuntimeProperties {
    RuntimeProperties::from([
        (
            TraceAttribute::SelectedVersions.into(),
            RuntimeValue::Unsigned(execution.selected_versions as u64),
        ),
        (
            "returned_rows".into(),
            RuntimeValue::Unsigned(execution.returned_rows as u64),
        ),
        (
            TraceAttribute::ReadPointReads.into(),
            RuntimeValue::Unsigned(execution.read_evidence.point_reads),
        ),
        (
            TraceAttribute::ReadRangeScans.into(),
            RuntimeValue::Unsigned(execution.read_evidence.range_scans),
        ),
        (
            TraceAttribute::KeysExamined.into(),
            RuntimeValue::Unsigned(execution.read_evidence.keys_examined),
        ),
        (
            TraceAttribute::ReadValuesDecoded.into(),
            RuntimeValue::Unsigned(execution.read_evidence.values_decoded),
        ),
        (
            TraceAttribute::DecodedBytes.into(),
            RuntimeValue::Unsigned(execution.read_evidence.decoded_bytes),
        ),
        (
            TraceAttribute::ReadStampValidation.into(),
            RuntimeValue::String(execution.read_evidence.stamp_validation.method.clone()),
        ),
        (
            TraceAttribute::ReadStampValidationChangeReads.into(),
            RuntimeValue::Unsigned(execution.read_evidence.stamp_validation.change_reads),
        ),
        (
            TraceAttribute::ReadStampValidationProofNodes.into(),
            RuntimeValue::Unsigned(u64::from(
                execution.read_evidence.stamp_validation.proof_nodes,
            )),
        ),
        (
            "output_bytes".into(),
            RuntimeValue::Unsigned(execution.output_bytes as u64),
        ),
        (
            "batch_count".into(),
            RuntimeValue::Unsigned(execution.batches.len() as u64),
        ),
        ("truncated".into(), RuntimeValue::Bool(execution.truncated)),
    ])
}

pub(super) fn source_identity(source: &Source) -> (&'static str, String) {
    match source {
        Source::Record { kind } => ("record", kind.to_string()),
        Source::Relation { kind } => ("relation", kind.to_string()),
        Source::Event { kind } => ("event", kind.to_string()),
        Source::Series { kind } => ("series", kind.to_string()),
        Source::Geo { kind } => ("geo", kind.to_string()),
        Source::Traversal {
            relation,
            start,
            direction,
            max_depth,
        } => (
            "traversal",
            format!(
                "{relation}:{}:{}:{direction:?}:{max_depth}",
                start.kind, start.id
            ),
        ),
        Source::Claim { predicate } => (
            "claim",
            predicate
                .as_ref()
                .map_or_else(|| "*".into(), ToString::to_string),
        ),
    }
}

pub(super) fn query_error_class(error: &rrd_query::Error) -> &'static str {
    match error {
        rrd_query::Error::Graphql(_) => "graphql",
        rrd_query::Error::Catalog(_) => "catalog",
        rrd_query::Error::Binding(_) => "binding",
        rrd_query::Error::Budget(_) => "budget",
        rrd_query::Error::StorageAdmission(_) => "storage_admission",
        rrd_query::Error::Execution(_) => "execution",
        rrd_query::Error::Integrity(_) => "integrity",
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fail_query<E: StorageEngine, T>(
    store: &E,
    stage: Option<DurableTraceSpan>,
    root: DurableTraceSpan,
    failed_stage: &str,
    error_class: &str,
    outcome: TraceOutcome,
    error: Box<dyn std::error::Error>,
) -> Result<T, Box<dyn std::error::Error>> {
    let rendered = error.to_string();
    let error_digest = digest::sha256_hex(rendered.as_bytes());
    let evidence = || {
        RuntimeProperties::from([
            (
                "error_class".into(),
                RuntimeValue::String(error_class.into()),
            ),
            (
                "error_digest".into(),
                RuntimeValue::Digest(error_digest.clone()),
            ),
            (
                "failed_stage".into(),
                RuntimeValue::String(failed_stage.into()),
            ),
        ])
    };
    let mut trace_errors = Vec::new();
    if let Some(stage) = stage {
        if let Err(trace_error) = stage.finish(store, outcome, Vec::new(), evidence()) {
            trace_errors.push(format!("stage finish: {trace_error}"));
        }
    }
    if let Err(trace_error) = root.finish(store, outcome, Vec::new(), evidence()) {
        trace_errors.push(format!("root finish: {trace_error}"));
    }
    if trace_errors.is_empty() {
        Err(error)
    } else {
        Err(format!(
            "{rendered}; authoritative query trace also failed ({})",
            trace_errors.join("; ")
        )
        .into())
    }
}
