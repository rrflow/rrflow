//! Provider-neutral projection of the storage direct-read evidence.
//!
//! Storage owns physical metering and stamp authentication. The composition
//! root owns the only projection into the public RRFlow contract so engine
//! consumers cannot invent path names, suppress counters, or expose a backend
//! implementation.

use crate::engine::{Result, ServiceError};
use rrd_contract::{
    ReadAccessPath, ReadEvidence, ReadPathEvidence, ReadStampValidationEvidence,
    ReadStampValidationMethod,
};
use rrd_store::{
    RuntimeReadAccessPath, RuntimeReadEvidence, RUNTIME_VERSIONED_READ_CONTRACT_VERSION,
};
use std::collections::BTreeMap;

pub(crate) fn public_read_evidence(evidence: RuntimeReadEvidence) -> Result<ReadEvidence> {
    if evidence.contract_version != RUNTIME_VERSIONED_READ_CONTRACT_VERSION {
        return Err(ServiceError::Storage(format!(
            "unsupported storage direct-read evidence contract version {}",
            evidence.contract_version
        )));
    }
    let method = match evidence.stamp_validation.method.as_str() {
        "authenticated_current_head" => ReadStampValidationMethod::AuthenticatedCurrentHead,
        "rfc9162_direct_versions" => ReadStampValidationMethod::Rfc9162DirectVersions,
        method => {
            return Err(ServiceError::Storage(format!(
                "storage returned non-direct read-stamp validation method {method}"
            )));
        }
    };
    let public = ReadEvidence {
        contract_version: evidence.contract_version,
        key_budget: evidence.key_budget,
        point_reads: evidence.point_reads,
        range_scans: evidence.range_scans,
        keys_examined: evidence.keys_examined,
        values_decoded: evidence.values_decoded,
        decoded_bytes: evidence.decoded_bytes,
        stamp_validation: ReadStampValidationEvidence {
            method,
            change_reads: evidence.stamp_validation.change_reads,
            proof_nodes: evidence.stamp_validation.proof_nodes,
        },
        paths: evidence
            .paths
            .into_iter()
            .map(|path| ReadPathEvidence {
                path: match path.path {
                    RuntimeReadAccessPath::ReadStamp => ReadAccessPath::ReadStamp,
                    RuntimeReadAccessPath::SchemaVersions => ReadAccessPath::SchemaVersions,
                    RuntimeReadAccessPath::ClaimVersions => ReadAccessPath::ClaimVersions,
                    RuntimeReadAccessPath::RecordVersions => ReadAccessPath::RecordVersions,
                    RuntimeReadAccessPath::RelationVersions => ReadAccessPath::RelationVersions,
                    RuntimeReadAccessPath::AdjacencyVersions => ReadAccessPath::AdjacencyVersions,
                    RuntimeReadAccessPath::EventVersions => ReadAccessPath::EventVersions,
                    RuntimeReadAccessPath::VectorVersions => ReadAccessPath::VectorVersions,
                    RuntimeReadAccessPath::SeriesVersions => ReadAccessPath::SeriesVersions,
                    RuntimeReadAccessPath::GeoVersions => ReadAccessPath::GeoVersions,
                    RuntimeReadAccessPath::ObjectVersions => ReadAccessPath::ObjectVersions,
                    RuntimeReadAccessPath::AccumulatorProof => ReadAccessPath::AccumulatorProof,
                },
                point_reads: path.point_reads,
                range_scans: path.range_scans,
                keys_examined: path.keys_examined,
                values_decoded: path.values_decoded,
                decoded_bytes: path.decoded_bytes,
            })
            .collect(),
    };
    public
        .validate()
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    Ok(public)
}

/// Merges direct reads performed by branches of one engine operation. The
/// caller-owned operation budget remains the hard aggregate key limit; a
/// branch combination that exceeds it fails instead of reporting a larger
/// synthetic allowance.
pub(crate) fn merge_read_evidence(
    key_budget: u64,
    reads: impl IntoIterator<Item = ReadEvidence>,
) -> Result<ReadEvidence> {
    if key_budget == 0 {
        return Err(ServiceError::Contract(
            "merged read evidence requires a non-zero key budget".into(),
        ));
    }
    let mut method = None;
    let mut point_reads = 0_u64;
    let mut range_scans = 0_u64;
    let mut keys_examined = 0_u64;
    let mut values_decoded = 0_u64;
    let mut decoded_bytes = 0_u64;
    let mut change_reads = 0_u64;
    let mut proof_nodes = 0_u16;
    let mut paths = BTreeMap::<ReadAccessPath, ReadPathEvidence>::new();
    let mut count = 0_usize;
    for read in reads {
        read.validate()
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        count = count
            .checked_add(1)
            .ok_or_else(|| ServiceError::Storage("read evidence count overflowed".into()))?;
        if let Some(expected) = method {
            if expected != read.stamp_validation.method {
                return Err(ServiceError::Storage(
                    "cannot merge reads with different stamp-validation methods".into(),
                ));
            }
        } else {
            method = Some(read.stamp_validation.method);
        }
        point_reads = add_counter(point_reads, read.point_reads)?;
        range_scans = add_counter(range_scans, read.range_scans)?;
        keys_examined = add_counter(keys_examined, read.keys_examined)?;
        values_decoded = add_counter(values_decoded, read.values_decoded)?;
        decoded_bytes = add_counter(decoded_bytes, read.decoded_bytes)?;
        change_reads = add_counter(change_reads, read.stamp_validation.change_reads)?;
        proof_nodes = proof_nodes
            .checked_add(read.stamp_validation.proof_nodes)
            .ok_or_else(|| ServiceError::Storage("read proof-node counter overflowed".into()))?;
        for path in read.paths {
            let aggregate = paths.entry(path.path).or_insert(ReadPathEvidence {
                path: path.path,
                point_reads: 0,
                range_scans: 0,
                keys_examined: 0,
                values_decoded: 0,
                decoded_bytes: 0,
            });
            aggregate.point_reads = add_counter(aggregate.point_reads, path.point_reads)?;
            aggregate.range_scans = add_counter(aggregate.range_scans, path.range_scans)?;
            aggregate.keys_examined = add_counter(aggregate.keys_examined, path.keys_examined)?;
            aggregate.values_decoded = add_counter(aggregate.values_decoded, path.values_decoded)?;
            aggregate.decoded_bytes = add_counter(aggregate.decoded_bytes, path.decoded_bytes)?;
        }
    }
    if count == 0 {
        return Err(ServiceError::Contract(
            "merged read evidence requires at least one read".into(),
        ));
    }
    if keys_examined > key_budget {
        return Err(ServiceError::Query(format!(
            "operation examined {keys_examined} storage keys, budget allows {key_budget}"
        )));
    }
    let merged = ReadEvidence {
        contract_version: rrd_contract::READ_EVIDENCE_CONTRACT_VERSION,
        key_budget,
        point_reads,
        range_scans,
        keys_examined,
        values_decoded,
        decoded_bytes,
        stamp_validation: ReadStampValidationEvidence {
            method: method.expect("non-empty reads establish a validation method"),
            change_reads,
            proof_nodes,
        },
        paths: paths.into_values().collect(),
    };
    merged
        .validate()
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    Ok(merged)
}

fn add_counter(left: u64, right: u64) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| ServiceError::Storage("read evidence counter overflowed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rrd_core::RuntimeReadValidation;
    use rrd_store::RuntimeReadPathEvidence;

    fn storage_evidence(method: &str) -> RuntimeReadEvidence {
        RuntimeReadEvidence {
            contract_version: RUNTIME_VERSIONED_READ_CONTRACT_VERSION,
            key_budget: 16,
            point_reads: 1,
            range_scans: 1,
            keys_examined: 2,
            values_decoded: 1,
            decoded_bytes: 8,
            stamp_validation: RuntimeReadValidation::new(method, 0, 0),
            paths: vec![
                RuntimeReadPathEvidence {
                    path: RuntimeReadAccessPath::ReadStamp,
                    point_reads: 1,
                    range_scans: 0,
                    keys_examined: 1,
                    values_decoded: 0,
                    decoded_bytes: 0,
                },
                RuntimeReadPathEvidence {
                    path: RuntimeReadAccessPath::RecordVersions,
                    point_reads: 0,
                    range_scans: 1,
                    keys_examined: 1,
                    values_decoded: 1,
                    decoded_bytes: 8,
                },
            ],
        }
    }

    #[test]
    fn maps_closed_direct_read_evidence() {
        let public = public_read_evidence(storage_evidence("authenticated_current_head"))
            .expect("direct evidence maps");
        assert_eq!(
            public.stamp_validation.method,
            ReadStampValidationMethod::AuthenticatedCurrentHead
        );
        assert_eq!(public.paths[1].path, ReadAccessPath::RecordVersions);
    }

    #[test]
    fn rejects_log_replay_validation_as_normal_read_evidence() {
        let error = public_read_evidence(storage_evidence("bounded_hash_chain_page"))
            .expect_err("replay validation is not a normal direct read");
        assert!(error.to_string().contains("non-direct"));
    }

    #[test]
    fn merges_branch_evidence_under_one_operation_budget() {
        let first = public_read_evidence(storage_evidence("authenticated_current_head"))
            .expect("first read maps");
        let second = first.clone();
        let merged = merge_read_evidence(8, [first, second]).expect("reads fit aggregate budget");
        assert_eq!(merged.keys_examined, 4);
        assert_eq!(merged.paths[0].point_reads, 2);
        assert_eq!(merged.key_budget, 8);
    }

    #[test]
    fn rejects_branch_evidence_over_the_operation_budget() {
        let first = public_read_evidence(storage_evidence("authenticated_current_head"))
            .expect("first read maps");
        let error = merge_read_evidence(3, [first.clone(), first])
            .expect_err("aggregate budget must fail closed");
        assert!(error.to_string().contains("examined 4 storage keys"));
    }
}
