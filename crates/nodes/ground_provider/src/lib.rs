use std::sync::Arc;

use color_eyre::Result;

use booster::ImuState;
use coordinate_systems::{Ground, Robot};
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::{Isometry3, Orientation3, vector};
use ros_z::{prelude::*, qos::QosDurability};
use types::support_foot::Side;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("ground_provider").build().await?;

    let imu_state_sub = node
        .subscriber::<ImuState>("inputs/imu_state")?
        .build()
        .await?;

    let robot_kinematics_cache = node
        .create_cache::<RobotKinematics>("robot_kinematics", 10)?
        .build()
        .await?;
    let support_foot_cache = node
        .create_cache::<Option<Side>>("support_foot", 10)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let robot_to_ground_pub = node
        .publisher::<Option<Isometry3<Robot, Ground>>>("robot_to_ground")?
        .build()
        .await?;
    let ground_to_robot_pub = node
        .publisher::<Option<Isometry3<Ground, Robot>>>("ground_to_robot")?
        .build()
        .await?;

    loop {
        let imu_state = imu_state_sub.recv_with_metadata().await?;

        let time_stamp = imu_state.source_time;

        let maybe_robot_kinematics = robot_kinematics_cache.get_nearest(time_stamp);
        let maybe_support_foot = support_foot_cache.get_nearest(time_stamp);

        let (Some(robot_kinematics), Some(support_foot)) =
            (maybe_robot_kinematics, maybe_support_foot)
        else {
            continue;
        };

        let ground_to_robot = if let Some(support_foot) = *support_foot {
            compute_ground_to_robot(
                &imu_state.into_message(),
                robot_kinematics.as_ref(),
                &support_foot,
            )
        } else {
            None
        };
        let robot_to_ground = ground_to_robot.map(|ground_to_robot| ground_to_robot.inverse());

        ground_to_robot_pub.publish(&ground_to_robot).await?;
        robot_to_ground_pub.publish(&robot_to_ground).await?;
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
