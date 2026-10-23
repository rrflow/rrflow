//! Checked composition of semantic read work across one stamped query run.
use crate::{Error, Result};
use rrd_store::RuntimeReadEvidence;
use std::collections::BTreeMap;

pub(super) fn remaining(limit: u64, used: u64) -> Result<u64> {
    limit
        .checked_sub(used)
        .filter(|remaining| *remaining > 0)
        .ok_or_else(|| {
            Error::Budget("stamped query exhausted its aggregate storage key budget".into())
        })
}

fn add(left: u64, right: u64) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| Error::Budget("stamped query evidence counter overflow".into()))
}

pub(super) fn combine(
    mut left: RuntimeReadEvidence,
    right: RuntimeReadEvidence,
    limit: u64,
) -> Result<RuntimeReadEvidence> {
    left.validate()?;
    right.validate()?;
    if left.contract_version != right.contract_version {
        return Err(Error::Integrity(
            "query evidence contract versions differ".into(),
        ));
    }
    left.key_budget = limit;
    left.point_reads = add(left.point_reads, right.point_reads)?;
    left.range_scans = add(left.range_scans, right.range_scans)?;
    left.keys_examined = add(left.keys_examined, right.keys_examined)?;
    left.values_decoded = add(left.values_decoded, right.values_decoded)?;
    left.decoded_bytes = add(left.decoded_bytes, right.decoded_bytes)?;
    if left.keys_examined > limit {
        return Err(Error::Budget(
            "stamped query exceeded its aggregate storage key budget".into(),
        ));
    }
    if left.stamp_validation.method != right.stamp_validation.method {
        // A live writer can advance the head between phases. Every phase
        // still validates the same immutable read through its accumulator.
        let permitted = |method: &str| {
            matches!(
                method,
                "authenticated_current_head" | "rfc9162_direct_versions"
            )
        };
        if !permitted(&left.stamp_validation.method) || !permitted(&right.stamp_validation.method) {
            return Err(Error::Integrity(
                "query evidence mixes incompatible stamp validation methods".into(),
            ));
        }
        left.stamp_validation.method = "rfc9162_direct_versions".into();
    }
    left.stamp_validation.change_reads = add(
        left.stamp_validation.change_reads,
        right.stamp_validation.change_reads,
    )?;
    left.stamp_validation.proof_nodes = left
        .stamp_validation
        .proof_nodes
        .checked_add(right.stamp_validation.proof_nodes)
        .ok_or_else(|| Error::Budget("stamped query validation proof count overflow".into()))?;
    let mut paths = left
        .paths
        .into_iter()
        .map(|path| (path.path, path))
        .collect::<BTreeMap<_, _>>();
    for right in right.paths {
        if let Some(left) = paths.get_mut(&right.path) {
            left.point_reads = add(left.point_reads, right.point_reads)?;
            left.range_scans = add(left.range_scans, right.range_scans)?;
            left.keys_examined = add(left.keys_examined, right.keys_examined)?;
            left.values_decoded = add(left.values_decoded, right.values_decoded)?;
            left.decoded_bytes = add(left.decoded_bytes, right.decoded_bytes)?;
        } else {
            paths.insert(right.path, right);
        }
    }
    left.paths = paths.into_values().collect();
    left.validate()?;
    Ok(left)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rrd_core::RuntimeReadValidation;
    use rrd_store::{RuntimeReadAccessPath, RuntimeReadPathEvidence};

    fn evidence(keys: u64) -> RuntimeReadEvidence {
        RuntimeReadEvidence {
            contract_version: 1,
            key_budget: u64::MAX,
            point_reads: keys,
            range_scans: 0,
            keys_examined: keys,
            values_decoded: 0,
            decoded_bytes: 0,
            stamp_validation: RuntimeReadValidation::new("authenticated_current_head", 0, 0),
            paths: vec![RuntimeReadPathEvidence {
                path: RuntimeReadAccessPath::ReadStamp,
                point_reads: keys,
                range_scans: 0,
                keys_examined: keys,
                values_decoded: 0,
                decoded_bytes: 0,
            }],
        }
    }

    #[test]
    fn combined_query_evidence_preserves_real_work_and_rejects_overflow_and_forgery() {
        let mut historical = evidence(3);
        historical.stamp_validation = RuntimeReadValidation::new("rfc9162_direct_versions", 0, 4);
        let total = combine(evidence(2), historical, 5).unwrap();
        assert_eq!(total.keys_examined, 5);
        assert_eq!(total.paths[0].point_reads, 5);
        assert_eq!(total.stamp_validation.method, "rfc9162_direct_versions");
        assert_eq!(total.stamp_validation.proof_nodes, 4);
        total.validate().unwrap();
        assert!(matches!(remaining(5, 5), Err(Error::Budget(_))));
        assert!(matches!(remaining(5, 6), Err(Error::Budget(_))));
        assert!(matches!(
            combine(evidence(2), evidence(3), 4),
            Err(Error::Budget(_))
        ));
        assert!(matches!(
            combine(evidence(u64::MAX), evidence(1), u64::MAX),
            Err(Error::Budget(_))
        ));
        let mut forged = evidence(1);
        forged.keys_examined = 0;
        assert!(combine(evidence(1), forged, 2).is_err());
        let mut unproven = evidence(1);
        unproven.stamp_validation.method = "unverified".into();
        assert!(matches!(
            combine(evidence(1), unproven, 2),
            Err(Error::Integrity(_))
        ));
        let mut saturated = evidence(1);
        saturated.stamp_validation.proof_nodes = u16::MAX;
        assert!(matches!(
            combine(saturated.clone(), saturated, 2),
            Err(Error::Budget(_))
        ));
    }
}
