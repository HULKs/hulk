use std::time::SystemTime;

use booster::ImuState;
use coordinate_systems::{Camera, Field, Robot};
use factrs::{
    core::{SE3, SO3},
    traits::Variable,
    variables::{MatrixLieGroup, SE23},
};
use linear_algebra::{Isometry3, Point3};
use thiserror::Error;
use tokio::sync::{mpsc::UnboundedSender, watch};

use crate::InitialState;
use crate::backend::OptimizationResult as BackendOptimizationResult;
use crate::camera_intrinsics::CameraIntrinsics;
use crate::conversions::robot_to_field_to_se23;
use crate::factors::{
    foot_above_ground::FootHeightMeasurement, visual_odometry::VisualOdometryMeasurement,
};
use crate::measurements::{
    GlobalPoseMeasurement, ImuMeasurement, ResetMeasurement, SensorMeasurement,
    VisualReprojectionAssociation, VisualReprojectionAssociationKind,
    VisualReprojectionMeasurement,
};

pub struct VinsFrontend {
    measurement_sender: UnboundedSender<SensorMeasurement>,
    result_receiver: watch::Receiver<Option<BackendOptimizationResult>>,
}

#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub time: SystemTime,
    pub transform: nalgebra::Isometry3<f64>,
    pub velocity: nalgebra::Vector3<f64>,
    pub camera_intrinsics: CameraIntrinsics<f64>,
}

impl VinsFrontend {
    pub fn new(
        measurement_sender: UnboundedSender<SensorMeasurement>,
        result_receiver: watch::Receiver<Option<BackendOptimizationResult>>,
    ) -> Self {
        Self {
            measurement_sender,
            result_receiver,
        }
    }

    pub async fn wait_for_optimization_result(&mut self) -> Result<(), VinsFrontendError> {
        self.result_receiver
            .changed()
            .await
            .map_err(|_| VinsFrontendError::BackendDisconnected)
    }

    /// Returns the latest backend result and marks it observed for `wait_for_optimization_result`.
    pub fn last_optimization_result(&mut self) -> Option<OptimizationResult> {
        self.result_receiver
            .borrow_and_update()
            .as_ref()
            .map(optimization_result_from_backend_result)
    }

    /// Returns the latest backend result without marking it observed.
    pub fn peek_last_optimization_result(&self) -> Option<OptimizationResult> {
        self.result_receiver
            .borrow()
            .as_ref()
            .map(optimization_result_from_backend_result)
    }

    /// Adds an IMU measurement to the optimization pipeline.
    pub fn ingest_imu(
        &mut self,
        time: SystemTime,
        state: ImuState,
    ) -> Result<(), VinsFrontendError> {
        let measurement = ImuMeasurement { time, state };
        self.measurement_sender
            .send(SensorMeasurement::Imu(measurement))
            .map_err(|_| VinsFrontendError::BackendDisconnected)
    }

    /// Reinitializes the backend around an accepted global visual pose.
    pub fn ingest_global_pose(
        &mut self,
        time: SystemTime,
        robot_to_field: Isometry3<Robot, Field>,
    ) -> Result<(), VinsFrontendError> {
        let measurement = GlobalPoseMeasurement {
            time,
            robot_to_field: robot_to_field_to_se23(robot_to_field),
        };

        self.measurement_sender
            .send(SensorMeasurement::GlobalPose(measurement))
            .map_err(|_| VinsFrontendError::BackendDisconnected)
    }

    /// Reinitializes the backend to a known startup state.
    pub fn reset(
        &mut self,
        time: SystemTime,
        initial_state: InitialState,
    ) -> Result<(), VinsFrontendError> {
        let measurement = ResetMeasurement {
            time,
            initial_state,
        };

        self.measurement_sender
            .send(SensorMeasurement::Reset(measurement))
            .map_err(|_| VinsFrontendError::BackendDisconnected)
    }

    /// Adds fixed visual feature associations to the optimization pipeline.
    pub fn ingest_visual_reprojection_associations(
        &mut self,
        time: SystemTime,
        associations: impl IntoIterator<Item = VisualReprojectionAssociation>,
        robot_to_camera: Isometry3<Robot, Camera>,
    ) -> Result<(), VinsFrontendError> {
        let robot_to_camera = isometry3_to_se3(robot_to_camera.inner);
        let mut global_measurements = Vec::new();
        let mut pose_hint_measurements = Vec::new();

        for association in associations {
            let measurement = VisualReprojectionMeasurement {
                time,
                detection: association.detection.inner.cast(),
                field_point: association.field_point.inner.cast(),
                robot_to_camera: robot_to_camera.clone(),
            };
            match association.kind {
                VisualReprojectionAssociationKind::GlobalUnique => {
                    global_measurements.push(measurement)
                }
                VisualReprojectionAssociationKind::PoseHint => {
                    pose_hint_measurements.push(measurement)
                }
            }
        }

        self.send_visual_measurements(SensorMeasurement::Visual, global_measurements)?;
        self.send_visual_measurements(SensorMeasurement::PoseHintVisual, pose_hint_measurements)
    }

    /// Adds a frame-to-frame visual odometry delta to the optimization pipeline.
    pub fn ingest_visual_odometry_delta(
        &mut self,
        previous_time: SystemTime,
        current_time: SystemTime,
        previous_robot_to_left_camera: Isometry3<Robot, Camera>,
        current_robot_to_left_camera: Isometry3<Robot, Camera>,
        current_left_camera_to_previous_left_camera: nalgebra::Isometry3<f32>,
    ) -> Result<(), VinsFrontendError> {
        let current_robot_to_previous_robot = previous_robot_to_left_camera.inner.inverse()
            * current_left_camera_to_previous_left_camera
            * current_robot_to_left_camera.inner;
        let measurement = VisualOdometryMeasurement {
            previous_time,
            current_time,
            robot_delta: isometry3_to_se3(current_robot_to_previous_robot),
        };

        self.measurement_sender
            .send(SensorMeasurement::VisualOdometry(measurement))
            .map_err(|_| VinsFrontendError::BackendDisconnected)
    }

    /// Adds sole positions to keep the estimated feet above the ground plane.
    pub fn ingest_foot_heights(
        &mut self,
        time: SystemTime,
        left_sole_in_robot: Point3<Robot>,
        right_sole_in_robot: Point3<Robot>,
    ) -> Result<(), VinsFrontendError> {
        let measurement = FootHeightMeasurement {
            time,
            left_sole_in_robot,
            right_sole_in_robot,
        };

        self.measurement_sender
            .send(SensorMeasurement::FootHeights(measurement))
            .map_err(|_| VinsFrontendError::BackendDisconnected)
    }

    fn send_visual_measurements(
        &mut self,
        make_measurement: impl FnOnce(Vec<VisualReprojectionMeasurement>) -> SensorMeasurement,
        measurements: Vec<VisualReprojectionMeasurement>,
    ) -> Result<(), VinsFrontendError> {
        if measurements.is_empty() {
            return Ok(());
        }

        self.measurement_sender
            .send(make_measurement(measurements))
            .map_err(|_| VinsFrontendError::BackendDisconnected)
    }
}

#[derive(Debug, Error)]
pub enum VinsFrontendError {
    #[error("the localization backend is disconnected")]
    BackendDisconnected,
}

fn isometry3_to_se3(isometry: nalgebra::Isometry3<f32>) -> SE3 {
    isometry3_f64_to_se3(isometry.cast())
}

fn isometry3_f64_to_se3(isometry: nalgebra::Isometry3<f64>) -> SE3 {
    let rotation = isometry.rotation.quaternion();
    let translation = isometry.translation.vector;

    SE3::from_rot_trans(
        SO3::from_xyzw(rotation.i, rotation.j, rotation.k, rotation.w),
        translation,
    )
}

fn optimization_result_from_backend_result(
    backend_result: &BackendOptimizationResult,
) -> OptimizationResult {
    let (transform, velocity) = se23_to_isometry3_and_velocity(&backend_result.latest_pose);
    OptimizationResult {
        time: backend_result.time,
        transform,
        velocity,
        camera_intrinsics: backend_result.camera_intrinsics.clone(),
    }
}

pub(crate) fn se23_to_isometry3_and_velocity(
    se23: &SE23,
) -> (nalgebra::Isometry3<f64>, nalgebra::Vector3<f64>) {
    let rotation = se23.rot();
    let local_velocity = rotation.inverse().apply(se23.uvw());

    let isometry = nalgebra::Isometry3::from_parts(
        nalgebra::Translation3::from(se23.xyz().into_owned()),
        nalgebra::UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
            rotation.w(),
            rotation.x(),
            rotation.y(),
            rotation.z(),
        )),
    );

    (isometry, local_velocity)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use crate::measurements::SensorMeasurement;

    use super::*;

    fn translation(x: f32, y: f32, z: f32) -> Isometry3<Robot, Camera> {
        Isometry3::from_translation(x, y, z)
    }

    #[test]
    fn visual_odometry_delta_uses_endpoint_camera_extrinsics() {
        let (measurement_sender, mut measurement_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (_result_sender, result_receiver) = tokio::sync::watch::channel(None);
        let mut frontend = VinsFrontend::new(measurement_sender, result_receiver);
        let previous_time = SystemTime::UNIX_EPOCH;
        let current_time = previous_time + Duration::from_millis(33);

        frontend
            .ingest_visual_odometry_delta(
                previous_time,
                current_time,
                translation(1.0, 0.0, 0.0),
                translation(2.0, 0.0, 0.0),
                nalgebra::Isometry3::translation(0.5, 0.0, 0.0),
            )
            .expect("visual odometry should ingest");

        let SensorMeasurement::VisualOdometry(measurement) = measurement_receiver
            .try_recv()
            .expect("measurement should be queued")
        else {
            panic!("expected visual odometry measurement");
        };

        assert_eq!(measurement.previous_time, previous_time);
        assert_eq!(measurement.current_time, current_time);
        assert!((measurement.robot_delta.xyz().x - 1.5).abs() < 1.0e-9);
    }
}
