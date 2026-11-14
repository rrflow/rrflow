//! The direct-convergence operation and attribute vocabulary and its validation.

use super::*;

// These names are the exact observed pre-A-07.2 call-site inventory. They are
// accepted only so the owning C-through-I packages can replace the associated
// behavior and trace together. New instrumentation must use `TraceOperation`;
// this list can only shrink.
pub(super) const DIRECT_CONVERGENCE_OPERATION_NAMES: &[&str] = &[
    "cluster.artifact_chunk",
    "cluster.artifact_transfer",
    "embedding.commit",
    "embedding.infer",
    "embedding.run",
    "object.replicate",
    "operator.knowledge.apply",
    "operator.knowledge.execute",
    "operator.knowledge.search",
    "operator.knowledge.sync",
    "rrflow.query.execute",
    "rrflow.query.parse_bind",
    "rrflow.query.plan",
    "rrflow.query.run",
    "rrflow.storage.runtime_read",
    "vector.execute",
    "vector.plan",
    "vector.projection.publish",
    "vector.quantization.activate",
    "vector.quantization.build",
    "vector.quantization.retire",
    "vector.search",
];

// Exact attribute names emitted by the reviewed trace producers that predate
// the canonical cross-boundary vocabulary. The behavior-owning C-through-I
// package must either map each value to `TraceAttribute`/`TraceLink` or remove
// it. This inventory can only shrink; it is not an extension namespace.
pub(super) const DIRECT_CONVERGENCE_ATTRIBUTE_NAMES: &[&str] = &[
    "adapter_digest",
    "adapter_elapsed_micros",
    "adapter_implementation_digest",
    "applied_now",
    "approximation_requested",
    "artifact_digest",
    "automatic_compactions_delta",
    "automatic_flushes_delta",
    "backend",
    "backend_digest",
    "batch_count",
    "binding_digest",
    "cache_capacity_bytes",
    "cache_entries",
    "cache_evictions_delta",
    "cache_hits_delta",
    "cache_misses_delta",
    "cache_resident_bytes",
    "canonical_digest",
    "catalog_digest",
    "catalog_entry_digest",
    "catalog_revision",
    "commit_id",
    "compaction_debt_segments",
    "compaction_input_bytes_delta",
    "compaction_output_bytes_delta",
    "compaction_target_segment_bytes",
    "config_digest",
    "contract_version",
    "deterministic",
    "dimensions",
    "distinct_objects",
    "durability",
    "durable_sequence",
    "ef_search",
    "estimated_candidates",
    "estimated_cost",
    "evidence_policy",
    "exact_rerank",
    "execution_target",
    "expected_catalog_revision",
    "expected_length",
    "expected_stable_revision",
    "failed_compactions_delta",
    "failed_maintenance_flushes_delta",
    "fallback_reason_digest",
    "fallback_to_exact",
    "filter_applied_after_ann",
    "filter_checks_delta",
    "filter_count",
    "filter_negatives_delta",
    "filter_present",
    "filter_property_count",
    "filter_selectivity",
    "first_cursor",
    "generation",
    "hit_count",
    "hnsw_ef_search",
    "hnsw_max_scan_tuples",
    "idempotent_replay",
    "index_digest",
    "iterative_scan",
    "ivfflat_max_probes",
    "ivfflat_probes",
    "job_digest",
    "l0_compaction_trigger",
    "l0_segment_count",
    "last_cursor",
    "lifecycle_action",
    "lifecycle_revision",
    "lifecycle_state",
    "logical_operator_count",
    "logical_output_bytes",
    "maintenance_write_stalls_delta",
    "manifest_digest",
    "manifest_generation",
    "max_output_bytes",
    "max_rows",
    "media_type_digest",
    "memtable_bytes",
    "memtable_max_versions",
    "memtable_versions",
    "metric",
    "mode",
    "model_space_digest",
    "mutation_kind",
    "network_required",
    "next_offset",
    "object_backend",
    "object_digest",
    "object_length",
    "object_references",
    "object_sha256",
    "operation",
    "ordering_exact",
    "outcome_cursor",
    "output_dimensions",
    "output_kind",
    "oversized_batches_delta",
    "page_bytes_allocated_delta",
    "page_bytes_borrowed_delta",
    "page_bytes_copied_delta",
    "page_bytes_decoded_delta",
    "page_bytes_decompressed_delta",
    "page_bytes_read_delta",
    "parameter_count",
    "parameter_digest",
    "payload_digest",
    "peak_compaction_buffer_bytes",
    "physical_error_digest",
    "physical_evidence",
    "physical_evidence_consistent",
    "physical_operator_count",
    "physical_sequence",
    "placement_epoch",
    "plan_digest",
    "project_digest",
    "projection_id",
    "projection_kind",
    "query_bytes",
    "query_digest",
    "query_dimensions",
    "query_kind",
    "rebased_over_trace_events",
    "receipt_digest",
    "rejected_path_count",
    "rejected_paths",
    "rejected_paths_digest",
    "request_digest",
    "requested_dimensions",
    "requested_path",
    "required_source_cursor",
    "result_count",
    "returned_rows",
    "schema_revision",
    "segment_bytes",
    "segment_count",
    "segment_io_bounded_reads_delta",
    "segment_io_bounded_segments",
    "segment_io_bytes_read_delta",
    "segment_io_fallbacks_delta",
    "segment_io_last_fallback",
    "segment_io_max_request_bytes",
    "segment_io_mmap_reads_delta",
    "segment_io_mmap_segments",
    "segment_io_peak_request_bytes",
    "segment_io_requested_mode",
    "segment_io_uring_reads_delta",
    "segment_io_uring_segments",
    "selected_path",
    "selected_paths",
    "selected_projection",
    "shard",
    "snapshot_commit_index",
    "snapshot_digest",
    "snapshot_state_digest",
    "snapshot_term",
    "source_change_digest",
    "source_cursor",
    "source_digest",
    "source_family",
    "source_node",
    "source_revision_digest",
    "source_type",
    "stable_revision_present",
    "target_digest",
    "target_node",
    "top_k",
    "transferred_bytes",
    "transferred_objects",
    "verified_references",
    "wal_payload_bytes",
    "wal_payload_max_bytes",
    "work_id",
];

pub(super) fn validate_operation_name(value: &str) -> Result<()> {
    validate_bounded_text("trace operation", value, MAX_TRACE_NAME_BYTES)?;
    if TraceOperation::from_name(value).is_some()
        || DIRECT_CONVERGENCE_OPERATION_NAMES.contains(&value)
    {
        return Ok(());
    }
    invalid(
        "trace operation is neither a canonical TraceOperation nor reviewed direct-convergence inventory",
    )
}

pub(super) fn direct_convergence_operation_boundary(value: &str) -> Option<TraceBoundary> {
    match value {
        "embedding.commit" => Some(TraceBoundary::Engine),
        "embedding.infer" | "embedding.run" => Some(TraceBoundary::Inference),
        "rrflow.query.execute"
        | "rrflow.query.parse_bind"
        | "rrflow.query.plan"
        | "rrflow.query.run" => Some(TraceBoundary::Ql),
        "rrflow.storage.runtime_read" => Some(TraceBoundary::Kv),
        "vector.execute"
        | "vector.plan"
        | "vector.projection.publish"
        | "vector.quantization.activate"
        | "vector.quantization.build"
        | "vector.quantization.retire"
        | "vector.search" => Some(TraceBoundary::Vector),
        "cluster.artifact_chunk"
        | "cluster.artifact_transfer"
        | "object.replicate"
        | "operator.knowledge.apply"
        | "operator.knowledge.execute"
        | "operator.knowledge.search"
        | "operator.knowledge.sync" => Some(TraceBoundary::Adapter),
        _ => None,
    }
}

pub(super) fn validate_attribute_name(value: &str) -> Result<()> {
    validate_bounded_text("trace attribute name", value, MAX_TRACE_NAME_BYTES)?;
    if TraceAttribute::from_name(value).is_some()
        || DIRECT_CONVERGENCE_ATTRIBUTE_NAMES.contains(&value)
    {
        return Ok(());
    }
    invalid(
        "trace attribute is neither a canonical TraceAttribute nor reviewed direct-convergence inventory",
    )
}

pub(super) fn validate_canonical_attribute_value(
    attribute: TraceAttribute,
    value: &RuntimeValue,
) -> Result<()> {
    match attribute {
        TraceAttribute::Stage
        | TraceAttribute::Phase
        | TraceAttribute::Action
        | TraceAttribute::SkipReason
        | TraceAttribute::CacheStatus
        | TraceAttribute::FailedStage
        | TraceAttribute::ErrorClass
        | TraceAttribute::ReadStampValidation
        | TraceAttribute::VerificationStatus
        | TraceAttribute::PropagationStatus => {
            let RuntimeValue::String(value) = value else {
                return invalid(format!(
                    "trace attribute {} must be a low-cardinality string",
                    attribute.as_str()
                ));
            };
            validate_attribute_token(attribute, value)
        }
        TraceAttribute::Selected
        | TraceAttribute::Exact
        | TraceAttribute::Fallback
        | TraceAttribute::Truncated
        | TraceAttribute::Retryable => {
            if matches!(value, RuntimeValue::Bool(_)) {
                Ok(())
            } else {
                invalid(format!(
                    "trace attribute {} must be a boolean",
                    attribute.as_str()
                ))
            }
        }
        TraceAttribute::Attempt
        | TraceAttribute::InputItems
        | TraceAttribute::InputBytes
        | TraceAttribute::MaxStorageKeys
        | TraceAttribute::OutputItems
        | TraceAttribute::OutputBytes
        | TraceAttribute::SelectedVersions
        | TraceAttribute::KeysExamined
        | TraceAttribute::PagesExamined
        | TraceAttribute::RowsExamined
        | TraceAttribute::ReadPointReads
        | TraceAttribute::ReadRangeScans
        | TraceAttribute::ReadValuesDecoded
        | TraceAttribute::ReadStampValidationChangeReads
        | TraceAttribute::ReadStampValidationProofNodes
        | TraceAttribute::GraphSteps
        | TraceAttribute::CandidatesExamined
        | TraceAttribute::MappedBytes
        | TraceAttribute::ReadBytes
        | TraceAttribute::DecodedBytes
        | TraceAttribute::DecompressedBytes
        | TraceAttribute::BorrowedBytes
        | TraceAttribute::CopiedBytes
        | TraceAttribute::AllocatedBytes
        | TraceAttribute::ElapsedMicros
        | TraceAttribute::MemoryPeakBytes
        | TraceAttribute::SpillBytes
        | TraceAttribute::InputTokens
        | TraceAttribute::OutputTokens
        | TraceAttribute::Rank
        | TraceAttribute::SelectedSourceCount
        | TraceAttribute::SkippedSourceCount
        | TraceAttribute::CompactedBytes
        | TraceAttribute::CompactedTokens
        | TraceAttribute::StartCursor => {
            if matches!(value, RuntimeValue::Unsigned(_)) {
                Ok(())
            } else {
                invalid(format!(
                    "trace attribute {} must be an unsigned integer",
                    attribute.as_str()
                ))
            }
        }
        TraceAttribute::Score | TraceAttribute::RrfContribution => {
            let RuntimeValue::Decimal(value) = value else {
                return invalid(format!(
                    "trace attribute {} must be a finite decimal string",
                    attribute.as_str()
                ));
            };
            if value.parse::<f64>().is_ok_and(f64::is_finite) {
                Ok(())
            } else {
                invalid(format!(
                    "trace attribute {} must be a finite decimal string",
                    attribute.as_str()
                ))
            }
        }
        TraceAttribute::ErrorDigest | TraceAttribute::PropagationDigest => {
            let RuntimeValue::Digest(value) = value else {
                return invalid(format!(
                    "trace attribute {} must be a SHA-256 digest",
                    attribute.as_str()
                ));
            };
            validate_digest("trace attribute", value)
        }
    }
}

pub(super) fn validate_attribute_token(attribute: TraceAttribute, value: &str) -> Result<()> {
    validate_bounded_text("trace attribute token", value, MAX_TRACE_NAME_BYTES)?;
    let mut bytes = value.bytes();
    if !bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        || bytes.any(|byte| {
            !(byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b':' | b'-'))
        })
    {
        return invalid(format!(
            "trace attribute {} must use a bounded lowercase token",
            attribute.as_str()
        ));
    }
    Ok(())
}

pub(super) fn validate_bounded_text(kind: &'static str, value: &str, max: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > max || value.as_bytes().contains(&0) {
        return invalid(format!(
            "{kind} must be non-empty, contain no NUL, and fit within {max} bytes"
        ));
    }
    Ok(())
}

pub(super) fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::InvalidRuntime {
        reason: reason.into(),
    })
}
