use nalgebra::{SMatrix, SVector};
use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Message)]
pub struct MultivariateNormalDistribution<const DIMENSION: usize> {
    pub mean: SVector<f32, DIMENSION>,
    pub covariance: SMatrix<f32, DIMENSION, DIMENSION>,
}
