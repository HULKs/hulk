use std::{boxed::Box, future::Future, pin::Pin, sync::Arc};

use booster::{FallDownState, FallDownStateType, ImuState};
use color_eyre::Result;
use coordinate_systems::Robot;
use linear_algebra::{Isometry3, Orientation3};
use serde::{Deserialize, Serialize};

use filtering::hysteresis::less_than_with_hysteresis;
use kinematics::robot_kinematics::RobotKinematics;
use ros_z::{prelude::*, qos::QosDurability};
use types::{support_foot::Side, time_wrapper::TimeWrapper};

pub const ACTUAL_IMAGE_HEIGHT: f32 = 448.0;
pub const ACTUAL_IMAGE_WIDTH: f32 = 544.0;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub switch_hysteresis: f32,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("support_foot_estimator").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("support_foot_estimator")?;
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

    let support_foot_pub = node
        .publisher::<TimeWrapper<Option<Side>>>("support_foot")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let mut last_support_foot = Side::default();
    let mut last_maybe_support_side = None;

    loop {
        let parameters = parameters.snapshot().typed().clone();

        let imu_state = imu_state_sub.recv_with_metadata().await?;

        let imu_source_time = imu_state.source_time;

        let maybe_robot_kinematics_wrapper = robot_kinematics_cache.get_nearest(imu_source_time);
        let maybe_fall_down_state = fall_down_state_cache.get_nearest(imu_source_time);

        let (Some(robot_kinematics_wrapper), Some(fall_down_state)) =
            (maybe_robot_kinematics_wrapper, maybe_fall_down_state)
        else {
            continue;
        };

        let current_support_foot = estimate_support_foot(
            &parameters,
            &imu_state,
            &robot_kinematics_wrapper.inner,
            last_support_foot,
        );

        let support_foot = if matches!(fall_down_state.fall_down_state, FallDownStateType::IsReady)
        {
            last_support_foot = current_support_foot;
            Some(current_support_foot)
        } else {
            None
        };

        if support_foot != last_maybe_support_side {
            let message = TimeWrapper {
                time: imu_source_time,
                inner: support_foot,
            };

            support_foot_pub.publish(&message).await?;
        }

        last_maybe_support_side = support_foot;
    }
}

fn estimate_support_foot(
    parameters: &Parameters,
    imu_state: &ImuState,
    robot_kinematics: &RobotKinematics,
    last_support_foot: Side,
) -> Side {
    struct Horizontal;

    let imu_orientation = Orientation3::from_euler_angles(
        imu_state.roll_pitch_yaw.x(),
        imu_state.roll_pitch_yaw.y(),
        imu_state.roll_pitch_yaw.z(),
    )
    .mirror();
    let horizontal_to_robot = Isometry3::<Horizontal, Robot>::from(imu_orientation);
    let robot_to_horizontal = horizontal_to_robot.inverse();

    let left_sole_in_horizontal =
        robot_to_horizontal * robot_kinematics.left_leg.sole_to_robot.translation();
    let right_sole_in_horizontal =
        robot_to_horizontal * robot_kinematics.right_leg.sole_to_robot.translation();
    let height_difference = left_sole_in_horizontal.z() - right_sole_in_horizontal.z();

    select_support_side(
        height_difference,
        last_support_foot,
        parameters.switch_hysteresis,
    )
}

fn select_support_side(
    height_difference: f32,
    last_support_side: Side,
    switch_hysteresis: f32,
) -> Side {
    let left_was_lower_last_time = last_support_side == Side::Left;

    let left_sole_is_lower_than_right_sole = less_than_with_hysteresis(
        left_was_lower_last_time,
        height_difference,
        0.0,
        switch_hysteresis,
    );

    if left_sole_is_lower_than_right_sole {
        Side::Left
    } else {
        Side::Right
    }
}
