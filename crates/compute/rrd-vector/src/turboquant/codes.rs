//! TurboQuant format, bit widths, estimators, vector codes and inner-product codes.

use super::*;

pub const TURBOQUANT_FORMAT_VERSION: u16 = 1;
pub(super) const MAX_DIMENSIONS: usize = 1_048_576;
pub(super) const ROTATION_ROUNDS: usize = 3;

pub(super) const CENTROIDS_1: [f32; 2] = [-0.797_884_6, 0.797_884_6];
pub(super) const CENTROIDS_2: [f32; 4] = [-1.510, -0.4528, 0.4528, 1.510];
/// The 8-level Lloyd-Max quantizer for the standard normal distribution: the
/// MSE stage of a 4-bit inner-product code.
pub(super) const CENTROIDS_3: [f32; 8] =
    [-2.152, -1.344, -0.756, -0.2451, 0.2451, 0.756, 1.344, 2.152];
pub(super) const CENTROIDS_4: [f32; 16] = [
    -2.733, -2.069, -1.618, -1.256, -0.9424, -0.6568, -0.3881, -0.1284, 0.1284, 0.3881, 0.6568,
    0.9424, 1.256, 1.618, 2.069, 2.733,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurboQuantBits {
    Bits4,
    Bits2,
    Bits1_5,
    Bits1,
}

/// How a TurboQuant generation estimates a query's inner product with a
/// stored vector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurboQuantEstimator {
    /// TurboQuant_mse: the reconstruction, rescaled to the vector's length.
    #[default]
    Mse,
    /// TurboQuant_prod: an MSE stage one bit narrower plus the QJL signs of
    /// its residual, which together estimate inner products without bias.
    InnerProduct,
}

impl TurboQuantEstimator {
    pub fn is_mse(&self) -> bool {
        *self == Self::Mse
    }
}

impl TurboQuantBits {
    /// The MSE stage of an inner-product code at this total width: its bits
    /// per coordinate and codebook, one bit narrower, as Algorithm 2
    /// instantiates it. 1.5 bits has no such stage.
    pub(crate) const fn inner_product_stage(self) -> Option<(u8, &'static [f32])> {
        match self {
            Self::Bits4 => Some((3, &CENTROIDS_3)),
            Self::Bits2 => Some((1, &CENTROIDS_1)),
            Self::Bits1 => Some((0, &[])),
            Self::Bits1_5 => None,
        }
    }

    pub(super) const fn stored_bits(self) -> u8 {
        match self {
            Self::Bits4 => 4,
            Self::Bits2 => 2,
            Self::Bits1_5 | Self::Bits1 => 1,
        }
    }

    pub(super) const fn centroids(self) -> &'static [f32] {
        match self {
            Self::Bits4 => &CENTROIDS_4,
            Self::Bits2 => &CENTROIDS_2,
            Self::Bits1_5 | Self::Bits1 => &CENTROIDS_1,
        }
    }

    pub(crate) fn padded_dimensions(self, dimensions: usize) -> usize {
        match self {
            Self::Bits4 => dimensions.next_multiple_of(2),
            Self::Bits2 => dimensions.next_multiple_of(4),
            Self::Bits1_5 => dimensions
                .checked_mul(3)
                .and_then(|value| value.checked_add(1))
                .map(|value| value / 2)
                .unwrap_or(usize::MAX)
                .next_multiple_of(8),
            Self::Bits1 => dimensions.next_multiple_of(8),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurboQuantVector {
    pub format_version: u16,
    pub bits: TurboQuantBits,
    pub seed: u64,
    pub dimensions: usize,
    pub padded_dimensions: usize,
    pub original_norm: f32,
    pub centroid_norm: f32,
    pub packed: Vec<u8>,
}

impl TurboQuantVector {
    pub fn encode(values: &[f32], bits: TurboQuantBits, seed: u64) -> Result<Self> {
        validate_input(values)?;
        let padded_dimensions = bits.padded_dimensions(values.len());
        if padded_dimensions > MAX_DIMENSIONS.saturating_mul(2) {
            return invalid("TurboQuant padded dimensions exceed the codec bound");
        }
        let original_norm = l2_norm(values);
        let mut rotated = vec![0.0_f64; padded_dimensions];
        for (target, source) in rotated.iter_mut().zip(values) {
            *target = f64::from(*source);
        }
        Rotation::new(padded_dimensions, seed).apply(&mut rotated);
        if original_norm > 0.0 {
            let scale = (padded_dimensions as f64).sqrt() / f64::from(original_norm);
            for value in &mut rotated {
                *value *= scale;
            }
        }
        let indices = rotated
            .iter()
            .map(|value| nearest_centroid(bits.centroids(), *value))
            .collect::<Vec<_>>();
        let centroid_norm = indices
            .iter()
            .map(|index| f64::from(bits.centroids()[usize::from(*index)]).powi(2))
            .sum::<f64>()
            .sqrt() as f32;
        let vector = Self {
            format_version: TURBOQUANT_FORMAT_VERSION,
            bits,
            seed,
            dimensions: values.len(),
            padded_dimensions,
            original_norm,
            centroid_norm,
            packed: pack(&indices, bits.stored_bits()),
        };
        vector.validate()?;
        Ok(vector)
    }

    pub fn validate(&self) -> Result<()> {
        if self.format_version != TURBOQUANT_FORMAT_VERSION
            || self.dimensions == 0
            || self.dimensions > MAX_DIMENSIONS
            || self.padded_dimensions != self.bits.padded_dimensions(self.dimensions)
        {
            return invalid("TurboQuant vector format or dimensions are invalid");
        }
        if !self.original_norm.is_finite()
            || self.original_norm < 0.0
            || !self.centroid_norm.is_finite()
            || self.centroid_norm <= 0.0
        {
            return invalid("TurboQuant vector norms are invalid");
        }
        let expected = packed_len(self.padded_dimensions, self.bits.stored_bits());
        if self.packed.len() != expected {
            return invalid("TurboQuant packed byte length differs from its dimensions");
        }
        let levels = self.bits.centroids().len();
        if unpack(
            &self.packed,
            self.padded_dimensions,
            self.bits.stored_bits(),
        )
        .iter()
        .any(|index| usize::from(*index) >= levels)
        {
            return invalid("TurboQuant packed centroid index is out of range");
        }
        Ok(())
    }

    pub fn estimated_payload_bytes(&self) -> usize {
        self.packed.len()
            + std::mem::size_of::<u16>()
            + std::mem::size_of::<u64>()
            + 2 * std::mem::size_of::<usize>()
            + 2 * std::mem::size_of::<f32>()
    }

    pub fn packed_vector_bytes(&self) -> usize {
        self.packed.len()
    }

    pub fn score(&self, query: &[f32], metric: ScoreMetric) -> Result<f64> {
        self.score_with_kernel(query, metric, QuantizedKernel::Auto)
    }

    pub fn score_with_kernel(
        &self,
        query: &[f32],
        metric: ScoreMetric,
        kernel: QuantizedKernel,
    ) -> Result<f64> {
        self.validate()?;
        if query.len() != self.dimensions {
            return invalid("TurboQuant score requires finite vectors with matching dimensions");
        }
        TurboQuantQuery::mse(query, metric, self.bits, self.seed)?.score_mse(
            self.original_norm,
            self.centroid_norm,
            &self.packed,
            kernel,
        )
    }

    pub fn reconstruct(&self) -> Result<Vec<f32>> {
        self.validate()?;
        let scale = if self.original_norm == 0.0 {
            0.0
        } else {
            f64::from(self.original_norm) / f64::from(self.centroid_norm)
        };
        let mut reconstructed = self
            .decoded_centroids()
            .into_iter()
            .map(|value| value * scale)
            .collect::<Vec<_>>();
        Rotation::new(self.padded_dimensions, self.seed).apply_inverse(&mut reconstructed);
        Ok(reconstructed
            .into_iter()
            .take(self.dimensions)
            .map(|value| value as f32)
            .collect())
    }

    fn decoded_centroids(&self) -> Vec<f64> {
        let centroids = self.bits.centroids();
        unpack(
            &self.packed,
            self.padded_dimensions,
            self.bits.stored_bits(),
        )
        .into_iter()
        .map(|index| f64::from(centroids[usize::from(index)]))
        .collect()
    }
}

/// One vector under the inner-product estimator: its MSE-stage indices
/// followed by the QJL signs of its residual, packed in that order, with the
/// vector's length and the unit residual's.
#[derive(Debug, Clone, PartialEq)]
pub struct TurboQuantInnerProductCode {
    pub original_norm: f32,
    /// The MSE stage's centroid norm, for accounting; scoring does not use it.
    pub centroid_norm: f32,
    pub residual_norm: f32,
    pub packed: Vec<u8>,
}

impl TurboQuantInnerProductCode {
    /// Algorithm 2's quantizer. `projection` is S for this seed at the padded
    /// dimensions; a build materializes it once for every vector.
    pub fn encode(
        values: &[f32],
        bits: TurboQuantBits,
        seed: u64,
        projection: &QjlProjection,
    ) -> Result<Self> {
        validate_input(values)?;
        let Some((stage_bits, centroids)) = bits.inner_product_stage() else {
            return invalid("1.5-bit TurboQuant has no inner-product estimator");
        };
        let padded_dimensions = inner_product_padded_dimensions(values.len());
        if projection.dimensions() != padded_dimensions || !projection.matches_seed(seed) {
            return invalid("the QJL projection differs from the vector's dimensions or seed");
        }
        let original_norm = l2_norm(values);
        let mut rotated = vec![0.0_f64; padded_dimensions];
        for (target, source) in rotated.iter_mut().zip(values) {
            *target = f64::from(*source);
        }
        Rotation::new(padded_dimensions, seed).apply(&mut rotated);
        if original_norm > 0.0 {
            let scale = (padded_dimensions as f64).sqrt() / f64::from(original_norm);
            for value in &mut rotated {
                *value *= scale;
            }
        }
        let indices = if stage_bits == 0 {
            Vec::new()
        } else {
            rotated
                .iter()
                .map(|value| nearest_centroid(centroids, *value))
                .collect::<Vec<_>>()
        };
        // The unit vector's residual after its MSE stage, in the rotated space.
        let root = (padded_dimensions as f64).sqrt();
        let residual = rotated
            .iter()
            .enumerate()
            .map(|(coordinate, value)| {
                let reconstructed = indices
                    .get(coordinate)
                    .map_or(0.0, |index| f64::from(centroids[usize::from(*index)]));
                (value - reconstructed) / root
            })
            .collect::<Vec<_>>();
        let residual_norm = residual
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt() as f32;
        let signs = projection
            .project(&residual)?
            .into_iter()
            .map(|value| u8::from(value >= 0.0))
            .collect::<Vec<_>>();
        let centroid_norm = indices
            .iter()
            .map(|index| f64::from(centroids[usize::from(*index)]).powi(2))
            .sum::<f64>()
            .sqrt() as f32;
        let mut packed = pack(&indices, stage_bits);
        packed.extend(pack(&signs, 1));
        Ok(Self {
            original_norm,
            centroid_norm,
            residual_norm,
            packed,
        })
    }
}
