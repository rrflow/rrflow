//! Packed inner-product layout and the TurboQuant query.

use super::*;

/// Coordinates an inner-product code covers: whole bytes of indices and of
/// signs at every stage width.
pub(crate) fn inner_product_padded_dimensions(dimensions: usize) -> usize {
    dimensions.next_multiple_of(8)
}

/// The packed bytes of one inner-product code.
pub(crate) fn inner_product_packed_len(
    bits: TurboQuantBits,
    padded_dimensions: usize,
) -> Option<usize> {
    let (stage_bits, _) = bits.inner_product_stage()?;
    Some(packed_len(padded_dimensions, stage_bits) + packed_len(padded_dimensions, 1))
}

/// A query prepared once for scoring many stored vectors of one generation:
/// rotated once, and projected once with S for the inner-product estimator.
#[derive(Debug, Clone)]
pub struct TurboQuantQuery {
    metric: ScoreMetric,
    bits: TurboQuantBits,
    dimensions: usize,
    padded_dimensions: usize,
    query_norm: f32,
    query: Vec<f32>,
    rotation: Rotation,
    rotated: Vec<f64>,
    /// S·Πy for the inner-product estimator.
    projected: Option<Vec<f64>>,
}

impl TurboQuantQuery {
    pub fn mse(
        query: &[f32],
        metric: ScoreMetric,
        bits: TurboQuantBits,
        seed: u64,
    ) -> Result<Self> {
        Self::prepare(
            query,
            metric,
            bits,
            seed,
            bits.padded_dimensions(query.len()),
            None,
        )
    }

    pub fn inner_product(
        query: &[f32],
        metric: ScoreMetric,
        bits: TurboQuantBits,
        seed: u64,
        projection: &QjlProjection,
    ) -> Result<Self> {
        if bits.inner_product_stage().is_none() {
            return invalid("1.5-bit TurboQuant has no inner-product estimator");
        }
        if metric == ScoreMetric::Manhattan {
            return invalid(
                "the TurboQuant inner-product estimator scores dot, cosine and euclidean metrics",
            );
        }
        let padded_dimensions = inner_product_padded_dimensions(query.len());
        if projection.dimensions() != padded_dimensions || !projection.matches_seed(seed) {
            return invalid("the QJL projection differs from the query's dimensions or seed");
        }
        Self::prepare(
            query,
            metric,
            bits,
            seed,
            padded_dimensions,
            Some(projection),
        )
    }

    fn prepare(
        query: &[f32],
        metric: ScoreMetric,
        bits: TurboQuantBits,
        seed: u64,
        padded_dimensions: usize,
        projection: Option<&QjlProjection>,
    ) -> Result<Self> {
        if query.is_empty()
            || query.len() > MAX_DIMENSIONS
            || query.iter().any(|value| !value.is_finite())
        {
            return invalid("TurboQuant score requires finite vectors with matching dimensions");
        }
        let query_norm = l2_norm(query);
        if metric == ScoreMetric::Cosine && query_norm == 0.0 {
            return invalid("cosine query must have non-zero norm");
        }
        let mut rotated = vec![0.0_f64; padded_dimensions];
        for (target, source) in rotated.iter_mut().zip(query) {
            *target = f64::from(*source);
        }
        let rotation = Rotation::new(padded_dimensions, seed);
        rotation.apply(&mut rotated);
        let projected = projection
            .map(|projection| projection.project(&rotated))
            .transpose()?;
        Ok(Self {
            metric,
            bits,
            dimensions: query.len(),
            padded_dimensions,
            query_norm,
            query: query.to_vec(),
            rotation,
            rotated,
            projected,
        })
    }

    /// TurboQuant_mse's score of one stored vector.
    pub fn score_mse(
        &self,
        original_norm: f32,
        centroid_norm: f32,
        packed: &[u8],
        kernel: QuantizedKernel,
    ) -> Result<f64> {
        if self.projected.is_some()
            || self.padded_dimensions != self.bits.padded_dimensions(self.dimensions)
            || packed.len() != packed_len(self.padded_dimensions, self.bits.stored_bits())
        {
            return invalid("TurboQuant score requires finite vectors with matching dimensions");
        }
        let codebook = self.bits.centroids();
        let indices = unpack(packed, self.padded_dimensions, self.bits.stored_bits());
        if indices
            .iter()
            .any(|index| usize::from(*index) >= codebook.len())
        {
            return invalid("TurboQuant packed centroid index is out of range");
        }
        let centroids = indices
            .into_iter()
            .map(|index| f64::from(codebook[usize::from(index)]))
            .collect::<Vec<_>>();
        let raw_dot = dot(&self.rotated, &centroids, kernel);
        let scale = if original_norm == 0.0 {
            0.0
        } else {
            f64::from(original_norm) / f64::from(centroid_norm)
        };
        let dot = raw_dot * scale;
        match self.metric {
            ScoreMetric::Dot => Ok(dot),
            ScoreMetric::Cosine => {
                if original_norm == 0.0 {
                    Ok(0.0)
                } else {
                    Ok(raw_dot / (f64::from(self.query_norm) * f64::from(centroid_norm)))
                }
            }
            ScoreMetric::Euclidean => {
                let squared = f64::from(self.query_norm).powi(2) + f64::from(original_norm).powi(2)
                    - 2.0 * dot;
                Ok(-squared.max(0.0).sqrt())
            }
            ScoreMetric::Manhattan => {
                let mut reconstructed = centroids
                    .into_iter()
                    .map(|value| value * scale)
                    .collect::<Vec<_>>();
                self.rotation.apply_inverse(&mut reconstructed);
                Ok(-self
                    .query
                    .iter()
                    .map(|value| f64::from(*value))
                    .zip(reconstructed)
                    .take(self.dimensions)
                    .map(|(left, right)| (left - right).abs())
                    .sum::<f64>())
            }
        }
    }

    /// TurboQuant_prod's score of one stored vector: from the estimate
    /// ⟨y, x⟩ ≈ ‖x‖·(⟨Πy, c⟩/√d + √(π/2)/d · γ · ⟨S·Πy, signs⟩).
    pub fn score_inner_product(
        &self,
        original_norm: f32,
        residual_norm: f32,
        packed: &[u8],
        kernel: QuantizedKernel,
    ) -> Result<f64> {
        let (Some(projected), Some((stage_bits, codebook))) =
            (&self.projected, self.bits.inner_product_stage())
        else {
            return invalid("an inner-product score requires a query prepared with its projection");
        };
        let stage_len = packed_len(self.padded_dimensions, stage_bits);
        if packed.len() != stage_len + packed_len(self.padded_dimensions, 1) {
            return invalid("TurboQuant inner-product code length differs from its dimensions");
        }
        let root = (self.padded_dimensions as f64).sqrt();
        let stage = if stage_bits == 0 {
            0.0
        } else {
            let indices = unpack(&packed[..stage_len], self.padded_dimensions, stage_bits);
            if indices
                .iter()
                .any(|index| usize::from(*index) >= codebook.len())
            {
                return invalid("TurboQuant packed centroid index is out of range");
            }
            let centroids = indices
                .into_iter()
                .map(|index| f64::from(codebook[usize::from(index)]))
                .collect::<Vec<_>>();
            dot(&self.rotated, &centroids, kernel) / root
        };
        let signs = unpack(&packed[stage_len..], self.padded_dimensions, 1)
            .into_iter()
            .map(|sign| if sign == 1 { 1.0 } else { -1.0 })
            .collect::<Vec<_>>();
        let correction = (std::f64::consts::PI / 2.0).sqrt() / self.padded_dimensions as f64
            * f64::from(residual_norm)
            * dot(projected, &signs, kernel);
        let unit = stage + correction;
        let dot = f64::from(original_norm) * unit;
        match self.metric {
            ScoreMetric::Dot => Ok(dot),
            ScoreMetric::Cosine => {
                if original_norm == 0.0 {
                    Ok(0.0)
                } else {
                    Ok(unit / f64::from(self.query_norm))
                }
            }
            ScoreMetric::Euclidean => {
                let squared = f64::from(self.query_norm).powi(2) + f64::from(original_norm).powi(2)
                    - 2.0 * dot;
                Ok(-squared.max(0.0).sqrt())
            }
            ScoreMetric::Manhattan => invalid(
                "the TurboQuant inner-product estimator scores dot, cosine and euclidean metrics",
            ),
        }
    }
}
