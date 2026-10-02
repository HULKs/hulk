//! Raw measured sensors for production kinematics, plus the chosen absolute torso reference.
use std::f32::consts::PI;

use booster::{ImuState, MotorState};
use color_eyre::Result;
use coordinate_systems::{Field, Robot};
use kinematics::joints::Joints;
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::{prelude::*, time::Time};
use ros2::{sensor_msgs::camera_info::CameraInfo, std_msgs::header::Header};
use types::{
    field_dimensions::GlobalFieldSide, time_wrapper::TimeWrapper,
    visual_localization::LOCALIZATION_POSE_3D_TOPIC,
};

use crate::robot_io::Observation;

pub struct GeometryInputs {
    joints: Publisher<Joints<MotorState>>,
    imu: Publisher<ImuState>,
    camera_info: Publisher<CameraInfo>,
    localization: Publisher<TimeWrapper<Option<Isometry3<Field, Robot>>>>,
}

impl GeometryInputs {
    pub async fn new(node: &Node) -> Result<Self> {
        Ok(Self {
            joints: node.publisher("inputs/serial_motor_states").build().await?,
            imu: node.publisher("inputs/imu_state").build().await?,
            camera_info: node.publisher("inputs/camera_info").build().await?,
            localization: node.publisher(LOCALIZATION_POSE_3D_TOPIC).build().await?,
        })
    }

    pub async fn publish(
        &self,
        observation: &Observation,
        time: Time,
        side: GlobalFieldSide,
    ) -> Result<()> {
        self.camera_info
            .publish_with_source_time(&camera_info(&observation.camera_matrix, time), time)
            .await?;
        self.joints
            .publish_with_source_time(&observation.low_state.serial_motor_states()?, time)
            .await?;
        self.imu
            .publish_with_source_time(&observation.low_state.imu_state, time)
            .await?;
        self.localization
            .publish_with_source_time(
                &TimeWrapper {
                    time,
                    inner: Some(field_to_robot(observation.robot_to_world, side)),
                },
                time,
            )
            .await?;
        Ok(())
    }
}

fn field_to_robot(
    robot_to_world: nalgebra::Isometry3<f32>,
    side: GlobalFieldSide,
) -> Isometry3<Field, Robot> {
    let world_to_field = nalgebra::Isometry3::rotation(
        nalgebra::Vector3::z()
            * if side == GlobalFieldSide::Home {
                0.0
            } else {
                PI
            },
    );
    Isometry3::wrap((world_to_field * robot_to_world).inverse())
}

fn camera_info(camera: &CameraMatrix, time: Time) -> CameraInfo {
    let fx = camera.intrinsics.focals.x as f64;
    let fy = camera.intrinsics.focals.y as f64;
    let cx = camera.intrinsics.optical_center.x() as f64;
    let cy = camera.intrinsics.optical_center.y() as f64;
    CameraInfo {
        header: Header {
            stamp: ros2::builtin_interfaces::time::Time {
                sec: time.as_nanos().div_euclid(1_000_000_000) as i32,
                nanosec: time.as_nanos().rem_euclid(1_000_000_000) as u32,
            },
            frame_id: "left_camera".into(),
        },
        width: camera.image_size.x() as u32,
        height: camera.image_size.y() as u32,
        k: [fx, 0.0, cx, 0.0, fy, cy, 0.0, 0.0, 1.0],
        p: [fx, 0.0, cx, 0.0, 0.0, fy, cy, 0.0, 0.0, 0.0, 1.0, 0.0],
        r: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ball_perception::{Metrics, Parameters, PerceptionIo},
        robot_io::RobotBinding,
    };
    use coordinate_systems::Ground;
    use linear_algebra::{Isometry2, Point3, point};
    use mujoco_rs::prelude::{MjData, MjSpec};
    use ros_z::{qos::QosDurability, time::Clock};
    use std::{path::PathBuf, sync::Arc, time::Duration};

    #[test]
    fn torso_reference_keeps_roll_pitch_height_and_field_side() {
        let torso = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.0, 2.0, 0.6),
            nalgebra::UnitQuaternion::from_euler_angles(0.12, -0.08, 0.7),
        );
        let home = field_to_robot(torso, GlobalFieldSide::Home).inverse();
        assert!(home.inner.rotation.angle_to(&torso.rotation) < 1e-6);
        assert!((home.inner.translation.vector - torso.translation.vector).norm() < 1e-6);
        let away = field_to_robot(torso, GlobalFieldSide::Away).inverse();
        assert!((away.translation() - point![-1.0, -2.0, 0.6]).norm() < 1e-6);
    }

    #[test]
    fn wobbling_mujoco_robot_runs_geometry_and_ball_filter_without_visual_localization() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let clock = Clock::logical(Time::zero());
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("tcp/127.0.0.1:{}", listener.local_addr().unwrap().port());
        drop(listener);
        let (
            context,
            _node,
            _field,
            low,
            geometry,
            mut perception,
            camera,
            ground,
            field_pose,
            odometry,
            metrics,
            mut tasks,
        ) = runtime.block_on(async {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../etc/parameters");
            let context = Arc::new(
                ContextBuilder::default()
                    .with_namespace("/geometry_test")
                    .with_mode("router")
                    .disable_multicast_scouting()
                    .with_connect_endpoints(std::iter::empty::<&str>())
                    .with_listen_endpoints([endpoint.as_str()])
                    .with_parameter_layers([root.join("base"), root.join("location/simulator")])
                    .with_clock(clock.clone())
                    .build()
                    .await
                    .unwrap(),
            );
            let node = context.create_node("test_sensors").build().await.unwrap();
            let field = node
                .publisher::<types::field_dimensions::FieldDimensions>("field_dimensions")
                .qos(QosProfile {
                    durability: QosDurability::TransientLocal,
                    ..Default::default()
                })
                .build()
                .await
                .unwrap();
            field
                .publish(&types::field_dimensions::FieldDimensions {
                    ball_radius: 0.105,
                    ..types::field_dimensions::FieldDimensions::SPL_2025
                })
                .await
                .unwrap();
            let low = node
                .publisher::<booster::LowState>("inputs/low_state")
                .build()
                .await
                .unwrap();
            let geometry = GeometryInputs::new(&node).await.unwrap();
            let perception = PerceptionIo::new(&node, runtime.handle()).await.unwrap();
            let camera = node
                .subscriber::<TimeWrapper<CameraMatrix>>("camera_matrix")
                .cache(1)
                .build()
                .await
                .unwrap();
            let ground = node
                .subscriber::<TimeWrapper<Option<Isometry3<Ground, Robot>>>>("ground_to_robot")
                .cache(1)
                .build()
                .await
                .unwrap();
            let field_pose = node
                .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
                .cache(1)
                .build()
                .await
                .unwrap();
            let odometry = node
                .subscriber::<linear_algebra::Pose2<coordinate_systems::Odometry>>(
                    "inputs/odometry",
                )
                .cache(1)
                .build()
                .await
                .unwrap();
            let metrics = node
                .subscriber::<Metrics>("simulation/ball_filter_metrics")
                .cache(1)
                .build()
                .await
                .unwrap();
            let mut tasks = tokio::task::JoinSet::new();
            tasks.spawn(fall_detection::run_boxed(context.clone()));
            tasks.spawn(kinematics_provider::run_boxed(context.clone()));
            tasks.spawn(support_foot_estimator::run_boxed(context.clone()));
            tasks.spawn(ground_provider::run_boxed(context.clone()));
            tasks.spawn(camera_matrix_calculator::run_boxed(context.clone()));
            tasks.spawn(odometry::run_boxed(context.clone()));
            tasks.spawn(localization_2d::run_boxed(context.clone()));
            tasks.spawn(ball_filter::run_boxed(context.clone()));
            (
                context, node, field, low, geometry, perception, camera, ground, field_pose,
                odometry, metrics, tasks,
            )
        });
        let mut spec =
            MjSpec::from_xml(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/k1_robot.xml")).unwrap();
        let mut data = MjData::new(Box::new(spec.compile().unwrap()));
        data.forward();
        let binding = RobotBinding::new(&data, "").unwrap();
        let pitch_joint = data
            .model()
            .name_to_id(mujoco_rs::prelude::MjtObj::mjOBJ_JOINT, "Head_pitch")
            .unwrap();
        let pitch_address = data.model().jnt_qposadr()[pitch_joint] as usize;
        let mut first_ground = None;
        let mut maximum_ground_change = 0.0_f32;
        let clean = Parameters {
            center_noise_pixels: 0.0,
            false_positive_probability: 0.0,
            ..Default::default()
        };
        for tick in 1..=200 {
            let phase = tick as f32 * 0.06;
            let rotation = nalgebra::UnitQuaternion::from_euler_angles(
                0.06 * phase.sin(),
                0.04 * phase.cos(),
                0.05 * phase.sin(),
            );
            let q = rotation.quaternion();
            data.qpos_mut()[0] = tick as f64 * 0.0005;
            data.qpos_mut()[2] = 0.6 + 0.01 * phase.sin() as f64;
            data.qpos_mut()[3..7]
                .copy_from_slice(&[q.w as f64, q.i as f64, q.j as f64, q.k as f64]);
            data.qpos_mut()[pitch_address] = 0.2 + 0.05 * phase.sin() as f64;
            data.forward();
            let time = Time::from_nanos(tick * 10_000_000);
            clock.set_time(time).unwrap();
            let observation = binding.observe(&data);
            let ball = Point3::wrap(binding.point_in_ground(&data, [2.0, 0.3, 0.105]));
            let truth_pose = crate::behavior_inputs::ground_to_field(
                binding.ground_to_world(&data),
                GlobalFieldSide::Home,
            );
            // Register exact truth before the sensor pipeline can emit odometry at this time.
            perception
                .publish(
                    time,
                    truth_pose,
                    &observation.camera_matrix,
                    vec![ball],
                    0.105,
                    &clean,
                )
                .unwrap();
            runtime.block_on(async {
                low.publish_with_source_time(&observation.low_state, time)
                    .await
                    .unwrap();
                geometry
                    .publish(&observation, time, GlobalFieldSide::Home)
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
            });
            if let Some(result) = tasks.try_join_next() {
                panic!("geometry node exited: {result:?}");
            }
            if let Some(ground) = ground.get_latest().and_then(|ground| ground.inner) {
                let first = *first_ground.get_or_insert(ground);
                maximum_ground_change = maximum_ground_change
                    .max(first.inner.rotation.angle_to(&ground.inner.rotation));
            }
        }
        let estimated_camera = camera.get_latest().expect("kinematic camera missing");
        let true_camera = binding.observe(&data).camera_matrix;
        assert!(
            (estimated_camera
                .inner
                .head_to_camera
                .inner
                .translation
                .vector
                - true_camera.head_to_camera.inner.translation.vector)
                .norm()
                < 1e-4,
            "left optical camera transforms disagree: estimated {:?}; truth {:?}",
            estimated_camera.inner.head_to_camera,
            true_camera.head_to_camera
        );
        assert!(
            estimated_camera
                .inner
                .head_to_camera
                .inner
                .rotation
                .angle_to(&true_camera.head_to_camera.inner.rotation)
                < 1e-4,
            "camera mounting conventions disagree"
        );
        assert!(
            maximum_ground_change > 0.04,
            "kinematic ground did not follow body wobble"
        );
        assert!(
            field_pose.get_latest().is_some(),
            "ground-to-field adapter did not run"
        );
        assert!(odometry.get_latest().is_some(), "leg odometry did not run");
        let metrics = metrics.get_latest().expect("ball filter metrics missing");
        assert!(metrics.matched_samples > 10, "{metrics:?}");
        assert!(metrics.field_matched_samples > 10, "{metrics:?}");
        assert_eq!(metrics.unmatched_timestamps, 0, "{metrics:?}");
        assert!(metrics.position_rmse_metres.unwrap().is_finite());
        assert!(metrics.field_position_rmse_metres.unwrap().is_finite());
        runtime.block_on(async {
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
        });
        context.shutdown().unwrap();
    }
}
