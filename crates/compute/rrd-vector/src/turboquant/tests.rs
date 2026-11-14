use super::*;

#[test]
fn rotation_is_norm_preserving_and_invertible_for_irregular_dimensions() {
    for dimensions in [1, 3, 63, 384, 769] {
        let rotation = Rotation::new(dimensions, 42);
        let original = deterministic_vector(dimensions, dimensions as u64);
        let original = original.into_iter().map(f64::from).collect::<Vec<_>>();
        let mut rotated = original.clone();
        rotation.apply(&mut rotated);
        let before = original.iter().map(|value| value * value).sum::<f64>();
        let after = rotated.iter().map(|value| value * value).sum::<f64>();
        assert!((before - after).abs() < 1e-8 * before.max(1.0));
        rotation.apply_inverse(&mut rotated);
        for (expected, actual) in original.iter().zip(rotated) {
            assert!((expected - actual).abs() < 1e-9);
        }
    }
}

#[test]
fn all_bit_modes_are_deterministic_packed_and_score_asymmetrically() {
    let values = deterministic_vector(384, 7);
    let query = deterministic_vector(384, 9);
    for (bits, expected_bytes) in [
        (TurboQuantBits::Bits4, 192),
        (TurboQuantBits::Bits2, 96),
        (TurboQuantBits::Bits1_5, 72),
        (TurboQuantBits::Bits1, 48),
    ] {
        let encoded = TurboQuantVector::encode(&values, bits, 11).unwrap();
        assert_eq!(encoded.packed_vector_bytes(), expected_bytes);
        assert_eq!(
            encoded,
            TurboQuantVector::encode(&values, bits, 11).unwrap()
        );
        assert!(encoded.score(&query, ScoreMetric::Dot).unwrap().is_finite());
        assert!(encoded
            .score(&query, ScoreMetric::Cosine)
            .unwrap()
            .is_finite());
        assert!(encoded
            .score(&query, ScoreMetric::Euclidean)
            .unwrap()
            .is_finite());
        assert!(encoded
            .score(&query, ScoreMetric::Manhattan)
            .unwrap()
            .is_finite());
        for metric in [
            ScoreMetric::Dot,
            ScoreMetric::Cosine,
            ScoreMetric::Euclidean,
            ScoreMetric::Manhattan,
        ] {
            let scalar = encoded
                .score_with_kernel(&query, metric, QuantizedKernel::Scalar)
                .unwrap();
            let automatic = encoded
                .score_with_kernel(&query, metric, QuantizedKernel::Auto)
                .unwrap();
            assert!((scalar - automatic).abs() < 1e-9);
        }
    }
}

#[test]
fn four_bit_reconstruction_and_top_k_recall_are_oracle_gated() {
    let dimensions = 128;
    let corpus = (0..256)
        .map(|index| deterministic_vector(dimensions, index + 100))
        .collect::<Vec<_>>();
    let encoded = corpus
        .iter()
        .map(|vector| TurboQuantVector::encode(vector, TurboQuantBits::Bits4, 91).unwrap())
        .collect::<Vec<_>>();
    let mut recall_total = 0.0;
    for query_seed in 0..16 {
        let query = deterministic_vector(dimensions, query_seed + 10_000);
        let exact = top_k(&corpus, &query, 10, |vector, query| {
            vector
                .iter()
                .zip(query)
                .map(|(left, right)| f64::from(*left) * f64::from(*right))
                .sum()
        });
        let approximate = top_k(&encoded, &query, 10, |vector, query| {
            vector.score(query, ScoreMetric::Dot).unwrap()
        });
        let overlap = exact
            .iter()
            .filter(|identity| approximate.contains(identity))
            .count();
        recall_total += overlap as f64 / 10.0;
    }
    let mean_recall = recall_total / 16.0;
    assert!(mean_recall >= 0.80, "four-bit mean recall@10={mean_recall}");

    let reconstructed = encoded[0].reconstruct().unwrap();
    let original_norm = f64::from(l2_norm(&corpus[0]));
    let reconstructed_norm = f64::from(l2_norm(&reconstructed));
    assert!((original_norm - reconstructed_norm).abs() < 1e-4);
}

fn unit(values: Vec<f32>) -> Vec<f32> {
    let norm = l2_norm(&values);
    values.into_iter().map(|value| value / norm).collect()
}

fn true_dot(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum()
}

/// (estimate, truth) for every pair of 120 unit vectors of 128 dimensions
/// and queries mixed from each vector and noise, so inner products spread
/// from about 0 to 0.9, under several seeds.
fn estimates(bits: TurboQuantBits, inner_product: bool) -> Vec<(f64, f64)> {
    let dimensions = 128;
    let corpus = (0..120)
        .map(|index| unit(deterministic_vector(dimensions, index + 500)))
        .collect::<Vec<_>>();
    let mut pairs = Vec::new();
    for seed in [3, 17, 29, 41] {
        let projection = QjlProjection::new(dimensions, seed).unwrap();
        for (row, vector) in corpus.iter().enumerate() {
            let noise = unit(deterministic_vector(dimensions, row as u64 + 90_000 + seed));
            for weight in [0.0_f32, 0.3, 0.6, 0.9] {
                let query = unit(
                    vector
                        .iter()
                        .zip(&noise)
                        .map(|(signal, noise)| weight * signal + (1.0 - weight) * noise)
                        .collect(),
                );
                pairs.push((
                    estimate(vector, &query, bits, seed, &projection, inner_product),
                    true_dot(vector, &query),
                ));
            }
        }
    }
    pairs
}

fn estimate(
    vector: &[f32],
    query: &[f32],
    bits: TurboQuantBits,
    seed: u64,
    projection: &QjlProjection,
    inner_product: bool,
) -> f64 {
    if inner_product {
        let code = TurboQuantInnerProductCode::encode(vector, bits, seed, projection).unwrap();
        TurboQuantQuery::inner_product(query, ScoreMetric::Dot, bits, seed, projection)
            .unwrap()
            .score_inner_product(
                code.original_norm,
                code.residual_norm,
                &code.packed,
                QuantizedKernel::Auto,
            )
            .unwrap()
    } else {
        let code = TurboQuantVector::encode(vector, bits, seed).unwrap();
        TurboQuantQuery::mse(query, ScoreMetric::Dot, bits, seed)
            .unwrap()
            .score_mse(
                code.original_norm,
                code.centroid_norm,
                &code.packed,
                QuantizedKernel::Auto,
            )
            .unwrap()
    }
}

/// Signed errors of an estimator over the spread of pairs.
fn errors(bits: TurboQuantBits, inner_product: bool) -> Vec<f64> {
    estimates(bits, inner_product)
        .into_iter()
        .map(|(estimate, truth)| estimate - truth)
        .collect()
}

/// The least-squares slope of estimate on truth, and its standard error.
fn slope(pairs: &[(f64, f64)]) -> (f64, f64) {
    let count = pairs.len() as f64;
    let mean_truth = pairs.iter().map(|(_, truth)| truth).sum::<f64>() / count;
    let mean_estimate = pairs.iter().map(|(estimate, _)| estimate).sum::<f64>() / count;
    let spread = pairs
        .iter()
        .map(|(_, truth)| (truth - mean_truth).powi(2))
        .sum::<f64>();
    let slope = pairs
        .iter()
        .map(|(estimate, truth)| (truth - mean_truth) * (estimate - mean_estimate))
        .sum::<f64>()
        / spread;
    let intercept = mean_estimate - slope * mean_truth;
    let residual = pairs
        .iter()
        .map(|(estimate, truth)| (estimate - intercept - slope * truth).powi(2))
        .sum::<f64>()
        / (count - 2.0);
    (slope, (residual / spread).sqrt())
}

#[test]
fn the_mse_estimator_shrinks_inner_products_at_one_bit() {
    // The paper's motivation: an MSE quantizer's inner products are
    // biased by a multiplicative factor, about sqrt(2/pi) at one bit.
    let (one_bit, _) = slope(&estimates(TurboQuantBits::Bits1, false));
    assert!(
        (one_bit - (2.0 / std::f64::consts::PI).sqrt()).abs() < 0.05,
        "one-bit MSE slope {one_bit}"
    );
}

fn mean_and_standard_error(errors: &[f64]) -> (f64, f64) {
    let count = errors.len() as f64;
    let mean = errors.iter().sum::<f64>() / count;
    let variance = errors
        .iter()
        .map(|error| (error - mean).powi(2))
        .sum::<f64>()
        / (count - 1.0);
    (mean, (variance / count).sqrt())
}

#[test]
fn inner_product_estimator_matches_the_bias_and_distortion_fixture() {
    // Theorem 2: D_prod <= sqrt(3)·pi²·|y|²/(d·4^b) for unit x, with b
    // the total bits per coordinate.
    for (bits, total) in [
        (TurboQuantBits::Bits4, 4),
        (TurboQuantBits::Bits2, 2),
        (TurboQuantBits::Bits1, 1),
    ] {
        let errors = errors(bits, true);
        let (mean, standard_error) = mean_and_standard_error(&errors);
        assert!(
            mean.abs() <= 4.0 * standard_error,
            "{bits:?}: mean signed error {mean} against standard error {standard_error}"
        );
        let (slope, slope_error) = slope(&estimates(bits, true));
        assert!(
            (slope - 1.0).abs() <= 4.0 * slope_error,
            "{bits:?}: slope {slope} against standard error {slope_error}"
        );
        let distortion =
            errors.iter().map(|error| error * error).sum::<f64>() / errors.len() as f64;
        let bound = 3f64.sqrt() * std::f64::consts::PI.powi(2) / (128.0 * 4f64.powi(total));
        assert!(
            distortion <= bound,
            "{bits:?}: distortion {distortion} above the paper's bound {bound}"
        );
    }
}

#[test]
fn inner_product_codes_are_deterministic_and_refuse_what_they_cannot_encode() {
    let values = deterministic_vector(100, 7);
    let projection = QjlProjection::new(104, 11).unwrap();
    for (bits, bytes) in [
        (TurboQuantBits::Bits4, 52),
        (TurboQuantBits::Bits2, 26),
        (TurboQuantBits::Bits1, 13),
    ] {
        let code = TurboQuantInnerProductCode::encode(&values, bits, 11, &projection).unwrap();
        assert_eq!(code.packed.len(), bytes, "{bits:?}");
        assert_eq!(
            code,
            TurboQuantInnerProductCode::encode(&values, bits, 11, &projection).unwrap()
        );
        assert!(code.residual_norm > 0.0 && code.residual_norm <= 1.0);
        let query = deterministic_vector(100, 9);
        let prepared =
            TurboQuantQuery::inner_product(&query, ScoreMetric::Cosine, bits, 11, &projection)
                .unwrap();
        for kernel in [QuantizedKernel::Scalar, QuantizedKernel::Auto] {
            assert!(prepared
                .score_inner_product(code.original_norm, code.residual_norm, &code.packed, kernel)
                .unwrap()
                .is_finite());
        }
        assert!(prepared
            .score_inner_product(
                code.original_norm,
                code.residual_norm,
                &code.packed[1..],
                QuantizedKernel::Auto
            )
            .is_err());
        assert!(prepared
            .score_mse(code.original_norm, 1.0, &code.packed, QuantizedKernel::Auto)
            .is_err());
    }
    assert!(
        TurboQuantInnerProductCode::encode(&values, TurboQuantBits::Bits1_5, 11, &projection)
            .is_err()
    );
    assert!(TurboQuantInnerProductCode::encode(
        &values,
        TurboQuantBits::Bits4,
        11,
        &QjlProjection::new(96, 11).unwrap()
    )
    .is_err());
    assert!(TurboQuantQuery::inner_product(
        &values,
        ScoreMetric::Manhattan,
        TurboQuantBits::Bits4,
        11,
        &projection
    )
    .is_err());
}

#[test]
fn malformed_and_non_finite_inputs_fail_closed() {
    assert!(TurboQuantVector::encode(&[], TurboQuantBits::Bits4, 1).is_err());
    assert!(TurboQuantVector::encode(&[f32::NAN], TurboQuantBits::Bits4, 1).is_err());
    let mut encoded = TurboQuantVector::encode(&[1.0, 2.0], TurboQuantBits::Bits2, 1).unwrap();
    encoded.packed.clear();
    assert!(encoded.validate().is_err());
}

fn deterministic_vector(dimensions: usize, seed: u64) -> Vec<f32> {
    let mut state = seed;
    (0..dimensions)
        .map(|index| {
            state = mix(state, index as u64 + 1);
            let unit = (state >> 40) as f32 / ((1_u32 << 24) - 1) as f32;
            unit * 2.0 - 1.0
        })
        .collect()
}

fn top_k<T>(
    corpus: &[T],
    query: &[f32],
    limit: usize,
    score: impl Fn(&T, &[f32]) -> f64,
) -> Vec<usize> {
    let mut scores = corpus
        .iter()
        .enumerate()
        .map(|(identity, vector)| (identity, score(vector, query)))
        .collect::<Vec<_>>();
    scores.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    scores
        .into_iter()
        .take(limit)
        .map(|(identity, _)| identity)
        .collect()
}
