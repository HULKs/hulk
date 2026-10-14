use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::Result;

use coordinate_systems::{Ground, Robot};
use kinematics::{forward::head_to_camera, robot_kinematics::RobotKinematics};
use linear_algebra::{Isometry3, Rotation3, vector};
use projection::camera_matrix::CameraMatrix;
use ros_z::prelude::*;
use ros_z::qos::QosDurability;
use ros2::sensor_msgs::camera_info::CameraInfo;
use types::{
    camera_geometry::CameraGeometry, parameters::CameraMatrixParameters, time_wrapper::TimeWrapper,
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("camera_matrix_calculator").build().await?;

    let parameters =
        node.bind_parameter_as::<CameraMatrixParameters>("camera_matrix_calculator")?;
    let capacities = parameters.snapshot().typed.synchronization.clone();
    let startup = capacities.clone();
    parameters.add_validation_hook(move |candidate| {
        let p = &candidate.synchronization;
        if p.max_ground_time_distance.is_zero()
            || p.ground_cache_capacity == 0
            || p.camera_info_cache_capacity == 0
        {
            return Err("camera synchronization gap and capacities must be positive".into());
        }
        if p.ground_cache_capacity != startup.ground_cache_capacity
            || p.camera_info_cache_capacity != startup.camera_info_cache_capacity
        {
            return Err("camera cache capacity changes require restart".into());
        }
        Ok(())
    })?;
    let robot_kinematics_sub = node
        .subscriber::<TimeWrapper<RobotKinematics>>("robot_kinematics")
        .build()
        .await?;
    let robot_to_ground_cache = node
        .subscriber::<TimeWrapper<Option<Isometry3<Robot, Ground>>>>("robot_to_ground")
        .cache(capacities.ground_cache_capacity)
        .with_stamp(|w| w.time)
        .build()
        .await?;
    let camera_info_cache = node
        .subscriber::<CameraInfo>("inputs/camera_info")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(capacities.camera_info_cache_capacity)
        .build()
        .await?;

    let camera_matrix_pub = node
        .publisher::<TimeWrapper<CameraMatrix>>("camera_matrix")
        .build()
        .await?;
    let camera_geometry_pub = node
        .publisher::<TimeWrapper<CameraGeometry>>("camera_geometry")
        .build()
        .await?;

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let timed_robot_kinematics = robot_kinematics_sub.recv().await?;
        let time_stamp = timed_robot_kinematics.time;
        let Some(camera_info) = camera_info_cache.get_latest() else {
            continue;
        };
        let ground = robot_to_ground_cache
            .get_nearest(time_stamp)
            .filter(|ground| {
                ground.time.abs_diff(time_stamp)
                    <= parameters.synchronization.max_ground_time_distance
            });
        let (geometry, matrix) = compute_cameras(
            parameters,
            &timed_robot_kinematics.inner,
            &camera_info,
            ground.as_ref().and_then(|ground| ground.inner.as_ref()),
        );
        camera_geometry_pub
            .publish(&TimeWrapper {
                time: time_stamp,
                inner: geometry,
            })
            .await?;

        let Some(camera_matrix) = matrix else {
            continue;
        };

        camera_matrix_pub
            .publish(&TimeWrapper {
                time: time_stamp,
                inner: camera_matrix,
            })
            .await?;
    }
}

fn compute_cameras(
    parameters: &CameraMatrixParameters,
    robot_kinematics: &RobotKinematics,
    camera_info: &CameraInfo,
    robot_to_ground: Option<&Isometry3<Robot, Ground>>,
) -> (CameraGeometry, Option<CameraMatrix>) {
    let image_size = vector![camera_info.width as f32, camera_info.height as f32];
    let (robot_to_head, head_to_camera) = calibrated_transforms(parameters, robot_kinematics);

    let geometry = CameraGeometry {
        robot_to_camera: head_to_camera * robot_to_head,
        intrinsics: projection::intrinsic::Intrinsic::from(camera_info),
    };
    let matrix = robot_to_ground.map(|ground| {
        CameraMatrix::from_camera_info(
            camera_info,
            image_size,
            ground.inverse(),
            robot_to_head,
            head_to_camera,
        )
    });
    (geometry, matrix)
}

fn calibrated_transforms(
    parameters: &CameraMatrixParameters,
    robot_kinematics: &RobotKinematics,
) -> (Isometry3<Robot, Head>, Isometry3<Head, Camera>) {
    let head_to_camera = head_to_camera(parameters.camera_to_head_pitch.to_radians());

    let correction_in_robot = Rotation3::from_euler_angles(
        parameters.correction_in_robot.x(),
        parameters.correction_in_robot.y(),
        parameters.correction_in_robot.z(),
    );
    let correction_in_camera = Rotation3::from_euler_angles(
        parameters.correction_in_camera.x(),
        parameters.correction_in_camera.y(),
        parameters.correction_in_camera.z(),
    );

    (
        robot_kinematics.head.head_to_robot.inverse() * correction_in_robot,
        correction_in_camera * head_to_camera,
    )
}

#[cfg(test)]
mod tests {
    use linear_algebra::vector;
    use ros2::sensor_msgs::camera_info::CameraInfo;
    use types::parameters::CameraMatrixParameters;

    use super::*;

    #[test]
    fn compute_camera_matrix_applies_configured_corrections() {
        let parameters = CameraMatrixParameters {
            camera_to_head_pitch: 0.0,
            correction_in_robot: vector![0.1, -0.2, 0.3],
            correction_in_camera: vector![-0.4, 0.5, -0.6],
            ..Default::default()
        };
        let robot_kinematics = RobotKinematics::default();
        let robot_to_ground = Isometry3::identity();
        let camera_info = camera_info();

        let (geometry, camera_matrix) = compute_cameras(
            &parameters,
            &robot_kinematics,
            &camera_info,
            Some(&robot_to_ground),
        );
        let camera_matrix = camera_matrix.unwrap();
        let (without_ground, missing_matrix) =
            compute_cameras(&parameters, &robot_kinematics, &camera_info, None);
        assert_eq!(without_ground, geometry);
        assert!(missing_matrix.is_none());
        assert_eq!(CameraGeometry::from(&camera_matrix), geometry);

        let zero_parameters = CameraMatrixParameters {
            correction_in_robot: vector![0.0, 0.0, 0.0],
            correction_in_camera: vector![0.0, 0.0, 0.0],
            ..parameters
        };
        let (_, uncorrected_camera_matrix) = compute_cameras(
            &zero_parameters,
            &robot_kinematics,
            &camera_info,
            Some(&robot_to_ground),
        );
        let expected = uncorrected_camera_matrix.unwrap().to_corrected(
            Rotation3::from_euler_angles(0.1, -0.2, 0.3),
            Rotation3::from_euler_angles(-0.4, 0.5, -0.6),
        );

        assert_isometry_near(camera_matrix.ground_to_robot, expected.ground_to_robot);
        assert_isometry_near(camera_matrix.robot_to_head, expected.robot_to_head);
        assert_isometry_near(camera_matrix.head_to_camera, expected.head_to_camera);
        assert_isometry_near(camera_matrix.ground_to_camera, expected.ground_to_camera);
    }

    fn camera_info() -> CameraInfo {
        CameraInfo {
            width: 544,
            height: 448,
            p: [
                210.0, 0.0, 251.0, 0.0, 0.0, 210.0, 232.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            ],
            ..Default::default()
        }
    }

    fn assert_isometry_near<From, To>(actual: Isometry3<From, To>, expected: Isometry3<From, To>) {
        assert!(
            (actual.inner.translation.vector - expected.inner.translation.vector).norm() < 1e-6
        );
        assert!(actual.inner.rotation.angle_to(&expected.inner.rotation) < 1e-6);
    }
}
