use nalgebra::{SMatrix, SVector};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct MultivariateNormalDistribution<const DIMENSION: usize> {
    #[serialize_hierarchy(leaf)]
    pub mean: SVector<f32, DIMENSION>,
    #[serialize_hierarchy(leaf)]
    pub covariance: SMatrix<f32, DIMENSION, DIMENSION>,
}
