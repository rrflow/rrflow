//! Fixed-size integer bins with conservative percentile bounds. Raw attempts
//! remain authoritative; no interpolation or coordinated-omission correction.
use super::super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FleetLatencyBand {
    pub engram: u64,
    pub samples: u64,
    pub p99_lower_ns: u64,
    pub p99_upper_ns: u64,
}
pub(super) fn isolated_ratio(
    quiet: &[FleetLatencyBand],
    abusive: &[FleetLatencyBand],
    engrams: u64,
) -> Result<bool> {
    require(
        quiet.len() == engrams as usize && abusive.len() == engrams as usize,
        "fleet latency member coverage differs",
    )?;
    let mut passed = true;
    for engram in 0..engrams {
        let q = &quiet[engram as usize];
        let a = &abusive[engram as usize];
        require(
            q.engram == engram
                && a.engram == engram
                && q.samples > 0
                && a.samples > 0
                && q.p99_lower_ns <= q.p99_upper_ns
                && a.p99_lower_ns <= a.p99_upper_ns,
            "fleet latency band identity invalid",
        )?;
        if engram != 0 {
            passed &= u128::from(a.p99_upper_ns) <= 2 * u128::from(q.p99_lower_ns);
        }
    }
    Ok(passed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn isolation_uses_upper_over_lower_and_does_not_hide_ambiguous_bins() {
        let q = vec![
            FleetLatencyBand {
                engram: 0,
                samples: 100,
                p99_lower_ns: 100,
                p99_upper_ns: 101,
            },
            FleetLatencyBand {
                engram: 1,
                samples: 100,
                p99_lower_ns: 100,
                p99_upper_ns: 101,
            },
        ];
        let mut a = q.clone();
        a[1].p99_lower_ns = 199;
        a[1].p99_upper_ns = 200;
        assert!(isolated_ratio(&q, &a, 2).unwrap());
        a[1].p99_upper_ns = 201;
        assert!(!isolated_ratio(&q, &a, 2).unwrap());
        a[1].engram = 0;
        assert!(isolated_ratio(&q, &a, 2).is_err());
    }
}
