use std::{
    boxed::Box,
    future::Future,
    pin::{Pin, pin},
    sync::Arc,
};

use color_eyre::Result;

use coordinate_systems::{Camera, Ground, Head, Robot};
use futures_util::future::{Either, select};
use kinematics::{forward::head_to_camera, robot_kinematics::RobotKinematics};
use linear_algebra::{Isometry3, Rotation3, vector};
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::prelude::*;
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
    let robot_kinematics_sub = node
        .subscriber::<TimeWrapper<RobotKinematics>>("robot_kinematics")
        .build()
        .await?;
    let robot_kinematics_cache = node
        .subscriber::<TimeWrapper<RobotKinematics>>("robot_kinematics")
        .cache(10)
        .with_stamp(|w: &TimeWrapper<RobotKinematics>| w.time)
        .build()
        .await?;
    let robot_to_ground_sub = node
        .subscriber::<TimeWrapper<Option<Isometry3<Robot, Ground>>>>("robot_to_ground")
        .build()
        .await?;
    let camera_info_cache = node
        .subscriber::<CameraInfo>("inputs/camera_info")
        .cache(1)
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
        let kinematics = pin!(robot_kinematics_sub.recv());
        let ground = pin!(robot_to_ground_sub.recv());
        let received = select(kinematics, ground).await;

        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        match received {
            Either::Left((kinematics, _)) => {
                let kinematics = kinematics?;
                let Some(camera_info) = camera_info_cache.get_latest() else {
                    continue;
                };

                let geometry = compute_camera_geometry(parameters, &kinematics.inner, &camera_info);

                camera_geometry_pub
                    .publish(&TimeWrapper {
                        time: kinematics.time,
                        inner: geometry,
                    })
                    .await?;
            }
            Either::Right((ground, _)) => {
                let ground = ground?;
                let Some(robot_to_ground) = ground.inner else {
                    continue;
                };
                let Some((kinematics, camera_info)) = robot_kinematics_cache
                    .get_nearest(ground.time)
                    .zip(camera_info_cache.get_nearest(ground.time))
                else {
                    continue;
                };

                let matrix = compute_camera_matrix(
                    parameters,
                    &kinematics.inner,
                    &robot_to_ground,
                    &camera_info,
                );

                camera_matrix_pub
                    .publish(&TimeWrapper {
                        time: ground.time,
                        inner: matrix,
                    })
                    .await?;
            }
        }
    }
}

fn compute_camera_matrix(
    parameters: &CameraMatrixParameters,
    robot_kinematics: &RobotKinematics,
    robot_to_ground: &Isometry3<Robot, Ground>,
    camera_info: &CameraInfo,
) -> CameraMatrix {
    let image_size = vector![camera_info.width as f32, camera_info.height as f32];
    let (robot_to_head, head_to_camera) = calibrated_transforms(parameters, robot_kinematics);

    CameraMatrix::from_camera_info(
        camera_info,
        image_size,
        robot_to_ground.inverse(),
        robot_to_head,
        head_to_camera,
    )
}

fn compute_camera_geometry(
    parameters: &CameraMatrixParameters,
    robot_kinematics: &RobotKinematics,
    camera_info: &CameraInfo,
) -> CameraGeometry {
    let (robot_to_head, head_to_camera) = calibrated_transforms(parameters, robot_kinematics);
    CameraGeometry {
        robot_to_camera: head_to_camera * robot_to_head,
        intrinsics: Intrinsic::from(camera_info),
    }
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
        };
        let mut robot_kinematics = RobotKinematics::default();
        robot_kinematics.head.head_to_robot = Isometry3::from_translation(0.02, -0.03, 0.3);
        let robot_to_ground = Isometry3::from_translation(0.1, -0.2, 0.5);
        let camera_info = camera_info();

        let camera_matrix = compute_camera_matrix(
            &parameters,
            &robot_kinematics,
            &robot_to_ground,
            &camera_info,
        );
        let geometry = compute_camera_geometry(&parameters, &robot_kinematics, &camera_info);
        assert_eq!(CameraGeometry::from(&camera_matrix), geometry);
        assert_eq!(geometry.intrinsics, Intrinsic::from(&camera_info));

        let zero_parameters = CameraMatrixParameters {
            correction_in_robot: vector![0.0, 0.0, 0.0],
            correction_in_camera: vector![0.0, 0.0, 0.0],
            ..parameters
        };
        let uncorrected_camera_matrix = compute_camera_matrix(
            &zero_parameters,
            &robot_kinematics,
            &robot_to_ground,
            &camera_info,
        );
        let expected = uncorrected_camera_matrix.to_corrected(
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
