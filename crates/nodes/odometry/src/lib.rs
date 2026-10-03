use std::{boxed::Box, future::Future, pin::Pin, sync::Arc};

use booster::{FallDownState, ImuState};
use color_eyre::Result;
use coordinate_systems::Odometry;
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Pose2;
use ros_z::prelude::*;
use ros_z_streams::CreateAnnouncingPublisher;
use types::odometry::{KINEMATIC_ODOMETRY_TOPIC, KinematicOdometryDelta};
use types::time_wrapper::TimeWrapper;

mod estimator;

pub use estimator::{EstimatorInput, OdometryEstimator, Parameters};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("odometry").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("odometry")?;
    let imu_state_sub = node
        .subscriber::<ImuState>("inputs/imu_state")
        .build()
        .await?;
    let robot_kinematics_cache = node
        .subscriber::<TimeWrapper<RobotKinematics>>("robot_kinematics")
        .cache(10)
        .with_stamp(|wrapper| wrapper.time)
        .build()
        .await?;
    let fall_down_state_cache = node
        .subscriber::<FallDownState>("inputs/fall_down_state")
        .cache(10)
        .build()
        .await?;
    let odometry_pub = node
        .announcing_publisher::<Pose2<Odometry>>("inputs/odometry")
        .await?;

    let mut estimator = OdometryEstimator::default();
    let deltas = node
        .publisher::<KinematicOdometryDelta>(KINEMATIC_ODOMETRY_TOPIC)
        .build()
        .await?;
    let mut last_kinematics_time = None;

    loop {
        let parameters = parameters.snapshot().typed().clone();
        let imu_state = imu_state_sub.recv_with_metadata().await?;
        let imu_time = imu_state.source_time;
        let maybe_robot_kinematics_wrapper = robot_kinematics_cache
            .get_nearest(imu_time)
            .filter(|sample| sample.time.abs_diff(imu_time) <= estimator::MAX_SAMPLE_GAP);
        // Reusing an encoder sample would manufacture a zero-motion observation.
        if let Some(sample) = &maybe_robot_kinematics_wrapper {
            if last_kinematics_time.is_some_and(|last| sample.time <= last) {
                continue;
            }
            last_kinematics_time = Some(sample.time);
        }
        let maybe_fall_down_state = fall_down_state_cache.get_nearest(imu_time);
        let imu_state = imu_state.into_message();

        if let Some(pose) = estimator.update(
            EstimatorInput {
                time: imu_time,
                imu_state: &imu_state,
                robot_kinematics: maybe_robot_kinematics_wrapper
                    .as_deref()
                    .map(|wrapper| &wrapper.inner),
                fall_down_state: maybe_fall_down_state.as_deref(),
            },
            &parameters,
        ) {
            odometry_pub
                .announce(imu_time)
                .await?
                .publish(&pose)
                .await?;
        }
        if let Some(delta) = estimator.take_delta() {
            deltas.publish(&delta).await?;
        }
    }
}
