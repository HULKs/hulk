use std::{boxed::Box, future::Future, pin::Pin, sync::Arc};

use booster::{FallDownState, FallDownStateType, ImuState};
use color_eyre::Result;
use coordinate_systems::{LeftSole, RightSole};
use kinematics::{
    robot_kinematics::RobotKinematics,
    sole_contact::{
        SoleSide, SupportSelectionParameters, estimate_sole_contact, select_support_side,
    },
};
use linear_algebra::Point3;
use serde::{Deserialize, Serialize};

use ros_z::{prelude::*, qos::QosDurability};
use types::{support_side::Side, time_wrapper::TimeWrapper};

pub const ACTUAL_IMAGE_HEIGHT: f32 = 448.0;
pub const ACTUAL_IMAGE_WIDTH: f32 = 544.0;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub contact_height_epsilon: f32,
    pub double_support_deadband: f32,
    pub support_switch_hysteresis: f32,
    pub left_sole_contact_vertices: Vec<Point3<LeftSole>>,
    pub right_sole_contact_vertices: Vec<Point3<RightSole>>,
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

    let mut last_support_foot = SoleSide::Left;
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
            imu_state.roll_pitch_yaw.x(),
            imu_state.roll_pitch_yaw.y(),
            &robot_kinematics_wrapper.inner,
            last_support_foot,
        );

        let support_foot = if matches!(fall_down_state.fall_down_state, FallDownStateType::IsReady)
        {
            if let Some(current_support_foot) = current_support_foot {
                last_support_foot = current_support_foot;
            }
            current_support_foot.map(side_from_sole_side)
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
    roll: f32,
    pitch: f32,
    robot_kinematics: &RobotKinematics,
    last_support_foot: SoleSide,
) -> Option<SoleSide> {
    let left_contact = estimate_sole_contact(
        robot_kinematics.left_leg.sole_to_robot,
        roll,
        pitch,
        &parameters.left_sole_contact_vertices,
        parameters.contact_height_epsilon,
    )?;
    let right_contact = estimate_sole_contact(
        robot_kinematics.right_leg.sole_to_robot,
        roll,
        pitch,
        &parameters.right_sole_contact_vertices,
        parameters.contact_height_epsilon,
    )?;

    select_support_side(
        left_contact,
        right_contact,
        Some(last_support_foot),
        SupportSelectionParameters {
            double_support_deadband: parameters.double_support_deadband,
            support_switch_hysteresis: parameters.support_switch_hysteresis,
        },
    )
}

fn side_from_sole_side(side: SoleSide) -> Side {
    match side {
        SoleSide::Left => Side::Left,
        SoleSide::Right => Side::Right,
    }
}

#[cfg(test)]
mod tests {
    use coordinate_systems::{LeftSole, RightSole};
    use kinematics::robot_kinematics::{
        RobotKinematics, RobotLeftLegKinematics, RobotRightLegKinematics,
    };
    use linear_algebra::{IntoTransform, nalgebra, point};

    use super::*;

    fn parameters() -> Parameters {
        Parameters {
            contact_height_epsilon: 0.001,
            double_support_deadband: 0.001,
            support_switch_hysteresis: 0.002,
            left_sole_contact_vertices: vec![
                point![<LeftSole>, 0.1, 0.05, 0.0],
                point![<LeftSole>, 0.1, -0.05, 0.0],
                point![<LeftSole>, -0.1, 0.05, 0.0],
                point![<LeftSole>, -0.1, -0.05, 0.0],
            ],
            right_sole_contact_vertices: vec![
                point![<RightSole>, 0.1, 0.05, 0.0],
                point![<RightSole>, 0.1, -0.05, 0.0],
                point![<RightSole>, -0.1, 0.05, 0.0],
                point![<RightSole>, -0.1, -0.05, 0.0],
            ],
        }
    }

    #[test]
    fn lower_contact_vertices_choose_left_support() {
        let robot_kinematics = RobotKinematics {
            left_leg: RobotLeftLegKinematics {
                sole_to_robot: nalgebra::Isometry3::translation(0.0, 0.05, -0.02)
                    .framed_transform(),
                ..Default::default()
            },
            right_leg: RobotRightLegKinematics {
                sole_to_robot: nalgebra::Isometry3::translation(0.0, -0.05, 0.0).framed_transform(),
                ..Default::default()
            },
            ..Default::default()
        };

        let support =
            estimate_support_foot(&parameters(), 0.0, 0.0, &robot_kinematics, SoleSide::Right);

        assert_eq!(support, Some(SoleSide::Left));
    }

    #[test]
    fn double_support_deadband_returns_none() {
        let robot_kinematics = RobotKinematics {
            left_leg: RobotLeftLegKinematics {
                sole_to_robot: nalgebra::Isometry3::translation(0.0, 0.05, 0.0).framed_transform(),
                ..Default::default()
            },
            right_leg: RobotRightLegKinematics {
                sole_to_robot: nalgebra::Isometry3::translation(0.0, -0.05, -0.0005)
                    .framed_transform(),
                ..Default::default()
            },
            ..Default::default()
        };

        let support =
            estimate_support_foot(&parameters(), 0.0, 0.0, &robot_kinematics, SoleSide::Left);

        assert_eq!(support, None);
    }
}
