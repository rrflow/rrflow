//! Dot products, norms, centroid lookup, bit packing and the randomized Hadamard rotation.

use super::*;

pub(super) fn dot(left: &[f64], right: &[f64], kernel: QuantizedKernel) -> f64 {
    debug_assert_eq!(left.len(), right.len());
    match kernel {
        QuantizedKernel::Scalar => dot_scalar(left, right),
        QuantizedKernel::Auto => {
            #[cfg(target_arch = "x86_64")]
            if std::arch::is_x86_feature_detected!("avx2") {
                // SAFETY: AVX2 support was checked and both slices have equal
                // lengths.
                return unsafe { dot_avx2(left, right) };
            }
            dot_scalar(left, right)
        }
    }
}

pub(super) fn dot_scalar(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn dot_avx2(left: &[f64], right: &[f64]) -> f64 {
    use std::arch::x86_64::*;
    let mut sum = _mm256_setzero_pd();
    let mut index = 0;
    while index + 4 <= left.len() {
        let left_values = _mm256_loadu_pd(left.as_ptr().add(index));
        let right_values = _mm256_loadu_pd(right.as_ptr().add(index));
        sum = _mm256_add_pd(sum, _mm256_mul_pd(left_values, right_values));
        index += 4;
    }
    let mut lanes = [0.0_f64; 4];
    _mm256_storeu_pd(lanes.as_mut_ptr(), sum);
    let mut result = lanes.into_iter().sum::<f64>();
    while index < left.len() {
        result += left[index] * right[index];
        index += 1;
    }
    result
}

pub(super) fn validate_input(values: &[f32]) -> Result<()> {
    if values.is_empty() || values.len() > MAX_DIMENSIONS {
        return invalid("TurboQuant dimensions must be in 1..=1048576");
    }
    if values.iter().any(|value| !value.is_finite()) {
        return invalid("TurboQuant input must be finite");
    }
    Ok(())
}

pub(super) fn l2_norm(values: &[f32]) -> f32 {
    values
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt() as f32
}

pub(super) fn nearest_centroid(centroids: &[f32], value: f64) -> u8 {
    centroids
        .windows(2)
        .position(|pair| value <= f64::from((pair[0] + pair[1]) / 2.0))
        .unwrap_or(centroids.len() - 1) as u8
}

pub(super) fn packed_len(dimensions: usize, bits: u8) -> usize {
    dimensions.saturating_mul(usize::from(bits)).div_ceil(8)
}

pub(super) fn pack(indices: &[u8], bits: u8) -> Vec<u8> {
    let mut packed = vec![0_u8; packed_len(indices.len(), bits)];
    let mut bit_offset = 0usize;
    for index in indices {
        let byte = bit_offset / 8;
        let shift = bit_offset % 8;
        packed[byte] |= *index << shift;
        if shift + usize::from(bits) > 8 {
            packed[byte + 1] |= *index >> (8 - shift);
        }
        bit_offset += usize::from(bits);
    }
    packed
}

pub(super) fn unpack(packed: &[u8], dimensions: usize, bits: u8) -> Vec<u8> {
    let mask = (1_u16 << bits) - 1;
    (0..dimensions)
        .map(|index| {
            let bit_offset = index * usize::from(bits);
            let byte = bit_offset / 8;
            let shift = bit_offset % 8;
            let lower = u16::from(packed[byte]);
            let upper = packed.get(byte + 1).copied().map(u16::from).unwrap_or(0);
            (((lower | (upper << 8)) >> shift) & mask) as u8
        })
        .collect()
}

#[derive(Debug, Clone)]
pub(super) struct Rotation {
    permutations: [Vec<usize>; ROTATION_ROUNDS],
}

impl Rotation {
    pub(super) fn new(dimensions: usize, seed: u64) -> Self {
        Self {
            permutations: std::array::from_fn(|round| {
                permutation(dimensions, mix(seed, round as u64 + 1))
            }),
        }
    }

    pub(super) fn apply(&self, values: &mut [f64]) {
        hadamard_chunks(values);
        let mut scratch = vec![0.0; values.len()];
        for permutation in &self.permutations {
            for (target, source) in scratch.iter_mut().zip(permutation) {
                *target = values[*source];
            }
            values.copy_from_slice(&scratch);
            hadamard_chunks(values);
        }
    }

    pub(super) fn apply_inverse(&self, values: &mut [f64]) {
        hadamard_chunks(values);
        let mut scratch = vec![0.0; values.len()];
        for permutation in self.permutations.iter().rev() {
            for (target, source) in permutation.iter().enumerate() {
                scratch[*source] = values[target];
            }
            values.copy_from_slice(&scratch);
            hadamard_chunks(values);
        }
    }
}

pub(super) fn hadamard_chunks(values: &mut [f64]) {
    let mut remaining = values.len();
    let mut offset = 0usize;
    while remaining > 0 {
        let size = 1usize << remaining.ilog2();
        let chunk = &mut values[offset..offset + size];
        hadamard(chunk);
        let normalization = 1.0 / (size as f64).sqrt();
        for value in chunk {
            *value *= normalization;
        }
        offset += size;
        remaining -= size;
    }
}

pub(super) fn hadamard(values: &mut [f64]) {
    let mut width = 1usize;
    while width < values.len() {
        for start in (0..values.len()).step_by(width * 2) {
            for offset in 0..width {
                let left = values[start + offset];
                let right = values[start + offset + width];
                values[start + offset] = left + right;
                values[start + offset + width] = left - right;
            }
        }
        width *= 2;
    }
}

pub(super) fn permutation(dimensions: usize, seed: u64) -> Vec<usize> {
    let mut values = (0..dimensions).collect::<Vec<_>>();
    let mut state = seed;
    for upper in (1..dimensions).rev() {
        state = mix(state, upper as u64);
        let selected = (state as usize) % (upper + 1);
        values.swap(upper, selected);
    }
    values
}

pub(super) fn mix(mut value: u64, salt: u64) -> u64 {
    value ^= salt.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
