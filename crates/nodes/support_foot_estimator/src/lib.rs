use std::sync::Arc;

use booster::{FallDownState, FallDownStateType, ImuState};
use color_eyre::Result;
use coordinate_systems::Robot;
use linear_algebra::{Isometry3, Orientation3};
use serde::{Deserialize, Serialize};

use filtering::hysteresis::less_than_with_hysteresis;
use kinematics::robot_kinematics::RobotKinematics;
use ros_z::{IntoEyreResultExt, prelude::*, qos::QosDurability};
use types::support_foot::Side;

pub const ACTUAL_IMAGE_HEIGHT: f32 = 448.0;
pub const ACTUAL_IMAGE_WIDTH: f32 = 544.0;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub switch_hysteresis: f32,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("camera_matrix_calculator")
        .build()
        .await
        .into_eyre()?;

    let parameters = node
        .bind_parameter_as::<Parameters>("support_foot_estimator")
        .into_eyre()?;
    let imu_state_sub = node
        .subscriber::<ImuState>("inputs/imu_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let robot_kinematics_cache = node
        .create_cache::<RobotKinematics>("robot_kinematics", 10)
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let fall_down_state_cache = node
        .create_cache::<FallDownState>("inputs/fall_down_state", 10)
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    let support_foot_pub = node
        .publisher::<Option<Side>>("support_foot")
        .into_eyre()?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await
        .into_eyre()?;

    let mut last_support_side = Side::default();
    let mut last_maybe_support_side = None;

    loop {
        let parameters = parameters.snapshot().typed().clone();

        let imu_state = imu_state_sub.recv_with_metadata().await.into_eyre()?;

        let time_stamp = imu_state.source_time;

        let maybe_robot_kinematics = robot_kinematics_cache.get_nearest(time_stamp);
        let maybe_fall_down_state = fall_down_state_cache.get_nearest(time_stamp);

        let (Some(robot_kinematics), Some(fall_down_state)) =
            (maybe_robot_kinematics, maybe_fall_down_state)
        else {
            continue;
        };

        let current_support_side = estimate_support_side(
            &parameters,
            &imu_state,
            &robot_kinematics,
            last_support_side,
        );

        let support_side = if matches!(fall_down_state.fall_down_state, FallDownStateType::IsReady)
        {
            last_support_side = current_support_side;
            Some(current_support_side)
        } else {
            None
        };

        if support_side != last_maybe_support_side {
            support_foot_pub.publish(&support_side).await.into_eyre()?;
        }

        last_maybe_support_side = support_side;
    }
}

fn estimate_support_side(
    parameters: &Parameters,
    imu_state: &ImuState,
    robot_kinematics: &RobotKinematics,
    last_support_side: Side,
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
        last_support_side,
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
