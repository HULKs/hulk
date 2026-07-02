use std::time::SystemTime;

use factrs::{core::Vector3, variables::SE23};

#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// The timestamp of the most recent variable in the optimized graph.
    pub time: SystemTime,
    /// The latest pose estimate from the optimized graph.
    pub latest_pose: SE23<f64>,
    /// The current optimized camera intrinsics estimate.
    pub camera_intrinsics: crate::camera_intrinsics::CameraIntrinsics<f64>,
}

impl OptimizationResult {
    pub fn position(&self) -> Vector3<f64> {
        self.latest_pose.xyz().into_owned()
    }
}
