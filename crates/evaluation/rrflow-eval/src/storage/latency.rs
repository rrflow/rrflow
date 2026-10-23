//! Bounded integer latency reduction shared by storage evidence consumers.
use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LatencyBounds {
    pub samples: u64,
    pub lower_ns: u64,
    pub upper_ns: u64,
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
            "storage latency exceeds exact integer",
        )?;
        let index = if value < 64 {
            value as usize
        } else {
            let exponent = (63 - value.leading_zeros()).saturating_sub(6);
            64 + exponent as usize * 64 + ((value >> exponent) - 64) as usize
        };
        self.counts[index] = self.counts[index]
            .checked_add(1)
            .ok_or("storage histogram count overflow")?;
        self.samples = self
            .samples
            .checked_add(1)
            .filter(|v| *v <= MAX_EXACT_INTEGER)
            .ok_or("storage sample count overflow")?;
        Ok(())
    }
    pub fn percentile(&self, percentile: u8) -> Result<LatencyBounds> {
        require(self.samples > 0, "storage measured latency is absent")?;
        require((1..=100).contains(&percentile), "percentile outside 1..100")?;
        let rank = (u128::from(self.samples) * u128::from(percentile)).div_ceil(100) as u64;
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
                return Ok(LatencyBounds {
                    samples: self.samples,
                    lower_ns: low,
                    upper_ns: high,
                });
            }
        }
        Err("storage histogram rank missing".into())
    }
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
        for percentile in [50, 95, 99, 100] {
            let exact = values[(values.len() * percentile).div_ceil(100) - 1];
            let band = histogram.percentile(percentile as u8).unwrap();
            assert!(band.lower_ns <= exact && exact <= band.upper_ns);
            assert!(band.upper_ns - band.lower_ns <= exact.div_ceil(64));
        }
    }
    #[test]
    fn empty_input_and_invalid_percentiles_fail() {
        assert!(Histogram::new().percentile(99).is_err());
        let mut histogram = Histogram::new();
        histogram.record(1).unwrap();
        assert!(histogram.percentile(0).is_err());
        assert!(histogram.percentile(101).is_err());
    }
}
