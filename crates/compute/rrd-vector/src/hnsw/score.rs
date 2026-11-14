//! Dense scoring, with NEON and AVX2 kernels and a portable fallback.

use super::*;

pub(super) fn score_dense_query(
    query: &[f32],
    candidate: &VectorCandidate,
    metric: ScoreMetric,
    kernel: HnswKernel,
) -> Result<f64> {
    let values = dense_candidate(candidate)?;
    score_dense_values(query, values, metric, kernel)
}

pub(super) fn score_dense_values(
    query: &[f32],
    candidate: &[f32],
    metric: ScoreMetric,
    kernel: HnswKernel,
) -> Result<f64> {
    if query.len() != candidate.len() {
        return invalid("dense query and candidate dimensions differ");
    }
    match kernel {
        HnswKernel::Scalar => score_dense(query, candidate, metric),
        HnswKernel::Auto => {
            if let Some(simd) = simd_score() {
                // SAFETY: simd_score only returns a kernel this host can run,
                // and both slices have the same proven length; a kernel
                // performs only unaligned in-bounds loads.
                let score = unsafe { simd(query, candidate, metric) };
                // A lane that overflowed says nothing about the real score.
                // A tiny score may have lost whole lanes to f32 underflow
                // (squares of values below about 1e-19 vanish) while a tail
                // element kept its f64 value, so it is finite and still wrong.
                // Either is the scalar f64 kernel's to decide; a duplicate or
                // orthogonal pair costs one scalar pass and keeps its answer.
                if score.is_finite() && score.abs() >= SIMD_SCORE_FLOOR {
                    return Ok(score);
                }
            }
            score_dense(query, candidate, metric)
        }
    }
}

/// Below this magnitude a SIMD score is recomputed by the scalar kernel. An
/// f32 product underflows near 1e-38, so a sum of squares this small can have
/// dropped most of its terms; every ordinary embedding distance is far above it.
pub(super) const SIMD_SCORE_FLOOR: f64 = 1e-15;

pub(super) type SimdScore = unsafe fn(&[f32], &[f32], ScoreMetric) -> f64;

/// The SIMD kernel this host can run, if it has one. NEON is part of the
/// aarch64 baseline, so it needs no detection.
pub(super) fn simd_score() -> Option<SimdScore> {
    #[cfg(target_arch = "aarch64")]
    return Some(score_dense_neon);
    #[cfg(not(target_arch = "aarch64"))]
    {
        #[cfg(target_arch = "x86_64")]
        if std::arch::is_x86_feature_detected!("avx2") {
            return Some(score_dense_avx2);
        }
        None
    }
}

/// A cosine from f32 lanes. A norm outside the normal f32 range has lost its
/// meaning, and a zero vector is the scalar kernel's to define, so either
/// returns NaN for the scalar kernel to decide.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub(super) fn cosine_from_lanes(dot: f64, left_norm: f64, right_norm: f64) -> f64 {
    let normal = f64::from(f32::MIN_POSITIVE)..=f64::from(f32::MAX);
    if normal.contains(&left_norm) && normal.contains(&right_norm) {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    } else {
        f64::NAN
    }
}

/// Sums `lane` over the slices sixteen floats at a time, in four groups of
/// registers so the core can run them in parallel, then `tail` over the rest.
#[cfg(target_arch = "aarch64")]
pub(super) unsafe fn neon_sums<const N: usize>(
    left: &[f32],
    right: &[f32],
    lane: impl Fn(
        &mut [std::arch::aarch64::float32x4_t; N],
        std::arch::aarch64::float32x4_t,
        std::arch::aarch64::float32x4_t,
    ),
    tail: impl Fn(&mut [f64; N], f64, f64),
) -> [f64; N] {
    use std::arch::aarch64::*;
    let mut groups = [[vdupq_n_f32(0.0); N]; 4];
    let mut index = 0;
    while index + 16 <= left.len() {
        for (offset, group) in groups.iter_mut().enumerate() {
            let at = index + offset * 4;
            lane(
                group,
                vld1q_f32(left.as_ptr().add(at)),
                vld1q_f32(right.as_ptr().add(at)),
            );
        }
        index += 16;
    }
    let mut sums = [0.0_f64; N];
    for (which, sum) in sums.iter_mut().enumerate() {
        let first = vaddq_f32(groups[0][which], groups[1][which]);
        let second = vaddq_f32(groups[2][which], groups[3][which]);
        *sum = f64::from(vaddvq_f32(vaddq_f32(first, second)));
    }
    for (lhs, rhs) in left[index..].iter().zip(&right[index..]) {
        tail(&mut sums, f64::from(*lhs), f64::from(*rhs));
    }
    sums
}

/// Computes only what its metric needs. NEON is part of the aarch64 baseline.
#[cfg(target_arch = "aarch64")]
pub(super) unsafe fn score_dense_neon(left: &[f32], right: &[f32], metric: ScoreMetric) -> f64 {
    use std::arch::aarch64::*;
    match metric {
        ScoreMetric::Dot => neon_sums::<1>(
            left,
            right,
            |[dot], lhs, rhs| *dot = vfmaq_f32(*dot, lhs, rhs),
            |[dot], lhs, rhs| *dot += lhs * rhs,
        )[0],
        ScoreMetric::Cosine => {
            let [dot, left_norm, right_norm] = neon_sums::<3>(
                left,
                right,
                |[dot, left_norm, right_norm], lhs, rhs| {
                    *dot = vfmaq_f32(*dot, lhs, rhs);
                    *left_norm = vfmaq_f32(*left_norm, lhs, lhs);
                    *right_norm = vfmaq_f32(*right_norm, rhs, rhs);
                },
                |[dot, left_norm, right_norm], lhs, rhs| {
                    *dot += lhs * rhs;
                    *left_norm += lhs * lhs;
                    *right_norm += rhs * rhs;
                },
            );
            cosine_from_lanes(dot, left_norm, right_norm)
        }
        ScoreMetric::Euclidean => -neon_sums::<1>(
            left,
            right,
            |[sum], lhs, rhs| {
                let delta = vsubq_f32(lhs, rhs);
                *sum = vfmaq_f32(*sum, delta, delta);
            },
            |[sum], lhs, rhs| *sum += (lhs - rhs).powi(2),
        )[0]
        .sqrt(),
        ScoreMetric::Manhattan => -neon_sums::<1>(
            left,
            right,
            |[sum], lhs, rhs| *sum = vaddq_f32(*sum, vabdq_f32(lhs, rhs)),
            |[sum], lhs, rhs| *sum += (lhs - rhs).abs(),
        )[0],
    }
}

/// Sums `lane` over the slices eight floats at a time, then `tail` over the rest.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn avx2_sums<const N: usize>(
    left: &[f32],
    right: &[f32],
    lane: impl Fn(
        &mut [std::arch::x86_64::__m256; N],
        std::arch::x86_64::__m256,
        std::arch::x86_64::__m256,
    ),
    tail: impl Fn(&mut [f64; N], f64, f64),
) -> [f64; N] {
    use std::arch::x86_64::*;
    let mut lanes = [_mm256_setzero_ps(); N];
    let mut index = 0;
    while index + 8 <= left.len() {
        lane(
            &mut lanes,
            _mm256_loadu_ps(left.as_ptr().add(index)),
            _mm256_loadu_ps(right.as_ptr().add(index)),
        );
        index += 8;
    }
    let mut sums = [0.0_f64; N];
    for (sum, register) in sums.iter_mut().zip(&lanes) {
        let mut values = [0.0_f32; 8];
        _mm256_storeu_ps(values.as_mut_ptr(), *register);
        *sum = values.iter().map(|value| f64::from(*value)).sum();
    }
    for (lhs, rhs) in left[index..].iter().zip(&right[index..]) {
        tail(&mut sums, f64::from(*lhs), f64::from(*rhs));
    }
    sums
}

/// Computes only what its metric needs.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn score_dense_avx2(left: &[f32], right: &[f32], metric: ScoreMetric) -> f64 {
    use std::arch::x86_64::*;
    match metric {
        ScoreMetric::Dot => avx2_sums::<1>(
            left,
            right,
            |[dot], lhs, rhs| *dot = _mm256_add_ps(*dot, _mm256_mul_ps(lhs, rhs)),
            |[dot], lhs, rhs| *dot += lhs * rhs,
        )[0],
        ScoreMetric::Cosine => {
            let [dot, left_norm, right_norm] = avx2_sums::<3>(
                left,
                right,
                |[dot, left_norm, right_norm], lhs, rhs| {
                    *dot = _mm256_add_ps(*dot, _mm256_mul_ps(lhs, rhs));
                    *left_norm = _mm256_add_ps(*left_norm, _mm256_mul_ps(lhs, lhs));
                    *right_norm = _mm256_add_ps(*right_norm, _mm256_mul_ps(rhs, rhs));
                },
                |[dot, left_norm, right_norm], lhs, rhs| {
                    *dot += lhs * rhs;
                    *left_norm += lhs * lhs;
                    *right_norm += rhs * rhs;
                },
            );
            cosine_from_lanes(dot, left_norm, right_norm)
        }
        ScoreMetric::Euclidean => -avx2_sums::<1>(
            left,
            right,
            |[sum], lhs, rhs| {
                let delta = _mm256_sub_ps(lhs, rhs);
                *sum = _mm256_add_ps(*sum, _mm256_mul_ps(delta, delta));
            },
            |[sum], lhs, rhs| *sum += (lhs - rhs).powi(2),
        )[0]
        .sqrt(),
        ScoreMetric::Manhattan => {
            let sign = _mm256_set1_ps(-0.0);
            -avx2_sums::<1>(
                left,
                right,
                |[sum], lhs, rhs| {
                    *sum = _mm256_add_ps(*sum, _mm256_andnot_ps(sign, _mm256_sub_ps(lhs, rhs)));
                },
                |[sum], lhs, rhs| *sum += (lhs - rhs).abs(),
            )[0]
        }
    }
}

pub(super) fn dense_query(query: &VectorQuery) -> Result<&[f32]> {
    match query {
        VectorQuery::Dense { values } => Ok(values),
        _ => invalid("HNSW currently supports dense queries only"),
    }
}

pub(super) fn dense_candidate(candidate: &VectorCandidate) -> Result<&[f32]> {
    match &candidate.vector.value {
        VectorValue::Dense { values } => Ok(values),
        _ => invalid("HNSW currently supports dense candidates only"),
    }
}
