use std::sync::Arc;

use color_eyre::Result;

use booster::ImuState;
use coordinate_systems::{Ground, Robot};
use kinematics::robot_kinematics::RobotKinematics;
use kinematics_provider::RobotKinematicsMessage;
use linear_algebra::{Isometry3, Orientation3, vector};
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use serde::{Deserialize, Serialize};
use support_foot_estimator::SupportFootMessage;
use types::support_foot::Side;

#[derive(Debug, Serialize, Deserialize, Message)]
pub struct RobotToGroundMessage {
    pub time: Time,
    pub robot_to_ground: Option<Isometry3<Robot, Ground>>,
}

#[derive(Debug, Serialize, Deserialize, Message)]
pub struct GroundToRobotMessage {
    pub time: Time,
    pub ground_to_robot: Option<Isometry3<Ground, Robot>>,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("ground_provider").build().await?;

    let imu_state_sub = node
        .subscriber::<ImuState>("inputs/imu_state")?
        .build()
        .await?;

    let robot_kinematics_cache = node
        .create_cache::<RobotKinematicsMessage>("robot_kinematics", 10)?
        .with_stamp(|message| message.time)
        .build()
        .await?;
    let support_foot_cache = node
        .create_cache::<SupportFootMessage>("support_foot", 10)?
        .with_stamp(|message| message.time)
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let robot_to_ground_pub = node
        .publisher::<RobotToGroundMessage>("robot_to_ground")?
        .build()
        .await?;
    let ground_to_robot_pub = node
        .publisher::<GroundToRobotMessage>("ground_to_robot")?
        .build()
        .await?;

    loop {
        let imu_state = imu_state_sub.recv_with_metadata().await?;

        let imu_source_time = imu_state.source_time;

        let maybe_robot_kinematics_message = robot_kinematics_cache.get_nearest(imu_source_time);
        let maybe_support_foot_message = support_foot_cache.get_nearest(imu_source_time);

        let (Some(robot_kinematics_message), Some(support_foot_message)) =
            (maybe_robot_kinematics_message, maybe_support_foot_message)
        else {
            continue;
        };

        let ground_to_robot = if let Some(support_foot) = support_foot_message.support_foot {
            compute_ground_to_robot(
                &imu_state.into_message(),
                &robot_kinematics_message.robot_kinematics,
                &support_foot,
            )
        } else {
            None
        };
        let robot_to_ground = ground_to_robot.map(|ground_to_robot| ground_to_robot.inverse());

        let robot_to_ground_message = RobotToGroundMessage {
            time: imu_source_time,
            robot_to_ground,
        };
        let ground_to_robot_message = GroundToRobotMessage {
            time: imu_source_time,
            ground_to_robot,
        };

        robot_to_ground_pub
            .publish(&robot_to_ground_message)
            .await?;
        ground_to_robot_pub
            .publish(&ground_to_robot_message)
            .await?;
    }
}

fn compute_ground_to_robot(
    imu_state: &ImuState,
    robot_kinematics: &RobotKinematics,
    support_foot: &Side,
) -> Option<Isometry3<Ground, Robot>> {
    struct LeftSoleHorizontal;
    struct RightSoleHorizontal;

    let roll = imu_state.roll_pitch_yaw.x();
    let pitch = imu_state.roll_pitch_yaw.y();

    let imu_orientation = Orientation3::from_euler_angles(roll, pitch, 0.0).mirror();

    let left_sole_horizontal_to_robot = Isometry3::from_parts(
        robot_kinematics
            .left_leg
            .sole_to_robot
            .translation()
            .coords(),
        imu_orientation,
    );
    let right_sole_horizontal_to_robot = Isometry3::from_parts(
        robot_kinematics
            .right_leg
            .sole_to_robot
            .translation()
            .coords(),
        imu_orientation,
    );

    let left_sole_in_robot = robot_kinematics.left_leg.sole_to_robot.translation();
    let right_sole_in_robot = robot_kinematics.right_leg.sole_to_robot.translation();

    let left_sole_to_right_sole = right_sole_in_robot - left_sole_in_robot;
    let ground_to_left_sole = Isometry3::<Ground, LeftSoleHorizontal>::from(
        vector![
            left_sole_to_right_sole.x(),
            left_sole_to_right_sole.y(),
            0.0
        ] / 2.0,
    );
    let ground_to_right_sole = Isometry3::<Ground, RightSoleHorizontal>::from(
        -vector![
            left_sole_to_right_sole.x(),
            left_sole_to_right_sole.y(),
            0.0
        ] / 2.0,
    );

    let ground_to_robot = match support_foot {
        Side::Left => left_sole_horizontal_to_robot * ground_to_left_sole,
        Side::Right => right_sole_horizontal_to_robot * ground_to_right_sole,
    };

    Some(ground_to_robot)
}
