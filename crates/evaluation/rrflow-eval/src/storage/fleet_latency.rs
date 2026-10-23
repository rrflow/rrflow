//! Fixed-size integer bins with conservative percentile bounds. Raw attempts
//! remain authoritative; no interpolation or coordinated-omission correction.
use super::super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FleetLatencyBand {
    pub estate: u64,
    pub samples: u64,
    pub p99_lower_ns: u64,
    pub p99_upper_ns: u64,
}
pub(super) struct Histogram {
    counts: Box<[u64; 4096]>,
    samples: u64,
}
impl Histogram {
    pub fn new() -> Self {
        Self {
            counts: Box::new([0; 4096]),
            samples: 0,
        }
    }
    pub fn record(&mut self, value: u64) -> Result<()> {
        require(
            value <= MAX_EXACT_INTEGER,
            "fleet latency exceeds exact integer",
        )?;
        let index = if value < 64 {
            value as usize
        } else {
            let exponent = (63 - value.leading_zeros()).saturating_sub(6);
            64 + exponent as usize * 64 + ((value >> exponent) - 64) as usize
        };
        self.counts[index] = self.counts[index]
            .checked_add(1)
            .ok_or("fleet histogram count overflow")?;
        self.samples = self
            .samples
            .checked_add(1)
            .filter(|v| *v <= MAX_EXACT_INTEGER)
            .ok_or("fleet sample count overflow")?;
        Ok(())
    }
    pub fn finish(self, estate: u64) -> Result<FleetLatencyBand> {
        require(self.samples > 0, "fleet measured latency is absent")?;
        let rank = (u128::from(self.samples) * 99).div_ceil(100) as u64;
        let mut seen = 0;
        for (index, count) in self.counts.iter().enumerate() {
            seen += count;
            if seen >= rank {
                let (low, high) = if index < 64 {
                    (index as u64, index as u64)
                } else {
                    let exponent = (index - 64) / 64;
                    let mantissa = 64 + (index - 64) % 64;
                    (
                        (mantissa as u64) << exponent,
                        (((mantissa + 1) as u64) << exponent) - 1,
                    )
                };
                return Ok(FleetLatencyBand {
                    estate,
                    samples: self.samples,
                    p99_lower_ns: low,
                    p99_upper_ns: high,
                });
            }
        }
        Err("fleet histogram rank missing".into())
    }
}
pub(super) fn isolated_ratio(
    quiet: &[FleetLatencyBand],
    abusive: &[FleetLatencyBand],
    estates: u64,
) -> Result<bool> {
    require(
        quiet.len() == estates as usize && abusive.len() == estates as usize,
        "fleet latency member coverage differs",
    )?;
    let mut passed = true;
    for estate in 0..estates {
        let q = &quiet[estate as usize];
        let a = &abusive[estate as usize];
        require(
            q.estate == estate
                && a.estate == estate
                && q.samples > 0
                && a.samples > 0
                && q.p99_lower_ns <= q.p99_upper_ns
                && a.p99_lower_ns <= a.p99_upper_ns,
            "fleet latency band identity invalid",
        )?;
        if estate != 0 {
            passed &= u128::from(a.p99_upper_ns) <= 2 * u128::from(q.p99_lower_ns);
        }
    }
    Ok(passed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn percentile_bounds_contain_independent_sorted_rank() {
        let mut values = (0..10000u64)
            .map(|i| (i.wrapping_mul(717013) % 10007) * 1234567)
            .collect::<Vec<_>>();
        values.extend([0, 1, 63, 64, 127, 128, MAX_EXACT_INTEGER]);
        let mut histogram = Histogram::new();
        for value in &values {
            histogram.record(*value).unwrap();
        }
        values.sort_unstable();
        let exact = values[(values.len() * 99).div_ceil(100) - 1];
        let band = histogram.finish(0).unwrap();
        assert!(band.p99_lower_ns <= exact && exact <= band.p99_upper_ns);
        assert!(band.p99_upper_ns - band.p99_lower_ns <= exact.div_ceil(64));
    }
    #[test]
    fn isolation_uses_upper_over_lower_and_does_not_hide_ambiguous_bins() {
        let q = vec![
            FleetLatencyBand {
                estate: 0,
                samples: 100,
                p99_lower_ns: 100,
                p99_upper_ns: 101,
            },
            FleetLatencyBand {
                estate: 1,
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
        a[1].estate = 0;
        assert!(isolated_ratio(&q, &a, 2).is_err());
        assert!(Histogram::new().finish(0).is_err());
    }
}
