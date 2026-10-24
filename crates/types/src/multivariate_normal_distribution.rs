use nalgebra::{SMatrix, SVector};
use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Message)]
pub struct MultivariateNormalDistribution<const DIMENSION: usize> {
    pub mean: SVector<f32, DIMENSION>,
    pub covariance: SMatrix<f32, DIMENSION, DIMENSION>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::{
        CdrCodec,
        message::{WireDecoder, WireEncoder},
    };

    fn roundtrip<const D: usize>()
    where
        SVector<f32, D>: Message,
        SMatrix<f32, D, D>: Message,
    {
        let value = MultivariateNormalDistribution::<D> {
            mean: SVector::zeros(),
            covariance: SMatrix::identity(),
        };
        let bytes = CdrCodec::<MultivariateNormalDistribution<D>>::serialize(&value).unwrap();
        let decoded = CdrCodec::<MultivariateNormalDistribution<D>>::deserialize(&bytes).unwrap();
        assert_eq!(decoded, value);
        MultivariateNormalDistribution::<D>::schema()
            .validate()
            .unwrap();
    }

    #[test]
    fn supported_const_generic_matrices_are_messages() {
        roundtrip::<2>();
        roundtrip::<3>();
        roundtrip::<4>();
    }
}
