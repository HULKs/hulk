//! Graph variables with right-side Lie increments and frame-tagged storage.
//!
//! Fagra's `compose` and `inverse` operate algebraically and return `Self`;
//! they are not frame-changing physical transform operations. Physical transforms
//! should use the directed `linear_algebra::Isometry` types at measurement boundaries.

mod camera_intrinsics;
mod field_alignment;
mod imu_bias;
mod pose_control;
pub(crate) mod rotation;
mod trajectory_state;

#[cfg(test)]
mod tests;

pub use camera_intrinsics::CameraIntrinsics;
pub use field_alignment::FieldAlignment;
pub use imu_bias::ImuBias;
pub use pose_control::PoseControl;
pub use trajectory_state::TrajectoryState;
