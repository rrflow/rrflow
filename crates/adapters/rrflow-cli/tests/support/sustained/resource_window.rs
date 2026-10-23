//! Preserve actual observations across overlapping offered windows. This does
//! not acquire samples or relax the campaign's independent coverage validator.
use super::{DriverResult, ResourceSample};

pub(super) fn overlapping_samples(
    previous: &[ResourceSample],
    started_ns: u64,
    pid: u32,
    maximum_samples: usize,
) -> DriverResult<Vec<ResourceSample>> {
    let mut carried = Vec::new();
    for sample in previous.iter().filter(|s| s.elapsed_ns >= started_ns) {
        if sample.pid != pid {
            return Err("overlapping resource observation belongs to another process".into());
        }
        // Reserve the fresh first observation and mandatory final observation.
        if carried.len().saturating_add(2) >= maximum_samples {
            return Err("overlapping resource observations exhaust frozen sample admission".into());
        }
        carried.push(sample.clone());
    }
    Ok(carried)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rrflow_eval::storage::{AllocationEnforcement, NativeMetric};

    fn sample(elapsed_ns: u64) -> ResourceSample {
        let observed = NativeMetric::Observed { value: 1234 };
        ResourceSample {
            elapsed_ns,
            pid: 77092,
            resident_bytes: observed.clone(),
            private_bytes: observed.clone(),
            mapped_bytes: observed.clone(),
            host_resident_bytes: observed.clone(),
            read_bytes: observed.clone(),
            written_bytes: observed.clone(),
            cpu_time_ns: observed.clone(),
            retained_bytes: observed.clone(),
            queue_requests: observed.clone(),
            maintenance_debt_segments: observed,
            allocation_enforcement: AllocationEnforcement::ObservedOnly,
            allocation_receipt: None,
            storage: None,
        }
    }

    #[test]
    fn resource_window_retains_actual_overlap_without_rewriting_time_or_values() {
        // Actual Windows campaign13 segment254/255 boundary timestamps. The
        // new segment's first sample alone exceeded the frozen two-second gap.
        let start = 2_560_409_933_400;
        let previous = vec![
            sample(2_560_337_501_400),
            sample(2_561_830_172_500),
            sample(2_562_966_803_500),
        ];
        let fresh = sample(2_563_149_587_600);
        assert!(fresh.elapsed_ns - start > 2_000_000_000);
        let mut selected = overlapping_samples(&previous, start, 77092, 256).unwrap();
        assert_eq!(selected.len(), 2);
        for (actual, original) in selected.iter().zip(&previous[1..]) {
            assert_eq!(
                serde_json::to_vec(actual).unwrap(),
                serde_json::to_vec(original).unwrap()
            );
        }
        selected.push(fresh);
        let mut at = start;
        for observed in selected {
            assert!(observed.elapsed_ns - at <= 2_000_000_000);
            at = observed.elapsed_ns;
        }
        assert_eq!(at, 2_563_149_587_600);
        assert!(overlapping_samples(&previous, start, 77093, 256).is_err());
        assert!(overlapping_samples(&previous, start, 77092, 3).is_err());
    }

    #[test]
    fn resource_window_cannot_fill_a_real_gap_or_reuse_an_earlier_phase() {
        let previous = vec![sample(1_000_000_000), sample(2_000_000_000)];
        let selected = overlapping_samples(&previous, 3_000_000_000, 77092, 256).unwrap();
        assert!(selected.is_empty());
        // Nothing was fabricated to bridge a genuinely unobserved interval.
        let fresh = sample(6_000_000_000);
        assert!(fresh.elapsed_ns - 3_000_000_000 > 2_000_000_000);
    }
}
