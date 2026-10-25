//! Typed measurement models with analytical right-increment Jacobians.
//!
//! Durations are in seconds; `tau` is normalized time within an interval, in [0, 1].
//! Information roots multiply residuals and Jacobians on the left:
//! `Wᵀ W = covariance⁻¹`, with covariance expressed in the documented residual
//! coordinates. All Jacobians differentiate the variables' right-side retraction.
//!
//! # Evaluation contract
//! Ordinary costs are `0.5 * ||W r||²`. Robust costs are the true Huber objective
//! of each observation's whitened residual norm. Linearization uses one frozen
//! IRLS weight per observation, applied to both residual and Jacobians; the weight
//! is not differentiated. Test raw residual derivatives separately from the
//! robust objective's gradient (`Jᵀr`).
//!
//! Batches evaluate only the supplied selection. Empty selections return zero
//! cost and emit nothing, without looking up states. Each selected observation
//! opens exactly one `sink.factor` scope, even if it emits multiple residual
//! blocks. Ordinary factors emit directly into their already-open scope.
//! Every observation declares all its dependencies, including batch-model keys.
//! Shared geometry is recomputed per evaluation call, including optimizer trials.
//!
//! Constructors/evaluators must validate finite inputs, positive durations, noise
//! scales and Huber thresholds, normalized times, and measurement-specific domains.
//! `controls` holds four consecutive, distinct pose-control keys for a uniform
//! cubic segment, in the order documented by [`crate::spline::PoseSpline`]. Adjacent
//! odometry uses five controls for two overlapping segments. `duration` is the
//! uniform knot spacing. Keys themselves do not encode timestamps.
//! Measurement and prior states are evaluated on the spline, never equated with
//! control poses. Invalid evaluations return `EvaluationError` rather
//! than silently dropping observations. Supplied information roots must be finite
//! and represent the intended residual-space noise; covariance decomposition belongs
//! in measurement/configuration preparation, not in the evaluation hot path.
//!
//! # Schema registration
//! Batch payload types identify factor families. Same-interval odometry uses
//! `VisualOdometryObservation`; adjacent odometry remains an ordinary factor.
//!
//! ```
//! use localization_fagra::{factors::*, variables::*};
//!
//! fagra::states! {
//!     States<R> {
//!         trajectory: PoseControl<R>,
//!         alignment: FieldAlignment<R>,
//!         intrinsics: CameraIntrinsics<R>,
//!         biases: ImuBias<R>,
//!     }
//! }
//! fagra::factors! {
//!     Factors<R> {
//!         trajectory_priors: TrajectoryPrior<R>,
//!         intrinsics_priors: CameraIntrinsicsPrior<R>,
//!         bias_priors: ImuBiasPrior<R>,
//!         bias_walks: ImuBiasWalk<R>,
//!         motion: MotionPrior<R>,
//!         imu: ImuKinematics<R>,
//!         yaw: RelativeYaw<R>,
//!         feet: Batch<FootGround<R>, FootObservation<R>>,
//!         containment: FieldContainment<R>,
//!         reprojections: Batch<FrameReprojections<R>, ReprojectionObservation<R>>,
//!         odometry: Batch<VisualOdometry<R>, VisualOdometryObservation<R>>,
//!         adjacent_odometry: AdjacentVisualOdometry<R>,
//!     }
//! }
//! let _single = fagra::Solver::<States<f32>, Factors<f32>>::new();
//! let _double = fagra::Solver::<States<f64>, Factors<f64>>::new();
//! ```

mod common;
mod field_containment;
mod ground;
mod imu;
mod imu_bias;
mod kinematic_odometry;
mod motion;
mod preintegrated_imu;
mod prior;
mod reprojection;
mod visual_odometry;

#[cfg(test)]
mod tests;

pub use field_containment::FieldContainment;
pub use ground::{FootGround, FootObservation};
pub use imu::{ImuKinematics, RelativeYaw};
pub use imu_bias::{ImuBiasPrior, ImuBiasWalk};
pub use kinematic_odometry::{AdjacentKinematicOdometry, KinematicOdometry};
pub use motion::MotionPrior;
pub use preintegrated_imu::PreintegratedImu;
pub use prior::{CameraIntrinsicsPrior, TrajectoryPrior};
pub use reprojection::{FrameReprojections, ReprojectionObservation};
pub use visual_odometry::{AdjacentVisualOdometry, VisualOdometry, VisualOdometryObservation};
