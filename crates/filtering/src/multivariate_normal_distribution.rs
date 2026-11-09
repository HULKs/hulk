use nalgebra::{SMatrix, SVector};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ros-z", derive(ros_z::Message))]
pub struct MultivariateNormalDistribution<const DIMENSION: usize> {
    pub mean: SVector<f32, DIMENSION>,
    pub covariance: SMatrix<f32, DIMENSION, DIMENSION>,
}
